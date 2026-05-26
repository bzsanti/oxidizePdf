//! Non-Identity CID encoding CMap (`code → CID`) for Type0 text extraction.
//! See docs/superpowers/specs/2026-05-25-cid-encoding-cmap-design.md.

use std::collections::HashMap;
use std::sync::OnceLock;

use crate::parser::ParseResult;
use crate::text::cmap::{tokenize_cmap, CodeRange, Token};

/// A CID encoding CMap: maps character codes (1–2 bytes, variable width per
/// the codespace) to CIDs. Distinct from `CMap` (ToUnicode), whose
/// destinations are Unicode hex strings.
#[derive(Debug, Clone, Default)]
pub(crate) struct EncodingCMap {
    pub codespace_ranges: Vec<CodeRange>,
    pub single_cid: HashMap<Vec<u8>, u16>,
    pub cid_ranges: Vec<CidRange>,
    pub notdef_ranges: Vec<CidRange>,
    /// CIDSystemInfo Ordering from the CMap (informational, not used in decode path).
    #[allow(dead_code)]
    pub ordering: Option<String>,
    /// Parent CMap name from `usecmap` (informational, not followed at runtime).
    #[allow(dead_code)]
    pub usecmap_parent: Option<String>,
    /// Writing mode (0 = horizontal, 1 = vertical). Reserved for future use.
    #[allow(dead_code)]
    pub wmode: u8,
}

#[derive(Debug, Clone)]
pub(crate) struct CidRange {
    pub lo: Vec<u8>,
    pub hi: Vec<u8>,
    pub base_cid: u16,
}

impl EncodingCMap {
    /// Parse the codespace ranges, usecmap parent, cidchar and cidrange entries.
    pub fn parse(data: &[u8]) -> ParseResult<Self> {
        let content = String::from_utf8_lossy(data);
        let tokens = tokenize_cmap(&content);
        let mut cmap = EncodingCMap::default();
        let mut i = 0;
        while i < tokens.len() {
            match &tokens[i] {
                Token::Keyword(k) if k == "begincodespacerange" => {
                    i += 1;
                    while i < tokens.len() {
                        match &tokens[i] {
                            Token::Keyword(k) if k == "endcodespacerange" => {
                                i += 1;
                                break;
                            }
                            Token::Hex(lo) => {
                                if let Some(Token::Hex(hi)) = tokens.get(i + 1) {
                                    cmap.codespace_ranges.push(CodeRange {
                                        start: lo.clone(),
                                        end: hi.clone(),
                                    });
                                    i += 2;
                                } else {
                                    i += 1;
                                }
                            }
                            _ => i += 1,
                        }
                    }
                }
                Token::Keyword(k) if k == "usecmap" => {
                    let mut j = i;
                    while j > 0 {
                        j -= 1;
                        if let Token::Name(p) = &tokens[j] {
                            cmap.usecmap_parent = Some(p.clone());
                            break;
                        }
                    }
                    i += 1;
                }
                Token::Keyword(k) if k == "begincidchar" => {
                    i += 1;
                    while i < tokens.len() {
                        match &tokens[i] {
                            Token::Keyword(k) if k == "endcidchar" => {
                                i += 1;
                                break;
                            }
                            Token::Hex(code) => {
                                if let Some(Token::Integer(cid)) = tokens.get(i + 1) {
                                    cmap.single_cid.insert(code.clone(), *cid as u16);
                                    i += 2;
                                } else {
                                    i += 1;
                                }
                            }
                            _ => i += 1,
                        }
                    }
                }
                Token::Keyword(k) if k == "begincidrange" => {
                    i += 1;
                    while i < tokens.len() {
                        match &tokens[i] {
                            Token::Keyword(k) if k == "endcidrange" => {
                                i += 1;
                                break;
                            }
                            Token::Hex(lo) => match (tokens.get(i + 1), tokens.get(i + 2)) {
                                (Some(Token::Hex(hi)), Some(Token::Integer(cid))) => {
                                    cmap.cid_ranges.push(CidRange {
                                        lo: lo.clone(),
                                        hi: hi.clone(),
                                        base_cid: *cid as u16,
                                    });
                                    i += 3;
                                }
                                _ => i += 1,
                            },
                            _ => i += 1,
                        }
                    }
                }
                Token::Keyword(k) if k == "beginnotdefchar" => {
                    i += 1;
                    while i < tokens.len() {
                        match &tokens[i] {
                            Token::Keyword(k) if k == "endnotdefchar" => {
                                i += 1;
                                break;
                            }
                            Token::Hex(code) => {
                                if let Some(Token::Integer(cid)) = tokens.get(i + 1) {
                                    cmap.notdef_ranges.push(CidRange {
                                        lo: code.clone(),
                                        hi: code.clone(),
                                        base_cid: *cid as u16,
                                    });
                                    i += 2;
                                } else {
                                    i += 1;
                                }
                            }
                            _ => i += 1,
                        }
                    }
                }
                Token::Keyword(k) if k == "beginnotdefrange" => {
                    i += 1;
                    while i < tokens.len() {
                        match &tokens[i] {
                            Token::Keyword(k) if k == "endnotdefrange" => {
                                i += 1;
                                break;
                            }
                            Token::Hex(lo) => match (tokens.get(i + 1), tokens.get(i + 2)) {
                                (Some(Token::Hex(hi)), Some(Token::Integer(cid))) => {
                                    cmap.notdef_ranges.push(CidRange {
                                        lo: lo.clone(),
                                        hi: hi.clone(),
                                        base_cid: *cid as u16,
                                    });
                                    i += 3;
                                }
                                _ => i += 1,
                            },
                            _ => i += 1,
                        }
                    }
                }
                _ => i += 1,
            }
        }
        Ok(cmap)
    }

    /// Resolve a code that falls in a notdef range to its notdef CID.
    pub fn map_notdef(&self, code: &[u8]) -> Option<u16> {
        for r in &self.notdef_ranges {
            if code.len() == r.lo.len()
                && code.len() == r.hi.len()
                && code >= &r.lo[..]
                && code <= &r.hi[..]
            {
                return Some(r.base_cid);
            }
        }
        None
    }

    /// Determine the byte width of the code starting at `pos` by matching the
    /// first byte against codespace ranges (ISO 32000-1 §9.7.6.2). Falls back
    /// to width 1 when no range matches, guaranteeing forward progress.
    ///
    /// # Panics
    /// Panics if `pos >= bytes.len()`. Callers iterate `while pos < bytes.len()`.
    pub fn code_len_at(&self, bytes: &[u8], pos: usize) -> usize {
        let b = bytes[pos];
        for r in &self.codespace_ranges {
            if !r.start.is_empty()
                && r.start.len() == r.end.len()
                && b >= r.start[0]
                && b <= r.end[0]
            {
                return r.start.len();
            }
        }
        1
    }

    /// Map a character code to its CID. `single_cid` first, then `cid_ranges`.
    pub fn map_code_to_cid(&self, code: &[u8]) -> Option<u16> {
        if let Some(&cid) = self.single_cid.get(code) {
            return Some(cid);
        }
        for r in &self.cid_ranges {
            if code.len() == r.lo.len()
                && code.len() == r.hi.len()
                && code >= &r.lo[..]
                && code <= &r.hi[..]
            {
                let offset = be_offset(code, &r.lo);
                return r.base_cid.checked_add(offset);
            }
        }
        None
    }
}

/// Big-endian numeric distance `code - lo`, truncated to the low 16 bits.
/// For well-formed CID ranges (codes ≤ 2 bytes) the distance is ≤ 0xFFFF,
/// so the mask is a no-op.
fn be_offset(code: &[u8], lo: &[u8]) -> u16 {
    let to_u64 = |b: &[u8]| b.iter().fold(0u64, |acc, &x| (acc << 8) | x as u64);
    (to_u64(code).saturating_sub(to_u64(lo)) & 0xFFFF) as u16
}

/// The resolved, non-Identity encoding of a Type0 font, as carried on `FontInfo`.
#[derive(Debug, Clone)]
pub(crate) enum CidEncoding {
    /// `Uni*-UCS2-*` / `Uni*-UTF16-*`: the code IS a UTF-16BE value.
    Utf16Be,
    /// An embedded stream CMap or a vendored predefined CMap (code → CID).
    Cmap(EncodingCMap),
}

/// Decode a byte string as UTF-16BE, replacing malformed units with U+FFFD.
/// A trailing odd byte is dropped (no complete code unit can be formed from it).
pub(crate) fn decode_utf16be(bytes: &[u8]) -> String {
    char::decode_utf16(
        bytes
            .chunks(2)
            .filter(|c| c.len() == 2)
            .map(|c| u16::from_be_bytes([c[0], c[1]])),
    )
    .map(|r| r.unwrap_or('\u{FFFD}'))
    .collect()
}

/// Lazily parse a vendored Adobe CMap embedded at compile time. Parsed once,
/// cached for the process lifetime. Returns `None` only if the embedded data
/// fails to parse (should never happen for the shipped files).
macro_rules! vendored_cmap {
    ($file:literal) => {{
        static CELL: OnceLock<Option<EncodingCMap>> = OnceLock::new();
        CELL.get_or_init(|| {
            EncodingCMap::parse(include_bytes!(concat!("cmap_resources/", $file))).ok()
        })
        .clone()
        .map(CidEncoding::Cmap)
    }};
}

/// Resolve a predefined `/Encoding` name. `Uni*-UCS2-*`/`Uni*-UTF16-*` are
/// algorithmic UTF-16BE. Vendored CJK names resolve to lazily-parsed
/// Adobe predefined CMaps (BSD-3-Clause, embedded at compile time).
/// Unknown names return `None` (caller falls back to current behavior).
///
/// Note: the `starts_with("Uni")` check is case-sensitive per PDF spec
/// (predefined CMap names are case-sensitive, ISO 32000-1 §9.7.5.2).
pub(crate) fn resolve_predefined(name: &str) -> Option<CidEncoding> {
    if name.starts_with("Uni") && (name.contains("UCS2") || name.contains("UTF16")) {
        return Some(CidEncoding::Utf16Be);
    }
    match name {
        "GBK-EUC-H" => vendored_cmap!("GBK-EUC-H"),
        "GBKp-EUC-H" => vendored_cmap!("GBKp-EUC-H"),
        "90ms-RKSJ-H" => vendored_cmap!("90ms-RKSJ-H"),
        "90pv-RKSJ-H" => vendored_cmap!("90pv-RKSJ-H"),
        "KSCms-UHC-H" => vendored_cmap!("KSCms-UHC-H"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cidchar_and_cidrange_map_to_cids() {
        let data = b"begincmap\n\
1 begincodespacerange <0000> <FFFF> endcodespacerange\n\
1 begincidchar <0041> 100 endcidchar\n\
1 begincidrange <0061> <0063> 200 endcidrange\n\
endcmap";
        let cmap = EncodingCMap::parse(data).expect("parse");
        assert_eq!(
            cmap.map_code_to_cid(&[0x00, 0x41]),
            Some(100),
            "cidchar exact"
        );
        assert_eq!(
            cmap.map_code_to_cid(&[0x00, 0x61]),
            Some(200),
            "cidrange base"
        );
        assert_eq!(
            cmap.map_code_to_cid(&[0x00, 0x62]),
            Some(201),
            "cidrange +1"
        );
        assert_eq!(
            cmap.map_code_to_cid(&[0x00, 0x63]),
            Some(202),
            "cidrange end"
        );
        assert_eq!(cmap.map_code_to_cid(&[0x00, 0x64]), None, "outside range");
    }

    #[test]
    fn gbk_codespace_yields_mixed_widths() {
        // GBK-EUC-H codespace: single-byte <00>..<80>, double-byte <8140>..<FEFE>.
        let cmap = EncodingCMap {
            codespace_ranges: vec![
                CodeRange {
                    start: vec![0x00],
                    end: vec![0x80],
                },
                CodeRange {
                    start: vec![0x81, 0x40],
                    end: vec![0xFE, 0xFE],
                },
            ],
            ..Default::default()
        };
        assert_eq!(cmap.code_len_at(&[0x41], 0), 1, "ASCII byte is single");
        assert_eq!(
            cmap.code_len_at(&[0x81, 0x40], 0),
            2,
            "lead byte 0x81 is double"
        );
        assert_eq!(cmap.code_len_at(&[0xFE, 0xFE], 0), 2);
    }

    #[test]
    fn parse_reads_codespace_and_usecmap_parent() {
        let data = b"begincmap\n/Foo-Base usecmap\n\
2 begincodespacerange <00> <80> <8140> <FEFE> endcodespacerange\n\
endcmap";
        let cmap = EncodingCMap::parse(data).expect("parse");
        assert_eq!(cmap.codespace_ranges.len(), 2);
        assert_eq!(cmap.code_len_at(&[0x81, 0x40], 0), 2);
        assert_eq!(cmap.usecmap_parent.as_deref(), Some("Foo-Base"));
    }

    #[test]
    fn single_cid_takes_precedence_over_overlapping_range() {
        // A cidchar entry whose code falls inside a cidrange must win.
        let data = b"begincmap\n\
1 begincodespacerange <0000> <FFFF> endcodespacerange\n\
1 begincidrange <0060> <0070> 200 endcidrange\n\
1 begincidchar <0061> 999 endcidchar\n\
endcmap";
        let cmap = EncodingCMap::parse(data).expect("parse");
        assert_eq!(
            cmap.map_code_to_cid(&[0x00, 0x61]),
            Some(999),
            "single_cid wins over range"
        );
        assert_eq!(
            cmap.map_code_to_cid(&[0x00, 0x62]),
            Some(202),
            "range still applies elsewhere"
        );
    }

    #[test]
    fn notdefrange_maps_to_notdef_cid() {
        let data = b"begincmap\n\
1 begincodespacerange <0000> <FFFF> endcodespacerange\n\
1 beginnotdefrange <0000> <001F> 0 endnotdefrange\n\
endcmap";
        let cmap = EncodingCMap::parse(data).expect("parse");
        assert_eq!(cmap.map_notdef(&[0x00, 0x10]), Some(0));
        assert_eq!(cmap.map_notdef(&[0x00, 0x41]), None);
    }

    #[test]
    fn notdefchar_maps_to_notdef_cid() {
        let data = b"begincmap\n\
1 begincodespacerange <0000> <FFFF> endcodespacerange\n\
1 beginnotdefchar <0041> 7 endnotdefchar\n\
endcmap";
        let cmap = EncodingCMap::parse(data).expect("parse");
        assert_eq!(cmap.map_notdef(&[0x00, 0x41]), Some(7));
        assert_eq!(cmap.map_notdef(&[0x00, 0x42]), None);
    }

    #[test]
    fn utf16be_decodes_bmp_and_surrogates() {
        // U+4E2D (中) then U+1F600 (😀, surrogate pair D83D DE00).
        let bytes = [0x4E, 0x2D, 0xD8, 0x3D, 0xDE, 0x00];
        assert_eq!(decode_utf16be(&bytes), "中😀");
    }

    #[test]
    fn utf16be_drops_trailing_odd_byte() {
        // U+4E2D (中) followed by a lone orphan byte that must be dropped.
        let bytes = [0x4E, 0x2D, 0xFF];
        assert_eq!(decode_utf16be(&bytes), "中");
    }

    #[test]
    fn predefined_uni_families_resolve_to_utf16be() {
        assert!(matches!(
            resolve_predefined("UniGB-UCS2-H"),
            Some(CidEncoding::Utf16Be)
        ));
        assert!(matches!(
            resolve_predefined("UniJIS-UTF16-H"),
            Some(CidEncoding::Utf16Be)
        ));
        assert!(matches!(
            resolve_predefined("UniKS-UTF16-H"),
            Some(CidEncoding::Utf16Be)
        ));
        assert!(matches!(
            resolve_predefined("UniCNS-UCS2-H"),
            Some(CidEncoding::Utf16Be)
        ));
        assert!(resolve_predefined("WhateverUnknown-H").is_none());
    }

    #[test]
    fn gbk_euc_h_loads_and_maps_ascii_and_cjk() {
        let enc = match resolve_predefined("GBK-EUC-H") {
            Some(CidEncoding::Cmap(c)) => c,
            other => panic!("expected vendored Cmap, got {other:?}"),
        };
        // GBK-EUC-H codespace: <00>..<80> (single-byte) and <8140>..<FEFE> (double-byte).
        assert_eq!(enc.code_len_at(&[0x41], 0), 1, "ASCII is single-byte");
        assert_eq!(
            enc.code_len_at(&[0x81, 0x40], 0),
            2,
            "GBK lead byte is double"
        );
        // ASCII 'A' (0x41) maps to GB1 CID 846 (GBK-EUC-H cidrange <21>..<7e>).
        assert_eq!(enc.map_code_to_cid(&[0x41]), Some(846));
        // First GBK double-byte code <8140> maps to GB1 CID 10072.
        assert_eq!(enc.map_code_to_cid(&[0x81, 0x40]), Some(10072));
    }

    #[test]
    fn adversarial_input_terminates_without_hang() {
        // Stray close delimiters and dangling ranges must not loop forever.
        for data in [
            b">>>".as_slice(),
            b"begincmap\n1 begincidrange <0041>".as_slice(),
            b"]]] endcidchar beginnotdefrange".as_slice(),
            b"beginnotdefchar <0041>".as_slice(),
            b"begincidchar <0041> 5 begincidrange <00".as_slice(),
        ] {
            let _ = EncodingCMap::parse(data).expect("must terminate, not hang");
        }
    }
}
