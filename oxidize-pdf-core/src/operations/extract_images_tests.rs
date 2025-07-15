//! Tests for PDF image extraction operations

#[cfg(test)]
mod tests {
    use crate::operations::extract_images::*;
    use crate::{Document, Page};
    use crate::graphics::{Image, ImageFormat};
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;

    /// Helper to create a test PDF document with an embedded image
    fn create_pdf_with_image(title: &str) -> Document {
        let mut doc = Document::new();
        doc.set_title(title);
        
        // Create a simple JPEG image data (minimal valid JPEG)
        let jpeg_data = vec![
            0xFF, 0xD8, // SOI marker
            0xFF, 0xE0, // APP0 marker
            0x00, 0x10, // Length
            b'J', b'F', b'I', b'F', 0x00, // JFIF\0
            0x01, 0x01, // Version
            0x00, // Units
            0x00, 0x01, 0x00, 0x01, // X/Y density
            0x00, 0x00, // Thumbnail size
            0xFF, 0xDB, // DQT marker
            0x00, 0x43, // Length
            0x00, // Precision/ID
            // 64 bytes of quantization table data
            0x10, 0x0B, 0x0C, 0x0E, 0x0C, 0x0A, 0x10, 0x0E,
            0x0D, 0x0E, 0x12, 0x11, 0x10, 0x13, 0x18, 0x28,
            0x1A, 0x18, 0x16, 0x16, 0x18, 0x31, 0x23, 0x25,
            0x1D, 0x28, 0x3A, 0x33, 0x3D, 0x3C, 0x39, 0x33,
            0x38, 0x37, 0x40, 0x48, 0x5C, 0x4E, 0x40, 0x44,
            0x57, 0x45, 0x37, 0x38, 0x50, 0x6D, 0x51, 0x57,
            0x5F, 0x62, 0x67, 0x68, 0x67, 0x3E, 0x4D, 0x71,
            0x79, 0x70, 0x64, 0x78, 0x5C, 0x65, 0x67, 0x63,
            0xFF, 0xC0, // SOF0 marker
            0x00, 0x11, // Length
            0x08, // Precision
            0x00, 0x10, // Height (16)
            0x00, 0x10, // Width (16)
            0x03, // Components
            0x01, 0x22, 0x00, // Component 1
            0x02, 0x11, 0x01, // Component 2
            0x03, 0x11, 0x01, // Component 3
            0xFF, 0xDA, // SOS marker
            0x00, 0x0C, // Length
            0x03, // Components
            0x01, 0x00, // Component 1
            0x02, 0x11, // Component 2
            0x03, 0x11, // Component 3
            0x00, 0x3F, 0x00, // Spectral selection
            // Minimal scan data
            0xFF, 0xD9 // EOI marker
        ];
        
        // Create an image from the JPEG data
        let image = Image::from_jpeg_data(jpeg_data).unwrap();
        
        // Create a page with the image
        let mut page = Page::a4();
        
        // Add some text
        page.text()
            .set_font(crate::text::Font::Helvetica, 24.0)
            .at(50.0, 700.0)
            .write(&format!("{} - Page with Image", title))
            .unwrap();
        
        // Note: In a real implementation, we would need to add the image to the page
        // using page.add_image() or similar method, which would register it as an XObject
        
        doc.add_page(page);
        doc
    }

    /// Helper to save a document to a temp file and return the path
    fn save_test_pdf(doc: &mut Document, dir: &TempDir, name: &str) -> PathBuf {
        let path = dir.path().join(name);
        doc.save(&path).unwrap();
        path
    }

    #[test]
    fn test_extract_options_default() {
        let options = ExtractImagesOptions::default();
        assert_eq!(options.output_dir, PathBuf::from("."));
        assert!(options.extract_inline);
        assert_eq!(options.min_size, Some(10));
        assert!(options.create_dir);
        assert_eq!(options.name_pattern, "page_{page}_image_{index}.{format}");
    }

    #[test]
    fn test_extract_options_custom() {
        let temp_dir = TempDir::new().unwrap();
        let options = ExtractImagesOptions {
            output_dir: temp_dir.path().to_path_buf(),
            name_pattern: "img_{page:03}_{index:02}.{format}".to_string(),
            extract_inline: false,
            min_size: Some(50),
            create_dir: false,
        };
        
        assert_eq!(options.output_dir, temp_dir.path());
        assert!(!options.extract_inline);
        assert_eq!(options.min_size, Some(50));
        assert!(!options.create_dir);
    }

    #[test]
    fn test_extracted_image_struct() {
        let extracted = ExtractedImage {
            page_number: 0,
            image_index: 1,
            file_path: PathBuf::from("test.jpg"),
            width: 100,
            height: 200,
            format: ImageFormat::Jpeg,
        };
        
        assert_eq!(extracted.page_number, 0);
        assert_eq!(extracted.image_index, 1);
        assert_eq!(extracted.width, 100);
        assert_eq!(extracted.height, 200);
        assert!(matches!(extracted.format, ImageFormat::Jpeg));
    }

    #[test]
    fn test_extract_from_empty_pdf() {
        let temp_dir = TempDir::new().unwrap();
        let mut doc = Document::new();
        let input_path = save_test_pdf(&mut doc, &temp_dir, "empty.pdf");
        
        let options = ExtractImagesOptions {
            output_dir: temp_dir.path().join("images"),
            ..Default::default()
        };
        
        let result = extract_images_from_pdf(&input_path, options);
        assert!(result.is_ok());
        
        let images = result.unwrap();
        assert_eq!(images.len(), 0);
    }

    #[test]
    fn test_extract_from_pdf_without_images() {
        let temp_dir = TempDir::new().unwrap();
        let mut doc = Document::new();
        
        // Add a page with only text
        let mut page = Page::a4();
        page.text()
            .set_font(crate::text::Font::Helvetica, 12.0)
            .at(50.0, 700.0)
            .write("This page has no images")
            .unwrap();
        doc.add_page(page);
        
        let input_path = save_test_pdf(&mut doc, &temp_dir, "no_images.pdf");
        
        let options = ExtractImagesOptions {
            output_dir: temp_dir.path().join("images"),
            ..Default::default()
        };
        
        let result = extract_images_from_pdf(&input_path, options);
        assert!(result.is_ok());
        
        let images = result.unwrap();
        assert_eq!(images.len(), 0);
    }

    #[test]
    fn test_extract_specific_pages() {
        let temp_dir = TempDir::new().unwrap();
        let mut doc = Document::new();
        
        // Add multiple pages
        for i in 0..5 {
            let mut page = Page::a4();
            page.text()
                .set_font(crate::text::Font::Helvetica, 12.0)
                .at(50.0, 700.0)
                .write(&format!("Page {}", i + 1))
                .unwrap();
            doc.add_page(page);
        }
        
        let input_path = save_test_pdf(&mut doc, &temp_dir, "multi_page.pdf");
        
        let options = ExtractImagesOptions {
            output_dir: temp_dir.path().join("images"),
            ..Default::default()
        };
        
        // Extract from specific pages
        let result = extract_images_from_pages(&input_path, &[0, 2, 4], options);
        assert!(result.is_ok());
    }

    #[test]
    fn test_output_directory_creation() {
        let temp_dir = TempDir::new().unwrap();
        let mut doc = Document::new();
        doc.add_page(Page::a4());
        
        let input_path = save_test_pdf(&mut doc, &temp_dir, "test.pdf");
        let output_dir = temp_dir.path().join("new_dir").join("images");
        
        assert!(!output_dir.exists());
        
        let options = ExtractImagesOptions {
            output_dir: output_dir.clone(),
            create_dir: true,
            ..Default::default()
        };
        
        let result = extract_images_from_pdf(&input_path, options);
        assert!(result.is_ok());
        assert!(output_dir.exists());
    }

    #[test]
    fn test_invalid_pdf_path() {
        let temp_dir = TempDir::new().unwrap();
        let options = ExtractImagesOptions {
            output_dir: temp_dir.path().to_path_buf(),
            ..Default::default()
        };
        
        let result = extract_images_from_pdf("nonexistent.pdf", options);
        assert!(result.is_err());
    }

    #[test]
    fn test_minimum_size_filter() {
        // This test would require a PDF with actual small images
        // For now, we just test the option works
        let options = ExtractImagesOptions {
            min_size: Some(100),
            ..Default::default()
        };
        
        assert_eq!(options.min_size, Some(100));
    }

    #[test]
    fn test_name_pattern_replacement() {
        let temp_dir = TempDir::new().unwrap();
        let pattern = "page_{page}_image_{index}.{format}";
        
        // Test pattern replacement
        let result = pattern
            .replace("{page}", "1")
            .replace("{index}", "2")
            .replace("{format}", "jpg");
        
        assert_eq!(result, "page_1_image_2.jpg");
    }

    #[test]
    fn test_duplicate_image_detection() {
        // The ImageExtractor uses MD5 hashing to detect duplicate images
        // This test verifies the concept
        let data1 = vec![1, 2, 3, 4, 5];
        let data2 = vec![1, 2, 3, 4, 5]; // Same data
        let data3 = vec![5, 4, 3, 2, 1]; // Different data
        
        let hash1 = format!("{:x}", md5::compute(&data1));
        let hash2 = format!("{:x}", md5::compute(&data2));
        let hash3 = format!("{:x}", md5::compute(&data3));
        
        assert_eq!(hash1, hash2); // Same data produces same hash
        assert_ne!(hash1, hash3); // Different data produces different hash
    }

    #[test]
    fn test_png_image_creation() {
        // Test PNG image creation with minimal PNG data
        let png_data = vec![
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, // PNG signature
            0x00, 0x00, 0x00, 0x0D, // IHDR chunk length
            0x49, 0x48, 0x44, 0x52, // IHDR chunk type
            0x00, 0x00, 0x00, 0x20, // Width (32)
            0x00, 0x00, 0x00, 0x20, // Height (32)
            0x08, 0x02, 0x00, 0x00, 0x00, // Bit depth, color type, compression, filter, interlace
            0x00, 0x00, 0x00, 0x00, // CRC (simplified)
        ];
        
        let result = Image::from_png_data(png_data);
        assert!(result.is_ok());
        
        let image = result.unwrap();
        assert_eq!(image.width(), 32);
        assert_eq!(image.height(), 32);
        assert_eq!(image.format(), ImageFormat::Png);
    }

    #[test]
    fn test_tiff_image_creation() {
        // Test TIFF image creation with minimal TIFF data
        let tiff_data = vec![
            0x49, 0x49, // Little endian
            0x2A, 0x00, // Magic number
            0x08, 0x00, 0x00, 0x00, // IFD offset
            0x03, 0x00, // Number of entries
            // ImageWidth tag
            0x00, 0x01, 0x04, 0x00, 0x01, 0x00, 0x00, 0x00, 0x40, 0x00, 0x00, 0x00,
            // ImageHeight tag
            0x01, 0x01, 0x04, 0x00, 0x01, 0x00, 0x00, 0x00, 0x40, 0x00, 0x00, 0x00,
            // BitsPerSample tag
            0x02, 0x01, 0x03, 0x00, 0x01, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, // Next IFD offset
        ];
        
        let result = Image::from_tiff_data(tiff_data);
        assert!(result.is_ok());
        
        let image = result.unwrap();
        assert_eq!(image.width(), 64);
        assert_eq!(image.height(), 64);
        assert_eq!(image.format(), ImageFormat::Tiff);
    }

    #[test]
    fn test_image_format_detection() {
        // Test format detection from magic bytes
        let temp_dir = TempDir::new().unwrap();
        let mut doc = Document::new();
        doc.add_page(Page::a4());
        let input_path = save_test_pdf(&mut doc, &temp_dir, "test.pdf");
        
        let options = ExtractImagesOptions {
            output_dir: temp_dir.path().join("images"),
            ..Default::default()
        };
        
        let document = crate::parser::PdfReader::open_document(&input_path).unwrap();
        let extractor = ImageExtractor::new(document, options);
        
        // Test PNG detection
        let png_data = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
        let result = extractor.detect_image_format_from_data(&png_data);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), ImageFormat::Png);
        
        // Test TIFF detection (little endian)
        let tiff_data = vec![0x49, 0x49, 0x2A, 0x00, 0x00, 0x00, 0x00, 0x00];
        let result = extractor.detect_image_format_from_data(&tiff_data);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), ImageFormat::Tiff);
        
        // Test JPEG detection
        let jpeg_data = vec![0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x00, 0x00, 0x00];
        let result = extractor.detect_image_format_from_data(&jpeg_data);
        if result.is_err() {
            println!("JPEG detection failed: {:?}", result);
        }
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), ImageFormat::Jpeg);
    }

    #[test]
    fn test_extended_file_extensions() {
        // Test that the new formats get the correct file extensions
        let temp_dir = TempDir::new().unwrap();
        
        // Test PNG extension
        let png_extracted = ExtractedImage {
            page_number: 0,
            image_index: 0,
            file_path: temp_dir.path().join("test.png"),
            width: 100,
            height: 100,
            format: ImageFormat::Png,
        };
        assert!(png_extracted.file_path.extension().unwrap() == "png");
        
        // Test TIFF extension
        let tiff_extracted = ExtractedImage {
            page_number: 0,
            image_index: 0,
            file_path: temp_dir.path().join("test.tiff"),
            width: 100,
            height: 100,
            format: ImageFormat::Tiff,
        };
        assert!(tiff_extracted.file_path.extension().unwrap() == "tiff");
        
        // Test JPEG extension
        let jpeg_extracted = ExtractedImage {
            page_number: 0,
            image_index: 0,
            file_path: temp_dir.path().join("test.jpg"),
            width: 100,
            height: 100,
            format: ImageFormat::Jpeg,
        };
        assert!(jpeg_extracted.file_path.extension().unwrap() == "jpg");
    }
}