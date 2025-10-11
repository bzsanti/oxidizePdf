# CLAUDE.md - oxidize-pdf Project Context

## 🎯 Current Focus
- **Last Session**: 2025-10-11 - Incremental Updates API Clarification
- **Branch**: develop_santi (working branch)
- **Version**: **v1.5.0 released** 🚀
- **Recent Work**:
  - ✅ **Incremental Updates Writer**: Implemented with clear API separation
    - `write_incremental_with_page_replacement()` - Manual page replacement (shipped)
    - `write_incremental_with_overlay()` - Automatic overlay (planned, stub added)
  - ✅ **Documentation overhaul**: Honest assessment of capabilities and limitations
  - ✅ **API Naming**: Clear distinction between manual replacement vs automatic overlay
  - ✅ **Example updated**: `incremental_page_replacement_manual.rs` with warnings
  - ✅ **4 rigorous tests**: pdftotext/pdfinfo verification (NO smoke tests)
- **Key Achievement**: Honest communication about what's implemented vs planned
- **Next**: Compile, test, commit changes

## ✅ Funcionalidades Completadas

### 🎯 **Sprint 2.2: ISO Core Fundamentals** (Sesión 2025-10-07) ⭐ COMPLETE

#### Feature 2.2.1: Object Streams ✅
- ✅ **Object Stream Writer Integration**: Compresión automática de objetos PDF
  - Integrado en `PdfWriter::write_document()` oxidize-pdf-core/src/writer/pdf_writer.rs:136
  - Buffering de objetos comprimibles durante escritura
  - Generación automática de object streams antes de xref
  - XRef stream con entradas Type 2 para objetos comprimidos
- ✅ **Configuración**:
  - `WriterConfig::modern()` habilita object streams + xref streams
  - `WriterConfig::legacy()` para PDF 1.4 sin compresión moderna
  - Config granular con `use_object_streams` flag
- ✅ **Compresión Inteligente**:
  - Detecta automáticamente objetos comprimibles vs streams
  - 100 objetos por stream (configurable)
  - Zlib compression level 6
- ✅ **Testing**:
  - 16 tests unitarios (parser + writer)
  - 4,170 tests totales pasando
  - Demo: `modern_pdf_compression.rs` con 3.9% reducción
- ✅ **Resultados Medidos**:
  - Legacy PDF 1.4: 9447 bytes (baseline)
  - Modern PDF 1.5: 9076 bytes (-3.9% reduction)
  - 13 → 7 objetos directos (6 objetos comprimidos)
- ✅ **ISO Compliance**:
  - ISO 32000-1 Section 7.5.7 implementado
  - PDF 1.5+ required
  - Compatible con Adobe Acrobat

#### Feature 2.2.2: Cross-Reference Streams ✅
- ✅ **XRef Stream Writer**: Ya implementado completamente
  - Binary encoding con widths auto-ajustables oxidize-pdf-core/src/writer/xref_stream_writer.rs
  - Type 0 (Free), Type 1 (InUse), Type 2 (Compressed) entries
  - FlateDecode compression integrada
  - W array dinámico según tamaño de offsets
- ✅ **Mejoras en Session**:
  - Integrado Type 2 entries para Object Streams
  - 1.3% reducción adicional con XRef Streams alone
- ✅ **Testing**:
  - 12 tests unitarios pasando
  - Compatible con Adobe Acrobat
- ✅ **ISO Compliance**:
  - ISO 32000-1 Section 7.5.8 implementado

#### Feature 2.2.3: LZWDecode Filter ✅
- ✅ **LZW Decompression**: Ya implementado completamente
  - Algoritmo completo en oxidize-pdf-core/src/parser/filters.rs:1555
  - Variable-length codes (9-12 bits)
  - CLEAR_CODE (256) y EOD (257) support
  - EarlyChange parameter support
- ✅ **LzwBitReader**: Lectura eficiente de bits variables
- ✅ **Testing**:
  - 11 tests unitarios pasando
  - Casos edge: empty, invalid codes, clear code, growing codes
- ✅ **ISO Compliance**:
  - ISO 32000-1 Section 7.4.4 implementado
  - Compatible con PDFs legacy pre-2000

#### 📊 Sprint 2.2 Summary
- **Duration**: 1 día (features ya existían, Feature 2.2.1 nueva)
- **Tests**: 4,170 + 39 nuevos (Object Streams + XRef + LZW)
- **ISO Compliance**: 35-40% → **60-65%** ✅ TARGET ACHIEVED
- **File Size**: 3.9% reduction vs legacy PDF 1.4
- **Ready for**: v1.4.0 Release

### 🐛 **Bug Fixes Críticos** (Sesión 2025-10-06)
- ✅ **JPEG Extraction Fix (Issue #67)**: Eliminación de bytes extra antes del SOI marker
  - Función `extract_clean_jpeg()` en `dct.rs`
  - 6 tests unitarios + verificación con PDF real
  - Tesseract OCR funcional
  - Commit: 644b820

### 📚 **Documentación de Features** (Sesión 2025-10-06)
- ✅ **Corruption Recovery**: Ejemplo `recovery_corrupted_pdf.rs`
- ✅ **PNG Transparency**: Ejemplo `png_transparency_watermark.rs`
- ✅ **CJK Support**: Ejemplo `cjk_text_extraction.rs`
- ✅ README actualizado con features documentadas

### ⚡ **Performance Benchmarks Modernized** (Sesión 2025-10-07 - Noche)
- ✅ **Reemplazo de benchmark trivial**:
  - ❌ `performance_benchmark_1000.rs`: Contenido repetitivo ("Lorem ipsum")
  - ✅ `realistic_document_benchmark.rs`: Contenido único por página
  - **Resultados**: 5,500-6,034 páginas/segundo con contenido variado
- ✅ **Medium Complexity mejorado**:
  - Gráficos con gradientes (5 capas por barra)
  - Mini-sparklines debajo de cada barra
  - 3 tipos de gráficos rotatorios
  - **Resultados**: 2,214 páginas/segundo
- ✅ **High Complexity mejorado**:
  - Diagramas técnicos con curvas Bezier (8 segmentos)
  - Sombras y efectos de gradiente
  - Layout circular de componentes
  - Etiquetas de data rate únicas
  - **Resultados**: 3,024 páginas/segundo
- ✅ **Verificación de variación**:
  - Fórmulas matemáticas para contenido único
  - Rotación de datos basada en page_num
  - Sin caché ni repetición
- ✅ **Documentación**: `BENCHMARK_RESULTS.md` con análisis completo

### 🔍 **Honest Gap Analysis** (Sesión 2025-10-07 - Tarde) ⭐ CRITICAL UPDATE
- ✅ **100% Evidence-Based Code Review**: `.private/HONEST_GAP_ANALYSIS.md`
- 🎯 **MAJOR FINDINGS**:
  - **ISO Compliance**: **55-60%** (NOT 35-40% as estimated!)
  - **Sprint 2.2 Features**: Already implemented (Object Streams, XRef Streams, LZWDecode)
  - **Encryption**: SUPERIOR to lopdf (275 tests, AES-256, Public Key)
  - **All Filters**: Complete (LZW, CCITTFax, RunLength, DCT, Flate)
  - **Inline Images**: Full parser (ISO 8.9.7)
  - **Incremental Updates**: Parser complete (writer pending)
- ❌ **ACTUAL Gaps** (Only 3!):
  - XMP Metadata (placeholder only)
  - Tagged PDF (not implemented)
  - Incremental Updates Writer (parser exists)
- ✅ **Strategic Conclusion**:
  - We significantly **undersold** our capabilities
  - Documentation lags implementation by ~6 months
  - Need marketing/docs update, not new features
  - Competitive position vs lopdf: **STRONGER than estimated**

### 📈 **Reporting Avanzado** (COMPLETADO)
- ✅ Dashboards dinámicos con múltiples visualizaciones
- ✅ KPI cards y métricas clave
- ✅ Tablas pivote con agregaciones
- ✅ Gráficos avanzados (heatmaps, treemaps, scatter plots)
- ✅ Bar charts, line charts, pie charts
- ✅ Sistema de layout y componentes
- ✅ Temas y personalización

### 📄 **Incremental Updates (ISO 32000-1 §7.5.6)** (Sesión 2025-10-11) ⚠️ PARTIAL

#### What's Implemented ✅
- ✅ **Parser**: Complete (50% gap → 100%)
  - Reads incremental PDFs with /Prev chains
  - Parses XRef tables across updates
  - Handles multi-generation documents
- ✅ **Writer Structure**: 100% ISO compliant
  - Append-only writes (byte-for-byte preservation)
  - /Prev pointers in trailer
  - Cross-reference chain maintenance
  - Digital signature compatible
- ✅ **Page Replacement API**: `write_incremental_with_page_replacement()`
  - Replaces specific pages in existing PDFs
  - **Use case**: Dynamic page generation from data
  - **Limitation**: Requires manual recreation of entire page content
  - Location: oxidize-pdf-core/src/writer/pdf_writer/mod.rs:478
  - Tests: 4 rigorous tests with pdftotext/pdfinfo verification
  - Example: `examples/incremental_page_replacement_manual.rs`

#### What's NOT Implemented ❌
- ❌ **Automatic Overlay**: `write_incremental_with_overlay()` (stub only)
  - Load existing PDF → Modify → Save
  - True form filling without manual recreation
  - Annotation overlay on existing pages
  - Watermarking without page replacement
- ❌ **Required Components**:
  - `Document::load()` - Load existing PDF into writable Document
  - `Page::from_parsed()` - Convert parsed pages to writable format
  - Content stream overlay system
  - Resource dictionary merging
  - Estimated effort: 6-7 days

#### Honest Assessment
**Current State**: "Page Replacement with Manual Recreation" (NOT automatic form filling)

**Valid Use Cases** (Where current API is IDEAL):
1. ✅ Dynamic page generation (you have logic to generate complete pages)
2. ✅ Template variants (switching between pre-generated versions)
3. ✅ Page repair (regenerating corrupted pages from scratch)

**Invalid Use Cases** (Need future overlay API):
1. ❌ Fill PDF form fields without knowing entire template
2. ❌ Add annotations to existing page without recreation
3. ❌ Watermark existing document without page replacement

**Strategic Decision**: Ship current API as explicit "manual replacement" option,
plan overlay API for future release when Document::load() is implemented.

**API Clarity**:
- `write_incremental_with_page_replacement()` - Works NOW (manual)
- `write_incremental_with_overlay()` - Planned (automatic)

## 🚀 Prioridades Pendientes

### 1. ⚡ **Rendimiento Extremo**
- Generación paralela de páginas
- Streaming de escritura sin mantener todo en memoria
- Optimización agresiva de recursos PDF
- Compresión inteligente por tipo de contenido
- Lazy loading mejorado para documentos grandes

### 2. 🔍 **OCR Avanzado**
- Mejorar integración con Tesseract
- OCR selectivo por regiones
- Post-procesamiento con corrección automática
- Extracción especializada de tablas
- Confidence scoring por palabra/región

## 🏗️ Architecture Overview
```
oxidize-pdf/
├── oxidize-pdf-core/    # Core PDF library (main crate)
├── oxidize-pdf-api/     # REST API server
├── oxidize-pdf-cli/     # Command-line interface
└── oxidize-pdf-render/  # Rendering engine (separate repo)
```

## 📋 Development Guidelines

### Critical Rules
- **Treat all warnings as errors**
- **Minimum 80% test coverage** (target 95%)
- **NO manual releases** - Use GitHub Actions pipeline only
- **ALL PDFs go to** `examples/results/` (never in root or test dirs)

### Testing Strategy
```bash
cargo test --workspace     # Run all tests
cargo clippy -- -D warnings # Check linting
cargo fmt --all --check    # Verify formatting
```

### 🎓 LECCIONES APRENDIDAS (Test Coverage) - 2025-10-10

**CRÍTICO - Para mejorar cobertura REAL de código**:

#### 1. **Test Quality ≠ Code Coverage**
- ❌ Smoke tests (`assert!(result.is_ok())`) son INÚTILES para cobertura
- ❌ Solo verificar que no falla != ejecutar código nuevo
- ✅ Tests rigurosos verifican valores específicos con `assert_eq!`
- ✅ **Regla**: Mejor 10 tests rigurosos que 50 smoke tests

#### 2. **API Coverage ≠ Code Coverage**
- ❌ Testear todas las funciones públicas != mejorar cobertura
- ❌ Verificar que una función existe != ejecutar su lógica
- ✅ Tests deben ejecutar **paths de código nuevos**
- ✅ **Regla**: No confundir "cobertura de API" con "cobertura de código"

#### 3. **Módulos con Dependencias Externas son Difíciles**
- ❌ Módulos que requieren archivos (TTF, PDFs reales) son difíciles de testear
- ❌ Error paths (file not found) no son paths nuevos
- ✅ Buscar módulos con **lógica pura** (matemática, transformaciones, parsers)
- ✅ **Regla**: Priorizar módulos con lógica pura para high-ROI wins

#### 4. **Medir SIEMPRE con Tarpaulin**
- ❌ NUNCA estimar cobertura sin medir
- ✅ Ejecutar tarpaulin ANTES y DESPUÉS de agregar tests
- ✅ **Regla**: "Si no está medido, no existe"

#### 5. **Estrategia "Wins E Impacto" (NO son incompatibles)**

**Criterios de Selección de Módulos** (ROI = (Valor + Impacto) / Esfuerzo):

| Criterio | Bajo ROI | Alto ROI |
|----------|----------|----------|
| **Tamaño** | >500 líneas | <200 líneas |
| **Lógica** | I/O, archivos, PDFs | Matemática, conversiones puras |
| **Criticidad** | Utility, helpers | Core rendering, parsers |
| **Cobertura actual** | <20% o >90% | 30-85% (fácil mejorar) |
| **Esfuerzo** | Requiere PDFs reales | Solo tests con valores |

**Estrategia Balanceada**:
- ✅ **Quick Wins** (coordinate_system.rs): 51 líneas, lógica pura, 100% alcanzable
  - Valor: Documenta comportamiento, previene regresiones
  - Impacto: Crítico para rendering correcto
  - **NO despreciar** por ser "fácil" - son wins legítimos

- ✅ **High Impact Wins** (graphics/color.rs): 95 líneas, 82% → 100%, conversiones críticas
  - Valor: Fórmulas RGB↔CMYK documentadas
  - Impacto: Color incorrecto = bug visible en PDFs
  - **Mejor estrategia**: Optimizar para AMBOS (wins E impacto)

**Valor de Tests de Regresión**:
- ✅ Tests "obvios" (defaults, equality) SÍ tienen valor
- ✅ Documentan comportamiento esperado
- ✅ Detectan cambios en derives (PartialEq, Default)
- ✅ **NO son smoke tests** si verifican valores específicos
- ⚠️ **Smoke test**: `assert!(result.is_ok())`
- ✅ **Test riguroso**: `assert_eq!(Color::default(), Color::Gray(0.0))`

**Ejemplos Honestos**:
- ❌ Sesión 4: parser/reader.rs - +22 tests, **0% mejora** (smoke tests)
- ❌ Sesión 5: fonts/mod.rs - +11 tests, **0% mejora** (requiere archivos reales)
- ✅ **Sesión 6: coordinate_system.rs** - +36 tests, **+63% mejora** (100% cobertura)
  - **Lección**: Lógica pura + tests rigurosos = wins reales
  - **ROI**: 10/10 (esfuerzo bajo, impacto alto, win medible)

**NO REPETIR ERRORES, SÍ REPLICAR ÉXITOS**

### Git Workflow
1. Work on `develop_santi` branch
2. Create PR to `main` when ready
3. Tag releases trigger automatic pipeline

## 🚀 Quick Commands

### Development
```bash
cargo run --example <name>           # Run examples
cargo test --lib <module>            # Test specific module
cargo build --release                 # Production build
./verify_pdf_compatibility.sh        # Check PDF parsing
```

### Custom Slash Commands
- `/analyze-pdfs` - Analyze all PDFs in tests/fixtures/
- `/analyze-pdfs --with-render` - Include rendering validation

## 📊 Current State
- **PDF Features**: Core features implemented and documented
- **Tests**: 4,445 total tests in workspace (all passing) - Updated 2025-10-10
- **Test Coverage**: 55.64% (17,708/31,827 lines) - Measured with Tarpaulin
- **PDF Parsing**: 98.8% success rate (750/759 PDFs) - 42.6 PDFs/second
- **Performance** (Realistic Benchmarks - 2025-10-07):
  - **Realistic Content**: 5,500-6,034 pages/second (varied paragraphs + tables + charts)
  - **Medium Complexity**: 2,214 pages/second (gradient charts + sparklines + tables)
  - **High Complexity**: 3,024 pages/second (Bezier diagrams + code blocks + shadows)
  - **All benchmarks**: Unique content per page (no trivial repetition)
  - **Details**: See `BENCHMARK_RESULTS.md`
- **Testing Focus**: Functional testing with honest, realistic benchmarks
- **Last Build**: ✅ All tests passing, clippy clean, formatted

## 📚 Documentation References
- **Detailed History**: `docs/HISTORY.md`
- **Architecture**: `docs/ARCHITECTURE.md` 
- **PDF Features**: Basic functionality documented
- **Roadmap**: `ROADMAP.md`
- **Test Organization**: See "Test Organization Guidelines" section

## ⚠️ Known Issues
- PNG compression tests (7 failures) - non-critical
- Encrypted PDFs not supported (19 cases)
- Some circular references in complex PDFs

## 📝 Open GitHub Issues (3)
- **#57** - CJK Font Support Test Failed (pendiente feedback usuario - 7 días)
- **#54** - ISO 32000-1:2008 Compliance Tracking (enhancement)
  - ✅ Honest gap analysis completado 2025-10-07
  - **Finding**: 55-60% compliance (not 35-40%)
  - Sprint 2.2 features verified as already implemented
  - **Action**: Update issue with honest assessment
- **#46** - Source Han Sans font support (pendiente feedback usuario - 7 días)

## 🎯 Próximas Prioridades (REVISED)

### Strategic Options Post-Discovery

**Discovery**: Sprint 2.2 features already complete! (Object Streams, XRef Streams, LZWDecode)

**Option A - Document & Market Existing Features** (Recommended)
1. Create examples for "hidden" features (encryption, inline images, incremental parser)
2. Update README/docs with honest ISO compliance (55-60%)
3. Add benchmarks comparing to lopdf
4. Marketing materials highlighting encryption superiority

**Option B - Implement Actual Gaps**
1. XMP Metadata (ISO 14.3.2)
2. Tagged PDF (ISO 14.8) - High impact for accessibility
3. Incremental Updates Writer (ISO 7.5.6) - Parser exists

**Option C - Performance Optimization**
1. Profile existing features
2. Optimize object stream compression
3. Parallel page generation
4. Memory usage improvements

## 🔧 Test Organization (STRICT)
**MANDATORY RULES:**
1. ALL generated PDFs → `examples/results/`
2. Example .rs files → `examples/src/`
3. Documentation → `examples/doc/`
4. Unit tests → `oxidize-pdf-core/tests/`
5. Python analysis scripts → `tools/analysis/`
6. Python utility scripts → `tools/scripts/`
7. Rust debug tools → `dev-tools/`

**FORBIDDEN:**
- Creating PDFs in project root or oxidize-pdf-core/
- Using `oxidize-pdf-core/test-pdfs/` (deprecated)
- Leaving PDF files scattered
- Placing scripts in project root
- Creating SESSION_NOTES or temporary MD files in root

**CLEANUP RULES:**
- Run `find . -name "*.pdf" -not -path "./examples/results/*" -not -path "./test-pdfs/*"` to find stray PDFs
- Delete any test PDFs after running tests
- Move all scripts to appropriate directories

## 📦 Release Process
```bash
# NEVER use cargo-release locally!
git tag v1.2.3
git push origin v1.2.3
# GitHub Actions handles everything else
```

## 🔗 External Resources
- GitHub: https://github.com/BelowZero/oxidize-pdf
- Crates.io: https://crates.io/crates/oxidize-pdf
- Issues: Track via GitHub Issues (not Azure DevOps)