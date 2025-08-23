//! Buffer management for FIX message creation and manipulation.
//!
//! This module provides high-performance buffer initialization and management
//! specifically designed for FIX protocol message generation. The core concept
//! is pre-initializing buffers with the fixed header structure that appears
//! in every FIX message.

use core::ptr;

/// Default buffer size for FIX message writing operations.
/// 1024 bytes should accommodate most FIX messages in practice.
pub const FORGE_BUFFER_SIZE: usize = 1024;

/// Length of the complete pre-initialized header for FIX 4.x versions.
/// Layout: "8=FIX.4.4\x019=0000\x0135=" (20 bytes)
/// Layout: "8=FIX.4.2\x019=0000\x0135=" (20 bytes)
/// All FIX 4.x versions have the same header length.
pub const FORGE_HEADER_LEN: usize = 20;

/// Position where BodyLength value starts (the "0000" part).
/// For all FIX 4.x versions: "8=FIX.4.x\x01" (10 bytes) + "9=" (2 bytes) = position 12
pub const BODY_LENGTH_VALUE_POS: usize = 12;

/// Starting position for writing MsgType value after the pre-initialized header.
/// This is where MsgType value writing should begin for all FIX 4.x versions.
pub const FORGE_WRITE_START: usize = 20;

/// Create a pre-initialized buffer for FIX message writing with specified version.
///
/// This function returns a buffer that is already initialized with the
/// version-specific FIX header structure.
///
/// # Arguments
/// * `fix_version` - The FIX version string (e.g., "FIX.4.4", "FIX.4.2", "FIXT.1.1")
///
/// # Buffer Layout Examples
/// For all FIX 4.x versions:
/// - Bytes 0-9: BeginString "8=FIX.4.x\x01"
/// - Bytes 10-16: BodyLength placeholder "9=0000\x01"
/// - Bytes 17-19: MsgType tag "35="
/// - Bytes 20+: Available for MsgType value and remaining message content
///
/// # Example
/// ```
/// # use fix44_forge_helpers::*;
/// let mut buffer = forge_out_buffer("FIX.4.4");
///
/// // Fixed header is already there, start writing MsgType value
/// let mut pos = FORGE_WRITE_START;
/// buffer[pos] = b'D'; pos += 1; // Just the MsgType value
/// buffer[pos] = 0x01; pos += 1; // SOH after MsgType
/// // ... continue writing other fields
/// // Finally, update BodyLength
/// ```
///
/// # Performance
/// This function performs a single copy to initialize the entire
/// fixed header structure. BeginString, BodyLength structure, and MsgType tag
/// are never written again during message serialization.
#[inline]
pub fn forge_out_buffer(fix_version: &str) -> [u8; FORGE_BUFFER_SIZE] {
    let mut buffer = [0u8; FORGE_BUFFER_SIZE];

    // Build the header dynamically: "8={version}\x019=0000\x0135="
    let mut pos = 0;

    // Write "8="
    buffer[pos] = b'8';
    buffer[pos + 1] = b'=';
    pos += 2;

    // Write version string
    let version_bytes = fix_version.as_bytes();
    unsafe {
        ptr::copy_nonoverlapping(
            version_bytes.as_ptr(),
            buffer.as_mut_ptr().add(pos),
            version_bytes.len(),
        );
    }
    pos += version_bytes.len();

    // Write SOH + "9=0000" + SOH + "35="
    let suffix = b"\x019=0000\x0135=";
    unsafe {
        ptr::copy_nonoverlapping(
            suffix.as_ptr(),
            buffer.as_mut_ptr().add(pos),
            suffix.len(),
        );
    }

    buffer
}

/// Update the BodyLength field in a forge buffer with the actual message length.
///
/// This function updates the "0000" placeholder in the BodyLength field (9=0000)
/// with the actual body length value. The body length is everything after
/// BeginString and BodyLength field itself.
///
/// # Arguments
/// * `buffer` - The forge buffer created with `forge_out_buffer()`
/// * `fix_version` - The FIX version used to create the buffer
/// * `message_length` - Position where CheckSum will be written
///
/// # Safety
/// Caller must ensure the buffer was created with `forge_out_buffer()` and
/// the body_length fits in 4 digits (0-9999).
///
/// # Example
/// ```
/// # use fix44_forge_helpers::*;
/// let mut buffer = forge_out_buffer("FIX.4.4");
/// let mut pos = forge_write_start("FIX.4.4");
/// // ... write message content and any trailer fields except CheckSum
/// // pos is now at the position where CheckSum (10=XXX) will be written
/// update_body_length(&mut buffer, pos);
/// ```
#[inline(always)]
pub fn update_body_length(
    buffer: &mut [u8],
    message_length: usize,
) {
    // All FIX 4.x versions have same structure: "8=FIX.4.x\x01" (10 bytes) + "9=0000\x01" (7 bytes) = 17 bytes total
    let body_length: u16 = (message_length - 17) as u16;

    // Write 4-digit zero-padded body length
    let thousands = (body_length / 1000) % 10;
    let hundreds = (body_length / 100) % 10;
    let tens = (body_length / 10) % 10;
    let units = body_length % 10;

    unsafe {
        let ptr = buffer
            .as_mut_ptr()
            .add(BODY_LENGTH_VALUE_POS);
        *ptr = b'0' + thousands as u8;
        *ptr.add(1) = b'0' + hundreds as u8;
        *ptr.add(2) = b'0' + tens as u8;
        *ptr.add(3) = b'0' + units as u8;
    }
}

/// Get the starting position for writing MsgType value for FIX 4.x versions.
/// All FIX 4.x versions have the same header length, so this returns the constant.
///
/// # Example
/// ```
/// # use fix44_forge_helpers::*;
/// let start_pos = forge_write_start("FIX.4.4");
/// assert_eq!(start_pos, 20);
///
/// let start_pos = forge_write_start("FIX.4.2");
/// assert_eq!(start_pos, 20);
/// ```
#[inline]
pub fn forge_write_start(_fix_version: &str) -> usize {
    FORGE_WRITE_START
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::writing::write_tag_and_u32;

    #[test]
    fn test_forge_out_buffer_fix44() {
        let buffer = forge_out_buffer("FIX.4.4");

        // Verify buffer size
        assert_eq!(
            buffer.len(),
            FORGE_BUFFER_SIZE
        );
        assert_eq!(buffer.len(), 1024);

        // Verify complete header is pre-initialized for FIX.4.4
        let expected_header = b"8=FIX.4.4\x019=0000\x0135=";
        assert_eq!(
            &buffer[..expected_header.len()],
            expected_header
        );

        // Verify rest is zero-initialized
        assert!(
            buffer[FORGE_WRITE_START..]
                .iter()
                .all(|&b| b == 0)
        );
    }

    #[test]
    fn test_forge_out_buffer_fix42() {
        let buffer = forge_out_buffer("FIX.4.2");

        // Verify complete header is pre-initialized for FIX.4.2
        let expected_header = b"8=FIX.4.2\x019=0000\x0135=";
        assert_eq!(
            &buffer[..expected_header.len()],
            expected_header
        );

        // Verify rest is zero-initialized
        assert!(
            buffer[FORGE_WRITE_START..]
                .iter()
                .all(|&b| b == 0)
        );
    }

    #[test]
    fn test_forge_out_buffer_ready_for_writing() {
        let mut buffer = forge_out_buffer("FIX.4.4");

        // Start writing MsgType value after pre-initialized header
        let mut pos = FORGE_WRITE_START;

        // Write MsgType value (tag "35=" is already there)
        buffer[pos] = b'D';
        pos += 1;
        buffer[pos] = 0x01;
        pos += 1; // SOH

        // Write other fields normally
        pos += write_tag_and_u32(&mut buffer, pos, b"34=", 123);

        // Update body length (everything after BeginString and BodyLength field)
        update_body_length(&mut buffer, pos);

        // Verify the complete message
        let expected = b"8=FIX.4.4\x019=0012\x0135=D\x0134=123\x01";
        assert_eq!(&buffer[..pos], expected);

        // Verify header was never overwritten
        assert_eq!(&buffer[..3], b"8=F"); // BeginString start
        assert_eq!(&buffer[10..12], b"9="); // BodyLength tag
        assert_eq!(&buffer[17..20], b"35="); // MsgType tag
    }

    #[test]
    fn test_forge_out_buffer_independence() {
        let buffer1 = forge_out_buffer("FIX.4.4");
        let mut buffer2 = forge_out_buffer("FIX.4.4");

        // Modify buffer2
        buffer2[FORGE_WRITE_START] = b'X';

        // buffer1 should be unchanged
        let expected_header = b"8=FIX.4.4\x019=0000\x0135=";
        assert_eq!(
            &buffer1[..expected_header.len()],
            expected_header
        );
        assert_eq!(buffer1[FORGE_WRITE_START], 0);

        // buffer2 should have the modification
        assert_eq!(
            &buffer2[..expected_header.len()],
            expected_header
        );
        assert_eq!(
            buffer2[FORGE_WRITE_START],
            b'X'
        );
    }

    #[test]
    fn test_update_body_length() {
        let mut buffer = forge_out_buffer("FIX.4.4");

        // Test various body lengths for FIX.4.4
        update_body_length(&mut buffer, 17); // body_length = 17 - 17 = 0
        assert_eq!(
            &buffer[BODY_LENGTH_VALUE_POS..BODY_LENGTH_VALUE_POS + 4],
            b"0000"
        );

        update_body_length(&mut buffer, 59); // body_length = 59 - 17 = 42
        assert_eq!(
            &buffer[BODY_LENGTH_VALUE_POS..BODY_LENGTH_VALUE_POS + 4],
            b"0042"
        );

        update_body_length(&mut buffer, 1251); // body_length = 1251 - 17 = 1234
        assert_eq!(
            &buffer[BODY_LENGTH_VALUE_POS..BODY_LENGTH_VALUE_POS + 4],
            b"1234"
        );

        update_body_length(&mut buffer, 10016); // body_length = 10016 - 17 = 9999
        assert_eq!(
            &buffer[BODY_LENGTH_VALUE_POS..BODY_LENGTH_VALUE_POS + 4],
            b"9999"
        );

        // Verify the rest of the header is unchanged
        assert_eq!(
            &buffer[..10],
            b"8=FIX.4.4\x01"
        );
        assert_eq!(&buffer[10..12], b"9=");
        assert_eq!(&buffer[16..20], b"\x0135=");
    }

    #[test]
    fn test_forge_helper_functions() {
        // Test FIX.4.4
        assert_eq!(
            forge_write_start("FIX.4.4"),
            20
        );

        // Test FIX.4.2
        assert_eq!(
            forge_write_start("FIX.4.2"),
            20
        );

        // Verify constants
        assert_eq!(FORGE_WRITE_START, 20);
        assert_eq!(BODY_LENGTH_VALUE_POS, 12);
        assert_eq!(FORGE_HEADER_LEN, 20);
    }
}
