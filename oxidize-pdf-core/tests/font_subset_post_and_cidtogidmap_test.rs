//! TDD tests for Issue #165 follow-up fixes:
//!
//! - **Bug #1**: `post` table must be rewritten as version 3.0 (32 bytes) in
//!   TTF subsets. Previously the subsetter copied the original font's `post`
//!   table verbatim, which for CJK fonts means ~370 KB of glyph names that
//!   PDF never consults.
//! - **Bug #2**: the CIDToGIDMap stream (CIDFontType2 only) must be
//!   FlateDecode-compressed. Previously it was emitted as raw binary which
//!   for high-codepoint characters added ~130 KB of sparse zero-filled map.
//!
//! Tests use the real `Roboto-Regular.ttf` fixture (post v2, 3387 glyph names)
//! so they exercise the exact code path the user reported, without requiring
//! a CJK TTF fixture that we don't ship in the repo.

use oxidize_pdf::text::fonts::truetype_subsetter::subset_font;
use oxidize_pdf::{Document, Font, Page};
use std::collections::HashSet;

const ROBOTO_PATH: &str = "../test-pdfs/Roboto-Regular.ttf";

fn load_fixture(path: &str) -> Option<Vec<u8>> {
    std::fs::read(path)
        .map_err(|_| eprintln!("SKIPPED: {} not found", path))
        .ok()
}

// =============================================================================
// Minimal SFNT / PDF helpers (test-local; duplicating here keeps tests
// self-contained without exposing parser internals).
// =============================================================================

fn u16_be(b: &[u8], off: usize) -> u16 {
    u16::from_be_bytes([b[off], b[off + 1]])
}

fn u32_be(b: &[u8], off: usize) -> u32 {
    u32::from_be_bytes([b[off], b[off + 1], b[off + 2], b[off + 3]])
}

/// Locate a table in a SFNT (TTF/OTF) byte buffer. Returns (offset, length).
fn find_sfnt_table(font: &[u8], tag: &[u8; 4]) -> Option<(usize, usize)> {
    if font.len() < 12 {
        return None;
    }
    let num_tables = u16_be(font, 4) as usize;
    for i in 0..num_tables {
        let entry = 12 + i * 16;
        if entry + 16 > font.len() {
            return None;
        }
        if &font[entry..entry + 4] == tag {
            let offset = u32_be(font, entry + 8) as usize;
            let length = u32_be(font, entry + 12) as usize;
            return Some((offset, length));
        }
    }
    None
}

fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || needle.len() > haystack.len() {
        return None;
    }
    (0..=haystack.len() - needle.len()).find(|&i| &haystack[i..i + needle.len()] == needle)
}

/// Locate object `N 0 obj ... endobj` and return the (header_start, stream_start,
/// stream_end, endobj_end) offsets plus the dict body slice.
struct PdfObjectRef<'a> {
    dict_body: &'a [u8],
    stream: Option<&'a [u8]>,
}

fn locate_object<'a>(pdf: &'a [u8], obj_num: u32) -> Option<PdfObjectRef<'a>> {
    let needle = format!("{} 0 obj", obj_num);
    let start = find_subslice(pdf, needle.as_bytes())?;
    let header_end = start + needle.len();
    let endobj_rel = find_subslice(&pdf[header_end..], b"endobj")?;
    let obj_end = header_end + endobj_rel;
    let obj_bytes = &pdf[header_end..obj_end];

    // Locate dict bounds: first `<<` to matching `>>` (top-level).
    let dict_start = find_subslice(obj_bytes, b"<<")?;
    // Walk to find matching >>
    let mut depth = 1usize;
    let mut i = dict_start + 2;
    while i + 1 < obj_bytes.len() {
        if &obj_bytes[i..i + 2] == b"<<" {
            depth += 1;
            i += 2;
        } else if &obj_bytes[i..i + 2] == b">>" {
            depth -= 1;
            i += 2;
            if depth == 0 {
                break;
            }
        } else {
            i += 1;
        }
    }
    if depth != 0 {
        return None;
    }
    let dict_body = &obj_bytes[dict_start + 2..i - 2];

    // Optional stream
    let stream = find_subslice(&obj_bytes[i..], b"stream").map(|rel| {
        let mut s = i + rel + b"stream".len();
        if obj_bytes.get(s) == Some(&b'\r') {
            s += 1;
        }
        if obj_bytes.get(s) == Some(&b'\n') {
            s += 1;
        }
        let e = find_subslice(&obj_bytes[s..], b"endstream").unwrap_or(obj_bytes.len() - s);
        // Trim trailing EOL
        let mut end = s + e;
        while end > s && (obj_bytes[end - 1] == b'\n' || obj_bytes[end - 1] == b'\r') {
            end -= 1;
        }
        &obj_bytes[s..end]
    });

    Some(PdfObjectRef { dict_body, stream })
}

// =============================================================================
// Bug #1: post table must be version 3.0 (32 bytes) in subset output
// =============================================================================

/// When subsetting Roboto-Regular.ttf (post v2, 3387 glyph names, ~37 KB),
/// the output subset font MUST emit `post` version 3.0 (header-only, 32 bytes).
/// PDF does not use glyph names from the post table — rendering is resolved
/// through ToUnicode + CIDToGIDMap — so carrying the Pascal name strings is
/// pure bloat. For CJK fonts (56K glyphs) the impact is ~370 KB of waste.
#[test]
fn test_ttf_subset_emits_post_version_3_0() {
    let font_data = match load_fixture(ROBOTO_PATH) {
        Some(d) => d,
        None => return,
    };

    let used: HashSet<char> = "ABC".chars().collect();
    let result = subset_font(font_data.clone(), &used).expect("subsetting must succeed");

    // Confirm pre-condition on the fixture: original font has post v2 with
    // many glyph names. If someone swaps the fixture for a post-v3 font this
    // test becomes vacuous, so assert the pre-condition explicitly.
    let (orig_post_off, orig_post_len) =
        find_sfnt_table(&font_data, b"post").expect("Roboto must have a post table");
    let orig_version = u32_be(&font_data, orig_post_off);
    assert_eq!(
        orig_version, 0x00020000,
        "fixture sanity: Roboto post must be version 2.0 for the test to be meaningful"
    );
    assert!(
        orig_post_len > 10_000,
        "fixture sanity: Roboto post is expected to be >10 KB with glyph names, got {} bytes",
        orig_post_len
    );

    // Subset output must carry a post table...
    let (post_off, post_len) =
        find_sfnt_table(&result.font_data, b"post").expect("subset must include post table");

    // ...of version 3.0...
    let version = u32_be(&result.font_data, post_off);
    assert_eq!(
        version, 0x00030000,
        "subset post table must be version 3.0 (header-only), got 0x{:08x}",
        version
    );

    // ...and header-only (32 bytes, no numGlyphs / glyphNameIndex / Pascal names).
    assert_eq!(
        post_len, 32,
        "subset post table (v3.0) must be exactly 32 bytes, got {}",
        post_len
    );
}

/// Guard against regressions that would copy the original `post` into the
/// subset. This test is deliberately strict on size: any attempt to embed
/// per-glyph name data would blow past 64 bytes.
#[test]
fn test_ttf_subset_post_table_strictly_bounded() {
    let font_data = match load_fixture(ROBOTO_PATH) {
        Some(d) => d,
        None => return,
    };

    let used: HashSet<char> = "Hello".chars().collect();
    let result = subset_font(font_data, &used).expect("subsetting must succeed");
    let (_, post_len) =
        find_sfnt_table(&result.font_data, b"post").expect("post table must be present");

    assert!(
        post_len <= 64,
        "subset post table must fit in <= 64 bytes (v3.0 header-only), got {}",
        post_len
    );
}

// =============================================================================
// Bug #2: CIDToGIDMap stream must be FlateDecode-compressed
// =============================================================================

#[cfg(feature = "compression")]
#[test]
fn test_cid_to_gid_map_stream_has_flate_filter() {
    let font_data = match load_fixture(ROBOTO_PATH) {
        Some(d) => d,
        None => return,
    };

    let mut doc = Document::new();
    doc.add_font_from_bytes("Roboto", font_data)
        .expect("add_font_from_bytes must succeed");
    let mut page = Page::a4();
    page.text()
        .set_font(Font::Custom("Roboto".to_string()), 12.0)
        .at(50.0, 500.0)
        .write("AB")
        .expect("writing must succeed");
    doc.add_page(page);

    let pdf_bytes = doc.to_bytes().expect("PDF generation must succeed");

    // Find the CIDFont dict: it contains `/Subtype /CIDFontType2` and
    // `/CIDToGIDMap N 0 R`. Resolve that N and inspect its stream dict.
    let key = b"/CIDToGIDMap";
    let key_pos = find_subslice(&pdf_bytes, key).expect("CIDToGIDMap must be referenced");
    let tail = &pdf_bytes[key_pos + key.len()..];
    // Skip whitespace
    let mut cursor = 0;
    while cursor < tail.len() && tail[cursor].is_ascii_whitespace() {
        cursor += 1;
    }
    // If the value is `/Identity`, the map has been inlined as a name and the
    // test is not applicable — skip. In practice Roboto + ASCII produces a
    // small binary map, not Identity.
    if tail[cursor..].starts_with(b"/Identity") {
        eprintln!("SKIPPED: CIDToGIDMap is /Identity, no stream to compress");
        return;
    }
    let start = cursor;
    while cursor < tail.len() && tail[cursor].is_ascii_digit() {
        cursor += 1;
    }
    let obj_num: u32 = std::str::from_utf8(&tail[start..cursor])
        .expect("ascii digits")
        .parse()
        .expect("parseable object number");

    let obj = locate_object(&pdf_bytes, obj_num).expect("CIDToGIDMap target object must exist");

    assert!(
        find_subslice(obj.dict_body, b"/Filter").is_some()
            && find_subslice(obj.dict_body, b"/FlateDecode").is_some(),
        "CIDToGIDMap stream dictionary must declare /Filter /FlateDecode; \
         got dict body: {:?}",
        std::str::from_utf8(obj.dict_body).unwrap_or("<non-utf8>")
    );

    let stream = obj.stream.expect("CIDToGIDMap must be a stream object");
    assert!(!stream.is_empty(), "CIDToGIDMap stream must not be empty");
}

/// After Flate-compressing a CIDToGIDMap that is mostly zeros (only a handful
/// of codepoints have non-zero glyph IDs), the on-disk stream must be at
/// least an order of magnitude smaller than the declared uncompressed size.
///
/// The uncompressed map is (max_codepoint + 1) * 2 bytes. For Roboto + a few
/// ASCII chars the uncompressed size is small, so this test uses a codepoint
/// set that forces a larger uncompressed map (Latin-1 Supplement) and asserts
/// that Flate compression leaves it below 10% of the uncompressed size.
#[cfg(feature = "compression")]
#[test]
fn test_cid_to_gid_map_compression_ratio_below_10_percent() {
    let font_data = match load_fixture(ROBOTO_PATH) {
        Some(d) => d,
        None => return,
    };

    let mut doc = Document::new();
    doc.add_font_from_bytes("Roboto", font_data)
        .expect("add_font_from_bytes must succeed");
    let mut page = Page::a4();
    page.text()
        .set_font(Font::Custom("Roboto".to_string()), 12.0)
        .at(50.0, 500.0)
        // ÿ (U+00FF) forces max_codepoint = 255 => uncompressed map = 512 bytes
        // of which only ~10 entries are non-zero. Flate should crush this.
        .write("Café ÿ")
        .expect("writing Latin-1 text must succeed");
    doc.add_page(page);

    let pdf_bytes = doc.to_bytes().expect("PDF generation must succeed");

    // Locate CIDToGIDMap target object
    let key_pos =
        find_subslice(&pdf_bytes, b"/CIDToGIDMap").expect("CIDToGIDMap must be referenced");
    let tail = &pdf_bytes[key_pos + b"/CIDToGIDMap".len()..];
    let mut cursor = 0;
    while cursor < tail.len() && tail[cursor].is_ascii_whitespace() {
        cursor += 1;
    }
    if tail[cursor..].starts_with(b"/Identity") {
        eprintln!("SKIPPED: CIDToGIDMap is /Identity");
        return;
    }
    let start = cursor;
    while cursor < tail.len() && tail[cursor].is_ascii_digit() {
        cursor += 1;
    }
    let obj_num: u32 = std::str::from_utf8(&tail[start..cursor])
        .expect("ascii digits")
        .parse()
        .expect("parseable object number");

    let obj = locate_object(&pdf_bytes, obj_num).expect("CIDToGIDMap target must exist");
    let stream = obj.stream.expect("must be a stream");

    // Pull the `/Length1`-free dict's `/Length` to compare against the
    // compressed stream length — but cheaper: decompress and compare.
    use flate2::read::ZlibDecoder;
    use std::io::Read;
    let mut dec = ZlibDecoder::new(stream);
    let mut decompressed = Vec::new();
    dec.read_to_end(&mut decompressed)
        .expect("stream must be valid Flate data");

    let compressed_len = stream.len() as f64;
    let uncompressed_len = decompressed.len() as f64;
    assert!(uncompressed_len > 0.0, "uncompressed CIDToGIDMap is empty");

    let ratio = compressed_len / uncompressed_len;
    assert!(
        ratio < 0.10,
        "Flate-compressed CIDToGIDMap ({} bytes) must be < 10% of uncompressed ({} bytes); ratio {:.4}",
        stream.len(),
        decompressed.len(),
        ratio
    );
}

// =============================================================================
// Combined end-to-end regression: Roboto + pangram must fit under 25 KB
// =============================================================================

/// This is the combined regression guard for Bug #1 + Bug #2.
/// Roboto-Regular.ttf is a 515 KB Latin TTF. Subset to ~35 unique characters
/// (a pangram) and embedded with both post v3.0 and FlateDecode-compressed
/// CIDToGIDMap, the output PDF must fit comfortably under 25 KB.
///
/// The previous ceiling of 50 KB (set in v2.5.3 by
/// `test_roboto_ttf_pdf_end_to_end_under_50kb`) still passes both before and
/// after these fixes because Roboto's post table is "only" ~37 KB (vs
/// ~373 KB for CJK fonts). The 25 KB bound here exercises the size budget
/// tighter and will catch either bug regressing.
#[cfg(feature = "compression")]
#[test]
fn test_roboto_ttf_pdf_end_to_end_under_25kb_combined_fixes() {
    let font_data = match load_fixture(ROBOTO_PATH) {
        Some(d) => d,
        None => return,
    };

    let text = "The quick brown fox jumps over the lazy dog.";
    let mut doc = Document::new();
    doc.add_font_from_bytes("Roboto", font_data)
        .expect("add_font_from_bytes must succeed");
    let mut page = Page::a4();
    page.text()
        .set_font(Font::Custom("Roboto".to_string()), 12.0)
        .at(50.0, 500.0)
        .write(text)
        .expect("writing text must succeed");
    doc.add_page(page);

    let pdf_bytes = doc.to_bytes().expect("PDF generation must succeed");
    assert!(
        pdf_bytes.len() < 25_000,
        "Roboto TTF PDF with post v3.0 + FlateDecode CIDToGIDMap must be under 25 KB, got {} bytes",
        pdf_bytes.len()
    );
}
