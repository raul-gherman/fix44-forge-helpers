//! Special FIX protocol functions for timestamps, ClOrdID generation, and other utilities.
//!
//! This module provides specialized functions for FIX protocol operations that go beyond
//! basic data type serialization, including high-performance timestamp formatting and
//! unique identifier generation.
//!
//! Optimization note (timestamp formatting):
//! - We cache the pre-rendered ASCII bytes "YYYYMMDD" (8 bytes) for the current UTC day.
//! - On each call we only render the time-of-day and milliseconds.
//! - Date recomputation (civil conversion + digit formatting) happens only on day rollover.
//! - Publication order: we write the cached bytes first, then publish the day with a
//!   `Release` store. Readers load the day with `Acquire` to ensure they observe
//!   corresponding digits (or recompute if mismatch).

use crate::DIGIT_PAIRS;
use core::ptr;
use std::sync::{
    OnceLock,
    atomic::{AtomicU64, Ordering},
};

// -----------------------------------------------------------------------------------------
// Date cache (days since Unix epoch -> pre-rendered "YYYYMMDD" ASCII)
//
// CACHED_DAY acts as the version/publish flag. When a new day is detected, we recompute
// and store the 8 ASCII digits, then store the day with `Release`. Readers load the day
// with `Acquire` before copying the digits (or recomputing if mismatch).
// -----------------------------------------------------------------------------------------
static CACHED_DAY: AtomicU64 = AtomicU64::new(u64::MAX); // Sentinel invalid day
static CACHED_YYYYMMDD: AtomicU64 = AtomicU64::new(0); // 8 ASCII bytes (native endian)

// ClOrdID generation state
const CNT_BITS: u64 = 32;
const CNT_MASK: u64 = (1u64 << CNT_BITS) - 1;
static COUNTER: AtomicU64 = AtomicU64::new(0);
static PROCESS_TAG: OnceLock<u32> = OnceLock::new();

// Constants
const SECS_PER_DAY: u64 = 86_400;

#[cfg(test)]
pub(crate) fn __reset_date_cache_for_test() {
    // Invalidate cached day so next call recomputes
    CACHED_DAY.store(u64::MAX, Ordering::Release);
}

/// Recompute and publish cached date digits for the given day number (days since Unix epoch).
#[inline(always)]
fn publish_date_for_day(day_number: u64) {
    // Convert day_number back to civil date (UTC) using the algorithm from
    // Howard Hinnant's date algorithms (same as original implementation).
    let days = day_number as i64;
    let z = days + 719_468;
    let era = z / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = mp + if mp < 10 { 3 } else { -9 };
    let year = (y + if m <= 2 { 1 } else { 0 }) as u16;
    let month = m as u8;
    let day = d as u8;

    // Render YYYYMMDD into 8 bytes using DIGIT_PAIRS
    let mut buf = [0u8; 8];
    unsafe {
        // Year
        let year_hi = (year / 100) as usize;
        let year_lo = (year % 100) as usize;
        ptr::copy_nonoverlapping(
            DIGIT_PAIRS.as_ptr().add(year_hi * 2),
            buf.as_mut_ptr().add(0),
            2,
        );
        ptr::copy_nonoverlapping(
            DIGIT_PAIRS.as_ptr().add(year_lo * 2),
            buf.as_mut_ptr().add(2),
            2,
        );
        // Month
        ptr::copy_nonoverlapping(
            DIGIT_PAIRS.as_ptr().add(month as usize * 2),
            buf.as_mut_ptr().add(4),
            2,
        );
        // Day
        ptr::copy_nonoverlapping(
            DIGIT_PAIRS.as_ptr().add(day as usize * 2),
            buf.as_mut_ptr().add(6),
            2,
        );
    }

    // Store digits (Relaxed) then publish the day (Release)
    let packed = u64::from_ne_bytes(buf);
    CACHED_YYYYMMDD.store(packed, Ordering::Relaxed);
    CACHED_DAY.store(day_number, Ordering::Release);
}

/// Ensure the date cache is initialized / up to date for the provided day number.
#[inline(always)]
fn ensure_date_cache(day_number: u64) {
    // Fast path: already current
    if CACHED_DAY.load(Ordering::Acquire) == day_number {
        return;
    }
    // Slow path: recompute (possible benign races on day boundary; last writer wins)
    publish_date_for_day(day_number);
}

/// Write a FIX-format UTC timestamp (YYYYMMDD-HH:MM:SS.mmm) with tag prefix.
///
/// This function writes a complete FIX timestamp field including the tag, equals sign,
/// timestamp value, and SOH delimiter. The timestamp is always 21 characters in the
/// format YYYYMMDD-HH:MM:SS.mmm.
///
/// # Performance
///
/// - Uses `libc::clock_gettime` (CLOCK_REALTIME) for speed.
/// - Caches pre-rendered date digits; on cache hits only time-of-day & millis are formatted.
///
/// # Example
/// ```
/// # use fix44_forge_helpers::write_tag_and_current_timestamp;
/// let mut buf = [0u8; 50];
/// let written = write_tag_and_current_timestamp(&mut buf, 0, b"52=");
/// assert!(written > 0);
/// ```
#[inline(always)]
pub fn write_tag_and_current_timestamp(
    bytes: &mut [u8],
    offset: usize,
    tag_and_eq: &[u8],
) -> usize {
    debug_assert!(bytes.len() >= offset + tag_and_eq.len() + 22);

    // Copy tag= prefix
    unsafe {
        ptr::copy_nonoverlapping(
            tag_and_eq.as_ptr(),
            bytes.as_mut_ptr().add(offset),
            tag_and_eq.len(),
        );
    }

    // Get current realtime
    let (secs_u64, millis) = {
        let mut ts = libc::timespec {
            tv_sec: 0,
            tv_nsec: 0,
        };
        unsafe { libc::clock_gettime(libc::CLOCK_REALTIME, &mut ts) };
        let secs = ts.tv_sec as i64;
        let secs_u64 = secs as u64;
        let millis = (ts.tv_nsec / 1_000_000) as u32;
        (secs_u64, millis)
    };

    let day_number = secs_u64 / SECS_PER_DAY;
    let sec_of_day = (secs_u64 - day_number * SECS_PER_DAY) as u32;

    // Time components
    let hour = (sec_of_day / 3600) as u8;
    let minute = ((sec_of_day % 3600) / 60) as u8;
    let second = (sec_of_day % 60) as u8;

    // Ensure date cache up-to-date
    ensure_date_cache(day_number);

    // Pointer to where date/time digits start (right after tag=)
    let p = unsafe { bytes.as_mut_ptr().add(offset + tag_and_eq.len()) };

    unsafe {
        // Copy cached YYYYMMDD
        let packed = CACHED_YYYYMMDD.load(Ordering::Relaxed);
        ptr::copy_nonoverlapping((&packed as *const u64) as *const u8, p, 8);

        // '-'
        *p.add(8) = b'-';

        // Hour
        ptr::copy_nonoverlapping(DIGIT_PAIRS.as_ptr().add(hour as usize * 2), p.add(9), 2);
        *p.add(11) = b':';

        // Minute
        ptr::copy_nonoverlapping(DIGIT_PAIRS.as_ptr().add(minute as usize * 2), p.add(12), 2);
        *p.add(14) = b':';

        // Second
        ptr::copy_nonoverlapping(DIGIT_PAIRS.as_ptr().add(second as usize * 2), p.add(15), 2);
        *p.add(17) = b'.';

        // Milliseconds: first two digits via pair, last digit single
        let millis_pair = (millis / 10) as usize;
        let millis_last = (millis % 10) as u8;
        ptr::copy_nonoverlapping(DIGIT_PAIRS.as_ptr().add(millis_pair * 2), p.add(18), 2);
        *p.add(20) = b'0' + millis_last;

        // SOH
        *p.add(21) = 0x01;
    }

    tag_and_eq.len() + 22
}

/// Write a FIX-format UTC timestamp using a pre-fetched `libc::timespec`.
///
/// This variant allows callers to obtain the time once (e.g. per message) and
/// reuse it for multiple timestamp tags (52=, 60=, etc.) without multiple
/// syscalls.
///
/// Buffer requirements:
/// - `bytes[offset..]` must have capacity for `tag_and_eq.len() + 22` bytes.
///
/// Returns the total number of bytes written: `tag_and_eq.len() + 22`.
#[inline(always)]
pub fn format_timestamp_from_timespec(
    bytes: &mut [u8],
    offset: usize,
    tag_and_eq: &[u8],
    ts: &libc::timespec,
) -> usize {
    debug_assert!(bytes.len() >= offset + tag_and_eq.len() + 22);

    // Copy tag=
    unsafe {
        ptr::copy_nonoverlapping(
            tag_and_eq.as_ptr(),
            bytes.as_mut_ptr().add(offset),
            tag_and_eq.len(),
        );
    }

    let secs_u64 = ts.tv_sec as u64;
    let millis = (ts.tv_nsec / 1_000_000) as u32;

    let day_number = secs_u64 / SECS_PER_DAY;
    let sec_of_day = (secs_u64 - day_number * SECS_PER_DAY) as u32;

    // Time components
    let hour = (sec_of_day / 3600) as u8;
    let minute = ((sec_of_day % 3600) / 60) as u8;
    let second = (sec_of_day % 60) as u8;

    ensure_date_cache(day_number);

    let p = unsafe { bytes.as_mut_ptr().add(offset + tag_and_eq.len()) };

    unsafe {
        // Date
        let packed = CACHED_YYYYMMDD.load(Ordering::Relaxed);
        ptr::copy_nonoverlapping((&packed as *const u64) as *const u8, p, 8);
        *p.add(8) = b'-';

        // Hour
        ptr::copy_nonoverlapping(DIGIT_PAIRS.as_ptr().add(hour as usize * 2), p.add(9), 2);
        *p.add(11) = b':';

        // Minute
        ptr::copy_nonoverlapping(DIGIT_PAIRS.as_ptr().add(minute as usize * 2), p.add(12), 2);
        *p.add(14) = b':';

        // Second
        ptr::copy_nonoverlapping(DIGIT_PAIRS.as_ptr().add(second as usize * 2), p.add(15), 2);
        *p.add(17) = b'.';

        // Milliseconds
        let millis_pair = (millis / 10) as usize;
        let millis_last = (millis % 10) as u8;
        ptr::copy_nonoverlapping(DIGIT_PAIRS.as_ptr().add(millis_pair * 2), p.add(18), 2);
        *p.add(20) = b'0' + millis_last;

        *p.add(21) = 0x01;
    }

    tag_and_eq.len() + 22
}

/// Simple 64-bit PRNG for process tag generation
#[inline(always)]
fn splitmix64(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = x;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// Generate a unique process tag from PID and memory addresses
#[inline(always)]
fn process_tag() -> u32 {
    *PROCESS_TAG.get_or_init(|| {
        // Cross-arch, no cfg: pid + addresses (ASLR noise)
        let pid = std::process::id() as u64;
        let addr_static1 = (&PROCESS_TAG as *const _) as u64;
        let addr_static2 = (&COUNTER as *const _) as u64;
        let addr_code = (splitmix64 as fn(u64) -> u64) as usize as u64;
        let local = 0u64;
        let addr_stack = (&local as *const u64) as u64;

        let mixed = splitmix64(
            pid ^ addr_static1.rotate_left(7)
                ^ addr_static2.rotate_left(17)
                ^ addr_code.rotate_left(29)
                ^ addr_stack.rotate_left(3),
        );

        // Fold to 32 bits and avoid zero
        let mut t = (mixed as u32)
            .wrapping_add((mixed >> 32) as u32)
            .rotate_left(5);
        if t == 0 {
            t = 1;
        }
        t
    })
}

/// Generate next unique 64-bit ID (32-bit process tag + 32-bit counter)
#[inline(always)]
fn next_id_u64() -> u64 {
    let tag = process_tag() as u64;
    let n = COUNTER.fetch_add(1, Ordering::Relaxed) & CNT_MASK;
    (tag << CNT_BITS) | n
}

/// Convert a remainder (0-35) to base36 digit
#[inline(always)]
fn digit36(rem: u8) -> u8 {
    debug_assert!(rem < 36);
    if rem < 10 {
        b'0' + rem
    } else {
        b'A' + (rem - 10)
    }
}

/// Encode a u64 as fixed-width 13-character base36 string.
///
/// Writes exactly 13 characters to the buffer at the specified offset.
/// Returns 13 on success, 0 if insufficient buffer capacity.
///
/// # Example
/// ```
/// # use fix44_forge_helpers::encode_base36_fixed13;
/// let mut buf = [0u8; 20];
/// let written = encode_base36_fixed13(&mut buf, 0, 12345678901234567890);
/// assert_eq!(written, 13);
/// // buf[0..13] contains the base36 representation
/// ```
#[inline(always)]
pub fn encode_base36_fixed13(dst: &mut [u8], offset: usize, mut n: u64) -> usize {
    if dst.len().saturating_sub(offset) < 13 {
        return 0;
    }
    for i in 0..13 {
        let q = n / 36;
        let rem = (n - q * 36) as u8;
        dst[offset + 12 - i] = digit36(rem);
        n = q;
    }
    13
}

/// Write a FIX tag with a unique ClOrdID (Client Order ID).
///
/// Generates a unique 13-character base36 ClOrdID and writes it as a complete
/// FIX field with tag, equals sign, value, and SOH delimiter.
///
/// The ClOrdID combines a process-unique tag (derived from PID and memory layout)
/// with an atomic counter to ensure uniqueness within and across processes.
///
/// # Example
/// ```
/// # use fix44_forge_helpers::write_tag_and_ClOrdID;
/// let mut buf = [0u8; 30];
/// let written = write_tag_and_ClOrdID(&mut buf, 0, b"11=");
/// assert_eq!(written, 17); // 3 (tag) + 13 (ID) + 1 (SOH)
/// ```
#[inline(always)]
#[allow(non_snake_case)]
pub fn write_tag_and_ClOrdID(bytes: &mut [u8], offset: usize, tag_and_eq: &[u8]) -> usize {
    let mut pos = 0;
    unsafe {
        ptr::copy_nonoverlapping(
            tag_and_eq.as_ptr(),
            bytes.as_mut_ptr().add(offset),
            tag_and_eq.len(),
        );
    }
    pos += tag_and_eq.len();
    let id = next_id_u64();
    pos += encode_base36_fixed13(bytes, offset + pos, id);
    unsafe {
        *bytes.get_unchecked_mut(offset + pos) = 0x01;
    }
    pos + 1
}

/// Format a high-resolution logging timestamp: "YYYY-MM-DD HH:MM:SS.mmm.uuu.nnn".
///
/// Length: 31 bytes. (No tag / SOH; intended for log lines.)
/// Uses cached date digits (YYYYMMDD) and inserts '-' separators.
/// Nanosecond subsecond partitioned into millisecond / microsecond / nanosecond groups.
///
/// Returns 31 on success (debug asserts sufficient capacity).
#[inline(always)]
pub fn format_logging_timestamp_from_timespec(
    bytes: &mut [u8],
    offset: usize,
    ts: &libc::timespec,
) -> usize {
    debug_assert!(bytes.len() >= offset + 31);

    let secs_u64 = ts.tv_sec as u64;
    let ns = ts.tv_nsec as u32;

    let day_number = secs_u64 / SECS_PER_DAY;
    let sec_of_day = (secs_u64 - day_number * SECS_PER_DAY) as u32;

    let hour = (sec_of_day / 3600) as u8;
    let minute = ((sec_of_day % 3600) / 60) as u8;
    let second = (sec_of_day % 60) as u8;

    // Subsecond groups
    let millis = ns / 1_000_000;
    let micros = (ns / 1_000) % 1000;
    let nanos = ns % 1000;

    ensure_date_cache(day_number);

    unsafe {
        let p = bytes.as_mut_ptr().add(offset);

        // Cached YYYYMMDD -> need YYYY-MM-DD
        let packed = CACHED_YYYYMMDD.load(Ordering::Relaxed);
        let date = packed.to_ne_bytes(); // [Y,Y,Y,Y,M,M,D,D]

        // Year
        ptr::copy_nonoverlapping(date.as_ptr().add(0), p.add(0), 4);
        // '-'
        *p.add(4) = b'-';
        // Month
        ptr::copy_nonoverlapping(date.as_ptr().add(4), p.add(5), 2);
        *p.add(7) = b'-';
        // Day
        ptr::copy_nonoverlapping(date.as_ptr().add(6), p.add(8), 2);
        *p.add(10) = b' ';

        // Hour
        ptr::copy_nonoverlapping(DIGIT_PAIRS.as_ptr().add(hour as usize * 2), p.add(11), 2);
        *p.add(13) = b':';

        // Minute
        ptr::copy_nonoverlapping(DIGIT_PAIRS.as_ptr().add(minute as usize * 2), p.add(14), 2);
        *p.add(16) = b':';

        // Second
        ptr::copy_nonoverlapping(DIGIT_PAIRS.as_ptr().add(second as usize * 2), p.add(17), 2);
        *p.add(19) = b'.';

        // Helper closure to write a 3‑digit zero-padded number fast
        #[inline(always)]
        unsafe fn write_3(dst: *mut u8, val: u32) {
            // val < 1000
            let hi = (val / 100) as usize;
            let lo2 = (val % 100) as usize;
            unsafe {
                *dst = b'0' + hi as u8;
                *dst.add(1) = *DIGIT_PAIRS.get_unchecked(lo2 * 2);
                *dst.add(2) = *DIGIT_PAIRS.get_unchecked(lo2 * 2 + 1);
            }
        }

        // Millis
        write_3(p.add(20), millis);
        *p.add(23) = b'.';
        // Micros
        write_3(p.add(24), micros);
        *p.add(27) = b'.';
        // Nanos
        write_3(p.add(28), nanos);
    }

    31
}

/// Convenience wrapper that fetches current time and formats logging timestamp.
///
/// Returns 31 bytes written.
#[inline(always)]
pub fn write_current_logging_timestamp(bytes: &mut [u8], offset: usize) -> usize {
    let mut ts = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    unsafe {
        libc::clock_gettime(libc::CLOCK_REALTIME, &mut ts);
    }
    format_logging_timestamp_from_timespec(bytes, offset, &ts)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_base36_fixed13() {
        let mut buf = [0u8; 20];
        let written = encode_base36_fixed13(&mut buf, 0, 0);
        assert_eq!(written, 13);
        assert_eq!(&buf[..13], b"0000000000000");

        let mut buf = [0u8; 20];
        let written = encode_base36_fixed13(&mut buf, 0, 35);
        assert_eq!(written, 13);
        assert_eq!(&buf[..13], b"000000000000Z");

        // Test insufficient capacity
        let mut buf = [0u8; 10];
        let written = encode_base36_fixed13(&mut buf, 0, 123);
        assert_eq!(written, 0);
    }

    #[test]
    fn test_digit36() {
        assert_eq!(digit36(0), b'0');
        assert_eq!(digit36(9), b'9');
        assert_eq!(digit36(10), b'A');
        assert_eq!(digit36(35), b'Z');
    }

    #[test]
    fn test_process_tag_consistency() {
        let tag1 = process_tag();
        let tag2 = process_tag();
        assert_eq!(tag1, tag2);
        assert_ne!(tag1, 0);
    }

    #[test]
    fn test_next_id_u64_uniqueness() {
        let id1 = next_id_u64();
        let id2 = next_id_u64();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_write_tag_and_clordid() {
        let mut buf = [0u8; 30];
        let written = write_tag_and_ClOrdID(&mut buf, 0, b"11=");
        assert_eq!(written, 17); // 3 + 13 + 1
        assert_eq!(&buf[..3], b"11=");
        assert_eq!(buf[16], 0x01); // SOH

        // Verify the ClOrdID is 13 characters of valid base36
        let clord_id = &buf[3..16];
        assert_eq!(clord_id.len(), 13);
        for &byte in clord_id {
            assert!(
                (byte >= b'0' && byte <= b'9') || (byte >= b'A' && byte <= b'Z'),
                "Invalid base36 character: {}",
                byte as char
            );
        }

        // (rollover test moved to module scope below)
    }

    #[test]
    fn test_write_tag_and_current_timestamp_format() {
        let mut buf = [0u8; 50];
        let written = write_tag_and_current_timestamp(&mut buf, 0, b"52=");

        assert_eq!(&buf[..3], b"52=");
        assert_eq!(buf[written - 1], 0x01);

        let timestamp = &buf[3..written - 1];
        assert_eq!(timestamp.len(), 21);
        assert_eq!(timestamp[8], b'-');
        assert_eq!(timestamp[11], b':');
        assert_eq!(timestamp[14], b':');
        assert_eq!(timestamp[17], b'.');

        for (i, &byte) in timestamp.iter().enumerate() {
            if ![8, 11, 14, 17].contains(&i) {
                assert!(
                    byte >= b'0' && byte <= b'9',
                    "Non-digit at position {}: {}",
                    i,
                    byte as char
                );
            }
        }
    }

    #[test]
    fn test_format_logging_timestamp_from_timespec_epoch() {
        __reset_date_cache_for_test();
        let ts = libc::timespec {
            tv_sec: 0,
            tv_nsec: 987_654_321,
        };
        let mut buf = [0u8; 64];
        let written = format_logging_timestamp_from_timespec(&mut buf, 0, &ts);
        assert_eq!(written, 31);
        let s = core::str::from_utf8(&buf[..written]).unwrap();
        assert_eq!(s, "1970-01-01 00:00:00.987.654.321");
    }

    #[test]
    fn test_write_current_logging_timestamp_basic() {
        let mut buf = [0u8; 64];
        let written = write_current_logging_timestamp(&mut buf, 0);
        assert_eq!(written, 31);
        assert_eq!(buf[4], b'-');
        assert_eq!(buf[7], b'-');
        assert_eq!(buf[10], b' ');
        assert_eq!(buf[13], b':');
        assert_eq!(buf[16], b':');
        assert_eq!(buf[19], b'.');
        assert_eq!(buf[23], b'.');
        assert_eq!(buf[27], b'.');
    }

    #[test]
    fn test_timestamp_date_cache_rollover() {
        __reset_date_cache_for_test();
        let day_n: i64 = 10;
        let ts1 = libc::timespec {
            tv_sec: day_n * 86_400 + 12 * 3600 + 34 * 60 + 56,
            tv_nsec: 123_000_000,
        };
        let ts2 = libc::timespec {
            tv_sec: (day_n + 1) * 86_400 + 1 * 3600 + 2 * 60 + 3,
            tv_nsec: 456_000_000,
        };
        let mut buf1 = [0u8; 64];
        let mut buf2 = [0u8; 64];
        let w1 = format_timestamp_from_timespec(&mut buf1, 0, b"52=", &ts1);
        let w2 = format_timestamp_from_timespec(&mut buf2, 0, b"52=", &ts2);
        assert_eq!(&buf1[..3], b"52=");
        assert_eq!(&buf2[..3], b"52=");
        assert_eq!(w1, 3 + 21 + 1);
        assert_eq!(w2, 3 + 21 + 1);
        let date1 = &buf1[3..11];
        let date2 = &buf2[3..11];
        assert_ne!(
            date1, date2,
            "Date cache failed to update across day boundary"
        );
        assert!(
            date2 > date1,
            "Rollover ordering unexpected: {:?} !< {:?}",
            date1,
            date2
        );
    }

    #[test]
    fn test_format_timestamp_from_timespec_epoch() {
        __reset_date_cache_for_test();
        let ts = libc::timespec {
            tv_sec: 0,
            tv_nsec: 123_000_000,
        };
        let mut buf = [0u8; 64];
        let written = format_timestamp_from_timespec(&mut buf, 0, b"52=", &ts);
        assert_eq!(written, 3 + 21 + 1);
        assert_eq!(&buf[..3], b"52=");
        let ts_bytes = &buf[3..written - 1];
        assert_eq!(ts_bytes, b"19700101-00:00:00.123");
        assert_eq!(buf[written - 1], 0x01);
    }
}
