use oxidize_pdf::parser::{ParseOptions, PdfReader};
use oxidize_pdf::writer::WriterConfig;
use oxidize_pdf::{Document, Page};
use std::io::Cursor;

#[test]
fn modern_writer_reserves_compact_unique_ids_for_all_object_streams() {
    for page_count in [1, 150] {
        let mut document = Document::new();
        for _ in 0..page_count {
            document.add_page(Page::a4());
        }
        let bytes = document
            .to_bytes_with_config(WriterConfig::modern())
            .unwrap();
        let mut reader =
            PdfReader::new_with_options(Cursor::new(&bytes), ParseOptions::strict()).unwrap();
        let size = reader.trailer().size().unwrap();
        assert!(
            size as usize <= bytes.len(),
            "object namespace must not exceed serialized byte count: {size}"
        );
        let mut stream_count = 0;
        for id in 1..size {
            let object = reader
                .get_object(id, 0)
                .unwrap_or_else(|error| panic!("missing or colliding object {id}: {error}"));
            if let oxidize_pdf::parser::PdfObject::Stream(stream) = object {
                if stream
                    .dict
                    .get("Type")
                    .and_then(|value| value.as_name())
                    .map(|name| name.0.as_str())
                    == Some("ObjStm")
                {
                    stream_count += 1;
                }
            }
        }
        assert!(stream_count > 0);
        if page_count == 150 {
            assert!(stream_count > 1);
        }
        assert_eq!(reader.into_document().page_count().unwrap(), page_count);
    }
}
