//! Issue #451 — el operador `TD` (ISO 32000-1 §9.4.2) se descartaba en silencio.
//!
//! `tx ty TD` está definido como `-ty TL` seguido de `tx ty Td`: mueve la línea
//! Y fija el leading. Al no tener brazo en el `match` de operaciones, el
//! extractor no movía la matriz de línea (el salto no existía: `dx = dy = 0`
//! medido en el punto de fusión) ni fijaba el leading (los `T*` posteriores
//! heredaban uno obsoleto).
//!
//! Medido sobre el corpus completo t3-stress (1802 PDFs) contra poppler:
//! 6298 -> 752 fusiones al implementarlo, y la métrica opuesta también mejora
//! (7428 -> 5545 palabras perdidas). Ningún umbral movía la métrica; ver
//! `.private/specs/2026-07-25-issue-448-flat-path-reading-order-design.md`.

#[path = "common/mod.rs"]
mod common;
use common::synthetic_pdf::build_pdf_with_content_stream;

use oxidize_pdf::parser::{PdfDocument, PdfReader};
use oxidize_pdf::text::{ExtractionOptions, TextExtractor};
use std::io::Cursor;

fn extract_text(content: &[u8]) -> String {
    let pdf = build_pdf_with_content_stream(content);
    let reader = PdfReader::new(Cursor::new(pdf)).expect("synthetic PDF must parse");
    let document = PdfDocument::new(reader);
    let mut extractor = TextExtractor::with_options(ExtractionOptions::default());
    extractor
        .extract_from_page(&document, 0)
        .expect("extract page 0")
        .text
}

/// El caso mínimo del defecto real (`format-corpus/preserve_303226.pdf`): dos
/// `Tj` separados por un `TD`. El salto vertical (20 unidades a 12pt) supera
/// `newline_threshold` (10.0), así que el separador correcto es un salto de
/// línea. Antes del fix el extractor medía `dx = dy = 0` y no emitía nada:
/// las dos palabras salían pegadas.
#[test]
fn td_moves_to_next_line_so_words_do_not_fuse() {
    let content = b"BT\n/F1 12 Tf\n100 700 Td\n(condition)Tj\n0 -20 TD\n(records)Tj\nET\n";
    let text = extract_text(content);
    assert!(
        !text.contains("conditionrecords"),
        "TD must break the line; got fused text {:?}",
        text
    );
    assert!(
        text.contains("condition\nrecords"),
        "expected a newline between the two runs; got {:?}",
        text
    );
}

/// `TD` es, por definición, `Td` con leading. La salida tiene que ser idéntica
/// a la del par explícito `TL` + `Td` para el mismo desplazamiento. Esto fija
/// el contrato completo, no solo el síntoma de la fusión.
#[test]
fn td_is_equivalent_to_tl_plus_td_lowercase() {
    let with_td = extract_text(b"BT\n/F1 12 Tf\n100 700 Td\n(alpha)Tj\n0 -20 TD\n(beta)Tj\nET\n");
    let explicit =
        extract_text(b"BT\n/F1 12 Tf\n100 700 Td\n(alpha)Tj\n20 TL\n0 -20 Td\n(beta)Tj\nET\n");
    assert_eq!(
        with_td, explicit,
        "`tx ty TD` must behave exactly as `-ty TL` followed by `tx ty Td`"
    );
}

/// La mitad olvidada del contrato: `TD` fija el leading, así que un `T*`
/// posterior tiene que avanzar por ESE leading. Con el leading sin fijar
/// (0.0 por defecto) el `T*` no movería nada y la tercera línea se pegaría a
/// la segunda. Este test falla incluso si alguien implementa solo la
/// traslación y olvida el leading.
#[test]
fn td_sets_leading_for_subsequent_t_star() {
    let content = b"BT\n/F1 12 Tf\n100 700 Td\n(one)Tj\n0 -20 TD\n(two)Tj\nT*\n(three)Tj\nET\n";
    let text = extract_text(content);
    assert!(
        text.contains("two\nthree"),
        "T* must advance by the leading set by TD; got {:?}",
        text
    );
    assert!(
        !text.contains("twothree"),
        "a stale leading of 0 fuses the T* line; got {:?}",
        text
    );
}

/// `TD` con desplazamiento horizontal además del vertical: la traslación es
/// de la matriz de LÍNEA, así que el origen de la nueva línea es
/// `(x + tx, y + ty)` y el `T*` siguiente parte de ahí, no del margen
/// original. Pin de que se traslada `text_line_matrix` y no solo
/// `text_matrix`.
#[test]
fn td_translates_the_line_matrix_including_tx() {
    let content =
        b"BT\n/F1 12 Tf\n100 700 Td\n(first)Tj\n50 -20 TD\n(second)Tj\nT*\n(third)Tj\nET\n";
    let text = extract_text(content);
    assert!(
        text.contains("first\nsecond\nthird"),
        "each TD/T* line must be its own line; got {:?}",
        text
    );
}
