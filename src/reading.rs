//! High-performance reading functions for FIX protocol data types.
//!
//! This module provides zero-allocation, minimal-branching functions for parsing
//! primitive types from byte slices. All functions are designed for hot-path
//! performance in financial applications.
//!
//! # Design Philosophy
//!
//! - **Zero allocations**: All parsing happens on the stack
//! - **Early termination**: Stop at first non-digit character
//! - **Wrapping semantics**: Overflow wraps around (no error reporting)
//! - **Minimal branching**: Optimized for CPU pipeline efficiency
//! - **Unsafe optimizations**: Uses unchecked array access for performance
//!
//! # Float Format
//!
//! Floats support the format: `[-]? [0-9]* ('.' [0-9]*)?`
//! - No scientific notation support (by design)
//! - Limited fractional precision (6 digits for f32, 15 for f64)
//! - Extra fractional digits are ignored

use core::str;

/// Maximum fractional digits to parse for f32 (writers emit <= 6)
const F32_FRAC_MAX: usize = 9;

/// Maximum fractional digits to parse for f64 (writers emit <= 15)
const F64_FRAC_MAX: usize = 18;

/// Powers of 10 as f32 for fractional part calculation
const POW10_F32: [f32; F32_FRAC_MAX + 1] = [
    1.0,
    10.0,
    100.0,
    1_000.0,
    10_000.0,
    100_000.0,
    1_000_000.0,
    10_000_000.0,
    100_000_000.0,
    1_000_000_000.0,
];

/// Powers of 10 as f64 for fractional part calculation
const POW10_F64: [f64; F64_FRAC_MAX + 1] = [
    1.0,
    10.0,
    100.0,
    1_000.0,
    10_000.0,
    100_000.0,
    1_000_000.0,
    10_000_000.0,
    100_000_000.0,
    1_000_000_000.0,
    10_000_000_000.0,
    100_000_000_000.0,
    1_000_000_000_000.0,
    10_000_000_000_000.0,
    100_000_000_000_000.0,
    1_000_000_000_000_000.0,
    10_000_000_000_000_000.0,
    100_000_000_000_000_000.0,
    1_000_000_000_000_000_000.0,
];

/// Fast check if a byte is an ASCII digit
#[inline(always)]
fn is_digit(b: u8) -> bool {
    b'0' <= b && b <= b'9'
}

/// Parse a boolean value from bytes.
///
/// Returns `true` for "Y", `false` for anything else.
///
/// # Example
/// ```
/// # use fix44_forge_helpers::read_bool;
/// assert_eq!(read_bool(b"Y"), true);
/// assert_eq!(read_bool(b"N"), false);
/// assert_eq!(read_bool(b""), false);
/// ```
#[inline(always)]
pub fn read_bool(buf: &[u8]) -> bool {
    matches!(buf, [b'Y'])
}

/// Convert bytes to a string slice without UTF-8 validation.
///
/// # Safety
/// This function assumes the input bytes are valid UTF-8.
/// In FIX protocol context, this is typically safe.
///
/// # Example
/// ```
/// # use fix44_forge_helpers::read_str;
/// assert_eq!(read_str(b"hello"), "hello");
/// ```
#[inline(always)]
pub fn read_str(buf: &[u8]) -> &str {
    unsafe { str::from_utf8_unchecked(buf) }
}

/// Parse a u16 from decimal bytes.
///
/// Stops at first non-digit character. Returns 0 for empty input.
/// Uses wrapping arithmetic on overflow.
///
/// # Example
/// ```
/// # use fix44_forge_helpers::read_u16;
/// assert_eq!(read_u16(b"12345"), 12345);
/// assert_eq!(read_u16(b"123abc"), 123);
/// assert_eq!(read_u16(b""), 0);
/// ```
#[inline(always)]
pub fn read_u16(buf: &[u8]) -> u16 {
    let len = buf.len();
    if len == 0 {
        return 0;
    }
    let mut acc: u16 = 0;
    let mut i: usize = 0;
    while i < len {
        let b = unsafe { *buf.get_unchecked(i) };
        if !is_digit(b) {
            break;
        }
        acc = acc * 10 + (b - b'0') as u16;
        i += 1;
    }
    acc
}

/// Parse a u32 from decimal bytes.
///
/// Stops at first non-digit character. Returns 0 for empty input.
/// Uses wrapping arithmetic on overflow.
///
/// # Example
/// ```
/// # use fix44_forge_helpers::read_u32;
/// assert_eq!(read_u32(b"1234567890"), 1234567890);
/// assert_eq!(read_u32(b"123abc"), 123);
/// assert_eq!(read_u32(b""), 0);
/// ```
#[inline(always)]
pub fn read_u32(buf: &[u8]) -> u32 {
    let len = buf.len();
    if len == 0 {
        return 0;
    }
    let mut acc: u32 = 0;
    let mut i: usize = 0;
    while i < len {
        let b = unsafe { *buf.get_unchecked(i) };
        if !is_digit(b) {
            break;
        }
        acc = acc.wrapping_mul(10).wrapping_add((b - b'0') as u32);
        i += 1;
    }
    acc
}

/// Parse a u64 from decimal bytes.
///
/// Stops at first non-digit character. Returns 0 for empty input.
/// Uses wrapping arithmetic on overflow.
///
/// # Example
/// ```
/// # use fix44_forge_helpers::read_u64;
/// assert_eq!(read_u64(b"12345678901234567890"), 12345678901234567890);
/// assert_eq!(read_u64(b"123abc"), 123);
/// assert_eq!(read_u64(b""), 0);
/// ```
#[inline(always)]
pub fn read_u64(buf: &[u8]) -> u64 {
    let len = buf.len();
    if len == 0 {
        return 0;
    }
    let mut acc: u64 = 0;
    let mut i: usize = 0;
    while i < len {
        let b = unsafe { *buf.get_unchecked(i) };
        if !is_digit(b) {
            break;
        }
        acc = acc.wrapping_mul(10).wrapping_add((b - b'0') as u64);
        i += 1;
    }
    acc
}

/// Parse an i16 from decimal bytes.
///
/// Supports optional leading minus sign. Handles i16::MIN correctly.
/// Stops at first non-digit character after optional sign.
///
/// # Example
/// ```
/// # use fix44_forge_helpers::read_i16;
/// assert_eq!(read_i16(b"12345"), 12345);
/// assert_eq!(read_i16(b"-12345"), -12345);
/// assert_eq!(read_i16(b"-32768"), i16::MIN);
/// ```
#[inline(always)]
pub fn read_i16(buf: &[u8]) -> i16 {
    if buf.is_empty() {
        return 0;
    }
    if buf[0] == b'-' {
        let mag = read_u16(&buf[1..]) as i32;
        // Handle i16::MIN (32768 magnitude) explicitly
        if mag == 1 << 15 {
            i16::MIN
        } else {
            (-mag) as i16
        }
    } else {
        read_u16(buf) as i16
    }
}

/// Parse an i32 from decimal bytes.
///
/// Supports optional leading minus sign. Handles i32::MIN correctly.
/// Stops at first non-digit character after optional sign.
///
/// # Example
/// ```
/// # use fix44_forge_helpers::read_i32;
/// assert_eq!(read_i32(b"1234567890"), 1234567890);
/// assert_eq!(read_i32(b"-1234567890"), -1234567890);
/// assert_eq!(read_i32(b"-2147483648"), i32::MIN);
/// ```
#[inline(always)]
pub fn read_i32(buf: &[u8]) -> i32 {
    if buf.is_empty() {
        return 0;
    }
    if buf[0] == b'-' {
        let mag = read_u32(&buf[1..]) as i64;
        if mag == (1u64 << 31) as i64 {
            i32::MIN
        } else {
            (-mag) as i32
        }
    } else {
        read_u32(buf) as i32
    }
}

/// Parse an i64 from decimal bytes.
///
/// Supports optional leading minus sign. Handles i64::MIN correctly.
/// Stops at first non-digit character after optional sign.
///
/// # Example
/// ```
/// # use fix44_forge_helpers::read_i64;
/// assert_eq!(read_i64(b"1234567890123456789"), 1234567890123456789);
/// assert_eq!(read_i64(b"-1234567890123456789"), -1234567890123456789);
/// assert_eq!(read_i64(b"-9223372036854775808"), i64::MIN);
/// ```
#[inline(always)]
pub fn read_i64(buf: &[u8]) -> i64 {
    if buf.is_empty() {
        return 0;
    }
    if buf[0] == b'-' {
        // Parse magnitude as u64, then special‑case MIN
        let mag = read_u64(&buf[1..]);
        if mag == (1u128 << 63) as u64 {
            i64::MIN
        } else {
            -(mag as i64)
        }
    } else {
        read_u64(buf) as i64
    }
}

/// Parse an f32 from decimal bytes.
///
/// Format: `[-]? [0-9]* ('.' [0-9]*)?`
/// - No scientific notation support
/// - Parses up to F32_FRAC_MAX fractional digits
/// - Extra fractional digits are ignored
/// - Preserves negative zero
///
/// # Example
/// ```
/// # use fix44_forge_helpers::read_f32;
/// assert_eq!(read_f32(b"123.456"), 123.456);
/// assert_eq!(read_f32(b"-123.456"), -123.456);
/// assert_eq!(read_f32(b"123"), 123.0);
/// ```
#[inline(always)]
pub fn read_f32(buf: &[u8]) -> f32 {
    if buf.is_empty() {
        return 0.0;
    }
    let mut i = 0usize;
    let neg = buf[0] == b'-';
    if neg {
        i = 1;
        if i == buf.len() {
            return -0.0;
        }
    }

    // Integer part
    let mut int_acc: u64 = 0;
    while i < buf.len() {
        let b = unsafe { *buf.get_unchecked(i) };
        if !is_digit(b) {
            break;
        }
        int_acc = int_acc.wrapping_mul(10).wrapping_add((b - b'0') as u64);
        i += 1;
    }

    // Fractional
    let mut frac_acc: u32 = 0;
    let mut frac_len: usize = 0;
    if i < buf.len() && unsafe { *buf.get_unchecked(i) } == b'.' {
        i += 1;
        while i < buf.len() {
            let b = unsafe { *buf.get_unchecked(i) };
            if !is_digit(b) {
                break;
            }
            if frac_len < F32_FRAC_MAX {
                frac_acc = frac_acc.wrapping_mul(10).wrapping_add((b - b'0') as u32);
                frac_len += 1;
            }
            i += 1;
        }
    }

    let mut value = int_acc as f32;
    if frac_len > 0 {
        // value += frac / 10^frac_len
        let pow = POW10_F32[frac_len];
        value += (frac_acc as f32) / pow;
    }

    if neg {
        // Preserve negative zero if magnitude is zero
        if value == 0.0 { -0.0 } else { -value }
    } else {
        value
    }
}

/// Parse an f64 from decimal bytes.
///
/// Format: `[-]? [0-9]* ('.' [0-9]*)?`
/// - No scientific notation support
/// - Parses up to F64_FRAC_MAX fractional digits
/// - Extra fractional digits are ignored
/// - Preserves negative zero
///
/// # Example
/// ```
/// # use fix44_forge_helpers::read_f64;
/// assert_eq!(read_f64(b"123.456789012345"), 123.456789012345);
/// assert_eq!(read_f64(b"-123.456"), -123.456);
/// assert_eq!(read_f64(b"123"), 123.0);
/// ```
#[inline(always)]
pub fn read_f64(buf: &[u8]) -> f64 {
    if buf.is_empty() {
        return 0.0;
    }
    let mut i = 0usize;
    let neg = buf[0] == b'-';
    if neg {
        i = 1;
        if i == buf.len() {
            return -0.0;
        }
    }

    // Integer part
    let mut int_acc: u128 = 0;
    while i < buf.len() {
        let b = unsafe { *buf.get_unchecked(i) };
        if !is_digit(b) {
            break;
        }
        int_acc = int_acc.wrapping_mul(10).wrapping_add((b - b'0') as u128);
        i += 1;
    }

    // Fractional
    let mut frac_acc: u128 = 0;
    let mut frac_len: usize = 0;
    if i < buf.len() && unsafe { *buf.get_unchecked(i) } == b'.' {
        i += 1;
        while i < buf.len() {
            let b = unsafe { *buf.get_unchecked(i) };
            if !is_digit(b) {
                break;
            }
            if frac_len < F64_FRAC_MAX {
                frac_acc = frac_acc.wrapping_mul(10).wrapping_add((b - b'0') as u128);
                frac_len += 1;
            }
            i += 1;
        }
    }

    let mut value = int_acc as f64;
    if frac_len > 0 {
        let pow = POW10_F64[frac_len];
        value += (frac_acc as f64) / pow;
    }

    if neg {
        if value == 0.0 { -0.0 } else { -value }
    } else {
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_bool() {
        assert_eq!(read_bool(b"Y"), true);
        assert_eq!(read_bool(b"N"), false);
        assert_eq!(read_bool(b""), false);
        assert_eq!(read_bool(b"yes"), false);
    }

    #[test]
    fn test_read_str() {
        assert_eq!(read_str(b"hello"), "hello");
        assert_eq!(read_str(b""), "");
        assert_eq!(read_str(b"test123"), "test123");
    }

    #[test]
    fn test_read_u16() {
        assert_eq!(read_u16(b"0"), 0);
        assert_eq!(read_u16(b"123"), 123);
        assert_eq!(read_u16(b"65535"), 65535);
        assert_eq!(read_u16(b"123abc"), 123);
        assert_eq!(read_u16(b""), 0);
        assert_eq!(read_u16(b"abc"), 0);
    }

    #[test]
    fn test_read_u32() {
        assert_eq!(read_u32(b"0"), 0);
        assert_eq!(read_u32(b"123456789"), 123456789);
        assert_eq!(read_u32(b"4294967295"), 4294967295);
        assert_eq!(read_u32(b"123abc"), 123);
        assert_eq!(read_u32(b""), 0);
    }

    #[test]
    fn test_read_u64() {
        assert_eq!(read_u64(b"0"), 0);
        assert_eq!(read_u64(b"123456789012345"), 123456789012345);
        assert_eq!(read_u64(b"18446744073709551615"), 18446744073709551615);
        assert_eq!(read_u64(b"123abc"), 123);
        assert_eq!(read_u64(b""), 0);
    }

    #[test]
    fn test_read_i16() {
        assert_eq!(read_i16(b"0"), 0);
        assert_eq!(read_i16(b"123"), 123);
        assert_eq!(read_i16(b"-123"), -123);
        assert_eq!(read_i16(b"32767"), 32767);
        assert_eq!(read_i16(b"-32768"), -32768);
        assert_eq!(read_i16(b""), 0);
        assert_eq!(read_i16(b"-"), 0);
    }

    #[test]
    fn test_read_i32() {
        assert_eq!(read_i32(b"0"), 0);
        assert_eq!(read_i32(b"123456789"), 123456789);
        assert_eq!(read_i32(b"-123456789"), -123456789);
        assert_eq!(read_i32(b"2147483647"), 2147483647);
        assert_eq!(read_i32(b"-2147483648"), -2147483648);
        assert_eq!(read_i32(b""), 0);
    }

    #[test]
    fn test_read_i64() {
        assert_eq!(read_i64(b"0"), 0);
        assert_eq!(read_i64(b"123456789012345"), 123456789012345);
        assert_eq!(read_i64(b"-123456789012345"), -123456789012345);
        assert_eq!(read_i64(b"9223372036854775807"), 9223372036854775807);
        assert_eq!(read_i64(b"-9223372036854775808"), -9223372036854775808);
        assert_eq!(read_i64(b""), 0);
    }

    #[test]
    fn test_read_f32() {
        assert_eq!(read_f32(b"0"), 0.0);
        assert_eq!(read_f32(b"123.456"), 123.456);
        assert_eq!(read_f32(b"-123.456"), -123.456);
        assert_eq!(read_f32(b"123"), 123.0);
        assert_eq!(read_f32(b".456"), 0.456);
        assert_eq!(read_f32(b"123."), 123.0);
        assert_eq!(read_f32(b""), 0.0);
        assert!(read_f32(b"-0").is_sign_negative());
    }

    #[test]
    fn test_read_f64() {
        assert_eq!(read_f64(b"0"), 0.0);
        assert_eq!(read_f64(b"123.456789012345"), 123.456789012345);
        assert_eq!(read_f64(b"-123.456"), -123.456);
        assert_eq!(read_f64(b"123"), 123.0);
        assert_eq!(read_f64(b".456"), 0.456);
        assert_eq!(read_f64(b"123."), 123.0);
        assert_eq!(read_f64(b""), 0.0);
        assert!(read_f64(b"-0").is_sign_negative());
    }
}
