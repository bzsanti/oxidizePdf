//! Incremental, unsigned signature fields with portable application metadata.
use super::signing::*;
use super::{ensure_modification_allowed, IncrementalModification, SignatureResult};
use crate::parser::{
    objects::{PdfArray, PdfDictionary, PdfObject, PdfStream},
    PdfReader,
};
use crate::writer::IncrementalUpdate;
use std::io::Cursor;

/// Portable slot. Completion with handwriting is distinct from a digital signature.
#[derive(Debug, Clone)]
pub struct SignatureSlot {
    pub name: String,
    pub metadata: String,
    pub page_index: usize,
    pub rotation: i32,
    pub rect: SignatureRect,
    pub completed: bool,
    pub digitally_signed: bool,
}

/// Create one empty field without touching any private key or original byte.
pub fn create_signature_slot(
    base: &[u8],
    field_name: &str,
    page_index: usize,
    rect: SignatureRect,
    metadata: &str,
) -> SignatureResult<Vec<u8>> {
    rect.validate()?;
    if field_name.is_empty()
        || field_name.len() > 128
        || !field_name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-')
        || metadata.len() > 4096
    {
        return Err(invalid("invalid signature slot identity or metadata"));
    }
    let mut reader = PdfReader::new(Cursor::new(base)).map_err(|e| invalid(e.to_string()))?;
    let mut catalog = reader
        .catalog()
        .map_err(|e| invalid(e.to_string()))?
        .clone();
    // Creating fields changes the form structure; certification must permit it.
    ensure_modification_allowed(
        &mut reader,
        &catalog,
        IncrementalModification::FormStructure,
    )?;
    ensure_fieldmdp_allows_signature(&mut reader, field_name)?;
    ensure_field_name_available(&mut reader, &catalog, field_name)?;
    let document = PdfReader::new(Cursor::new(base))
        .map_err(|e| invalid(e.to_string()))?
        .into_document();
    let page = document
        .get_page(u32::try_from(page_index).map_err(|_| invalid("page index overflow"))?)
        .map_err(|e| invalid(e.to_string()))?;
    let [l, b, r, t] = page.crop_box.unwrap_or(page.media_box);
    if rect.left < l || rect.bottom < b || rect.right > r || rect.top > t {
        return Err(invalid("slot lies outside the page"));
    }
    let mut update = IncrementalUpdate::from_base(base)?;
    let id = update.allocate_id()?;
    let mut field = PdfDictionary::new();
    field.insert("Type".into(), name("Annot"));
    field.insert("Subtype".into(), name("Widget"));
    field.insert("FT".into(), name("Sig"));
    field.insert("T".into(), text(field_name));
    field.insert("F".into(), PdfObject::Integer(4));
    field.insert("P".into(), reference(page.obj_ref));
    field.insert(
        "OxidizePageIndex".into(),
        PdfObject::Integer(page_index as i64),
    );
    field.insert("Rect".into(), rect.object());
    field.insert("OxidizeSlot".into(), text(metadata));
    update.replace(id, PdfObject::Dictionary(field))?;
    append_page_annotation(&mut reader, &mut update, page.obj_ref, &page.dict, id)?;
    append_acroform_field(&mut reader, &mut update, &mut catalog, id)?;
    update.replace(
        reader
            .trailer()
            .root()
            .map_err(|e| invalid(e.to_string()))?,
        PdfObject::Dictionary(catalog),
    )?;
    Ok(update.finish()?)
}

fn slot(dictionary: &PdfDictionary, source: &[u8]) -> SignatureResult<SignatureSlot> {
    if dictionary
        .get("FT")
        .and_then(PdfObject::as_name)
        .is_none_or(|n| n.0 != "Sig")
        || dictionary
            .get("Subtype")
            .and_then(PdfObject::as_name)
            .is_none_or(|n| n.0 != "Widget")
        || dictionary.contains_key("Parent")
    {
        return Err(invalid("prepared slot must be a root signature widget"));
    }
    let flags = |key: &str| -> SignatureResult<i64> {
        match dictionary.get(key) {
            None => Ok(0),
            Some(PdfObject::Integer(n)) if *n >= 0 => Ok(*n),
            _ => Err(invalid("invalid slot flags")),
        }
    };
    if flags("F")? & 35 != 0 {
        return Err(invalid("prepared slot is hidden"));
    }
    let read_only = flags("Ff")? & 1 != 0;
    let page_index = dictionary
        .get("OxidizePageIndex")
        .and_then(PdfObject::as_integer)
        .and_then(|v| usize::try_from(v).ok())
        .filter(|v| *v <= u32::MAX as usize)
        .ok_or_else(|| invalid("slot page index missing"))?;
    let document = PdfReader::new(Cursor::new(source))
        .map_err(|e| invalid(e.to_string()))?
        .into_document();
    let page = document
        .get_page(page_index as u32)
        .map_err(|e| invalid(e.to_string()))?;
    if dictionary.get("P").and_then(PdfObject::as_reference) != Some(page.obj_ref) {
        return Err(invalid("slot page reference does not match its page"));
    }
    let rotation = page.rotation.rem_euclid(360);
    if ![0, 90, 180, 270].contains(&rotation) {
        return Err(invalid("unsupported page rotation"));
    }
    let metadata = dictionary
        .get("OxidizeSlot")
        .and_then(PdfObject::as_string)
        .ok_or_else(|| invalid("field has no slot metadata"))?;
    if metadata.as_bytes().len() > 4096 {
        return Err(invalid("slot metadata exceeds limit"));
    }
    let metadata = String::from_utf8(metadata.as_bytes().to_vec())
        .map_err(|_| invalid("slot metadata is not UTF-8"))?;
    let values = dictionary
        .get("Rect")
        .and_then(PdfObject::as_array)
        .filter(|v| v.0.len() == 4)
        .ok_or_else(|| invalid("invalid slot rectangle"))?;
    let number = |i: usize| -> SignatureResult<f64> {
        match &values.0[i] {
            PdfObject::Integer(n) => Ok(*n as f64),
            PdfObject::Real(n) => Ok(*n),
            _ => Err(invalid("invalid slot rectangle")),
        }
    };
    let rect = SignatureRect {
        left: number(0)?,
        bottom: number(1)?,
        right: number(2)?,
        top: number(3)?,
    };
    rect.validate()?;
    let digitally_signed = dictionary
        .get("V")
        .is_some_and(|v| !matches!(v, PdfObject::Null));
    let completed = match dictionary.get("OxidizeCompleted") {
        None | Some(PdfObject::Boolean(false)) => digitally_signed,
        Some(PdfObject::Boolean(true)) => true,
        _ => return Err(invalid("invalid slot completion")),
    };
    if read_only && !completed {
        return Err(invalid("prepared slot is read-only"));
    }
    if completed && !dictionary.contains_key("AP") {
        return Err(invalid("completed slot has no appearance"));
    }
    Ok(SignatureSlot {
        name: dictionary
            .get("T")
            .and_then(PdfObject::as_string)
            .ok_or_else(|| invalid("slot name missing"))?
            .to_text(),
        metadata,
        page_index,
        rotation,
        rect,
        completed,
        digitally_signed,
    })
}

/// Read one prepared field. A missing or foreign field is an error.
pub fn read_signature_slot(base: &[u8], field_name: &str) -> SignatureResult<SignatureSlot> {
    let mut reader = PdfReader::new(Cursor::new(base)).map_err(|e| invalid(e.to_string()))?;
    let catalog = reader
        .catalog()
        .map_err(|e| invalid(e.to_string()))?
        .clone();
    let id = find_signature_field(&mut reader, &catalog, field_name)?;
    slot(
        reader
            .get_object(id.0, id.1)
            .map_err(|e| invalid(e.to_string()))?
            .as_dict()
            .ok_or_else(|| invalid("invalid slot field"))?,
        base,
    )
}

/// List application-prepared root fields; ordinary PDF fields remain untouched.
pub fn list_signature_slots(base: &[u8]) -> SignatureResult<Vec<SignatureSlot>> {
    let mut reader = PdfReader::new(Cursor::new(base)).map_err(|e| invalid(e.to_string()))?;
    let catalog = reader
        .catalog()
        .map_err(|e| invalid(e.to_string()))?
        .clone();
    let mut result = Vec::new();
    for id in root_field_references(&mut reader, &catalog)? {
        let field = reader
            .get_object(id.0, id.1)
            .map_err(|e| invalid(e.to_string()))?
            .as_dict()
            .ok_or_else(|| invalid("invalid root field"))?;
        if field.contains_key("OxidizeSlot") {
            if result.len() >= 100 {
                return Err(invalid("too many signature slots"));
            }
            result.push(slot(field, base)?);
        }
    }
    Ok(result)
}

/// Remove an unsigned prepared field before replacing a preparation. Signed or
/// handwritten-completed fields are immutable through this API.
pub fn remove_signature_slot(base: &[u8], field_name: &str) -> SignatureResult<Vec<u8>> {
    if read_signature_slot(base, field_name)?.completed {
        return Err(invalid("completed slots cannot be removed"));
    }
    let document = PdfReader::new(Cursor::new(base))
        .map_err(|e| invalid(e.to_string()))?
        .into_document();
    let mut reader = PdfReader::new(Cursor::new(base)).map_err(|e| invalid(e.to_string()))?;
    let mut catalog = reader
        .catalog()
        .map_err(|e| invalid(e.to_string()))?
        .clone();
    ensure_modification_allowed(
        &mut reader,
        &catalog,
        IncrementalModification::FormStructure,
    )?;
    ensure_fieldmdp_allows_signature(&mut reader, field_name)?;
    let id = find_signature_field(&mut reader, &catalog, field_name)?;
    let field = reader
        .get_object(id.0, id.1)
        .map_err(|e| invalid(e.to_string()))?
        .as_dict()
        .cloned()
        .ok_or_else(|| invalid("invalid field"))?;
    let page_id = field
        .get("P")
        .and_then(PdfObject::as_reference)
        .ok_or_else(|| invalid("slot page missing"))?;
    let mut page = reader
        .get_object(page_id.0, page_id.1)
        .map_err(|e| invalid(e.to_string()))?
        .as_dict()
        .cloned()
        .ok_or_else(|| invalid("invalid page"))?;
    let annotations = document
        .resolve(
            page.get("Annots")
                .ok_or_else(|| invalid("slot page annotations missing"))?,
        )
        .map_err(|e| invalid(e.to_string()))?;
    let mut annotations = annotations
        .as_array()
        .cloned()
        .ok_or_else(|| invalid("invalid page annotations"))?;
    annotations.0.retain(|a| a.as_reference() != Some(id));
    page.insert("Annots".into(), PdfObject::Array(annotations));
    let form = document
        .resolve(
            catalog
                .get("AcroForm")
                .ok_or_else(|| invalid("form missing"))?,
        )
        .map_err(|e| invalid(e.to_string()))?;
    let mut form = form
        .as_dict()
        .cloned()
        .ok_or_else(|| invalid("invalid form"))?;
    let fields = root_field_references(&mut reader, &catalog)?
        .into_iter()
        .filter(|r| *r != id)
        .map(reference)
        .collect();
    form.insert("Fields".into(), PdfObject::Array(PdfArray(fields)));
    catalog.insert("AcroForm".into(), PdfObject::Dictionary(form));
    let mut update = IncrementalUpdate::from_base(base)?;
    update.replace(page_id, PdfObject::Dictionary(page))?;
    update.replace(
        reader
            .trailer()
            .root()
            .map_err(|e| invalid(e.to_string()))?,
        PdfObject::Dictionary(catalog),
    )?;
    Ok(update.finish()?)
}

/// Put normalized handwritten strokes into the field appearance. For combined
/// signing, leave completion false and immediately digitally sign these bytes.
pub fn complete_signature_slot(
    base: &[u8],
    field_name: &str,
    strokes: &[Vec<[f64; 2]>],
    complete: bool,
) -> SignatureResult<Vec<u8>> {
    draw_signature_slot(base, field_name, strokes, complete, None)
}

/// Draw reviewed artwork before the cryptographic signing phase.
pub fn draw_signature_slot(
    base: &[u8],
    field_name: &str,
    strokes: &[Vec<[f64; 2]>],
    complete: bool,
    label: Option<&str>,
) -> SignatureResult<Vec<u8>> {
    if (strokes.is_empty() && label.is_none())
        || (!strokes.is_empty() && !strokes.iter().any(|s| s.windows(2).any(|p| p[0] != p[1])))
        || strokes.len() > 1000
        || strokes.iter().map(Vec::len).sum::<usize>() > 20_000
        || strokes.iter().any(|s| {
            s.len() < 2
                || s.iter()
                    .flatten()
                    .any(|v| !v.is_finite() || !(0.0..=1.0).contains(v))
        })
    {
        return Err(invalid("invalid or empty handwritten signature"));
    }
    let current = read_signature_slot(base, field_name)?;
    if current.completed {
        return Err(invalid("slot is already completed"));
    }
    let mut reader = PdfReader::new(Cursor::new(base)).map_err(|e| invalid(e.to_string()))?;
    let catalog = reader
        .catalog()
        .map_err(|e| invalid(e.to_string()))?
        .clone();
    ensure_modification_allowed(&mut reader, &catalog, IncrementalModification::FormFill)?;
    ensure_fieldmdp_allows_signature(&mut reader, field_name)?;
    let id = find_signature_field(&mut reader, &catalog, field_name)?;
    let mut field = reader
        .get_object(id.0, id.1)
        .map_err(|e| invalid(e.to_string()))?
        .as_dict()
        .cloned()
        .ok_or_else(|| invalid("invalid slot"))?;
    if field
        .get("Ff")
        .and_then(PdfObject::as_integer)
        .is_some_and(|v| v & 1 != 0)
    {
        return Err(invalid("slot is read-only"));
    }
    let bbox_width = current.rect.right - current.rect.left;
    let bbox_height = current.rect.top - current.rect.bottom;
    let (width, height) = if current.rotation % 180 == 90 {
        (bbox_height, bbox_width)
    } else {
        (bbox_width, bbox_height)
    };
    let transform = match current.rotation {
        90 => format!("0 1 -1 0 {bbox_width} 0 cm "),
        180 => format!("-1 0 0 -1 {bbox_width} {bbox_height} cm "),
        270 => format!("0 -1 1 0 0 {bbox_height} cm "),
        _ => String::new(),
    };
    if let Some(label) = label {
        if label.is_empty()
            || label.len() > 512
            || crate::text::TextEncoding::WinAnsiEncoding
                .encode_strict(label)
                .is_err()
            || crate::text::metrics::measure_text(label, &crate::text::Font::Helvetica, 8.0)
                > width - 8.0
            || height < 25.0
        {
            return Err(invalid(
                "certificate name does not fit in the signature rectangle",
            ));
        }
    }
    let label_height = if label.is_some() { 16.0 } else { 0.0 };
    // Input pad is 3:1. Fit it inside the field without changing proportions.
    let scale = (width / 3.0).min(height - label_height) * 0.9;
    let ox = (width - scale * 3.0) / 2.0;
    let oy = label_height + (height - label_height - scale) / 2.0;
    let mut data = format!(
        "q {transform}0.05 0.1 0.2 RG 1 J 1 j {} w\n",
        (scale / 100.0).max(0.5)
    );
    for stroke in strokes {
        for (i, [x, y]) in stroke.iter().enumerate() {
            data.push_str(&format!(
                "{} {} {}\n",
                ox + x * scale * 3.0,
                oy + (1.0 - y) * scale,
                if i == 0 { "m" } else { "l" }
            ));
        }
        data.push_str("S\n");
    }
    let mut resources = PdfDictionary::new();
    if let Some(label) = label {
        let encoded = crate::text::TextEncoding::WinAnsiEncoding
            .encode_strict(label)
            .map_err(|_| invalid("unsupported certificate name"))?;
        data.push_str(&format!(
            "BT /F1 8 Tf 0 g 4 4 Td ({}) Tj ET\n",
            crate::text::escape_pdf_string_literal(&encoded)
        ));
        let mut font = PdfDictionary::new();
        font.insert("Type".into(), name("Font"));
        font.insert("Subtype".into(), name("Type1"));
        font.insert("BaseFont".into(), name("Helvetica"));
        font.insert("Encoding".into(), name("WinAnsiEncoding"));
        let mut fonts = PdfDictionary::new();
        fonts.insert("F1".into(), PdfObject::Dictionary(font));
        resources.insert("Font".into(), PdfObject::Dictionary(fonts));
    }
    data.push_str("Q\n");
    let mut appearance = PdfDictionary::new();
    appearance.insert("Type".into(), name("XObject"));
    appearance.insert("Subtype".into(), name("Form"));
    appearance.insert(
        "BBox".into(),
        PdfObject::Array(PdfArray(vec![
            PdfObject::Integer(0),
            PdfObject::Integer(0),
            PdfObject::Real(bbox_width),
            PdfObject::Real(bbox_height),
        ])),
    );
    appearance.insert("Resources".into(), PdfObject::Dictionary(resources));
    let mut update = IncrementalUpdate::from_base(base)?;
    let appearance_id = update.allocate_id()?;
    update.replace(
        appearance_id,
        PdfObject::Stream(PdfStream {
            dict: appearance,
            data: data.into_bytes(),
        }),
    )?;
    let mut ap = PdfDictionary::new();
    ap.insert("N".into(), reference(appearance_id));
    field.insert("AP".into(), PdfObject::Dictionary(ap));
    if complete {
        field.insert("OxidizeCompleted".into(), PdfObject::Boolean(true));
        field.insert("Ff".into(), PdfObject::Integer(1));
    }
    update.replace(id, PdfObject::Dictionary(field))?;
    Ok(update.finish()?)
}
