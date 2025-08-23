# fix44-forge-helpers

[![Crates.io](https://img.shields.io/crates/v/fix44-forge-helpers.svg)](https://crates.io/crates/fix44-forge-helpers)
[![Documentation](https://docs.rs/fix44-forge-helpers/badge.svg)](https://docs.rs/fix44-forge-helpers)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](#license)
[![Build Status](https://img.shields.io/badge/build-passing-brightgreen.svg)](https://github.com/raul-gherman/fix44-forge)

High-performance helper functions for FIX 4.4 protocol parsing and serialization.

This crate provides zero-allocation, minimal-branching functions for reading and writing FIX protocol data types. It's designed for hot-path performance in financial applications where every nanosecond counts.

## Platform Support

**Unix/Linux Only** - This crate is designed exclusively for Unix-like systems and will **NOT** compile or run on Windows.

The crate uses platform-specific optimizations including:
- `libc::clock_gettime` for high-precision timestamps
- Unix-specific system calls for performance-critical operations
- Platform-optimized memory operations

**Supported platforms:**
- ✅ Linux (x86_64, aarch64)
- ✅ macOS (Intel, Apple Silicon)
- ✅ FreeBSD, OpenBSD, NetBSD
- ✅ Other Unix-like systems with libc support
- ❌ Windows (not supported)

If you need Windows compatibility, please consider crates in rust ecosystem that perform well under it - this one is not and will not be one of them.

## Features

- **Zero Allocations**: All operations use stack-only memory or write directly to caller-provided buffers
- **High Performance**: Optimized with unsafe code, precomputed lookup tables, and minimal branching
- **Comprehensive**: Supports all FIX data types including integers, floats, booleans, strings, and timestamps
- **Specialized Functions**: Includes ClOrdID generation, timestamp formatting, and Base36 encoding
- **Well Tested**: Comprehensive test suite with edge cases and performance benchmarks

## Quick Start

Add this to your `Cargo.toml`:

```toml
[dependencies]
fix44-forge-helpers = "0.1"
```

**Platform Requirements:**
- Unix-like operating system (Linux, macOS, BSD)
- Rust 1.89+ with edition 2024 support
- libc support for system calls

## Usage Examples

### Reading Data

```rust
use fix44_forge_helpers::*;

// Parse integers
let value = read_u32(b"12345");
assert_eq!(value, 12345);

// Parse floats
let price = read_f64(b"123.45");
assert_eq!(price, 123.45);

// Parse booleans (FIX Y/N format)
let flag = read_bool(b"Y");
assert_eq!(flag, true);

// Parse strings
let symbol = read_str(b"MSFT");
assert_eq!(symbol, "MSFT");
```

### Writing Data

```rust
use fix44_forge_helpers::*;

let mut buffer = [0u8; 100];
let mut pos = 0;

// Write integers
pos += write_u32(12345, &mut buffer, pos);
assert_eq!(&buffer[..pos], b"12345");

// Write floats with automatic precision
pos = 0;
pos += write_f64(123.456789, &mut buffer, pos);
assert_eq!(&buffer[..pos], b"123.456789");

// Write complete FIX fields
pos = 0;
pos += write_tag_and_u32(&mut buffer, pos, b"34=", 123);
assert_eq!(&buffer[..pos], b"34=123\x01");
```

### Special Functions

```rust
use fix44_forge_helpers::*;

let mut buffer = [0u8; 100];

// Generate unique ClOrdID
let written = write_tag_and_ClOrdID(&mut buffer, 0, b"11=");
// Result: b"11=ABCDEFGHIJKLM\x01" (13-char base36 ID)

// Write current timestamp
let written = write_tag_and_current_timestamp(&mut buffer, 0, b"52=");
// Result: b"52=20240101-12:34:56.789\x01"
```

## Performance Characteristics

### Design Philosophy

- **No bounds checking**: Uses `unsafe` operations for maximum speed (caller must ensure buffer capacity)
- **Precomputed lookup tables**: Uses digit pairs for faster numeric conversion
- **Backward fill**: Writes numbers from right to left for optimal cache usage
- **Minimal branching**: Optimized for CPU pipeline efficiency
- **SIMD-friendly**: Scalar implementations optimized for typical FIX field lengths

### Benchmarks

Run benchmarks with:

```bash
cargo bench
```

Typical performance on modern hardware:
- Integer parsing: ~1-2 ns per digit
- Float parsing: ~10-20 ns depending on precision
- Tag writing: ~5-25 ns depending on value type
- Timestamp generation: ~25-30 ns (with date caching)

## Safety Considerations

This crate uses `unsafe` code extensively for performance. When using writing functions:

1. **Platform Compatibility**: Unix-like systems only - will not compile on Windows
2. **Buffer Capacity**: Ensure sufficient buffer space (see capacity requirements below)
3. **Float Inputs**: Ensure finite values for float writers (NaN/Inf behavior is undefined)
4. **Memory Safety**: All unsafe operations are contained within function boundaries

### Buffer Capacity Requirements

| Type | Maximum Bytes |
|------|---------------|
| `u16` | 5 |
| `u32` | 10 |
| `u64` | 20 |
| `u128` | 39 |
| `i16` | 6 (includes sign) |
| `i32` | 11 (includes sign) |
| `i64` | 20 (includes sign) |
| `f32` | ~15 (sign + integer + '.' + 6 fractional) |
| `f64` | ~25 (sign + integer + '.' + 15 fractional) |
| Timestamp | 21 (YYYYMMDD-HH:MM:SS.mmm) |
| ClOrdID | 13 (base36 encoding) |

## Float Handling

### Precision
- **f32**: Up to 6 decimal places (scaled by 1e6)
- **f64**: Up to 15 decimal places (scaled by 1e15)

### Format
- No scientific notation (always decimal format)
- Automatic trailing zero trimming
- Nearest-even rounding
- Preserves negative zero

### Limitations
- No exponential notation parsing/writing
- NaN and infinity handling is undefined (caller should validate)
- Very large integers in floats may lose precision

## ClOrdID Generation

Generates unique 13-character base36 identifiers suitable for FIX ClOrdID fields:

- **Uniqueness**: Combines process-specific tag with atomic counter
- **Format**: 13 characters using [0-9A-Z]
- **Performance**: ~12-15 ns per ID generation
- **Thread Safety**: Atomic operations ensure uniqueness across threads

## Timestamp Handling

Optimized UTC timestamp generation in FIX format (YYYYMMDD-HH:MM:SS.mmm):

- **Performance**: Uses `libc::clock_gettime` for speed (Unix-only)
- **Caching**: Date calculations cached when multiple timestamps in same day
- **Format**: Always 21 characters, zero-padded
- **Precision**: Millisecond accuracy
- **Platform**: Requires Unix libc - not available on Windows

## Error Handling

### Reading Functions
- Stop at first non-digit character
- Return 0 for empty input
- Use wrapping arithmetic on overflow
- No error reporting (designed for speed)

### Writing Functions
- Assume valid finite inputs
- No bounds checking (caller responsibility)
- Return bytes written

### Validation
Use the `ReadError` type for structured validation when needed:

```rust
use fix44_forge_helpers::ReadError;

let error = ReadError::InvalidValue {
    name: "Price",
    tag: 44,
    msg: "Non-numeric character found",
};
```

## Testing

Run the test suite:

```bash
# All tests
cargo test

# Just this crate
cargo test -p fix44-forge-helpers

# With optimizations (for performance validation)
cargo test --release
```

The test suite includes:
- Unit tests for all functions
- Edge case testing (overflow, boundary values, special inputs)
- Integration tests simulating real FIX message parsing
- Property-based testing for numeric roundtrips

## FAQ

### Why doesn't this work on Windows?

This crate is intentionally Unix-only. It uses:

- **`libc::clock_gettime`** for nanosecond-precision timestamps (not available on Windows)
- **Unix-specific memory optimizations** that don't exist in Windows APIs
- **Platform-specific system calls** optimized for trading systems running on Linux

**Alternatives for Windows users:**
- Use **WSL2** (Windows Subsystem for Linux) for full compatibility
- Run in a **Docker container** with Linux base image
- Use a **Linux VM** for development and deployment
- Deploy to **Linux servers**
- Switch to **Linux**

### Can you / will you add Windows support?

No. Adding Windows support would require:
- Slower timestamp APIs (Windows doesn't have `clock_gettime`)
- Different memory management approaches
- Platform abstraction layers that add overhead
- Compromising the core performance guarantees
- Too many headaches...

This crate prioritizes **absolute performance** over platform compatibility.

## Contributing

1. Ensure all tests pass: `cargo test`
2. Run benchmarks to verify no performance regressions: `cargo bench`
3. Add tests for new functionality
4. Update documentation for any API changes
5. Verify Unix platform compatibility (Linux, macOS, BSD)

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT License ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
