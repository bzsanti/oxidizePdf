//! Acceptance tests for audit issue #334 item #1+#2+#3: XMP packet bytes
//! for `XmpValue::Struct` and `XmpValue::ArrayStruct` must be deterministic.
//!
//! #331 fixed the `xmlns:*` declaration ordering (namespace prefixes are now
//! sorted lexicographically). The Struct / ArrayStruct field iteration was
//! out of scope for that PR and still uses `HashMap<String, Box<XmpValue>>`
//! internally (xmp.rs:145, 147), so the inner field elements emitted under
//! `<rdf:Description>` (Struct) and `<rdf:li rdf:parseType="Resource">`
//! (ArrayStruct items) appear in HashMap iteration order — randomized per
//! instance.
//!
//! Two distinct sources of non-determinism for these values:
//!   1. Each call to `set_struct` / `set_array_struct` allocates a fresh
//!      `HashMap` via `collect()` — fresh random seed → fresh order.
//!   2. Two `XmpMetadata` instances built with identical inputs get
//!      different orderings (different seeds, same data).
//!
//! Test strategy: build the SAME metadata N times in one process and assert
//! every resulting `to_xmp_packet()` is byte-identical.

use oxidize_pdf::metadata::xmp::{XmpMetadata, XmpNamespace, XmpValue};
use std::collections::HashMap;

/// Build an `XmpMetadata` with both a `set_struct` and a `set_array_struct`
/// property, each carrying field names spread across the lexical space so
/// any sort-order regression is visible.
fn build_with_structs() -> XmpMetadata {
    let mut xmp = XmpMetadata::new();
    xmp.set_text(XmpNamespace::DublinCore, "title", "Issue 334 fixture");

    // Single struct property — `xmpMM:DerivedFrom` is a real XMP structure.
    let mut derived = HashMap::new();
    derived.insert(
        "instanceID".to_string(),
        XmpValue::Text("uuid:src-instance".to_string()),
    );
    derived.insert(
        "documentID".to_string(),
        XmpValue::Text("uuid:src-doc".to_string()),
    );
    derived.insert(
        "renditionClass".to_string(),
        XmpValue::Text("default".to_string()),
    );
    derived.insert(
        "alternatePaths".to_string(),
        XmpValue::Text("/tmp/sources".to_string()),
    );
    derived.insert("zVersion".to_string(), XmpValue::Text("1.0".to_string()));
    xmp.set_struct(XmpNamespace::XmpMediaManagement, "DerivedFrom", derived);

    // Array of structs — `xmpMM:History` is typically a sequence of edit events.
    let mut event1 = HashMap::new();
    event1.insert("action".to_string(), XmpValue::Text("created".to_string()));
    event1.insert(
        "softwareAgent".to_string(),
        XmpValue::Text("oxidize-pdf".to_string()),
    );
    event1.insert(
        "when".to_string(),
        XmpValue::Text("2026-06-16T00:00:00Z".to_string()),
    );
    event1.insert(
        "instanceID".to_string(),
        XmpValue::Text("uuid:event1".to_string()),
    );
    event1.insert(
        "zChanged".to_string(),
        XmpValue::Text("/metadata".to_string()),
    );

    let mut event2 = HashMap::new();
    event2.insert("action".to_string(), XmpValue::Text("saved".to_string()));
    event2.insert(
        "softwareAgent".to_string(),
        XmpValue::Text("oxidize-pdf".to_string()),
    );
    event2.insert(
        "when".to_string(),
        XmpValue::Text("2026-06-16T00:05:00Z".to_string()),
    );
    event2.insert(
        "instanceID".to_string(),
        XmpValue::Text("uuid:event2".to_string()),
    );
    event2.insert(
        "zChanged".to_string(),
        XmpValue::Text("/content".to_string()),
    );

    xmp.set_array_struct(
        XmpNamespace::XmpMediaManagement,
        "History",
        vec![event1, event2],
    );
    xmp
}

/// Extract just the lines that emit struct field elements.
///
/// Struct fields are serialized as `<fieldName>value</fieldName>` (the
/// `serialize_value` call passes the field name as the tag, with no
/// namespace prefix). Top-level XMP properties carry a prefix
/// (`<dc:title>`, `<xmpMM:DerivedFrom>`); container tags use `rdf:`
/// (`<rdf:Description>`, `<rdf:li>`); the XMP packet wrapper uses `x:`
/// and `<?xpacket ?>`. Anything else with an opening tag of bare letters
/// is a struct field.
fn struct_field_lines(packet: &str) -> Vec<&str> {
    packet
        .lines()
        .filter(|l| {
            let trimmed = l.trim();
            if !trimmed.starts_with('<') {
                return false;
            }
            if trimmed.starts_with("</")
                || trimmed.starts_with("<?")
                || trimmed.starts_with("<rdf:")
                || trimmed.starts_with("<x:")
            {
                return false;
            }
            // Tag content between '<' and the first '>' or space. A struct
            // field tag has no namespace prefix (no ':').
            let after_lt = &trimmed[1..];
            let tag_end = after_lt
                .find(|c: char| c == '>' || c.is_whitespace())
                .unwrap_or(after_lt.len());
            let tag = &after_lt[..tag_end];
            !tag.is_empty() && !tag.contains(':')
        })
        .collect()
}

/// Primary RED contract: building the same XMP metadata N times in a single
/// process must produce N byte-identical packets. Two sources of HashMap
/// randomness must both be neutralized: the caller-side input HashMap and
/// the internal storage HashMap inside `XmpValue::Struct/ArrayStruct`.
#[test]
fn xmp_packet_with_struct_is_byte_stable_across_independent_builds() {
    let baseline = build_with_structs().to_xmp_packet();
    for i in 1..10 {
        let again = build_with_structs().to_xmp_packet();
        if again != baseline {
            // Diff the struct-field projection so the failure message stays
            // readable even when the packet is long.
            assert_eq!(
                again,
                baseline,
                "build #{i} produced a different XMP packet — \
                 non-deterministic Struct/ArrayStruct iteration. \
                 Baseline struct-field lines:\n{:#?}\n\nDivergent struct-field lines:\n{:#?}",
                struct_field_lines(&baseline),
                struct_field_lines(&again),
            );
        }
    }
}

/// Anchor the deterministic contract to its semantics: within a single
/// struct, field elements must be emitted in lexicographic name order.
/// Guards against future refactors that "restore determinism" via a
/// non-canonical scheme (e.g. insertion-order Vec that happens to be stable
/// but reorders on cheap permutations of the caller's input).
#[test]
fn xmp_struct_fields_are_sorted_by_name() {
    let xmp = build_with_structs();
    let packet = xmp.to_xmp_packet();

    // Pull field names from struct-field lines. Each line shape (struct
    // fields have NO namespace prefix; they are emitted by `serialize_value`
    // with the raw field name as the tag):
    //   <fieldName>value</fieldName>
    let field_names: Vec<String> = struct_field_lines(&packet)
        .iter()
        .filter_map(|line| {
            let trimmed = line.trim();
            let open_lt = trimmed.find('<')?;
            let close_gt = trimmed.find('>')?;
            if close_gt <= open_lt + 1 {
                return None;
            }
            let tag = &trimmed[open_lt + 1..close_gt];
            // Tag may end at the first whitespace if attributes follow.
            let end = tag.find(char::is_whitespace).unwrap_or(tag.len());
            Some(tag[..end].to_string())
        })
        .collect();

    assert!(
        field_names.len() >= 5,
        "fixture must emit >= 5 struct-field elements — got {} ({:?})",
        field_names.len(),
        field_names
    );

    // The fields are split across multiple structs (DerivedFrom + 2x History
    // event). Verify each contiguous run of field names corresponding to a
    // single struct is sorted independently — concretely: check that within
    // every run of consecutive identical "context", the field names are
    // sorted. The simplest check that captures intent: take the first 5
    // field names (DerivedFrom: 5 fields) and assert they are sorted.
    let derived_from: Vec<&String> = field_names.iter().take(5).collect();
    let mut sorted: Vec<&String> = derived_from.clone();
    sorted.sort();
    assert_eq!(
        derived_from, sorted,
        "first struct's fields must be in lexicographic name order — \
         got {:?}, expected {:?}",
        derived_from, sorted
    );
}
