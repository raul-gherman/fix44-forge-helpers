//! Special FIX protocol functions for timestamps, ClOrdID generation, and other utilities.
//!
//! This module provides specialized functions for FIX protocol operations that go beyond
//! basic data type serialization, including high-performance timestamp formatting and
//! unique identifier generation.

use crate::DIGIT_PAIRS;
use core::ptr;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    OnceLock,
};

// Cache for date calculations to avoid recomputing when in same day
static CACHED_DATE: AtomicU64 = AtomicU64::new(0);
static CACHED_DATE_BYTES: AtomicU64 = AtomicU64::new(0);

// ClOrdID generation state
const CNT_BITS: u64 = 32;
const CNT_MASK: u64 = (1u64 << CNT_BITS) - 1;
static COUNTER: AtomicU64 = AtomicU64::new(0);
static PROCESS_TAG: OnceLock<u32> = OnceLock::new();

/// Write a FIX-format UTC timestamp (YYYYMMDD-HH:MM:SS.mmm) with tag prefix.
///
/// This function writes a complete FIX timestamp field including the tag, equals sign,
/// timestamp value, and SOH delimiter. The timestamp is always 21 characters in the
/// format YYYYMMDD-HH:MM:SS.mmm.
///
/// # Performance
///
/// Uses optimized date caching and `libc::clock_gettime` for maximum performance.
/// Date calculations are cached when multiple timestamps occur on the same day.
///
/// # Example
/// ```
/// # use fix44_forge_helpers::write_tag_and_current_timestamp;
/// let mut buf = [0u8; 50];
/// let written = write_tag_and_current_timestamp(&mut buf, 0, b"52=");
/// // Result: b"52=20240101-12:34:56.789\x01" (actual timestamp varies)
/// ```
#[inline(always)]
pub fn write_tag_and_current_timestamp(
    bytes: &mut [u8],
    offset: usize,
    tag_and_eq: &[u8],
) -> usize {
    // Copy tag= prefix
    unsafe {
        ptr::copy_nonoverlapping(
            tag_and_eq.as_ptr(),
            bytes.as_mut_ptr().add(offset),
            tag_and_eq.len(),
        );
    }

    // Use faster libc clock_gettime instead of SystemTime
    let (secs, millis) = {
        let mut ts = libc::timespec {
            tv_sec: 0,
            tv_nsec: 0,
        };
        unsafe { libc::clock_gettime(libc::CLOCK_REALTIME, &mut ts) };
        (
            ts.tv_sec as i64,
            (ts.tv_nsec / 1_000_000) as u32,
        )
    };

    // Fast time decomposition using bit operations where possible
    let sec_of_day = secs % 86_400;
    let days = secs / 86_400;

    // Time components - optimized with fewer divisions
    let hour = (sec_of_day / 3600) as u8;
    let remaining = sec_of_day % 3600;
    let minute = (remaining / 60) as u8;
    let second = (remaining % 60) as u8;

    // Check if we can use cached date calculation
    let current_day = days as u64;
    let cached_day = CACHED_DATE.load(Ordering::Relaxed);
    let (year, month, day) = if current_day == cached_day && cached_day != 0 {
        // Use cached date bytes
        let cached_bytes = CACHED_DATE_BYTES.load(Ordering::Relaxed);
        let year = ((cached_bytes >> 32) & 0xFFFF) as u16;
        let month = ((cached_bytes >> 16) & 0xFF) as u8;
        let day = (cached_bytes & 0xFF) as u8;
        (year, month, day)
    } else {
        // Compute new date and cache it
        let z = days + 719_468;
        let era = z / 146_097;
        let doe = z - era * 146_097;
        let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
        let y = yoe + era * 400;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let d = doy - (153 * mp + 2) / 5 + 1;
        let m = mp + if mp < 10 { 3 } else { -9 };
        let year = (y + if m <= 2 { -1 } else { 0 }) as u16;
        let month = m as u8;
        let day = d as u8;

        // Cache the results
        let packed_date = ((year as u64) << 32) | ((month as u64) << 16) | (day as u64);
        CACHED_DATE.store(current_day, Ordering::Relaxed);
        CACHED_DATE_BYTES.store(packed_date, Ordering::Relaxed);

        (year, month, day)
    };

    // Write digits with pointer arithmetic using DIGIT_PAIRS lookup
    let p = unsafe {
        bytes
            .as_mut_ptr()
            .add(offset + tag_and_eq.len())
    };

    // YYYYMMDD-HH:MM:SS.mmm - use DIGIT_PAIRS for faster digit conversion
    unsafe {
        // Year (4 digits) - split into two 2-digit pairs
        let year_hi = (year / 100) as usize;
        let year_lo = (year % 100) as usize;
        ptr::copy_nonoverlapping(
            DIGIT_PAIRS
                .as_ptr()
                .add(year_hi * 2),
            p.add(0),
            2,
        );
        ptr::copy_nonoverlapping(
            DIGIT_PAIRS
                .as_ptr()
                .add(year_lo * 2),
            p.add(2),
            2,
        );

        // Month (2 digits)
        ptr::copy_nonoverlapping(
            DIGIT_PAIRS
                .as_ptr()
                .add(month as usize * 2),
            p.add(4),
            2,
        );

        // Day (2 digits)
        ptr::copy_nonoverlapping(
            DIGIT_PAIRS
                .as_ptr()
                .add(day as usize * 2),
            p.add(6),
            2,
        );

        *p.add(8) = b'-';

        // Hour (2 digits)
        ptr::copy_nonoverlapping(
            DIGIT_PAIRS
                .as_ptr()
                .add(hour as usize * 2),
            p.add(9),
            2,
        );

        *p.add(11) = b':';

        // Minute (2 digits)
        ptr::copy_nonoverlapping(
            DIGIT_PAIRS
                .as_ptr()
                .add(minute as usize * 2),
            p.add(12),
            2,
        );

        *p.add(14) = b':';

        // Second (2 digits)
        ptr::copy_nonoverlapping(
            DIGIT_PAIRS
                .as_ptr()
                .add(second as usize * 2),
            p.add(15),
            2,
        );

        *p.add(17) = b'.';

        // Milliseconds (3 digits) - first two digits as pair, last digit individual
        let millis_pair = (millis / 10) as usize;
        let millis_last = (millis % 10) as u8;
        ptr::copy_nonoverlapping(
            DIGIT_PAIRS
                .as_ptr()
                .add(millis_pair * 2),
            p.add(18),
            2,
        );
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
pub fn encode_base36_fixed13(
    dst: &mut [u8],
    offset: usize,
    mut n: u64,
) -> usize {
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
/// // Result: b"11=ABCDEFGHIJKLM\x01" (actual ID varies)
/// assert_eq!(written, 17); // 3 (tag) + 13 (ID) + 1 (SOH)
/// ```
#[inline(always)]
#[allow(non_snake_case)]
pub fn write_tag_and_ClOrdID(
    bytes: &mut [u8],
    offset: usize,
    tag_and_eq: &[u8],
) -> usize {
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
    }

    #[test]
    fn test_write_tag_and_current_timestamp_format() {
        let mut buf = [0u8; 50];
        let written = write_tag_and_current_timestamp(&mut buf, 0, b"52=");

        // Should write tag + timestamp + SOH
        assert!(written >= 25); // 3 + 21 + 1 minimum
        assert_eq!(&buf[..3], b"52=");
        assert_eq!(buf[written - 1], 0x01); // SOH

        // Verify timestamp format: YYYYMMDD-HH:MM:SS.mmm
        let timestamp = &buf[3..written - 1];
        assert_eq!(timestamp.len(), 21);
        assert_eq!(timestamp[8], b'-');
        assert_eq!(timestamp[11], b':');
        assert_eq!(timestamp[14], b':');
        assert_eq!(timestamp[17], b'.');

        // Verify all other characters are digits
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
}
