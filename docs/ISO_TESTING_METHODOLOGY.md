# ISO 32000-1:2008 Testing Methodology

## 🎯 Objetivo

Establecer una metodología REAL para verificar el cumplimiento de ISO 32000-1:2008, basada en verificación de PDFs generados, no solo en existencia de APIs.

## 📋 Niveles de Verificación

### Nivel 0: NO IMPLEMENTADO (0%)
- No existe código para la funcionalidad
- No se puede usar la feature
- **Criterio**: `compile_error!` o función no existe

### Nivel 1: CÓDIGO EXISTS (25%)
- API existe y es invocable
- No crashea al ejecutarse
- **NO** verifica que el output sea correcto
- **Criterio**: Función ejecuta sin panic

```rust
#[test]
fn test_level_1_api_exists() {
    let mut page = Page::a4();
    let graphics = page.graphics();
    graphics.set_fill_color(Color::rgb(1.0, 0.0, 0.0));
    // Solo verifica que no crashea
}
```

### Nivel 2: OUTPUT GENERADO (50%)
- Genera PDF válido
- PDF se puede abrir en viewers
- **NO** verifica contenido específico
- **Criterio**: PDF generado tiene estructura básica

```rust
#[test]
fn test_level_2_generates_pdf() {
    let mut doc = Document::new();
    // ... setup ...
    
    let pdf_bytes = doc.save_to_bytes().unwrap();
    assert!(pdf_bytes.starts_with(b"%PDF-"));
    assert!(pdf_bytes.len() > 1000);
}
```

### Nivel 3: CONTENIDO VERIFICADO (75%)
- PDF contiene objetos esperados
- Estructura interna correcta según nuestra implementación
- Parseable por nuestro parser
- **Criterio**: Objetos PDF contienen valores esperados

```rust
#[test]
fn test_level_3_content_verified() {
    let pdf_bytes = generate_color_pdf();
    let parsed = parse_pdf(&pdf_bytes).unwrap();
    
    // Verificar objetos específicos
    assert!(parsed.contains_colorspace("/DeviceRGB"));
    assert_eq!(parsed.fill_color, Color::rgb(1.0, 0.0, 0.0));
}
```

### Nivel 4: ISO COMPLIANT (100%)
- Validado con herramientas externas (qpdf, veraPDF)
- Cumple especificación ISO byte por byte
- Interoperable con otros PDF readers
- **Criterio**: Pasa validación externa + comparación con referencia

```rust
#[test]
fn test_level_4_iso_compliant() {
    let pdf_bytes = generate_color_pdf();
    
    // 1. Validar estructura interna
    assert!(verify_internal_structure(&pdf_bytes));
    
    // 2. Validar con qpdf
    assert!(validate_with_qpdf(&pdf_bytes));
    
    // 3. Comparar con PDF de referencia ISO
    let reference = load_iso_reference("8.6.3_rgb_color.pdf");
    assert!(pdfs_structurally_equivalent(&pdf_bytes, &reference));
}
```

## 🏗️ Arquitectura del Sistema

### Módulos de Verificación

```rust
// src/verification/mod.rs
pub mod parser;           // Parsear PDFs generados
pub mod validators;       // Validadores externos
pub mod comparators;      // Comparar con referencias
pub mod iso_matrix;       // Matriz de features ISO

// src/verification/parser.rs
pub fn parse_pdf(bytes: &[u8]) -> Result<ParsedPdf>;
pub fn extract_objects(pdf: &ParsedPdf) -> HashMap<String, Object>;
pub fn verify_structure(pdf: &ParsedPdf) -> bool;

// src/verification/validators.rs
pub fn validate_with_qpdf(pdf: &[u8]) -> ValidationResult;
pub fn validate_with_verapdf(pdf: &[u8]) -> ValidationResult;
pub fn validate_adobe_preflight(pdf: &[u8]) -> ValidationResult;

// src/verification/comparators.rs  
pub fn compare_with_reference(generated: &[u8], reference: &[u8]) -> bool;
pub fn extract_diff(pdf1: &[u8], pdf2: &[u8]) -> Vec<Difference>;
```

### Tests por Sección ISO

```
tests/iso_verification/
├── section_7_document_structure/
│   ├── test_7_5_2_document_catalog.rs
│   ├── test_7_5_3_page_tree.rs
│   └── test_7_5_4_page_objects.rs
├── section_8_graphics/
│   ├── test_8_4_graphics_state.rs
│   ├── test_8_6_color_spaces.rs
│   └── test_8_7_patterns.rs
├── section_9_text/
│   ├── test_9_2_text_state.rs
│   ├── test_9_3_text_objects.rs
│   └── test_9_7_font_descriptors.rs
└── ...
```

## 📊 Matriz de Seguimiento

### Formato TOML

```toml
# ISO_COMPLIANCE_MATRIX.toml

[metadata]
version = "2025-08-21"
total_features = 286
specification = "ISO 32000-1:2008"

[section_7_5_2]
name = "Document Catalog"
description = "Required entries for document catalog dictionary"
iso_reference = "7.5.2"

[[section_7_5_2.requirements]]
id = "7.5.2.1"
name = "Type entry"
description = "Catalog must have /Type /Catalog"
implementation = "src/document.rs:45"
test_file = "tests/iso_verification/section_7/test_7_5_2.rs"
level = 3  # 0-4
verified = true
notes = "Basic implementation, missing some optional entries"

[[section_7_5_2.requirements]]
id = "7.5.2.2"
name = "Version entry"
description = "Optional /Version entry"
implementation = "src/document.rs:67"
test_file = "tests/iso_verification/section_7/test_7_5_2.rs"
level = 0
verified = false
notes = "Not implemented"
```

## 🔧 Herramientas de Validación Externa

### qpdf Integration

```bash
#!/bin/bash
# scripts/validate_with_qpdf.sh

validate_pdf_with_qpdf() {
    local pdf_file="$1"
    local output_file=$(mktemp)
    
    if qpdf --check --show-all-pages "$pdf_file" > "$output_file" 2>&1; then
        echo "PASS: $pdf_file"
        return 0
    else
        echo "FAIL: $pdf_file"
        cat "$output_file"
        return 1
    fi
}
```

### veraPDF Integration

```bash
validate_pdf_with_verapdf() {
    local pdf_file="$1"
    
    verapdf --format pdf --flavour 1b "$pdf_file" 2>&1 | \
    grep -q "ValidationProfile: PDF/A-1B validation profile" && \
    echo "PASS: $pdf_file" || echo "FAIL: $pdf_file"
}
```

## 📈 Métricas y Reporting

### Cálculo de Cumplimiento Real

```rust
pub fn calculate_real_compliance() -> ComplianceReport {
    let matrix = load_iso_matrix();
    let mut total_score = 0.0;
    let mut total_features = 0;
    
    for section in matrix.sections {
        for requirement in section.requirements {
            let level_score = match requirement.level {
                0 => 0.0,    // No implementado
                1 => 0.25,   // Código existe
                2 => 0.50,   // Genera PDF
                3 => 0.75,   // Contenido verificado
                4 => 1.0,    // ISO compliant
                _ => 0.0,
            };
            
            total_score += level_score;
            total_features += 1;
        }
    }
    
    ComplianceReport {
        total_features,
        average_score: total_score / total_features as f64,
        percentage: (total_score / total_features as f64) * 100.0,
        sections: generate_section_reports(&matrix),
        timestamp: SystemTime::now(),
    }
}
```

### Formato de Reporte

```markdown
# ISO 32000-1:2008 Real Compliance Report

**Generated**: 2025-08-21 12:34:56  
**Version**: oxidize-pdf v1.1.9  

## Overall Compliance: 34.7%

**Total Features**: 286  
**Average Level**: 1.39/4.0  

## Section Breakdown

| Section | Features | Avg Level | Compliance |
|---------|----------|-----------|------------|
| 7.5 Document Structure | 43 | 2.1/4.0 | 52.5% |
| 8.4 Graphics State | 28 | 1.8/4.0 | 45.0% |  
| 8.6 Color Spaces | 15 | 2.4/4.0 | 60.0% |
| 9.2 Text State | 12 | 1.2/4.0 | 30.0% |

## Detailed Results

### Section 7.5.2: Document Catalog
- ✅ 7.5.2.1 Type entry (Level 3) - Content verified
- ⚠️  7.5.2.2 Version entry (Level 1) - Code exists only  
- ❌ 7.5.2.3 Extensions (Level 0) - Not implemented
```

## 🚀 Proceso de Implementación

### Fase 1: Infraestructura (Semana 1-2)
1. Crear módulo de verificación
2. Implementar parser básico
3. Integrar qpdf como validador externo
4. Crear matriz de seguimiento inicial

### Fase 2: Tests por Sección (Semana 3-6)
1. Mapear todas las secciones ISO a features
2. Crear tests de verificación para cada sección
3. Eliminar tests superficiales existentes
4. Implementar verificación de contenido

### Fase 3: Validación Completa (Semana 7-8)
1. Integrar múltiples validadores externos
2. Crear suite de PDFs de referencia
3. Implementar comparación estructural
4. Automatizar reporting

### Fase 4: Optimización (Semana 9-12)
1. Optimizar velocidad de tests
2. Mejorar precisión de verificación
3. Documentar gaps encontrados
4. Crear roadmap basado en resultados reales

## ⚠️ Criterios de Éxito

### Definición de "Implementado"
Una feature solo se considera implementada al Nivel 4 si:

1. ✅ **API funciona** sin crashes
2. ✅ **Genera PDF** que se puede abrir
3. ✅ **Contenido verificado** con nuestro parser
4. ✅ **Validación externa** pasa con qpdf/veraPDF  
5. ✅ **Comparación estructural** equivalente a referencia ISO

### Métricas Prohibidas
- ❌ "API exists" como implementación completa
- ❌ Porcentajes basados solo en cobertura de código
- ❌ Tests que siempre pasan (`assert!(true)`)
- ❌ "Works in our viewer" como único criterio

### Transparencia Total
- Reportar nivel exacto alcanzado por cada feature
- Documentar limitaciones conocidas
- Proveer evidencia de validación externa
- Mantener histórico de progreso real

Esta metodología garantiza que los porcentajes de cumplimiento ISO sean **REALES** y **VERIFICABLES**.