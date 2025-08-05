# API Alignment Plan - Path to 60% ISO Compliance

This document outlines the implementation plan to align the oxidize-pdf API with the claimed 60% ISO 32000-1:2008 compliance.

## Current Status

- **Real API Compliance**: 17.8% (33 of 185 features tested)
- **Internal Implementation**: ~25-30% (features exist but not exposed)
- **Target**: 60% compliance for Community Edition

## Priority 1: Critical Missing Methods (Immediate)

These methods are used throughout documentation and tests but don't exist:

### 1.1 Document::to_bytes() - **CRITICAL**
```rust
impl Document {
    /// Generate PDF content as bytes (in-memory generation)
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        // Implementation needed
    }
}
```
**Impact**: Enables in-memory PDF generation, eliminating file I/O requirement  
**Effort**: Medium (2-3 days)  
**Files**: `document.rs`, expose existing internal serialization

### 1.2 Document::set_compress()
```rust
impl Document {
    /// Enable/disable compression for streams
    pub fn set_compress(&mut self, compress: bool) {
        self.compress = compress;
    }
}
```
**Impact**: Control over PDF file size  
**Effort**: Low (few hours)  
**Files**: `document.rs`, add field and propagate to writer

### 1.3 GraphicsContext::clip()
```rust
impl GraphicsContext {
    /// Create clipping path from current path
    pub fn clip(&mut self) -> &mut Self {
        self.write_op("W");
        self
    }
    
    /// Even-odd clipping
    pub fn clip_even_odd(&mut self) -> &mut Self {
        self.write_op("W*");
        self
    }
}
```
**Impact**: Essential graphics operation  
**Effort**: Low (1-2 hours)  
**Files**: `graphics/mod.rs`

## Priority 2: Expose Existing Internal Features

These features are implemented but not accessible via public API:

### 2.1 Text State Parameters
```rust
impl TextObject {
    pub fn set_character_spacing(&mut self, spacing: f64) -> &mut Self;
    pub fn set_word_spacing(&mut self, spacing: f64) -> &mut Self;
    pub fn set_horizontal_scaling(&mut self, scale: f64) -> &mut Self;
    pub fn set_leading(&mut self, leading: f64) -> &mut Self;
    pub fn set_rendering_mode(&mut self, mode: TextRenderingMode) -> &mut Self;
    pub fn set_rise(&mut self, rise: f64) -> &mut Self;
}
```
**Impact**: Enables advanced text formatting  
**Effort**: Low (1 day) - operators already implemented  
**Files**: `text/mod.rs`, expose existing operators

### 2.2 Font Loading
```rust
impl Font {
    /// Load font from file
    pub fn from_file(path: impl AsRef<Path>) -> Result<CustomFont>;
    
    /// Load font from bytes
    pub fn from_bytes(data: &[u8]) -> Result<CustomFont>;
}

pub struct CustomFont {
    // Wrapper around internal TrueTypeFont
}
```
**Impact**: Custom font support  
**Effort**: Medium (2-3 days) - need safe wrapper  
**Files**: `text/font.rs`, expose font embedding system

### 2.3 Filter Access
```rust
pub mod filters {
    pub use crate::parser::filters::{
        Filter, FilterType, 
        apply_filter, apply_filter_with_params
    };
}
```
**Impact**: Direct filter manipulation  
**Effort**: Low (few hours)  
**Files**: Re-export existing filter module

### 2.4 Encryption API
```rust
pub struct EncryptionOptions {
    pub user_password: Option<String>,
    pub owner_password: Option<String>,
    pub permissions: Permissions,
    pub algorithm: EncryptionAlgorithm,
}

impl Document {
    pub fn set_encryption(&mut self, options: EncryptionOptions) -> Result<()>;
}
```
**Impact**: Security features  
**Effort**: Medium (3-4 days) - need safe API design  
**Files**: Create public wrapper for encryption system

## Priority 3: Complete Partially Implemented Features

### 3.1 Image Support
```rust
pub enum ImageFormat {
    Jpeg,
    Png,  // New
    Tiff, // New
}

impl Image {
    pub fn from_png_file(path: impl AsRef<Path>) -> Result<Self>;
    pub fn from_png_bytes(data: &[u8]) -> Result<Self>;
}
```
**Impact**: Broader image format support  
**Effort**: High (1 week) - PNG decoder needed  
**Files**: `image/mod.rs`, add PNG support

### 3.2 Pattern Support
```rust
impl GraphicsContext {
    pub fn set_pattern(&mut self, pattern: &Pattern) -> &mut Self;
}

pub struct Pattern {
    pub fn tiling(/* params */) -> Self;
    pub fn shading(/* params */) -> Self;
}
```
**Impact**: Advanced graphics  
**Effort**: High (1 week)  
**Files**: Complete pattern implementation

### 3.3 Advanced Color Spaces
```rust
pub enum ColorSpace {
    DeviceGray,
    DeviceRGB,
    DeviceCMYK,
    CalGray { /* params */ },    // New
    CalRGB { /* params */ },     // New
    ICCBased { profile: Vec<u8> }, // New
}
```
**Impact**: Professional color management  
**Effort**: High (1 week)  
**Files**: Extend color system

## Implementation Timeline

### Phase 1: Quick Wins (1 week)
- [ ] Document::to_bytes()
- [ ] Document::set_compress()
- [ ] GraphicsContext::clip()
- [ ] Text state parameters
- [ ] Method name fixes

**Expected compliance gain**: +8-10%

### Phase 2: Expose Internals (2 weeks)
- [ ] Font loading API
- [ ] Filter access
- [ ] Basic encryption API
- [ ] Extended graphics state access

**Expected compliance gain**: +10-12%

### Phase 3: Feature Completion (4 weeks)
- [ ] PNG image support
- [ ] Pattern implementation
- [ ] Advanced color spaces
- [ ] Form XObjects
- [ ] Basic annotations

**Expected compliance gain**: +15-20%

### Total Expected Outcome
- **Starting**: 17.8%
- **After Phase 1**: ~26-28%
- **After Phase 2**: ~36-40%
- **After Phase 3**: ~51-60%

## Testing Strategy

1. **Update all tests** to use real API methods
2. **Add integration tests** for each new API
3. **Update examples** to showcase new features
4. **Run compliance test** after each phase
5. **Update documentation** continuously

## Breaking Changes

Some changes may require API modifications:

1. **Font enum** → **Font trait** to support custom fonts
2. **Color** struct may need variants for advanced spaces
3. **Graphics/Text** objects may need lifetime parameters

## Success Criteria

- [ ] ISO compliance test shows ≥60% for exposed API
- [ ] All documentation examples compile and run
- [ ] No regression in existing functionality
- [ ] All new APIs have comprehensive tests
- [ ] Performance remains acceptable

## Next Steps

1. Create feature branch `feature/api-alignment`
2. Implement Phase 1 (quick wins)
3. Run compliance tests and verify gains
4. Proceed with Phase 2 if on track
5. Regular progress updates in CHANGELOG

This plan provides a realistic path to achieving the claimed 60% ISO compliance through exposing existing functionality and completing partial implementations.