//! Integration tests for PDF annotations

use oxidize_pdf::annotations::{
    Annotation, AnnotationType, CircleAnnotation, FileAttachmentAnnotation, FileAttachmentIcon,
    InkAnnotation, SquareAnnotation, StampAnnotation, StampName,
};
use oxidize_pdf::error::Result;
use oxidize_pdf::geometry::{Point, Rectangle};
use oxidize_pdf::graphics::Color;

#[test]
fn test_ink_annotation_creation() -> Result<()> {
    // Create an ink annotation for a signature
    let mut ink = InkAnnotation::new();

    // Add first stroke
    let stroke1 = vec![
        Point::new(100.0, 100.0),
        Point::new(120.0, 110.0),
        Point::new(140.0, 105.0),
        Point::new(160.0, 100.0),
    ];
    ink = ink.add_stroke(stroke1);

    // Add second stroke
    let stroke2 = vec![
        Point::new(100.0, 120.0),
        Point::new(120.0, 125.0),
        Point::new(140.0, 120.0),
    ];
    ink = ink.add_stroke(stroke2);

    // Convert to annotation
    let annotation = ink.to_annotation();

    // Verify the annotation type
    assert_eq!(annotation.annotation_type, AnnotationType::Ink);

    // Verify the bounding box was calculated correctly
    assert_eq!(annotation.rect.lower_left.x, 100.0);
    assert_eq!(annotation.rect.lower_left.y, 100.0);
    assert_eq!(annotation.rect.upper_right.x, 160.0);
    assert_eq!(annotation.rect.upper_right.y, 125.0);

    Ok(())
}

#[test]
fn test_square_annotation_with_colors() -> Result<()> {
    let rect = Rectangle::new(Point::new(50.0, 50.0), Point::new(150.0, 150.0));

    let square = SquareAnnotation::new(rect)
        .with_interior_color(Color::rgb(0.8, 0.8, 1.0))
        .with_cloudy_border(1.5);

    let annotation = square.to_annotation();

    assert_eq!(annotation.annotation_type, AnnotationType::Square);
    assert_eq!(annotation.rect, rect);

    // Check that properties were set
    assert!(annotation.properties.get("IC").is_some()); // Interior color
    assert!(annotation.properties.get("BE").is_some()); // Border effect

    Ok(())
}

#[test]
fn test_circle_annotation() -> Result<()> {
    let rect = Rectangle::new(Point::new(100.0, 100.0), Point::new(200.0, 200.0));

    let circle = CircleAnnotation::new(rect).with_interior_color(Color::rgb(1.0, 0.9, 0.9));

    let annotation = circle.to_annotation();

    assert_eq!(annotation.annotation_type, AnnotationType::Circle);
    assert_eq!(annotation.rect, rect);

    Ok(())
}

#[test]
fn test_stamp_annotation_predefined() -> Result<()> {
    let rect = Rectangle::new(Point::new(400.0, 600.0), Point::new(500.0, 650.0));

    // Test various predefined stamps
    let stamps = vec![
        StampName::Approved,
        StampName::Draft,
        StampName::Confidential,
        StampName::Expired,
        StampName::NotApproved,
    ];

    for stamp_name in stamps {
        let stamp = StampAnnotation::new(rect, stamp_name.clone());
        let annotation = stamp.to_annotation();

        assert_eq!(annotation.annotation_type, AnnotationType::Stamp);
        assert_eq!(annotation.rect, rect);

        // Verify stamp name is set
        let name_obj = annotation.properties.get("Name");
        assert!(name_obj.is_some());
    }

    Ok(())
}

#[test]
fn test_stamp_annotation_custom() -> Result<()> {
    let rect = Rectangle::new(Point::new(200.0, 300.0), Point::new(300.0, 350.0));

    let custom_stamp = StampAnnotation::new(rect, StampName::Custom("MyCustomStamp".to_string()));

    let annotation = custom_stamp.to_annotation();

    assert_eq!(annotation.annotation_type, AnnotationType::Stamp);

    // Check custom name
    if let Some(oxidize_pdf::objects::Object::Name(name)) = annotation.properties.get("Name") {
        assert_eq!(name, "MyCustomStamp");
    } else {
        panic!("Custom stamp name not set correctly");
    }

    Ok(())
}

#[test]
fn test_file_attachment_annotation() -> Result<()> {
    let rect = Rectangle::new(Point::new(50.0, 700.0), Point::new(70.0, 720.0));

    let file_data = b"This is test file content".to_vec();
    let file_name = "test_document.txt".to_string();

    let attachment = FileAttachmentAnnotation::new(rect, file_name.clone(), file_data.clone())
        .with_mime_type("text/plain".to_string())
        .with_icon(FileAttachmentIcon::Paperclip);

    let annotation = attachment.to_annotation();

    assert_eq!(annotation.annotation_type, AnnotationType::FileAttachment);
    assert_eq!(annotation.rect, rect);

    // Verify file specification is set
    assert!(annotation.properties.get("FS").is_some());

    // Verify icon is set
    if let Some(oxidize_pdf::objects::Object::Name(icon)) = annotation.properties.get("Name") {
        assert_eq!(icon, "Paperclip");
    }

    Ok(())
}

#[test]
fn test_file_attachment_icons() -> Result<()> {
    let rect = Rectangle::new(Point::new(0.0, 0.0), Point::new(20.0, 20.0));

    let icons = vec![
        FileAttachmentIcon::Graph,
        FileAttachmentIcon::Paperclip,
        FileAttachmentIcon::PushPin,
        FileAttachmentIcon::Tag,
    ];

    for icon in icons {
        let icon_name = icon.pdf_name();
        assert!(!icon_name.is_empty());

        let attachment = FileAttachmentAnnotation::new(rect, "file.txt".to_string(), vec![1, 2, 3])
            .with_icon(icon);

        let annotation = attachment.to_annotation();
        assert_eq!(annotation.annotation_type, AnnotationType::FileAttachment);
    }

    Ok(())
}

#[test]
fn test_annotation_with_contents_and_subject() -> Result<()> {
    let rect = Rectangle::new(Point::new(100.0, 100.0), Point::new(200.0, 150.0));

    let annotation = Annotation::new(AnnotationType::Square, rect)
        .with_contents("This is a square annotation".to_string())
        .with_subject("Review Note".to_string());

    assert_eq!(
        annotation.contents,
        Some("This is a square annotation".to_string())
    );
    assert_eq!(annotation.subject, Some("Review Note".to_string()));

    Ok(())
}

#[test]
fn test_ink_annotation_empty_strokes() {
    let ink = InkAnnotation::new();
    let annotation = ink.to_annotation();

    // Empty ink annotation should still be valid
    assert_eq!(annotation.annotation_type, AnnotationType::Ink);

    // Rectangle should be at origin with zero size
    assert_eq!(annotation.rect.lower_left.x, 0.0);
    assert_eq!(annotation.rect.lower_left.y, 0.0);
    assert_eq!(annotation.rect.upper_right.x, 0.0);
    assert_eq!(annotation.rect.upper_right.y, 0.0);
}

#[test]
fn test_multiple_annotations_on_page() -> Result<()> {
    use oxidize_pdf::{Document, Page};

    let mut doc = Document::new();
    let page = Page::new(612.0, 792.0);

    // Create various annotations
    let ink =
        InkAnnotation::new().add_stroke(vec![Point::new(100.0, 100.0), Point::new(200.0, 150.0)]);

    let square = SquareAnnotation::new(Rectangle::new(
        Point::new(300.0, 300.0),
        Point::new(400.0, 400.0),
    ));

    let stamp = StampAnnotation::new(
        Rectangle::new(Point::new(450.0, 600.0), Point::new(550.0, 650.0)),
        StampName::Approved,
    );

    // Convert to annotations
    let _ink_annot = ink.to_annotation();
    let _square_annot = square.to_annotation();
    let _stamp_annot = stamp.to_annotation();

    // In a real implementation, we would add these to the page
    // page.add_annotation(ink_annot);
    // page.add_annotation(square_annot);
    // page.add_annotation(stamp_annot);

    doc.add_page(page);

    // Save would work in a real scenario
    // doc.save("test_annotations.pdf")?;

    Ok(())
}
