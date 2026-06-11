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
//!    `/AcroForm/NeedAppearances true` (ISO 32000-1 §12.7.2 — compliant
//!    viewers regenerate the appearance stream; `/AP` generation is a
//!    documented follow-up).
//! 3. Emit ONLY the modified objects (each reusing its existing object id),
//!    a PARTIAL incremental cross-reference section covering only the
//!    changed ids, and a trailer chaining `/Prev` to the previous
//!    `startxref`, reusing the base `/Root` and `/ID`.
//!
//! The original bytes are emitted verbatim as the prefix of the output, so
//! every untouched object, page, font and content stream is preserved
//! byte-for-byte.

use crate::error::{PdfError, Result};
use crate::parser::objects::{PdfDictionary, PdfName, PdfObject, PdfString};
use crate::parser::PdfReader;
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

/// Resolve every terminal/named AcroForm field of `bytes` to its object id,
/// keyed by fully-qualified name (ISO 32000-1 §12.7.3.1).
fn resolve_acroform_fields(bytes: &[u8]) -> Result<HashMap<String, (u32, u16)>> {
    let mut reader = PdfReader::new(Cursor::new(bytes))
        .map_err(|e| PdfError::InvalidStructure(format!("parse base PDF: {e}")))?;

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
        collect_fields(&mut reader, (n, g), "", &mut out, 0)?;
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

    if kids.is_empty() {
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

    // Resolve field name -> object id.
    let field_map = resolve_acroform_fields(base_bytes)?;

    // Build the set of modified objects.
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
        modified.push((num, gen, field_dict));
    }

    // Flag NeedAppearances so viewers regenerate the appearance stream.
    acro_dict.insert("NeedAppearances".to_string(), PdfObject::Boolean(true));

    // ----- assemble appended bytes -----
    let mut out = Vec::with_capacity(base_bytes.len() + 1024);
    out.extend_from_slice(base_bytes);

    let mut changed: Vec<(u32, u16, u64)> = Vec::new();

    for (num, gen, dict) in &modified {
        let offset = out.len() as u64;
        write_indirect_object(&mut out, *num, *gen, dict)?;
        changed.push((*num, *gen, offset));
    }

    // AcroForm object.
    let acro_offset = out.len() as u64;
    write_indirect_object(&mut out, acro_ref.0, acro_ref.1, &acro_dict)?;
    changed.push((acro_ref.0, acro_ref.1, acro_offset));

    let xref_pos = out.len() as u64;
    out.extend_from_slice(&write_partial_xref_section(&changed));
    out.extend_from_slice(&write_incremental_trailer(
        base_startxref,
        base_root,
        base_size,
        xref_pos,
        base_id_first,
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
    if f == f.trunc() && f.is_finite() {
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
/// base `/Root`, `/Size` and `/ID`.
fn write_incremental_trailer(
    base_prev_xref: u64,
    base_root: (u32, u16),
    base_size: u32,
    new_xref_pos: u64,
    base_id_first: Option<Vec<u8>>,
) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(b"trailer\n<< ");
    out.extend_from_slice(format!("/Size {base_size} ").as_bytes());
    out.extend_from_slice(format!("/Root {} {} R ", base_root.0, base_root.1).as_bytes());
    out.extend_from_slice(format!("/Prev {base_prev_xref} ").as_bytes());
    if let Some(id) = base_id_first {
        let hex: String = id.iter().map(|b| format!("{b:02X}")).collect();
        out.extend_from_slice(format!("/ID [<{hex}> <{hex}>] ").as_bytes());
    }
    out.extend_from_slice(b">>\n");
    out.extend_from_slice(b"startxref\n");
    out.extend_from_slice(format!("{new_xref_pos}\n").as_bytes());
    out.extend_from_slice(b"%%EOF\n");
    out
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
    fn incremental_trailer_emits_id_when_present() {
        let bytes = write_incremental_trailer(10, (2, 0), 5, 99, Some(vec![0xDE, 0xAD]));
        let s = String::from_utf8(bytes).unwrap();
        assert!(s.contains("/ID [<DEAD> <DEAD>]"), "id reused: {s}");
    }

    #[test]
    fn literal_string_escapes_reserved_bytes() {
        let mut out = Vec::new();
        write_literal_string(&mut out, b"a(b)c\\d");
        assert_eq!(out, b"(a\\(b\\)c\\\\d)");
    }
}
