# API Discrepancies Report

Generated from actual testing of oxidize-pdf public API.

## Critical Missing Methods

### Document API
- **`Document::to_bytes()`** - This method is used throughout the documentation and ISO compliance tests but DOES NOT EXIST
  - Impact: Cannot generate PDFs in memory, must save to file
  - Workaround: Use `save()` with a temporary file

- **`Document::set_compress()`** - Compression control is listed as implemented but NOT EXPOSED
  - Impact: Cannot control PDF compression

### Graphics API
- **`graphics.clip()`** - Clipping path operation DOES NOT EXIST
  - Impact: Cannot create clipping paths

- **`graphics.set_dash_pattern()`** - Method name mismatch
  - Actual method: `set_line_dash_pattern()`
  - Requires `LineDashPattern` struct, not array

### Text API
- **67% of text state methods NOT EXPOSED**:
  - `set_character_spacing()`
  - `set_word_spacing()`
  - `set_horizontal_scaling()`
  - `set_leading()`
  - `set_rendering_mode()`
  - `set_rise()`
  - Impact: Limited text formatting capabilities

### Font API
- **Font loading methods NOT EXPOSED**:
  - `Font::from_file()`
  - `Font::from_bytes()`
  - Impact: Cannot load custom fonts despite font embedding being "implemented"

## Actual vs Claimed Compliance

Based on API availability:
- **Claimed compliance**: 60-64% (from ISO_COMPLIANCE.md)
- **API availability**: 72.7% of documented methods exist
- **Estimated real compliance**: 25-30% (considering missing critical features)

## Key Findings

1. The most critical missing method is `Document::to_bytes()` which is used in almost all examples
2. Text formatting is severely limited with only basic positioning available
3. Font embedding exists internally but is not exposed through the API
4. Many advanced graphics features are structurally present but not functional
5. The API forces file I/O for all PDF generation (no in-memory generation)

## Recommendations

1. **Immediate**: Add `Document::to_bytes()` method or update all documentation
2. **High Priority**: Expose text formatting methods that already exist internally
3. **High Priority**: Expose font loading methods to enable custom fonts
4. **Medium Priority**: Add missing graphics operations like `clip()`
5. **Update Documentation**: Reflect actual API in all examples and tests
