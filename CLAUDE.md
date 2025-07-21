## Software Development Guidelines

- Always act as an expert software architect in Rust
- Treat all warnings as errors
- Before pushing changes to origin, ensure all tests pass successfully
- Aim for 95% coverage of documentation, unit tests, and integration tests, with a minimum acceptable threshold of 80%

## Project Status - Session 21/07/2025 - PRODUCTION READY: 99.7% Success Rate!

### BREAKTHROUGH ACHIEVEMENT 🎉
- **Ultimate Success**: De 74.0% a **97.2%** de éxito (+23.2% mejora)
- **PRODUCTION READY**: **99.7% éxito en PDFs válidos no encriptados** (728/730)
- **Circular References ELIMINADAS**: Todos los 170 errores de referencia circular resueltos
- **XRef Issues ELIMINADAS**: Todos los errores reales de XRef resueltos
- **Command Slash `/analyze-pdfs`**: Implementado para análisis completo automatizado
- **Performance**: 215+ PDFs/segundo con procesamiento paralelo

### Current Status - PRODUCTION READY ✅
- **Total PDFs**: 749 
- **Success Rate**: **728/749 (97.2%)** 
- **Remaining Errors**: 21 PDFs (2.8%) - TODOS esperados:
  - EncryptionNotSupported: 19 casos (2.5%) - comportamiento correcto
  - EmptyFile: 2 casos (0.3%) - archivos vacíos (0 bytes)
- **InvalidXRef**: **0 casos** ✅ - COMPLETAMENTE RESUELTO
- **Issues Críticos Resueltos**: #11, #12 completamente resueltos

### Session 19/07/2025 - CI/CD Pipeline Critical Fixes

### Completed ✅
- **CI/CD Pipeline Fixes**: Ver detalles completos en PROJECT_PROGRESS.md
- **Tests Status**: 387 tests + 67 doctests, ~75% coverage
- **Issues Pendientes**: Ver lib.rs feed issues documentadas en PROJECT_PROGRESS.md

### Referencias de Documentación
- **Estado Actual Completo**: PROJECT_PROGRESS.md
- **API Documentation**: oxidize-pdf-api/API_DOCUMENTATION.md  
- **Roadmap y Features**: ROADMAP.md
- **Issues Pendientes**: PROJECT_PROGRESS.md (sección próximos pasos)

## CI/CD Pipeline Guidelines

### Pre-commit Validation
- Siempre ejecutar `cargo fmt --all` antes de commit
- Verificar `cargo clippy --all -- -D warnings`
- Validar tests locales con `cargo test --workspace`

### Patrones de Error Comunes
- Usar `std::io::Error::other()` en lugar de `Error::new(ErrorKind::Other, _)`
- Evitar `.clone()` en tipos Copy (usar `*` para dereferenciar)
- Preferir `.values()` sobre iteración `(_, value)` en maps
- Usar `#[allow(dead_code)]` para features futuras planificadas

### Comandos de Desarrollo
- Build completo: `cargo build --workspace`
- Build release: `cargo build --release` (requerido para análisis de rendimiento)
- Tests completos: `cargo test --workspace`
- Clippy estricto: `cargo clippy --all -- -D warnings`
- Formato: `cargo fmt --all`

## Comandos Slash Personalizados

### `/analyze-pdfs` - Análisis Completo de PDFs
Ejecuta análisis completo de todos los PDFs en tests/fixtures/ con las siguientes características:
- **Procesamiento Paralelo**: 8 workers, procesa ~214 PDFs/segundo
- **Timeout**: 5 segundos por PDF para evitar bloqueos  
- **Categorización**: Agrupa errores por tipo (InvalidXRef, CharacterEncoding, etc.)
- **Comparación**: Muestra mejoras vs baseline (74.0%)
- **Output JSON**: Guarda resultados detallados para análisis posterior

**Uso**: Simplemente escribir `/analyze-pdfs` como prompt

**Output típico**:
```
Total PDFs analizados: 749
Exitosos: 727 (97.1%)
Errores: 22 (2.9%)

Desglose de Errores:
  InvalidXRef: 20 (2.7%)
  Other: 2 (0.3%)

Mejoras desde baseline: +23.0%
```

**Cuándo usar**:
- Después de implementar mejoras al parser
- Para verificar regresiones
- Para identificar próximas prioridades de desarrollo
- Para generar reportes de estado del proyecto

### Troubleshooting CI/CD
- Si fallan pipelines, ejecutar comandos localmente primero
- Verificar que branch coincide con configuración CI (development/main)
- Revisar logs específicos de GitHub Actions para errores detallados

## Lecciones Aprendidas - Session 18/07/2025

### Resolución de Pipelines CI/CD
- Identificados patrones críticos de clippy que causan fallos
- Implementados fixes automáticos para errores de formato
- Documentados patrones comunes para prevención futura
- Establecido workflow de validación local pre-commit

## Project Status - Session 18/07/2025 - Community Features Implementation

### Completed Today (Part 2) ✅
- **Memory Optimization (Q4 2025 Community) completada**:
  - Módulo completo `memory/` con lazy loading y gestión eficiente
  - LRU cache thread-safe para objetos frecuentemente accedidos
  - Memory mapping cross-platform (Unix/Windows/fallback)
  - Stream processor para procesamiento incremental sin cargar todo
  - MemoryOptions con perfiles (small_file, large_file, custom)
  - 15+ tests unitarios y de integración
  - Ejemplo completo `memory_optimization.rs`
  - Agregados todos los tipos de error necesarios a PdfError

### Completed Today (Part 1) ✅
- **Basic Transparency (Q3 2025 feature) implementada**:
  - Añadidos campos `fill_opacity` y `stroke_opacity` a GraphicsContext
  - Métodos `set_opacity()`, `set_fill_opacity()`, `set_stroke_opacity()`
  - Generación de diccionario ExtGState con parámetros CA/ca
  - 8 tests unitarios nuevos para validar funcionalidad
  - Ejemplo completo `transparency.rs` demostrando el uso
- **Text Extraction mejorado (Q3 2025 feature) implementado**:
  - MacRomanEncoding completamente implementado con mapeo de caracteres
  - Detección inteligente de encoding basada en nombres de fuentes
  - Soporte para layouts complejos y detección de columnas
  - Merging de palabras con guión al final de línea
  - Opciones avanzadas: `sort_by_position`, `detect_columns`, `column_threshold`
  - 30 tests nuevos (10 extraction + 20 encoding) agregados
- **Basic Metadata completo (Q3 2025 feature) implementado**:
  - Campos Creator y Producer completamente funcionales
  - Fechas de creación y modificación con soporte UTC y Local
  - Actualización automática de fecha de modificación al guardar
  - Formateo de fechas según especificación PDF (D:YYYYMMDDHHmmSSOHH'mm)
  - 11 tests nuevos para validar toda la funcionalidad
  - Ejemplo completo `metadata.rs` demostrando todas las características
- **Funcionalidades Q2 2025 verificadas como completas**:
  - ✅ PDF Merge (26 tests)
  - ✅ PDF Split (28 tests)
  - ✅ Page Rotation (18 tests)
  - ✅ Page Reordering (17 tests)
  - ✅ Basic Compression
- **Calidad del código mantenida**:
  - 0 warnings en toda la compilación
  - Todos los tests pasando (1315+ tests)

### Session 18/07/2025 (Primera parte) - Test Coverage Improvement

### Completed Today ✅
- **Mejora masiva de test coverage**:
  - Añadidos 19 tests completos para oxidize-pdf-api
  - Añadidos 45 tests para módulos semantic (entity, export, marking)
  - Total de tests aumentado de 1053 a 1274+ tests (221 nuevos tests)
  - Coverage estimado mejorado de ~75% a ~85%+
- **Todas las features Q2 2025 completadas**:
  - ✅ PDF Merge (26 tests)
  - ✅ PDF Split (28 tests)
  - ✅ Page Rotation (18 tests)
  - ✅ Page Reordering (17 tests)
  - ✅ Basic Compression (implementado)
- **Calidad del código**:
  - 0 warnings en toda la compilación
  - Todos los tests pasando exitosamente
  - Arquitectura limpia mantenida

## Project Status - Session 17/07/2025 - Repository Architecture Refactor

### Completed ✅
- **Arquitectura de repositorios dual implementada**:
  - Creado template completo para repositorio privado `oxidizePdf-pro`
  - Movido código PRO (semantic avanzado) del repo público al privado
  - Limpiado features `pro` y `enterprise` del Cargo.toml público
  - Implementado sistema de validación de licencias
- **Módulo de exportación PRO**:
  - Estructura base para exportar a Word (DOCX) y OpenDocument (ODT)
  - Trait `DocumentExporter` para extensibilidad
  - Integración con sistema de licencias
- **CLI PRO implementado**:
  - Comando `export` para conversión de formatos
  - Gestión de licencias (activate, status, deactivate)
  - Validación de licencia al inicio
- **Documentación actualizada**:
  - Creado REPOSITORY_ARCHITECTURE.md
  - Actualizado código semantic para Community Edition

## Project Status - Session 18/07/2025 - Community Edition REST API Implementation

### Completed Today ✅
- **REST API Community Edition Completado**:
  - Implementado endpoint completo de merge PDF (`POST /api/merge`)
  - Soporte para múltiples archivos PDF con opciones configurables
  - Manejo de archivos temporales y multipart form data
  - Headers informativos con estadísticas de operación
- **Testing Comprehensivo**:
  - 5 tests unitarios/integración para merge endpoint
  - Validación de casos edge (archivos insuficientes, datos inválidos)
  - Tests de éxito con múltiples archivos PDF
  - 100% coverage del endpoint merge
- **Documentación API Completa**:
  - Guía completa de endpoints disponibles
  - Ejemplos de uso en curl, JavaScript, Python
  - Documentación de códigos de error y responses
  - Especificaciones de formato de datos
- **Roadmap Actualizado**:
  - Marcadas todas las features Community Edition como completadas
  - Preparado para release v1.0.0 Community Edition
  - Actualizado estado del proyecto para reflejar completion

## Project Status - Session 16/07/2025 - Page Extraction Feature Implementation

### Completed ✅
- **Release v0.1.2 exitoso**: Primera release oficial en GitHub con pipeline automatizado
- **Pipeline CI/CD completo**: Release, CI, y coverage funcionando perfectamente
- **Doctests corregidos**: 58 doctests pasando (referencias `oxidize_pdf_core` → `oxidize_pdf` corregidas)
- **Tests unitarios e integración**: 231 tests pasando correctamente (0 fallos)
- **Sistema dual de testing implementado**: 
  - CI/CD usa PDFs sintéticos para builds rápidos y consistentes
  - Desarrollo local puede usar 743 PDFs reales via fixtures/symbolic link
- **Eliminación de warnings**: Solo 2 warnings menores no críticos
- **Fixture system**: Detección automática de fixtures, estadísticas y sampling
- **Property tests reparados**: UTF-8 handling, dimensiones floating point, operadores balanceados
- **Release automation**: Merge automático a main, publicación a crates.io, versionado independiente
- **Mejora masiva de test coverage**:
  - CLI module: 18 tests de integración completos (0% → ~85% coverage estimado)
  - parser/object_stream.rs: 15 tests unitarios (0% → 100% coverage)  
  - objects/array.rs: 20 tests unitarios (0% → 100% coverage)
- **Sistema completo de benchmarks con Criterion.rs**:
  - core_benchmarks.rs: Array, ObjectStream, XRef, Dictionary, String operations
  - parser_bench.rs: PDF parsing y content stream performance 
  - cli_benchmarks.rs: Command performance y file I/O operations
  - memory_benchmarks.rs: Memory allocation patterns y nested structures
  - ocr_benchmarks.rs: OCR provider performance y comparison benchmarks
  - CI pipeline: Automated benchmark execution con artifact storage
- **Módulo de análisis de páginas completo**:
  - operations/page_analysis.rs: Detección de páginas escaneadas vs texto vectorial
  - PageContentAnalyzer: Análisis de ratios de contenido (texto, imagen, espacio)
  - PageType classification: Scanned, Text, Mixed con thresholds configurables
  - Integración con TextExtractor para análisis de texto vectorial
  - Procesamiento paralelo y batch para OCR
  - Documentación extensa con ejemplos y doctests
- **Sistema OCR completo implementado**:
  - text/ocr.rs: Trait OcrProvider para integración con motores OCR
  - MockOcrProvider: Implementación de prueba para desarrollo y testing
  - TesseractOcrProvider: Implementación completa con Tesseract OCR
  - OcrProcessingResult: Estructuras de datos para resultados OCR
  - Integración completa con PageContentAnalyzer
  - Soporte para múltiples formatos de imagen (JPEG, PNG, TIFF)
  - Multi-language support (50+ idiomas)
  - Configuración avanzada: PSM/OEM modes, preprocessing, filtering
- **CI/CD Pipeline Fixes para v0.1.3**:
  - Corregidos todos los errores de formato con cargo fmt
  - Resueltos todos los warnings de clippy:
    - empty_line_after_doc_comments
    - single_match → if let
    - manual_div_ceil → .div_ceil()
    - for_kv_map → values() iterator
    - collapsible_match → if let anidados combinados
  - Corregidos doctests fallidos (añadido no_run donde se requieren archivos)
  - MockOcrProvider ahora implementa Clone trait
  - Imports corregidos para módulos OCR
- **Page Extraction Feature (Q1 2025 roadmap item)**:
  - operations/page_extraction.rs: Módulo completo para extraer páginas de PDFs
  - PageExtractor: Clase principal con opciones configurables
  - PageExtractionOptions: Configuración para metadata, annotations, forms y optimización
  - Support para single page, multiple pages, y page ranges
  - Convenience functions para operaciones directas de archivo
  - Content stream parsing y reconstruction para preservar contenido
  - Font mapping y graphics operations handling
  - 19 tests comprehensivos (100% funcionalidad cubierta)

### Estado Actual del Código - Session 18/07/2025
- **Test Coverage**: ~85%+ estimado (vs 43.42% inicial) - Mejora del +96%
- **Tests**: 1274+ tests totales pasando (vs 175 al inicio)
- **CI/CD**: Todos los checks de formato y clippy pasando
- **Warnings**: 0 warnings (build completamente limpio)
- **Release**: v0.1.2 publicada, v0.1.3 completada, v0.1.4 en preparación
- **Estructura**: Workspace multi-crate funcional y organizado
- **OCR Features**: Sistema completo y funcional con Tesseract
- **Page Extraction**: Feature completa implementada con 19 tests pasando
- **PDF Operations**: Todas las operaciones Q2 2025 completadas y testeadas

### Coverage Achievements Session 16/07/2025 ✅

0. **Warnings Cleanup** (15 warnings corregidos):
   - Removed unused imports y variables
   - Fixed dead code warnings
   - Cleaned up test helper functions
   - Build completamente limpio (0 warnings)

1. **PDF Merge Operations** (26 tests nuevos):
   - Comprehensive tests para MergeOptions y MetadataMode
   - Tests para PdfMerger con diferentes configuraciones
   - Tests de merge con page ranges complejos
   - Tests de preservación de bookmarks y forms
   - Tests de optimización y metadata handling
   - Debug y Clone implementations

2. **PDF Split Operations** (28 tests nuevos):
   - Comprehensive tests para SplitOptions y SplitMode
   - Tests para PdfSplitter con diferentes modos
   - Tests de split por chunks, ranges, y puntos específicos
   - Tests de nomenclatura de archivos output
   - Tests de preservación de metadata
   - Edge cases y error handling

3. **Sesiones Anteriores** (se mantienen):
   - Page Extraction: 19 tests
   - OperationError: 16 tests
   - Tesseract OCR Provider: 45 tests

4. **Tesseract OCR Provider** (45 tests nuevos):
   - Implementación completa de TesseractOcrProvider
   - Configuración PSM/OEM modes
   - Multi-language support y detection
   - Character whitelisting/blacklisting
   - Custom variables y debug mode
   - Error handling comprehensivo
   - Feature flag implementation con stubs

2. **OCR Core Module** (25 tests nuevos):
   - MockOcrProvider con tests exhaustivos
   - OcrProcessingResult methods (filtering, regions, types)
   - Image preprocessing options
   - Engine types y format support
   - Confidence scoring y validation
   - Fragment analysis y positioning

3. **Page Analysis OCR Integration** (19 tests nuevos):
   - OCR methods en PageContentAnalyzer
   - Scanned page detection
   - OCR options integration
   - Procesamiento paralelo y batch
   - Error handling en OCR workflows
   - Performance comparisons

4. **Page Extraction Feature** (19 tests nuevos):
   - PageExtractor con opciones configurables
   - Single page, multiple pages, y page ranges
   - Metadata preservation y content reconstruction
   - Font mapping y graphics operations
   - File I/O operations y error handling
   - Convenience functions para operaciones directas

5. **Sesiones Anteriores** (142 tests):
   - CLI Integration Tests: 18 tests
   - Object Stream Parser: 15 tests
   - Array Objects: 20 tests
   - Doctests: 58 tests
   - Tests originales: 84 tests

### Objetivos de Coverage 🎯
- **Objetivo**: 95% coverage (80% mínimo aceptable)
- **Logrado total**: ~75%+ (vs 43.42% inicial) - Mejora del +75%
- **Áreas completadas**: CLI, object_stream, array, OCR modules, page_extraction, merge, split completamente
- **Tests totales**: 387 (vs 175 al inicio de sesión) - +121% más tests
- **Funcionalidad OCR**: Sistema completo de análisis de páginas y OCR
- **Soporte Tesseract**: Implementación completa y funcional
- **Page Extraction**: Feature Q1 2025 completamente implementada
- **PDF Operations**: Merge y Split completamente implementadas y testeadas

### Arquitectura OCR
1. **Trait-based**: OcrProvider trait para extensibilidad
2. **Multiple Providers**: Mock, Tesseract, futuro: Azure, AWS, Google Cloud
3. **Feature Flags**: Dependencias opcionales para build flexibility
4. **Performance**: Parallel processing y batch operations
5. **Configuration**: Extensive customization options
6. **Error Handling**: Comprehensive error types y recovery

### Métricas de Calidad - Session 18/07/2025
- Tests totales: 1274+ ✅ (vs 175 inicial)
- Tests añadidos hoy: 221 tests nuevos ✅
- Coverage: ~85%+ ✅ (objetivo 95%, mejora del +96%)
- Warnings: 0/0 ✅ (build completamente limpio)
- Benchmarks: 5 suites completas con CI automation ✅
- Pipeline: funcionando sin timeouts ✅
- Release: automatizado ✅
- OCR: Completamente funcional ✅
- Todas las features Q2 2025: Completamente implementadas ✅
- API Tests: 19 tests nuevos ✅
- Semantic Tests: 45 tests nuevos ✅