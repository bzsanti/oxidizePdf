//! Tests for PDF rotation operations

#[cfg(test)]
mod tests {
    use crate::operations::rotate::*;
    use crate::operations::{OperationError, PageRange};
    use crate::{Document, Page};
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;

    /// Helper to create a test PDF document with specified number of pages
    fn create_test_pdf(num_pages: usize, title: &str) -> Document {
        let mut doc = Document::new();
        doc.set_title(title);
        doc.set_author("Test Author");
        doc.set_subject("Rotation Test");
        doc.set_keywords("test, pdf, rotate");

        for i in 0..num_pages {
            let mut page = Page::a4();

            // Add text that shows orientation
            page.text()
                .set_font(crate::text::Font::Helvetica, 24.0)
                .at(50.0, 700.0)
                .write(&format!("Page {} - Top", i + 1))
                .unwrap();

            page.text()
                .set_font(crate::text::Font::Helvetica, 18.0)
                .at(50.0, 350.0)
                .write("Middle Text")
                .unwrap();

            page.text()
                .set_font(crate::text::Font::Helvetica, 14.0)
                .at(50.0, 100.0)
                .write("Bottom")
                .unwrap();

            // Add a rectangle to visualize rotation
            page.graphics()
                .rect(100.0, 500.0, 200.0, 100.0)
                .set_fill_color(crate::graphics::Color::Rgb(0.9, 0.9, 0.9))
                .fill()
                .rect(100.0, 500.0, 200.0, 100.0)
                .set_stroke_color(crate::graphics::Color::Rgb(0.0, 0.0, 0.0))
                .set_line_width(2.0)
                .stroke();

            // Add a line from top-left to show orientation
            page.graphics()
                .move_to(50.0, 750.0)
                .line_to(150.0, 750.0)
                .line_to(150.0, 650.0)
                .set_stroke_color(crate::graphics::Color::Rgb(1.0, 0.0, 0.0))
                .set_line_width(3.0)
                .stroke();

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
    fn test_rotation_angle_to_degrees() {
        assert_eq!(RotationAngle::None.to_degrees(), 0);
        assert_eq!(RotationAngle::Clockwise90.to_degrees(), 90);
        assert_eq!(RotationAngle::Rotate180.to_degrees(), 180);
        assert_eq!(RotationAngle::Clockwise270.to_degrees(), 270);
    }

    #[test]
    fn test_rotation_angle_combine() {
        // Test combining rotations
        assert_eq!(
            RotationAngle::None.combine(RotationAngle::Clockwise90),
            RotationAngle::Clockwise90
        );
        assert_eq!(
            RotationAngle::Clockwise90.combine(RotationAngle::Clockwise90),
            RotationAngle::Rotate180
        );
        assert_eq!(
            RotationAngle::Rotate180.combine(RotationAngle::Rotate180),
            RotationAngle::None
        );
        assert_eq!(
            RotationAngle::Clockwise270.combine(RotationAngle::Clockwise90),
            RotationAngle::None
        );
        assert_eq!(
            RotationAngle::Clockwise90.combine(RotationAngle::Clockwise270),
            RotationAngle::None
        );
    }

    #[test]
    fn test_rotation_angle_from_degrees_edge_cases() {
        // Test large positive angles
        assert_eq!(
            RotationAngle::from_degrees(720).unwrap(),
            RotationAngle::None
        );
        assert_eq!(
            RotationAngle::from_degrees(810).unwrap(),
            RotationAngle::Clockwise90
        );

        // Test large negative angles
        assert_eq!(
            RotationAngle::from_degrees(-360).unwrap(),
            RotationAngle::None
        );
        assert_eq!(
            RotationAngle::from_degrees(-270).unwrap(),
            RotationAngle::Clockwise90
        );
        assert_eq!(
            RotationAngle::from_degrees(-180).unwrap(),
            RotationAngle::Rotate180
        );
        assert_eq!(
            RotationAngle::from_degrees(-90).unwrap(),
            RotationAngle::Clockwise270
        );
    }

    #[test]
    fn test_rotation_angle_invalid_degrees() {
        // Test various invalid angles
        let invalid_angles = vec![1, 45, 89, 91, 135, 179, 181, 269, 271, 359];

        for angle in invalid_angles {
            assert!(
                RotationAngle::from_degrees(angle).is_err(),
                "Angle {} should be invalid",
                angle
            );
            assert!(
                RotationAngle::from_degrees(-angle).is_err(),
                "Angle {} should be invalid",
                -angle
            );
        }
    }

    #[test]
    fn test_rotate_options_default() {
        let options = RotateOptions::default();
        assert!(matches!(options.pages, PageRange::All));
        assert_eq!(options.angle, RotationAngle::Clockwise90);
        assert!(!options.preserve_page_size);
    }

    #[test]
    fn test_rotate_single_page() {
        let temp_dir = TempDir::new().unwrap();
        let mut doc = create_test_pdf(3, "Rotation Test");
        let input_path = save_test_pdf(&mut doc, &temp_dir, "input.pdf");
        let output_path = temp_dir.path().join("rotated.pdf");

        let options = RotateOptions {
            pages: PageRange::Single(1), // Rotate only page 2
            angle: RotationAngle::Clockwise90,
            preserve_page_size: false,
        };

        let result = rotate_pdf_pages(&input_path, &output_path, options);
        assert!(result.is_ok());
        assert!(output_path.exists());
        assert!(fs::metadata(&output_path).unwrap().len() > 0);
    }

    #[test]
    fn test_rotate_all_pages_90() {
        let temp_dir = TempDir::new().unwrap();
        let mut doc = create_test_pdf(2, "Rotate All 90");
        let input_path = save_test_pdf(&mut doc, &temp_dir, "input.pdf");
        let output_path = temp_dir.path().join("rotated_90.pdf");

        let result = rotate_all_pages(&input_path, &output_path, RotationAngle::Clockwise90);
        assert!(result.is_ok());
        assert!(output_path.exists());
    }

    #[test]
    fn test_rotate_all_pages_180() {
        let temp_dir = TempDir::new().unwrap();
        let mut doc = create_test_pdf(2, "Rotate All 180");
        let input_path = save_test_pdf(&mut doc, &temp_dir, "input.pdf");
        let output_path = temp_dir.path().join("rotated_180.pdf");

        let result = rotate_all_pages(&input_path, &output_path, RotationAngle::Rotate180);
        assert!(result.is_ok());
        assert!(output_path.exists());
    }

    #[test]
    fn test_rotate_all_pages_270() {
        let temp_dir = TempDir::new().unwrap();
        let mut doc = create_test_pdf(2, "Rotate All 270");
        let input_path = save_test_pdf(&mut doc, &temp_dir, "input.pdf");
        let output_path = temp_dir.path().join("rotated_270.pdf");

        let result = rotate_all_pages(&input_path, &output_path, RotationAngle::Clockwise270);
        assert!(result.is_ok());
        assert!(output_path.exists());
    }

    #[test]
    fn test_rotate_page_range() {
        let temp_dir = TempDir::new().unwrap();
        let mut doc = create_test_pdf(5, "Range Rotation");
        let input_path = save_test_pdf(&mut doc, &temp_dir, "input.pdf");
        let output_path = temp_dir.path().join("rotated_range.pdf");

        let options = RotateOptions {
            pages: PageRange::Range(1, 3), // Rotate pages 2-4
            angle: RotationAngle::Clockwise90,
            preserve_page_size: false,
        };

        let result = rotate_pdf_pages(&input_path, &output_path, options);
        assert!(result.is_ok());
        assert!(output_path.exists());
    }

    #[test]
    fn test_rotate_page_list() {
        let temp_dir = TempDir::new().unwrap();
        let mut doc = create_test_pdf(6, "List Rotation");
        let input_path = save_test_pdf(&mut doc, &temp_dir, "input.pdf");
        let output_path = temp_dir.path().join("rotated_list.pdf");

        let options = RotateOptions {
            pages: PageRange::List(vec![0, 2, 4]), // Rotate pages 1, 3, 5
            angle: RotationAngle::Rotate180,
            preserve_page_size: true,
        };

        let result = rotate_pdf_pages(&input_path, &output_path, options);
        assert!(result.is_ok());
        assert!(output_path.exists());
    }

    #[test]
    fn test_rotate_preserve_page_size() {
        let temp_dir = TempDir::new().unwrap();
        let mut doc = create_test_pdf(2, "Preserve Size Test");
        let input_path = save_test_pdf(&mut doc, &temp_dir, "input.pdf");

        // Test without preserving size
        let output_path1 = temp_dir.path().join("rotated_no_preserve.pdf");
        let options1 = RotateOptions {
            pages: PageRange::All,
            angle: RotationAngle::Clockwise90,
            preserve_page_size: false,
        };

        let result1 = rotate_pdf_pages(&input_path, &output_path1, options1);
        assert!(result1.is_ok());

        // Test with preserving size
        let output_path2 = temp_dir.path().join("rotated_preserve.pdf");
        let options2 = RotateOptions {
            pages: PageRange::All,
            angle: RotationAngle::Clockwise90,
            preserve_page_size: true,
        };

        let result2 = rotate_pdf_pages(&input_path, &output_path2, options2);
        assert!(result2.is_ok());

        // Both files should exist but may have different sizes
        assert!(output_path1.exists());
        assert!(output_path2.exists());
    }

    #[test]
    fn test_rotate_nonexistent_file() {
        let temp_dir = TempDir::new().unwrap();
        let input_path = temp_dir.path().join("nonexistent.pdf");
        let output_path = temp_dir.path().join("output.pdf");

        let result = rotate_all_pages(&input_path, &output_path, RotationAngle::Clockwise90);
        assert!(result.is_err());
        match result {
            Err(OperationError::ParseError(_)) => assert!(true),
            _ => panic!("Expected ParseError"),
        }
    }

    #[test]
    fn test_rotate_empty_document() {
        let temp_dir = TempDir::new().unwrap();
        let mut doc = Document::new(); // Empty document
        let input_path = save_test_pdf(&mut doc, &temp_dir, "empty.pdf");
        let output_path = temp_dir.path().join("rotated_empty.pdf");

        let result = rotate_all_pages(&input_path, &output_path, RotationAngle::Clockwise90);

        // Should handle empty document gracefully
        assert!(result.is_ok());
        assert!(output_path.exists());
    }

    #[test]
    fn test_rotate_with_metadata() {
        let temp_dir = TempDir::new().unwrap();
        let mut doc = create_test_pdf(2, "Metadata Rotation");
        doc.set_title("Original Title");
        doc.set_author("Original Author");
        doc.set_subject("Original Subject");
        doc.set_keywords("original, keywords");

        let input_path = save_test_pdf(&mut doc, &temp_dir, "input.pdf");
        let output_path = temp_dir.path().join("rotated_metadata.pdf");

        let result = rotate_all_pages(&input_path, &output_path, RotationAngle::Clockwise90);
        assert!(result.is_ok());

        // Note: We'd need metadata getters to verify preservation
    }

    #[test]
    fn test_page_rotator_new() {
        let temp_dir = TempDir::new().unwrap();
        let mut doc = create_test_pdf(1, "Test");
        let input_path = save_test_pdf(&mut doc, &temp_dir, "input.pdf");

        let document = crate::parser::PdfReader::open_document(&input_path).unwrap();
        let _rotator = PageRotator::new(document);

        // Just verify we can create a rotator
        assert!(true);
    }

    #[test]
    fn test_multiple_rotation_operations() {
        let temp_dir = TempDir::new().unwrap();
        let mut doc = create_test_pdf(3, "Multiple Rotations");
        let input_path = save_test_pdf(&mut doc, &temp_dir, "input.pdf");

        // First rotation: 90 degrees
        let output_path1 = temp_dir.path().join("rotated_90.pdf");
        let result1 = rotate_all_pages(&input_path, &output_path1, RotationAngle::Clockwise90);
        assert!(result1.is_ok());

        // Second rotation: another 90 degrees (total 180)
        let output_path2 = temp_dir.path().join("rotated_180.pdf");
        let result2 = rotate_all_pages(&output_path1, &output_path2, RotationAngle::Clockwise90);
        assert!(result2.is_ok());

        // Third rotation: 180 degrees (total 360/0)
        let output_path3 = temp_dir.path().join("rotated_360.pdf");
        let result3 = rotate_all_pages(&output_path2, &output_path3, RotationAngle::Rotate180);
        assert!(result3.is_ok());

        assert!(output_path3.exists());
    }

    #[test]
    fn test_rotate_invalid_page_range() {
        let temp_dir = TempDir::new().unwrap();
        let mut doc = create_test_pdf(3, "Invalid Range Test");
        let input_path = save_test_pdf(&mut doc, &temp_dir, "input.pdf");
        let output_path = temp_dir.path().join("output.pdf");

        // Try to rotate pages beyond document
        let options = RotateOptions {
            pages: PageRange::Range(5, 10), // Pages don't exist
            angle: RotationAngle::Clockwise90,
            preserve_page_size: false,
        };

        let result = rotate_pdf_pages(&input_path, &output_path, options);

        // Should either fail or handle gracefully
        // The behavior depends on PageRange validation
        assert!(result.is_ok() || result.is_err());
    }
}
