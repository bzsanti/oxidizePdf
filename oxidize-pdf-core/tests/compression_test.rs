//! Test compressed PDF generation and parsing

#[cfg(feature = "compression")]
mod compression_tests {
    use oxidize_pdf::parser::PdfReader;
    use oxidize_pdf::{Document, Font, Page};
    use tempfile::TempDir;

    #[test]
    fn test_compressed_pdf_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
        // Create a temporary directory for test files
        let temp_dir = TempDir::new()?;
        let pdf_path = temp_dir.path().join("test_compressed.pdf");

        // Create a document with content
        let mut doc = Document::new();
        doc.set_title("Compression Test");

        // Create a page with text
        let mut page = Page::a4();

        let mut text_flow = page.text_flow();
        text_flow.set_font(Font::Helvetica, 12.0);
        text_flow.write_wrapped("This is a test of PDF compression. This text should be compressed using FlateDecode filter.")?;
        page.add_text_flow(&text_flow);

        doc.add_page(page);

        // Save the document (should use compression by default when feature is enabled)
        doc.save(&pdf_path)?;

        // Verify the file was created
        assert!(pdf_path.exists());

        // Now try to parse the PDF
        let mut reader = PdfReader::open(&pdf_path)?;

        // Verify basic properties
        assert_eq!(reader.version().major, 1);
        assert_eq!(reader.version().minor, 7);

        // Try to get metadata
        let metadata = reader.metadata()?;
        assert_eq!(metadata.title.as_deref(), Some("Compression Test"));

        // Clean up is automatic when temp_dir is dropped
        Ok(())
    }

    #[test]
    fn test_filter_is_name_object() -> Result<(), Box<dyn std::error::Error>> {
        use oxidize_pdf::objects::{Object, Stream};

        // Create a stream and set filter
        let data = vec![1, 2, 3, 4, 5];
        let mut stream = Stream::new(data);
        stream.set_filter("FlateDecode");

        // Verify the Filter is set as a Name object, not a String
        let dict = stream.dictionary();
        let filter = dict.get("Filter").expect("Filter should be set");

        match filter {
            Object::Name(name) => {
                assert_eq!(name, "FlateDecode");
            }
            _ => {
                panic!("Filter should be a Name object, but was: {filter:?}");
            }
        }

        Ok(())
    }
}
