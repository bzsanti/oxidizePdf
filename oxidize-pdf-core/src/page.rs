use crate::error::Result;
use crate::graphics::{GraphicsContext, Image};
use crate::text::{Font, TextContext, TextFlowContext};
use std::collections::{HashMap, HashSet};

/// Page margins in points (1/72 inch).
#[derive(Clone, Debug)]
pub struct Margins {
    /// Left margin
    pub left: f64,
    /// Right margin
    pub right: f64,
    /// Top margin
    pub top: f64,
    /// Bottom margin
    pub bottom: f64,
}

impl Default for Margins {
    fn default() -> Self {
        Self {
            left: 72.0,   // 1 inch
            right: 72.0,  // 1 inch
            top: 72.0,    // 1 inch
            bottom: 72.0, // 1 inch
        }
    }
}

/// A single page in a PDF document.
///
/// Pages have a size (width and height in points), margins, and can contain
/// graphics, text, and images.
///
/// # Example
///
/// ```rust
/// use oxidize_pdf::{Page, Font, Color};
///
/// let mut page = Page::a4();
///
/// // Add text
/// page.text()
///     .set_font(Font::Helvetica, 12.0)
///     .at(100.0, 700.0)
///     .write("Hello World")?;
///
/// // Add graphics
/// page.graphics()
///     .set_fill_color(Color::red())
///     .rect(100.0, 100.0, 200.0, 150.0)
///     .fill();
/// # Ok::<(), oxidize_pdf::PdfError>(())
/// ```
#[derive(Clone)]
pub struct Page {
    width: f64,
    height: f64,
    margins: Margins,
    content: Vec<u8>,
    graphics_context: GraphicsContext,
    text_context: TextContext,
    images: HashMap<String, Image>,
}

impl Page {
    /// Creates a new page with the specified width and height in points.
    ///
    /// Points are 1/72 of an inch.
    pub fn new(width: f64, height: f64) -> Self {
        Self {
            width,
            height,
            margins: Margins::default(),
            content: Vec::new(),
            graphics_context: GraphicsContext::new(),
            text_context: TextContext::new(),
            images: HashMap::new(),
        }
    }

    /// Creates a new A4 page (595 x 842 points).
    pub fn a4() -> Self {
        Self::new(595.0, 842.0)
    }

    /// Creates a new US Letter page (612 x 792 points).
    pub fn letter() -> Self {
        Self::new(612.0, 792.0)
    }

    /// Creates a new US Legal page (612 x 1008 points).
    pub fn legal() -> Self {
        Self::new(612.0, 1008.0)
    }

    /// Returns a mutable reference to the graphics context for drawing shapes.
    pub fn graphics(&mut self) -> &mut GraphicsContext {
        &mut self.graphics_context
    }

    /// Returns a mutable reference to the text context for adding text.
    pub fn text(&mut self) -> &mut TextContext {
        &mut self.text_context
    }

    pub fn set_margins(&mut self, left: f64, right: f64, top: f64, bottom: f64) {
        self.margins = Margins {
            left,
            right,
            top,
            bottom,
        };
    }

    pub fn margins(&self) -> &Margins {
        &self.margins
    }

    pub fn content_width(&self) -> f64 {
        self.width - self.margins.left - self.margins.right
    }

    pub fn content_height(&self) -> f64 {
        self.height - self.margins.top - self.margins.bottom
    }

    pub fn content_area(&self) -> (f64, f64, f64, f64) {
        (
            self.margins.left,
            self.margins.bottom,
            self.width - self.margins.right,
            self.height - self.margins.top,
        )
    }

    pub(crate) fn width(&self) -> f64 {
        self.width
    }

    pub(crate) fn height(&self) -> f64 {
        self.height
    }

    pub fn text_flow(&self) -> TextFlowContext {
        TextFlowContext::new(self.width, self.height, self.margins.clone())
    }

    pub fn add_text_flow(&mut self, text_flow: &TextFlowContext) {
        let operations = text_flow.generate_operations();
        self.content.extend_from_slice(&operations);
    }

    pub fn add_image(&mut self, name: impl Into<String>, image: Image) {
        self.images.insert(name.into(), image);
    }

    pub fn draw_image(
        &mut self,
        name: &str,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
    ) -> Result<()> {
        if self.images.contains_key(name) {
            self.graphics_context.draw_image(name, x, y, width, height);
            Ok(())
        } else {
            Err(crate::PdfError::InvalidReference(format!(
                "Image '{name}' not found"
            )))
        }
    }

    pub(crate) fn images(&self) -> &HashMap<String, Image> {
        &self.images
    }

    pub(crate) fn generate_content(&mut self) -> Result<Vec<u8>> {
        // Don't clear content, as it may contain text flow operations
        let mut final_content = Vec::new();

        // Add graphics operations
        final_content.extend_from_slice(&self.graphics_context.generate_operations()?);

        // Add text operations
        final_content.extend_from_slice(&self.text_context.generate_operations()?);

        // Add any content that was added via add_text_flow
        final_content.extend_from_slice(&self.content);

        Ok(final_content)
    }

    /// Gets all fonts used in this page.
    ///
    /// This method scans the page content to identify which fonts are being used.
    /// For now, it returns a simple set based on the current text context font,
    /// but in a full implementation it would parse all text operations.
    pub(crate) fn get_used_fonts(&self) -> Vec<Font> {
        let mut fonts = HashSet::new();

        // Add the current font from text context
        fonts.insert(self.text_context.current_font());

        // TODO: In a full implementation, we would:
        // 1. Parse the content stream to find all Tf (set font) operations
        // 2. Extract font names from those operations
        // 3. Map them back to Font enum values
        // For now, we'll just return the fonts we know are commonly used

        // Add some commonly used fonts as a baseline
        fonts.insert(Font::Helvetica);
        fonts.insert(Font::TimesRoman);
        fonts.insert(Font::Courier);

        fonts.into_iter().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graphics::Color;
    use crate::text::Font;

    #[test]
    fn test_page_new() {
        let page = Page::new(100.0, 200.0);
        assert_eq!(page.width(), 100.0);
        assert_eq!(page.height(), 200.0);
        assert_eq!(page.margins().left, 72.0);
        assert_eq!(page.margins().right, 72.0);
        assert_eq!(page.margins().top, 72.0);
        assert_eq!(page.margins().bottom, 72.0);
    }

    #[test]
    fn test_page_a4() {
        let page = Page::a4();
        assert_eq!(page.width(), 595.0);
        assert_eq!(page.height(), 842.0);
    }

    #[test]
    fn test_page_letter() {
        let page = Page::letter();
        assert_eq!(page.width(), 612.0);
        assert_eq!(page.height(), 792.0);
    }

    #[test]
    fn test_set_margins() {
        let mut page = Page::a4();
        page.set_margins(10.0, 20.0, 30.0, 40.0);

        assert_eq!(page.margins().left, 10.0);
        assert_eq!(page.margins().right, 20.0);
        assert_eq!(page.margins().top, 30.0);
        assert_eq!(page.margins().bottom, 40.0);
    }

    #[test]
    fn test_content_dimensions() {
        let mut page = Page::new(300.0, 400.0);
        page.set_margins(50.0, 50.0, 50.0, 50.0);

        assert_eq!(page.content_width(), 200.0);
        assert_eq!(page.content_height(), 300.0);
    }

    #[test]
    fn test_content_area() {
        let mut page = Page::new(300.0, 400.0);
        page.set_margins(10.0, 20.0, 30.0, 40.0);

        let (left, bottom, right, top) = page.content_area();
        assert_eq!(left, 10.0);
        assert_eq!(bottom, 40.0);
        assert_eq!(right, 280.0);
        assert_eq!(top, 370.0);
    }

    #[test]
    fn test_graphics_context() {
        let mut page = Page::a4();
        let graphics = page.graphics();
        graphics.set_fill_color(Color::red());
        graphics.rect(100.0, 100.0, 200.0, 150.0);
        graphics.fill();

        // Graphics context should be accessible and modifiable
        assert!(page.generate_content().is_ok());
    }

    #[test]
    fn test_text_context() {
        let mut page = Page::a4();
        let text = page.text();
        text.set_font(Font::Helvetica, 12.0);
        text.at(100.0, 700.0);
        text.write("Hello World").unwrap();

        // Text context should be accessible and modifiable
        assert!(page.generate_content().is_ok());
    }

    #[test]
    fn test_text_flow() {
        let page = Page::a4();
        let text_flow = page.text_flow();

        // Text flow should be created with page dimensions and margins
        // Just verify it can be created
        drop(text_flow);
    }

    #[test]
    fn test_add_text_flow() {
        let mut page = Page::a4();
        let mut text_flow = page.text_flow();
        text_flow.at(100.0, 700.0);
        text_flow.set_font(Font::TimesRoman, 14.0);
        text_flow.write_wrapped("Test text flow").unwrap();

        page.add_text_flow(&text_flow);

        let content = page.generate_content().unwrap();
        assert!(!content.is_empty());
    }

    #[test]
    fn test_add_image() {
        let mut page = Page::a4();
        // Create a minimal valid JPEG with SOF0 header
        let jpeg_data = vec![
            0xFF, 0xD8, // SOI marker
            0xFF, 0xC0, // SOF0 marker
            0x00, 0x11, // Length (17 bytes)
            0x08, // Precision (8 bits)
            0x00, 0x64, // Height (100)
            0x00, 0xC8, // Width (200)
            0x03, // Components (3 = RGB)
            0xFF, 0xD9, // EOI marker
        ];
        let image = Image::from_jpeg_data(jpeg_data).unwrap();

        page.add_image("test_image", image.clone());
        assert!(page.images().contains_key("test_image"));
        assert_eq!(page.images().len(), 1);
    }

    #[test]
    fn test_draw_image() {
        let mut page = Page::a4();
        // Create a minimal valid JPEG with SOF0 header
        let jpeg_data = vec![
            0xFF, 0xD8, // SOI marker
            0xFF, 0xC0, // SOF0 marker
            0x00, 0x11, // Length (17 bytes)
            0x08, // Precision (8 bits)
            0x00, 0x64, // Height (100)
            0x00, 0xC8, // Width (200)
            0x03, // Components (3 = RGB)
            0xFF, 0xD9, // EOI marker
        ];
        let image = Image::from_jpeg_data(jpeg_data).unwrap();

        page.add_image("test_image", image);
        let result = page.draw_image("test_image", 50.0, 50.0, 200.0, 200.0);
        assert!(result.is_ok());
    }

    #[test]
    fn test_draw_nonexistent_image() {
        let mut page = Page::a4();
        let result = page.draw_image("nonexistent", 50.0, 50.0, 200.0, 200.0);
        assert!(result.is_err());
    }

    #[test]
    fn test_generate_content() {
        let mut page = Page::a4();

        // Add some graphics
        page.graphics()
            .set_fill_color(Color::blue())
            .circle(200.0, 400.0, 50.0)
            .fill();

        // Add some text
        page.text()
            .set_font(Font::Courier, 10.0)
            .at(50.0, 650.0)
            .write("Test content")
            .unwrap();

        let content = page.generate_content().unwrap();
        assert!(!content.is_empty());
    }

    #[test]
    fn test_margins_default() {
        let margins = Margins::default();
        assert_eq!(margins.left, 72.0);
        assert_eq!(margins.right, 72.0);
        assert_eq!(margins.top, 72.0);
        assert_eq!(margins.bottom, 72.0);
    }

    #[test]
    fn test_page_clone() {
        let mut page1 = Page::a4();
        page1.set_margins(10.0, 20.0, 30.0, 40.0);
        // Create a minimal valid JPEG with SOF0 header
        let jpeg_data = vec![
            0xFF, 0xD8, // SOI marker
            0xFF, 0xC0, // SOF0 marker
            0x00, 0x11, // Length (17 bytes)
            0x08, // Precision (8 bits)
            0x00, 0x32, // Height (50)
            0x00, 0x32, // Width (50)
            0x03, // Components (3 = RGB)
            0xFF, 0xD9, // EOI marker
        ];
        let image = Image::from_jpeg_data(jpeg_data).unwrap();
        page1.add_image("img1", image);

        let page2 = page1.clone();
        assert_eq!(page2.width(), page1.width());
        assert_eq!(page2.height(), page1.height());
        assert_eq!(page2.margins().left, page1.margins().left);
        assert_eq!(page2.images().len(), page1.images().len());
    }

    // Integration tests for Page ↔ Document ↔ Writer interactions
    mod integration_tests {
        use super::*;
        use crate::document::Document;
        use crate::writer::PdfWriter;
        use std::fs;
        use tempfile::TempDir;

        #[test]
        fn test_page_document_integration() {
            let mut doc = Document::new();
            doc.set_title("Page Integration Test");

            // Create pages with different sizes
            let page1 = Page::a4();
            let page2 = Page::letter();
            let mut page3 = Page::new(400.0, 600.0);

            // Add content to custom page
            page3.set_margins(20.0, 20.0, 20.0, 20.0);
            page3
                .text()
                .set_font(Font::Helvetica, 14.0)
                .at(50.0, 550.0)
                .write("Custom page content")
                .unwrap();

            doc.add_page(page1);
            doc.add_page(page2);
            doc.add_page(page3);

            assert_eq!(doc.pages.len(), 3);

            // Verify page properties are preserved
            assert_eq!(doc.pages[0].width(), 595.0); // A4
            assert_eq!(doc.pages[1].width(), 612.0); // Letter
            assert_eq!(doc.pages[2].width(), 400.0); // Custom

            // Verify content generation works
            let mut page_copy = doc.pages[2].clone();
            let content = page_copy.generate_content().unwrap();
            assert!(!content.is_empty());
        }

        #[test]
        fn test_page_writer_integration() {
            let temp_dir = TempDir::new().unwrap();
            let file_path = temp_dir.path().join("page_writer_test.pdf");

            let mut doc = Document::new();
            doc.set_title("Page Writer Integration");

            // Create a page with complex content
            let mut page = Page::a4();
            page.set_margins(50.0, 50.0, 50.0, 50.0);

            // Add text content
            page.text()
                .set_font(Font::Helvetica, 16.0)
                .at(100.0, 750.0)
                .write("Integration Test Header")
                .unwrap();

            page.text()
                .set_font(Font::TimesRoman, 12.0)
                .at(100.0, 700.0)
                .write("This is body text for the integration test.")
                .unwrap();

            // Add graphics content
            page.graphics()
                .set_fill_color(Color::rgb(0.2, 0.6, 0.9))
                .rect(100.0, 600.0, 200.0, 50.0)
                .fill();

            page.graphics()
                .set_stroke_color(Color::rgb(0.8, 0.2, 0.2))
                .set_line_width(3.0)
                .circle(300.0, 500.0, 40.0)
                .stroke();

            doc.add_page(page);

            // Write to file
            let mut writer = PdfWriter::new(&file_path).unwrap();
            writer.write_document(&mut doc).unwrap();

            // Verify file was created and has content
            assert!(file_path.exists());
            let metadata = fs::metadata(&file_path).unwrap();
            assert!(metadata.len() > 1000); // Should be substantial

            // Verify PDF structure (text may be compressed, so check for basic structure)
            let content = fs::read(&file_path).unwrap();
            let content_str = String::from_utf8_lossy(&content);
            assert!(content_str.contains("obj")); // Should contain PDF objects
            assert!(content_str.contains("stream")); // Should contain content streams
        }

        #[test]
        fn test_page_margins_integration() {
            let temp_dir = TempDir::new().unwrap();
            let file_path = temp_dir.path().join("margins_test.pdf");

            let mut doc = Document::new();
            doc.set_title("Margins Integration Test");

            // Test different margin configurations
            let mut page1 = Page::a4();
            page1.set_margins(10.0, 20.0, 30.0, 40.0);

            let mut page2 = Page::letter();
            page2.set_margins(72.0, 72.0, 72.0, 72.0); // 1 inch margins

            let mut page3 = Page::new(500.0, 700.0);
            page3.set_margins(0.0, 0.0, 0.0, 0.0); // No margins

            // Add content that uses margin information
            for (i, page) in [&mut page1, &mut page2, &mut page3].iter_mut().enumerate() {
                let (left, bottom, right, top) = page.content_area();

                // Place text at content area boundaries
                page.text()
                    .set_font(Font::Helvetica, 10.0)
                    .at(left, top - 20.0)
                    .write(&format!(
                        "Page {} - Content area: ({:.1}, {:.1}, {:.1}, {:.1})",
                        i + 1,
                        left,
                        bottom,
                        right,
                        top
                    ))
                    .unwrap();

                // Draw border around content area
                page.graphics()
                    .set_stroke_color(Color::rgb(0.5, 0.5, 0.5))
                    .set_line_width(1.0)
                    .rect(left, bottom, right - left, top - bottom)
                    .stroke();
            }

            doc.add_page(page1);
            doc.add_page(page2);
            doc.add_page(page3);

            // Write and verify
            let mut writer = PdfWriter::new(&file_path).unwrap();
            writer.write_document(&mut doc).unwrap();

            assert!(file_path.exists());
            let metadata = fs::metadata(&file_path).unwrap();
            assert!(metadata.len() > 2000); // Should contain substantial content
        }

        #[test]
        fn test_page_image_integration() {
            let temp_dir = TempDir::new().unwrap();
            let file_path = temp_dir.path().join("image_test.pdf");

            let mut doc = Document::new();
            doc.set_title("Image Integration Test");

            let mut page = Page::a4();

            // Create test images
            let jpeg_data1 = vec![
                0xFF, 0xD8, 0xFF, 0xC0, 0x00, 0x11, 0x08, 0x00, 0x64, 0x00, 0xC8, 0x03, 0xFF, 0xD9,
            ];
            let image1 = Image::from_jpeg_data(jpeg_data1).unwrap();

            let jpeg_data2 = vec![
                0xFF, 0xD8, 0xFF, 0xC0, 0x00, 0x11, 0x08, 0x00, 0x32, 0x00, 0x32, 0x01, 0xFF, 0xD9,
            ];
            let image2 = Image::from_jpeg_data(jpeg_data2).unwrap();

            // Add images to page
            page.add_image("image1", image1);
            page.add_image("image2", image2);

            // Draw images at different positions
            page.draw_image("image1", 100.0, 600.0, 200.0, 100.0)
                .unwrap();
            page.draw_image("image2", 350.0, 600.0, 50.0, 50.0).unwrap();

            // Add text labels
            page.text()
                .set_font(Font::Helvetica, 12.0)
                .at(100.0, 580.0)
                .write("Image 1 (200x100)")
                .unwrap();

            page.text()
                .set_font(Font::Helvetica, 12.0)
                .at(350.0, 580.0)
                .write("Image 2 (50x50)")
                .unwrap();

            doc.add_page(page);

            // Write and verify
            let mut writer = PdfWriter::new(&file_path).unwrap();
            writer.write_document(&mut doc).unwrap();

            assert!(file_path.exists());
            let metadata = fs::metadata(&file_path).unwrap();
            assert!(metadata.len() > 1500); // Should contain images and text

            // Verify XObject references in PDF
            let content = fs::read(&file_path).unwrap();
            let content_str = String::from_utf8_lossy(&content);
            assert!(content_str.contains("XObject"));
        }

        #[test]
        fn test_page_text_flow_integration() {
            let temp_dir = TempDir::new().unwrap();
            let file_path = temp_dir.path().join("text_flow_test.pdf");

            let mut doc = Document::new();
            doc.set_title("Text Flow Integration Test");

            let mut page = Page::a4();
            page.set_margins(50.0, 50.0, 50.0, 50.0);

            // Create text flow with long content
            let mut text_flow = page.text_flow();
            text_flow.set_font(Font::TimesRoman, 12.0);
            text_flow.at(100.0, 700.0);

            let long_text =
                "This is a long paragraph that should demonstrate text flow capabilities. "
                    .repeat(10);
            text_flow.write_wrapped(&long_text).unwrap();

            // Add the text flow to the page
            page.add_text_flow(&text_flow);

            // Also add regular text
            page.text()
                .set_font(Font::Helvetica, 14.0)
                .at(100.0, 750.0)
                .write("Regular Text Above Text Flow")
                .unwrap();

            doc.add_page(page);

            // Write and verify
            let mut writer = PdfWriter::new(&file_path).unwrap();
            writer.write_document(&mut doc).unwrap();

            assert!(file_path.exists());
            let metadata = fs::metadata(&file_path).unwrap();
            assert!(metadata.len() > 1000); // Should contain text content

            // Verify text structure appears in PDF
            let content = fs::read(&file_path).unwrap();
            let content_str = String::from_utf8_lossy(&content);
            assert!(content_str.contains("obj")); // Should contain PDF objects
            assert!(content_str.contains("stream")); // Should contain content streams
        }

        #[test]
        fn test_page_complex_content_integration() {
            let temp_dir = TempDir::new().unwrap();
            let file_path = temp_dir.path().join("complex_content_test.pdf");

            let mut doc = Document::new();
            doc.set_title("Complex Content Integration Test");

            let mut page = Page::a4();
            page.set_margins(40.0, 40.0, 40.0, 40.0);

            // Create complex layered content

            // Background graphics
            page.graphics()
                .set_fill_color(Color::rgb(0.95, 0.95, 0.95))
                .rect(50.0, 50.0, 495.0, 742.0)
                .fill();

            // Header section
            page.graphics()
                .set_fill_color(Color::rgb(0.2, 0.4, 0.8))
                .rect(50.0, 750.0, 495.0, 42.0)
                .fill();

            page.text()
                .set_font(Font::HelveticaBold, 18.0)
                .at(60.0, 765.0)
                .write("Complex Content Integration Test")
                .unwrap();

            // Content sections with mixed elements
            let mut y_pos = 700.0;
            for i in 1..=3 {
                // Section header
                page.graphics()
                    .set_fill_color(Color::rgb(0.8, 0.8, 0.9))
                    .rect(60.0, y_pos, 475.0, 20.0)
                    .fill();

                page.text()
                    .set_font(Font::HelveticaBold, 12.0)
                    .at(70.0, y_pos + 5.0)
                    .write(&format!("Section {}", i))
                    .unwrap();

                y_pos -= 30.0;

                // Section content
                page.text()
                    .set_font(Font::TimesRoman, 10.0)
                    .at(70.0, y_pos)
                    .write(&format!(
                        "This is the content for section {}. It demonstrates mixed content.",
                        i
                    ))
                    .unwrap();

                // Section graphics
                page.graphics()
                    .set_stroke_color(Color::rgb(0.6, 0.2, 0.2))
                    .set_line_width(2.0)
                    .move_to(70.0, y_pos - 10.0)
                    .line_to(530.0, y_pos - 10.0)
                    .stroke();

                y_pos -= 50.0;
            }

            // Footer
            page.graphics()
                .set_fill_color(Color::rgb(0.3, 0.3, 0.3))
                .rect(50.0, 50.0, 495.0, 30.0)
                .fill();

            page.text()
                .set_font(Font::Helvetica, 10.0)
                .at(60.0, 60.0)
                .write("Generated by oxidize-pdf integration test")
                .unwrap();

            doc.add_page(page);

            // Write and verify
            let mut writer = PdfWriter::new(&file_path).unwrap();
            writer.write_document(&mut doc).unwrap();

            assert!(file_path.exists());
            let metadata = fs::metadata(&file_path).unwrap();
            assert!(metadata.len() > 2000); // Should contain substantial content

            // Verify content structure (text may be compressed, so check for basic structure)
            let content = fs::read(&file_path).unwrap();
            let content_str = String::from_utf8_lossy(&content);
            assert!(content_str.contains("obj")); // Should contain PDF objects
            assert!(content_str.contains("stream")); // Should contain content streams
            assert!(content_str.contains("endobj")); // Should contain object endings
        }

        #[test]
        fn test_page_content_generation_performance() {
            let mut page = Page::a4();

            // Add many elements to test performance
            for i in 0..100 {
                let y = 800.0 - (i as f64 * 7.0);
                if y > 50.0 {
                    page.text()
                        .set_font(Font::Helvetica, 8.0)
                        .at(50.0, y)
                        .write(&format!("Performance test line {}", i))
                        .unwrap();
                }
            }

            // Add graphics elements
            for i in 0..50 {
                let x = 50.0 + (i as f64 * 10.0);
                if x < 550.0 {
                    page.graphics()
                        .set_fill_color(Color::rgb(0.5, 0.5, 0.8))
                        .rect(x, 400.0, 8.0, 8.0)
                        .fill();
                }
            }

            // Content generation should complete in reasonable time
            let start = std::time::Instant::now();
            let content = page.generate_content().unwrap();
            let duration = start.elapsed();

            assert!(!content.is_empty());
            assert!(duration.as_millis() < 1000); // Should complete within 1 second
        }

        #[test]
        fn test_page_error_handling() {
            let mut page = Page::a4();

            // Test drawing non-existent image
            let result = page.draw_image("nonexistent", 100.0, 100.0, 50.0, 50.0);
            assert!(result.is_err());

            // Test with invalid parameters - should still work
            let result = page.draw_image("still_nonexistent", -100.0, -100.0, 0.0, 0.0);
            assert!(result.is_err());

            // Add an image and test valid drawing
            let jpeg_data = vec![
                0xFF, 0xD8, 0xFF, 0xC0, 0x00, 0x11, 0x08, 0x00, 0x32, 0x00, 0x32, 0x01, 0xFF, 0xD9,
            ];
            let image = Image::from_jpeg_data(jpeg_data).unwrap();
            page.add_image("valid_image", image);

            let result = page.draw_image("valid_image", 100.0, 100.0, 50.0, 50.0);
            assert!(result.is_ok());
        }

        #[test]
        fn test_page_memory_management() {
            let mut pages = Vec::new();

            // Create many pages to test memory usage
            for i in 0..100 {
                let mut page = Page::a4();
                page.set_margins(i as f64, i as f64, i as f64, i as f64);

                page.text()
                    .set_font(Font::Helvetica, 12.0)
                    .at(100.0, 700.0)
                    .write(&format!("Page {}", i))
                    .unwrap();

                pages.push(page);
            }

            // All pages should be valid
            assert_eq!(pages.len(), 100);

            // Content generation should work for all pages
            for page in pages.iter_mut() {
                let content = page.generate_content().unwrap();
                assert!(!content.is_empty());
            }
        }

        #[test]
        fn test_page_standard_sizes() {
            let a4 = Page::a4();
            let letter = Page::letter();
            let custom = Page::new(200.0, 300.0);

            // Test standard dimensions
            assert_eq!(a4.width(), 595.0);
            assert_eq!(a4.height(), 842.0);
            assert_eq!(letter.width(), 612.0);
            assert_eq!(letter.height(), 792.0);
            assert_eq!(custom.width(), 200.0);
            assert_eq!(custom.height(), 300.0);

            // Test content areas with default margins
            let a4_content_width = a4.content_width();
            let letter_content_width = letter.content_width();
            let custom_content_width = custom.content_width();

            assert_eq!(a4_content_width, 595.0 - 144.0); // 595 - 2*72
            assert_eq!(letter_content_width, 612.0 - 144.0); // 612 - 2*72
            assert_eq!(custom_content_width, 200.0 - 144.0); // 200 - 2*72
        }
    }
}
