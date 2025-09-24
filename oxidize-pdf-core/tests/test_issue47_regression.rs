// Test de regresión para Issue #47
// Verifica que el parsing de diccionarios anidados funciona correctamente
// y que la extracción de texto de PDFs con Object Streams funciona

use oxidize_pdf::text::{ExtractionOptions, TextExtractor};
use oxidize_pdf::{PdfDocument, PdfReader};
use std::path::Path;

#[test]
fn test_issue47_nested_dictionary_parsing() {
    let pdf_path = "test-pdfs/Cold_Email_Hacks.pdf";

    // Skip si el PDF no existe (para CI)
    if !Path::new(pdf_path).exists() {
        return;
    }

    // Abrir el PDF debe funcionar
    let reader = PdfReader::open(pdf_path).expect("Should open PDF");
    let document = PdfDocument::new(reader);

    // Obtener la página 14 debe funcionar (antes fallaba con "Page not found in tree")
    let _page = document
        .get_page(13)
        .expect("Should get page 14 (0-indexed as 13)");

    // La extracción de texto debe funcionar correctamente
    let options = ExtractionOptions::default();
    let mut extractor = TextExtractor::with_options(options);

    let text = extractor
        .extract_from_page(&document, 14)
        .expect("Should extract text from page 14");

    // El texto debe contener la frase correcta, no texto garbled
    assert!(
        text.text.contains("Read"),
        "Text should contain 'Read', got: {}",
        &text.text[..50.min(text.text.len())]
    );
    assert!(text.text.contains("your"), "Text should contain 'your'");
    assert!(text.text.contains("email"), "Text should contain 'email'");
    assert!(
        !text.text.contains("5 H D G"),
        "Text should not contain garbled text like '5 H D G'"
    );

    // Verificar que es texto legible, no codificado
    assert!(
        text.text.len() > 500,
        "Text should be substantial (>500 chars), got {}",
        text.text.len()
    );
}

#[test]
fn test_object_stream_reconstruction() {
    let pdf_path = "test-pdfs/Cold_Email_Hacks.pdf";

    if !Path::new(pdf_path).exists() {
        return;
    }

    let reader = PdfReader::open(pdf_path).expect("Should open PDF");
    let document = PdfDocument::new(reader);

    // Verificar que podemos acceder a páginas que requieren reconstrucción
    // (páginas 15, 17, 19, 21, 23, 25 estaban faltando del XRef original)
    for page_num in [15, 17, 19, 21, 23, 25] {
        let page_result = document.get_page(page_num - 1); // 0-indexed
        assert!(
            page_result.is_ok(),
            "Should be able to get page {}",
            page_num
        );
    }
}

#[test]
fn test_font_resources_extraction() {
    let pdf_path = "test-pdfs/Cold_Email_Hacks.pdf";

    if !Path::new(pdf_path).exists() {
        return;
    }

    let reader = PdfReader::open(pdf_path).expect("Should open PDF");
    let document = PdfDocument::new(reader);

    // Verificar que podemos acceder a los objetos Font del Object Stream
    // (116, 119, 124 son objetos Font importantes en el Object Stream 111)
    for font_id in [116, 119, 124] {
        let font_result = document.get_object(font_id, 0);
        assert!(
            font_result.is_ok(),
            "Should be able to get font object {}",
            font_id
        );
    }
}
