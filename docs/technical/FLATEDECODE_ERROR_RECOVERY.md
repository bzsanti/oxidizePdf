# FlateDecode Error Recovery Implementation

## Summary

This document describes the implementation of robust error recovery for FlateDecode streams in oxidize-pdf, addressing the issue reported in `flate_decode_error_report.md` where PDFs with corrupted FlateDecode streams couldn't be rendered.

## Changes Implemented

### 1. ParseOptions Structure

Added `ParseOptions` to control parsing behavior with different levels of strictness:

```rust
pub struct ParseOptions {
    pub strict_mode: bool,                    // Enforce PDF spec compliance
    pub recover_from_stream_errors: bool,     // Attempt stream recovery
    pub ignore_corrupt_streams: bool,         // Skip corrupt streams
    pub partial_content_allowed: bool,        // Allow partial content
    pub max_recovery_attempts: usize,         // Max recovery attempts
    pub log_recovery_details: bool,           // Enable recovery logging
}
```

Three preset modes are available:
- `ParseOptions::strict()` - Default, follows PDF spec strictly
- `ParseOptions::tolerant()` - Attempts recovery from errors  
- `ParseOptions::skip_errors()` - Ignores corrupt streams

### 2. Robust FlateDecode Decoder

Enhanced the FlateDecode decoder with multiple recovery strategies:

1. **Standard zlib decoding** - Try normal decompression first
2. **Raw deflate without wrapper** - For PDFs with incorrect stream format
3. **Checksum validation disabled** - For streams with corrupted checksums
4. **Header byte skipping** - For corrupted headers (tries skipping 1-10 bytes)

### 3. API Changes

#### PdfReader
- Added `open_tolerant()` - Opens PDFs with tolerant parsing
- Added `open_with_options()` - Opens PDFs with custom options
- Added `with_options()` - Creates reader with custom options

#### PdfStream
- Updated `decode()` to accept `&ParseOptions` parameter
- Recovery behavior controlled by options

#### Integration
- ParseOptions propagated through all stream decoding paths
- PdfDocument exposes `options()` method
- All decode operations respect the configured options

### 4. Error Handling

With tolerant parsing:
- Corrupted streams can return empty data (with `ignore_corrupt_streams`)
- Multiple recovery strategies attempted based on `max_recovery_attempts`
- Partial content supported with `partial_content_allowed`
- Optional logging of recovery attempts (requires "logging" feature)

## Usage Examples

### Basic Usage

```rust
use oxidize_pdf::parser::{PdfReader, ParseOptions};

// Try with default strict parsing
let reader = PdfReader::open("document.pdf")?;

// Try with tolerant parsing
let tolerant_reader = PdfReader::open_tolerant("corrupted.pdf")?;

// Custom options
let options = ParseOptions {
    strict_mode: false,
    recover_from_stream_errors: true,
    ignore_corrupt_streams: true,
    partial_content_allowed: true,
    max_recovery_attempts: 5,
    log_recovery_details: false,
};
let custom_reader = PdfReader::open_with_options("problematic.pdf", options)?;
```

### Example Program

See `examples/tolerant_parsing.rs` for a complete example that:
1. Tries strict parsing first
2. Falls back to tolerant parsing
3. Uses skip_errors mode as last resort
4. Attempts to extract text even from corrupted PDFs

## Testing

Added comprehensive tests in `filters.rs`:
- `test_flate_decode_corrupt_stream` - Tests recovery with corrupted data
- `test_flate_decode_raw_deflate` - Tests raw deflate recovery
- `test_decode_stream_with_recovery` - Tests full stream decoding with recovery

## Performance Impact

- No performance impact for strict mode (default)
- Recovery attempts only triggered on decode errors
- Recovery strategies executed sequentially until success or max attempts

## Future Enhancements

The following tasks remain for future implementation:
1. Extend recovery module with more stream recovery strategies
2. Enhance StreamDecodeError with detailed diagnostics
3. Add support for more filter types in recovery mode
4. Implement recovery for other corruption types beyond streams

## Compatibility

- Backward compatible - default behavior unchanged
- Opt-in recovery through explicit options
- No breaking changes to existing API