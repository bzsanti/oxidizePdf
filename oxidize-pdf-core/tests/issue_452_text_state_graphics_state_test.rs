//! Issue #452 — el estado de TEXTO forma parte del estado gráfico y `Q` tiene
//! que restaurarlo.
//!
//! ISO 32000-1 §9.3 y Tabla 52: interlineado (`TL`), espaciado de carácter
//! (`Tc`) y de palabra (`Tw`), escala horizontal (`Tz`), fuente y tamaño
//! (`Tf`), desplazamiento vertical (`Ts`) y modo de pintado (`Tr`) son
//! parámetros del estado gráfico. `q` los apila y `Q` los restaura.
//!
//! El extractor guardaba solo la CTM y el color de relleno, así que todo lo
//! anterior se escapaba del bloque: un interlineado fijado dentro de un
//! `q … Q` seguía gobernando los saltos de línea después de cerrarlo.
//!
//! `Tm` y `Tlm` NO son estado gráfico (son estado del objeto de texto, que `BT`
//! reinicia), así que `Q` no debe restaurarlos. Eso no se asierta aquí porque
//! el propio `BT` posterior los fija; se documenta en el código.
//!
//! Oráculo: coordenadas de fragmento bajo `preserve_layout` para lo que es
//! aritmética exacta (el interlineado), y comparación diferencial contra el
//! mismo documento SIN el bloque `q … Q` para lo que depende de métricas de
//! fuente (la escala horizontal).

#[path = "common/mod.rs"]
mod common;
use common::pdf_assembler::{assemble_pdf, stream_obj};
use common::synthetic_pdf::build_pdf_with_content_stream;

use oxidize_pdf::parser::{PdfDocument, PdfReader};
use oxidize_pdf::text::plaintext::{LineBreakMode, PlainTextConfig, PlainTextExtractor};
use oxidize_pdf::text::{ExtractionOptions, TextExtractor};
use std::io::Cursor;

/// `(text, x, y, font_size)` de cada fragmento bajo `preserve_layout`.
fn fragments_of(content: &[u8]) -> Vec<(String, f64, f64, f64)> {
    let pdf = build_pdf_with_content_stream(content);
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

fn fragment_named<'a>(
    frags: &'a [(String, f64, f64, f64)],
    text: &str,
) -> &'a (String, f64, f64, f64) {
    frags
        .iter()
        .find(|f| f.0 == text)
        .unwrap_or_else(|| panic!("no fragment {text:?} among {frags:?}"))
}

/// El caso del issue: un `TL` de 40 dentro del bloque no puede seguir vigente
/// al salir. La última línea la mueve un `T*`, que avanza por el interlineado
/// EN VIGOR: 20, el de fuera. Con la fuga avanzaría 40.
#[test]
fn q_restores_the_leading_set_inside_the_block() {
    let content = b"BT\n/F1 12 Tf\n100 700 Td\n20 TL\n(one)Tj\nT*\n(two)Tj\nET\n\
                    q\nBT\n/F1 12 Tf\n100 600 Td\n40 TL\n(three)Tj\nET\nQ\n\
                    BT\n/F1 12 Tf\n100 400 Td\n(four)Tj\nT*\n(five)Tj\nET\n";
    let frags = fragments_of(content);

    let two = fragment_named(&frags, "two");
    assert!(
        (two.2 - 680.0).abs() < 0.01,
        "sanity: inside the first block the leading is 20, so `two` sits at 680; got {two:?}"
    );

    let five = fragment_named(&frags, "five");
    assert!(
        (five.2 - 380.0).abs() < 0.01,
        "after Q the leading must be the outer 20, so `five` sits at 380; got y = {} \
         (360 means the 40 set inside the q…Q block leaked out)",
        five.2
    );
}

/// Sin ningún `TL` fuera del bloque, el interlineado que vuelve es el inicial
/// (0): el `T*` posterior no mueve nada. Pin de que `Q` restaura el valor
/// guardado y no simplemente "el último que vio antes del bloque".
#[test]
fn q_restores_the_initial_leading_when_none_was_set_outside() {
    let content = b"BT\n/F1 12 Tf\n100 700 Td\n(one)Tj\nET\n\
                    q\nBT\n/F1 12 Tf\n100 600 Td\n40 TL\n(two)Tj\nET\nQ\n\
                    BT\n/F1 12 Tf\n100 400 Td\n(three)Tj\nT*\n(four)Tj\nET\n";
    let frags = fragments_of(content);
    let four = fragment_named(&frags, "four");
    assert!(
        (four.2 - 400.0).abs() < 0.01,
        "with no leading in force outside the block, T* must not move; got y = {} \
         (360 means the inner 40 leaked)",
        four.2
    );
}

/// El tamaño de fuente también es estado gráfico. El último bloque no emite
/// `Tf`, así que hereda el que `Q` haya restaurado.
#[test]
fn q_restores_the_font_size_set_inside_the_block() {
    let content = b"BT\n/F1 12 Tf\n100 700 Td\n(one)Tj\nET\n\
                    q\nBT\n/F1 30 Tf\n100 600 Td\n(two)Tj\nET\nQ\n\
                    BT\n100 400 Td\n(three)Tj\nET\n";
    let frags = fragments_of(content);
    let three = fragment_named(&frags, "three");
    assert!(
        (three.3 - 12.0).abs() < 0.01,
        "after Q the font size must be the outer 12; got {} (30 means it leaked)",
        three.3
    );
}

/// La escala horizontal gobierna el avance de la pluma, así que su fuga mueve
/// la x de todo lo que venga después. El oráculo es diferencial: el mismo
/// documento sin el bloque `q … Q` tiene que dar exactamente las mismas
/// coordenadas, sin depender de las métricas de la fuente.
#[test]
fn q_restores_the_horizontal_scale_set_inside_the_block() {
    let with_block = b"BT\n/F1 12 Tf\n100 700 Td\n(one)Tj\nET\n\
                       q\nBT\n/F1 12 Tf\n200 Tz\n100 600 Td\n(two)Tj\nET\nQ\n\
                       BT\n/F1 12 Tf\n100 400 Td\n(three)Tj\n(four)Tj\nET\n";
    let without_block = b"BT\n/F1 12 Tf\n100 700 Td\n(one)Tj\nET\n\
                          BT\n/F1 12 Tf\n100 400 Td\n(three)Tj\n(four)Tj\nET\n";

    // Toda la línea de abajo, en las dos versiones del documento. Con la fuga,
    // el avance de la pluma se duplica: los dos `Tj` dejan de tocarse, así que
    // ni se fusionan en un fragmento ni caen en la misma x.
    let line_of = |content: &[u8]| -> Vec<(String, f64)> {
        fragments_of(content)
            .into_iter()
            .filter(|f| (f.2 - 400.0).abs() < 0.01)
            .map(|f| (f.0, (f.1 * 100.0).round() / 100.0))
            .collect()
    };

    let expected = line_of(without_block);
    assert!(
        !expected.is_empty(),
        "the oracle produced no fragments at y = 400 — comparing two empty lists would pass \
         while measuring nothing"
    );
    assert_eq!(
        line_of(with_block),
        expected,
        "the pen after Q must advance as if `200 Tz` had never happened; a leaked scale \
         doubles the advance, which splits the run and moves it right"
    );
}

/// Los dos extractores públicos tienen que coincidir: el plano no expone
/// coordenadas, pero sí la estructura de línea que produce el interlineado, y
/// es una implementación independiente del mismo contrato.
#[test]
fn both_public_extractors_restore_the_leading_on_q() {
    // Con el interlineado restaurado a 0, el `T*` final no mueve la pluma, así
    // que las dos últimas palabras quedan en la misma línea. Con la fuga de 40
    // se separan.
    let content = b"BT\n/F1 12 Tf\n100 700 Td\n(one)Tj\nET\n\
                    q\nBT\n/F1 12 Tf\n100 600 Td\n40 TL\n(two)Tj\nET\nQ\n\
                    BT\n/F1 12 Tf\n100 400 Td\n(three)Tj\nT*\n(four)Tj\nET\n";

    let pdf = build_pdf_with_content_stream(content);
    let reader = PdfReader::new(Cursor::new(pdf)).expect("synthetic PDF must parse");
    let document = PdfDocument::new(reader);
    let mut plain = PlainTextExtractor::with_config(PlainTextConfig {
        line_break_mode: LineBreakMode::PreserveAll,
        ..Default::default()
    });
    let plain_text = plain.extract(&document, 0).expect("extract page 0").text;

    // La negativa sola pasaría con la salida vacía o con una decodificación
    // rota, así que se fija además lo que TIENE que salir.
    assert!(
        plain_text.contains("threefour"),
        "with the leading restored to 0 the final T* does not move, so the last two runs share \
         a line; got {plain_text:?}"
    );
    assert!(
        !plain_text.contains("three\nfour"),
        "PlainTextExtractor must restore the leading on Q too: a 40-unit leading from inside \
         the block broke the last line; got {plain_text:?}"
    );
}

/// Un `Q` sin `q` sigue siendo tolerado: los PDF mal formados no deben tumbar
/// la extracción. Cubre los DOS extractores: el `pop` del extractor plano es
/// código nuevo de este cambio y es el único que no tenía guarda de pila vacía.
#[test]
fn an_unbalanced_q_does_not_break_either_extractor() {
    let content = b"Q\nBT\n/F1 12 Tf\n100 700 Td\n20 TL\n(one)Tj\nT*\n(two)Tj\nET\nQ\nQ\n";

    let frags = fragments_of(content);
    let two = fragment_named(&frags, "two");
    assert!(
        (two.2 - 680.0).abs() < 0.01,
        "extraction must survive unbalanced Q and keep the leading in force; got {two:?}"
    );

    let pdf = build_pdf_with_content_stream(content);
    let reader = PdfReader::new(Cursor::new(pdf)).expect("synthetic PDF must parse");
    let document = PdfDocument::new(reader);
    let mut plain = PlainTextExtractor::with_config(PlainTextConfig {
        line_break_mode: LineBreakMode::PreserveAll,
        ..Default::default()
    });
    let plain_text = plain.extract(&document, 0).expect("extract page 0").text;
    assert!(
        plain_text.contains("one\ntwo"),
        "PlainTextExtractor must survive unbalanced Q and still apply the leading; got \
         {plain_text:?}"
    );
}

/// Anidamiento de más de un nivel: cada `Q` tiene que emparejar con SU `q`. Un
/// error de emparejamiento (guardar de menos, restaurar de más) solo se ve con
/// dos niveles y tres valores distintos de interlineado.
#[test]
fn nested_q_blocks_restore_their_own_leading_at_each_level() {
    let content = b"BT\n/F1 12 Tf\n100 700 Td\n10 TL\n(a)Tj\nET\n\
                    q\n20 TL\n\
                      q\n40 TL\n\
                        BT\n/F1 12 Tf\n100 650 Td\n(inner)Tj\nET\n\
                      Q\n\
                      BT\n/F1 12 Tf\n100 550 Td\n(mid)Tj\nT*\n(mid2)Tj\nET\n\
                    Q\n\
                    BT\n/F1 12 Tf\n100 400 Td\n(out)Tj\nT*\n(out2)Tj\nET\n";
    let frags = fragments_of(content);

    let mid2 = fragment_named(&frags, "mid2");
    assert!(
        (mid2.2 - 530.0).abs() < 0.01,
        "the inner Q must restore the middle level's leading of 20, so `mid2` sits at 530; \
         got y = {} (510 means the innermost 40 survived its own Q)",
        mid2.2
    );

    let out2 = fragment_named(&frags, "out2");
    assert!(
        (out2.2 - 390.0).abs() < 0.01,
        "the outer Q must restore the page's leading of 10, so `out2` sits at 390; got y = {}",
        out2.2
    );
}

/// `q`/`Q` dentro de un objeto de texto: la norma no lo bendice (§8.2), pero es
/// abundantísimo en documentos reales, y ahora importa porque un `Q` a mitad de
/// un `BT … ET` cambia la fuente. La restauración no puede tocar la matriz de
/// texto: las palabras siguen en la línea donde estaban.
#[test]
fn a_q_block_inside_a_text_object_restores_state_without_moving_the_pen() {
    let content = b"BT\n/F1 12 Tf\n100 700 Td\n20 TL\n(one)Tj\n\
                    q\n/F1 30 Tf\n40 TL\n(two)Tj\nQ\n\
                    T*\n(three)Tj\nET\n";
    let frags = fragments_of(content);

    // `q` no toca la matriz de texto, así que `two` continúa la línea de `one`:
    // quedan lo bastante juntos como para fusionarse en un solo fragmento en el
    // mismo origen. Si la restauración moviera la pluma, `two` saldría aparte.
    let joined = fragment_named(&frags, "onetwo");
    assert!(
        (joined.2 - 700.0).abs() < 0.01 && (joined.1 - 100.0).abs() < 0.01,
        "the q/Q must not move the pen: the two runs stay on the line that started at \
         (100, 700); got {joined:?}"
    );

    let three = fragment_named(&frags, "three");
    assert!(
        (three.3 - 12.0).abs() < 0.01,
        "after Q the font size is the outer 12 again; got {}",
        three.3
    );
    assert!(
        (three.2 - 680.0).abs() < 0.01,
        "and the T* advances by the restored leading of 20, landing at 680; got y = {} \
         (660 means the inner 40 leaked past the Q)",
        three.2
    );
}

/// Ensambla una página con un Form XObject en sus recursos. `page_content` se
/// ejecuta en la página; `xobject_content` dentro del `Do`.
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

fn fragments_of_pdf(pdf: Vec<u8>) -> Vec<(String, f64, f64, f64)> {
    let reader = PdfReader::new(Cursor::new(pdf)).expect("assembled PDF must parse");
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

/// `Do` pinta un Form XObject dentro de un `q`/`Q` IMPLÍCITO (§8.10.1), así que
/// el estado gráfico completo — y por tanto el de texto — vuelve al salir. Un
/// `TL` puesto dentro del XObject no puede gobernar el `T*` de la página.
#[test]
fn a_form_xobject_does_not_leak_its_text_state_into_the_page() {
    let pdf = pdf_with_form_xobject(
        b"BT /F1 12 Tf 100 700 Td 20 TL (one)Tj ET\n/X1 Do\n\
          BT /F1 12 Tf 100 400 Td (two)Tj T* (three)Tj ET\n",
        b"BT /F1 12 Tf 100 600 Td 40 TL (inner)Tj ET\n",
    );
    let frags = fragments_of_pdf(pdf);
    assert!(
        frags.iter().any(|f| f.0 == "inner"),
        "sanity: the XObject's own text must be extracted, or this proves nothing; got {frags:?}"
    );
    let three = fragment_named(&frags, "three");
    assert!(
        (three.2 - 380.0).abs() < 0.01,
        "the leading of 40 set inside the XObject must die with the implicit Q, leaving the \
         page's 20: expected y = 380, got {}",
        three.2
    );
}

/// Un `Q` de más dentro del XObject no puede consumir el estado guardado por la
/// PÁGINA. Con la pila compartida, ese `Q` huérfano se llevaba la instantánea
/// del `q` de la página y el `Q` posterior de la página restauraba desde una
/// pila vacía, es decir, no restauraba nada.
///
/// El XObject cambia el interlineado DESPUÉS de su `Q` huérfano a propósito: si
/// no lo hiciera, ese `Q` restauraría por casualidad el mismo valor que el `Q`
/// de la página debía restaurar, y el test pasaría sin discriminar nada.
#[test]
fn an_unbalanced_q_inside_a_form_xobject_cannot_consume_the_pages_saved_state() {
    let pdf = pdf_with_form_xobject(
        b"BT /F1 12 Tf 100 700 Td 10 TL (one)Tj ET\n\
          q\n30 TL\n/X1 Do\nQ\n\
          BT /F1 12 Tf 100 400 Td (two)Tj T* (three)Tj ET\n",
        b"Q\n50 TL\nBT /F1 12 Tf 100 600 Td (inner)Tj ET\n",
    );
    let frags = fragments_of_pdf(pdf);
    let three = fragment_named(&frags, "three");
    assert!(
        (three.2 - 390.0).abs() < 0.01,
        "the page's Q must restore the leading of 10 it saved, whatever the XObject did to \
         the stack: expected y = 390, got {} (370 means the stray Q ate the page's snapshot)",
        three.2
    );
}
