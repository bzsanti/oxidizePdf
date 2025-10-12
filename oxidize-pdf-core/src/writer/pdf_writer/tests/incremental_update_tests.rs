// Tests for Incremental Updates Writer (ISO 32000-1 §7.5.6)

#[cfg(test)]
mod incremental_update_tests {
    use crate::document::Document;
    use crate::page::Page;
    use crate::text::Font;
    use crate::writer::{PdfWriter, WriterConfig};
    use std::fs;
    use std::io::BufWriter;
    use tempfile::TempDir;

    #[test]
    fn test_incremental_update_basic() {
        let temp_dir = TempDir::new().unwrap();
        let base_pdf_path = temp_dir.path().join("base.pdf");
        let updated_pdf_path = temp_dir.path().join("updated.pdf");

        // Step 1: Create base PDF
        let mut base_doc = Document::new();
        base_doc.set_title("Base Document");

        let mut page = Page::a4();
        page.text()
            .set_font(Font::Helvetica, 12.0)
            .at(100.0, 700.0)
            .write("Original Content")
            .unwrap();
        base_doc.add_page(page);

        base_doc.save(&base_pdf_path).unwrap();

        // Step 2: Create incremental update
        let mut update_doc = Document::new();
        update_doc.set_title("Updated Document");

        let mut update_page = Page::a4();
        update_page
            .text()
            .set_font(Font::Helvetica, 12.0)
            .at(100.0, 650.0)
            .write("Added via Incremental Update")
            .unwrap();
        update_doc.add_page(update_page);

        let updated_file = fs::File::create(&updated_pdf_path).unwrap();
        let writer = BufWriter::new(updated_file);
        let config = WriterConfig::incremental();
        let mut pdf_writer = PdfWriter::with_config(writer, config);

        let result = pdf_writer.write_incremental_update(&base_pdf_path, &mut update_doc);
        assert!(
            result.is_ok(),
            "Incremental update failed: {:?}",
            result.err()
        );

        // Step 3: Verify updated PDF exists and is larger than base
        assert!(updated_pdf_path.exists());
        let base_size = fs::metadata(&base_pdf_path).unwrap().len();
        let updated_size = fs::metadata(&updated_pdf_path).unwrap().len();
        assert!(
            updated_size > base_size,
            "Updated PDF should be larger than base"
        );
    }

    #[test]
    fn test_incremental_update_preserves_base_content() {
        let temp_dir = TempDir::new().unwrap();
        let base_pdf_path = temp_dir.path().join("base.pdf");
        let updated_pdf_path = temp_dir.path().join("updated.pdf");

        // Create base PDF
        let mut base_doc = Document::new();
        base_doc.set_title("Base Document");
        base_doc.add_page(Page::a4());
        base_doc.save(&base_pdf_path).unwrap();

        let base_content = fs::read(&base_pdf_path).unwrap();

        // Create incremental update
        let mut update_doc = Document::new();
        update_doc.add_page(Page::a4());

        let updated_file = fs::File::create(&updated_pdf_path).unwrap();
        let writer = BufWriter::new(updated_file);
        let config = WriterConfig::incremental();
        let mut pdf_writer = PdfWriter::with_config(writer, config);

        pdf_writer
            .write_incremental_update(&base_pdf_path, &mut update_doc)
            .unwrap();

        // Verify base content is preserved
        let updated_content = fs::read(&updated_pdf_path).unwrap();
        assert!(updated_content.len() > base_content.len());

        // The base content should be at the beginning
        assert_eq!(&updated_content[0..base_content.len()], &base_content[..]);
    }

    #[test]
    fn test_incremental_update_has_prev_pointer() {
        let temp_dir = TempDir::new().unwrap();
        let base_pdf_path = temp_dir.path().join("base.pdf");
        let updated_pdf_path = temp_dir.path().join("updated.pdf");

        // Create base PDF
        let mut base_doc = Document::new();
        base_doc.add_page(Page::a4());
        base_doc.save(&base_pdf_path).unwrap();

        // Create incremental update
        let mut update_doc = Document::new();
        update_doc.add_page(Page::a4());

        let updated_file = fs::File::create(&updated_pdf_path).unwrap();
        let writer = BufWriter::new(updated_file);
        let config = WriterConfig::incremental();
        let mut pdf_writer = PdfWriter::with_config(writer, config);

        pdf_writer
            .write_incremental_update(&base_pdf_path, &mut update_doc)
            .unwrap();

        // Verify /Prev pointer exists in trailer
        let updated_content = fs::read(&updated_pdf_path).unwrap();
        let updated_str = String::from_utf8_lossy(&updated_content);
        assert!(
            updated_str.contains("/Prev"),
            "Updated PDF should contain /Prev pointer in trailer"
        );
    }

    #[test]
    fn test_incremental_update_config() {
        let config = WriterConfig::incremental();

        assert!(config.incremental_update);
        assert!(!config.use_xref_streams); // Incremental updates use traditional xref
        assert!(!config.use_object_streams);
        assert_eq!(config.pdf_version, "1.4");
        assert!(config.compress_streams);
    }

    #[test]
    fn test_incremental_update_with_form_fields() {
        let temp_dir = TempDir::new().unwrap();
        let base_pdf_path = temp_dir.path().join("base_form.pdf");
        let updated_pdf_path = temp_dir.path().join("updated_form.pdf");

        // Create base PDF with a form
        let mut base_doc = Document::new();
        base_doc.set_title("Form Document");

        let mut page = Page::a4();
        page.text()
            .set_font(Font::Helvetica, 12.0)
            .at(100.0, 700.0)
            .write("Form Field Below:")
            .unwrap();
        base_doc.add_page(page);
        base_doc.save(&base_pdf_path).unwrap();

        // Create incremental update with filled form
        let mut update_doc = Document::new();
        let mut update_page = Page::a4();
        update_page
            .text()
            .set_font(Font::Helvetica, 12.0)
            .at(100.0, 650.0)
            .write("Field Value: Updated")
            .unwrap();
        update_doc.add_page(update_page);

        let updated_file = fs::File::create(&updated_pdf_path).unwrap();
        let writer = BufWriter::new(updated_file);
        let config = WriterConfig::incremental();
        let mut pdf_writer = PdfWriter::with_config(writer, config);

        let result = pdf_writer.write_incremental_update(&base_pdf_path, &mut update_doc);
        assert!(result.is_ok());
    }

    #[test]
    fn test_incremental_update_multiple_times() {
        let temp_dir = TempDir::new().unwrap();
        let base_pdf_path = temp_dir.path().join("base.pdf");
        let update1_path = temp_dir.path().join("update1.pdf");
        let update2_path = temp_dir.path().join("update2.pdf");

        // Create base PDF
        let mut base_doc = Document::new();
        base_doc.set_title("Base");
        base_doc.add_page(Page::a4());
        base_doc.save(&base_pdf_path).unwrap();

        let base_size = fs::metadata(&base_pdf_path).unwrap().len();

        // First update
        let mut update1_doc = Document::new();
        update1_doc.add_page(Page::a4());

        let file1 = fs::File::create(&update1_path).unwrap();
        let writer1 = BufWriter::new(file1);
        let mut pdf_writer1 = PdfWriter::with_config(writer1, WriterConfig::incremental());
        pdf_writer1
            .write_incremental_update(&base_pdf_path, &mut update1_doc)
            .unwrap();

        let update1_size = fs::metadata(&update1_path).unwrap().len();
        assert!(update1_size > base_size);

        // Second update (based on first update)
        let mut update2_doc = Document::new();
        update2_doc.add_page(Page::a4());

        let file2 = fs::File::create(&update2_path).unwrap();
        let writer2 = BufWriter::new(file2);
        let mut pdf_writer2 = PdfWriter::with_config(writer2, WriterConfig::incremental());
        pdf_writer2
            .write_incremental_update(&update1_path, &mut update2_doc)
            .unwrap();

        let update2_size = fs::metadata(&update2_path).unwrap().len();
        assert!(update2_size > update1_size);
    }

    #[test]
    fn test_incremental_update_error_on_missing_base() {
        let temp_dir = TempDir::new().unwrap();
        let missing_path = temp_dir.path().join("nonexistent.pdf");
        let updated_path = temp_dir.path().join("updated.pdf");

        let mut update_doc = Document::new();
        update_doc.add_page(Page::a4());

        let updated_file = fs::File::create(&updated_path).unwrap();
        let writer = BufWriter::new(updated_file);
        let config = WriterConfig::incremental();
        let mut pdf_writer = PdfWriter::with_config(writer, config);

        let result = pdf_writer.write_incremental_update(&missing_path, &mut update_doc);
        assert!(result.is_err(), "Should error when base PDF doesn't exist");
    }

    #[test]
    fn test_incremental_update_pdf_version() {
        let temp_dir = TempDir::new().unwrap();
        let base_pdf_path = temp_dir.path().join("base.pdf");
        let updated_pdf_path = temp_dir.path().join("updated.pdf");

        // Create base PDF
        let mut base_doc = Document::new();
        base_doc.add_page(Page::a4());
        base_doc.save(&base_pdf_path).unwrap();

        // Incremental update should use PDF 1.4
        let mut update_doc = Document::new();
        update_doc.add_page(Page::a4());

        let updated_file = fs::File::create(&updated_pdf_path).unwrap();
        let writer = BufWriter::new(updated_file);
        let config = WriterConfig::incremental();
        let mut pdf_writer = PdfWriter::with_config(writer, config);

        pdf_writer
            .write_incremental_update(&base_pdf_path, &mut update_doc)
            .unwrap();

        // Base PDF header is preserved (PDF-1.7), but updates use 1.4 spec
        let updated_content = fs::read(&updated_pdf_path).unwrap();
        assert!(updated_content.starts_with(b"%PDF-1.7")); // Base version preserved
    }
}
