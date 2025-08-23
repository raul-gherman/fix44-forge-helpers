# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2025-08-20

### Added
- Initial release of fix44-forge-helpers
- High-performance FIX 4.4 protocol parsing and serialization functions
- **Platform Support**: Unix-like systems only (Linux, macOS, BSD) - Windows not supported
- Zero-allocation reading functions for all FIX data types:
  - Integer types: `read_u16`, `read_u32`, `read_u64`, `read_i16`, `read_i32`, `read_i64`
  - Floating point: `read_f32`, `read_f64` with automatic precision handling
  - Boolean: `read_bool` supporting FIX Y/N format
  - String: `read_str` for text fields
- Zero-allocation writing functions for all FIX data types:
  - Integer types: `write_u16`, `write_u32`, `write_u64`, `write_u128`, `write_i16`, `write_i32`, `write_i64`
  - Floating point: `write_f32`, `write_f64` with automatic precision and no scientific notation
  - Boolean: `write_bool` in FIX Y/N format
- Complete set of tag writing functions:
  - `write_tag_and_bool`, `write_tag_and_str`, `write_tag_and_bytes`
  - `write_tag_and_u16`, `write_tag_and_u32`, `write_tag_and_u64`
  - `write_tag_and_i16`, `write_tag_and_i32`, `write_tag_and_i64`
  - `write_tag_and_f32`, `write_tag_and_f64`
- Pre-initialized buffer system with `forge_out_buffer()`:
  - Contains pre-written FIX header: `"8=FIX.4.4\x019=0000\x0135="`
  - Eliminates redundant header writes in message generation
  - `update_body_length()` for trailer-based body length calculation
- Special FIX functions:
  - `write_tag_and_current_timestamp()` for high-performance UTC timestamps
  - `write_tag_and_ClOrdID()` for unique order ID generation
  - `encode_base36_fixed13()` for 13-character base36 encoding
- Error handling with `ReadError` type for structured validation
- Comprehensive test suite (31 unit tests + 18 doc tests)
- Complete benchmark coverage for all critical paths
- Extensive documentation with usage examples and performance characteristics

### Performance Optimizations
- Pointer arithmetic optimizations eliminating repeated buffer access
- Precomputed digit pair lookup tables for fast numeric conversion
- Backward fill algorithm for optimal cache usage
- Minimal branching for CPU pipeline efficiency
- Aggressive use of unsafe code for maximum performance
- Sub-nanosecond operations for simple writes
- Single-digit nanosecond performance for complex tag writes
- Platform-specific optimizations using Unix system calls (libc::clock_gettime)

### Documentation
- Complete API documentation with examples
- Performance characteristics and benchmark results
- Safety considerations and buffer capacity requirements
- Integration guide for fix44-forge code generator
- Comprehensive README with quick start guide