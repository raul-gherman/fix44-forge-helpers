//! High-performance writing functions for FIX protocol data types.
//!
//! This module provides zero-allocation, minimal-branching functions for serializing
//! primitive types directly to byte buffers. All functions are designed for hot-path
//! performance in financial applications.
//!
//! # Design Philosophy
//!
//! - **Zero allocations**: All writing happens directly to caller-provided buffers
//! - **No bounds checking**: Caller must guarantee sufficient buffer capacity
//! - **Backward fill**: Uses precomputed digit pairs for optimal performance
//! - **Fixed precision floats**: f32 uses 6 decimal places, f64 uses 15
//! - **Trailing zero trimming**: Removes unnecessary zeros from fractional parts
//! - **No scientific notation**: Always uses decimal format
//!
//! # Safety
//!
//! All functions use `unsafe` operations for maximum performance. The caller MUST:
//! - Guarantee sufficient buffer capacity beyond the offset
//! - Ensure input values are finite (for floats)
//! - Handle NaN/Inf inputs before calling float writers
//!
//! # Capacity Requirements
//!
//! - `u16`: 5 bytes max
//! - `u32`: 10 bytes max
//! - `u64`: 20 bytes max
//! - `u128`: 39 bytes max
//! - Signed integers: +1 byte for optional minus sign
//! - `f32`: ~15 bytes (sign + integer + '.' + 6 fractional)
//! - `f64`: ~25 bytes (sign + integer + '.' + 15 fractional)

use crate::DIGIT_PAIRS;
use core::ptr;

// Constants for float scaling
const SCALE_F32: f32 = 1_000_000.0; // 6 decimal places
const SCALE_F64: f64 = 1_000_000_000_000_000.0; // 15 decimal places

// Powers of 10 for u128 digit counting
const POW10_U128: [u128; 39] = [
    1,
    10,
    100,
    1_000,
    10_000,
    100_000,
    1_000_000,
    10_000_000,
    100_000_000,
    1_000_000_000,
    10_000_000_000,
    100_000_000_000,
    1_000_000_000_000,
    10_000_000_000_000,
    100_000_000_000_000,
    1_000_000_000_000_000,
    10_000_000_000_000_000,
    100_000_000_000_000_000,
    1_000_000_000_000_000_000,
    10_000_000_000_000_000_000,
    100_000_000_000_000_000_000,
    1_000_000_000_000_000_000_000,
    10_000_000_000_000_000_000_000,
    100_000_000_000_000_000_000_000,
    1_000_000_000_000_000_000_000_000,
    10_000_000_000_000_000_000_000_000,
    100_000_000_000_000_000_000_000_000,
    1_000_000_000_000_000_000_000_000_000,
    10_000_000_000_000_000_000_000_000_000,
    100_000_000_000_000_000_000_000_000_000,
    1_000_000_000_000_000_000_000_000_000_000,
    10_000_000_000_000_000_000_000_000_000_000,
    100_000_000_000_000_000_000_000_000_000_000,
    1_000_000_000_000_000_000_000_000_000_000_000,
    10_000_000_000_000_000_000_000_000_000_000_000,
    100_000_000_000_000_000_000_000_000_000_000_000,
    1_000_000_000_000_000_000_000_000_000_000_000_000,
    10_000_000_000_000_000_000_000_000_000_000_000_000,
    100_000_000_000_000_000_000_000_000_000_000_000_000,
];

/// Calculate the decimal digit length of a u16
#[inline(always)]
fn digits_u16(n: u16) -> usize {
    if n >= 10000 {
        5
    } else if n >= 1000 {
        4
    } else if n >= 100 {
        3
    } else if n >= 10 {
        2
    } else {
        1
    }
}

/// Calculate the decimal digit length of a u32
#[inline(always)]
fn digits_u32(n: u32) -> usize {
    if n >= 1_000_000_000 {
        10
    } else if n >= 100_000_000 {
        9
    } else if n >= 10_000_000 {
        8
    } else if n >= 1_000_000 {
        7
    } else if n >= 100_000 {
        6
    } else if n >= 10_000 {
        5
    } else if n >= 1_000 {
        4
    } else if n >= 100 {
        3
    } else if n >= 10 {
        2
    } else {
        1
    }
}

/// Calculate the decimal digit length of a u64
#[inline(always)]
fn digits_u64(n: u64) -> usize {
    if n >= 10_000_000_000_000_000_000 {
        20
    } else if n >= 1_000_000_000_000_000_000 {
        19
    } else if n >= 100_000_000_000_000_000 {
        18
    } else if n >= 10_000_000_000_000_000 {
        17
    } else if n >= 1_000_000_000_000_000 {
        16
    } else if n >= 100_000_000_000_000 {
        15
    } else if n >= 10_000_000_000_000 {
        14
    } else if n >= 1_000_000_000_000 {
        13
    } else if n >= 100_000_000_000 {
        12
    } else if n >= 10_000_000_000 {
        11
    } else if n >= 1_000_000_000 {
        10
    } else if n >= 100_000_000 {
        9
    } else if n >= 10_000_000 {
        8
    } else if n >= 1_000_000 {
        7
    } else if n >= 100_000 {
        6
    } else if n >= 10_000 {
        5
    } else if n >= 1_000 {
        4
    } else if n >= 100 {
        3
    } else if n >= 10 {
        2
    } else {
        1
    }
}

/// Calculate the decimal digit length of a u128
#[inline(always)]
fn digits_u128(n: u128) -> usize {
    let mut d: usize = 39;
    while d > 1 && d <= POW10_U128.len() && n < POW10_U128[d - 1] {
        d -= 1;
    }
    d
}

/// Write a u16 to buffer at offset, returns bytes written.
///
/// # Safety
/// Caller must ensure buffer has at least 5 bytes available from offset.
///
/// # Example
/// ```
/// # use fix44_forge_helpers::write_u16;
/// let mut buf = [0u8; 10];
/// let written = write_u16(&mut buf, 0, 12345);
/// assert_eq!(written, 5);
/// assert_eq!(&buf[..written], b"12345");
/// ```
#[inline(always)]
pub fn write_u16(buf: &mut [u8], pos: usize, mut n: u16) -> usize {
    let len = digits_u16(n);
    let mut i = pos + len;
    while n >= 100 {
        let rem = (n % 100) as usize;
        n /= 100;
        i -= 2;
        unsafe {
            let ptr = buf.as_mut_ptr().add(i);
            *ptr = *DIGIT_PAIRS.get_unchecked(rem * 2);
            *ptr.add(1) = *DIGIT_PAIRS.get_unchecked(rem * 2 + 1);
        }
    }
    if n < 10 {
        i -= 1;
        unsafe {
            *buf.get_unchecked_mut(i) = (n as u8) + b'0';
        }
    } else {
        i -= 2;
        let rem = n as usize;
        unsafe {
            let ptr = buf.as_mut_ptr().add(i);
            *ptr = *DIGIT_PAIRS.get_unchecked(rem * 2);
            *ptr.add(1) = *DIGIT_PAIRS.get_unchecked(rem * 2 + 1);
        }
    }
    len
}

/// Write a u32 to buffer at offset, returns bytes written.
///
/// # Safety
/// Caller must ensure buffer has at least 10 bytes available from offset.
#[inline(always)]
pub fn write_u32(buf: &mut [u8], pos: usize, mut n: u32) -> usize {
    let len = digits_u32(n);
    let mut i = pos + len;
    while n >= 100 {
        let rem = (n % 100) as usize;
        n /= 100;
        i -= 2;
        unsafe {
            let ptr = buf.as_mut_ptr().add(i);
            *ptr = *DIGIT_PAIRS.get_unchecked(rem * 2);
            *ptr.add(1) = *DIGIT_PAIRS.get_unchecked(rem * 2 + 1);
        }
    }
    if n < 10 {
        i -= 1;
        unsafe {
            *buf.get_unchecked_mut(i) = (n as u8) + b'0';
        }
    } else {
        i -= 2;
        let rem = n as usize;
        unsafe {
            let ptr = buf.as_mut_ptr().add(i);
            *ptr = *DIGIT_PAIRS.get_unchecked(rem * 2);
            *ptr.add(1) = *DIGIT_PAIRS.get_unchecked(rem * 2 + 1);
        }
    }
    len
}

/// Write a u64 to buffer at offset, returns bytes written.
///
/// # Safety
/// Caller must ensure buffer has at least 20 bytes available from offset.
#[inline(always)]
pub fn write_u64(buf: &mut [u8], pos: usize, mut n: u64) -> usize {
    let len = digits_u64(n);
    let mut i = pos + len;
    while n >= 100 {
        let rem = (n % 100) as usize;
        n /= 100;
        i -= 2;
        unsafe {
            let ptr = buf.as_mut_ptr().add(i);
            *ptr = *DIGIT_PAIRS.get_unchecked(rem * 2);
            *ptr.add(1) = *DIGIT_PAIRS.get_unchecked(rem * 2 + 1);
        }
    }
    if n < 10 {
        i -= 1;
        unsafe {
            *buf.get_unchecked_mut(i) = (n as u8) + b'0';
        }
    } else {
        i -= 2;
        let rem = n as usize;
        unsafe {
            let ptr = buf.as_mut_ptr().add(i);
            *ptr = *DIGIT_PAIRS.get_unchecked(rem * 2);
            *ptr.add(1) = *DIGIT_PAIRS.get_unchecked(rem * 2 + 1);
        }
    }
    len
}

/// Write a u128 to buffer at offset, returns bytes written.
///
/// # Safety
/// Caller must ensure buffer has at least 39 bytes available from offset.
#[inline(always)]
pub fn write_u128(buf: &mut [u8], pos: usize, mut n: u128) -> usize {
    let len = digits_u128(n);
    let mut i = pos + len;
    while n >= 100 {
        let rem = (n % 100) as usize;
        n /= 100;
        i -= 2;
        unsafe {
            let ptr = buf.as_mut_ptr().add(i);
            *ptr = *DIGIT_PAIRS.get_unchecked(rem * 2);
            *ptr.add(1) = *DIGIT_PAIRS.get_unchecked(rem * 2 + 1);
        }
    }
    if n < 10 {
        i -= 1;
        unsafe {
            *buf.get_unchecked_mut(i) = (n as u8) + b'0';
        }
    } else {
        i -= 2;
        let rem = n as usize;
        unsafe {
            let ptr = buf.as_mut_ptr().add(i);
            *ptr = *DIGIT_PAIRS.get_unchecked(rem * 2);
            *ptr.add(1) = *DIGIT_PAIRS.get_unchecked(rem * 2 + 1);
        }
    }
    len
}

/// Write an i16 to buffer at offset, returns bytes written.
///
/// Handles the sign and delegates to write_u16 for the magnitude.
#[inline(always)]
pub fn write_i16(buf: &mut [u8], offset: usize, n: i16) -> usize {
    if n >= 0 {
        write_u16(buf, offset, n as u16)
    } else if n == i16::MIN {
        // Special case for MIN value to avoid overflow
        let bytes = b"32768";
        unsafe {
            *buf.get_unchecked_mut(offset) = b'-';
            ptr::copy_nonoverlapping(bytes.as_ptr(), buf.as_mut_ptr().add(offset + 1), 5);
        }
        6
    } else {
        unsafe {
            *buf.get_unchecked_mut(offset) = b'-';
        }
        1 + write_u16(buf, offset + 1, (-n) as u16)
    }
}

/// Write an i32 to buffer at offset, returns bytes written.
#[inline(always)]
pub fn write_i32(buf: &mut [u8], offset: usize, n: i32) -> usize {
    if n >= 0 {
        write_u32(buf, offset, n as u32)
    } else if n == i32::MIN {
        let bytes = b"2147483648";
        unsafe {
            *buf.get_unchecked_mut(offset) = b'-';
            ptr::copy_nonoverlapping(bytes.as_ptr(), buf.as_mut_ptr().add(offset + 1), 10);
        }
        11
    } else {
        unsafe {
            *buf.get_unchecked_mut(offset) = b'-';
        }
        1 + write_u32(buf, offset + 1, (-n) as u32)
    }
}

/// Write an i64 to buffer at offset, returns bytes written.
#[inline(always)]
pub fn write_i64(buf: &mut [u8], offset: usize, n: i64) -> usize {
    if n >= 0 {
        write_u64(buf, offset, n as u64)
    } else if n == i64::MIN {
        let bytes = b"9223372036854775808";
        unsafe {
            *buf.get_unchecked_mut(offset) = b'-';
            ptr::copy_nonoverlapping(bytes.as_ptr(), buf.as_mut_ptr().add(offset + 1), 19);
        }
        20
    } else {
        unsafe {
            *buf.get_unchecked_mut(offset) = b'-';
        }
        1 + write_u64(buf, offset + 1, (-n) as u64)
    }
}

/// Write exactly 6 fractional digits from a u32 scaled value.
#[inline(always)]
fn write_frac6_from_u32(mut v: u32, buf: &mut [u8], pos: usize) -> usize {
    let p0 = (v % 100) as usize;
    v /= 100;
    let p1 = (v % 100) as usize;
    v /= 100;
    let p2 = (v % 100) as usize;
    unsafe {
        let ptr = buf.as_mut_ptr().add(pos);
        *ptr = *DIGIT_PAIRS.get_unchecked(p2 * 2);
        *ptr.add(1) = *DIGIT_PAIRS.get_unchecked(p2 * 2 + 1);
        *ptr.add(2) = *DIGIT_PAIRS.get_unchecked(p1 * 2);
        *ptr.add(3) = *DIGIT_PAIRS.get_unchecked(p1 * 2 + 1);
        *ptr.add(4) = *DIGIT_PAIRS.get_unchecked(p0 * 2);
        *ptr.add(5) = *DIGIT_PAIRS.get_unchecked(p0 * 2 + 1);
    }
    6
}

/// Write exactly 15 fractional digits from a u128 scaled value.
#[inline(always)]
fn write_frac15_from_u128(mut v: u128, buf: &mut [u8], pos: usize) -> usize {
    let p0 = (v % 100) as usize;
    v /= 100;
    let p1 = (v % 100) as usize;
    v /= 100;
    let p2 = (v % 100) as usize;
    v /= 100;
    let p3 = (v % 100) as usize;
    v /= 100;
    let p4 = (v % 100) as usize;
    v /= 100;
    let p5 = (v % 100) as usize;
    v /= 100;
    let p6 = (v % 100) as usize;
    v /= 100;
    let hi = (v % 10) as u8;

    unsafe {
        let ptr = buf.as_mut_ptr().add(pos);
        *ptr = hi + b'0';
        *ptr.add(1) = *DIGIT_PAIRS.get_unchecked(p6 * 2);
        *ptr.add(2) = *DIGIT_PAIRS.get_unchecked(p6 * 2 + 1);
        *ptr.add(3) = *DIGIT_PAIRS.get_unchecked(p5 * 2);
        *ptr.add(4) = *DIGIT_PAIRS.get_unchecked(p5 * 2 + 1);
        *ptr.add(5) = *DIGIT_PAIRS.get_unchecked(p4 * 2);
        *ptr.add(6) = *DIGIT_PAIRS.get_unchecked(p4 * 2 + 1);
        *ptr.add(7) = *DIGIT_PAIRS.get_unchecked(p3 * 2);
        *ptr.add(8) = *DIGIT_PAIRS.get_unchecked(p3 * 2 + 1);
        *ptr.add(9) = *DIGIT_PAIRS.get_unchecked(p2 * 2);
        *ptr.add(10) = *DIGIT_PAIRS.get_unchecked(p2 * 2 + 1);
        *ptr.add(11) = *DIGIT_PAIRS.get_unchecked(p1 * 2);
        *ptr.add(12) = *DIGIT_PAIRS.get_unchecked(p1 * 2 + 1);
        *ptr.add(13) = *DIGIT_PAIRS.get_unchecked(p0 * 2);
        *ptr.add(14) = *DIGIT_PAIRS.get_unchecked(p0 * 2 + 1);
    }
    15
}

/// Write an f32 with up to 6 decimal places.
///
/// - Scales by 1e6, rounds to nearest-even, splits integer/fraction
/// - Omits decimal point if fractional part is zero
/// - Trims trailing zeros in fractional part
/// - Returns bytes written
///
/// # Safety
/// Caller must ensure finite input values. NaN/Inf behavior is undefined.
#[inline(always)]
pub fn write_f32(buf: &mut [u8], offset: usize, n: f32) -> usize {
    let mut pos = offset;

    let neg = n.is_sign_negative();
    let x = if neg { -n } else { n };
    if neg {
        unsafe {
            *buf.get_unchecked_mut(pos) = b'-';
        }
        pos += 1;
    }

    let scaled = (x * SCALE_F32).round() as u64;
    let int_part = scaled / 1_000_000;
    let frac = (scaled % 1_000_000) as u32;

    pos += write_u64(buf, pos, int_part);

    if frac != 0 {
        unsafe {
            *buf.get_unchecked_mut(pos) = b'.';
        }
        pos += 1;

        let start = pos;
        pos += write_frac6_from_u32(frac, buf, start);

        // Trim trailing zeros
        while pos > start {
            let c = unsafe { *buf.get_unchecked(pos - 1) };
            if c != b'0' {
                break;
            }
            pos -= 1;
        }
    }

    pos - offset
}

/// Write an f64 with up to 15 decimal places.
///
/// - Scales by 1e15, rounds to nearest-even, splits integer/fraction
/// - Omits decimal point if fractional part is zero
/// - Trims trailing zeros in fractional part
/// - Returns bytes written
///
/// # Safety
/// Caller must ensure finite input values. NaN/Inf behavior is undefined.
#[inline(always)]
pub fn write_f64(buf: &mut [u8], offset: usize, n: f64) -> usize {
    let mut pos = offset;

    let neg = n.is_sign_negative();
    let x = if neg { -n } else { n };
    if neg {
        unsafe {
            *buf.get_unchecked_mut(pos) = b'-';
        }
        pos += 1;
    }

    let scaled = (x * SCALE_F64).round();
    let scaled_u128 = if scaled <= 0.0 { 0u128 } else { scaled as u128 };

    let int_part = scaled_u128 / 1_000_000_000_000_000u128;
    let frac_u128 = scaled_u128 % 1_000_000_000_000_000u128;

    pos += if int_part <= u64::MAX as u128 {
        write_u64(buf, pos, int_part as u64)
    } else {
        write_u128(buf, pos, int_part)
    };

    if frac_u128 != 0 {
        unsafe {
            *buf.get_unchecked_mut(pos) = b'.';
        }
        pos += 1;

        let start = pos;
        pos += write_frac15_from_u128(frac_u128, buf, start);

        // Trim trailing zeros
        while pos > start {
            let c = unsafe { *buf.get_unchecked(pos - 1) };
            if c != b'0' {
                break;
            }
            pos -= 1;
        }
    }

    pos - offset
}

/// Write a FIX tag, equals sign, boolean value, and SOH delimiter.
///
/// # Example
/// ```
/// # use fix44_forge_helpers::write_tag_and_bool;
/// let mut buf = [0u8; 20];
/// let written = write_tag_and_bool(&mut buf, 0, b"54=", true);
/// assert_eq!(&buf[..written], b"54=Y\x01");
/// ```
#[inline(always)]
pub fn write_tag_and_bool(
    bytes: &mut [u8],
    offset: usize,
    tag_and_eq: &[u8],
    value: bool,
) -> usize {
    unsafe {
        ptr::copy_nonoverlapping(
            tag_and_eq.as_ptr(),
            bytes.as_mut_ptr().add(offset),
            tag_and_eq.len(),
        );
    }
    let pos = tag_and_eq.len();
    unsafe {
        let ptr = bytes.as_mut_ptr().add(offset + pos);
        *ptr = if value { b'Y' } else { b'N' };
        *ptr.add(1) = 0x01;
    }
    pos + 2
}

/// Write a FIX tag, equals sign, byte slice value, and SOH delimiter.
#[inline(always)]
pub fn write_tag_and_bytes(
    bytes: &mut [u8],
    offset: usize,
    tag_and_eq: &[u8],
    value: &[u8],
) -> usize {
    unsafe {
        ptr::copy_nonoverlapping(
            tag_and_eq.as_ptr(),
            bytes.as_mut_ptr().add(offset),
            tag_and_eq.len(),
        );
    }
    let mut pos = tag_and_eq.len();
    unsafe {
        ptr::copy_nonoverlapping(
            value.as_ptr(),
            bytes.as_mut_ptr().add(offset + pos),
            value.len(),
        );
    }
    pos += value.len();
    unsafe {
        *bytes.get_unchecked_mut(offset + pos) = 0x01;
    }
    pos + 1
}

/// Write a FIX tag, equals sign, string value, and SOH delimiter.
#[inline(always)]
pub fn write_tag_and_str(bytes: &mut [u8], offset: usize, tag_and_eq: &[u8], value: &str) -> usize {
    write_tag_and_bytes(bytes, offset, tag_and_eq, value.as_bytes())
}

/// Write a FIX tag, equals sign, u16 value, and SOH delimiter.
#[inline(always)]
pub fn write_tag_and_u16(bytes: &mut [u8], offset: usize, tag_and_eq: &[u8], value: u16) -> usize {
    unsafe {
        ptr::copy_nonoverlapping(
            tag_and_eq.as_ptr(),
            bytes.as_mut_ptr().add(offset),
            tag_and_eq.len(),
        );
    }
    let mut pos = tag_and_eq.len();
    pos += write_u16(bytes, offset + pos, value);
    unsafe {
        *bytes.get_unchecked_mut(offset + pos) = 0x01;
    }
    pos + 1
}

/// Write a FIX tag, equals sign, u32 value, and SOH delimiter.
#[inline(always)]
pub fn write_tag_and_u32(bytes: &mut [u8], offset: usize, tag_and_eq: &[u8], value: u32) -> usize {
    unsafe {
        ptr::copy_nonoverlapping(
            tag_and_eq.as_ptr(),
            bytes.as_mut_ptr().add(offset),
            tag_and_eq.len(),
        );
    }
    let mut pos = tag_and_eq.len();
    pos += write_u32(bytes, offset + pos, value);
    unsafe {
        *bytes.get_unchecked_mut(offset + pos) = 0x01;
    }
    pos + 1
}

/// Write a FIX tag, equals sign, u64 value, and SOH delimiter.
#[inline(always)]
pub fn write_tag_and_u64(bytes: &mut [u8], offset: usize, tag_and_eq: &[u8], value: u64) -> usize {
    unsafe {
        ptr::copy_nonoverlapping(
            tag_and_eq.as_ptr(),
            bytes.as_mut_ptr().add(offset),
            tag_and_eq.len(),
        );
    }
    let mut pos = tag_and_eq.len();
    pos += write_u64(bytes, offset + pos, value);
    unsafe {
        *bytes.get_unchecked_mut(offset + pos) = 0x01;
    }
    pos + 1
}

/// Write a FIX tag, equals sign, i16 value, and SOH delimiter.
#[inline(always)]
pub fn write_tag_and_i16(bytes: &mut [u8], offset: usize, tag_and_eq: &[u8], value: i16) -> usize {
    unsafe {
        ptr::copy_nonoverlapping(
            tag_and_eq.as_ptr(),
            bytes.as_mut_ptr().add(offset),
            tag_and_eq.len(),
        );
    }
    let mut pos = tag_and_eq.len();
    pos += write_i16(bytes, offset + pos, value);
    unsafe {
        *bytes.get_unchecked_mut(offset + pos) = 0x01;
    }
    pos + 1
}

/// Write a FIX tag, equals sign, i32 value, and SOH delimiter.
#[inline(always)]
pub fn write_tag_and_i32(bytes: &mut [u8], offset: usize, tag_and_eq: &[u8], value: i32) -> usize {
    unsafe {
        ptr::copy_nonoverlapping(
            tag_and_eq.as_ptr(),
            bytes.as_mut_ptr().add(offset),
            tag_and_eq.len(),
        );
    }
    let mut pos = tag_and_eq.len();
    pos += write_i32(bytes, offset + pos, value);
    unsafe {
        *bytes.get_unchecked_mut(offset + pos) = 0x01;
    }
    pos + 1
}

/// Write a FIX tag, equals sign, i64 value, and SOH delimiter.
#[inline(always)]
pub fn write_tag_and_i64(bytes: &mut [u8], offset: usize, tag_and_eq: &[u8], value: i64) -> usize {
    unsafe {
        ptr::copy_nonoverlapping(
            tag_and_eq.as_ptr(),
            bytes.as_mut_ptr().add(offset),
            tag_and_eq.len(),
        );
    }
    let mut pos = tag_and_eq.len();
    pos += write_i64(bytes, offset + pos, value);
    unsafe {
        *bytes.get_unchecked_mut(offset + pos) = 0x01;
    }
    pos + 1
}

/// Write a FIX tag, equals sign, f32 value, and SOH delimiter.
#[inline(always)]
pub fn write_tag_and_f32(bytes: &mut [u8], offset: usize, tag_and_eq: &[u8], value: f32) -> usize {
    unsafe {
        ptr::copy_nonoverlapping(
            tag_and_eq.as_ptr(),
            bytes.as_mut_ptr().add(offset),
            tag_and_eq.len(),
        );
    }
    let mut pos = tag_and_eq.len();
    pos += write_f32(bytes, offset + pos, value);
    unsafe {
        *bytes.get_unchecked_mut(offset + pos) = 0x01;
    }
    pos + 1
}

/// Write a FIX tag, equals sign, f64 value, and SOH delimiter.
#[inline(always)]
pub fn write_tag_and_f64(bytes: &mut [u8], offset: usize, tag_and_eq: &[u8], value: f64) -> usize {
    unsafe {
        ptr::copy_nonoverlapping(
            tag_and_eq.as_ptr(),
            bytes.as_mut_ptr().add(offset),
            tag_and_eq.len(),
        );
    }
    let mut pos = tag_and_eq.len();
    pos += write_f64(bytes, offset + pos, value);
    unsafe {
        *bytes.get_unchecked_mut(offset + pos) = 0x01;
    }
    pos + 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_write_u16() {
        let mut buf = [0u8; 10];
        assert_eq!(write_u16(&mut buf, 0, 0), 1);
        assert_eq!(&buf[..1], b"0");

        let mut buf = [0u8; 10];
        assert_eq!(write_u16(&mut buf, 0, 123), 3);
        assert_eq!(&buf[..3], b"123");

        let mut buf = [0u8; 10];
        assert_eq!(write_u16(&mut buf, 0, 65535), 5);
        assert_eq!(&buf[..5], b"65535");
    }

    #[test]
    fn test_write_i16() {
        let mut buf = [0u8; 10];
        assert_eq!(write_i16(&mut buf, 0, -123), 4);
        assert_eq!(&buf[..4], b"-123");

        let mut buf = [0u8; 10];
        assert_eq!(write_i16(&mut buf, 0, i16::MIN), 6);
        assert_eq!(&buf[..6], b"-32768");
    }

    #[test]
    fn test_write_f32() {
        let mut buf = [0u8; 20];
        let written = write_f32(&mut buf, 0, 123.456);
        assert_eq!(&buf[..written], b"123.456");

        let mut buf = [0u8; 20];
        let written = write_f32(&mut buf, 0, 123.0);
        assert_eq!(&buf[..written], b"123");

        let mut buf = [0u8; 20];
        let written = write_f32(&mut buf, 0, -123.456);
        assert_eq!(&buf[..written], b"-123.456");
    }

    #[test]
    fn test_write_tag_and_bool() {
        let mut buf = [0u8; 20];
        let written = write_tag_and_bool(&mut buf, 0, b"54=", true);
        assert_eq!(&buf[..written], b"54=Y\x01");

        let mut buf = [0u8; 20];
        let written = write_tag_and_bool(&mut buf, 0, b"54=", false);
        assert_eq!(&buf[..written], b"54=N\x01");
    }

    #[test]
    fn test_write_tag_and_str() {
        let mut buf = [0u8; 20];
        let written = write_tag_and_str(&mut buf, 0, b"35=", "D");
        assert_eq!(&buf[..written], b"35=D\x01");
    }

    #[test]
    fn test_write_tag_and_u32() {
        let mut buf = [0u8; 20];
        let written = write_tag_and_u32(&mut buf, 0, b"34=", 12345);
        assert_eq!(&buf[..written], b"34=12345\x01");
    }
}
