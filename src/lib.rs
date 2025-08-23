//! # fix44-forge-helpers
//!
//! High-performance helper functions for FIX 4.4 protocol parsing and serialization.
//!
//! This crate provides zero-allocation, minimal-branching functions for reading and writing
//! FIX protocol data types. It's designed for hot-path performance in financial applications
//! where every nanosecond counts.
//!
//! ## Platform Support
//!
//! **Unix/Linux Only** - This crate requires Unix-like systems and will NOT compile on Windows.
//! Uses `libc::clock_gettime` and other Unix-specific system calls for maximum performance.
//!
//! ## Performance Philosophy
//!
//! - Zero allocations in hot paths
//! - Minimal branching for maximum performance
//! - Early termination on invalid input
//! - Wrapping arithmetic semantics for overflow handling
//! - Direct buffer manipulation for optimal cache usage
//!
//! # Example
//!
//! ```rust
//! use fix44_forge_helpers::*;
//!
//! // Reading from bytes
//! let value = read_u32(b"12345");
//! assert_eq!(value, 12345);
//!
//! // Writing with pre-initialized forge buffer
//! let mut buffer = forge_out_buffer("FIX.4.4");
//! let mut pos = FORGE_WRITE_START; // Header "8=FIX.4.4\x019=0000\x0135=" already there!
//!
//! // Write MsgType value (tag "35=" already present)
//! buffer[pos] = b'D'; pos += 1;
//! buffer[pos] = 0x01; pos += 1; // SOH
//!
//! // Continue with other fields
//! pos += write_tag_and_u32(&mut buffer, pos, b"34=", 123);
//!
//! // Update BodyLength with actual length
//! update_body_length(&mut buffer, pos);
//! // Result: "8=FIX.4.4\x019=0012\x0135=D\x0134=123\x01"
//! ```

// Compile-time platform check
#[cfg(not(unix))]
compile_error!(
    "fix44-forge-helpers requires a Unix-like operating system (Linux, macOS, BSD). Windows is not supported due to the use of Unix-specific system calls like libc::clock_gettime. Consider using WSL2 or a containerized Linux environment for Windows development."
);

pub mod buffer;
pub mod errors;
pub mod reading;
pub mod special;
pub mod writing;

// Re-export all public items for convenience
pub use buffer::*;
pub use errors::*;
pub use reading::*;
pub use special::*;
pub use writing::*;

// Common constants used across modules
pub(crate) const DIGIT_PAIRS: &[u8; 200] = &[
    48, 48, 48, 49, 48, 50, 48, 51, 48, 52, 48, 53, 48, 54, 48, 55, 48, 56, 48, 57, 49, 48, 49, 49,
    49, 50, 49, 51, 49, 52, 49, 53, 49, 54, 49, 55, 49, 56, 49, 57, 50, 48, 50, 49, 50, 50, 50, 51,
    50, 52, 50, 53, 50, 54, 50, 55, 50, 56, 50, 57, 51, 48, 51, 49, 51, 50, 51, 51, 51, 52, 51, 53,
    51, 54, 51, 55, 51, 56, 51, 57, 52, 48, 52, 49, 52, 50, 52, 51, 52, 52, 52, 53, 52, 54, 52, 55,
    52, 56, 52, 57, 53, 48, 53, 49, 53, 50, 53, 51, 53, 52, 53, 53, 53, 54, 53, 55, 53, 56, 53, 57,
    54, 48, 54, 49, 54, 50, 54, 51, 54, 52, 54, 53, 54, 54, 54, 55, 54, 56, 54, 57, 55, 48, 55, 49,
    55, 50, 55, 51, 55, 52, 55, 53, 55, 54, 55, 55, 55, 56, 55, 57, 56, 48, 56, 49, 56, 50, 56, 51,
    56, 52, 56, 53, 56, 54, 56, 55, 56, 56, 56, 57, 57, 48, 57, 49, 57, 50, 57, 51, 57, 52, 57, 53,
    57, 54, 57, 55, 57, 56, 57, 57,
];
