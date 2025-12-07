//! Tests for PDF split operations

#[cfg(test)]
mod tests {
    use crate::operations::split::*;
    use crate::operations::PageRange;
    use crate::{Document, Page};
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;

    /// Helper to create a test PDF document with specified number of pages
    fn create_test_pdf(num_pages: usize, title: &str) -> Document {
        let mut doc = Document::new();
        doc.set_title(title);
        doc.set_author("Test Author");
        doc.set_subject("Test Subject");
        doc.set_keywords("test, pdf, split");

        for i in 0..num_pages {
            let mut page = Page::a4();

            // Add some content to each page
            page.text()
                .set_font(crate::text::Font::Helvetica, 24.0)
                .at(50.0, 700.0)
                .write(&format!("{} - Page {}", title, i + 1))
                .unwrap();

            // Add some graphics to make pages different
            page.graphics()
                .rect(100.0, 500.0, 200.0, 100.0)
                .set_fill_color(crate::graphics::Color::Rgb(0.8, 0.8, 0.8))
                .fill();

            doc.add_page(page);
        }

        doc
    }

    /// Helper to save a document to a temp file and return the path
    fn save_test_pdf(doc: &mut Document, dir: &TempDir, name: &str) -> PathBuf {
        let path = dir.path().join(name);
        doc.save(&path).unwrap();
        path
    }

    #[test]
    fn test_split_mode_variants() {
        // Test all SplitMode variants
        let single = SplitMode::SinglePages;
        let ranges = SplitMode::Ranges(vec![PageRange::Range(0, 2), PageRange::Single(5)]);
        let chunk = SplitMode::ChunkSize(3);
        let split_at = SplitMode::SplitAt(vec![3, 6, 9]);

        match single {
            SplitMode::SinglePages => assert!(true),
            _ => panic!("Wrong variant"),
        }

        match ranges {
            SplitMode::Ranges(ref r) => assert_eq!(r.len(), 2),
            _ => panic!("Wrong variant"),
        }

        match chunk {
            SplitMode::ChunkSize(size) => assert_eq!(size, 3),
            _ => panic!("Wrong variant"),
        }

        match split_at {
            SplitMode::SplitAt(ref points) => assert_eq!(points.len(), 3),
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn test_split_options_custom() {
        let options = SplitOptions {
            mode: SplitMode::ChunkSize(5),
            output_pattern: "chunk_{n}.pdf".to_string(),
            preserve_metadata: false,
            optimize: true,
        };

        assert!(matches!(options.mode, SplitMode::ChunkSize(5)));
        assert_eq!(options.output_pattern, "chunk_{n}.pdf");
        assert!(!options.preserve_metadata);
        assert!(options.optimize);
    }

    #[test]
    fn test_split_into_single_pages() {
        let temp_dir = TempDir::new().unwrap();
        let mut doc = create_test_pdf(5, "Test Document");
        let input_path = save_test_pdf(&mut doc, &temp_dir, "input.pdf");

        let options = SplitOptions {
            mode: SplitMode::SinglePages,
            output_pattern: temp_dir
                .path()
                .join("page_{}.pdf")
                .to_str()
                .unwrap()
                .to_string(),
            preserve_metadata: true,
            optimize: false,
        };

        let result = split_pdf(&input_path, options);
        assert!(result.is_ok());

        let output_files = result.unwrap();
        assert_eq!(output_files.len(), 5);

        // Check that all output files exist
        for (i, output_path) in output_files.iter().enumerate() {
            assert!(output_path.exists(), "Output file {} should exist", i + 1);
            assert!(fs::metadata(output_path).unwrap().len() > 0);
        }
    }

    #[test]
    fn test_split_by_chunks() {
        let temp_dir = TempDir::new().unwrap();
        let mut doc = create_test_pdf(10, "Chunked Document");
        let input_path = save_test_pdf(&mut doc, &temp_dir, "input.pdf");

        let options = SplitOptions {
            mode: SplitMode::ChunkSize(3),
            output_pattern: temp_dir
                .path()
                .join("chunk_{n}.pdf")
                .to_str()
                .unwrap()
                .to_string(),
            preserve_metadata: true,
            optimize: false,
        };

        let result = split_pdf(&input_path, options);
        assert!(result.is_ok());

        let output_files = result.unwrap();
        // 10 pages split into chunks of 3: [0-2], [3-5], [6-8], [9]
        assert_eq!(output_files.len(), 4);

        for output_path in output_files {
            assert!(output_path.exists());
        }
    }

    #[test]
    fn test_split_by_ranges() {
        let temp_dir = TempDir::new().unwrap();
        let mut doc = create_test_pdf(8, "Range Split Document");
        let input_path = save_test_pdf(&mut doc, &temp_dir, "input.pdf");

        let ranges = vec![
            PageRange::Single(0),        // Page 1
            PageRange::Range(1, 3),      // Pages 2-4
            PageRange::List(vec![4, 6]), // Pages 5 and 7
        ];

        let options = SplitOptions {
            mode: SplitMode::Ranges(ranges),
            output_pattern: temp_dir
                .path()
                .join("range_{n}.pdf")
                .to_str()
                .unwrap()
                .to_string(),
            preserve_metadata: false,
            optimize: false,
        };

        let result = split_pdf(&input_path, options);
        assert!(result.is_ok());

        let output_files = result.unwrap();
        assert_eq!(output_files.len(), 3);
    }

    #[test]
    fn test_split_at_points() {
        let temp_dir = TempDir::new().unwrap();
        let mut doc = create_test_pdf(12, "Split At Document");
        let input_path = save_test_pdf(&mut doc, &temp_dir, "input.pdf");

        // Split at pages 3, 6, 9 creates: [0-2], [3-5], [6-8], [9-11]
        let split_points = vec![3, 6, 9];

        let options = SplitOptions {
            mode: SplitMode::SplitAt(split_points),
            output_pattern: temp_dir
                .path()
                .join("part_{n}.pdf")
                .to_str()
                .unwrap()
                .to_string(),
            preserve_metadata: true,
            optimize: false,
        };

        let result = split_pdf(&input_path, options);
        assert!(result.is_ok());

        let output_files = result.unwrap();
        assert_eq!(output_files.len(), 4);
    }

    #[test]
    fn test_split_into_pages_function() {
        let temp_dir = TempDir::new().unwrap();
        let mut doc = create_test_pdf(3, "Simple Split");
        let input_path = save_test_pdf(&mut doc, &temp_dir, "input.pdf");

        let output_pattern = temp_dir
            .path()
            .join("output_{}.pdf")
            .to_str()
            .unwrap()
            .to_string();
        let result = split_into_pages(&input_path, &output_pattern);

        assert!(result.is_ok());
        let output_files = result.unwrap();
        assert_eq!(output_files.len(), 3);
    }

    #[test]
    fn test_split_empty_pdf() {
        let temp_dir = TempDir::new().unwrap();
        let mut doc = Document::new(); // Empty document
        let input_path = save_test_pdf(&mut doc, &temp_dir, "empty.pdf");

        let result = split_into_pages(&input_path, "page_{}.pdf");

        // Should fail with NoPagesToProcess
        assert!(result.is_err());
        match result {
            Err(crate::operations::OperationError::NoPagesToProcess) => assert!(true),
            _ => panic!("Expected NoPagesToProcess error"),
        }
    }

    #[test]
    fn test_split_nonexistent_file() {
        let options = SplitOptions::default();
        let result = split_pdf("nonexistent.pdf", options);

        assert!(result.is_err());
        match result {
            Err(crate::operations::OperationError::ParseError(_)) => assert!(true),
            _ => panic!("Expected ParseError"),
        }
    }

    #[test]
    fn test_format_output_path_patterns() {
        let temp_dir = TempDir::new().unwrap();
        let mut doc = create_test_pdf(5, "Pattern Test");
        let input_path = save_test_pdf(&mut doc, &temp_dir, "input.pdf");

        // Test various output patterns
        let patterns = vec![
            ("page_{}.pdf", "Single page pattern"),
            ("doc_{n}_page_{page}.pdf", "Pattern with index and page"),
            ("split_{start}-{end}.pdf", "Pattern with range"),
        ];

        for (pattern, description) in patterns {
            let options = SplitOptions {
                mode: SplitMode::SinglePages,
                output_pattern: temp_dir.path().join(pattern).to_str().unwrap().to_string(),
                preserve_metadata: true,
                optimize: false,
            };

            let result = split_pdf(&input_path, options);
            assert!(result.is_ok(), "Pattern '{pattern}' failed: {description}");
        }
    }

    #[test]
    fn test_split_preserve_metadata() {
        let temp_dir = TempDir::new().unwrap();
        let mut doc = create_test_pdf(2, "Metadata Test");
        doc.set_title("Original Title");
        doc.set_author("Original Author");
        doc.set_subject("Original Subject");
        doc.set_keywords("original, keywords");

        let input_path = save_test_pdf(&mut doc, &temp_dir, "input.pdf");

        // Split with metadata preservation
        let options = SplitOptions {
            mode: SplitMode::SinglePages,
            output_pattern: temp_dir
                .path()
                .join("preserved_{}.pdf")
                .to_str()
                .unwrap()
                .to_string(),
            preserve_metadata: true,
            optimize: false,
        };

        let result = split_pdf(&input_path, options);
        assert!(result.is_ok());

        // Note: We'd need metadata getters to verify preservation
    }

    #[test]
    fn test_split_without_metadata() {
        let temp_dir = TempDir::new().unwrap();
        let mut doc = create_test_pdf(2, "No Metadata Test");
        doc.set_title("Should Not Be Preserved");

        let input_path = save_test_pdf(&mut doc, &temp_dir, "input.pdf");

        // Split without metadata preservation
        let options = SplitOptions {
            mode: SplitMode::SinglePages,
            output_pattern: temp_dir
                .path()
                .join("no_meta_{}.pdf")
                .to_str()
                .unwrap()
                .to_string(),
            preserve_metadata: false,
            optimize: false,
        };

        let result = split_pdf(&input_path, options);
        assert!(result.is_ok());
    }

    #[test]
    fn test_split_edge_cases() {
        let temp_dir = TempDir::new().unwrap();

        // Test with 1-page document
        let mut doc1 = create_test_pdf(1, "Single Page");
        let path1 = save_test_pdf(&mut doc1, &temp_dir, "single.pdf");

        let result1 = split_into_pages(
            &path1,
            temp_dir.path().join("single_{}.pdf").to_str().unwrap(),
        );
        assert!(result1.is_ok());
        assert_eq!(result1.unwrap().len(), 1);

        // Test chunk size larger than document
        let mut doc2 = create_test_pdf(3, "Small Document");
        let path2 = save_test_pdf(&mut doc2, &temp_dir, "small.pdf");

        let options2 = SplitOptions {
            mode: SplitMode::ChunkSize(10), // Larger than document
            output_pattern: temp_dir
                .path()
                .join("large_chunk_{}.pdf")
                .to_str()
                .unwrap()
                .to_string(),
            ..Default::default()
        };

        let result2 = split_pdf(&path2, options2);
        assert!(result2.is_ok());
        assert_eq!(result2.unwrap().len(), 1); // Should create single output
    }

    #[test]
    fn test_split_at_invalid_points() {
        let temp_dir = TempDir::new().unwrap();
        let mut doc = create_test_pdf(5, "Invalid Split Points");
        let input_path = save_test_pdf(&mut doc, &temp_dir, "input.pdf");

        // Test with invalid split points (0 and beyond document)
        let split_points = vec![0, 10, 15]; // 0 and points beyond page 5

        let options = SplitOptions {
            mode: SplitMode::SplitAt(split_points),
            output_pattern: temp_dir
                .path()
                .join("invalid_{}.pdf")
                .to_str()
                .unwrap()
                .to_string(),
            ..Default::default()
        };

        let result = split_pdf(&input_path, options);
        assert!(result.is_ok());

        // Should handle gracefully and create valid splits
        let output_files = result.unwrap();
        assert!(!output_files.is_empty());
    }

    #[test]
    fn test_split_large_document() {
        let temp_dir = TempDir::new().unwrap();
        let mut doc = create_test_pdf(15, "Large Document");
        let input_path = save_test_pdf(&mut doc, &temp_dir, "large.pdf");

        // Split into chunks of 4
        let options = SplitOptions {
            mode: SplitMode::ChunkSize(4),
            output_pattern: temp_dir
                .path()
                .join("chunk_{n}.pdf")
                .to_str()
                .unwrap()
                .to_string(),
            preserve_metadata: true,
            optimize: false,
        };

        let result = split_pdf(&input_path, options);
        assert!(result.is_ok(), "Split operation failed: {result:?}");

        let output_files = result.unwrap();
        // 15 pages / 4 per chunk = 4 files (last one has 3 pages)
        assert_eq!(output_files.len(), 4);
    }

    #[test]
    fn test_split_options_default() {
        let options = SplitOptions::default();
        assert!(matches!(options.mode, SplitMode::SinglePages));
        assert_eq!(options.output_pattern, "page_{}.pdf");
        assert!(options.preserve_metadata);
        assert!(!options.optimize);
    }

    #[test]
    fn test_split_options_custom_extended() {
        let options = SplitOptions {
            mode: SplitMode::ChunkSize(5),
            output_pattern: "custom_pattern_{}.pdf".to_string(),
            preserve_metadata: false,
            optimize: true,
        };

        assert!(matches!(options.mode, SplitMode::ChunkSize(5)));
        assert_eq!(options.output_pattern, "custom_pattern_{}.pdf");
        assert!(!options.preserve_metadata);
        assert!(options.optimize);
    }

    #[test]
    fn test_all_split_mode_variants() {
        // Test all SplitMode variants
        let modes = vec![
            SplitMode::SinglePages,
            SplitMode::ChunkSize(3),
            SplitMode::SplitAt(vec![2, 4, 6]),
            SplitMode::Ranges(vec![PageRange::Single(0), PageRange::Range(1, 3)]),
        ];

        for mode in modes {
            let options = SplitOptions {
                mode,
                ..Default::default()
            };
            // Just verify we can create options with all split modes
            assert!(options.preserve_metadata);
        }
    }

    #[test]
    fn test_pdf_splitter_new() {
        let temp_dir = TempDir::new().unwrap();
        let mut doc = create_test_pdf(3, "Test");
        let input_path = save_test_pdf(&mut doc, &temp_dir, "input.pdf");

        let document = crate::parser::PdfReader::open_document(&input_path).unwrap();
        let options = SplitOptions::default();
        let _splitter = PdfSplitter::new(document, options);
        // Just verify we can create a splitter
        assert!(true);
    }

    #[test]
    fn test_pdf_splitter_split_method() {
        let temp_dir = TempDir::new().unwrap();
        let mut doc = create_test_pdf(3, "Test");
        let input_path = save_test_pdf(&mut doc, &temp_dir, "input.pdf");

        let document = crate::parser::PdfReader::open_document(&input_path).unwrap();
        let options = SplitOptions {
            mode: SplitMode::SinglePages,
            output_pattern: temp_dir
                .path()
                .join("splitter_test_{}.pdf")
                .to_str()
                .unwrap()
                .to_string(),
            ..Default::default()
        };

        let mut splitter = PdfSplitter::new(document, options);
        let result = splitter.split();
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 3);
    }

    #[test]
    fn test_split_with_ranges_mode() {
        let temp_dir = TempDir::new().unwrap();
        let mut doc = create_test_pdf(10, "Range Test");
        let input_path = save_test_pdf(&mut doc, &temp_dir, "input.pdf");

        let ranges = vec![
            PageRange::Single(0),   // First page
            PageRange::Range(1, 3), // Pages 2-4
            PageRange::Range(5, 7), // Pages 6-8
            PageRange::Single(9),   // Last page
        ];

        let options = SplitOptions {
            mode: SplitMode::Ranges(ranges),
            output_pattern: temp_dir
                .path()
                .join("range_{}.pdf")
                .to_str()
                .unwrap()
                .to_string(),
            preserve_metadata: true,
            optimize: false,
        };

        let result = split_pdf(&input_path, options);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 4);
    }

    #[test]
    fn test_split_debug_implementations() {
        // Test Debug implementations
        let options = SplitOptions::default();
        let debug_str = format!("{options:?}");
        assert!(debug_str.contains("SplitOptions"));

        let mode = SplitMode::SinglePages;
        let debug_str = format!("{mode:?}");
        assert!(debug_str.contains("SinglePages"));

        let mode = SplitMode::ChunkSize(5);
        let debug_str = format!("{mode:?}");
        assert!(debug_str.contains("ChunkSize"));
    }

    #[test]
    fn test_split_clone_implementations() {
        // Test Clone implementations
        let options = SplitOptions::default();
        let cloned_options = options.clone();
        assert_eq!(cloned_options.preserve_metadata, options.preserve_metadata);

        let mode = SplitMode::SinglePages;
        let cloned_mode = mode;
        assert!(matches!(cloned_mode, SplitMode::SinglePages));

        let mode = SplitMode::ChunkSize(3);
        let cloned_mode = mode;
        if let SplitMode::ChunkSize(size) = cloned_mode {
            assert_eq!(size, 3);
        }
    }

    #[test]
    fn test_split_single_chunk_size() {
        let temp_dir = TempDir::new().unwrap();
        let mut doc = create_test_pdf(5, "Single Chunk Test");
        let input_path = save_test_pdf(&mut doc, &temp_dir, "input.pdf");

        let options = SplitOptions {
            mode: SplitMode::ChunkSize(1), // Minimum valid chunk size
            output_pattern: temp_dir
                .path()
                .join("single_chunk_{}.pdf")
                .to_str()
                .unwrap()
                .to_string(),
            ..Default::default()
        };

        let result = split_pdf(&input_path, options);
        assert!(result.is_ok());
        // Should create 5 files (one per page)
        assert_eq!(result.unwrap().len(), 5);
    }

    #[test]
    fn test_split_empty_split_points() {
        let temp_dir = TempDir::new().unwrap();
        let mut doc = create_test_pdf(5, "Empty Split Points");
        let input_path = save_test_pdf(&mut doc, &temp_dir, "input.pdf");

        let options = SplitOptions {
            mode: SplitMode::SplitAt(vec![]), // Empty split points
            output_pattern: temp_dir
                .path()
                .join("empty_split_{}.pdf")
                .to_str()
                .unwrap()
                .to_string(),
            ..Default::default()
        };

        let result = split_pdf(&input_path, options);
        assert!(result.is_ok());
        // Should create one file with all pages
        assert_eq!(result.unwrap().len(), 1);
    }

    #[test]
    fn test_split_empty_ranges() {
        let temp_dir = TempDir::new().unwrap();
        let mut doc = create_test_pdf(5, "Empty Ranges");
        let input_path = save_test_pdf(&mut doc, &temp_dir, "input.pdf");

        let options = SplitOptions {
            mode: SplitMode::Ranges(vec![]), // Empty ranges
            output_pattern: temp_dir
                .path()
                .join("empty_ranges_{}.pdf")
                .to_str()
                .unwrap()
                .to_string(),
            ..Default::default()
        };

        let result = split_pdf(&input_path, options);
        assert!(result.is_ok());
        // Should handle gracefully
        assert_eq!(result.unwrap().len(), 0);
    }

    #[test]
    fn test_split_with_optimization() {
        let temp_dir = TempDir::new().unwrap();
        let mut doc = create_test_pdf(3, "Optimization Test");
        let input_path = save_test_pdf(&mut doc, &temp_dir, "input.pdf");

        let options = SplitOptions {
            mode: SplitMode::SinglePages,
            output_pattern: temp_dir
                .path()
                .join("optimized_{}.pdf")
                .to_str()
                .unwrap()
                .to_string(),
            preserve_metadata: true,
            optimize: true, // Enable optimization
        };

        let result = split_pdf(&input_path, options);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 3);
    }

    #[test]
    fn test_split_pattern_placeholders() {
        let temp_dir = TempDir::new().unwrap();
        let mut doc = create_test_pdf(3, "Pattern Test");
        let input_path = save_test_pdf(&mut doc, &temp_dir, "input.pdf");

        // Test different pattern placeholders
        let options = SplitOptions {
            mode: SplitMode::SinglePages,
            output_pattern: temp_dir
                .path()
                .join("pattern_test_{}.pdf")
                .to_str()
                .unwrap()
                .to_string(),
            ..Default::default()
        };

        let result = split_pdf(&input_path, options);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 3);
    }
}
