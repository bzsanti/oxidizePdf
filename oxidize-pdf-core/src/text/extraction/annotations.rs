use super::*;

/// Normal appearance stream of one annotation, ready to run as page content.
struct AnnotationAppearance {
    stream: crate::parser::PdfStream,
    object: (u32, u16),
    /// Form space to page space: the form `/Matrix` followed by the mapping
    /// of its transformed `/BBox` onto the annotation `/Rect` (§12.5.5).
    matrix: [f64; 6],
}

/// Annotation flag bit 2, `Hidden` (ISO 32000-1 Table 165): neither displayed
/// nor printed.
const ANNOTATION_FLAG_HIDDEN: i64 = 1 << 1;

/// Select the normal appearance stream to extract for `annotation`, or `None`
/// when it has none, is not rendered, or is malformed. Never fails: appearances
/// are optional, so bad ones are skipped rather than failing the page.
fn resolve_annotation_appearance<R: Read + Seek>(
    document: &PdfDocument<R>,
    annotation: &PdfDictionary,
) -> Option<AnnotationAppearance> {
    // Link appearances are decoration and their targets are handled by
    // `with_link_annotation_extraction`; a Popup only displays its parent's text.
    let subtype = annotation.get("Subtype").and_then(PdfObject::as_name);
    if subtype.is_some_and(|s| s.0 == "Link" || s.0 == "Popup") {
        return None;
    }
    let flags = annotation
        .get("F")
        .and_then(PdfObject::as_integer)
        .unwrap_or(0);
    if flags & ANNOTATION_FLAG_HIDDEN != 0 {
        return None;
    }

    let appearances = resolve_annotation_dictionary(document, annotation.get("AP")?)?;
    let normal = appearances.get("N")?;
    let resolved = document.resolve(normal).ok()?;
    let (stream, object) = match resolved {
        PdfObject::Stream(stream) => (stream, normal.as_reference()?),
        PdfObject::Dictionary(states) => {
            let state = annotation.get("AS").and_then(PdfObject::as_name)?;
            let selected = states.get(&state.0)?;
            let object = selected.as_reference()?;
            let PdfObject::Stream(stream) = document.resolve(selected).ok()? else {
                return None;
            };
            (stream, object)
        }
        _ => return None,
    };

    let rect = read_rectangle(annotation.get("Rect")?)?;
    let bbox = read_rectangle(stream.dict.get("BBox")?)?;
    let form_matrix = stream
        .dict
        .get("Matrix")
        .and_then(read_numbers::<6>)
        .unwrap_or([1.0, 0.0, 0.0, 1.0, 0.0, 0.0]);
    let matrix = appearance_matrix(&form_matrix, bbox, rect)?;

    Some(AnnotationAppearance {
        stream,
        object,
        matrix,
    })
}

/// Read a numeric array of exactly `N` finite values (`/Rect`, `/BBox`,
/// `/Matrix`). Any other shape is treated as absent.
fn read_numbers<const N: usize>(object: &PdfObject) -> Option<[f64; N]> {
    let values: &[PdfObject] = &object.as_array()?.0;
    let values: &[PdfObject; N] = values.try_into().ok()?;
    let mut out = [0.0; N];
    for (slot, value) in out.iter_mut().zip(values) {
        *slot = value.as_real().filter(|v| v.is_finite())?;
    }
    Some(out)
}

/// Read a rectangle as normalised `[x_min, y_min, x_max, y_max]`.
fn read_rectangle(object: &PdfObject) -> Option<[f64; 4]> {
    let [x0, y0, x1, y1] = read_numbers::<4>(object)?;
    Some([x0.min(x1), y0.min(y1), x0.max(x1), y0.max(y1)])
}

/// Appearance-to-page matrix per ISO 32000-1 §12.5.5: transform `/BBox` by the
/// form `/Matrix`, then scale and translate the resulting box onto `/Rect`. A
/// zero-extent box axis keeps unit scale so degenerate forms still place their
/// origin at the rectangle. Returns `None` for non-finite results.
fn appearance_matrix(form_matrix: &[f64; 6], bbox: [f64; 4], rect: [f64; 4]) -> Option<[f64; 6]> {
    let corners = [
        (bbox[0], bbox[1]),
        (bbox[2], bbox[1]),
        (bbox[0], bbox[3]),
        (bbox[2], bbox[3]),
    ]
    .map(|(x, y)| transform_point(x, y, form_matrix));
    let min_x = corners.iter().map(|p| p.0).fold(f64::INFINITY, f64::min);
    let max_x = corners
        .iter()
        .map(|p| p.0)
        .fold(f64::NEG_INFINITY, f64::max);
    let min_y = corners.iter().map(|p| p.1).fold(f64::INFINITY, f64::min);
    let max_y = corners
        .iter()
        .map(|p| p.1)
        .fold(f64::NEG_INFINITY, f64::max);

    let scale = |target: f64, source: f64| {
        if source.abs() <= f64::EPSILON {
            1.0
        } else {
            target / source
        }
    };
    let sx = scale(rect[2] - rect[0], max_x - min_x);
    let sy = scale(rect[3] - rect[1], max_y - min_y);
    let fit = [sx, 0.0, 0.0, sy, rect[0] - min_x * sx, rect[1] - min_y * sy];

    let matrix = multiply_matrix(form_matrix, &fit);
    matrix.iter().all(|v| v.is_finite()).then_some(matrix)
}

impl TextExtractor {
    /// Finalize each appearance in its own resource/layout scope, then join it
    /// to the page without exposing either side to the other's layout repairs.
    pub(super) fn append_annotation_appearances<R: Read + Seek>(
        &mut self,
        document: &PdfDocument<R>,
        page_index: u32,
        mut page: ExtractedText,
    ) -> ParseResult<ExtractedText> {
        let annotations = document
            .get_page_annotations(page_index)
            .unwrap_or_default();
        for annotation in annotations {
            if page.truncated {
                break;
            }
            let Some(AnnotationAppearance {
                stream,
                object,
                matrix,
            }) = resolve_annotation_appearance(document, &annotation)
            else {
                continue;
            };
            let Some(PreparedStream {
                operations,
                resources,
                mut structure_actual_text,
            }) = self.prepare_stream(
                document,
                stream,
                RecoveryLocation::AnnotationAppearance { object },
            )?
            else {
                continue;
            };
            let separator_len = usize::from(!page.text.is_empty());
            let remaining = self
                .options
                .max_extracted_bytes
                .map(|limit| limit.saturating_sub(page.text.len().saturating_add(separator_len)));
            let run = OpRunState {
                state: TextState {
                    ctm: matrix,
                    ..TextState::default()
                },
                in_text_object: false,
                at_text_object_start: false,
                last_x: 0.0,
                last_y: 0.0,
                last_shown_font_name: None,
                extracted_text: String::new(),
                fragments: Vec::new(),
                truncated: false,
                line_groups: Vec::new(),
                cur_group: None,
            };
            let appearance = self.with_font_scope(
                document,
                resources.as_ref(),
                FontScope::Independent,
                |extractor| {
                    let run = extractor.process_operations(
                        operations,
                        document,
                        resources.as_ref(),
                        &mut structure_actual_text,
                        run,
                        page_index,
                        1,
                        remaining,
                    )?;
                    extractor.finish_run(run, remaining)
                },
            )?;
            page.truncated |= appearance.truncated;
            page.fragments.extend(appearance.fragments);
            if !appearance.text.is_empty() {
                if separator_len > 0 {
                    page.text.push('\n');
                }
                page.text.push_str(&appearance.text);
            }
        }
        Ok(page)
    }
}
