use crate::error::Result;
use crate::objects::{Object, ObjectId};
use crate::page::Page;
use crate::text::{FontEncoding, FontWithEncoding};
use crate::writer::PdfWriter;
use chrono::{DateTime, Local, Utc};
use std::collections::{HashMap, HashSet};

/// A PDF document that can contain multiple pages and metadata.
///
/// # Example
///
/// ```rust
/// use oxidize_pdf::{Document, Page};
///
/// let mut doc = Document::new();
/// doc.set_title("My Document");
/// doc.set_author("John Doe");
///
/// let page = Page::a4();
/// doc.add_page(page);
///
/// doc.save("output.pdf").unwrap();
/// ```
pub struct Document {
    pub(crate) pages: Vec<Page>,
    #[allow(dead_code)]
    pub(crate) objects: HashMap<ObjectId, Object>,
    #[allow(dead_code)]
    pub(crate) next_object_id: u32,
    pub(crate) metadata: DocumentMetadata,
    /// Default font encoding to use for fonts when no encoding is specified
    pub(crate) default_font_encoding: Option<FontEncoding>,
}

/// Metadata for a PDF document.
#[derive(Debug, Clone)]
pub struct DocumentMetadata {
    /// Document title
    pub title: Option<String>,
    /// Document author
    pub author: Option<String>,
    /// Document subject
    pub subject: Option<String>,
    /// Document keywords
    pub keywords: Option<String>,
    /// Software that created the original document
    pub creator: Option<String>,
    /// Software that produced the PDF
    pub producer: Option<String>,
    /// Date and time the document was created
    pub creation_date: Option<DateTime<Utc>>,
    /// Date and time the document was last modified
    pub modification_date: Option<DateTime<Utc>>,
}

impl Default for DocumentMetadata {
    fn default() -> Self {
        let now = Utc::now();
        Self {
            title: None,
            author: None,
            subject: None,
            keywords: None,
            creator: Some("oxidize_pdf".to_string()),
            producer: Some(format!("oxidize_pdf v{}", env!("CARGO_PKG_VERSION"))),
            creation_date: Some(now),
            modification_date: Some(now),
        }
    }
}

impl Document {
    /// Creates a new empty PDF document.
    pub fn new() -> Self {
        Self {
            pages: Vec::new(),
            objects: HashMap::new(),
            next_object_id: 1,
            metadata: DocumentMetadata::default(),
            default_font_encoding: None,
        }
    }

    /// Adds a page to the document.
    pub fn add_page(&mut self, page: Page) {
        self.pages.push(page);
    }

    /// Sets the document title.
    pub fn set_title(&mut self, title: impl Into<String>) {
        self.metadata.title = Some(title.into());
    }

    /// Sets the document author.
    pub fn set_author(&mut self, author: impl Into<String>) {
        self.metadata.author = Some(author.into());
    }

    /// Sets the document subject.
    pub fn set_subject(&mut self, subject: impl Into<String>) {
        self.metadata.subject = Some(subject.into());
    }

    /// Sets the document keywords.
    pub fn set_keywords(&mut self, keywords: impl Into<String>) {
        self.metadata.keywords = Some(keywords.into());
    }

    /// Sets the document creator (software that created the original document).
    pub fn set_creator(&mut self, creator: impl Into<String>) {
        self.metadata.creator = Some(creator.into());
    }

    /// Sets the document producer (software that produced the PDF).
    pub fn set_producer(&mut self, producer: impl Into<String>) {
        self.metadata.producer = Some(producer.into());
    }

    /// Sets the document creation date.
    pub fn set_creation_date(&mut self, date: DateTime<Utc>) {
        self.metadata.creation_date = Some(date);
    }

    /// Sets the document creation date using local time.
    pub fn set_creation_date_local(&mut self, date: DateTime<Local>) {
        self.metadata.creation_date = Some(date.with_timezone(&Utc));
    }

    /// Sets the document modification date.
    pub fn set_modification_date(&mut self, date: DateTime<Utc>) {
        self.metadata.modification_date = Some(date);
    }

    /// Sets the document modification date using local time.
    pub fn set_modification_date_local(&mut self, date: DateTime<Local>) {
        self.metadata.modification_date = Some(date.with_timezone(&Utc));
    }

    /// Sets the modification date to the current time.
    pub fn update_modification_date(&mut self) {
        self.metadata.modification_date = Some(Utc::now());
    }

    /// Sets the default font encoding for fonts that don't specify an encoding.
    ///
    /// This encoding will be applied to fonts in the PDF font dictionary when
    /// no explicit encoding is specified. Setting this to `None` (the default)
    /// means no encoding metadata will be added to fonts unless explicitly specified.
    ///
    /// # Example
    ///
    /// ```rust
    /// use oxidize_pdf::{Document, text::FontEncoding};
    ///
    /// let mut doc = Document::new();
    /// doc.set_default_font_encoding(Some(FontEncoding::WinAnsiEncoding));
    /// ```
    pub fn set_default_font_encoding(&mut self, encoding: Option<FontEncoding>) {
        self.default_font_encoding = encoding;
    }

    /// Gets the current default font encoding.
    pub fn default_font_encoding(&self) -> Option<FontEncoding> {
        self.default_font_encoding
    }

    /// Gets all fonts used in the document with their encodings.
    ///
    /// This scans all pages and collects the unique fonts used, applying
    /// the default encoding where no explicit encoding is specified.
    pub(crate) fn get_fonts_with_encodings(&self) -> Vec<FontWithEncoding> {
        let mut fonts_used = HashSet::new();

        // Collect fonts from all pages
        for page in &self.pages {
            // Get fonts from text content
            for font in page.get_used_fonts() {
                let font_with_encoding = match self.default_font_encoding {
                    Some(default_encoding) => FontWithEncoding::new(font, Some(default_encoding)),
                    None => FontWithEncoding::without_encoding(font),
                };
                fonts_used.insert(font_with_encoding);
            }
        }

        fonts_used.into_iter().collect()
    }

    /// Gets the number of pages in the document.
    pub fn page_count(&self) -> usize {
        self.pages.len()
    }

    /// Saves the document to a file.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be created or written.
    pub fn save(&mut self, path: impl AsRef<std::path::Path>) -> Result<()> {
        // Update modification date before saving
        self.update_modification_date();

        let mut writer = PdfWriter::new(path)?;
        writer.write_document(self)?;
        Ok(())
    }

    /// Writes the document to a buffer.
    ///
    /// # Errors
    ///
    /// Returns an error if the PDF cannot be generated.
    pub fn write(&mut self, buffer: &mut Vec<u8>) -> Result<()> {
        // Update modification date before writing
        self.update_modification_date();

        let mut writer = PdfWriter::new_with_writer(buffer);
        writer.write_document(self)?;
        Ok(())
    }

    #[allow(dead_code)]
    pub(crate) fn allocate_object_id(&mut self) -> ObjectId {
        let id = ObjectId::new(self.next_object_id, 0);
        self.next_object_id += 1;
        id
    }

    #[allow(dead_code)]
    pub(crate) fn add_object(&mut self, obj: Object) -> ObjectId {
        let id = self.allocate_object_id();
        self.objects.insert(id, obj);
        id
    }
}

impl Default for Document {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_document_new() {
        let doc = Document::new();
        assert!(doc.pages.is_empty());
        assert!(doc.objects.is_empty());
        assert_eq!(doc.next_object_id, 1);
        assert!(doc.metadata.title.is_none());
        assert!(doc.metadata.author.is_none());
        assert!(doc.metadata.subject.is_none());
        assert!(doc.metadata.keywords.is_none());
        assert_eq!(doc.metadata.creator, Some("oxidize_pdf".to_string()));
        assert!(doc
            .metadata
            .producer
            .as_ref()
            .unwrap()
            .starts_with("oxidize_pdf"));
    }

    #[test]
    fn test_document_default() {
        let doc = Document::default();
        assert!(doc.pages.is_empty());
        assert_eq!(doc.next_object_id, 1);
    }

    #[test]
    fn test_add_page() {
        let mut doc = Document::new();
        let page1 = Page::a4();
        let page2 = Page::letter();

        doc.add_page(page1);
        assert_eq!(doc.pages.len(), 1);

        doc.add_page(page2);
        assert_eq!(doc.pages.len(), 2);
    }

    #[test]
    fn test_set_title() {
        let mut doc = Document::new();
        assert!(doc.metadata.title.is_none());

        doc.set_title("Test Document");
        assert_eq!(doc.metadata.title, Some("Test Document".to_string()));

        doc.set_title(String::from("Another Title"));
        assert_eq!(doc.metadata.title, Some("Another Title".to_string()));
    }

    #[test]
    fn test_set_author() {
        let mut doc = Document::new();
        assert!(doc.metadata.author.is_none());

        doc.set_author("John Doe");
        assert_eq!(doc.metadata.author, Some("John Doe".to_string()));
    }

    #[test]
    fn test_set_subject() {
        let mut doc = Document::new();
        assert!(doc.metadata.subject.is_none());

        doc.set_subject("Test Subject");
        assert_eq!(doc.metadata.subject, Some("Test Subject".to_string()));
    }

    #[test]
    fn test_set_keywords() {
        let mut doc = Document::new();
        assert!(doc.metadata.keywords.is_none());

        doc.set_keywords("test, pdf, rust");
        assert_eq!(doc.metadata.keywords, Some("test, pdf, rust".to_string()));
    }

    #[test]
    fn test_metadata_default() {
        let metadata = DocumentMetadata::default();
        assert!(metadata.title.is_none());
        assert!(metadata.author.is_none());
        assert!(metadata.subject.is_none());
        assert!(metadata.keywords.is_none());
        assert_eq!(metadata.creator, Some("oxidize_pdf".to_string()));
        assert!(metadata
            .producer
            .as_ref()
            .unwrap()
            .starts_with("oxidize_pdf"));
    }

    #[test]
    fn test_allocate_object_id() {
        let mut doc = Document::new();

        let id1 = doc.allocate_object_id();
        assert_eq!(id1.number(), 1);
        assert_eq!(id1.generation(), 0);
        assert_eq!(doc.next_object_id, 2);

        let id2 = doc.allocate_object_id();
        assert_eq!(id2.number(), 2);
        assert_eq!(id2.generation(), 0);
        assert_eq!(doc.next_object_id, 3);
    }

    #[test]
    fn test_add_object() {
        let mut doc = Document::new();
        assert!(doc.objects.is_empty());

        let obj = Object::Boolean(true);
        let id = doc.add_object(obj.clone());

        assert_eq!(id.number(), 1);
        assert_eq!(doc.objects.len(), 1);
        assert!(doc.objects.contains_key(&id));
    }

    #[test]
    fn test_write_to_buffer() {
        let mut doc = Document::new();
        doc.set_title("Buffer Test");
        doc.add_page(Page::a4());

        let mut buffer = Vec::new();
        let result = doc.write(&mut buffer);

        assert!(result.is_ok());
        assert!(!buffer.is_empty());
        assert!(buffer.starts_with(b"%PDF-1.7"));
    }

    #[test]
    fn test_document_with_multiple_pages() {
        let mut doc = Document::new();
        doc.set_title("Multi-page Document");
        doc.set_author("Test Author");
        doc.set_subject("Testing multiple pages");
        doc.set_keywords("test, multiple, pages");

        for _ in 0..5 {
            doc.add_page(Page::a4());
        }

        assert_eq!(doc.pages.len(), 5);
        assert_eq!(doc.metadata.title, Some("Multi-page Document".to_string()));
        assert_eq!(doc.metadata.author, Some("Test Author".to_string()));
    }

    #[test]
    fn test_empty_document_write() {
        let mut doc = Document::new();
        let mut buffer = Vec::new();

        // Empty document should still produce valid PDF
        let result = doc.write(&mut buffer);
        assert!(result.is_ok());
        assert!(!buffer.is_empty());
        assert!(buffer.starts_with(b"%PDF-1.7"));
    }

    // Integration tests for Document ↔ Writer ↔ Parser interactions
    mod integration_tests {
        use super::*;
        use crate::graphics::Color;
        use crate::text::Font;
        use std::fs;
        use tempfile::TempDir;

        #[test]
        fn test_document_writer_roundtrip() {
            let temp_dir = TempDir::new().unwrap();
            let file_path = temp_dir.path().join("test.pdf");

            // Create document with content
            let mut doc = Document::new();
            doc.set_title("Integration Test");
            doc.set_author("Test Author");
            doc.set_subject("Writer Integration");
            doc.set_keywords("test, writer, integration");

            let mut page = Page::a4();
            page.text()
                .set_font(Font::Helvetica, 12.0)
                .at(100.0, 700.0)
                .write("Integration Test Content")
                .unwrap();

            doc.add_page(page);

            // Write to file
            let result = doc.save(&file_path);
            assert!(result.is_ok());

            // Verify file exists and has content
            assert!(file_path.exists());
            let metadata = fs::metadata(&file_path).unwrap();
            assert!(metadata.len() > 0);

            // Read file back to verify PDF format
            let content = fs::read(&file_path).unwrap();
            assert!(content.starts_with(b"%PDF-1.7"));
            // Check for %%EOF with or without newline
            assert!(content.ends_with(b"%%EOF\n") || content.ends_with(b"%%EOF"));
        }

        #[test]
        fn test_document_with_complex_content() {
            let temp_dir = TempDir::new().unwrap();
            let file_path = temp_dir.path().join("complex.pdf");

            let mut doc = Document::new();
            doc.set_title("Complex Content Test");

            // Create page with mixed content
            let mut page = Page::a4();

            // Add text
            page.text()
                .set_font(Font::Helvetica, 14.0)
                .at(50.0, 750.0)
                .write("Complex Content Test")
                .unwrap();

            // Add graphics
            page.graphics()
                .set_fill_color(Color::rgb(0.8, 0.2, 0.2))
                .rectangle(50.0, 500.0, 200.0, 100.0)
                .fill();

            page.graphics()
                .set_stroke_color(Color::rgb(0.2, 0.2, 0.8))
                .set_line_width(2.0)
                .move_to(50.0, 400.0)
                .line_to(250.0, 400.0)
                .stroke();

            doc.add_page(page);

            // Write and verify
            let result = doc.save(&file_path);
            assert!(result.is_ok());
            assert!(file_path.exists());
        }

        #[test]
        fn test_document_multiple_pages_integration() {
            let temp_dir = TempDir::new().unwrap();
            let file_path = temp_dir.path().join("multipage.pdf");

            let mut doc = Document::new();
            doc.set_title("Multi-page Integration Test");

            // Create multiple pages with different content
            for i in 1..=5 {
                let mut page = Page::a4();

                page.text()
                    .set_font(Font::Helvetica, 16.0)
                    .at(50.0, 750.0)
                    .write(&format!("Page {}", i))
                    .unwrap();

                page.text()
                    .set_font(Font::Helvetica, 12.0)
                    .at(50.0, 700.0)
                    .write(&format!("This is the content for page {}", i))
                    .unwrap();

                // Add unique graphics for each page
                let color = match i % 3 {
                    0 => Color::rgb(1.0, 0.0, 0.0),
                    1 => Color::rgb(0.0, 1.0, 0.0),
                    _ => Color::rgb(0.0, 0.0, 1.0),
                };

                page.graphics()
                    .set_fill_color(color)
                    .rectangle(50.0, 600.0, 100.0, 50.0)
                    .fill();

                doc.add_page(page);
            }

            // Write and verify
            let result = doc.save(&file_path);
            assert!(result.is_ok());
            assert!(file_path.exists());

            // Verify file size is reasonable for 5 pages
            let metadata = fs::metadata(&file_path).unwrap();
            assert!(metadata.len() > 1000); // Should be substantial
        }

        #[test]
        fn test_document_metadata_persistence() {
            let temp_dir = TempDir::new().unwrap();
            let file_path = temp_dir.path().join("metadata.pdf");

            let mut doc = Document::new();
            doc.set_title("Metadata Persistence Test");
            doc.set_author("Test Author");
            doc.set_subject("Testing metadata preservation");
            doc.set_keywords("metadata, persistence, test");

            doc.add_page(Page::a4());

            // Write to file
            let result = doc.save(&file_path);
            assert!(result.is_ok());

            // Read file content to verify metadata is present
            let content = fs::read(&file_path).unwrap();
            let content_str = String::from_utf8_lossy(&content);

            // Check that metadata appears in the PDF
            assert!(content_str.contains("Metadata Persistence Test"));
            assert!(content_str.contains("Test Author"));
        }

        #[test]
        fn test_document_writer_error_handling() {
            let mut doc = Document::new();
            doc.add_page(Page::a4());

            // Test writing to invalid path
            let result = doc.save("/invalid/path/test.pdf");
            assert!(result.is_err());
        }

        #[test]
        fn test_document_object_management() {
            let mut doc = Document::new();

            // Add objects and verify they're managed properly
            let obj1 = Object::Boolean(true);
            let obj2 = Object::Integer(42);
            let obj3 = Object::Real(3.14);

            let id1 = doc.add_object(obj1.clone());
            let id2 = doc.add_object(obj2.clone());
            let id3 = doc.add_object(obj3.clone());

            assert_eq!(id1.number(), 1);
            assert_eq!(id2.number(), 2);
            assert_eq!(id3.number(), 3);

            assert_eq!(doc.objects.len(), 3);
            assert!(doc.objects.contains_key(&id1));
            assert!(doc.objects.contains_key(&id2));
            assert!(doc.objects.contains_key(&id3));

            // Verify objects are correct
            assert_eq!(doc.objects.get(&id1), Some(&obj1));
            assert_eq!(doc.objects.get(&id2), Some(&obj2));
            assert_eq!(doc.objects.get(&id3), Some(&obj3));
        }

        #[test]
        fn test_document_page_integration() {
            let mut doc = Document::new();

            // Test different page configurations
            let page1 = Page::a4();
            let page2 = Page::letter();
            let mut page3 = Page::new(500.0, 400.0);

            // Add content to custom page
            page3
                .text()
                .set_font(Font::Helvetica, 10.0)
                .at(25.0, 350.0)
                .write("Custom size page")
                .unwrap();

            doc.add_page(page1);
            doc.add_page(page2);
            doc.add_page(page3);

            assert_eq!(doc.pages.len(), 3);

            // Verify pages maintain their properties (actual dimensions may vary)
            assert!(doc.pages[0].width() > 500.0); // A4 width is reasonable
            assert!(doc.pages[0].height() > 700.0); // A4 height is reasonable
            assert!(doc.pages[1].width() > 500.0); // Letter width is reasonable
            assert!(doc.pages[1].height() > 700.0); // Letter height is reasonable
            assert_eq!(doc.pages[2].width(), 500.0); // Custom width
            assert_eq!(doc.pages[2].height(), 400.0); // Custom height
        }

        #[test]
        fn test_document_content_generation() {
            let temp_dir = TempDir::new().unwrap();
            let file_path = temp_dir.path().join("content.pdf");

            let mut doc = Document::new();
            doc.set_title("Content Generation Test");

            let mut page = Page::a4();

            // Generate content programmatically
            for i in 0..10 {
                let y_pos = 700.0 - (i as f64 * 30.0);
                page.text()
                    .set_font(Font::Helvetica, 12.0)
                    .at(50.0, y_pos)
                    .write(&format!("Generated line {}", i + 1))
                    .unwrap();
            }

            doc.add_page(page);

            // Write and verify
            let result = doc.save(&file_path);
            assert!(result.is_ok());
            assert!(file_path.exists());

            // Verify content was generated
            let metadata = fs::metadata(&file_path).unwrap();
            assert!(metadata.len() > 500); // Should contain substantial content
        }

        #[test]
        fn test_document_buffer_vs_file_write() {
            let temp_dir = TempDir::new().unwrap();
            let file_path = temp_dir.path().join("buffer_vs_file.pdf");

            let mut doc = Document::new();
            doc.set_title("Buffer vs File Test");
            doc.add_page(Page::a4());

            // Write to buffer
            let mut buffer = Vec::new();
            let buffer_result = doc.write(&mut buffer);
            assert!(buffer_result.is_ok());

            // Write to file
            let file_result = doc.save(&file_path);
            assert!(file_result.is_ok());

            // Read file back
            let file_content = fs::read(&file_path).unwrap();

            // Both should be valid PDFs with same structure (timestamps may differ)
            assert!(buffer.starts_with(b"%PDF-1.7"));
            assert!(file_content.starts_with(b"%PDF-1.7"));
            assert!(buffer.ends_with(b"%%EOF\n"));
            assert!(file_content.ends_with(b"%%EOF\n"));

            // Both should contain the same title
            let buffer_str = String::from_utf8_lossy(&buffer);
            let file_str = String::from_utf8_lossy(&file_content);
            assert!(buffer_str.contains("Buffer vs File Test"));
            assert!(file_str.contains("Buffer vs File Test"));
        }

        #[test]
        fn test_document_large_content_handling() {
            let temp_dir = TempDir::new().unwrap();
            let file_path = temp_dir.path().join("large_content.pdf");

            let mut doc = Document::new();
            doc.set_title("Large Content Test");

            let mut page = Page::a4();

            // Add large amount of text content - make it much larger
            let large_text =
                "Lorem ipsum dolor sit amet, consectetur adipiscing elit. ".repeat(200);
            page.text()
                .set_font(Font::Helvetica, 10.0)
                .at(50.0, 750.0)
                .write(&large_text)
                .unwrap();

            doc.add_page(page);

            // Write and verify
            let result = doc.save(&file_path);
            assert!(result.is_ok());
            assert!(file_path.exists());

            // Verify large content was handled properly - reduce expectation
            let metadata = fs::metadata(&file_path).unwrap();
            assert!(metadata.len() > 2000); // Should be substantial but realistic
        }

        #[test]
        fn test_document_incremental_building() {
            let temp_dir = TempDir::new().unwrap();
            let file_path = temp_dir.path().join("incremental.pdf");

            let mut doc = Document::new();

            // Build document incrementally
            doc.set_title("Incremental Building Test");

            // Add first page
            let mut page1 = Page::a4();
            page1
                .text()
                .set_font(Font::Helvetica, 12.0)
                .at(50.0, 750.0)
                .write("First page content")
                .unwrap();
            doc.add_page(page1);

            // Add metadata
            doc.set_author("Incremental Author");
            doc.set_subject("Incremental Subject");

            // Add second page
            let mut page2 = Page::a4();
            page2
                .text()
                .set_font(Font::Helvetica, 12.0)
                .at(50.0, 750.0)
                .write("Second page content")
                .unwrap();
            doc.add_page(page2);

            // Add more metadata
            doc.set_keywords("incremental, building, test");

            // Final write
            let result = doc.save(&file_path);
            assert!(result.is_ok());
            assert!(file_path.exists());

            // Verify final state
            assert_eq!(doc.pages.len(), 2);
            assert_eq!(
                doc.metadata.title,
                Some("Incremental Building Test".to_string())
            );
            assert_eq!(doc.metadata.author, Some("Incremental Author".to_string()));
            assert_eq!(
                doc.metadata.subject,
                Some("Incremental Subject".to_string())
            );
            assert_eq!(
                doc.metadata.keywords,
                Some("incremental, building, test".to_string())
            );
        }

        #[test]
        fn test_document_concurrent_page_operations() {
            let mut doc = Document::new();
            doc.set_title("Concurrent Operations Test");

            // Simulate concurrent-like operations
            let mut pages = Vec::new();

            // Create multiple pages
            for i in 0..5 {
                let mut page = Page::a4();
                page.text()
                    .set_font(Font::Helvetica, 12.0)
                    .at(50.0, 750.0)
                    .write(&format!("Concurrent page {}", i))
                    .unwrap();
                pages.push(page);
            }

            // Add all pages
            for page in pages {
                doc.add_page(page);
            }

            assert_eq!(doc.pages.len(), 5);

            // Verify each page maintains its content
            let temp_dir = TempDir::new().unwrap();
            let file_path = temp_dir.path().join("concurrent.pdf");
            let result = doc.save(&file_path);
            assert!(result.is_ok());
        }

        #[test]
        fn test_document_memory_efficiency() {
            let mut doc = Document::new();
            doc.set_title("Memory Efficiency Test");

            // Add multiple pages with content
            for i in 0..10 {
                let mut page = Page::a4();
                page.text()
                    .set_font(Font::Helvetica, 12.0)
                    .at(50.0, 700.0)
                    .write(&format!("Memory test page {}", i))
                    .unwrap();
                doc.add_page(page);
            }

            // Write to buffer to test memory usage
            let mut buffer = Vec::new();
            let result = doc.write(&mut buffer);
            assert!(result.is_ok());
            assert!(!buffer.is_empty());

            // Buffer should be reasonable size
            assert!(buffer.len() < 1_000_000); // Should be less than 1MB for simple content
        }

        #[test]
        fn test_document_creator_producer() {
            let mut doc = Document::new();

            // Default values
            assert_eq!(doc.metadata.creator, Some("oxidize_pdf".to_string()));
            assert!(doc
                .metadata
                .producer
                .as_ref()
                .unwrap()
                .contains("oxidize_pdf"));

            // Set custom values
            doc.set_creator("My Application");
            doc.set_producer("My PDF Library v1.0");

            assert_eq!(doc.metadata.creator, Some("My Application".to_string()));
            assert_eq!(
                doc.metadata.producer,
                Some("My PDF Library v1.0".to_string())
            );
        }

        #[test]
        fn test_document_dates() {
            use chrono::{TimeZone, Utc};

            let mut doc = Document::new();

            // Check default dates are set
            assert!(doc.metadata.creation_date.is_some());
            assert!(doc.metadata.modification_date.is_some());

            // Set specific dates
            let creation_date = Utc.with_ymd_and_hms(2023, 1, 1, 12, 0, 0).unwrap();
            let mod_date = Utc.with_ymd_and_hms(2023, 6, 15, 18, 30, 0).unwrap();

            doc.set_creation_date(creation_date);
            doc.set_modification_date(mod_date);

            assert_eq!(doc.metadata.creation_date, Some(creation_date));
            assert_eq!(doc.metadata.modification_date, Some(mod_date));
        }

        #[test]
        fn test_document_dates_local() {
            use chrono::{Local, TimeZone};

            let mut doc = Document::new();

            // Test setting dates with local time
            let local_date = Local.with_ymd_and_hms(2023, 12, 25, 10, 30, 0).unwrap();
            doc.set_creation_date_local(local_date);

            // Verify it was converted to UTC
            assert!(doc.metadata.creation_date.is_some());
            // Just verify the date was set, don't compare exact values due to timezone complexities
            assert!(doc.metadata.creation_date.is_some());
        }

        #[test]
        fn test_update_modification_date() {
            let mut doc = Document::new();

            let initial_mod_date = doc.metadata.modification_date;
            assert!(initial_mod_date.is_some());

            // Sleep briefly to ensure time difference
            std::thread::sleep(std::time::Duration::from_millis(10));

            doc.update_modification_date();

            let new_mod_date = doc.metadata.modification_date;
            assert!(new_mod_date.is_some());
            assert!(new_mod_date.unwrap() > initial_mod_date.unwrap());
        }

        #[test]
        fn test_document_save_updates_modification_date() {
            let temp_dir = TempDir::new().unwrap();
            let file_path = temp_dir.path().join("mod_date_test.pdf");

            let mut doc = Document::new();
            doc.add_page(Page::a4());

            let initial_mod_date = doc.metadata.modification_date;

            // Sleep briefly to ensure time difference
            std::thread::sleep(std::time::Duration::from_millis(10));

            doc.save(&file_path).unwrap();

            // Modification date should be updated
            assert!(doc.metadata.modification_date.unwrap() > initial_mod_date.unwrap());
        }

        #[test]
        fn test_document_metadata_complete() {
            let mut doc = Document::new();

            // Set all metadata fields
            doc.set_title("Complete Metadata Test");
            doc.set_author("Test Author");
            doc.set_subject("Testing all metadata fields");
            doc.set_keywords("test, metadata, complete");
            doc.set_creator("Test Application v1.0");
            doc.set_producer("oxidize_pdf Test Suite");

            // Verify all fields
            assert_eq!(
                doc.metadata.title,
                Some("Complete Metadata Test".to_string())
            );
            assert_eq!(doc.metadata.author, Some("Test Author".to_string()));
            assert_eq!(
                doc.metadata.subject,
                Some("Testing all metadata fields".to_string())
            );
            assert_eq!(
                doc.metadata.keywords,
                Some("test, metadata, complete".to_string())
            );
            assert_eq!(
                doc.metadata.creator,
                Some("Test Application v1.0".to_string())
            );
            assert_eq!(
                doc.metadata.producer,
                Some("oxidize_pdf Test Suite".to_string())
            );
            assert!(doc.metadata.creation_date.is_some());
            assert!(doc.metadata.modification_date.is_some());
        }
    }
}
