# CLAUDE.md - oxidize-pdf Project Context

## 🎯 Current Focus
- **Branch**: develop_santi (working branch)
- **Version**: **v1.6.3 (pending release)** 🚀
- **Status**: Sprint 2.2 Complete (3/3 features shipped in 26% of estimated time)
- **Quality Metrics**:
  - Tests: 4633 passing (all green)
  - Clippy: Clean (0 warnings)
  - Zero Unwraps: 100% library code compliance
  - Documentation: 100% rustdoc coverage
- **Next**: v1.6.3 release → Sprint 2.3 planning

## ✅ Features Completadas (v1.6.x)

| Feature | Version | Location | Status | Docs |
|---------|---------|----------|--------|------|
| **Structured Data Extraction** | v1.6.3 | `oxidize-pdf-core/src/text/structured/` | ✅ Shipped | rustdoc (41 tests) |
| **Plain Text Optimization** | v1.6.3 | `oxidize-pdf-core/src/text/plaintext/` | ✅ Shipped | rustdoc (23 tests) |
| **Invoice Data Extraction** | v1.6.3 | `oxidize-pdf-core/src/text/invoice/` | ✅ Shipped | `INVOICE_EXTRACTION_GUIDE.md` (23 tests) |
| **Unwrap Elimination Campaign** | v1.6.2 | Workspace-wide | ✅ Complete | `LINTS.md` (51 unwraps eliminated) |
| **Kerning Normalization** | v1.6.1 | `src/text/extraction_cmap.rs` | ✅ Complete | rustdoc (9 tests) |
| **Dylint Custom Lints** | v1.6.1 | `lints/` workspace | ✅ Operational | `LINTS.md` (5 production lints) |
| **LLM-Optimized Formats** | v1.6.0 | `oxidize-pdf-core/src/ai/formats.rs` | ✅ Released | README (MD/JSON/TXT export) |
| **ISO Core Fundamentals** | v1.5.0 | Multiple modules | ✅ Complete | Object Streams, XRef Streams, LZW |

**Implementation details**: See git history (`git log --grep="<feature>"`) for commits and code changes.

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
- **Treat all warnings as errors** (clippy + rustc)
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
cargo build --release                # Production build
./verify_pdf_compatibility.sh        # Check PDF parsing
```

### Custom Slash Commands
- `/start-session rust` - Initialize development session with Rust context
- `/gitflow-feature <name>` - Create feature branch from develop
- `/end-session` - Run tests, commit, push, update issues

## 📊 Current State
- **PDF Features**: Core features implemented and documented
- **Tests**: 4,633 total tests in workspace (all passing)
- **Test Coverage**: 55.64% (17,708/31,827 lines) - Measured with Tarpaulin
- **PDF Parsing**: 98.8% success rate (750/759 PDFs) - 42.6 PDFs/second
- **Performance** (Realistic Benchmarks - 2025-10-07):
  - **Realistic Content**: 5,500-6,034 pages/second (varied paragraphs + tables + charts)
  - **Medium Complexity**: 2,214 pages/second (gradient charts + sparklines + tables)
  - **High Complexity**: 3,024 pages/second (Bezier diagrams + code blocks + shadows)
  - **Details**: See `BENCHMARK_RESULTS.md`
- **Code Quality**: Zero unwraps in library code (100% compliance)
- **ISO Compliance**: 55-60% (honest assessment from gap analysis)
- **Last Build**: ✅ All tests passing, clippy clean, formatted

## 📚 Documentation References
- **Architecture**: `docs/ARCHITECTURE.md`
- **Invoice Extraction**: `docs/INVOICE_EXTRACTION_GUIDE.md`
- **Lints**: `docs/LINTS.md`
- **Roadmap**: `.private/ROADMAP_MASTER.md`
- **History**: Use `git log` for detailed commit history

## ⚠️ Known Issues & Limitations

### Technical Limitations (Documented)
- **Invoice `use_kerning` flag** (2025-10-21) - Stored but not yet functional
  - **Status**: Documented as "PLANNED for v2.0"
  - **Blocker**: `TextFragment` lacks font metadata (requires breaking change)
  - **Impact**: LOW - Invoice patterns work without kerning-aware spacing
  - **Location**: `oxidize-pdf-core/src/text/invoice/extractor.rs:88-103`
  - **Tests**: 18 passing (including storage verification)
  - **Docs**: Complete rustdoc with architectural explanation

### Non-Critical Issues
- PNG compression tests (7 failures) - non-critical
- Encrypted PDFs not supported (19 cases)
- Some circular references in complex PDFs

## 📝 Open GitHub Issues

- **#90** - **OPEN** - Advanced Text Extraction with Table Detection (2025-10-16)
  - ✅ Gap analysis completed with competitor comparison
  - **Phase 1**: Font metadata exposure in TextFragment
  - **Phase 2**: Vector graphics line extraction
  - **Phase 3**: Table detection with border-based cell assignment
  - **Status**: Ready for prioritization
  - **Estimated effort**: 28-42 hours (3.5-5 days)

- **#87** - **OPEN** - Kerning Normalization (2025-10-16)
  - ✅ All 3 phases completed (font metrics, TrueType kerning, Type1 documentation)
  - ✅ 9 rigorous tests passing
  - **Action**: Close issue with documentation summary

- **#54** - **OPEN** - ISO 32000-1:2008 Compliance Tracking (2025-10-13)
  - ✅ Honest gap analysis completed 2025-10-07
  - **Finding**: 55-60% compliance (not 35-40%)
  - Sprint 2.2 features verified as already implemented
  - **Action**: Update issue with honest assessment

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
