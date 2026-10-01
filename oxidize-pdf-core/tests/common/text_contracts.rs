//! Public-API fixtures for encoding and tracking regressions.
//! No production encoding tables are used to compute expectations.
#![allow(dead_code)]

#[path = "pdf_assembler.rs"]
pub mod assembler;

use oxidize_pdf::parser::{ParseOptions, PdfReader};
use oxidize_pdf::text::{ExtractedText, ExtractionOptions, TextExtractor};
use std::io::Cursor;

pub const LATIN_STANDARD14: [&str; 12] = [
    "Helvetica",
    "Helvetica-Bold",
    "Helvetica-Oblique",
    "Helvetica-BoldOblique",
    "Times-Roman",
    "Times-Bold",
    "Times-Italic",
    "Times-BoldItalic",
    "Courier",
    "Courier-Bold",
    "Courier-Oblique",
    "Courier-BoldOblique",
];

pub fn pdf(font: &str, content: &[u8], extra: Vec<Vec<u8>>) -> Vec<u8> {
    let mut objects = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>".to_vec(),
        font.as_bytes().to_vec(),
        assembler::stream_obj("", content),
    ];
    objects.extend(extra);
    assembler::assemble_pdf(&objects)
}

pub fn extract(bytes: Vec<u8>, options: ParseOptions) -> ExtractedText {
    let doc = PdfReader::new_with_options(Cursor::new(bytes), options)
        .expect("fixture must parse under its declared policy")
        .into_document();
    let options = ExtractionOptions {
        sort_by_position: false,
        ..ExtractionOptions::default()
    };
    TextExtractor::with_options(options)
        .extract_from_page(&doc, 0)
        .expect("fixture extraction must succeed")
}

pub fn font(name: &str, encoding: &str, widths: Option<f64>, extra: &str) -> String {
    let widths = widths.map_or_else(String::new, |width| {
        format!(
            "/FirstChar 0 /LastChar 255 /Widths [{}]",
            format!("{width} ").repeat(256)
        )
    });
    format!(
        "<< /Type /Font /Subtype /Type1 /BaseFont /{name} /Encoding {encoding} {widths} {extra} >>"
    )
}

pub fn cmap(entries: &str, count: usize, code_space: &str) -> Vec<u8> {
    let content = format!(
        "/CIDInit /ProcSet findresource begin 12 dict begin begincmap\n\
         /CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def\n\
         /CMapName /TextContract def /CMapType 2 def\n\
         1 begincodespacerange {code_space} endcodespacerange\n\
         {count} beginbfchar\n{entries}\nendbfchar\n\
         endcmap CMapName currentdict /CMap defineresource pop end end"
    );
    assembler::stream_obj("", content.as_bytes())
}
