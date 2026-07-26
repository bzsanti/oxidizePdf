//! Issue #451 en la OTRA API pública: `PlainTextExtractor` también descartaba
//! `TD`.
//!
//! El fix de #451 se aplicó al brazo de operaciones de `TextExtractor`, pero
//! `text::plaintext::PlainTextExtractor` (re-exportado, camino público
//! independiente) mantiene su propio `match`: maneja `Td`
//! (`ContentOperation::MoveText`) y `TL` (`SetLeading`), y deja
//! `MoveTextSetLeading` en el `_ =>`. Consecuencia doble e idéntica a la del
//! extractor principal: el salto de línea de `tx ty TD` no existe (`dx = dy = 0`
//! en el punto de decisión → palabras pegadas) y el leading nunca se fija, así
//! que cada `T*` posterior avanza con un leading obsoleto (0.0 por defecto).
//!
//! El oráculo aquí es más pobre que en `extraction_td_operator_test.rs`: la
//! salida plana no tiene coordenadas y el separador compara `dy.abs()` contra
//! `newline_threshold`, así que el signo del leading es INOBSERVABLE por este
//! camino. La magnitud sí se acota desde ambos lados usando desplazamientos a
//! uno y otro lado del umbral.

#[path = "common/mod.rs"]
mod common;
use common::synthetic_pdf::build_pdf_with_content_stream;

use oxidize_pdf::parser::{PdfDocument, PdfReader};
use oxidize_pdf::text::plaintext::{LineBreakMode, PlainTextConfig, PlainTextExtractor};
use std::io::Cursor;

/// Extracción con el modo por defecto (`LineBreakMode::Auto`), que es lo que ve
/// quien llama a `PlainTextExtractor::new()`. Auto convierte un `\n` interno en
/// espacio salvo tras puntuación, así que sirve para distinguir "hay separador"
/// de "palabras fusionadas", no para contar líneas.
fn extract_default(content: &[u8]) -> String {
    extract_with(content, PlainTextConfig::default())
}

/// Extracción con `PreserveAll`: los saltos que decide el extractor llegan
/// intactos a la salida. Es el modo necesario para asertar estructura de línea.
fn extract_preserving_breaks(content: &[u8]) -> String {
    extract_with(
        content,
        PlainTextConfig {
            line_break_mode: LineBreakMode::PreserveAll,
            ..Default::default()
        },
    )
}

fn extract_with(content: &[u8], config: PlainTextConfig) -> String {
    let pdf = build_pdf_with_content_stream(content);
    let reader = PdfReader::new(Cursor::new(pdf)).expect("synthetic PDF must parse");
    let document = PdfDocument::new(reader);
    let mut extractor = PlainTextExtractor::with_config(config);
    extractor
        .extract(&document, 0)
        .expect("extract page 0")
        .text
}

/// El síntoma que reporta #451, por el camino de `PlainTextExtractor`: dos `Tj`
/// separados por un `TD` de 20 unidades a 12pt (por encima de
/// `newline_threshold = 10`) tienen que quedar separados. Sin el brazo del
/// operador el extractor mide `dx = dy = 0` y las pega.
#[test]
fn plaintext_td_breaks_the_line_so_words_do_not_fuse() {
    let content = b"BT\n/F1 12 Tf\n100 700 Td\n(condition)Tj\n0 -20 TD\n(records)Tj\nET\n";
    let text = extract_preserving_breaks(content);
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

/// El mismo defecto visto desde la configuración por defecto, que es la que usa
/// el 99% de las llamadas: `Auto` degrada el salto a espacio, pero fusionar o no
/// fusionar sigue siendo observable. Pin de que el arreglo llega a la API tal
/// como se consume, no solo bajo una configuración escogida para el test.
#[test]
fn plaintext_td_does_not_fuse_words_under_the_default_config() {
    let content = b"BT\n/F1 12 Tf\n100 700 Td\n(condition)Tj\n0 -20 TD\n(records)Tj\nET\n";
    let text = extract_default(content);
    assert!(
        !text.contains("conditionrecords"),
        "TD must separate the runs under the default config; got {:?}",
        text
    );
    assert!(
        text.contains("condition records") || text.contains("condition\nrecords"),
        "expected the two runs separated; got {:?}",
        text
    );
}

/// La mitad olvidada del contrato (`TD` = `-ty TL` + `tx ty Td`): un `T*`
/// posterior avanza por el leading que fijó `TD`. Con el leading sin fijar
/// (0.0) el `T*` no mueve nada y la tercera línea se pega a la segunda.
#[test]
fn plaintext_td_sets_leading_for_subsequent_t_star() {
    let content = b"BT\n/F1 12 Tf\n100 700 Td\n(one)Tj\n0 -20 TD\n(two)Tj\nT*\n(three)Tj\nET\n";
    let text = extract_preserving_breaks(content);
    assert!(
        text.contains("two\nthree"),
        "T* must advance by the leading set by TD; got {:?}",
        text
    );
}

/// Cota superior de la magnitud del leading, imposible de fijar con el test
/// anterior: `0 -6 TD` a 12pt queda POR DEBAJO de `newline_threshold = 10`, así
/// que ni el propio `TD` ni el `T*` que hereda su leading pueden romper línea.
/// Un leading exagerado (p. ej. `3 * ty`, 18 unidades) cruzaría el umbral en el
/// `T*` e insertaría un salto que no toca.
#[test]
fn plaintext_td_leading_magnitude_does_not_exceed_ty() {
    let content = b"BT\n/F1 12 Tf\n100 700 Td\n(one)Tj\n0 -6 TD\n(two)Tj\nT*\n(three)Tj\nET\n";
    let text = extract_preserving_breaks(content);
    assert!(
        !text.contains('\n'),
        "a 6-unit TD is below the 10-unit newline threshold, so neither it nor the T* that \
         inherits its leading may break the line; got {:?}",
        text
    );
}

/// `TD` es, por definición, `Td` con leading: la salida tiene que ser idéntica a
/// la del par explícito `TL` + `Td` para el mismo desplazamiento. Fija el
/// contrato entero por este camino, no solo el síntoma de la fusión.
#[test]
fn plaintext_td_is_equivalent_to_tl_plus_td_lowercase() {
    let with_td = extract_preserving_breaks(
        b"BT\n/F1 12 Tf\n100 700 Td\n(alpha)Tj\n0 -20 TD\n(beta)Tj\nT*\n(gamma)Tj\nET\n",
    );
    let explicit = extract_preserving_breaks(
        b"BT\n/F1 12 Tf\n100 700 Td\n(alpha)Tj\n20 TL\n0 -20 Td\n(beta)Tj\nT*\n(gamma)Tj\nET\n",
    );
    assert_eq!(
        with_td, explicit,
        "`tx ty TD` must behave exactly as `-ty TL` followed by `tx ty Td`"
    );
}
