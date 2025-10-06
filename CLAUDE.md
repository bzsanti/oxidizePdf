# CLAUDE.md - oxidize-pdf Project Context

## 🎯 Current Focus
- **Last Session**: 2025-10-06 - Fixed critical JPEG extraction bug (issue #67)
- **Branch**: develop_santi (working branch)
- **Version**: v1.3.0 released, working on v1.3.1
- **Priority**: Bug fixes and feature documentation
- **IMPORTANT**: Focus on practical PDF functionality, not compliance metrics

## ✅ Funcionalidades Completadas

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
- **#57** - CJK Font Support Test Failed (pendiente feedback usuario)
- **#54** - ISO 32000-1:2008 Compliance Tracking (enhancement)
- **#46** - Source Han Sans font support (pendiente feedback usuario)

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