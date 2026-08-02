//! PDF page rotation functionality
//!
//! This module provides functionality to rotate pages in PDF documents.

use super::{OperationError, OperationResult, PageRange};
use crate::parser::page_tree::ParsedPage;
use crate::parser::{PdfDocument, PdfReader};
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
        // SAFETY: The modulo 360 operation guarantees only valid angles (0, 90, 180, 270)
        // since all RotationAngle variants are multiples of 90 degrees
        match total {
            0 => RotationAngle::None,
            90 => RotationAngle::Clockwise90,
            180 => RotationAngle::Rotate180,
            270 => RotationAngle::Clockwise270,
            _ => unreachable!("Modulo 360 of multiples of 90 can only be 0, 90, 180, or 270"),
        }
    }
}

/// Options for page rotation
#[derive(Debug, Clone)]
pub struct RotateOptions {
    /// Pages to rotate
    pub pages: PageRange,
    /// Rotation angle
    pub angle: RotationAngle,
    /// No longer has any effect (since the switch to the native `/Rotate`
    /// attribute in #453). Rotation is now a viewing transform: the stored
    /// MediaBox never changes under `/Rotate` (ISO 32000-1 §7.7.3.3), so there
    /// is no page-size adjustment to opt out of. Retained for API
    /// compatibility; a candidate for removal in the next major release.
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

    /// Create a rotated copy of a page using the native `/Rotate` attribute.
    ///
    /// Rotation in PDF is a viewing transform, not a content edit: the page's
    /// `/Rotate` entry (a multiple of 90) tells the viewer how to display the
    /// unchanged content and MediaBox (ISO 32000-1 §7.7.3.3). So the content is
    /// copied verbatim via [`Page::from_parsed_with_content`] and the new angle
    /// is composed onto whatever rotation the source already had.
    ///
    /// The former implementation baked a rotation matrix into a *reconstructed*
    /// content stream, which lost images and mangled text (#453) and, unlike
    /// `/Rotate`, could not be undone losslessly. `preserve_page_size` no longer
    /// has any effect: `/Rotate` never changes the stored page size.
    fn create_rotated_page(
        &mut self,
        parsed_page: &ParsedPage,
        angle: RotationAngle,
        _preserve_page_size: bool,
    ) -> OperationResult<Page> {
        let mut page = Page::from_parsed_with_content(parsed_page, &self.document)
            .map_err(|e| OperationError::ParseError(e.to_string()))?;
        let combined = (parsed_page.rotation + angle.to_degrees()).rem_euclid(360);
        page.set_rotation(combined);
        Ok(page)
    }

    /// Copy a page unchanged, content and existing `/Rotate` preserved verbatim.
    fn create_page_copy(&mut self, parsed_page: &ParsedPage) -> OperationResult<Page> {
        Page::from_parsed_with_content(parsed_page, &self.document)
            .map_err(|e| OperationError::ParseError(e.to_string()))
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
        assert_eq!(
            RotationAngle::from_degrees(360).unwrap(),
            RotationAngle::None
        );
        assert_eq!(
            RotationAngle::from_degrees(450).unwrap(),
            RotationAngle::Clockwise90
        );
        assert_eq!(
            RotationAngle::from_degrees(540).unwrap(),
            RotationAngle::Rotate180
        );
        assert_eq!(
            RotationAngle::from_degrees(630).unwrap(),
            RotationAngle::Clockwise270
        );
        assert_eq!(
            RotationAngle::from_degrees(720).unwrap(),
            RotationAngle::None
        );
        assert_eq!(
            RotationAngle::from_degrees(810).unwrap(),
            RotationAngle::Clockwise90
        );
    }

    #[test]
    fn test_rotation_normalization_negative() {
        // Test negative angle normalization
        assert_eq!(
            RotationAngle::from_degrees(-90).unwrap(),
            RotationAngle::Clockwise270
        );
        assert_eq!(
            RotationAngle::from_degrees(-180).unwrap(),
            RotationAngle::Rotate180
        );
        assert_eq!(
            RotationAngle::from_degrees(-270).unwrap(),
            RotationAngle::Clockwise90
        );
        assert_eq!(
            RotationAngle::from_degrees(-360).unwrap(),
            RotationAngle::None
        );
        assert_eq!(
            RotationAngle::from_degrees(-450).unwrap(),
            RotationAngle::Clockwise270
        );
        assert_eq!(
            RotationAngle::from_degrees(-540).unwrap(),
            RotationAngle::Rotate180
        );
    }

    #[test]
    fn test_rotation_invalid_angles() {
        // Test all invalid angles that should return errors
        let invalid_angles = vec![
            1, 15, 30, 45, 60, 75, 89, 91, 105, 120, 135, 150, 165, 179, 181, 195, 210, 225, 240,
            255, 269, 271, 285, 300, 315, 330, 345, 359,
        ];

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
            vec![
                RotationAngle::None,
                RotationAngle::Clockwise90,
                RotationAngle::Rotate180,
                RotationAngle::Clockwise270,
            ],
            vec![
                RotationAngle::Clockwise90,
                RotationAngle::Rotate180,
                RotationAngle::Clockwise270,
                RotationAngle::None,
            ],
            vec![
                RotationAngle::Rotate180,
                RotationAngle::Clockwise270,
                RotationAngle::None,
                RotationAngle::Clockwise90,
            ],
            vec![
                RotationAngle::Clockwise270,
                RotationAngle::None,
                RotationAngle::Clockwise90,
                RotationAngle::Rotate180,
            ],
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
        let options = RotateOptions::default();
        assert!(matches!(options.angle, RotationAngle::Clockwise90));
        assert!(matches!(options.pages, PageRange::All));
        assert!(!options.preserve_page_size);
    }

    #[test]
    fn test_rotation_options_with_angle() {
        let options = RotateOptions {
            angle: RotationAngle::Rotate180,
            pages: PageRange::Range(5, 10),
            preserve_page_size: false,
        };

        assert_eq!(options.angle, RotationAngle::Rotate180);

        if let PageRange::Range(start, end) = options.pages {
            assert_eq!(start, 5);
            assert_eq!(end, 10);
        } else {
            panic!("Expected Range page specification");
        }
    }

    #[test]
    fn test_rotation_options_all_pages() {
        let options = RotateOptions {
            angle: RotationAngle::Clockwise270,
            pages: PageRange::All,
            preserve_page_size: true,
        };

        assert_eq!(options.angle, RotationAngle::Clockwise270);
        assert!(matches!(options.pages, PageRange::All));
        assert!(options.preserve_page_size);
    }

    #[test]
    fn test_rotation_options_single_page() {
        let options = RotateOptions {
            angle: RotationAngle::Clockwise90,
            pages: PageRange::Single(0),
            preserve_page_size: false,
        };

        assert_eq!(options.angle, RotationAngle::Clockwise90);

        if let PageRange::Single(page) = options.pages {
            assert_eq!(page, 0);
        } else {
            panic!("Expected Single page specification");
        }
    }

    #[test]
    fn test_rotation_options_page_list() {
        let pages = vec![1, 3, 5, 7, 9];
        let options = RotateOptions {
            angle: RotationAngle::Rotate180,
            pages: PageRange::List(pages.clone()),
            preserve_page_size: false,
        };

        if let PageRange::List(list) = options.pages {
            assert_eq!(list, pages);
        } else {
            panic!("Expected List page specification");
        }
    }

    #[test]
    fn test_pdf_rotator_new() {
        // This test would need actual PDF document setup
        // For now, just test that the structure is correct
        let options = RotateOptions::default();
        assert_eq!(options.angle.to_degrees(), 90);
    }

    #[test]
    fn test_rotation_edge_cases() {
        // Test edge cases with large positive numbers
        assert_eq!(
            RotationAngle::from_degrees(1080).unwrap(),
            RotationAngle::None
        ); // 3 * 360
        assert_eq!(
            RotationAngle::from_degrees(990).unwrap(),
            RotationAngle::Clockwise270
        ); // 2*360 + 270

        // Test edge cases with large negative numbers
        assert_eq!(
            RotationAngle::from_degrees(-720).unwrap(),
            RotationAngle::None
        ); // -2 * 360
        assert_eq!(
            RotationAngle::from_degrees(-810).unwrap(),
            RotationAngle::Clockwise270
        ); // -2*360 - 90
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
                "Angle {} should normalize to None",
                angle
            );
            assert_eq!(
                RotationAngle::from_degrees(-angle).unwrap(),
                RotationAngle::None,
                "Angle {} should normalize to None",
                -angle
            );
        }
    }

    #[test]
    fn test_rotation_large_negative_angles() {
        // Test large negative angles (line 29-32)
        assert_eq!(
            RotationAngle::from_degrees(-720).unwrap(),
            RotationAngle::None
        );
        assert_eq!(
            RotationAngle::from_degrees(-1080).unwrap(),
            RotationAngle::None
        );
        assert_eq!(
            RotationAngle::from_degrees(-450).unwrap(),
            RotationAngle::Clockwise270
        );
        assert_eq!(
            RotationAngle::from_degrees(-630).unwrap(),
            RotationAngle::Clockwise90
        );
    }

    #[test]
    fn test_rotation_combine_overflow() {
        // Test combine that results in > 360 (line 56-57)
        let angle1 = RotationAngle::Clockwise270;
        let angle2 = RotationAngle::Rotate180;
        let combined = angle1.combine(angle2);
        assert_eq!(combined, RotationAngle::Clockwise90); // 270 + 180 = 450 % 360 = 90

        // Test multiple combines
        let angle3 = RotationAngle::Clockwise270;
        let result = angle1.combine(angle2).combine(angle3);
        assert_eq!(result, RotationAngle::None); // 90 + 270 = 360 % 360 = 0
    }

    #[test]
    fn test_rotation_extreme_values() {
        // Test with extreme i32 values (edge case for line 28-33)
        // These should not panic but might give unexpected results due to overflow
        let large_positive = 2147483647; // i32::MAX
        let result = RotationAngle::from_degrees(large_positive);
        assert!(result.is_err() || result.is_ok()); // Just ensure no panic

        let large_negative = -2147483648; // i32::MIN
        let result2 = RotationAngle::from_degrees(large_negative);
        assert!(result2.is_err() || result2.is_ok()); // Just ensure no panic

        // Test reasonable but large values
        assert_eq!(
            RotationAngle::from_degrees(3690).unwrap(),
            RotationAngle::Clockwise90 // 3690 % 360 = 90
        );
    }

    #[test]
    fn test_rotation_combine_unwrap_safety() {
        // Test that combine's unwrap is safe (line 57)
        // All valid combinations should produce valid results
        let angles = vec![
            RotationAngle::None,
            RotationAngle::Clockwise90,
            RotationAngle::Rotate180,
            RotationAngle::Clockwise270,
        ];

        for angle1 in &angles {
            for angle2 in &angles {
                // This should never panic
                let combined = angle1.combine(*angle2);

                // Verify the result is valid
                let total_degrees = (angle1.to_degrees() + angle2.to_degrees()) % 360;
                assert_eq!(combined.to_degrees(), total_degrees);
            }
        }
    }
}

#[cfg(test)]
#[path = "rotate_tests.rs"]
mod rotate_tests;
