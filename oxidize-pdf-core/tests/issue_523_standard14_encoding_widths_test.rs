//! End-to-end coverage for encoding-aware Standard-14 widths (#523).

use oxidize_pdf::parser::{ParseOptions, PdfReader};
use oxidize_pdf::text::{ExtractionOptions, TextExtractor};

fn build_pdf(font: &str, code: u8) -> Vec<u8> {
    let mut content = b"BT\n/F1 10 Tf\n1 0 0 1 100 700 Tm\n(".to_vec();
    content.push(code);
    content.extend_from_slice(b") Tj\nET");

    let objects = [
        b"<< /Type /Catalog /Pages 3 0 R >>".as_slice(),
        b"<< /Type /Page /Parent 3 0 R /MediaBox [0 0 595 842] /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>".as_slice(),
        b"<< /Type /Pages /Kids [2 0 R] /Count 1 >>".as_slice(),
        font.as_bytes(),
    ];

    let mut pdf = b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n".to_vec();
    let mut offsets = [0usize; 6];
    for (index, body) in objects.iter().enumerate() {
        let object_number = index + 1;
        offsets[object_number] = pdf.len();
        pdf.extend_from_slice(format!("{object_number} 0 obj\n").as_bytes());
        pdf.extend_from_slice(body);
        pdf.extend_from_slice(b"\nendobj\n");
    }
    offsets[5] = pdf.len();
    pdf.extend_from_slice(format!("5 0 obj\n<< /Length {} >>\nstream\n", content.len()).as_bytes());
    pdf.extend_from_slice(&content);
    pdf.extend_from_slice(b"\nendstream\nendobj\n");

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

fn extracted_width(font: &str, code: u8) -> f64 {
    let document = PdfReader::new_with_options(
        std::io::Cursor::new(build_pdf(font, code)),
        ParseOptions::lenient(),
    )
    .expect("minimal PDF should parse")
    .into_document();
    let extracted = TextExtractor::with_options(ExtractionOptions {
        preserve_layout: true,
        ..Default::default()
    })
    .extract_from_page(&document, 0)
    .expect("text should extract");
    extracted
        .fragments
        .first()
        .unwrap_or_else(|| panic!("one text fragment should be emitted: {:?}", extracted.text))
        .width
}

#[test]
fn pdf_font_dictionaries_drive_standard14_afm_widths() {
    let cases = [
        (
            "StandardEncoding",
            "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /StandardEncoding >>",
            b'\'',
            2.22,
        ),
        (
            "WinAnsiEncoding",
            "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>",
            b'\'',
            1.91,
        ),
        (
            "MacRomanEncoding",
            "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /MacRomanEncoding >>",
            0xDB,
            5.56,
        ),
        (
            "Differences",
            "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding << /BaseEncoding /WinAnsiEncoding /Differences [65 /fi] >> >>",
            b'A',
            5.0,
        ),
        (
            "Symbol builtin",
            "<< /Type /Font /Subtype /Type1 /BaseFont /Symbol >>",
            b'a',
            6.31,
        ),
        (
            "ZapfDingbats builtin",
            "<< /Type /Font /Subtype /Type1 /BaseFont /ZapfDingbats >>",
            b'!',
            9.74,
        ),
    ];

    for (label, font, code, expected) in cases {
        let actual = extracted_width(font, code);
        assert!(
            (actual - expected).abs() < 1e-12,
            "{label}: expected width {expected}, got {actual}"
        );
    }
}
