//! Writable AcroForm field filling on an existing (parsed) PDF via a real
//! ISO 32000-1 §7.5.6 incremental update (issue #318).
//!
//! The writer-side [`crate::Document`] can only fill fields that were built
//! in the current process through a [`crate::forms::FormManager`]. There is
//! no way to set `/V` on the fields of a PDF produced elsewhere (Acrobat,
//! pdftk, another library) so a form reader recovers the value after a
//! re-serialize. [`IncrementalFormFiller`] closes that gap.
//!
//! # Approach (non-lossy)
//!
//! Rather than rehydrating a full writable `Document` (which cannot
//! faithfully represent an arbitrary parsed PDF and would corrupt complex
//! documents on round-trip), this appends a true incremental update to the
//! original bytes:
//!
//! 1. Parse the base PDF, resolve the `/AcroForm` field tree (handling
//!    `/Kids` and hierarchical names via `/Parent`).
//! 2. Clone the affected field dictionaries, set `/V`, and set
//!    `/AcroForm/NeedAppearances true` (ISO 32000-1 §12.7.2).
//! 3. Synthesize the visual appearance so non-compliant viewers and
//!    flatten/print pipelines (which never regenerate from `/V`) still show
//!    the value: text fields (`/FT /Tx`) get a freshly-built `/AP /N` Form
//!    XObject; button fields (`/FT /Btn`) get `/AS` set to the selected
//!    state, activating their pre-authored `/AP`. `NeedAppearances` stays
//!    true — the two are complementary, not in conflict.
//! 4. Emit the modified objects (each reusing its existing object id), the
//!    newly-allocated appearance streams (ids from the base `/Size`), a
//!    PARTIAL incremental cross-reference section covering only the changed
//!    ids, and a trailer chaining `/Prev` to the previous `startxref`,
//!    reusing the base `/Root` and `/ID`.
//!
//! The original bytes are emitted verbatim as the prefix of the output, so
//! every untouched object, page, font and content stream is preserved
//! byte-for-byte.

use crate::error::{PdfError, Result};
use crate::parser::objects::{PdfDictionary, PdfName, PdfObject, PdfString};
use crate::parser::PdfReader;
use crate::text::TextEncoding;
use std::collections::HashMap;
use std::io::Cursor;

/// Fills AcroForm fields on an existing parsed PDF by appending an
/// ISO 32000-1 §7.5.6 incremental update. See the module docs.
pub struct IncrementalFormFiller<'a> {
    base_bytes: &'a [u8],
}

impl<'a> IncrementalFormFiller<'a> {
    /// Create a filler over the bytes of an existing PDF.
    pub fn new(base_bytes: &'a [u8]) -> Self {
        Self { base_bytes }
    }

    /// Fill a single field by its fully-qualified name, returning the
    /// updated PDF bytes (base bytes + appended incremental update).
    pub fn fill(&self, field_name: &str, value: &str) -> Result<Vec<u8>> {
        self.fill_many(&[(field_name, value)])
    }

    /// Fill multiple fields in a single incremental update (one parse, one
    /// appended section). Field names are fully qualified
    /// (e.g. `"address.street"`).
    pub fn fill_many(&self, fields: &[(&str, &str)]) -> Result<Vec<u8>> {
        fill_many_impl(self.base_bytes, fields)
    }
}

// ---------------------------------------------------------------------------
// Field-tree resolution
// ---------------------------------------------------------------------------

/// Maximum field-tree recursion depth (defensive guard against pathological
/// or cyclic `/Kids` structures).
const MAX_FIELD_DEPTH: u8 = 32;

/// Resolve every terminal/named AcroForm field to its object id, keyed by
/// fully-qualified name (ISO 32000-1 §12.7.3.1). Reuses the caller's reader
/// (the base PDF is parsed once per fill).
fn resolve_acroform_fields(
    reader: &mut PdfReader<Cursor<&[u8]>>,
) -> Result<HashMap<String, (u32, u16)>> {
    let acroform_dict = {
        let catalog = reader
            .catalog()
            .map_err(|e| PdfError::InvalidStructure(format!("read catalog: {e}")))?
            .clone();
        match catalog.get("AcroForm") {
            Some(PdfObject::Reference(n, g)) => reader
                .get_object(*n, *g)
                .map_err(|e| PdfError::InvalidStructure(format!("resolve /AcroForm: {e}")))?
                .as_dict()
                .cloned()
                .ok_or_else(|| {
                    PdfError::InvalidStructure("/AcroForm is not a dictionary".to_string())
                })?,
            Some(PdfObject::Dictionary(d)) => d.clone(),
            _ => {
                return Err(PdfError::InvalidStructure(
                    "document has no /AcroForm".to_string(),
                ))
            }
        }
    };

    let field_refs: Vec<(u32, u16)> = match acroform_dict.get("Fields") {
        Some(PdfObject::Array(arr)) => arr.0.iter().filter_map(|o| o.as_reference()).collect(),
        _ => Vec::new(),
    };

    let mut out = HashMap::new();
    for (n, g) in field_refs {
        collect_fields(reader, (n, g), "", &mut out, 0)?;
    }
    Ok(out)
}

/// Recursively walk a field node, accumulating fully-qualified names. A node
/// is named if it carries `/T`; nodes with `/Kids` are intermediate (their
/// `/T` prefixes the children's names).
fn collect_fields(
    reader: &mut PdfReader<Cursor<&[u8]>>,
    node_ref: (u32, u16),
    parent_prefix: &str,
    out: &mut HashMap<String, (u32, u16)>,
    depth: u8,
) -> Result<()> {
    if depth >= MAX_FIELD_DEPTH {
        return Err(PdfError::InvalidStructure(
            "AcroForm field tree exceeds maximum depth".to_string(),
        ));
    }

    let node = reader
        .get_object(node_ref.0, node_ref.1)
        .map_err(|e| PdfError::InvalidStructure(format!("resolve field object: {e}")))?
        .as_dict()
        .cloned()
        .ok_or_else(|| {
            PdfError::InvalidStructure("field object is not a dictionary".to_string())
        })?;

    let partial = node
        .get("T")
        .and_then(|o| o.as_string())
        .map(|s| String::from_utf8_lossy(s.as_bytes()).into_owned());

    let full_name = match (&partial, parent_prefix.is_empty()) {
        (Some(t), true) => t.clone(),
        (Some(t), false) => format!("{parent_prefix}.{t}"),
        (None, _) => parent_prefix.to_string(),
    };

    let kids: Vec<(u32, u16)> = match node.get("Kids") {
        Some(PdfObject::Array(arr)) => arr.0.iter().filter_map(|o| o.as_reference()).collect(),
        _ => Vec::new(),
    };

    // A non-empty `/Kids` does NOT imply an intermediate field. Per ISO
    // 32000-1 §12.7.3.1 a single terminal field may carry `/T`/`/FT`/`/V`
    // AND a `/Kids` array whose elements are its WIDGET annotations
    // (`/Subtype /Widget`, no `/T`) — the dominant Acrobat layout. Only
    // recurse when at least one kid is itself a field (has `/T`); otherwise
    // the kids are widgets and THIS node is the terminal field.
    let kids_are_subfields = kids.iter().any(|(n, g)| {
        reader
            .get_object(*n, *g)
            .ok()
            .and_then(|o| o.as_dict())
            .map(|d| d.contains_key("T"))
            .unwrap_or(false)
    });

    if kids.is_empty() || !kids_are_subfields {
        // Terminal field — record it under its full name (if it has one).
        if partial.is_some() {
            out.insert(full_name, node_ref);
        }
    } else {
        for kid in kids {
            collect_fields(reader, kid, &full_name, out, depth + 1)?;
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Incremental update assembly
// ---------------------------------------------------------------------------

fn fill_many_impl(base_bytes: &[u8], fields: &[(&str, &str)]) -> Result<Vec<u8>> {
    let mut reader = PdfReader::new(Cursor::new(base_bytes))
        .map_err(|e| PdfError::InvalidStructure(format!("parse base PDF: {e}")))?;

    if reader.is_encrypted() {
        return Err(PdfError::PermissionDenied(
            "incremental form fill is not supported on encrypted PDFs".to_string(),
        ));
    }

    // Base trailer facts needed for the appended trailer.
    let base_startxref = reader.trailer().xref_offset;
    let base_root = reader
        .trailer()
        .root()
        .map_err(|e| PdfError::InvalidStructure(format!("base /Root: {e}")))?;
    let base_size = reader
        .trailer()
        .size()
        .map_err(|e| PdfError::InvalidStructure(format!("base /Size: {e}")))?;
    let base_id_first: Option<Vec<u8>> = first_id_bytes(reader.trailer().id());

    // Resolve the AcroForm dict object id and its current contents.
    let (acro_ref, mut acro_dict) = resolve_acroform_object(&mut reader)?;

    // Resolve field name -> object id (reusing the open reader, no second parse).
    let field_map = resolve_acroform_fields(&mut reader)?;

    // Build the set of modified objects. Duplicate field names (or distinct
    // names resolving to the same object id) collapse to a single rewrite
    // with the LAST value, so we never emit two objects for one id.
    let mut modified: Vec<(u32, u16, PdfDictionary)> = Vec::new();
    for (name, value) in fields {
        let (num, gen) = field_map
            .get(*name)
            .copied()
            .ok_or_else(|| PdfError::FieldNotFound((*name).to_string()))?;
        let mut field_dict = reader
            .get_object(num, gen)
            .map_err(|e| PdfError::InvalidStructure(format!("resolve field {name}: {e}")))?
            .as_dict()
            .cloned()
            .ok_or_else(|| {
                PdfError::InvalidStructure(format!("field {name} is not a dictionary"))
            })?;
        field_dict.insert(
            "V".to_string(),
            PdfObject::String(PdfString(value.as_bytes().to_vec())),
        );
        match modified.iter_mut().find(|(n, g, _)| *n == num && *g == gen) {
            Some(slot) => slot.2 = field_dict,
            None => modified.push((num, gen, field_dict)),
        }
    }

    // Keep NeedAppearances true: our synthesized /AP serves non-compliant
    // viewers and flatten/print pipelines, while compliant viewers may still
    // regenerate from /V. The two are not in conflict.
    acro_dict.insert("NeedAppearances".to_string(), PdfObject::Boolean(true));

    // Synthesize /AP appearance streams. New stream objects take fresh ids
    // starting at base_size (the next free object number); the field/widget
    // dicts are mutated to reference them. The AcroForm-level /DA is the
    // fallback font selector when a field carries none.
    let acro_da = da_of(&acro_dict);
    let widgets_by_parent = collect_widgets_by_parent(&mut reader);
    let mut new_streams: Vec<(u32, Vec<u8>, [f64; 4], Vec<u8>)> = Vec::new();
    let mut extra_widgets: Vec<(u32, u16, PdfDictionary)> = Vec::new();
    let mut next_id = base_size;
    for (num, gen, dict) in modified.iter_mut() {
        let ft = dict
            .get("FT")
            .and_then(|o| o.as_name())
            .map(|n| n.0.clone());
        match ft.as_deref() {
            Some("Tx") => {
                synthesize_text_field_ap(
                    (*num, *gen),
                    dict,
                    &acro_da,
                    &mut reader,
                    &widgets_by_parent,
                    &mut next_id,
                    &mut new_streams,
                    &mut extra_widgets,
                )?;
            }
            Some("Btn") => {
                // Buttons (checkbox/radio) carry pre-authored /AP states; the
                // visible state is selected by /AS, not a synthesized stream.
                if let Some(PdfObject::String(v)) = dict.get("V").cloned() {
                    let state = String::from_utf8_lossy(v.as_bytes()).into_owned();
                    dict.insert("AS".to_string(), PdfObject::Name(PdfName(state)));
                }
            }
            _ => {}
        }
    }
    let total_size = next_id;

    // ----- assemble appended bytes -----
    let mut out = Vec::with_capacity(base_bytes.len() + 1024);
    out.extend_from_slice(base_bytes);

    let mut changed: Vec<(u32, u16, u64)> = Vec::new();

    for (num, gen, dict) in &modified {
        let offset = out.len() as u64;
        write_indirect_object(&mut out, *num, *gen, dict)?;
        changed.push((*num, *gen, offset));
    }

    // Widget annotations rewritten to carry /AP (separate-widget /Kids layout).
    for (num, gen, dict) in &extra_widgets {
        let offset = out.len() as u64;
        write_indirect_object(&mut out, *num, *gen, dict)?;
        changed.push((*num, *gen, offset));
    }

    // AcroForm object.
    let acro_offset = out.len() as u64;
    write_indirect_object(&mut out, acro_ref.0, acro_ref.1, &acro_dict)?;
    changed.push((acro_ref.0, acro_ref.1, acro_offset));

    // Appearance-stream Form XObjects (new objects).
    for (id, content, bbox, resources) in &new_streams {
        let offset = out.len() as u64;
        write_indirect_stream_object(&mut out, *id, 0, content, *bbox, resources)?;
        changed.push((*id, 0, offset));
    }

    let xref_pos = out.len() as u64;
    // Keep the permanent first /ID element; regenerate the second so the
    // revision is distinguishable (§14.4). Omit /ID entirely if the base had
    // none.
    let id_pair = base_id_first.map(|first| {
        let second = derive_revision_id(&first, fields, xref_pos);
        (first, second)
    });
    out.extend_from_slice(&write_partial_xref_section(&changed));
    out.extend_from_slice(&write_incremental_trailer(
        base_startxref,
        base_root,
        total_size,
        xref_pos,
        id_pair,
    ));

    Ok(out)
}

/// Resolve the `/AcroForm` indirect object id and a clone of its dict.
fn resolve_acroform_object(
    reader: &mut PdfReader<Cursor<&[u8]>>,
) -> Result<((u32, u16), PdfDictionary)> {
    let catalog = reader
        .catalog()
        .map_err(|e| PdfError::InvalidStructure(format!("read catalog: {e}")))?
        .clone();
    match catalog.get("AcroForm") {
        Some(PdfObject::Reference(n, g)) => {
            let dict = reader
                .get_object(*n, *g)
                .map_err(|e| PdfError::InvalidStructure(format!("resolve /AcroForm: {e}")))?
                .as_dict()
                .cloned()
                .ok_or_else(|| {
                    PdfError::InvalidStructure("/AcroForm is not a dictionary".to_string())
                })?;
            Ok(((*n, *g), dict))
        }
        _ => Err(PdfError::InvalidStructure(
            "/AcroForm must be an indirect reference for incremental fill".to_string(),
        )),
    }
}

fn first_id_bytes(id: Option<&PdfObject>) -> Option<Vec<u8>> {
    match id {
        Some(PdfObject::Array(arr)) => arr
            .0
            .first()
            .and_then(|o| o.as_string())
            .map(|s| s.as_bytes().to_vec()),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Low-level serialization (Vec<u8>, no PdfWriter state)
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Appearance-stream (/AP /N) synthesis
//
// The filler operates on parser-side `PdfObject`s with NO `Document` / font
// registry, so it CANNOT drive the writer-side `forms::appearance` generator
// (which needs a resolved `Font` handle for the Type0/CID path). The two
// genuinely shareable, correctness-critical pieces are reused directly:
// WinAnsi strict encoding (`TextEncoding::WinAnsiEncoding`) and PDF literal
// escaping (`write_literal_string`). The remaining BT/Tf/Td/Tj/ET scaffold is
// PDF wire syntax, not algorithm — replicating it here is simpler and lower
// risk than bridging the writer object model. Scope is therefore built-in
// non-symbolic Type1 fonts (the dominant `/DA` case: Helv/Cour/Times).
// ---------------------------------------------------------------------------

/// Map a `/DA` font resource name to a Standard-14 `/BaseFont`. AcroForm `/DR`
/// commonly aliases the base-14 fonts (Acrobat's `Helv`, `HeBo`, `Cour`,
/// `TiRo`, …). Unknown names pass through verbatim — they may already be a
/// real base font (e.g. `Helvetica-Bold`).
fn da_font_to_base_font(name: &str) -> &str {
    match name {
        "Helv" => "Helvetica",
        "HeBO" | "HeBo" => "Helvetica-Bold",
        "HeOb" => "Helvetica-Oblique",
        "Cour" => "Courier",
        "CoBO" | "CoBo" => "Courier-Bold",
        "TiRo" => "Times-Roman",
        "TiBo" => "Times-Bold",
        "TiIt" => "Times-Italic",
        "Symb" => "Symbol",
        "ZaDb" => "ZapfDingbats",
        other => other,
    }
}

/// Read a 4-number `/Rect` ([llx lly urx ury]) as `[f64; 4]`, accepting
/// integer or real components.
fn rect_of(dict: &PdfDictionary) -> Option<[f64; 4]> {
    let arr = dict.get("Rect").and_then(|o| o.as_array())?;
    if arr.0.len() != 4 {
        return None;
    }
    let mut r = [0.0f64; 4];
    for (i, slot) in r.iter_mut().enumerate() {
        *slot = arr.0[i].as_real()?;
    }
    Some(r)
}

/// Extract a `/DA` string value from a dictionary entry, if present and a
/// string object.
fn da_of(dict: &PdfDictionary) -> Option<String> {
    dict.get("DA")
        .and_then(|o| o.as_string())
        .map(|s| String::from_utf8_lossy(s.as_bytes()).into_owned())
}

/// Build the `/AP /N` content stream for a single-line text widget: a
/// `q … BT /Font size Tf x y Td (value) Tj ET … Q` sequence in the widget's
/// local (BBox) coordinate space. Returns the content bytes, the BBox and the
/// serialized `/Resources` dict. Fails (rather than emitting `?`) when the
/// value carries a codepoint WinAnsiEncoding cannot represent — matching the
/// writer-side generator's strict contract.
fn build_text_ap(
    value: &str,
    da_font_name: &str,
    da_font_size: f64,
    rect: [f64; 4],
) -> Result<(Vec<u8>, [f64; 4], Vec<u8>)> {
    let width = (rect[2] - rect[0]).abs();
    let height = (rect[3] - rect[1]).abs();
    // Size 0 is "auto"; with no glyph metrics here, pick a sane concrete size.
    let font_size = if da_font_size > 0.0 {
        da_font_size
    } else {
        12.0
    };

    let encoded = TextEncoding::WinAnsiEncoding
        .encode_strict(value)
        .map_err(|ch| {
            PdfError::EncodingError(format!(
                "form value contains character {:?} (U+{:04X}) not representable in \
                 WinAnsiEncoding used by built-in font {:?}; an embedded Type0 font \
                 would be required (not yet supported on the incremental fill path)",
                ch, ch as u32, da_font_name,
            ))
        })?;

    // Vertical centring identical to forms::appearance::TextFieldAppearance.
    let pad = 2.0;
    let text_y = (height - font_size) / 2.0 + font_size * 0.3;

    let mut content = Vec::new();
    content.extend_from_slice(b"q\n");
    content.extend_from_slice(b"BT\n");
    content
        .extend_from_slice(format!("/{da_font_name} {} Tf\n", format_real(font_size)).as_bytes());
    content.extend_from_slice(b"0 g\n");
    content
        .extend_from_slice(format!("{} {} Td\n", format_real(pad), format_real(text_y)).as_bytes());
    write_literal_string(&mut content, &encoded);
    content.extend_from_slice(b" Tj\n");
    content.extend_from_slice(b"ET\n");
    content.extend_from_slice(b"Q");

    // /Resources << /Font << /{da_font_name} << /Type /Font /Subtype /Type1
    //                        /BaseFont /{base} >> >> >>
    let mut font_entry = PdfDictionary::new();
    font_entry.insert(
        "Type".to_string(),
        PdfObject::Name(PdfName("Font".to_string())),
    );
    font_entry.insert(
        "Subtype".to_string(),
        PdfObject::Name(PdfName("Type1".to_string())),
    );
    font_entry.insert(
        "BaseFont".to_string(),
        PdfObject::Name(PdfName(da_font_to_base_font(da_font_name).to_string())),
    );
    let mut font_dict = PdfDictionary::new();
    font_dict.insert(da_font_name.to_string(), PdfObject::Dictionary(font_entry));
    let mut resources = PdfDictionary::new();
    resources.insert("Font".to_string(), PdfObject::Dictionary(font_dict));
    let mut resources_bytes = Vec::new();
    write_dict(&mut resources_bytes, &resources)?;

    Ok((content, [0.0, 0.0, width, height], resources_bytes))
}

/// Resolve the effective `(font_name, size)` for a widget from its own `/DA`,
/// then the field's `/DA`, then the AcroForm `/DA`, defaulting to Helvetica 12.
fn resolve_da(
    widget_da: Option<&str>,
    field_da: Option<&str>,
    acro_da: Option<&str>,
) -> (String, f64) {
    widget_da
        .and_then(parse_da_string)
        .or_else(|| field_da.and_then(parse_da_string))
        .or_else(|| acro_da.and_then(parse_da_string))
        .unwrap_or_else(|| ("Helv".to_string(), 12.0))
}

/// Map each field object id to the widget annotation ids that reference it via
/// `/Parent`, by walking the page tree and scanning every page's `/Annots`.
/// This recovers the widget(s) for the common layout where a single-widget
/// field omits `/Kids` and is linked only by the widget's `/Parent`
/// back-reference (ISO 32000-1 §12.7.3.1 NOTE).
fn collect_widgets_by_parent(
    reader: &mut PdfReader<Cursor<&[u8]>>,
) -> HashMap<(u32, u16), Vec<(u32, u16)>> {
    let mut map: HashMap<(u32, u16), Vec<(u32, u16)>> = HashMap::new();
    let root = match reader.pages() {
        Ok(p) => p.clone(),
        Err(_) => return map,
    };
    // Iterative page-tree walk with a visited set + bound (defensive against
    // cyclic or pathological /Kids).
    let mut stack: Vec<(u32, u16)> = match root.get("Kids") {
        Some(PdfObject::Array(arr)) => arr.0.iter().filter_map(|o| o.as_reference()).collect(),
        _ => Vec::new(),
    };
    let mut visited: std::collections::HashSet<(u32, u16)> = std::collections::HashSet::new();
    let mut budget = 100_000u32;
    while let Some(node_ref) = stack.pop() {
        if budget == 0 || !visited.insert(node_ref) {
            continue;
        }
        budget -= 1;
        let node = match reader
            .get_object(node_ref.0, node_ref.1)
            .ok()
            .and_then(|o| o.as_dict().cloned())
        {
            Some(d) => d,
            None => continue,
        };
        // Intermediate Pages node: descend.
        if let Some(PdfObject::Array(kids)) = node.get("Kids") {
            let is_pages = node
                .get("Type")
                .and_then(|o| o.as_name())
                .map(|n| n.0 == "Pages")
                .unwrap_or(false);
            if is_pages || node.get("Count").is_some() {
                for k in &kids.0 {
                    if let Some(r) = k.as_reference() {
                        stack.push(r);
                    }
                }
                continue;
            }
        }
        // Leaf page: scan /Annots for widgets carrying a /Parent reference.
        if let Some(PdfObject::Array(annots)) = node.get("Annots") {
            let annot_refs: Vec<(u32, u16)> =
                annots.0.iter().filter_map(|o| o.as_reference()).collect();
            for (an, ag) in annot_refs {
                if let Some(annot) = reader
                    .get_object(an, ag)
                    .ok()
                    .and_then(|o| o.as_dict().cloned())
                {
                    if let Some(parent) = annot.get("Parent").and_then(|o| o.as_reference()) {
                        map.entry(parent).or_default().push((an, ag));
                    }
                }
            }
        }
    }
    map
}

/// Synthesize the `/AP /N` stream(s) for one text field, mutating the field
/// (merged layout) or its widget annotations (separate-widget layouts) to
/// reference a freshly-allocated appearance object. New stream descriptors go
/// to `new_streams`; rewritten widgets go to `extra_widgets`.
///
/// Three layouts (ISO 32000-1 §12.7.3.1 / §12.5.6.19):
/// - field dict carries `/Rect` → merged field+widget; `/AP` on the field.
/// - field dict has `/Kids` widget annotations → `/AP` on each kid.
/// - field has neither, widgets linked only by `/Parent` → resolved via
///   `widgets_by_parent`.
fn synthesize_text_field_ap(
    field_id: (u32, u16),
    field_dict: &mut PdfDictionary,
    acro_da: &Option<String>,
    reader: &mut PdfReader<Cursor<&[u8]>>,
    widgets_by_parent: &HashMap<(u32, u16), Vec<(u32, u16)>>,
    next_id: &mut u32,
    new_streams: &mut Vec<(u32, Vec<u8>, [f64; 4], Vec<u8>)>,
    extra_widgets: &mut Vec<(u32, u16, PdfDictionary)>,
) -> Result<()> {
    let value = match field_dict.get("V") {
        Some(PdfObject::String(s)) => String::from_utf8_lossy(s.as_bytes()).into_owned(),
        _ => return Ok(()),
    };
    let field_da = da_of(field_dict);

    // Merged field+widget: the field dict itself has the /Rect.
    if let Some(rect) = rect_of(field_dict) {
        let (font_name, font_size) =
            resolve_da(field_da.as_deref(), field_da.as_deref(), acro_da.as_deref());
        let (content, bbox, resources) = build_text_ap(&value, &font_name, font_size, rect)?;
        let ap_id = *next_id;
        *next_id += 1;
        new_streams.push((ap_id, content, bbox, resources));
        let mut ap = PdfDictionary::new();
        ap.insert("N".to_string(), PdfObject::Reference(ap_id, 0));
        field_dict.insert("AP".to_string(), PdfObject::Dictionary(ap));
        return Ok(());
    }

    // Separate-widget layouts: widget refs from /Kids, else from the
    // /Parent reverse map.
    let mut widget_refs: Vec<(u32, u16)> = match field_dict.get("Kids") {
        Some(PdfObject::Array(arr)) => arr.0.iter().filter_map(|o| o.as_reference()).collect(),
        _ => Vec::new(),
    };
    if widget_refs.is_empty() {
        if let Some(refs) = widgets_by_parent.get(&field_id) {
            widget_refs = refs.clone();
        }
    }
    for (kn, kg) in widget_refs {
        let mut kid = match reader
            .get_object(kn, kg)
            .ok()
            .and_then(|o| o.as_dict().cloned())
        {
            Some(d) => d,
            None => continue,
        };
        let is_widget = kid
            .get("Subtype")
            .and_then(|o| o.as_name())
            .map(|n| n.0 == "Widget")
            .unwrap_or(false);
        let rect = match rect_of(&kid) {
            Some(r) if is_widget => r,
            _ => continue,
        };
        let widget_da = da_of(&kid);
        let (font_name, font_size) = resolve_da(
            widget_da.as_deref(),
            field_da.as_deref(),
            acro_da.as_deref(),
        );
        let (content, bbox, resources) = build_text_ap(&value, &font_name, font_size, rect)?;
        let ap_id = *next_id;
        *next_id += 1;
        new_streams.push((ap_id, content, bbox, resources));
        let mut ap = PdfDictionary::new();
        ap.insert("N".to_string(), PdfObject::Reference(ap_id, 0));
        kid.insert("AP".to_string(), PdfObject::Dictionary(ap));
        extra_widgets.push((kn, kg, kid));
    }
    Ok(())
}

/// Parse a `/DA` (default appearance) string, returning the selected font
/// resource name (without the leading `/`) and size in points.
///
/// A `/DA` value is a content-stream fragment whose font selector is
/// `/{name} {size} Tf` (ISO 32000-1 §12.7.3.3), optionally preceded by colour
/// operators (`g`, `rg`, `k`). The scanner finds the `Tf` operator and takes
/// the two tokens immediately before it. A size of `0` is the auto-size
/// sentinel (the viewer chooses) and is preserved verbatim. Returns `None`
/// when no well-formed font selector is present.
fn parse_da_string(da: &str) -> Option<(String, f64)> {
    let tokens: Vec<&str> = da.split_whitespace().collect();
    let tf_idx = tokens.iter().position(|t| *t == "Tf")?;
    if tf_idx < 2 {
        return None;
    }
    let name_tok = tokens[tf_idx - 2];
    let size_tok = tokens[tf_idx - 1];
    let name = name_tok.strip_prefix('/')?;
    if name.is_empty() {
        return None;
    }
    let size: f64 = size_tok.parse().ok()?;
    Some((name.to_string(), size))
}

/// Write a Form XObject appearance-stream object
/// `{num} {gen} obj\n<< /Type /XObject /Subtype /Form /BBox [...] /Length L
/// /Resources ... >>\nstream\n{content}\nendstream\nendobj\n`.
///
/// The content is emitted UNCOMPRESSED, so `/Length` is exactly the raw byte
/// count — no filter, no two-pass buffering. `resources_snippet` is a
/// pre-serialized `<< ... >>` dictionary (built by the caller via the
/// module's `write_dict`) inlined as the `/Resources` value.
fn write_indirect_stream_object(
    out: &mut Vec<u8>,
    num: u32,
    gen: u16,
    content: &[u8],
    bbox: [f64; 4],
    resources_snippet: &[u8],
) -> Result<()> {
    out.extend_from_slice(format!("{num} {gen} obj\n").as_bytes());
    out.extend_from_slice(b"<< /Type /XObject /Subtype /Form /BBox [");
    out.extend_from_slice(
        format!(
            "{} {} {} {}",
            format_real(bbox[0]),
            format_real(bbox[1]),
            format_real(bbox[2]),
            format_real(bbox[3])
        )
        .as_bytes(),
    );
    out.extend_from_slice(b"] /Resources ");
    out.extend_from_slice(resources_snippet);
    out.extend_from_slice(format!(" /Length {} >>\n", content.len()).as_bytes());
    out.extend_from_slice(b"stream\n");
    out.extend_from_slice(content);
    out.extend_from_slice(b"\nendstream\nendobj\n");
    Ok(())
}

/// Write `{num} {gen} obj\n<dict>\nendobj\n` into `out`.
fn write_indirect_object(
    out: &mut Vec<u8>,
    num: u32,
    gen: u16,
    dict: &PdfDictionary,
) -> Result<()> {
    out.extend_from_slice(format!("{num} {gen} obj\n").as_bytes());
    write_object_value(out, &PdfObject::Dictionary(dict.clone()))?;
    out.extend_from_slice(b"\nendobj\n");
    Ok(())
}

/// Serialize a parser [`PdfObject`] to PDF wire bytes. Streams are rejected:
/// AcroForm field and form dictionaries never carry an embedded stream, and
/// emitting one without a fresh `/Length` would corrupt the file.
fn write_object_value(out: &mut Vec<u8>, obj: &PdfObject) -> Result<()> {
    match obj {
        PdfObject::Null => out.extend_from_slice(b"null"),
        PdfObject::Boolean(b) => out.extend_from_slice(if *b { b"true" } else { b"false" }),
        PdfObject::Integer(i) => out.extend_from_slice(i.to_string().as_bytes()),
        PdfObject::Real(f) => out.extend_from_slice(format_real(*f).as_bytes()),
        PdfObject::String(s) => write_literal_string(out, s.as_bytes()),
        PdfObject::Name(n) => write_name(out, n),
        PdfObject::Reference(num, gen) => {
            out.extend_from_slice(format!("{num} {gen} R").as_bytes())
        }
        PdfObject::Array(arr) => {
            out.extend_from_slice(b"[");
            for (i, item) in arr.0.iter().enumerate() {
                if i > 0 {
                    out.extend_from_slice(b" ");
                }
                write_object_value(out, item)?;
            }
            out.extend_from_slice(b"]");
        }
        PdfObject::Dictionary(d) => write_dict(out, d)?,
        PdfObject::Stream(_) => {
            return Err(PdfError::InvalidStructure(
                "unexpected stream object in AcroForm field dictionary".to_string(),
            ))
        }
    }
    Ok(())
}

fn write_dict(out: &mut Vec<u8>, dict: &PdfDictionary) -> Result<()> {
    out.extend_from_slice(b"<< ");
    // Deterministic key order: keeps output stable and tests reproducible.
    let mut keys: Vec<&PdfName> = dict.0.keys().collect();
    keys.sort_by(|a, b| a.0.cmp(&b.0));
    for key in keys {
        write_name(out, key);
        out.extend_from_slice(b" ");
        // Safe: key came from the dict.
        write_object_value(out, &dict.0[key])?;
        out.extend_from_slice(b" ");
    }
    out.extend_from_slice(b">>");
    Ok(())
}

fn write_name(out: &mut Vec<u8>, name: &PdfName) {
    out.extend_from_slice(b"/");
    for &b in name.0.as_bytes() {
        // Regular characters pass through; everything else is #XX-escaped
        // (ISO 32000-1 §7.3.5).
        if b.is_ascii_alphanumeric()
            || matches!(
                b,
                b'+' | b'-' | b'.' | b'_' | b'@' | b'$' | b':' | b';' | b'*' | b'?'
            )
        {
            out.push(b);
        } else {
            out.extend_from_slice(format!("#{b:02X}").as_bytes());
        }
    }
}

/// Serialize a PDF literal string `(...)`, escaping the reserved bytes and
/// emitting non-printable bytes as `\ddd` octal (ISO 32000-1 §7.3.4.2).
fn write_literal_string(out: &mut Vec<u8>, bytes: &[u8]) {
    out.push(b'(');
    for &b in bytes {
        match b {
            b'(' => out.extend_from_slice(b"\\("),
            b')' => out.extend_from_slice(b"\\)"),
            b'\\' => out.extend_from_slice(b"\\\\"),
            b'\n' => out.extend_from_slice(b"\\n"),
            b'\r' => out.extend_from_slice(b"\\r"),
            b'\t' => out.extend_from_slice(b"\\t"),
            0x20..=0x7E => out.push(b),
            _ => out.extend_from_slice(format!("\\{b:03o}").as_bytes()),
        }
    }
    out.push(b')');
}

/// Format a PDF real without scientific notation, trimming trailing zeros.
fn format_real(f: f64) -> String {
    // PDF has no token for NaN/Infinity; emit a benign 0 rather than a
    // malformed numeric. Field dicts essentially never carry such values,
    // but the serializer must stay total.
    if !f.is_finite() {
        return "0".to_string();
    }
    if f == f.trunc() {
        return format!("{}", f as i64);
    }
    let mut s = format!("{f:.6}");
    while s.ends_with('0') {
        s.pop();
    }
    if s.ends_with('.') {
        s.pop();
    }
    s
}

// ---------------------------------------------------------------------------
// Partial incremental xref + trailer
// ---------------------------------------------------------------------------

/// Build a partial cross-reference section listing ONLY the changed objects,
/// grouped into contiguous subsections (ISO 32000-1 §7.5.4).
fn write_partial_xref_section(changed: &[(u32, u16, u64)]) -> Vec<u8> {
    let mut entries = changed.to_vec();
    entries.sort_by_key(|(num, _, _)| *num);

    let mut out = Vec::new();
    out.extend_from_slice(b"xref\n");

    let mut i = 0;
    while i < entries.len() {
        let start = entries[i].0;
        let mut j = i;
        // Extend the subsection while object numbers stay contiguous.
        while j + 1 < entries.len() && entries[j + 1].0 == entries[j].0 + 1 {
            j += 1;
        }
        let count = j - i + 1;
        out.extend_from_slice(format!("{start} {count}\n").as_bytes());
        for entry in &entries[i..=j] {
            out.extend_from_slice(format!("{:010} {:05} n \n", entry.2, entry.1).as_bytes());
        }
        i = j + 1;
    }
    out
}

/// Build the incremental-update trailer chaining `/Prev` and reusing the
/// base `/Root` and `/Size`. The `/ID` array, when present, keeps the
/// permanent first element and carries a fresh second element that changes
/// with this revision (ISO 32000-1 Table 15 / §14.4 — signature validators
/// rely on the second element changing per update).
fn write_incremental_trailer(
    base_prev_xref: u64,
    base_root: (u32, u16),
    base_size: u32,
    new_xref_pos: u64,
    id_pair: Option<(Vec<u8>, Vec<u8>)>,
) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(b"trailer\n<< ");
    out.extend_from_slice(format!("/Size {base_size} ").as_bytes());
    out.extend_from_slice(format!("/Root {} {} R ", base_root.0, base_root.1).as_bytes());
    out.extend_from_slice(format!("/Prev {base_prev_xref} ").as_bytes());
    if let Some((first, second)) = id_pair {
        let hex = |bytes: &[u8]| -> String { bytes.iter().map(|b| format!("{b:02X}")).collect() };
        out.extend_from_slice(format!("/ID [<{}> <{}>] ", hex(&first), hex(&second)).as_bytes());
    }
    out.extend_from_slice(b">>\n");
    out.extend_from_slice(b"startxref\n");
    out.extend_from_slice(format!("{new_xref_pos}\n").as_bytes());
    out.extend_from_slice(b"%%EOF\n");
    out
}

/// Derive a fresh 16-byte `/ID` second element bound to this revision's
/// content (permanent id + the values written + the new xref position).
/// Deterministic so a given fill reproduces byte-for-byte (testable), while
/// still differing from the base second element and from other revisions.
fn derive_revision_id(first: &[u8], fields: &[(&str, &str)], xref_pos: u64) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.extend_from_slice(first);
    for (name, value) in fields {
        buf.extend_from_slice(name.as_bytes());
        buf.push(0);
        buf.extend_from_slice(value.as_bytes());
        buf.push(0);
    }
    buf.extend_from_slice(&xref_pos.to_le_bytes());
    md5::compute(&buf).0.to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn partial_xref_groups_contiguous_subsections() {
        let bytes = write_partial_xref_section(&[(5, 0, 1024), (7, 0, 2048)]);
        let s = String::from_utf8(bytes).unwrap();
        assert!(s.starts_with("xref\n"), "must start with xref keyword");
        assert!(s.contains("5 1\n0000001024 00000 n \n"), "obj 5 subsection");
        assert!(s.contains("7 1\n0000002048 00000 n \n"), "obj 7 subsection");
        assert!(!s.contains("\n6 "), "gap object 6 must not appear");
    }

    #[test]
    fn partial_xref_merges_adjacent_ids() {
        let bytes = write_partial_xref_section(&[(7, 0, 2048), (5, 0, 1024), (6, 0, 1536)]);
        let s = String::from_utf8(bytes).unwrap();
        // 5,6,7 are contiguous -> one subsection "5 3".
        assert!(
            s.contains("5 3\n"),
            "contiguous ids form one subsection: {s}"
        );
        assert!(s.contains("0000001024 00000 n \n0000001536 00000 n \n0000002048 00000 n \n"));
    }

    #[test]
    fn incremental_trailer_carries_root_prev_size() {
        let bytes = write_incremental_trailer(312, (1, 0), 8, 5000, None);
        let s = String::from_utf8(bytes).unwrap();
        assert!(s.contains("/Prev 312"), "must chain /Prev: {s}");
        assert!(s.contains("/Root 1 0 R"), "must reuse base /Root");
        assert!(s.contains("/Size 8"), "must carry /Size");
        assert!(s.ends_with("startxref\n5000\n%%EOF\n"), "suffix: {s}");
        assert!(!s.contains("/Info"), "no /Info in incremental trailer");
    }

    #[test]
    fn incremental_trailer_emits_distinct_id_pair() {
        // The first element is preserved; the second changes with the
        // revision (§14.4). They must NOT be identical.
        let bytes = write_incremental_trailer(
            10,
            (2, 0),
            5,
            99,
            Some((vec![0xDE, 0xAD], vec![0xBE, 0xEF])),
        );
        let s = String::from_utf8(bytes).unwrap();
        assert!(s.contains("/ID [<DEAD> <BEEF>]"), "distinct id pair: {s}");
    }

    #[test]
    fn revision_id_differs_from_permanent_and_is_deterministic() {
        let first = vec![1u8, 2, 3, 4];
        let a = derive_revision_id(&first, &[("name", "Ada")], 100);
        let b = derive_revision_id(&first, &[("name", "Ada")], 100);
        let c = derive_revision_id(&first, &[("name", "Grace")], 100);
        assert_eq!(a, b, "same inputs -> same id (reproducible)");
        assert_ne!(
            a, first,
            "second element must differ from the permanent one"
        );
        assert_ne!(a, c, "different content -> different revision id");
        assert_eq!(a.len(), 16, "PDF /ID elements are 16 bytes");
    }

    #[test]
    fn literal_string_escapes_reserved_bytes() {
        let mut out = Vec::new();
        write_literal_string(&mut out, b"a(b)c\\d");
        assert_eq!(out, b"(a\\(b\\)c\\\\d)");
    }

    // ----- Cycle 1: /DA string parsing -----

    #[test]
    fn parse_da_string_extracts_font_and_size() {
        assert_eq!(
            parse_da_string("/Helv 12 Tf 0 g"),
            Some(("Helv".to_string(), 12.0))
        );
        assert_eq!(
            parse_da_string("/Helvetica-Bold 9.5 Tf 0 g"),
            Some(("Helvetica-Bold".to_string(), 9.5))
        );
        // Color operators before Tf must not confuse the scanner.
        assert_eq!(
            parse_da_string("0 0 1 rg /Cour 8 Tf"),
            Some(("Cour".to_string(), 8.0))
        );
        // Size 0 is the auto-size sentinel (viewer chooses); preserved as 0.0.
        assert_eq!(parse_da_string("/F1 0 Tf"), Some(("F1".to_string(), 0.0)));
        assert_eq!(parse_da_string(""), None);
        assert_eq!(parse_da_string("garbage"), None);
        // A Tf with no preceding name/size is not a valid font selector.
        assert_eq!(parse_da_string("Tf"), None);
    }

    // ----- Cycle 2: appearance stream object serialization -----

    #[test]
    fn write_indirect_stream_object_produces_valid_form_xobject() {
        let content = b"q BT /Helv 12 Tf 2 5 Td (Hi) Tj ET Q";
        let mut resources = Vec::new();
        resources.extend_from_slice(b"<< /Font << /Helv << /Type /Font >> >> >>");
        let mut out = Vec::new();
        write_indirect_stream_object(&mut out, 9, 0, content, [0.0, 0.0, 200.0, 20.0], &resources)
            .unwrap();
        let s = String::from_utf8(out.clone()).unwrap();

        assert!(s.starts_with("9 0 obj\n<<"), "object header: {s}");
        assert!(s.contains("/Type /XObject"), "must be XObject: {s}");
        assert!(s.contains("/Subtype /Form"), "must be Form: {s}");
        assert!(s.contains("/BBox [0 0 200 20]"), "bbox: {s}");
        assert!(
            s.contains(&format!("/Length {}", content.len())),
            "length must equal raw content byte count: {s}"
        );
        assert!(s.contains("/Resources << /Font"), "resources inlined: {s}");
        // Exact stream framing around the verbatim content.
        let marker = format!(
            "stream\n{}\nendstream\nendobj\n",
            String::from_utf8_lossy(content)
        );
        assert!(s.ends_with(&marker), "stream framing: {s}");

        // Deterministic.
        let mut out2 = Vec::new();
        write_indirect_stream_object(
            &mut out2,
            9,
            0,
            content,
            [0.0, 0.0, 200.0, 20.0],
            &resources,
        )
        .unwrap();
        assert_eq!(out, out2, "serialization must be deterministic");
    }
}
