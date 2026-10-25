//! Follow painted image instances and retain their PDF transforms for OCR.
use crate::coordinate_system::TransformMatrix;
use crate::error::{PdfError, Result};
use crate::geometry::Point;
use crate::parser::content::{ContentOperation, ContentParser};
use crate::parser::objects::{PdfDictionary, PdfObject};
use crate::parser::PdfDocument;
use std::fs::File;

pub(crate) struct PlacedOcrImage {
    pub reference: PdfObject,
    pub matrix: TransformMatrix,
}

pub(crate) fn collect(
    document: &PdfDocument<File>,
    page_index: u32,
) -> Result<Vec<PlacedOcrImage>> {
    let page = document.get_page(page_index)?;
    let resources = document
        .get_page_resources(&page)?
        .cloned()
        .unwrap_or_default();
    let streams = document.get_page_content_streams(&page)?;
    let content = ContentParser::combine_streams_owned(streams);
    let mut traversal = Traversal {
        document,
        images: Vec::new(),
        remaining: 100_000,
    };
    traversal.visit(
        &content,
        &resources,
        TransformMatrix::IDENTITY,
        page.crop_box.unwrap_or(page.media_box),
        0,
    )?;
    Ok(traversal.images)
}

struct Traversal<'a> {
    document: &'a PdfDocument<File>,
    images: Vec<PlacedOcrImage>,
    remaining: usize,
}

impl Traversal<'_> {
    fn visit(
        &mut self,
        content: &[u8],
        resources: &PdfDictionary,
        mut matrix: TransformMatrix,
        mut clip: [f64; 4],
        depth: usize,
    ) -> Result<()> {
        let document = self.document;
        if depth > 32 {
            return Err(invalid("OCR Form nesting exceeds 32 levels"));
        }
        if self.remaining == 0 {
            return Err(invalid("OCR content traversal budget exceeded"));
        }
        self.remaining -= 1;
        let mut stack = Vec::new();
        let mut rectangle = None;
        let mut complex_path = false;
        for op in ContentParser::parse_strict(content)? {
            if self.remaining == 0 {
                return Err(invalid("OCR content traversal budget exceeded"));
            }
            self.remaining -= 1;
            match op {
                ContentOperation::SaveGraphicsState => stack.push((matrix, clip)),
                ContentOperation::RestoreGraphicsState => {
                    (matrix, clip) = stack
                        .pop()
                        .ok_or_else(|| invalid("Unbalanced OCR graphics state"))?
                }
                ContentOperation::SetTransformMatrix(a, b, c, d, e, f) => {
                    matrix = matrix.multiply(&TransformMatrix::new(
                        a as f64, b as f64, c as f64, d as f64, e as f64, f as f64,
                    ));
                }
                ContentOperation::Rectangle(x, y, w, h) => {
                    if rectangle.is_some() {
                        complex_path = true;
                    }
                    rectangle =
                        Some(([x as f64, y as f64, (x + w) as f64, (y + h) as f64], matrix));
                }
                ContentOperation::MoveTo(..)
                | ContentOperation::LineTo(..)
                | ContentOperation::CurveTo(..)
                | ContentOperation::CurveToV(..)
                | ContentOperation::CurveToY(..) => complex_path = true,
                ContentOperation::Clip | ContentOperation::ClipEvenOdd => {
                    if complex_path {
                        return Err(invalid(
                            "OCR image conversion requires rendering for non-rectangular clips",
                        ));
                    }
                    let (bounds, transform) =
                        rectangle.ok_or_else(|| invalid("Missing OCR clipping path"))?;
                    clip = intersect(clip, transformed_bounds(bounds, transform)?);
                }
                ContentOperation::EndPath
                | ContentOperation::Stroke
                | ContentOperation::CloseStroke
                | ContentOperation::Fill
                | ContentOperation::FillEvenOdd
                | ContentOperation::FillStroke
                | ContentOperation::FillStrokeEvenOdd
                | ContentOperation::CloseFillStroke
                | ContentOperation::CloseFillStrokeEvenOdd => {
                    rectangle = None;
                    complex_path = false;
                }
                ContentOperation::SetGraphicsStateParams(_) => {
                    return Err(invalid(
                        "OCR image conversion requires rendering for external graphics states",
                    ))
                }
                ContentOperation::InlineImage { .. } => {
                    return Err(invalid(
                        "OCR image conversion does not yet support inline images",
                    ))
                }
                ContentOperation::PaintXObject(name) => {
                    let objects = resources
                        .get("XObject")
                        .ok_or_else(|| invalid("Missing OCR XObject resources"))?;
                    let objects = document.resolve(objects)?;
                    let object = objects
                        .as_dict()
                        .and_then(|dict| dict.get(&name))
                        .ok_or_else(|| invalid("Missing painted OCR XObject"))?;
                    let reference = object.clone();
                    let object = document.resolve(object)?;
                    let stream = object
                        .as_stream()
                        .ok_or_else(|| invalid("Painted OCR XObject is not a stream"))?;
                    match stream
                        .dict
                        .get("Subtype")
                        .and_then(PdfObject::as_name)
                        .map(|name| name.0.as_str())
                    {
                        Some("Image") => {
                            if self.images.len() >= 1024 {
                                return Err(invalid("OCR page exceeds 1024 image instances"));
                            }
                            if stream.dict.get("SMask").is_some()
                                || stream.dict.get("Mask").is_some()
                                || stream.dict.get("ImageMask").and_then(PdfObject::as_bool)
                                    == Some(true)
                            {
                                return Err(invalid(
                                    "OCR image conversion requires rendering for masked images",
                                ));
                            }
                            let bounds = transformed_bounds([0., 0., 1., 1.], matrix)?;
                            if bounds[0] < clip[0] - 1e-6
                                || bounds[1] < clip[1] - 1e-6
                                || bounds[2] > clip[2] + 1e-6
                                || bounds[3] > clip[3] + 1e-6
                            {
                                return Err(invalid("OCR image conversion requires rendering for partially clipped images"));
                            }
                            self.images.push(PlacedOcrImage { reference, matrix });
                        }
                        Some("Form") => {
                            let form_matrix = match stream.dict.get("Matrix") {
                                None => TransformMatrix::IDENTITY,
                                Some(value) => {
                                    let array = value
                                        .as_array()
                                        .filter(|a| a.len() == 6)
                                        .ok_or_else(|| invalid("Invalid OCR Form matrix"))?;
                                    let mut values = [0.; 6];
                                    for (i, v) in array.0.iter().enumerate() {
                                        values[i] = v.as_real().ok_or_else(|| {
                                            invalid("Invalid OCR Form matrix value")
                                        })?;
                                    }
                                    TransformMatrix::new(
                                        values[0], values[1], values[2], values[3], values[4],
                                        values[5],
                                    )
                                }
                            };
                            let resolved = stream
                                .dict
                                .get("Resources")
                                .map(|r| document.resolve(r))
                                .transpose()?;
                            let form_resources = resolved
                                .as_ref()
                                .map(|r| {
                                    r.as_dict()
                                        .ok_or_else(|| invalid("Invalid OCR Form resources"))
                                })
                                .transpose()?
                                .unwrap_or(resources);
                            let matrix = matrix.multiply(&form_matrix);
                            let bbox = stream
                                .dict
                                .get("BBox")
                                .and_then(PdfObject::as_array)
                                .filter(|a| a.len() == 4)
                                .ok_or_else(|| invalid("Missing OCR Form BBox"))?;
                            let mut bounds = [0.; 4];
                            for (i, value) in bbox.0.iter().enumerate() {
                                bounds[i] = value
                                    .as_real()
                                    .ok_or_else(|| invalid("Invalid OCR Form BBox"))?;
                            }
                            let clip = intersect(clip, transformed_bounds(bounds, matrix)?);
                            self.visit(
                                &stream.decode(&document.options())?,
                                form_resources,
                                matrix,
                                clip,
                                depth + 1,
                            )?;
                        }
                        _ => return Err(invalid("Unsupported painted OCR XObject")),
                    }
                }
                _ => {}
            }
        }
        if !stack.is_empty() {
            return Err(invalid("Unbalanced OCR graphics state"));
        }
        Ok(())
    }
}

fn intersect(a: [f64; 4], b: [f64; 4]) -> [f64; 4] {
    [
        a[0].max(b[0]),
        a[1].max(b[1]),
        a[2].min(b[2]),
        a[3].min(b[3]),
    ]
}

fn transformed_bounds(bounds: [f64; 4], matrix: TransformMatrix) -> Result<[f64; 4]> {
    if !((matrix.b == 0. && matrix.c == 0.) || (matrix.a == 0. && matrix.d == 0.)) {
        return Err(invalid(
            "OCR image conversion requires rendering for non-axis-aligned clips",
        ));
    }
    let [x, y, right, top] = bounds;
    let points = [(x, y), (right, y), (x, top), (right, top)]
        .map(|(x, y)| matrix.transform_point(Point::new(x, y)));
    if points.iter().any(|p| !p.x.is_finite() || !p.y.is_finite()) {
        return Err(invalid("Non-finite OCR bounds"));
    }
    Ok([
        points.iter().map(|p| p.x).fold(f64::INFINITY, f64::min),
        points.iter().map(|p| p.y).fold(f64::INFINITY, f64::min),
        points.iter().map(|p| p.x).fold(f64::NEG_INFINITY, f64::max),
        points.iter().map(|p| p.y).fold(f64::NEG_INFINITY, f64::max),
    ])
}

pub(crate) fn orient_image(
    data: Vec<u8>,
    matrix: TransformMatrix,
    page_rotation: i32,
) -> Result<(Vec<u8>, TransformMatrix)> {
    // Validate orientation before invoking OCR, including blank-image results.
    map_region([0., 0., 1., 1.], (1, 1), matrix)?;
    let angle = matrix.b.atan2(matrix.a).to_degrees().round() as i32;
    let rotation = (page_rotation - angle).rem_euclid(360);
    if rotation == 0 {
        return Ok((data, matrix));
    }
    #[cfg(feature = "external-images")]
    {
        let image = image::load_from_memory(&data)
            .map_err(|e| invalid(&format!("Decode OCR image: {e}")))?;
        let (image, inverse) = match rotation {
            90 => (image.rotate90(), [0., 1., -1., 0., 1., 0.]),
            180 => (image.rotate180(), [-1., 0., 0., -1., 1., 1.]),
            270 => (image.rotate270(), [0., -1., 1., 0., 0., 1.]),
            _ => {
                return Err(invalid(
                    "OCR page rotation must be a multiple of 90 degrees",
                ))
            }
        };
        let mut output = std::io::Cursor::new(Vec::new());
        image
            .write_to(&mut output, image::ImageFormat::Png)
            .map_err(|e| invalid(&format!("Encode OCR image: {e}")))?;
        Ok((
            output.into_inner(),
            matrix.multiply(&TransformMatrix::new(
                inverse[0], inverse[1], inverse[2], inverse[3], inverse[4], inverse[5],
            )),
        ))
    }
    #[cfg(not(feature = "external-images"))]
    Err(invalid(
        "Rotated OCR image conversion requires external-images",
    ))
}

/// Convert an image-canvas rectangle (origin at bottom left, one unit per pixel)
/// to the unrotated page rectangle consumed by the incremental layer editor.
pub(crate) fn map_region(
    region: [f64; 4],
    dimensions: (u32, u32),
    matrix: TransformMatrix,
) -> Result<[f64; 4]> {
    if dimensions.0 == 0 || dimensions.1 == 0 {
        return Err(invalid("OCR result has no image dimensions"));
    }
    let [x, y, w, h] = region;
    if region.iter().any(|n| !n.is_finite())
        || x < 0.
        || y < 0.
        || w <= 0.
        || h <= 0.
        || x + w > f64::from(dimensions.0)
        || y + h > f64::from(dimensions.1)
    {
        return Err(invalid("OCR region is outside its source image"));
    }
    let TransformMatrix { a, b, c, d, e, f } = matrix;
    if [a, b, c, d, e, f].iter().any(|v| !v.is_finite())
        || a * d - b * c <= 0.
        || !((b == 0. && c == 0.) || (a == 0. && d == 0.))
    {
        return Err(invalid(
            "OCR conversion requires an orthogonal, non-reflected image placement",
        ));
    }
    let corners = [(x, y), (x + w, y), (x, y + h), (x + w, y + h)].map(|(px, py)| {
        let u = px / f64::from(dimensions.0);
        let v = py / f64::from(dimensions.1);
        let point = matrix.transform_point(Point::new(u, v));
        (point.x, point.y)
    });
    let left = corners.iter().map(|p| p.0).fold(f64::INFINITY, f64::min);
    let bottom = corners.iter().map(|p| p.1).fold(f64::INFINITY, f64::min);
    let right = corners
        .iter()
        .map(|p| p.0)
        .fold(f64::NEG_INFINITY, f64::max);
    let top = corners
        .iter()
        .map(|p| p.1)
        .fold(f64::NEG_INFINITY, f64::max);
    Ok([left, bottom, right - left, top - bottom])
}

fn invalid(message: &str) -> PdfError {
    PdfError::InvalidStructure(message.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stream(dictionary: &str, data: &[u8]) -> Vec<u8> {
        let mut bytes = format!("<< {dictionary} /Length {} >>\nstream\n", data.len()).into_bytes();
        bytes.extend(data);
        bytes.extend(b"\nendstream");
        bytes
    }

    fn document(content: &str, extra: Vec<Vec<u8>>) -> (tempfile::TempDir, PdfDocument<File>) {
        let mut objects=vec![
            b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
            b"<< /Type /Pages /Count 1 /Kids [3 0 R] /MediaBox [0 0 100 100] >>".to_vec(),
            b"<< /Type /Page /Parent 2 0 R /Resources << /XObject << /Image 5 0 R /Form 6 0 R >> >> /Contents 4 0 R >>".to_vec(),
            stream("",content.as_bytes()),
            stream("/Type /XObject /Subtype /Image /Width 1 /Height 1 /ColorSpace /DeviceGray /BitsPerComponent 8",&[0]),
        ];
        objects.extend(extra);
        let mut bytes = b"%PDF-1.7\n".to_vec();
        let mut offsets = Vec::new();
        for (i, obj) in objects.iter().enumerate() {
            offsets.push(bytes.len());
            bytes.extend(format!("{} 0 obj\n", i + 1).as_bytes());
            bytes.extend(obj);
            bytes.extend(b"\nendobj\n");
        }
        let start = bytes.len();
        bytes.extend(format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).as_bytes());
        for offset in offsets {
            bytes.extend(format!("{offset:010} 00000 n \n").as_bytes());
        }
        bytes.extend(
            format!(
                "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{start}\n%%EOF\n",
                objects.len() + 1
            )
            .as_bytes(),
        );
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("input.pdf");
        std::fs::write(&path, bytes).unwrap();
        let doc =
            PdfDocument::new(crate::parser::PdfReader::new(File::open(path).unwrap()).unwrap());
        (dir, doc)
    }

    #[test]
    fn image_repetition_keeps_references_and_graphics_state() {
        let (_dir, doc) = document("q 10 0 0 10 20 30 cm /Image Do Q /Image Do", vec![]);
        let images = collect(&doc, 0).unwrap();
        assert_eq!(images.len(), 2);
        assert!(images
            .iter()
            .all(|image| matches!(image.reference, PdfObject::Reference(5, 0))));
        assert_eq!(
            map_region([0., 0., 1., 1.], (1, 1), images[0].matrix).unwrap(),
            [20., 30., 10., 10.]
        );
        assert_eq!(images[1].matrix, TransformMatrix::IDENTITY);
    }

    #[test]
    fn rectangular_clip_accepts_contained_images_but_rejects_hidden_pixels() {
        let (_dir, doc) = document("0 0 20 20 re W n 10 0 0 10 0 0 cm /Image Do", vec![]);
        assert_eq!(collect(&doc, 0).unwrap().len(), 1);
        let (_dir, doc) = document("0 0 5 5 re W n 10 0 0 10 0 0 cm /Image Do", vec![]);
        assert!(collect(&doc, 0)
            .err()
            .unwrap()
            .to_string()
            .contains("partially clipped"));
    }

    #[test]
    fn painted_paths_do_not_contaminate_later_rectangular_clips() {
        for paint in ["S", "s", "f", "f*", "B", "B*", "b", "b*"] {
            let (_dir, doc) = document(
                &format!("0 0 10 10 re {paint} 0 0 100 100 re W n /Image Do"),
                vec![],
            );
            assert_eq!(collect(&doc, 0).unwrap().len(), 1, "{paint}");
        }
    }

    #[test]
    fn non_clipping_background_paths_do_not_restrict_image_placement() {
        for background in [
            "20 20 -10 10 re f",
            "q 0.707 0.707 -0.707 0.707 20 20 cm 0 0 10 10 re f Q",
            "0 0 m 5 5 l S",
        ] {
            let (_dir, doc) = document(
                &format!("{background} 0 0 100 100 re W n /Image Do"),
                vec![],
            );
            assert_eq!(collect(&doc, 0).unwrap().len(), 1);
        }
        let (_dir, doc) = document("100 100 -100 -100 re W n /Image Do", vec![]);
        assert_eq!(collect(&doc, 0).unwrap().len(), 1);
    }

    #[test]
    fn form_bbox_limits_image_visibility() {
        for (bbox, accepted) in [("0 0 20 20", true), ("0 0 5 5", false)] {
            let form=stream(&format!("/Type /XObject /Subtype /Form /BBox [{bbox}] /Resources << /XObject << /Image 5 0 R >> >>"),b"10 0 0 10 0 0 cm /Image Do");
            let (_dir, doc) = document("/Form Do", vec![form]);
            assert_eq!(collect(&doc, 0).is_ok(), accepted);
        }
    }

    #[test]
    fn repeated_form_dag_has_a_cumulative_work_limit() {
        let mut forms = Vec::new();
        for level in 0..18 {
            let content = if level == 17 { "" } else { "/Next Do /Next Do" };
            forms.push(stream(&format!("/Type /XObject /Subtype /Form /BBox [0 0 100 100] /Resources << /XObject << /Next {} 0 R >> >>",7+level),content.as_bytes()));
        }
        let (_dir, doc) = document("/Form Do", forms);
        assert!(collect(&doc, 0)
            .err()
            .unwrap()
            .to_string()
            .contains("budget exceeded"));
    }

    #[test]
    fn mapping_rejects_invalid_geometry_and_maps_quarter_turns() {
        assert!(map_region([0., 0., 1., 1.], (0, 1), TransformMatrix::IDENTITY).is_err());
        assert!(map_region([0., 0., 2., 1.], (1, 1), TransformMatrix::IDENTITY).is_err());
        assert!(map_region(
            [0., 0., 1., 1.],
            (1, 1),
            TransformMatrix::new(1., 1., 0., 1., 0., 0.)
        )
        .is_err());
        assert_eq!(
            map_region(
                [10., 20., 30., 40.],
                (100, 100),
                TransformMatrix::new(0., 100., -100., 0., 100., 0.)
            )
            .unwrap(),
            [40., 10., 40., 30.]
        );
    }
}
