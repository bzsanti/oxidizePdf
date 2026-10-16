//! Independent xref/object-stream fixture: MCIDs are document-wide, not compact
//! per-page indices. Neither sparsity nor an unrelated stream object may remove
//! the structure replacement.
use flate2::{write::ZlibEncoder, Compression};
use oxidize_pdf::parser::{PdfDocument, PdfReader};
use std::io::{Cursor, Write};

fn tagged_pdf(padding: usize, compressed: bool, bad_sibling: bool) -> Vec<u8> {
    let content = format!("BT /F1 12 Tf 72 720 Td /Span << /MCID {padding} >> BDC (VISUAL) Tj EMC 0 -20 Td /Span << /MCID 0 >> BDC (NULL_OWNER) Tj EMC 0 -20 Td /Span << /MCID {padding} /ActualText (INLINE) >> BDC (HIDDEN) Tj EMC ET");
    let owners = format!("[{}9 0 R null]", "null ".repeat(padding));
    let sibling = if bad_sibling {
        "[unterminated".to_string()
    } else {
        format!("[{}]", "null ".repeat(padding * 4))
    };
    let mut bodies = vec![
        (1, b"<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 6 0 R >>".to_vec()),
        (2, b"<< /Type /Pages /Count 1 /Kids [3 0 R] >>".to_vec()),
        (3, b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /StructParents 42 /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>".to_vec()),
        (4, b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_vec()),
        (5, format!("<< /Length {} >>\nstream\n{content}\nendstream", content.len()).into_bytes()),
        (6, b"<< /Type /StructTreeRoot /ParentTree 7 0 R >>".to_vec()),
        (7, b"<< /Nums [42 8 0 R] >>".to_vec()),
    ];
    let compressed_bodies = [
        owners,
        "<< /Type /StructElem /ActualText (STRUCTURE) >>".to_string(),
        sibling,
    ];
    if compressed {
        let mut data = String::new();
        let mut header = String::new();
        for (index, body) in compressed_bodies.iter().enumerate() {
            header.push_str(&format!("{} {} ", index + 8, data.len()));
            data.push_str(body);
            data.push('\n');
        }
        let first = header.len();
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder
            .write_all(format!("{header}{data}").as_bytes())
            .unwrap();
        let encoded = encoder.finish().unwrap();
        let mut stream = format!(
            "<< /Type /ObjStm /N 3 /First {first} /Filter /FlateDecode /Length {} >>\nstream\n",
            encoded.len()
        )
        .into_bytes();
        stream.extend(encoded);
        stream.extend(b"\nendstream");
        bodies.push((11, stream));
    } else {
        for (index, body) in compressed_bodies.iter().enumerate() {
            bodies.push((index + 8, body.as_bytes().to_vec()));
        }
    }
    let mut pdf = b"%PDF-1.7\n".to_vec();
    let mut offsets = [0; 13];
    for (id, body) in bodies {
        offsets[id] = pdf.len();
        pdf.extend(format!("{id} 0 obj\n").as_bytes());
        pdf.extend(body);
        pdf.extend(b"\nendobj\n");
    }
    offsets[12] = pdf.len();
    let mut xref = Vec::new();
    for (id, offset) in offsets.iter().enumerate() {
        let (kind, field, generation) = if compressed && (8..=10).contains(&id) {
            (2u8, 11u32, (id - 8) as u16)
        } else if *offset != 0 {
            (1, *offset as u32, 0)
        } else {
            (0, 0, 65535)
        };
        xref.push(kind);
        xref.extend(field.to_be_bytes());
        xref.extend(generation.to_be_bytes());
    }
    pdf.extend(
        format!(
            "12 0 obj\n<< /Type /XRef /Size 13 /Root 1 0 R /W [1 4 2] /Length {} >>\nstream\n",
            xref.len()
        )
        .as_bytes(),
    );
    pdf.extend(xref);
    pdf.extend(format!("\nendstream\nendobj\nstartxref\n{}\n%%EOF\n", offsets[12]).as_bytes());
    pdf
}

#[test]
fn sparse_compressed_and_plain_owners_preserve_mcid_and_inline_precedence() {
    for compressed in [false, true] {
        let doc = PdfDocument::new(
            PdfReader::new(Cursor::new(tagged_pdf(20_000, compressed, false))).unwrap(),
        );
        for _ in 0..2 {
            let text = doc.extract_text_from_page(0).unwrap().text;
            assert!(text.contains("STRUCTURE"), "{compressed}: {text}");
            assert!(text.contains("NULL_OWNER"));
            assert!(text.contains("INLINE"));
            assert!(!text.contains("VISUAL") && !text.contains("HIDDEN"));
        }
        let markdown = doc.to_markdown().unwrap();
        assert!(markdown.contains("STRUCTURE"), "{markdown}");
        assert!(doc.extract_text().unwrap()[0].text.contains("STRUCTURE"));
    }
}

#[test]
fn unrelated_object_is_not_parsed_during_structure_lookup() {
    let doc = PdfDocument::new(PdfReader::new(Cursor::new(tagged_pdf(200, true, true))).unwrap());
    let text = doc.extract_text_from_page(0).unwrap().text;
    assert!(text.contains("STRUCTURE"), "{text}");
}
