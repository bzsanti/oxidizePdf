//! Tests for OCR integration with page analysis

#[cfg(test)]
mod tests {
    use crate::operations::page_analysis::*;
    use crate::text::{MockOcrProvider, OcrOptions, OcrProvider};
    use crate::{Document, Page};
    use std::path::PathBuf;
    use tempfile::TempDir;

    /// Helper to create a mock scanned PDF document
    fn _create_mock_scanned_pdf(title: &str) -> Document {
        let mut doc = Document::new();
        doc.set_title(title);

        // Create a page with minimal text content to simulate a scanned page
        let mut page = Page::a4();
        page.text()
            .set_font(crate::text::Font::Helvetica, 8.0)
            .at(50.0, 50.0)
            .write("Scanned") // Very minimal text
            .unwrap();

        doc.add_page(page);
        doc
    }

    /// Helper to save a document to a temp file and return the path
    fn _save_test_pdf(doc: &mut Document, dir: &TempDir, name: &str) -> PathBuf {
        let path = dir.path().join(name);
        doc.save(&path).unwrap();
        path
    }

    #[test]
    fn test_analysis_options_with_ocr() {
        let ocr_options = OcrOptions {
            language: "es".to_string(),
            min_confidence: 0.8,
            ..Default::default()
        };

        let analysis_options = AnalysisOptions {
            min_text_fragment_size: 5,
            min_image_size: 100,
            scanned_threshold: 0.85,
            text_threshold: 0.75,
            ocr_options: Some(ocr_options),
        };

        assert_eq!(analysis_options.min_text_fragment_size, 5);
        assert_eq!(analysis_options.min_image_size, 100);
        assert_eq!(analysis_options.scanned_threshold, 0.85);
        assert_eq!(analysis_options.text_threshold, 0.75);
        assert!(analysis_options.ocr_options.is_some());

        let ocr_opts = analysis_options.ocr_options.unwrap();
        assert_eq!(ocr_opts.language, "es");
        assert_eq!(ocr_opts.min_confidence, 0.8);
    }

    #[test]
    fn test_mock_ocr_provider_integration() {
        let provider = MockOcrProvider::new();
        let options = OcrOptions::default();

        // Test with mock JPEG data
        let mock_jpeg = vec![0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46, 0x49, 0x46];

        let result = provider.process_image(&mock_jpeg, &options);
        assert!(result.is_ok());

        let ocr_result = result.unwrap();
        assert!(ocr_result.text.contains("Mock OCR"));
        assert!(ocr_result.confidence > 0.0);
        assert!(!ocr_result.fragments.is_empty());
        assert_eq!(ocr_result.engine_name, "Mock OCR");
    }

    #[test]
    fn test_mock_ocr_provider_custom_text() {
        let mut provider = MockOcrProvider::new();
        provider.set_mock_text("Custom extracted text from scanned page".to_string());
        provider.set_confidence(0.95);

        let options = OcrOptions::default();
        let mock_jpeg = vec![0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46, 0x49, 0x46];

        let result = provider.process_image(&mock_jpeg, &options).unwrap();
        assert!(result.text.contains("Custom extracted text"));
        assert_eq!(result.confidence, 0.95);
    }

    #[test]
    fn test_mock_ocr_provider_supported_formats() {
        let provider = MockOcrProvider::new();

        assert!(provider.supports_format(crate::graphics::ImageFormat::Jpeg));
        assert!(provider.supports_format(crate::graphics::ImageFormat::Png));
        assert!(provider.supports_format(crate::graphics::ImageFormat::Tiff));

        let formats = provider.supported_formats();
        assert_eq!(formats.len(), 3);
    }

    #[test]
    fn test_mock_ocr_provider_validate_image_data() {
        let provider = MockOcrProvider::new();

        // Valid JPEG
        let jpeg_data = vec![0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46, 0x49, 0x46];
        assert!(provider.validate_image_data(&jpeg_data).is_ok());

        // Valid PNG
        let png_data = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
        assert!(provider.validate_image_data(&png_data).is_ok());

        // Invalid data
        let invalid_data = vec![0x00, 0x01, 0x02, 0x03];
        assert!(provider.validate_image_data(&invalid_data).is_err());
    }

    #[test]
    fn test_mock_ocr_provider_process_page() {
        let provider = MockOcrProvider::new();
        let options = OcrOptions::default();

        // Create a mock content analysis
        let analysis = ContentAnalysis {
            page_number: 0,
            page_type: PageType::Scanned,
            text_ratio: 0.05,
            image_ratio: 0.90,
            blank_space_ratio: 0.05,
            text_fragment_count: 2,
            image_count: 1,
            character_count: 15,
        };

        let mock_page_data = vec![0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46, 0x49, 0x46];

        let result = provider.process_page(&analysis, &mock_page_data, &options);
        assert!(result.is_ok());

        let ocr_result = result.unwrap();
        assert!(ocr_result.text.contains("Mock OCR"));
        assert_eq!(ocr_result.language, "en");
    }

    #[test]
    fn test_ocr_processing_result_methods() {
        let provider = MockOcrProvider::new();
        let options = OcrOptions::default();
        let mock_jpeg = vec![0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46, 0x49, 0x46];

        let result = provider.process_image(&mock_jpeg, &options).unwrap();

        // Test filter by confidence
        let high_confidence = result.filter_by_confidence(0.8);
        assert!(!high_confidence.is_empty());

        // Test fragments in region
        let in_region = result.fragments_in_region(0.0, 0.0, 1000.0, 1000.0);
        assert!(!in_region.is_empty());

        // Test fragments of type
        let lines = result.fragments_of_type(crate::text::FragmentType::Line);
        assert!(!lines.is_empty());

        // Test average confidence
        let avg_confidence = result.average_confidence();
        assert!(avg_confidence > 0.0);
    }

    #[test]
    fn test_ocr_error_types() {
        use crate::text::OcrError;

        let provider_error = OcrError::ProviderNotAvailable("Test provider".to_string());
        assert!(provider_error.to_string().contains("Test provider"));

        let format_error = OcrError::UnsupportedImageFormat(crate::graphics::ImageFormat::Jpeg);
        assert!(format_error.to_string().contains("Jpeg"));

        let invalid_data_error = OcrError::InvalidImageData("Corrupted".to_string());
        assert!(invalid_data_error.to_string().contains("Corrupted"));

        let processing_error = OcrError::ProcessingFailed("OCR failed".to_string());
        assert!(processing_error.to_string().contains("OCR failed"));
    }

    #[test]
    fn test_fragment_types() {
        use crate::text::FragmentType;

        assert_eq!(FragmentType::Character, FragmentType::Character);
        assert_ne!(FragmentType::Character, FragmentType::Word);
        assert_ne!(FragmentType::Word, FragmentType::Line);
        assert_ne!(FragmentType::Line, FragmentType::Paragraph);
    }

    #[test]
    fn test_ocr_engines() {
        use crate::text::OcrEngine;

        assert_eq!(OcrEngine::Mock.name(), "Mock OCR");
        assert_eq!(OcrEngine::Tesseract.name(), "Tesseract");
        assert_eq!(OcrEngine::Azure.name(), "Azure Computer Vision");

        // Test format support
        assert!(OcrEngine::Mock.supports_format(crate::graphics::ImageFormat::Jpeg));
        assert!(OcrEngine::Tesseract.supports_format(crate::graphics::ImageFormat::Tiff));
        assert!(!OcrEngine::Azure.supports_format(crate::graphics::ImageFormat::Tiff));
    }

    #[test]
    fn test_image_preprocessing_options() {
        use crate::text::ImagePreprocessing;

        let default_preprocessing = ImagePreprocessing::default();
        assert!(default_preprocessing.denoise);
        assert!(default_preprocessing.deskew);
        assert!(default_preprocessing.enhance_contrast);
        assert!(!default_preprocessing.sharpen);
        assert_eq!(default_preprocessing.scale_factor, 1.0);

        let custom_preprocessing = ImagePreprocessing {
            denoise: false,
            deskew: false,
            enhance_contrast: false,
            sharpen: true,
            scale_factor: 2.0,
        };

        assert!(!custom_preprocessing.denoise);
        assert!(!custom_preprocessing.deskew);
        assert!(!custom_preprocessing.enhance_contrast);
        assert!(custom_preprocessing.sharpen);
        assert_eq!(custom_preprocessing.scale_factor, 2.0);
    }

    #[test]
    fn test_ocr_options_customization() {
        let options = OcrOptions {
            language: "fr".to_string(),
            min_confidence: 0.75,
            preserve_layout: false,
            timeout_seconds: 60,
            ..Default::default()
        };

        assert_eq!(options.language, "fr");
        assert_eq!(options.min_confidence, 0.75);
        assert!(!options.preserve_layout);
        assert_eq!(options.timeout_seconds, 60);
    }

    #[test]
    fn test_ocr_options_default_values() {
        let options = OcrOptions::default();

        assert_eq!(options.language, "en");
        assert_eq!(options.min_confidence, 0.6);
        assert!(options.preserve_layout);
        assert_eq!(options.timeout_seconds, 60); // Updated to match actual default
    }

    #[test]
    fn test_ocr_text_fragment_creation() {
        use crate::text::{FragmentType, OcrTextFragment};

        let fragment = OcrTextFragment {
            text: "Sample text".to_string(),
            x: 100.0,
            y: 200.0,
            width: 150.0,
            height: 20.0,
            confidence: 0.95,
            word_confidences: None,
            font_size: 12.0,
            fragment_type: FragmentType::Word,
        };

        assert_eq!(fragment.text, "Sample text");
        assert_eq!(fragment.x, 100.0);
        assert_eq!(fragment.y, 200.0);
        assert_eq!(fragment.width, 150.0);
        assert_eq!(fragment.height, 20.0);
        assert_eq!(fragment.confidence, 0.95);
        assert_eq!(fragment.font_size, 12.0);
        assert_eq!(fragment.fragment_type, FragmentType::Word);
    }

    #[test]
    fn test_mock_ocr_provider_processing_delay() {
        let mut provider = MockOcrProvider::new();
        provider.set_processing_delay(50); // 50ms delay

        let options = OcrOptions::default();
        let mock_jpeg = vec![0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46, 0x49, 0x46];

        let start_time = std::time::Instant::now();
        let result = provider.process_image(&mock_jpeg, &options);
        let elapsed = start_time.elapsed();

        assert!(result.is_ok());
        assert!(elapsed.as_millis() >= 50); // Should take at least 50ms

        let ocr_result = result.unwrap();
        assert_eq!(ocr_result.processing_time_ms, 50);
    }

    #[test]
    fn test_engine_options_hashmap() {
        let mut engine_options = std::collections::HashMap::new();
        engine_options.insert("tesseract_psm".to_string(), "6".to_string());
        engine_options.insert("tesseract_oem".to_string(), "3".to_string());

        let options = OcrOptions {
            engine_options,
            ..Default::default()
        };

        assert_eq!(
            options.engine_options.get("tesseract_psm"),
            Some(&"6".to_string())
        );
        assert_eq!(
            options.engine_options.get("tesseract_oem"),
            Some(&"3".to_string())
        );
    }
}
