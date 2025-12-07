//! Tests for PDF merge operations

#[cfg(test)]
mod tests {
    use crate::operations::merge::*;
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

        for i in 0..num_pages {
            let mut page = Page::a4();
            page.text()
                .set_font(crate::text::Font::Helvetica, 24.0)
                .at(50.0, 700.0)
                .write(&format!("{} - Page {}", title, i + 1))
                .unwrap();
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
    fn test_merge_options_custom_metadata() {
        let options = MergeOptions {
            page_ranges: Some(vec![PageRange::All]),
            preserve_bookmarks: false,
            preserve_forms: true,
            optimize: true,
            metadata_mode: MetadataMode::Custom {
                title: Some("Merged Document".to_string()),
                author: Some("Merger".to_string()),
                subject: Some("Test Subject".to_string()),
                keywords: Some("test, pdf, merge".to_string()),
            },
        };

        assert!(options.page_ranges.is_some());
        assert!(!options.preserve_bookmarks);
        assert!(options.preserve_forms);
        assert!(options.optimize);

        match options.metadata_mode {
            MetadataMode::Custom { ref title, .. } => {
                assert_eq!(title.as_deref(), Some("Merged Document"));
            }
            _ => panic!("Expected Custom metadata mode"),
        }
    }

    #[test]
    fn test_metadata_mode_variants() {
        let from_first = MetadataMode::FromFirst;
        let from_second = MetadataMode::FromDocument(1);
        let none = MetadataMode::None;

        // Test pattern matching
        match from_first {
            MetadataMode::FromFirst => {}
            _ => panic!("Wrong variant"),
        }

        match from_second {
            MetadataMode::FromDocument(idx) => assert_eq!(idx, 1),
            _ => panic!("Wrong variant"),
        }

        match none {
            MetadataMode::None => {}
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn test_pdf_merger_new() {
        let merger = PdfMerger::new(MergeOptions::default());
        assert_eq!(merger.inputs.len(), 0);
        assert!(merger.object_mappings.is_empty());
    }

    #[test]
    fn test_pdf_merger_add_input() {
        let mut merger = PdfMerger::new(MergeOptions::default());

        merger.add_input(MergeInput::new("test1.pdf"));
        assert_eq!(merger.inputs.len(), 1);

        merger.add_input(MergeInput::with_pages("test2.pdf", PageRange::Range(0, 5)));
        assert_eq!(merger.inputs.len(), 2);

        assert_eq!(merger.inputs[0].path, PathBuf::from("test1.pdf"));
        assert!(merger.inputs[0].pages.is_none());
        assert!(merger.inputs[1].pages.is_some());
    }

    #[test]
    fn test_pdf_merger_add_inputs() {
        let mut merger = PdfMerger::new(MergeOptions::default());

        let inputs = vec![
            MergeInput::new("test1.pdf"),
            MergeInput::new("test2.pdf"),
            MergeInput::new("test3.pdf"),
        ];

        merger.add_inputs(inputs);
        assert_eq!(merger.inputs.len(), 3);
    }

    #[test]
    fn test_merge_empty_inputs() {
        let mut merger = PdfMerger::new(MergeOptions::default());
        let result = merger.merge();

        assert!(result.is_err());
        match result {
            Err(crate::operations::OperationError::NoPagesToProcess) => {}
            _ => panic!("Expected NoPagesToProcess error"),
        }
    }

    #[test]
    fn test_merge_two_documents() {
        let temp_dir = TempDir::new().unwrap();

        // Create two test PDFs
        let mut doc1 = create_test_pdf(3, "Document 1");
        let mut doc2 = create_test_pdf(2, "Document 2");

        let path1 = save_test_pdf(&mut doc1, &temp_dir, "doc1.pdf");
        let path2 = save_test_pdf(&mut doc2, &temp_dir, "doc2.pdf");

        // Merge them
        let mut merger = PdfMerger::new(MergeOptions::default());
        merger.add_input(MergeInput::new(&path1));
        merger.add_input(MergeInput::new(&path2));

        let result = merger.merge();
        assert!(result.is_ok());

        let _merged_doc = result.unwrap();
        // Should have 5 pages total (3 + 2)
        // Note: We can't directly check page count without accessing internal state
        // but we can verify the merge succeeded
    }

    #[test]
    fn test_merge_with_page_ranges() {
        let temp_dir = TempDir::new().unwrap();

        // Create test PDFs
        let mut doc1 = create_test_pdf(5, "Document 1");
        let mut doc2 = create_test_pdf(4, "Document 2");

        let path1 = save_test_pdf(&mut doc1, &temp_dir, "doc1.pdf");
        let path2 = save_test_pdf(&mut doc2, &temp_dir, "doc2.pdf");

        // Merge with specific page ranges
        let mut merger = PdfMerger::new(MergeOptions::default());
        merger.add_input(MergeInput::with_pages(&path1, PageRange::Range(0, 2))); // First 3 pages
        merger.add_input(MergeInput::with_pages(&path2, PageRange::Single(1))); // Second page only

        let result = merger.merge();
        assert!(result.is_ok());
    }

    #[test]
    fn test_merge_to_file() {
        let temp_dir = TempDir::new().unwrap();

        // Create test PDFs
        let mut doc1 = create_test_pdf(2, "Document 1");
        let mut doc2 = create_test_pdf(2, "Document 2");

        let path1 = save_test_pdf(&mut doc1, &temp_dir, "doc1.pdf");
        let path2 = save_test_pdf(&mut doc2, &temp_dir, "doc2.pdf");
        let output_path = temp_dir.path().join("merged.pdf");

        // Merge to file
        let mut merger = PdfMerger::new(MergeOptions::default());
        merger.add_input(MergeInput::new(&path1));
        merger.add_input(MergeInput::new(&path2));

        let result = merger.merge_to_file(&output_path);
        assert!(result.is_ok());

        // Verify output file exists
        assert!(output_path.exists());
        assert!(fs::metadata(&output_path).unwrap().len() > 0);
    }

    #[test]
    fn test_merge_with_custom_metadata() {
        let temp_dir = TempDir::new().unwrap();

        // Create test PDFs with metadata
        let mut doc1 = create_test_pdf(1, "Original Title 1");
        doc1.set_subject("Original Subject 1");

        let mut doc2 = create_test_pdf(1, "Original Title 2");
        doc2.set_subject("Original Subject 2");

        let path1 = save_test_pdf(&mut doc1, &temp_dir, "doc1.pdf");
        let path2 = save_test_pdf(&mut doc2, &temp_dir, "doc2.pdf");

        // Merge with custom metadata
        let options = MergeOptions {
            metadata_mode: MetadataMode::Custom {
                title: Some("Custom Merged Title".to_string()),
                author: Some("Custom Author".to_string()),
                subject: Some("Custom Subject".to_string()),
                keywords: Some("custom, keywords".to_string()),
            },
            ..Default::default()
        };

        let mut merger = PdfMerger::new(options);
        merger.add_input(MergeInput::new(&path1));
        merger.add_input(MergeInput::new(&path2));

        let result = merger.merge();
        assert!(result.is_ok());

        // Note: We'd need metadata getters to verify the custom metadata was applied
    }

    #[test]
    fn test_merge_preserve_metadata_from_first() {
        let temp_dir = TempDir::new().unwrap();

        // Create test PDFs
        let mut doc1 = create_test_pdf(1, "First Document");
        doc1.set_keywords("first, document");

        let mut doc2 = create_test_pdf(1, "Second Document");
        doc2.set_keywords("second, document");

        let path1 = save_test_pdf(&mut doc1, &temp_dir, "doc1.pdf");
        let path2 = save_test_pdf(&mut doc2, &temp_dir, "doc2.pdf");

        // Default options use metadata from first
        let mut merger = PdfMerger::new(MergeOptions::default());
        merger.add_input(MergeInput::new(&path1));
        merger.add_input(MergeInput::new(&path2));

        let result = merger.merge();
        assert!(result.is_ok());
    }

    #[test]
    fn test_merge_preserve_metadata_from_specific() {
        let temp_dir = TempDir::new().unwrap();

        // Create test PDFs
        let mut doc1 = create_test_pdf(1, "First Document");
        let mut doc2 = create_test_pdf(1, "Second Document");
        let mut doc3 = create_test_pdf(1, "Third Document");

        let path1 = save_test_pdf(&mut doc1, &temp_dir, "doc1.pdf");
        let path2 = save_test_pdf(&mut doc2, &temp_dir, "doc2.pdf");
        let path3 = save_test_pdf(&mut doc3, &temp_dir, "doc3.pdf");

        // Use metadata from second document (index 1)
        let options = MergeOptions {
            metadata_mode: MetadataMode::FromDocument(1),
            ..Default::default()
        };

        let mut merger = PdfMerger::new(options);
        merger.add_input(MergeInput::new(&path1));
        merger.add_input(MergeInput::new(&path2));
        merger.add_input(MergeInput::new(&path3));

        let result = merger.merge();
        assert!(result.is_ok());
    }

    #[test]
    fn test_merge_pdfs_function() {
        let temp_dir = TempDir::new().unwrap();

        // Create test PDFs
        let mut doc1 = create_test_pdf(2, "Document 1");
        let mut doc2 = create_test_pdf(3, "Document 2");

        let path1 = save_test_pdf(&mut doc1, &temp_dir, "doc1.pdf");
        let path2 = save_test_pdf(&mut doc2, &temp_dir, "doc2.pdf");
        let output_path = temp_dir.path().join("merged.pdf");

        let inputs = vec![MergeInput::new(&path1), MergeInput::new(&path2)];

        let result = merge_pdfs(inputs, &output_path, MergeOptions::default());
        assert!(result.is_ok());
        assert!(output_path.exists());
    }

    #[test]
    fn test_merge_pdf_files_simple() {
        let temp_dir = TempDir::new().unwrap();

        // Create test PDFs
        let mut doc1 = create_test_pdf(1, "Document 1");
        let mut doc2 = create_test_pdf(1, "Document 2");
        let mut doc3 = create_test_pdf(1, "Document 3");

        let path1 = save_test_pdf(&mut doc1, &temp_dir, "doc1.pdf");
        let path2 = save_test_pdf(&mut doc2, &temp_dir, "doc2.pdf");
        let path3 = save_test_pdf(&mut doc3, &temp_dir, "doc3.pdf");
        let output_path = temp_dir.path().join("merged.pdf");

        let input_paths = vec![&path1, &path2, &path3];
        let result = merge_pdf_files(&input_paths, &output_path);

        assert!(result.is_ok());
        assert!(output_path.exists());
    }

    #[test]
    fn test_merge_invalid_file() {
        let temp_dir = TempDir::new().unwrap();
        let invalid_path = temp_dir.path().join("nonexistent.pdf");
        let output_path = temp_dir.path().join("output.pdf");

        let mut merger = PdfMerger::new(MergeOptions::default());
        merger.add_input(MergeInput::new(&invalid_path));

        let result = merger.merge_to_file(&output_path);
        assert!(result.is_err());
    }

    #[test]
    fn test_merge_with_all_page_range_types() {
        let temp_dir = TempDir::new().unwrap();

        // Create test PDF with multiple pages
        let mut doc = create_test_pdf(10, "Test Document");
        let path = save_test_pdf(&mut doc, &temp_dir, "doc.pdf");
        let output_path = temp_dir.path().join("merged.pdf");

        let inputs = vec![
            MergeInput::with_pages(&path, PageRange::All),
            MergeInput::with_pages(&path, PageRange::Single(5)),
            MergeInput::with_pages(&path, PageRange::Range(0, 2)),
            MergeInput::with_pages(&path, PageRange::List(vec![1, 3, 5, 7])),
        ];

        let mut merger = PdfMerger::new(MergeOptions::default());
        merger.add_inputs(inputs);

        let result = merger.merge_to_file(&output_path);
        assert!(result.is_ok());
        assert!(output_path.exists());
    }

    #[test]
    fn test_object_number_allocation() {
        let mut merger = PdfMerger::new(MergeOptions::default());

        // Test internal object allocation
        let num1 = merger.allocate_object_number();
        let num2 = merger.allocate_object_number();
        let num3 = merger.allocate_object_number();

        assert_eq!(num1, 1);
        assert_eq!(num2, 2);
        assert_eq!(num3, 3);
    }

    #[test]
    fn test_object_number_mapping() {
        let mut merger = PdfMerger::new(MergeOptions::default());

        // Initialize mappings for two documents
        merger
            .object_mappings
            .push(std::collections::HashMap::new());
        merger
            .object_mappings
            .push(std::collections::HashMap::new());

        // Map object from first document
        let new_num1 = merger.map_object_number(0, 10);
        assert_eq!(new_num1, 1);

        // Mapping same object again should return same number
        let new_num1_again = merger.map_object_number(0, 10);
        assert_eq!(new_num1_again, 1);

        // Map object from second document
        let new_num2 = merger.map_object_number(1, 10);
        assert_eq!(new_num2, 2); // Different from first document's mapping
    }

    #[test]
    fn test_merge_options_variants() {
        // Test all MergeOptions variants
        let default_options = MergeOptions::default();
        assert!(default_options.preserve_bookmarks);
        assert!(!default_options.preserve_forms);
        assert!(!default_options.optimize);
        assert!(matches!(
            default_options.metadata_mode,
            MetadataMode::FromFirst
        ));
        assert!(default_options.page_ranges.is_none());

        let custom_options = MergeOptions {
            page_ranges: Some(vec![PageRange::Single(0), PageRange::Range(1, 3)]),
            preserve_bookmarks: false,
            preserve_forms: true,
            optimize: true,
            metadata_mode: MetadataMode::None,
        };
        assert!(!custom_options.preserve_bookmarks);
        assert!(custom_options.preserve_forms);
        assert!(custom_options.optimize);
        assert!(matches!(custom_options.metadata_mode, MetadataMode::None));
        assert!(custom_options.page_ranges.is_some());
    }

    #[test]
    fn test_all_metadata_mode_variants() {
        // Test all MetadataMode variants
        let modes = vec![
            MetadataMode::FromFirst,
            MetadataMode::FromDocument(2),
            MetadataMode::Custom {
                title: Some("Title".to_string()),
                author: Some("Author".to_string()),
                subject: Some("Subject".to_string()),
                keywords: Some("Keywords".to_string()),
            },
            MetadataMode::None,
        ];

        for mode in modes {
            let options = MergeOptions {
                metadata_mode: mode,
                ..Default::default()
            };
            // Just verify we can create options with all metadata modes
            assert!(options.preserve_bookmarks);
        }
    }

    #[test]
    fn test_merge_input_creation() {
        let path = PathBuf::from("test.pdf");

        // Test MergeInput::new
        let input1 = MergeInput::new(&path);
        assert_eq!(input1.path, path);
        assert!(input1.pages.is_none());

        // Test MergeInput::with_pages
        let input2 = MergeInput::with_pages(&path, PageRange::Single(5));
        assert_eq!(input2.path, path);
        assert!(input2.pages.is_some());
        if let Some(PageRange::Single(page)) = input2.pages {
            assert_eq!(page, 5);
        }

        let input3 = MergeInput::with_pages(&path, PageRange::Range(1, 10));
        assert_eq!(input3.path, path);
        assert!(input3.pages.is_some());
        if let Some(PageRange::Range(start, end)) = input3.pages {
            assert_eq!(start, 1);
            assert_eq!(end, 10);
        }
    }

    #[test]
    fn test_merger_add_inputs() {
        let mut merger = PdfMerger::new(MergeOptions::default());

        // Test adding single input
        let input1 = MergeInput::new("doc1.pdf");
        merger.add_input(input1);
        assert_eq!(merger.inputs.len(), 1);

        // Test adding multiple inputs
        let inputs = vec![
            MergeInput::new("doc2.pdf"),
            MergeInput::new("doc3.pdf"),
            MergeInput::with_pages("doc4.pdf", PageRange::Range(0, 2)),
        ];
        merger.add_inputs(inputs);
        assert_eq!(merger.inputs.len(), 4);
    }

    #[test]
    fn test_merge_with_no_metadata_mode() {
        let temp_dir = TempDir::new().unwrap();

        // Create test PDFs with metadata
        let mut doc1 = create_test_pdf(1, "Document 1");
        doc1.set_author("Author 1");
        doc1.set_subject("Subject 1");

        let mut doc2 = create_test_pdf(1, "Document 2");
        doc2.set_author("Author 2");
        doc2.set_subject("Subject 2");

        let path1 = save_test_pdf(&mut doc1, &temp_dir, "doc1.pdf");
        let path2 = save_test_pdf(&mut doc2, &temp_dir, "doc2.pdf");

        // Use None metadata mode
        let options = MergeOptions {
            metadata_mode: MetadataMode::None,
            ..Default::default()
        };

        let mut merger = PdfMerger::new(options);
        merger.add_input(MergeInput::new(&path1));
        merger.add_input(MergeInput::new(&path2));

        let result = merger.merge();
        assert!(result.is_ok());
    }

    #[test]
    fn test_merge_with_partial_custom_metadata() {
        let temp_dir = TempDir::new().unwrap();

        let mut doc1 = create_test_pdf(1, "Document 1");
        let path1 = save_test_pdf(&mut doc1, &temp_dir, "doc1.pdf");

        // Custom metadata with only some fields
        let options = MergeOptions {
            metadata_mode: MetadataMode::Custom {
                title: Some("Custom Title".to_string()),
                author: None,
                subject: Some("Custom Subject".to_string()),
                keywords: None,
            },
            ..Default::default()
        };

        let mut merger = PdfMerger::new(options);
        merger.add_input(MergeInput::new(&path1));

        let result = merger.merge();
        assert!(result.is_ok());
    }

    #[test]
    fn test_merge_debug_implementations() {
        // Test Debug implementations
        let options = MergeOptions::default();
        let debug_str = format!("{options:?}");
        assert!(debug_str.contains("MergeOptions"));

        let metadata_mode = MetadataMode::FromFirst;
        let debug_str = format!("{metadata_mode:?}");
        assert!(debug_str.contains("FromFirst"));

        let input = MergeInput::new("test.pdf");
        let debug_str = format!("{input:?}");
        assert!(debug_str.contains("MergeInput"));
    }

    #[test]
    fn test_merge_clone_implementations() {
        // Test Clone implementations
        let options = MergeOptions::default();
        let cloned_options = options.clone();
        assert_eq!(
            cloned_options.preserve_bookmarks,
            options.preserve_bookmarks
        );

        let metadata_mode = MetadataMode::FromFirst;
        let cloned_mode = metadata_mode;
        assert!(matches!(cloned_mode, MetadataMode::FromFirst));
    }
}
