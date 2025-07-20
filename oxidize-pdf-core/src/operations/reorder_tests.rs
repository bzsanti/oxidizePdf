//! Tests for PDF page reordering operations

#[cfg(test)]
mod tests {
    use crate::operations::reorder::*;
    use crate::{Document, Page};
    use std::fs;
    use tempfile::TempDir;

    /// Helper to create a test PDF document with numbered pages
    fn create_test_pdf(num_pages: usize) -> Document {
        let mut doc = Document::new();
        doc.set_title("Test Reorder Document");

        for i in 0..num_pages {
            let mut page = Page::a4();
            page.text()
                .set_font(crate::text::Font::Helvetica, 24.0)
                .at(100.0, 700.0)
                .write(&format!("Page {}", i + 1))
                .unwrap();
            doc.add_page(page);
        }

        doc
    }

    #[test]
    fn test_page_reorderer_new() {
        let temp_dir = TempDir::new().unwrap();
        let mut doc = create_test_pdf(3);
        let path = temp_dir.path().join("test.pdf");
        doc.save(&path).unwrap();

        let document = crate::parser::PdfReader::open_document(&path).unwrap();
        let options = ReorderOptions {
            page_order: vec![2, 0, 1],
            ..Default::default()
        };

        let reorderer = PageReorderer::new(document, options);
        assert_eq!(reorderer.options.page_order, vec![2, 0, 1]);
    }

    #[test]
    fn test_reorder_simple() {
        let temp_dir = TempDir::new().unwrap();
        let mut doc = create_test_pdf(3);
        let input_path = temp_dir.path().join("input.pdf");
        let output_path = temp_dir.path().join("output.pdf");
        doc.save(&input_path).unwrap();

        let result = reorder_pdf_pages(&input_path, &output_path, vec![2, 0, 1]);
        assert!(result.is_ok());
        assert!(output_path.exists());
        assert!(fs::metadata(&output_path).unwrap().len() > 0);
    }

    #[test]
    fn test_reorder_all_pages() {
        let temp_dir = TempDir::new().unwrap();
        let mut doc = create_test_pdf(5);
        let input_path = temp_dir.path().join("input.pdf");
        let output_path = temp_dir.path().join("output.pdf");
        doc.save(&input_path).unwrap();

        // Reorder all pages: 5, 3, 1, 2, 4
        let result = reorder_pdf_pages(&input_path, &output_path, vec![4, 2, 0, 1, 3]);
        assert!(result.is_ok());
        assert!(output_path.exists());
    }

    #[test]
    fn test_reorder_subset_of_pages() {
        let temp_dir = TempDir::new().unwrap();
        let mut doc = create_test_pdf(5);
        let input_path = temp_dir.path().join("input.pdf");
        let output_path = temp_dir.path().join("output.pdf");
        doc.save(&input_path).unwrap();

        // Select only some pages
        let result = reorder_pdf_pages(&input_path, &output_path, vec![0, 2, 4]);
        assert!(result.is_ok());
        assert!(output_path.exists());
    }

    #[test]
    fn test_reorder_duplicate_pages() {
        let temp_dir = TempDir::new().unwrap();
        let mut doc = create_test_pdf(3);
        let input_path = temp_dir.path().join("input.pdf");
        let output_path = temp_dir.path().join("output.pdf");
        doc.save(&input_path).unwrap();

        // Duplicate page 1
        let result = reorder_pdf_pages(&input_path, &output_path, vec![0, 0, 1, 2]);
        assert!(result.is_ok());
        assert!(output_path.exists());
    }

    #[test]
    fn test_reorder_empty_order() {
        let temp_dir = TempDir::new().unwrap();
        let mut doc = create_test_pdf(3);
        let input_path = temp_dir.path().join("input.pdf");
        let output_path = temp_dir.path().join("output.pdf");
        doc.save(&input_path).unwrap();

        let result = reorder_pdf_pages(&input_path, &output_path, vec![]);
        assert!(result.is_err());
    }

    #[test]
    fn test_reorder_out_of_bounds() {
        let temp_dir = TempDir::new().unwrap();
        let mut doc = create_test_pdf(3);
        let input_path = temp_dir.path().join("input.pdf");
        let output_path = temp_dir.path().join("output.pdf");
        doc.save(&input_path).unwrap();

        // Try to access page 5 when only 3 exist
        let result = reorder_pdf_pages(&input_path, &output_path, vec![0, 1, 5]);
        assert!(result.is_err());
    }

    #[test]
    fn test_reverse_pdf_pages() {
        let temp_dir = TempDir::new().unwrap();
        let mut doc = create_test_pdf(4);
        let input_path = temp_dir.path().join("input.pdf");
        let output_path = temp_dir.path().join("output.pdf");
        doc.save(&input_path).unwrap();

        let result = reverse_pdf_pages(&input_path, &output_path);
        assert!(result.is_ok());
        assert!(output_path.exists());
    }

    #[test]
    fn test_move_pdf_page() {
        let temp_dir = TempDir::new().unwrap();
        let mut doc = create_test_pdf(5);
        let input_path = temp_dir.path().join("input.pdf");
        let output_path = temp_dir.path().join("output.pdf");
        doc.save(&input_path).unwrap();

        // Move page 0 to position 3
        let result = move_pdf_page(&input_path, &output_path, 0, 3);
        assert!(result.is_ok());
        assert!(output_path.exists());
    }

    #[test]
    fn test_move_pdf_page_out_of_bounds() {
        let temp_dir = TempDir::new().unwrap();
        let mut doc = create_test_pdf(3);
        let input_path = temp_dir.path().join("input.pdf");
        let output_path = temp_dir.path().join("output.pdf");
        doc.save(&input_path).unwrap();

        let result = move_pdf_page(&input_path, &output_path, 0, 5);
        assert!(result.is_err());
    }

    #[test]
    fn test_swap_pdf_pages() {
        let temp_dir = TempDir::new().unwrap();
        let mut doc = create_test_pdf(4);
        let input_path = temp_dir.path().join("input.pdf");
        let output_path = temp_dir.path().join("output.pdf");
        doc.save(&input_path).unwrap();

        // Swap first and last pages
        let result = swap_pdf_pages(&input_path, &output_path, 0, 3);
        assert!(result.is_ok());
        assert!(output_path.exists());
    }

    #[test]
    fn test_swap_same_page() {
        let temp_dir = TempDir::new().unwrap();
        let mut doc = create_test_pdf(3);
        let input_path = temp_dir.path().join("input.pdf");
        let output_path = temp_dir.path().join("output.pdf");
        doc.save(&input_path).unwrap();

        // Swap page with itself (should work but have no effect)
        let result = swap_pdf_pages(&input_path, &output_path, 1, 1);
        assert!(result.is_ok());
        assert!(output_path.exists());
    }

    #[test]
    fn test_reorder_with_metadata() {
        let temp_dir = TempDir::new().unwrap();
        let mut doc = create_test_pdf(3);
        doc.set_author("Test Author");
        doc.set_subject("Test Subject");
        doc.set_keywords("test, reorder, pdf");

        let input_path = temp_dir.path().join("input.pdf");
        let output_path = temp_dir.path().join("output.pdf");
        doc.save(&input_path).unwrap();

        let document = crate::parser::PdfReader::open_document(&input_path).unwrap();
        let options = ReorderOptions {
            page_order: vec![2, 0, 1],
            preserve_metadata: true,
            optimize: false,
        };

        let reorderer = PageReorderer::new(document, options);
        let result = reorderer.reorder_to_file(&output_path);
        assert!(result.is_ok());
        assert!(output_path.exists());
    }

    #[test]
    fn test_reorder_without_metadata() {
        let temp_dir = TempDir::new().unwrap();
        let mut doc = create_test_pdf(3);
        doc.set_author("Test Author");

        let input_path = temp_dir.path().join("input.pdf");
        let output_path = temp_dir.path().join("output.pdf");
        doc.save(&input_path).unwrap();

        let document = crate::parser::PdfReader::open_document(&input_path).unwrap();
        let options = ReorderOptions {
            page_order: vec![2, 0, 1],
            preserve_metadata: false,
            optimize: false,
        };

        let reorderer = PageReorderer::new(document, options);
        let result = reorderer.reorder_to_file(&output_path);
        assert!(result.is_ok());
        assert!(output_path.exists());
    }

    #[test]
    fn test_edge_cases() {
        let temp_dir = TempDir::new().unwrap();

        // Single page document
        let mut doc = create_test_pdf(1);
        let input_path = temp_dir.path().join("single.pdf");
        let output_path = temp_dir.path().join("single_out.pdf");
        doc.save(&input_path).unwrap();

        let result = reorder_pdf_pages(&input_path, &output_path, vec![0]);
        assert!(result.is_ok());

        // Empty document (0 pages)
        let _empty_doc = Document::new();
        let _empty_path = temp_dir.path().join("empty.pdf");
        let _empty_out = temp_dir.path().join("empty_out.pdf");
        // Note: We can't save an empty document, so we'll test differently

        // Large reorder
        let mut large_doc = create_test_pdf(10);
        let large_path = temp_dir.path().join("large.pdf");
        let large_out = temp_dir.path().join("large_out.pdf");
        large_doc.save(&large_path).unwrap();

        // Reverse order for large document
        let order: Vec<usize> = (0..10).rev().collect();
        let result = reorder_pdf_pages(&large_path, &large_out, order);
        assert!(result.is_ok());
    }
}
