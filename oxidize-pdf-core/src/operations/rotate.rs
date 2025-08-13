//! PDF page rotation functionality
//!
//! This module provides functionality to rotate pages in PDF documents.

use super::{OperationError, OperationResult, PageRange};
use crate::parser::page_tree::ParsedPage;
use crate::parser::{ContentOperation, ContentParser, PdfDocument, PdfReader};
use crate::{Document, Page};
use std::fs::File;
use std::path::Path;

/// Rotation angle
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RotationAngle {
    /// No rotation (0 degrees)
    None,
    /// 90 degrees clockwise
    Clockwise90,
    /// 180 degrees
    Rotate180,
    /// 270 degrees clockwise (90 degrees counter-clockwise)
    Clockwise270,
}

impl RotationAngle {
    /// Create from degrees
    pub fn from_degrees(degrees: i32) -> Result<Self, OperationError> {
        let normalized = degrees % 360;
        let normalized = if normalized < 0 {
            normalized + 360
        } else {
            normalized
        };

        match normalized {
            0 => Ok(RotationAngle::None),
            90 => Ok(RotationAngle::Clockwise90),
            180 => Ok(RotationAngle::Rotate180),
            270 => Ok(RotationAngle::Clockwise270),
            _ => Err(OperationError::InvalidRotation(degrees)),
        }
    }

    /// Convert to degrees
    pub fn to_degrees(self) -> i32 {
        match self {
            RotationAngle::None => 0,
            RotationAngle::Clockwise90 => 90,
            RotationAngle::Rotate180 => 180,
            RotationAngle::Clockwise270 => 270,
        }
    }

    /// Combine two rotations
    pub fn combine(self, other: RotationAngle) -> RotationAngle {
        let total = (self.to_degrees() + other.to_degrees()) % 360;
        RotationAngle::from_degrees(total).unwrap()
    }
}

/// Options for page rotation
#[derive(Debug, Clone)]
pub struct RotateOptions {
    /// Pages to rotate
    pub pages: PageRange,
    /// Rotation angle
    pub angle: RotationAngle,
    /// Whether to preserve the original page size (vs adjusting for rotated content)
    pub preserve_page_size: bool,
}

impl Default for RotateOptions {
    fn default() -> Self {
        Self {
            pages: PageRange::All,
            angle: RotationAngle::Clockwise90,
            preserve_page_size: false,
        }
    }
}

/// PDF page rotator
pub struct PageRotator {
    document: PdfDocument<File>,
}

impl PageRotator {
    /// Create a new page rotator
    pub fn new(document: PdfDocument<File>) -> Self {
        Self { document }
    }

    /// Rotate pages according to options
    pub fn rotate(&mut self, options: &RotateOptions) -> OperationResult<Document> {
        let total_pages =
            self.document
                .page_count()
                .map_err(|e| OperationError::ParseError(e.to_string()))? as usize;

        let page_indices = options.pages.get_indices(total_pages)?;
        let mut output_doc = Document::new();

        // Copy metadata
        if let Ok(metadata) = self.document.metadata() {
            if let Some(title) = metadata.title {
                output_doc.set_title(&title);
            }
            if let Some(author) = metadata.author {
                output_doc.set_author(&author);
            }
            if let Some(subject) = metadata.subject {
                output_doc.set_subject(&subject);
            }
            if let Some(keywords) = metadata.keywords {
                output_doc.set_keywords(&keywords);
            }
        }

        // Process each page
        for page_idx in 0..total_pages {
            let parsed_page = self
                .document
                .get_page(page_idx as u32)
                .map_err(|e| OperationError::ParseError(e.to_string()))?;

            let should_rotate = page_indices.contains(&page_idx);

            let page = if should_rotate {
                self.create_rotated_page(&parsed_page, options.angle, options.preserve_page_size)?
            } else {
                self.create_page_copy(&parsed_page)?
            };

            output_doc.add_page(page);
        }

        Ok(output_doc)
    }

    /// Create a rotated copy of a page
    fn create_rotated_page(
        &mut self,
        parsed_page: &ParsedPage,
        angle: RotationAngle,
        preserve_size: bool,
    ) -> OperationResult<Page> {
        // Calculate the effective rotation
        let current_rotation = parsed_page.rotation;
        let _new_rotation = (current_rotation + angle.to_degrees()) % 360;

        // Get original dimensions
        let orig_width = parsed_page.media_box[2] - parsed_page.media_box[0];
        let orig_height = parsed_page.media_box[3] - parsed_page.media_box[1];

        // Calculate new dimensions based on rotation
        let (new_width, new_height) = if preserve_size {
            (orig_width, orig_height)
        } else {
            match angle {
                RotationAngle::None | RotationAngle::Rotate180 => (orig_width, orig_height),
                RotationAngle::Clockwise90 | RotationAngle::Clockwise270 => {
                    (orig_height, orig_width)
                }
            }
        };

        let mut page = Page::new(new_width, new_height);

        // Get content streams
        let content_streams = self
            .document
            .get_page_content_streams(parsed_page)
            .map_err(|e| OperationError::ParseError(e.to_string()))?;

        // Add rotation transformation
        match angle {
            RotationAngle::None => {
                // No rotation needed
            }
            RotationAngle::Clockwise90 => {
                // Rotate 90 degrees clockwise
                // Transform: x' = y, y' = width - x
                page.graphics()
                    .save_state()
                    .transform(0.0, 1.0, -1.0, 0.0, new_width, 0.0);
            }
            RotationAngle::Rotate180 => {
                // Rotate 180 degrees
                // Transform: x' = width - x, y' = height - y
                page.graphics()
                    .save_state()
                    .transform(-1.0, 0.0, 0.0, -1.0, new_width, new_height);
            }
            RotationAngle::Clockwise270 => {
                // Rotate 270 degrees clockwise (90 counter-clockwise)
                // Transform: x' = height - y, y' = x
                page.graphics()
                    .save_state()
                    .transform(0.0, -1.0, 1.0, 0.0, 0.0, new_height);
            }
        }

        // Parse and process content streams with rotation
        let mut has_content = false;
        for stream_data in &content_streams {
            match ContentParser::parse_content(stream_data) {
                Ok(operators) => {
                    // Process the operators with rotation transformation already applied
                    self.process_operators_with_rotation(&mut page, &operators)?;
                    has_content = true;
                }
                Err(e) => {
                    eprintln!("Warning: Failed to parse content stream: {e}");
                }
            }
        }

        // If no content was successfully processed, add a placeholder
        if !has_content {
            page.text()
                .set_font(crate::text::Font::Helvetica, 10.0)
                .at(50.0, new_height - 50.0)
                .write(&format!(
                    "[Page rotated {} degrees - content reconstruction in progress]",
                    angle.to_degrees()
                ))
                .map_err(OperationError::PdfError)?;
        }

        // Restore graphics state if we transformed
        if angle != RotationAngle::None {
            page.graphics().restore_state();
        }

        Ok(page)
    }

    /// Create a copy of a page without rotation
    fn create_page_copy(&mut self, parsed_page: &ParsedPage) -> OperationResult<Page> {
        let width = parsed_page.width();
        let height = parsed_page.height();
        let mut page = Page::new(width, height);

        // Get content streams
        let content_streams = self
            .document
            .get_page_content_streams(parsed_page)
            .map_err(|e| OperationError::ParseError(e.to_string()))?;

        // Parse and process content streams
        let mut has_content = false;
        for stream_data in content_streams {
            match ContentParser::parse_content(&stream_data) {
                Ok(operators) => {
                    self.process_operators_with_rotation(&mut page, &operators)?;
                    has_content = true;
                }
                Err(e) => {
                    eprintln!("Warning: Failed to parse content stream: {e}");
                }
            }
        }

        // If no content was successfully processed, add a placeholder
        if !has_content {
            page.text()
                .set_font(crate::text::Font::Helvetica, 10.0)
                .at(50.0, height - 50.0)
                .write("[Page copied - content reconstruction in progress]")
                .map_err(OperationError::PdfError)?;
        }

        Ok(page)
    }

    /// Process content operators (rotation transformation already applied via graphics state)
    fn process_operators_with_rotation(
        &self,
        page: &mut Page,
        operators: &[ContentOperation],
    ) -> OperationResult<()> {
        // Track graphics state
        let mut text_object = false;
        let mut current_font = crate::text::Font::Helvetica;
        let mut current_font_size = 12.0;
        let mut current_x = 0.0;
        let mut current_y = 0.0;

        for operator in operators {
            match operator {
                ContentOperation::BeginText => {
                    text_object = true;
                }
                ContentOperation::EndText => {
                    text_object = false;
                }
                ContentOperation::SetFont(name, size) => {
                    // Map PDF font names to our fonts
                    current_font = match name.as_str() {
                        "Times-Roman" => crate::text::Font::TimesRoman,
                        "Times-Bold" => crate::text::Font::TimesBold,
                        "Times-Italic" => crate::text::Font::TimesItalic,
                        "Times-BoldItalic" => crate::text::Font::TimesBoldItalic,
                        "Helvetica-Bold" => crate::text::Font::HelveticaBold,
                        "Helvetica-Oblique" => crate::text::Font::HelveticaOblique,
                        "Helvetica-BoldOblique" => crate::text::Font::HelveticaBoldOblique,
                        "Courier" => crate::text::Font::Courier,
                        "Courier-Bold" => crate::text::Font::CourierBold,
                        "Courier-Oblique" => crate::text::Font::CourierOblique,
                        "Courier-BoldOblique" => crate::text::Font::CourierBoldOblique,
                        _ => crate::text::Font::Helvetica,
                    };
                    current_font_size = *size;
                }
                ContentOperation::MoveText(tx, ty) => {
                    current_x += tx;
                    current_y += ty;
                }
                ContentOperation::ShowText(text_bytes) => {
                    if text_object {
                        if let Ok(text) = String::from_utf8(text_bytes.clone()) {
                            page.text()
                                .set_font(current_font.clone(), current_font_size as f64)
                                .at(current_x as f64, current_y as f64)
                                .write(&text)
                                .map_err(OperationError::PdfError)?;
                        }
                    }
                }
                ContentOperation::Rectangle(x, y, width, height) => {
                    page.graphics()
                        .rect(*x as f64, *y as f64, *width as f64, *height as f64);
                }
                ContentOperation::MoveTo(x, y) => {
                    page.graphics().move_to(*x as f64, *y as f64);
                }
                ContentOperation::LineTo(x, y) => {
                    page.graphics().line_to(*x as f64, *y as f64);
                }
                ContentOperation::Stroke => {
                    page.graphics().stroke();
                }
                ContentOperation::Fill => {
                    page.graphics().fill();
                }
                ContentOperation::SetNonStrokingRGB(r, g, b) => {
                    page.graphics().set_fill_color(crate::graphics::Color::Rgb(
                        *r as f64, *g as f64, *b as f64,
                    ));
                }
                ContentOperation::SetStrokingRGB(r, g, b) => {
                    page.graphics()
                        .set_stroke_color(crate::graphics::Color::Rgb(
                            *r as f64, *g as f64, *b as f64,
                        ));
                }
                ContentOperation::SetLineWidth(width) => {
                    page.graphics().set_line_width(*width as f64);
                }
                // Graphics state operators are important for rotation
                ContentOperation::SaveGraphicsState => {
                    page.graphics().save_state();
                }
                ContentOperation::RestoreGraphicsState => {
                    page.graphics().restore_state();
                }
                ContentOperation::SetTransformMatrix(a, b, c, d, e, f) => {
                    page.graphics().transform(
                        *a as f64, *b as f64, *c as f64, *d as f64, *e as f64, *f as f64,
                    );
                }
                _ => {
                    // Silently skip unimplemented operators for now
                }
            }
        }

        Ok(())
    }
}

/// Rotate pages in a PDF file
pub fn rotate_pdf_pages<P: AsRef<Path>, Q: AsRef<Path>>(
    input_path: P,
    output_path: Q,
    options: RotateOptions,
) -> OperationResult<()> {
    let document = PdfReader::open_document(input_path)
        .map_err(|e| OperationError::ParseError(e.to_string()))?;

    let mut rotator = PageRotator::new(document);
    let mut doc = rotator.rotate(&options)?;

    doc.save(output_path)?;
    Ok(())
}

/// Rotate all pages in a PDF file
pub fn rotate_all_pages<P: AsRef<Path>, Q: AsRef<Path>>(
    input_path: P,
    output_path: Q,
    angle: RotationAngle,
) -> OperationResult<()> {
    let options = RotateOptions {
        pages: PageRange::All,
        angle,
        preserve_page_size: false,
    };

    rotate_pdf_pages(input_path, output_path, options)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rotation_angle() {
        assert_eq!(RotationAngle::from_degrees(0).unwrap(), RotationAngle::None);
        assert_eq!(
            RotationAngle::from_degrees(90).unwrap(),
            RotationAngle::Clockwise90
        );
        assert_eq!(
            RotationAngle::from_degrees(180).unwrap(),
            RotationAngle::Rotate180
        );
        assert_eq!(
            RotationAngle::from_degrees(270).unwrap(),
            RotationAngle::Clockwise270
        );

        // Test normalization
        assert_eq!(
            RotationAngle::from_degrees(360).unwrap(),
            RotationAngle::None
        );
        assert_eq!(
            RotationAngle::from_degrees(450).unwrap(),
            RotationAngle::Clockwise90
        );
        assert_eq!(
            RotationAngle::from_degrees(-90).unwrap(),
            RotationAngle::Clockwise270
        );

        // Test invalid angles
        assert!(RotationAngle::from_degrees(45).is_err());
        assert!(RotationAngle::from_degrees(135).is_err());
    }

    #[test]
    fn test_rotation_combine() {
        let r1 = RotationAngle::Clockwise90;
        let r2 = RotationAngle::Clockwise90;
        assert_eq!(r1.combine(r2), RotationAngle::Rotate180);

        let r3 = RotationAngle::Clockwise270;
        let r4 = RotationAngle::Clockwise90;
        assert_eq!(r3.combine(r4), RotationAngle::None);
    }

    // ============= Comprehensive Rotation Tests =============
    
    #[test]
    fn test_rotation_to_degrees() {
        assert_eq!(RotationAngle::None.to_degrees(), 0);
        assert_eq!(RotationAngle::Clockwise90.to_degrees(), 90);
        assert_eq!(RotationAngle::Rotate180.to_degrees(), 180);
        assert_eq!(RotationAngle::Clockwise270.to_degrees(), 270);
    }

    #[test]
    fn test_rotation_normalization_positive() {
        // Test positive angle normalization
        assert_eq!(RotationAngle::from_degrees(360).unwrap(), RotationAngle::None);
        assert_eq!(RotationAngle::from_degrees(450).unwrap(), RotationAngle::Clockwise90);
        assert_eq!(RotationAngle::from_degrees(540).unwrap(), RotationAngle::Rotate180);
        assert_eq!(RotationAngle::from_degrees(630).unwrap(), RotationAngle::Clockwise270);
        assert_eq!(RotationAngle::from_degrees(720).unwrap(), RotationAngle::None);
        assert_eq!(RotationAngle::from_degrees(810).unwrap(), RotationAngle::Clockwise90);
    }

    #[test]
    fn test_rotation_normalization_negative() {
        // Test negative angle normalization
        assert_eq!(RotationAngle::from_degrees(-90).unwrap(), RotationAngle::Clockwise270);
        assert_eq!(RotationAngle::from_degrees(-180).unwrap(), RotationAngle::Rotate180);
        assert_eq!(RotationAngle::from_degrees(-270).unwrap(), RotationAngle::Clockwise90);
        assert_eq!(RotationAngle::from_degrees(-360).unwrap(), RotationAngle::None);
        assert_eq!(RotationAngle::from_degrees(-450).unwrap(), RotationAngle::Clockwise270);
        assert_eq!(RotationAngle::from_degrees(-540).unwrap(), RotationAngle::Rotate180);
    }

    #[test]
    fn test_rotation_invalid_angles() {
        // Test all invalid angles that should return errors
        let invalid_angles = vec![
            1, 15, 30, 45, 60, 75, 89, 91,
            105, 120, 135, 150, 165, 179, 181,
            195, 210, 225, 240, 255, 269, 271,
            285, 300, 315, 330, 345, 359,
        ];
        
        for angle in invalid_angles {
            assert!(
                RotationAngle::from_degrees(angle).is_err(),
                "Angle {} should be invalid", angle
            );
            assert!(
                RotationAngle::from_degrees(-angle).is_err(),
                "Angle {} should be invalid", -angle
            );
        }
    }

    #[test]
    fn test_rotation_combine_all_combinations() {
        // Test all possible combinations of rotations
        let rotations = vec![
            RotationAngle::None,
            RotationAngle::Clockwise90,
            RotationAngle::Rotate180,
            RotationAngle::Clockwise270,
        ];
        
        // Expected results for each combination (row combines with column)
        let expected = vec![
            vec![RotationAngle::None, RotationAngle::Clockwise90, RotationAngle::Rotate180, RotationAngle::Clockwise270],
            vec![RotationAngle::Clockwise90, RotationAngle::Rotate180, RotationAngle::Clockwise270, RotationAngle::None],
            vec![RotationAngle::Rotate180, RotationAngle::Clockwise270, RotationAngle::None, RotationAngle::Clockwise90],
            vec![RotationAngle::Clockwise270, RotationAngle::None, RotationAngle::Clockwise90, RotationAngle::Rotate180],
        ];
        
        for (i, r1) in rotations.iter().enumerate() {
            for (j, r2) in rotations.iter().enumerate() {
                let result = r1.combine(*r2);
                assert_eq!(
                    result, expected[i][j],
                    "Combining {:?} with {:?} should give {:?}, got {:?}",
                    r1, r2, expected[i][j], result
                );
            }
        }
    }

    #[test]
    fn test_rotation_combine_chain() {
        // Test chaining multiple rotations
        let r1 = RotationAngle::Clockwise90;
        let r2 = RotationAngle::Clockwise90;
        let r3 = RotationAngle::Clockwise90;
        let r4 = RotationAngle::Clockwise90;
        
        let result = r1.combine(r2).combine(r3).combine(r4);
        assert_eq!(result, RotationAngle::None); // 4 * 90 = 360 = 0
        
        // Test another chain
        let result2 = RotationAngle::Clockwise270
            .combine(RotationAngle::Clockwise90)
            .combine(RotationAngle::Rotate180);
        assert_eq!(result2, RotationAngle::Rotate180); // 270 + 90 + 180 = 540 = 180
    }

    #[test]
    fn test_rotation_identity() {
        // Test that None is the identity element
        let rotations = vec![
            RotationAngle::None,
            RotationAngle::Clockwise90,
            RotationAngle::Rotate180,
            RotationAngle::Clockwise270,
        ];
        
        for rotation in rotations {
            assert_eq!(rotation.combine(RotationAngle::None), rotation);
            assert_eq!(RotationAngle::None.combine(rotation), rotation);
        }
    }

    #[test]
    fn test_rotation_inverse() {
        // Test that each rotation has an inverse
        assert_eq!(
            RotationAngle::Clockwise90.combine(RotationAngle::Clockwise270),
            RotationAngle::None
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
            RotationAngle::None.combine(RotationAngle::None),
            RotationAngle::None
        );
    }

    #[test]
    fn test_rotation_options_default() {
        let options = RotationOptions::default();
        assert!(matches!(options.angle, RotationAngle::Clockwise90));
        assert!(options.pages.is_none());
    }

    #[test]
    fn test_rotation_options_with_angle() {
        let options = RotationOptions {
            angle: RotationAngle::Rotate180,
            pages: Some(PageRange::Range(5, 10)),
        };
        
        assert_eq!(options.angle, RotationAngle::Rotate180);
        assert!(options.pages.is_some());
        
        if let Some(PageRange::Range(start, end)) = options.pages {
            assert_eq!(start, 5);
            assert_eq!(end, 10);
        } else {
            panic!("Expected Range page specification");
        }
    }

    #[test]
    fn test_rotation_options_all_pages() {
        let options = RotationOptions {
            angle: RotationAngle::Clockwise270,
            pages: Some(PageRange::All),
        };
        
        assert_eq!(options.angle, RotationAngle::Clockwise270);
        assert!(matches!(options.pages, Some(PageRange::All)));
    }

    #[test]
    fn test_rotation_options_single_page() {
        let options = RotationOptions {
            angle: RotationAngle::Clockwise90,
            pages: Some(PageRange::Single(0)),
        };
        
        assert_eq!(options.angle, RotationAngle::Clockwise90);
        
        if let Some(PageRange::Single(page)) = options.pages {
            assert_eq!(page, 0);
        } else {
            panic!("Expected Single page specification");
        }
    }

    #[test]
    fn test_rotation_options_page_list() {
        let pages = vec![1, 3, 5, 7, 9];
        let options = RotationOptions {
            angle: RotationAngle::Rotate180,
            pages: Some(PageRange::List(pages.clone())),
        };
        
        if let Some(PageRange::List(list)) = options.pages {
            assert_eq!(list, pages);
        } else {
            panic!("Expected List page specification");
        }
    }

    #[test]
    fn test_pdf_rotator_new() {
        // This test would need actual PDF document setup
        // For now, just test that the structure is correct
        let options = RotationOptions::default();
        assert_eq!(options.angle.to_degrees(), 90);
    }

    #[test]
    fn test_rotation_edge_cases() {
        // Test edge cases with large positive numbers
        assert_eq!(RotationAngle::from_degrees(1080).unwrap(), RotationAngle::None); // 3 * 360
        assert_eq!(RotationAngle::from_degrees(990).unwrap(), RotationAngle::Clockwise270); // 2*360 + 270
        
        // Test edge cases with large negative numbers
        assert_eq!(RotationAngle::from_degrees(-720).unwrap(), RotationAngle::None); // -2 * 360
        assert_eq!(RotationAngle::from_degrees(-810).unwrap(), RotationAngle::Clockwise270); // -2*360 - 90
    }

    #[test]
    fn test_rotation_associativity() {
        // Test that rotation combination is associative
        let r1 = RotationAngle::Clockwise90;
        let r2 = RotationAngle::Rotate180;
        let r3 = RotationAngle::Clockwise270;
        
        // (r1 + r2) + r3 should equal r1 + (r2 + r3)
        let left = r1.combine(r2).combine(r3);
        let right = r1.combine(r2.combine(r3));
        assert_eq!(left, right);
    }

    #[test]
    fn test_rotation_consistency() {
        // Test that from_degrees and to_degrees are consistent
        for angle in [0, 90, 180, 270].iter() {
            let rotation = RotationAngle::from_degrees(*angle).unwrap();
            assert_eq!(rotation.to_degrees(), *angle);
        }
    }

    #[test]
    fn test_rotation_multiple_full_rotations() {
        // Test multiple full rotations
        for multiplier in 1..5 {
            let angle = 360 * multiplier;
            assert_eq!(
                RotationAngle::from_degrees(angle).unwrap(),
                RotationAngle::None,
                "Angle {} should normalize to None", angle
            );
            assert_eq!(
                RotationAngle::from_degrees(-angle).unwrap(),
                RotationAngle::None,
                "Angle {} should normalize to None", -angle
            );
        }
    }
}

#[cfg(test)]
#[path = "rotate_tests.rs"]
mod rotate_tests;
