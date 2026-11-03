use oxidize_pdf::parser::PdfReader;
use oxidize_pdf::text::{ExtractionOptions, TextExtractor};

#[test]
fn test_tounicode_identity_h_name_raw_pdf() {
    let pdf_bytes = b"%PDF-1.5
1 0 obj
<< /Type /Catalog /Pages 2 0 R >>
endobj
2 0 obj
<< /Type /Pages /Kids [3 0 R] /Count 1 >>
endobj
3 0 obj
<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 4 0 R >> >> /Contents 6 0 R >>
endobj
4 0 obj
<< /Type /Font /Subtype /Type0 /BaseFont /TestFont /Encoding /Identity-H /ToUnicode /Identity-H /DescendantFonts [5 0 R] >>
endobj
5 0 obj
<< /Type /Font /Subtype /CIDFontType2 /BaseFont /TestFont /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> /DW 1000 >>
endobj
6 0 obj
<< /Length 44 >>
stream
BT
/F1 12 Tf
<00480065006C006C006F> Tj
ET
endstream
endobj
xref
0 7
0000000000 65535 f 
0000000009 00000 n 
0000000058 00000 n 
0000000115 00000 n 
0000000228 00000 n 
0000000350 00000 n 
0000000477 00000 n 
trailer
<< /Size 7 /Root 1 0 R >>
startxref
570
%%EOF
";

    let reader = PdfReader::new(std::io::Cursor::new(pdf_bytes.as_slice())).unwrap();
    let doc = reader.into_document();

    let mut extractor = TextExtractor::with_options(ExtractionOptions::default());
    let page_text = extractor.extract_from_page(&doc, 0).unwrap().text;

    assert_eq!(page_text.trim(), "Hello");
}

#[test]
fn test_tounicode_identity_v_name_raw_pdf() {
    let pdf_bytes = b"%PDF-1.5
1 0 obj
<< /Type /Catalog /Pages 2 0 R >>
endobj
2 0 obj
<< /Type /Pages /Kids [3 0 R] /Count 1 >>
endobj
3 0 obj
<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 4 0 R >> >> /Contents 6 0 R >>
endobj
4 0 obj
<< /Type /Font /Subtype /Type0 /BaseFont /TestFont /Encoding /Identity-V /ToUnicode /Identity-V /DescendantFonts [5 0 R] >>
endobj
5 0 obj
<< /Type /Font /Subtype /CIDFontType2 /BaseFont /TestFont /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> /DW 1000 >>
endobj
6 0 obj
<< /Length 44 >>
stream
BT
/F1 12 Tf
<0057006F0072006C0064> Tj
ET
endstream
endobj
xref
0 7
0000000000 65535 f 
0000000009 00000 n 
0000000058 00000 n 
0000000115 00000 n 
0000000228 00000 n 
0000000350 00000 n 
0000000477 00000 n 
trailer
<< /Size 7 /Root 1 0 R >>
startxref
570
%%EOF
";

    let reader = PdfReader::new(std::io::Cursor::new(pdf_bytes.as_slice())).unwrap();
    let doc = reader.into_document();

    let mut extractor = TextExtractor::with_options(ExtractionOptions::default());
    let page_text = extractor.extract_from_page(&doc, 0).unwrap().text;

    assert_eq!(page_text.trim(), "World");
}
