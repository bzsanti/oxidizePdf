//! Issue #455 — la pila de `q`/`Q` está acotada, y el recorte NO puede
//! desemparejar los operadores.
//!
//! Un flujo de contenido de 2 MB hecho de `q` repetidos apila un millón de
//! instantáneas. Desde #452 cada instantánea lleva el estado de texto —una
//! reserva de heap por `q` cuando hay fuente activa—, así que el factor de
//! amplificación creció. El anexo C de ISO 32000-1 da 28 como límite histórico
//! de anidamiento del estado gráfico, así que un tope holgado no toca ningún
//! documento real.
//!
//! El contrato que fijan estos tests es el del recorte CORRECTO:
//!
//! 1. Los `q` que exceden el tope no guardan instantánea, pero se CUENTAN, y
//!    los primeros `Q` del desenrollado consumen esa cuenta sin restaurar nada.
//!    Un tope que descartara empujes y honrase todos los `Q` restauraría desde
//!    la instantánea equivocada: cada `Q` a partir de ahí devolvería el estado
//!    de un nivel más externo del que le toca, y con el estado de texto dentro
//!    (#452) eso corrompe la decodificación de fuentes, no solo la CTM.
//! 2. Los niveles que SÍ caben siguen siendo exactos: el desenrollado completo
//!    devuelve el estado con el que se abrió el primer `q`.
//!
//! El oráculo es la y de los fragmentos bajo `preserve_layout`: el interlineado
//! en vigor es aritmética exacta sobre el avance de `T*`, no depende de las
//! métricas de la fuente.

#[path = "common/mod.rs"]
mod common;
use common::pdf_assembler::{assemble_pdf, stream_obj};
use common::synthetic_pdf::build_pdf_with_content_stream;

use oxidize_pdf::parser::{PdfDocument, PdfReader};
use oxidize_pdf::text::plaintext::{LineBreakMode, PlainTextConfig, PlainTextExtractor};
use oxidize_pdf::text::{ExtractionOptions, TextExtractor};
use std::io::Cursor;

/// Profundidad a la que ambos extractores dejan de guardar instantáneas.
/// Duplicada aquí a propósito: el test fija el contrato observable, no lee la
/// constante del código, así que un cambio del tope tiene que ser deliberado.
const CAP: usize = 1024;

/// Empujes por encima del tope en los casos de este fichero.
const OVERFLOW: usize = 976;

/// `q` repetido `n` veces, en un flujo de contenido.
fn q_flood(n: usize) -> Vec<u8> {
    b"q\n".repeat(n)
}

/// `Q` repetido `n` veces.
fn q_unwind(n: usize) -> Vec<u8> {
    b"Q\n".repeat(n)
}

/// `(text, x, y, font_size)` de cada fragmento bajo `preserve_layout`.
fn fragments_of_pdf(pdf: Vec<u8>) -> Vec<(String, f64, f64, f64)> {
    let reader = PdfReader::new(Cursor::new(pdf)).expect("synthetic PDF must parse");
    let document = PdfDocument::new(reader);
    let options = ExtractionOptions {
        preserve_layout: true,
        ..Default::default()
    };
    let mut extractor = TextExtractor::with_options(options);
    extractor
        .extract_from_page(&document, 0)
        .expect("extract page 0")
        .fragments
        .into_iter()
        .map(|f| (f.text, f.x, f.y, f.font_size))
        .collect()
}

fn fragments_of(content: &[u8]) -> Vec<(String, f64, f64, f64)> {
    fragments_of_pdf(build_pdf_with_content_stream(content))
}

fn fragment_named<'a>(
    frags: &'a [(String, f64, f64, f64)],
    text: &str,
) -> &'a (String, f64, f64, f64) {
    frags
        .iter()
        .find(|f| f.0 == text)
        .unwrap_or_else(|| panic!("no fragment {text:?} among {frags:?}"))
}

fn plain_text_of(pdf: Vec<u8>) -> String {
    let reader = PdfReader::new(Cursor::new(pdf)).expect("synthetic PDF must parse");
    let document = PdfDocument::new(reader);
    let mut plain = PlainTextExtractor::with_config(PlainTextConfig {
        line_break_mode: LineBreakMode::PreserveAll,
        ..Default::default()
    });
    plain.extract(&document, 0).expect("extract page 0").text
}

/// Documento testigo de los dos extractores: interlineado 10 fuera, avalancha
/// de `CAP + OVERFLOW` empujes, interlineado 50 dentro, y solo los `OVERFLOW`
/// `Q` que corresponden a los empujes descartados.
///
/// El interlineado se cambia DESPUÉS de la avalancha a propósito: todas las
/// instantáneas guardan 10, así que restaurar una sola de más es directamente
/// visible como un avance de `T*` de 10 en vez de 50.
fn overflow_only_unwind_content() -> Vec<u8> {
    let mut content = b"BT\n/F1 12 Tf\n100 700 Td\n10 TL\n(one)Tj\nET\n".to_vec();
    content.extend_from_slice(&q_flood(CAP + OVERFLOW));
    content.extend_from_slice(b"50 TL\n");
    content.extend_from_slice(&q_unwind(OVERFLOW));
    content.extend_from_slice(b"BT\n/F1 12 Tf\n100 400 Td\n(two)Tj\nT*\n(three)Tj\nET\n");
    content
}

/// Los `Q` que corresponden a empujes descartados no pueden restaurar nada.
#[test]
fn q_operators_matching_dropped_pushes_restore_nothing() {
    let frags = fragments_of(&overflow_only_unwind_content());

    let three = fragment_named(&frags, "three");
    assert!(
        (three.2 - 350.0).abs() < 0.01,
        "the {OVERFLOW} Q operators that pair with dropped pushes must restore nothing, so the \
         leading in force is still the 50 set inside the flood and `three` sits at 350; got \
         y = {} (390 means a Q consumed a snapshot that belongs to an outer level, restoring \
         the leading of 10)",
        three.2
    );
}

/// Y los niveles que sí caben en el tope siguen siendo exactos: desenrollar del
/// todo devuelve el estado con el que se abrió el primer `q`.
#[test]
fn the_levels_within_the_cap_still_restore_exactly() {
    let mut content = overflow_only_unwind_content();
    content.extend_from_slice(&q_unwind(CAP));
    content.extend_from_slice(b"BT\n/F1 12 Tf\n100 200 Td\n(four)Tj\nT*\n(five)Tj\nET\n");
    let frags = fragments_of(&content);

    let five = fragment_named(&frags, "five");
    assert!(
        (five.2 - 190.0).abs() < 0.01,
        "after unwinding the whole flood the leading is the outer 10 again, so `five` sits at \
         190; got y = {} (150 means the 50 set inside the flood survived the last Q)",
        five.2
    );
}

/// Desenrollar de más tras la avalancha sigue siendo tolerado: la cuenta de
/// descartes no puede quedar en negativo ni convertir un `Q` huérfano en una
/// restauración fantasma.
#[test]
fn unbalanced_q_after_a_flood_does_not_break_extraction() {
    let mut content = overflow_only_unwind_content();
    content.extend_from_slice(&q_unwind(CAP + 5));
    content.extend_from_slice(b"20 TL\n");
    content.extend_from_slice(&q_unwind(3));
    content.extend_from_slice(b"BT\n/F1 12 Tf\n100 200 Td\n(four)Tj\nT*\n(five)Tj\nET\n");
    let frags = fragments_of(&content);

    let five = fragment_named(&frags, "five");
    assert!(
        (five.2 - 180.0).abs() < 0.01,
        "with the stack empty, the extra Q operators must be ignored and the leading of 20 set \
         after the unwind stays in force, so `five` sits at 180; got y = {}",
        five.2
    );
}

/// El extractor plano es una implementación independiente del mismo contrato y
/// también apila una instantánea por `q` desde #452.
///
/// No expone coordenadas, pero sí la estructura de línea: con el interlineado
/// en 50 el `T*` final salta de línea; con el 10 restaurado de más, el salto es
/// menor que el umbral de línea nueva y las dos palabras se pegan.
#[test]
fn the_plain_extractor_also_keeps_q_and_q_paired_under_a_flood() {
    let mut content = b"BT\n/F1 12 Tf\n100 700 Td\n(one)Tj\nET\n".to_vec();
    content.extend_from_slice(&q_flood(CAP + OVERFLOW));
    content.extend_from_slice(b"50 TL\n");
    content.extend_from_slice(&q_unwind(OVERFLOW));
    content.extend_from_slice(b"BT\n/F1 12 Tf\n100 400 Td\n(two)Tj\nT*\n(three)Tj\nET\n");

    let text = plain_text_of(build_pdf_with_content_stream(&content));
    assert!(
        text.contains("two\nthree"),
        "the {OVERFLOW} Q operators pair with dropped pushes, so the leading of 50 is still in \
         force and the final T* breaks the line; got {text:?} (\"twothree\" means a Q restored \
         the outer leading of 0 and the T* stopped moving the pen)"
    );
}

/// Ensambla una página con un Form XObject en sus recursos.
fn pdf_with_form_xobject(page_content: &[u8], xobject_content: &[u8]) -> Vec<u8> {
    assemble_pdf(&[
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 5 0 R >> \
           /XObject << /X1 6 0 R >> >> /Contents 4 0 R >>"
            .to_vec(),
        stream_obj("", page_content),
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_vec(),
        stream_obj(
            "/Type /XObject /Subtype /Form /BBox [0 0 612 792] \
             /Resources << /Font << /F1 5 0 R >> >>",
            xobject_content,
        ),
    ])
}

/// El caso que el tope hace fácil de romper: la cuenta de empujes descartados
/// tiene que viajar con la pila al entrar y salir de un Form XObject.
///
/// `Do` da al formulario su propia pila para que un `Q` huérfano suyo no se
/// coma las instantáneas de la página. Si la cuenta de descartes se quedara
/// fuera de ese relevo, el formulario heredaría los descartes de la página —su
/// propio `Q` los consumiría— y la página volvería con la cuenta mermada: uno
/// de sus `Q` de desbordamiento acabaría restaurando una instantánea real.
#[test]
fn the_overflow_count_does_not_cross_a_form_xobject_boundary() {
    let mut page = b"BT /F1 12 Tf 100 700 Td 10 TL (one)Tj ET\n".to_vec();
    page.extend_from_slice(&q_flood(CAP + OVERFLOW));
    page.extend_from_slice(b"50 TL\n/X1 Do\n");
    page.extend_from_slice(&q_unwind(OVERFLOW));
    page.extend_from_slice(b"BT /F1 12 Tf 100 400 Td (two)Tj T* (three)Tj ET\n");

    // El formulario abre y cierra su propio bloque, y cambia el interlineado
    // dentro: si sus `Q` consumieran descartes de la página, ese 70 sobreviviría
    // a su propio `Q` y además mermaría la cuenta que la página necesita.
    let xobject = b"q\n70 TL\nBT /F1 12 Tf 100 600 Td (inner)Tj T* (inner2)Tj ET\nQ\n";

    let frags = fragments_of_pdf(pdf_with_form_xobject(&page, xobject));

    let inner2 = fragment_named(&frags, "inner2");
    assert!(
        (inner2.2 - 530.0).abs() < 0.01,
        "sanity: inside the form its own leading of 70 is in force, so `inner2` sits at 530; \
         got y = {} (a form that inherited the page's overflow count would have its own q \
         dropped or its Q eaten)",
        inner2.2
    );

    let three = fragment_named(&frags, "three");
    assert!(
        (three.2 - 350.0).abs() < 0.01,
        "back on the page, the {OVERFLOW} Q operators still pair with the page's dropped \
         pushes, so the leading is the 50 set before the Do and `three` sits at 350; got \
         y = {} (390 means the form ate part of the page's overflow count and a Q restored a \
         real snapshot)",
        three.2
    );
}
