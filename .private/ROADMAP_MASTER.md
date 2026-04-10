# ROADMAP MASTER - oxidize-pdf
**Ultima actualizacion**: 2026-04-05 (cleanup: feature/pdf-editor borrada, rag-aligned-editing actualizada con develop)
**Horizonte**: Sin deadline — roadmap es orientativo, no vinculante
**Owner**: bzsanti
**Repositorio**: https://github.com/bzsanti/oxidizePdf

---

## ESTADO ACTUAL

### Posicion en Roadmap
- **Version**: v2.4.3 released (tag v2.4.3 en main)
- **Sprint Actual**: CID CFF font subsetting fix (#165)
- **Tests**: 6,261 passing (lib), clippy limpio
- **Coverage**: 72.14%
- **Quality Grade**: A (95/100)
- **PDF Success Rate**: 99.3% (275/277 failure corpus)

### Metricas Actuales
- **Downloads**: ~10K+ total (estimado)
- **Stars**: 20+
- **Test coverage**: 72.14% (objetivo: 80%)
- **Tests**: 8,070 passing
- **Licencia**: MIT (cambiada desde AGPL-3.0 — monetizacion ya no es objetivo)
- **ISO compliance**: 310 requisitos curados, 100% linked to code (66.8% alta verificacion)

---

## RELEASES PUBLICADOS

| Version | Fecha | Highlights |
|---------|-------|------------|
| v2.4.3 | 2026-04-04 | CID CFF font subsetting fix (#165): raw CFF embedding, Private DICT Subrs patching, glyph mapping filter |
| v2.4.2 | 2026-04-02 | TTF tittle offset fix, TextFlow width (#167), PNG rendering (#174) |
| v2.4.1 | 2026-03-29 | 14 quality fixes (CFF scanner, overflow protection, measure_text &Font, cmap consolidation, PNG fix, TextFlow width), table system overhaul, CID font subsetting |
| v2.4.0 | 2026-03-27 | CFF font subsetting, cmap Format 12, multilingual corpus tests |
| v2.3.4 | 2026-03-25 | Table improvements (#162, #163), CJK table font fix (#160) |
| v2.3.3 | 2026-03-21 | CJK CID→Unicode tables (79K entries, 4 collections), SMask remap fix (#156, #157) |
| v2.3.2 | 2026-03-15 | Overlay/watermark (OPS-005), Image::from_file, reorder exports, XObject stream fix |
| v2.3.0 | 2026-03-14 | RagChunk API — one-liner RAG pipeline with serializable metadata |
| v2.2.0 | 2026-03-14 | Encryption-on-write (RC4+AES-128+AES-256), pipeline profiling, ElementGraph, HybridChunker v2 |
| v2.1.0 | 2026-03-09 | Pipeline Architecture v2: ElementGraph, HybridChunker v2, table detection improvements |
| v1.8.0 | 2026-03-01 | JBIG2 decoder completo, float sort fix (Rust 1.81+) |
| v1.7.1 | 2026-02-18 | Fix CFF fonts CIDToGIDMap (Issue #127) |
| v1.7.0 | 2026-02-16 | Digital Signatures, PDF/A Validation, BER-to-DER, Issue #128 fix |
| v1.6.13 | 2026-02-12 | Dependency updates (rand 0.10, quick-xml 0.39, criterion 0.8) |
| v1.6.12 | 2026-02-07 | Generic ImageExtractor (PR #121), +234 tests |
| v1.6.11 | 2026-02-01 | Per-page extraction options (Issue #116) |
| v1.6.10 | 2026-01-29 | space_threshold tuning (0.2→0.3) |
| v1.6.9 | 2026-01-28 | Text sanitization NUL bytes fix (Issue #116) |
| v1.6.8 | 2026-01-10 | Cargo.toml version desync fix, test fixtures |
| v1.6.7 | 2026-01-02 | Type0 security hardening, encryption improvements |

---

## FEATURES COMPLETADAS

### v1.8.x
| Feature | Version | Estado |
|---------|---------|--------|
| JBIG2 Decoder (9 fases, pure Rust) | v1.8.0 | Shipped |
| Float sort panic fix (Rust 1.81+) | v1.8.0 | Shipped |
| 7-tier corpus test infrastructure | WIP | En rama feature |
| Decompression bomb protection | WIP | En rama feature |

### v1.7.x
| Feature | Version | Estado |
|---------|---------|--------|
| Digital Signatures API (6 fases) | v1.7.0 | Shipped |
| PDF/A Validation module | v1.7.0 | Shipped |
| BER-to-DER security hardening | v1.7.0 | Shipped |
| CFF fonts CIDToGIDMap fix | v1.7.1 | Shipped |
| merge_pdfs blank PDF fix (Issue #128) | v1.7.0 | Shipped |
| text_wrap implementation (Issue #131) | v1.7.0 | Shipped |

### v1.6.x
| Feature | Version | Estado |
|---------|---------|--------|
| AES-256 R5/R6 encryption (Algorithm 2.B) | v1.6.8+ | Shipped |
| Owner password R5/R6 support | v1.6.8+ | Shipped |
| Cross-validation pypdf | v1.6.8+ | Shipped |
| CID/Type0 font embedding | v1.6.8+ | Shipped |
| Font subsetting large fonts (Issue #115) | v1.6.9 | Shipped |
| Text sanitization NUL bytes (Issue #116) | v1.6.9 | Shipped |
| Per-page extraction options | v1.6.11 | Shipped |
| Generic ImageExtractor | v1.6.12 | Shipped |
| Dependency updates | v1.6.13 | Shipped |
| BDC inline dict parsing fix | v1.6.6 | Shipped |
| XRef stream double-decode fix | v1.6.6 | Shipped |
| TextContext used_characters fix | v1.6.5 | Shipped |
| UTF-8 Panic Fix | v1.6.4 | Shipped |
| Table Detection | v1.6.4 | Shipped |
| Structured Data Extraction | v1.6.3 | Shipped |
| Plain Text Optimization | v1.6.3 | Shipped |
| Invoice Data Extraction | v1.6.3 | Shipped |
| Unwrap Elimination (51) | v1.6.2 | Shipped |
| Kerning Normalization | v1.6.1 | Shipped |
| LLM-Optimized Formats | v1.6.0 | Shipped |

---

## ISSUES GITHUB

### Abiertos
- **#160** - CJK font NOT displayed correctly in Table (fix en rama `fix/issue-160-cjk-table-font`, pendiente PR+merge)
- Feature branches activas:
  - `fix/issue-160-cjk-table-font` — fix encoding CJK en GraphicsContext::show_text

### Cerrados (2026)
- **#159** - Release v2.3.3 (PR merged)
- **#157** - CID-keyed fonts with CMaps — CJK text extraction fix (v2.3.3)
- **#156** - SMask references not remapped — overlay fix (v2.3.3)
- **#136** - JBIG2 decoder (PR merged)
- **#137** - Release v1.8.0 (PR merged)
- **#135** - JBIG2 full implementation (auto-closed by PR #136)
- **#131** - text_wrap truncation bug
- **#128** - merge_pdfs blank PDF (3 fases)
- **#127** - CFF font rendering artifacts Firefox
- **#116** - Text extraction NUL bytes + space_threshold
- **#115** - Font subsetting large CJK fonts

### Cerrados (2025)
- **#104** - XRef non-contiguous subsections (v1.6.6)
- **#97** - TextContext used_characters (v1.6.5)
- **#98** - Linearized PDF XRef confusion (v1.6.5)
- **#93** - UTF-8 Panic Fix (v1.6.4)
- **#90** - Table Detection (v1.6.4)
- **#87** - Kerning Normalization (v1.6.1)
- **#83** - Parser Fails on Digitally Signed PDFs
- **#82** - Stack Overflow Circular References (P0)
- **#54** - ISO 32000-1:2008 Compliance Tracking

---

## EN PROGRESO

### Architecture Changes (COMPETITIVE_ANALYSIS steps) — COMPLETADO
**Merged**: PR #142, #143 → develop → main → v2.2.0
**Origen**: COMPETITIVE_ANALYSIS.md seccion 7.3

- ✅ Step 1: Merge `feature/element-type-system` → v2.1.0
- ✅ Step 2: ElementGraph — index-based parent/child/next/prev relationships
- ✅ Step 3: HybridChunker v2 — agnostic merge, sentence splitting, `full_text()`
- ✅ Step 4: Table detection — region segmentation, confidence threshold, anti-list
- ✅ Encryption-on-write: RC4-40/128, AES-128 R4, AES-256 R5/R6 (28 tests)
- ⏳ Step 5: WASM target (baja prioridad, sin rama)

### Pipeline Profiling — COMPLETADO (merged v2.2.0)
**Merged**: PR #144 → develop → main → v2.2.0

- ✅ Criterion benchmarks + corpus profiler + `verbose-debug` feature gate
- ✅ Font cache two-tier + quick wins (-2.3%)
- **Decision**: Performance no es prioridad. No invertir mas tiempo.

### Corpus 7K Test Infrastructure (feature branch)
**Rama**: `feature/corpus-7k-test-infrastructure`

**Completado**:
- ✅ `corpus_support.rs` (1,312 lineas) — runner streaming, panic safety, timeout 30s/file
- ✅ T0-T6 test suites (7 archivos)
- ✅ CI workflow `.github/workflows/corpus-tests.yml` (commit/nightly/weekly)
- ✅ Download scripts para T1, T2, T4, T5
- ✅ Decompression bomb protection (256MB limite, ratio 1000:1) en todos los filtros
- ✅ 10 tests de seguridad para decompression bombs

**Pendiente**:
- [ ] Descargar y ejecutar corpus T0 con fixtures existentes
- [ ] Descargar y ejecutar corpus T1 (veraPDF + pdf.js)
- [ ] Generar baselines T0 (baseline_times.json)

### Tiers del Corpus

| Tier | PDFs | Trigger | Source | Proposito | Estado |
|------|------|---------|--------|-----------|--------|
| T0 | 749 | Cada commit | Production fixtures | Regresion | Codigo listo, pendiente ejecucion |
| T1 | ~2,000 | Cada commit | veraPDF + pdf.js | Spec conformance | Codigo listo, descargando corpus |
| T2 | 2,000 | Nightly | GovDocs1 | Diversidad real | Codigo listo |
| T3 | 750 | Nightly | SafeDocs (curado) | Error recovery | Codigo listo, corpus manual |
| T4 | 500 | Weekly | PubMed Central OA | AI/RAG accuracy | Codigo listo |
| T5 | ~900 | Weekly | OmniDocBench | Quality benchmarking | Codigo listo |
| T6 | 200 | Weekly | Qiqqa + SafeDocs | Adversarial safety | Codigo listo, corpus manual |

---

## PROXIMOS PASOS INMEDIATOS

### 1. Benchmark RAG vs competencia
- Medir chunks/sec contra pypdf, unstructured, docling
- Requiere setup Python

### 2. Coverage 72% → 80% (cuando haya ganas)
- Modulos target: pure logic, <200 lineas, 30-85% cobertura actual

### 3. Fix: detect_columns overflow (bug pre-existente)
- extraction.rs:618 panic con Cold_Email_Hacks.pdf cuando detect_columns: true
- Afecta ExtractionProfile::Academic

### 4. Optimizar tamaño CID CFF subset (Local Subr subsetting)
- Actualmente ~1MB por font CID (Local Subr INDEXes completos para FDs necesarios)
- Competencia (krilla): 17KB para el mismo contenido
- Requiere: parser de CharStrings Type 2 para detectar callsubr/callgsubr, seguir referencias recursivamente, y emitir solo los subrs usados
- Impacto estimado: de ~1MB a <100KB

---

## SPRINTS COMPLETADOS

### Q4 2025
| Sprint | Estado | Grade | Logro Principal |
|--------|--------|-------|-----------------|
| Sprint 1 | COMPLETO | A- (90) | Code hygiene: 171 prints migrados, backup files eliminados, tracing |
| Sprint 2 | COMPLETO | A (93) | Performance: 91 clones eliminados, 10-20% ahorro memoria |
| Sprint 3 | PARCIAL | C (67%) | CI: pre-commit hooks, docs; coverage fallida |
| Sprint 4 | COMPLETO | A (95) | ISO Compliance: 7,775→310 requisitos curados |

### Q1 2026
| Sprint | Estado | Grade | Logro Principal |
|--------|--------|-------|-----------------|
| Encryption R5/R6 | COMPLETO | A | Algorithm 2.B, owner passwords, cross-validation pypdf |
| CID/Type0 Fonts | COMPLETO | A | Full embedding, security hardening |
| Coverage 54→72% | COMPLETO | A | +1,515 tests en multiple sprints |
| PDF/A Validation | COMPLETO | A | 8 niveles, 70 tests, integration tests |
| Digital Signatures | COMPLETO | A | 6 fases, detection→validation→API |
| JBIG2 Decoder | COMPLETO | A+ | 9 fases, pure Rust, 376 tests, ITU-T T.88 compliant |
| Dependency Updates | COMPLETO | A | rand 0.10, quick-xml 0.39, criterion 0.8 |
| Corpus 7K Infra | EN PROGRESO | - | 7-tier framework, CI, decompression protection |
| Pipeline Architecture v2 | COMPLETADO | A | ElementGraph, HybridChunker v2, table detection |
| Encryption on Write | COMPLETADO | A | RC4+AES-128+AES-256, 28 tests, round-trip verified |
| Pipeline Profiling | COMPLETADO | A | Criterion benchmarks, profiler tool, font cache |
| oxidize-pdf-dotnet v0.3.1 | COMPLETADO | A | Bump to v2.1.0, MIT license, NuGet published |
| **Release v2.2.0** | COMPLETADO | A | Encryption-on-write, profiling, all branches merged |
| RAG Pipeline API | COMPLETADO | A | RagChunk, rag_chunks(), rag_chunks_json(), deprecate chunk() |
| **Release v2.3.0** | COMPLETADO | A | One-liner RAG pipeline, README updated |
| OPS-005 Overlay/Watermark | COMPLETADO | A | overlay_pdf(), OverlayOptions, Form XObjects, 31 tests |
| **Release v2.3.2** | COMPLETADO | A | Overlay, Image::from_file, reorder exports, XObject fix |
| Fix #156 SMask remap | COMPLETADO | A | SMask references resolved during overlay, 11 tests |
| Fix #157 CID CJK text | COMPLETADO | A | CID→Unicode tables (79K entries), 8 tests, user-reported bug |
| **Release v2.3.3** | COMPLETADO | A | CJK fix, SMask fix, CLAUDE.md removed from VCS, branch cleanup |

---

## CONTEXTO ESTRATEGICO

### Pivot Estrategico (Post v1.2.x Issues)

**Aprendizaje Critico**: Issues reales de usuarios revelaron foundation debil

**Issues que nos ensenaron**:
1. PDFs en chino → Encoding/CJK failure (RESUELTO)
2. Transparencias PNG → Alpha channel missing (RESUELTO)
3. PDFs corruptos → Pipeline crash (RESUELTO)
4. Stack overflow circular refs → (RESUELTO v1.6.0)

**Balance Actual**: 70% fundamentals + 30% AI polish

### Modelo de Negocio — DESCARTADO (2026-03-03)

**Licencia**: MIT (cambiada desde AGPL-3.0)
**Monetizacion**: Ya no es objetivo. Proyecto open-source puro.
**Motivacion**: Aprendizaje, calidad, contribuir a la comunidad Rust.

### Ventaja Competitiva
- Pure Rust, type-safe, zero external deps (goal)
- Performance: 12K pages/sec (simple), 161x faster que target en chunking
- AI-Ready: Native RAG support, chunking, semantic tagging
- Production-Grade: 99.3% parsing success (275/277 PDFs)
- JBIG2 decoder puro Rust (sin dependencias C)
- Digital Signatures & PDF/A validation
- Decompression bomb protection

### Competencia Principal
- **lopdf**: General purpose, no optimizado para IA, sin foco comercial
- **pypdf**: Python, mas lento, dominante en data science
- **iText**: Java, EUR1,500+/year, target enterprise legacy

---

## ARQUITECTURA DEL PROYECTO

### Estructura de Repositorio
```
oxidize-pdf/
├── .private/                          # NUNCA commit a git
│   ├── ROADMAP_MASTER.md             # Este archivo
│   └── ...                           # Analisis estrategicos
│
├── oxidize-pdf-core/                  # Core library (MIT)
│   ├── src/
│   │   ├── ai/                       # AI/ML features
│   │   ├── semantic/                 # Semantic tagging
│   │   ├── parser/                   # PDF parsing
│   │   ├── text/                     # Text extraction
│   │   ├── signatures/              # Digital signatures
│   │   ├── pdfa/                    # PDF/A validation
│   │   └── ...
│   ├── examples/                     # TODOS los ejemplos aqui
│   │   └── results/                  # PDFs generados
│   └── tests/                        # Unit tests + corpus tiers
│
├── oxidize-pdf-api/                   # REST API server
├── oxidize-pdf-cli/                   # Command-line interface
├── test-corpus/                       # 7-tier PDF corpus (gitignored PDFs)
│   ├── t0-regression/                # Fixtures + baselines
│   ├── t1-spec/                      # veraPDF + pdf.js
│   ├── t2-realworld/                 # GovDocs1
│   ├── t3-stress/                    # SafeDocs (curado)
│   ├── t4-ai-target/                 # PubMed Central
│   ├── t5-quality/                   # OmniDocBench
│   ├── t6-adversarial/              # Malformed/malicious
│   └── results/                      # JSON reports (gitignored)
└── docs/                              # Community docs
```

### Reglas de Organizacion (ESTRICTAS)

| Contenido | Ubicacion |
|-----------|-----------|
| PDFs generados | `oxidize-pdf-core/examples/results/` |
| Archivos .rs de ejemplos | `oxidize-pdf-core/examples/` (flat) |
| Unit tests | `oxidize-pdf-core/tests/` |
| Corpus tests | `oxidize-pdf-core/tests/t0_*.rs` ... `t6_*.rs` |
| Scripts Python | `tools/analysis/` o `tools/scripts/` |
| Herramientas Rust debug | `dev-tools/` |

**PROHIBIDO**: ejemplos en workspace root, PDFs dispersos, `test-pdfs/` (deprecated)

---

## ROADMAP POR TRIMESTRE

### Q1 2025: ISO FUNDAMENTALS — COMPLETADO
- PNG & transparency
- CJK text extraction
- Corruption recovery
- Stack overflow protection

### Q2 2025: AI/ML INTEGRATION — COMPLETADO
- Document Chunking para RAG
- LLM-Optimized Formats
- Semantic Entity Tagging
- Invoice Text Extraction API
- Table Detection
- Structured Data Extraction

### Q3 2025: QUALITY & STABILITY — COMPLETADO
- Unwrap Elimination (51)
- Kerning Normalization
- UTF-8 Panic Fix
- Code Quality Sprints 1-3

### Q4 2025: MAINTENANCE & POLISH — COMPLETADO
- ISO Compliance Matrix Curation (Sprint 4)
- BDC/XRef fixes (v1.6.6)
- Cleanup sprint
- Encryption RC4 (v1.6.7)

### Q1 2026: SECURITY & ADVANCED FEATURES — 90% COMPLETADO
- ✅ AES-256 R5/R6 encryption
- ✅ CID/Type0 full font embedding
- ✅ Coverage 54% → 72%
- ✅ PDF/A Validation
- ✅ Digital Signatures (6 fases)
- ✅ JBIG2 decoder pure Rust (9 fases)
- ✅ Dependency updates
- ✅ Float sort safety (Rust 1.81+)
- ✅ Decompression bomb protection
- 🔄 Corpus 7K test infrastructure (en progreso)
- ⏳ PDF Editor (feature branch creada)

### Q2 2026: CONSOLIDACION (PLANIFICADO)
- oxidize-pdf-dotnet bump a v2.0.0 + MIT + Windows .dll
- README v2.0.0 con feature set real
- Coverage 72% → 80%+ (cuando haya ganas)
- Corpus T0/T1 validados y mergeados

---

## LECCIONES APRENDIDAS

### Test Coverage
1. **Test Quality != Code Coverage** - Smoke tests son inutiles para coverage
2. **API Coverage != Code Coverage** - Testear API publica no mejora coverage
3. **Priorizar Pure Logic** - Math, transformaciones, parsers (no I/O)
4. **Siempre Medir** - Correr tarpaulin ANTES y DESPUES de agregar tests

### Module Selection (ROI)

| Alto ROI | Bajo ROI |
|----------|----------|
| <200 lineas | >500 lineas |
| Pure math/conversiones | I/O, file operations |
| 30-85% coverage actual | <20% o >90% |
| Sin deps externas | Requiere PDFs/fonts reales |

### Corpus Testing
- **Streaming runner** esencial para 7K+ PDFs (previene OOM)
- **Thread-per-file** con timeout 30s protege contra loops infinitos
- **Graceful skip** cuando corpus no disponible (CI sigue funcionando)
- **Dual-layer decompression** (size + ratio) para proteccion completa

---

## LIMITACIONES CONOCIDAS

| Issue | Impacto | Estado |
|-------|---------|--------|
| 2 PDFs malformados | VERY LOW | Violaciones genuinas de formato, no bugs |

**TODO RESUELTO**:
- ~~Encrypted PDFs~~ → AES-256 R5/R6 completo
- ~~CID/Type0 fonts~~ → Full embedding
- ~~PNG compression~~ → Todos tests passing
- ~~CFF fonts Firefox~~ → CIDToGIDMap fix

---

## DOCUMENTACION REFERENCIAS

| Doc | Ubicacion |
|-----|-----------|
| Architecture | `docs/ARCHITECTURE.md` |
| Invoice Extraction | `docs/INVOICE_EXTRACTION_GUIDE.md` |
| Lints | `docs/LINTS.md` |
| Roadmap | `.private/ROADMAP_MASTER.md` |
| Corpus README | `test-corpus/README.md` |

---

## PARA CLAUDE CODE

### Workflow
1. Leer este archivo primero (estado actual)
2. Verificar branch correcto
3. Implementar segun estructura de archivos
4. Tests obligatorios
5. Actualizar este archivo

### NO Hacer
- Features fuera roadmap sin preguntar
- Cambiar estructura de archivos sin discutir
- Skip tests
- Commits sin tests passing

---

**Ultima actualizacion**: 2026-03-04
**Proxima review**: Despues de mergear profiling branch y completar dotnet v0.3.0
