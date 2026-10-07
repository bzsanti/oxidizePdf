//! Writer resource sharing must preserve Raw names, custom fonts and repeated saves.
use oxidize_pdf::parser::{PdfDocument, PdfObject, PdfReader};
use oxidize_pdf::writer::WriterConfig;
use oxidize_pdf::{Document, Font, Page};
use std::io::Cursor;

const NAMES: [&str; 12] = [
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

fn raw_page() -> Page {
    let mut page = Page::a4();
    for (i, name) in NAMES.iter().enumerate() {
        page.graphics().add_command(&format!(
            "BT /{name} 12 Tf 50 {} Td (Sample{i}) Tj ET\n",
            750 - i * 20
        ));
    }
    page
}

#[test]
fn standard_resources_are_written_once_and_raw_variants_survive() {
    let mut doc = Document::new();
    doc.set_compress(false);
    for _ in 0..3 {
        doc.add_page(raw_page());
    }
    let bytes = doc.to_bytes().unwrap();
    let source = String::from_utf8_lossy(&bytes);
    assert_eq!(
        source.matches("/Encoding /WinAnsiEncoding").count(),
        12,
        "standard dictionaries must be serialized once per document"
    );
    let parsed = PdfDocument::new(PdfReader::new(Cursor::new(&bytes)).unwrap());
    for page in 0..3 {
        let parsed_page = parsed.get_page(page).unwrap();
        let resources = parsed_page.get_resources().unwrap();
        let font_object = resources.get("Font").unwrap();
        let resolved;
        let fonts = match font_object {
            PdfObject::Reference(n, g) => {
                resolved = parsed.get_object(*n, *g).unwrap();
                resolved.as_dict().unwrap()
            }
            object => object.as_dict().unwrap(),
        };
        for name in NAMES {
            let font = fonts.get(name).unwrap().as_dict().unwrap();
            assert_eq!(
                font.get("BaseFont").unwrap().as_name().unwrap().as_str(),
                name
            );
            assert_eq!(
                font.get("Subtype").unwrap().as_name().unwrap().as_str(),
                "Type1"
            );
            assert_eq!(
                font.get("Encoding").unwrap().as_name().unwrap().as_str(),
                "WinAnsiEncoding"
            );
        }
        let text = parsed.extract_text_from_page(page).unwrap().text;
        assert_eq!(
            text.lines()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>(),
            (0..12).map(|i| format!("Sample{i}")).collect::<Vec<_>>()
        );
    }
}

#[test]
fn shared_resources_survive_repeated_saves_and_added_custom_page() {
    let mut doc = Document::new();
    doc.add_page(raw_page());
    for config in [WriterConfig::legacy(), WriterConfig::modern()] {
        let bytes = doc.to_bytes_with_config(config).unwrap();
        let parsed = PdfDocument::new(PdfReader::new(Cursor::new(bytes)).unwrap());
        assert!(parsed
            .extract_text_from_page(0)
            .unwrap()
            .text
            .contains("Sample11"));
    }
    doc.add_font_from_bytes(
        "Roboto",
        include_bytes!("fixtures/text_contracts/fonts/SourceSans3-Regular.ttf").to_vec(),
    )
    .unwrap();
    let mut page = Page::a4();
    page.text()
        .set_font(Font::Custom("Roboto".into()), 12.0)
        .at(50.0, 700.0)
        .write("Custom cafe")
        .unwrap();
    doc.add_page(page);
    for config in [WriterConfig::legacy(), WriterConfig::modern()] {
        let bytes = doc.to_bytes_with_config(config).unwrap();
        let parsed = PdfDocument::new(PdfReader::new(Cursor::new(bytes)).unwrap());
        assert!(parsed
            .extract_text_from_page(0)
            .unwrap()
            .text
            .contains("Sample11"));
        assert_eq!(
            parsed.extract_text_from_page(1).unwrap().text.trim(),
            "Custom cafe"
        );
    }
}

#[test]
fn imported_shared_resources_remain_local_while_new_pages_share() {
    let mut source = Document::new();
    source.add_page(raw_page());
    source.add_page(raw_page());
    let bytes = source.to_bytes().unwrap();
    let parsed = PdfDocument::new(PdfReader::new(Cursor::new(bytes)).unwrap());
    let imported = Page::from_parsed_with_content(&parsed.get_page(0).unwrap(), &parsed).unwrap();
    let mut output = Document::new();
    output.add_page(raw_page());
    output.add_page(imported);
    output.add_page(raw_page());
    let bytes = output.to_bytes().unwrap();
    let parsed = PdfDocument::new(PdfReader::new(Cursor::new(bytes)).unwrap());
    for index in 0..3 {
        let text = parsed.extract_text_from_page(index).unwrap().text;
        assert_eq!(
            text.lines()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>(),
            (0..12).map(|i| format!("Sample{i}")).collect::<Vec<_>>()
        );
    }
}

#[test]
fn shared_custom_font_named_like_a_standard_font_keeps_its_glyphs() {
    let mut doc = Document::new();
    doc.add_font_from_bytes(
        "Helvetica",
        include_bytes!("fixtures/text_contracts/fonts/SourceSans3-Regular.ttf").to_vec(),
    )
    .unwrap();
    for text in ["Alpha", "Omega"] {
        let mut page = Page::a4();
        page.text()
            .set_font(Font::Custom("Helvetica".into()), 12.0)
            .at(50.0, 700.0)
            .write(text)
            .unwrap();
        doc.add_page(page);
    }
    let bytes = doc.to_bytes().unwrap();
    let parsed = PdfDocument::new(PdfReader::new(Cursor::new(bytes)).unwrap());
    for (index, expected) in ["Alpha", "Omega"].iter().enumerate() {
        assert_eq!(
            parsed
                .extract_text_from_page(index as u32)
                .unwrap()
                .text
                .trim(),
            *expected
        );
        let page = parsed.get_page(index as u32).unwrap();
        let resource = page.get_resources().unwrap().get("Font").unwrap();
        let PdfObject::Reference(n, g) = resource else {
            panic!("shared dictionary expected");
        };
        let fonts = parsed.get_object(*n, *g).unwrap();
        let PdfObject::Reference(n, g) = fonts.as_dict().unwrap().get("Helvetica").unwrap() else {
            panic!("embedded font reference expected");
        };
        let font = parsed.get_object(*n, *g).unwrap();
        assert_eq!(
            font.as_dict()
                .unwrap()
                .get("Subtype")
                .unwrap()
                .as_name()
                .unwrap()
                .as_str(),
            "Type0"
        );
    }
}

#[test]
fn shared_cjk_font_preserves_characters_from_every_page() {
    let mut doc = Document::new();
    let font = include_bytes!("fixtures/writer_resources/WriterCjkTest-Regular.otf").to_vec();
    doc.add_font_from_bytes("CJK", font).unwrap();
    for text in ["中文", "测试"] {
        let mut page = Page::a4();
        page.text()
            .set_font(Font::Custom("CJK".into()), 12.0)
            .at(50.0, 700.0)
            .write(text)
            .unwrap();
        doc.add_page(page);
    }
    let bytes = doc.to_bytes().unwrap();
    let parsed = PdfDocument::new(PdfReader::new(Cursor::new(bytes)).unwrap());
    for (index, expected) in ["中文", "测试"].iter().enumerate() {
        assert_eq!(
            parsed
                .extract_text_from_page(index as u32)
                .unwrap()
                .text
                .trim(),
            *expected
        );
    }
}
