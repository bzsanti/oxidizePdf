# CLAUDE.md - oxidize-pdf Project Context

## 🎯 Current Focus
- **Last Session**: 2025-10-07 - Feature 2.2.1 Object Streams COMPLETE
- **Branch**: develop_santi (working branch)
- **Version**: v1.3.0 released, working on v1.4.0 (Sprint 2.2)
- **Priority**: **ISO Core Fundamentals (Sprint 2.2)** - ✅ Object Streams | XRef Streams | LZWDecode
- **Progress**: Feature 2.2.1 Object Streams integrated and tested
- **Target**: 35-40% → 60-65% ISO compliance (on track)

## ✅ Funcionalidades Completadas

### 🗜️ **Feature 2.2.1: Object Streams** (Sesión 2025-10-07) ⭐ NEW
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

### 📊 **Gap Analysis & Roadmap** (Sesión 2025-10-06 - Tarde)
- ✅ **Gap Analysis vs lopdf**: Documento completo en `.private/GAP_ANALYSIS_LOPDF.md`
- ✅ **Gaps Críticos Identificados** (P0):
  - Object Streams (11-61% file size reduction)
  - Cross-Reference Streams (PDF 1.5+ compliance)
  - LZWDecode (legacy PDF compatibility)
- ✅ **Ventajas Confirmadas**:
  - Encryption: COMPLETO (RC4, AES-128/256, Public Key)
  - CJK, Transparency, Annotations, Forms: Superiores a lopdf
- ✅ **Roadmap Actualizado**:
  - ROADMAP_MASTER.md con Sprint 2.2 ISO Core Fundamentals
  - Sprint 2.2 detallado: 3 features P0/P1 en 3 semanas
  - Sprint 2.3 planeado: Tagged PDF + Incremental Updates

### 📈 **Reporting Avanzado** (COMPLETADO)
- ✅ Dashboards dinámicos con múltiples visualizaciones
- ✅ KPI cards y métricas clave
- ✅ Tablas pivote con agregaciones
- ✅ Gráficos avanzados (heatmaps, treemaps, scatter plots)
- ✅ Bar charts, line charts, pie charts
- ✅ Sistema de layout y componentes
- ✅ Temas y personalización

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
- **Tests**: 4,163 total tests in workspace (all passing)
- **PDF Parsing**: 98.8% success rate (750/759 PDFs) - 42.6 PDFs/second
- **Performance**: ~12,000 pages/second for simple content (realistic measurement)
- **Testing Focus**: Functional testing with honest benchmarks
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
  - Gap analysis completado 2025-10-06
  - Sprint 2.2 planeado para cerrar P0/P1 gaps
- **#46** - Source Han Sans font support (pendiente feedback usuario - 7 días)

## 🎯 Próximas Prioridades (Sprint 2.2)
1. **Feature 2.2.1**: Object Streams (3 días) - File size parity con lopdf
2. **Feature 2.2.2**: Cross-Reference Streams (3 días) - PDF 1.5+ compliance
3. **Feature 2.2.3**: LZWDecode (2 días) - Legacy PDF compatibility

**Objetivo**: 35-40% → 60-65% ISO compliance en 3 semanas

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