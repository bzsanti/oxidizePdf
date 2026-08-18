//! Regression tests for issue #495: independently positioned grid cells must
//! not be fused when a new text object jumps backward on the same baseline.

use oxidize_pdf::parser::{ParseOptions, PdfReader};
use oxidize_pdf::text::TextExtractor;

fn build_pdf(content: &str) -> Vec<u8> {
    let content_len = content.len();
    let objects = [
        "<< /Type /Catalog /Pages 3 0 R >>",
        "<< /Type /Page /Parent 3 0 R /MediaBox [0 0 595 842] \
         /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>",
        "<< /Type /Pages /Kids [2 0 R] /Count 1 >>",
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
    ];

    let mut pdf = b"%PDF-1.4\n".to_vec();
    let mut offsets = [0usize; 6];
    for (index, body) in objects.iter().enumerate() {
        let object_number = index + 1;
        offsets[object_number] = pdf.len();
        pdf.extend_from_slice(format!("{object_number} 0 obj\n{body}\nendobj\n").as_bytes());
    }
    offsets[5] = pdf.len();
    pdf.extend_from_slice(
        format!("5 0 obj\n<< /Length {content_len} >>\nstream\n{content}\nendstream\nendobj\n")
            .as_bytes(),
    );

    let xref = pdf.len();
    pdf.extend_from_slice(b"xref\n0 6\n0000000000 65535 f \n");
    for offset in offsets.iter().skip(1) {
        pdf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    pdf.extend_from_slice(
        format!("trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n").as_bytes(),
    );
    pdf
}

fn extract_flat(content: &str) -> String {
    let document = PdfReader::new_with_options(
        std::io::Cursor::new(build_pdf(content)),
        ParseOptions::lenient(),
    )
    .expect("PDF should parse")
    .into_document();
    TextExtractor::new()
        .extract_from_page(&document, 0)
        .expect("text should extract")
        .text
}

#[test]
fn separate_tj_objects_do_not_fuse_backward_positioned_grid_cells() {
    let content = concat!(
        "BT\n/F1 9 Tf\n1 0 0 1 32.1 653.33 Tm\n( ABCDE1234F) Tj\nET\n",
        "BT\n/F1 9 Tf\n1 0 0 1 11.32 653.33 Tm\n(PAN:) Tj\nET\n",
        "BT\n/F1 9 Tf\n1 0 0 1 28.73 635.66 Tm\n( U12345DL2019PTC123456) Tj\nET\n",
        "BT\n/F1 9 Tf\n1 0 0 1 11.32 635.66 Tm\n(CIN:) Tj\nET"
    );

    let text = extract_flat(content);
    assert_eq!(text, " ABCDE1234F\nPAN:\n U12345DL2019PTC123456\nCIN:");
}

#[test]
fn separate_tj_array_objects_use_the_same_grid_boundary() {
    let content = concat!(
        "BT\n/F1 9 Tf\n1 0 0 1 32.1 653.33 Tm\n[( ABCDE1234F)] TJ\nET\n",
        "BT\n/F1 9 Tf\n1 0 0 1 11.32 653.33 Tm\n[(PAN:)] TJ\nET"
    );

    assert_eq!(extract_flat(content), " ABCDE1234F\nPAN:");
}
