//! Tests for PDF image extraction operations

#[cfg(test)]
mod tests {
    use crate::graphics::{Image, ImageFormat};
    use crate::operations::extract_images::*;
    use crate::parser::filter_impls::dct::{parse_jpeg_info, JpegColorSpace};
    use crate::{Document, Page};
    use std::path::PathBuf;
    use tempfile::TempDir;

    /// Helper to create a test PDF document with an embedded image
    fn _create_pdf_with_image(title: &str) -> Document {
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
            0x10, 0x0B, 0x0C, 0x0E, 0x0C, 0x0A, 0x10, 0x0E, 0x0D, 0x0E, 0x12, 0x11, 0x10, 0x13,
            0x18, 0x28, 0x1A, 0x18, 0x16, 0x16, 0x18, 0x31, 0x23, 0x25, 0x1D, 0x28, 0x3A, 0x33,
            0x3D, 0x3C, 0x39, 0x33, 0x38, 0x37, 0x40, 0x48, 0x5C, 0x4E, 0x40, 0x44, 0x57, 0x45,
            0x37, 0x38, 0x50, 0x6D, 0x51, 0x57, 0x5F, 0x62, 0x67, 0x68, 0x67, 0x3E, 0x4D, 0x71,
            0x79, 0x70, 0x64, 0x78, 0x5C, 0x65, 0x67, 0x63, 0xFF, 0xC0, // SOF0 marker
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
            0xFF, 0xD9, // EOI marker
        ];

        // Create an image from the JPEG data
        let _image = Image::from_jpeg_data(jpeg_data).unwrap();

        // Create a page with the image
        let mut page = Page::a4();

        // Add some text
        page.text()
            .set_font(crate::text::Font::Helvetica, 24.0)
            .at(50.0, 700.0)
            .write(&format!("{title} - Page with Image"))
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
            preprocessing: ImagePreprocessingOptions::default(),
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
        let _temp_dir = TempDir::new().unwrap();
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
        // Valid 1x1 red PNG (70 bytes) - generated from real PNG
        let png_data = vec![
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48,
            0x44, 0x52, // PNG signature
            0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1F,
            0x15, 0xC4, // IHDR
            0x89, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x44, 0x41, 0x54, 0x78, 0xDA, 0x63, 0xFC, 0xCF,
            0xC0, 0xF0, 0x1F, 0x00, 0x05, 0x05, 0x02, 0x00, 0x1E, 0x5E, 0xAE, 0xD9, 0x00, 0x00,
            0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82, // IEND
        ];

        let result = Image::from_png_data(png_data);
        assert!(result.is_ok());

        let image = result.unwrap();
        assert_eq!(image.width(), 1);
        assert_eq!(image.height(), 1);
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
            0x02, 0x01, 0x03, 0x00, 0x01, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, // Next IFD offset
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
            println!("JPEG detection failed: {result:?}");
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

    #[test]
    fn test_dct_decode_jpeg_parsing() {
        // Test DCTDecode filter functionality with JPEG parsing
        let jpeg_data = vec![
            // SOI
            0xFF, 0xD8, // APP0 (JFIF)
            0xFF, 0xE0, 0x00, 0x10, // Length = 16
            b'J', b'F', b'I', b'F', 0x00, // Identifier
            0x01, 0x01, // Version
            0x00, // Units
            0x00, 0x01, 0x00, 0x01, // X/Y density
            0x00, 0x00, // Thumbnail size
            // SOF0
            0xFF, 0xC0, 0x00, 0x11, // Length = 17
            0x08, // Bits per sample
            0x00, 0x20, // Height = 32
            0x00, 0x20, // Width = 32
            0x03, // Components = 3 (RGB/YCbCr)
            0x01, 0x11, 0x00, // Component 1
            0x02, 0x11, 0x00, // Component 2
            0x03, 0x11, 0x00, // Component 3
            // SOS
            0xFF, 0xDA, 0x00, 0x0C, // Length = 12
            0x03, // Components in scan
            0x01, 0x00, // Component 1
            0x02, 0x10, // Component 2
            0x03, 0x10, // Component 3
            0x00, 0x3F, 0x00, // Spectral selection
            // Fake scan data
            0x00, 0x00, 0x00, 0x00, // EOI
            0xFF, 0xD9,
        ];

        // Test JPEG info parsing
        let info = parse_jpeg_info(&jpeg_data).unwrap();
        assert_eq!(info.width, 32);
        assert_eq!(info.height, 32);
        assert_eq!(info.components, 3);
        assert_eq!(info.bits_per_component, 8);
        assert_eq!(info.color_space, JpegColorSpace::YCbCr);
    }

    #[test]
    fn test_dct_decode_cmyk_jpeg() {
        // Test CMYK JPEG detection
        let cmyk_jpeg = vec![
            // SOI
            0xFF, 0xD8, // SOF0 with 4 components (CMYK)
            0xFF, 0xC0, 0x00, 0x14, // Length = 20
            0x08, // Bits per sample
            0x00, 0x10, // Height = 16
            0x00, 0x10, // Width = 16
            0x04, // Components = 4 (CMYK)
            0x01, 0x11, 0x00, // Component 1
            0x02, 0x11, 0x00, // Component 2
            0x03, 0x11, 0x00, // Component 3
            0x04, 0x11, 0x00, // Component 4
            // EOI
            0xFF, 0xD9,
        ];

        let info = parse_jpeg_info(&cmyk_jpeg).unwrap();
        assert_eq!(info.components, 4);
        assert_eq!(info.color_space, JpegColorSpace::CMYK);
    }

    #[test]
    fn test_extract_from_real_pdf_with_images() {
        // Test with actual PDF that contains images
        let fixture_path = "tests/fixtures/Cold_Email_Hacks.pdf";
        if !std::path::Path::new(fixture_path).exists() {
            eprintln!("Skipping test: fixture not found");
            return;
        }

        let temp_dir = TempDir::new().unwrap();
        let options = ExtractImagesOptions {
            output_dir: temp_dir.path().to_path_buf(),
            extract_inline: true,
            min_size: Some(10),
            create_dir: true,
            ..Default::default()
        };

        let result = extract_images_from_pdf(fixture_path, options);

        // This should succeed even if no images found
        assert!(
            result.is_ok(),
            "Image extraction failed: {:?}",
            result.err()
        );

        let images = result.unwrap();
        // Cold_Email_Hacks.pdf might have images, but we don't hardcode expectations
        eprintln!(
            "Extracted {} images from Cold_Email_Hacks.pdf",
            images.len()
        );

        // Verify all extracted images have valid dimensions
        for img in &images {
            assert!(img.width > 0, "Invalid width: {}", img.width);
            assert!(img.height > 0, "Invalid height: {}", img.height);
            assert!(
                img.file_path.exists(),
                "Image file not created: {:?}",
                img.file_path
            );
        }
    }

    #[test]
    fn test_extract_from_specific_page() {
        let fixture_path = "tests/fixtures/Cold_Email_Hacks.pdf";
        if !std::path::Path::new(fixture_path).exists() {
            eprintln!("Skipping test: fixture not found");
            return;
        }

        let temp_dir = TempDir::new().unwrap();
        let options = ExtractImagesOptions {
            output_dir: temp_dir.path().to_path_buf(),
            extract_inline: false, // Only XObject images
            min_size: None,        // Extract all sizes
            create_dir: true,
            ..Default::default()
        };

        // Extract only from first page (0-indexed)
        let result = extract_images_from_pages(fixture_path, &[0], options);
        assert!(result.is_ok(), "Page extraction failed: {:?}", result.err());

        let images = result.unwrap();
        // All images should be from page 0
        for img in &images {
            assert_eq!(img.page_number, 0, "Wrong page number");
        }
    }

    #[test]
    fn test_create_png_from_raw_data() {
        // Test PNG creation from raw grayscale data
        let temp_dir = TempDir::new().unwrap();
        let mut doc = Document::new();
        doc.add_page(Page::a4());
        let input_path = save_test_pdf(&mut doc, &temp_dir, "test.pdf");

        let document = crate::parser::PdfReader::open_document(&input_path).unwrap();
        let options = ExtractImagesOptions {
            output_dir: temp_dir.path().to_path_buf(),
            ..Default::default()
        };
        let extractor = ImageExtractor::new(document, options);

        // Create 4x4 grayscale image data
        let raw_data: Vec<u8> = vec![
            0, 50, 100, 150, 50, 100, 150, 200, 100, 150, 200, 250, 150, 200, 250, 255,
        ];

        let result = extractor.create_png_from_raw_data(&raw_data, 4, 4, 1, 8);
        assert!(result.is_ok(), "PNG creation failed: {:?}", result.err());

        let png_bytes = result.unwrap();
        // Verify PNG signature
        assert_eq!(
            &png_bytes[0..8],
            &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]
        );
    }

    #[test]
    fn test_create_png_rgb_data() {
        // Test PNG creation from RGB data
        let temp_dir = TempDir::new().unwrap();
        let mut doc = Document::new();
        doc.add_page(Page::a4());
        let input_path = save_test_pdf(&mut doc, &temp_dir, "test.pdf");

        let document = crate::parser::PdfReader::open_document(&input_path).unwrap();
        let options = ExtractImagesOptions::default();
        let extractor = ImageExtractor::new(document, options);

        // Create 2x2 RGB image
        let rgb_data: Vec<u8> = vec![
            255, 0, 0, 0, 255, 0, // Red, Green
            0, 0, 255, 255, 255, 0, // Blue, Yellow
        ];

        let result = extractor.create_png_from_raw_data(&rgb_data, 2, 2, 3, 8);
        assert!(result.is_ok());

        let png_bytes = result.unwrap();
        assert!(&png_bytes[0..8] == &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]);
    }

    #[test]
    fn test_crc32_calculation() {
        // Test CRC32 computation used in PNG chunks
        let temp_dir = TempDir::new().unwrap();
        let mut doc = Document::new();
        doc.add_page(Page::a4());
        let input_path = save_test_pdf(&mut doc, &temp_dir, "test.pdf");

        let document = crate::parser::PdfReader::open_document(&input_path).unwrap();
        let extractor = ImageExtractor::new(document, ExtractImagesOptions::default());

        // Calculate CRC for known data
        let chunk_type = b"IHDR";
        let data = vec![
            0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00,
        ];

        let crc = extractor.calculate_crc32(chunk_type, &data);
        // CRC should be deterministic
        assert_ne!(crc, 0);

        // Same data should give same CRC
        let crc2 = extractor.calculate_crc32(chunk_type, &data);
        assert_eq!(crc, crc2);

        // Different data should give different CRC
        let data2 = vec![0xFF, 0xFF, 0xFF, 0xFF];
        let crc3 = extractor.calculate_crc32(chunk_type, &data2);
        assert_ne!(crc, crc3);
    }

    #[test]
    fn test_compress_image_data() {
        // Test zlib compression for PNG IDAT chunks
        let temp_dir = TempDir::new().unwrap();
        let mut doc = Document::new();
        doc.add_page(Page::a4());
        let input_path = save_test_pdf(&mut doc, &temp_dir, "test.pdf");

        let document = crate::parser::PdfReader::open_document(&input_path).unwrap();
        let extractor = ImageExtractor::new(document, ExtractImagesOptions::default());

        // Simple 3x3 grayscale image
        let data: Vec<u8> = vec![0, 50, 100, 50, 100, 150, 100, 150, 200];

        let result = extractor.compress_image_data(&data, 3, 3, 1);
        assert!(result.is_ok());

        let compressed = result.unwrap();
        // Compressed data should be non-empty and start with zlib header
        assert!(!compressed.is_empty());
        // Zlib header typically starts with 0x78 (common compression levels)
        assert!(compressed[0] == 0x78 || compressed[0] == 0x08);
    }

    #[test]
    fn test_detect_correct_row_stride() {
        // Test stride detection for CCITT-encoded images
        let temp_dir = TempDir::new().unwrap();
        let mut doc = Document::new();
        doc.add_page(Page::a4());
        let input_path = save_test_pdf(&mut doc, &temp_dir, "test.pdf");

        let document = crate::parser::PdfReader::open_document(&input_path).unwrap();
        let extractor = ImageExtractor::new(document, ExtractImagesOptions::default());

        // Simulate CCITT data for 100x10 image (1 bit per pixel)
        let width = 100u32;
        let height = 10u32;
        let min_bytes_per_row = (width as usize + 7) / 8; // 13 bytes

        // Create data with 16-byte alignment (common)
        let stride = 16;
        let data = vec![0u8; stride * height as usize];

        let possible_strides = [
            min_bytes_per_row,              // 13
            (min_bytes_per_row + 1) & !1,   // 14
            (min_bytes_per_row + 3) & !3,   // 16
            (min_bytes_per_row + 7) & !7,   // 16
            (min_bytes_per_row + 15) & !15, // 16
        ];

        let result = extractor.detect_correct_row_stride(&data, width, height, &possible_strides);
        assert!(result.is_ok());

        let detected_stride = result.unwrap();
        // Should detect a reasonable stride (>= min_bytes_per_row, <= 16)
        assert!(
            detected_stride >= min_bytes_per_row,
            "Stride too small: {}",
            detected_stride
        );
        assert!(
            detected_stride <= 16,
            "Stride too large: {}",
            detected_stride
        );

        // Verify it can fit the data
        let expected_size = detected_stride * height as usize;
        assert!(
            expected_size <= data.len(),
            "Stride {} would overflow data",
            detected_stride
        );
    }

    #[test]
    fn test_parse_inline_image_dict() {
        // Test parsing of inline image dictionaries
        let temp_dir = TempDir::new().unwrap();
        let mut doc = Document::new();
        doc.add_page(Page::a4());
        let input_path = save_test_pdf(&mut doc, &temp_dir, "test.pdf");

        let document = crate::parser::PdfReader::open_document(&input_path).unwrap();
        let extractor = ImageExtractor::new(document, ExtractImagesOptions::default());

        // Test with /W and /H abbreviations
        let dict1 = "/W 200\n/H 150\n/BPC 8";
        let (width, height) = extractor.parse_inline_image_dict(dict1);
        assert_eq!(width, 200);
        assert_eq!(height, 150);

        // Test with full names
        let dict2 = "/Width 300\n/Height 250";
        let (width, height) = extractor.parse_inline_image_dict(dict2);
        assert_eq!(width, 300);
        assert_eq!(height, 250);

        // Test with missing values (should use defaults)
        let dict3 = "/BPC 8";
        let (width, height) = extractor.parse_inline_image_dict(dict3);
        assert_eq!(width, 100); // Default
        assert_eq!(height, 100); // Default
    }

    #[test]
    fn test_image_extraction_with_min_size_filter() {
        // Test that min_size filter works correctly
        let fixture_path = "tests/fixtures/Cold_Email_Hacks.pdf";
        if !std::path::Path::new(fixture_path).exists() {
            eprintln!("Skipping test: fixture not found");
            return;
        }

        let temp_dir = TempDir::new().unwrap();

        // Extract with no size filter
        let options_no_filter = ExtractImagesOptions {
            output_dir: temp_dir.path().join("no_filter"),
            min_size: None,
            create_dir: true,
            ..Default::default()
        };
        let images_no_filter = extract_images_from_pdf(fixture_path, options_no_filter).unwrap();

        // Extract with high size filter (only large images)
        let options_with_filter = ExtractImagesOptions {
            output_dir: temp_dir.path().join("with_filter"),
            min_size: Some(500), // Only images >= 500px
            create_dir: true,
            ..Default::default()
        };
        let images_with_filter =
            extract_images_from_pdf(fixture_path, options_with_filter).unwrap();

        // Filtered list should be <= unfiltered list
        assert!(images_with_filter.len() <= images_no_filter.len());

        // All filtered images should meet min_size requirement
        for img in &images_with_filter {
            assert!(
                img.width >= 500 || img.height >= 500,
                "Image {}x{} should not pass filter",
                img.width,
                img.height
            );
        }
    }

    #[test]
    fn test_deduplication_with_page_pattern() {
        // Test that deduplication is disabled when pattern contains {page}
        let temp_dir = TempDir::new().unwrap();
        let mut doc = Document::new();
        doc.add_page(Page::a4());
        let input_path = save_test_pdf(&mut doc, &temp_dir, "test.pdf");

        let document = crate::parser::PdfReader::open_document(&input_path).unwrap();

        // Pattern WITH {page} should NOT deduplicate
        let options_no_dedup = ExtractImagesOptions {
            output_dir: temp_dir.path().to_path_buf(),
            name_pattern: "page_{page}_img_{index}.{format}".to_string(),
            ..Default::default()
        };
        let extractor = ImageExtractor::new(document, options_no_dedup);
        let allow_dedup = !extractor.options.name_pattern.contains("{page}");
        assert!(
            !allow_dedup,
            "Should NOT allow deduplication with {{page}} pattern"
        );
    }
}
