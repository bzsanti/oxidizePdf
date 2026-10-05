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
    /// Writing mode declared by `/WMode` (0 horizontal, 1 vertical).
    pub wmode: u8,
    /// Registry/Ordering certified by the pinned predefined resource.
    /// Missing metadata does not assert a different collection.
    pub(crate) collection: Option<(String, String)>,
    /// Explicit malformed or conflicting metadata cannot authorize Unicode.
    pub(crate) invalid_collection: bool,
    pub codespace_ranges: Vec<CodeRange>,
    // CID and number of ranges already declared: later ranges may override it.
    pub single_cid: HashMap<Vec<u8>, (u16, usize)>,
    pub cid_ranges: Vec<CidRange>,
    pub notdef_ranges: Vec<CidRange>,
    /// Parent name from `usecmap`, resolved by the document-aware font parser.
    pub usecmap_parent: Option<String>,
    pub(crate) parent: Option<Box<EncodingCMap>>,
    pub(crate) explicit_wmode: bool,
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
        super::cmap_collection::apply(data, &mut cmap);
        let mut i = 0;
        while i < tokens.len() {
            match &tokens[i] {
                Token::Name(name) if name == "WMode" => {
                    if let Some(Token::Integer(value)) = tokens.get(i + 1) {
                        cmap.wmode = (*value).clamp(0, 1) as u8;
                        cmap.explicit_wmode = true;
                    }
                    i += 1;
                }
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
                                    if let Ok(cid) = u16::try_from(*cid) {
                                        cmap.single_cid
                                            .insert(code.clone(), (cid, cmap.cid_ranges.len()));
                                    }
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
                                    if let Ok(cid) = u16::try_from(*cid) {
                                        cmap.cid_ranges.push(CidRange {
                                            lo: lo.clone(),
                                            hi: hi.clone(),
                                            base_cid: cid,
                                        });
                                    }
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
                                    if let Ok(cid) = u16::try_from(*cid) {
                                        cmap.notdef_ranges.push(CidRange {
                                            lo: code.clone(),
                                            hi: code.clone(),
                                            base_cid: cid,
                                        });
                                    }
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
                                    if let Ok(cid) = u16::try_from(*cid) {
                                        cmap.notdef_ranges.push(CidRange {
                                            lo: lo.clone(),
                                            hi: hi.clone(),
                                            base_cid: cid,
                                        });
                                    }
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
        for r in self.notdef_ranges.iter().rev() {
            if code.len() == r.lo.len()
                && code.len() == r.hi.len()
                && code >= &r.lo[..]
                && code <= &r.hi[..]
            {
                return Some(r.base_cid);
            }
        }
        self.parent
            .as_ref()
            .and_then(|parent| parent.map_notdef(code))
    }

    /// Determine the byte width of the code starting at `pos` by matching the
    /// available prefix against every byte of codespace ranges (§9.7.6.2). Falls back
    /// to width 1 when no range matches, guaranteeing forward progress.
    ///
    /// # Panics
    /// Panics if `pos > bytes.len()`. Callers iterate `while pos < bytes.len()`.
    pub fn code_len_at(&self, bytes: &[u8], pos: usize) -> usize {
        let remaining = &bytes[pos..];
        let mut truncated = None;
        for r in &self.codespace_ranges {
            if !r.start.is_empty()
                && r.start.len() == r.end.len()
                && remaining
                    .iter()
                    .zip(r.start.iter().zip(&r.end))
                    .all(|(byte, (lo, hi))| byte >= lo && byte <= hi)
            {
                if remaining.len() >= r.start.len() {
                    return r.start.len();
                }
                truncated = Some(r.start.len());
            }
        }
        truncated.unwrap_or(1)
    }

    /// Later declarations override earlier mappings within the same layer.
    pub fn map_code_to_cid(&self, code: &[u8]) -> Option<u16> {
        let single = self.single_cid.get(code);
        let first_range = single.map_or(0, |(_, ranges)| *ranges);
        for r in self.cid_ranges[first_range..].iter().rev() {
            if code.len() == r.lo.len()
                && code.len() == r.hi.len()
                && code >= &r.lo[..]
                && code <= &r.hi[..]
            {
                let offset = be_offset(code, &r.lo)?;
                return r.base_cid.checked_add(offset);
            }
        }
        if let Some(&(cid, _)) = single {
            return Some(cid);
        }
        self.parent
            .as_ref()
            .and_then(|parent| parent.map_code_to_cid(code))
    }

    /// Keep layers separate so a child range overrides a parent single entry.
    pub(crate) fn inherit(&mut self, parent: Self) {
        if self.codespace_ranges.is_empty() {
            self.codespace_ranges.clone_from(&parent.codespace_ranges);
        }
        if !self.explicit_wmode {
            self.wmode = parent.wmode;
        }
        self.invalid_collection |= parent.invalid_collection;
        if let (Some(child), Some(parent)) = (&self.collection, &parent.collection) {
            self.invalid_collection |= child != parent;
        }
        if self.collection.is_none() {
            self.collection.clone_from(&parent.collection);
        }
        self.parent = Some(Box::new(parent));
    }

    /// Reconcile an explicit declaration without letting malformed metadata
    /// disappear into the distinct, historically supported absent case.
    pub(crate) fn declare_collection(&mut self, collection: Option<(String, String)>) {
        let Some(collection) = collection else {
            self.invalid_collection = true;
            return;
        };
        if self
            .collection
            .as_ref()
            .is_some_and(|old| old != &collection)
        {
            self.invalid_collection = true;
        }
        self.collection = Some(collection);
    }

    pub(crate) fn identity(vertical: bool) -> Self {
        Self {
            wmode: u8::from(vertical),
            codespace_ranges: vec![CodeRange {
                start: vec![0, 0],
                end: vec![255, 255],
            }],
            cid_ranges: vec![CidRange {
                lo: vec![0, 0],
                hi: vec![255, 255],
                base_cid: 0,
            }],
            ..Self::default()
        }
    }
}

/// CID offsets must fit in 16 bits even when source codes use three/four bytes.
fn be_offset(code: &[u8], lo: &[u8]) -> Option<u16> {
    if code.len() != lo.len() || !(1..=4).contains(&code.len()) {
        return None;
    }
    let to_u32 = |b: &[u8]| b.iter().fold(0u32, |acc, &x| (acc << 8) | x as u32);
    u16::try_from(to_u32(code).checked_sub(to_u32(lo))?).ok()
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
    let mut result: String = char::decode_utf16(
        bytes
            .chunks_exact(2)
            .map(|c| u16::from_be_bytes([c[0], c[1]])),
    )
    .map(|r| r.unwrap_or('\u{FFFD}'))
    .collect();
    if bytes.len() % 2 != 0 {
        result.push('\u{FFFD}');
    }
    result
}

/// Lazily parse a vendored Adobe CMap embedded at compile time. Parsed once,
/// cached for the process lifetime. Returns `None` only if the embedded data
/// fails to parse (should never happen for the shipped files).
macro_rules! vendored_cmap {
    ($file:literal, $ordering:literal) => {{
        static CELL: OnceLock<Option<EncodingCMap>> = OnceLock::new();
        CELL.get_or_init(|| {
            let mut cmap =
                EncodingCMap::parse(include_bytes!(concat!("cmap_resources/", $file))).ok()?;
            if let Some(parent) = cmap.usecmap_parent.as_deref() {
                match resolve_predefined(parent)? {
                    CidEncoding::Cmap(parent) => cmap.inherit(parent),
                    CidEncoding::Utf16Be => return None,
                }
            }
            cmap.collection = Some(("Adobe".into(), $ordering.into()));
            Some(cmap)
        })
        .clone()
        .map(CidEncoding::Cmap)
    }};
}

/// Resolve a predefined Encoding through pinned Adobe code→CID resources.
/// Vertical resources inherit their horizontal parent. Unvendored Uni* names
/// retain the historical UTF-16 recovery; other unknown names return None.
pub(crate) fn resolve_predefined(name: &str) -> Option<CidEncoding> {
    match name {
        "90ms-RKSJ-H" => vendored_cmap!("90ms-RKSJ-H", "Japan1"),
        "90ms-RKSJ-V" => vendored_cmap!("90ms-RKSJ-V", "Japan1"),
        "90pv-RKSJ-H" => vendored_cmap!("90pv-RKSJ-H", "Japan1"),
        "90pv-RKSJ-V" => vendored_cmap!("90pv-RKSJ-V", "Japan1"),
        "B5pc-H" => vendored_cmap!("B5pc-H", "CNS1"),
        "B5pc-V" => vendored_cmap!("B5pc-V", "CNS1"),
        "ETen-B5-H" => vendored_cmap!("ETen-B5-H", "CNS1"),
        "ETen-B5-V" => vendored_cmap!("ETen-B5-V", "CNS1"),
        "GB-EUC-H" => vendored_cmap!("GB-EUC-H", "GB1"),
        "GB-EUC-V" => vendored_cmap!("GB-EUC-V", "GB1"),
        "GBK-EUC-H" => vendored_cmap!("GBK-EUC-H", "GB1"),
        "GBK-EUC-V" => vendored_cmap!("GBK-EUC-V", "GB1"),
        "GBKp-EUC-H" => vendored_cmap!("GBKp-EUC-H", "GB1"),
        "GBKp-EUC-V" => vendored_cmap!("GBKp-EUC-V", "GB1"),
        "KSC-EUC-H" => vendored_cmap!("KSC-EUC-H", "Korea1"),
        "KSC-EUC-V" => vendored_cmap!("KSC-EUC-V", "Korea1"),
        "KSCms-UHC-H" => vendored_cmap!("KSCms-UHC-H", "Korea1"),
        "KSCms-UHC-V" => vendored_cmap!("KSCms-UHC-V", "Korea1"),
        "UniAKR-UTF16-H" => vendored_cmap!("UniAKR-UTF16-H", "KR"),
        "Adobe-KR-0" => vendored_cmap!("Adobe-KR-0", "KR"),
        "Adobe-KR-1" => vendored_cmap!("Adobe-KR-1", "KR"),
        "Adobe-KR-2" => vendored_cmap!("Adobe-KR-2", "KR"),
        "Adobe-KR-3" => vendored_cmap!("Adobe-KR-3", "KR"),
        "Adobe-KR-4" => vendored_cmap!("Adobe-KR-4", "KR"),
        "Adobe-KR-5" => vendored_cmap!("Adobe-KR-5", "KR"),
        "Adobe-KR-6" => vendored_cmap!("Adobe-KR-6", "KR"),
        "Adobe-KR-7" => vendored_cmap!("Adobe-KR-7", "KR"),
        "Adobe-KR-8" => vendored_cmap!("Adobe-KR-8", "KR"),
        "Adobe-KR-9" => vendored_cmap!("Adobe-KR-9", "KR"),
        "UniAKR-UTF8-H" => vendored_cmap!("UniAKR-UTF8-H", "KR"),
        "UniAKR-UTF32-H" => vendored_cmap!("UniAKR-UTF32-H", "KR"),
        "UniCNS-UCS2-H" => vendored_cmap!("UniCNS-UCS2-H", "CNS1"),
        "UniCNS-UCS2-V" => vendored_cmap!("UniCNS-UCS2-V", "CNS1"),
        "UniCNS-UTF16-H" => vendored_cmap!("UniCNS-UTF16-H", "CNS1"),
        "UniCNS-UTF16-V" => vendored_cmap!("UniCNS-UTF16-V", "CNS1"),
        "UniGB-UCS2-H" => vendored_cmap!("UniGB-UCS2-H", "GB1"),
        "UniGB-UCS2-V" => vendored_cmap!("UniGB-UCS2-V", "GB1"),
        "UniGB-UTF16-H" => vendored_cmap!("UniGB-UTF16-H", "GB1"),
        "UniGB-UTF16-V" => vendored_cmap!("UniGB-UTF16-V", "GB1"),
        "UniJIS-UCS2-H" => vendored_cmap!("UniJIS-UCS2-H", "Japan1"),
        "UniJIS-UCS2-V" => vendored_cmap!("UniJIS-UCS2-V", "Japan1"),
        "UniJIS-UTF16-H" => vendored_cmap!("UniJIS-UTF16-H", "Japan1"),
        "UniJIS-UTF16-V" => vendored_cmap!("UniJIS-UTF16-V", "Japan1"),
        "UniKS-UCS2-H" => vendored_cmap!("UniKS-UCS2-H", "Korea1"),
        "UniKS-UCS2-V" => vendored_cmap!("UniKS-UCS2-V", "Korea1"),
        "UniKS-UTF16-H" => vendored_cmap!("UniKS-UTF16-H", "Korea1"),
        "UniKS-UTF16-V" => vendored_cmap!("UniKS-UTF16-V", "Korea1"),
        // Preserve the historical Unicode recovery for unvendored Uni* names.
        _ if name.starts_with("Uni") && (name.contains("UCS2") || name.contains("UTF16")) => {
            Some(CidEncoding::Utf16Be)
        }
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
    fn parse_retains_vertical_writing_mode() {
        let cmap = EncodingCMap::parse(
            b"begincmap\n/WMode 1 def\n1 begincodespacerange <0000> <FFFF> endcodespacerange\nendcmap",
        )
        .expect("parse");
        assert_eq!(cmap.wmode, 1);
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
    fn utf16be_marks_trailing_odd_byte() {
        // #678: keep an incomplete code visible instead of silently dropping it.
        let bytes = [0x4E, 0x2D, 0xFF];
        assert_eq!(decode_utf16be(&bytes), "中�");
    }

    #[test]
    fn predefined_uni_families_resolve_to_collection_cmaps() {
        assert!(matches!(
            resolve_predefined("UniGB-UCS2-H"),
            Some(CidEncoding::Cmap(_))
        ));
        assert!(matches!(
            resolve_predefined("UniJIS-UTF16-H"),
            Some(CidEncoding::Cmap(_))
        ));
        assert!(matches!(
            resolve_predefined("UniKS-UTF16-H"),
            Some(CidEncoding::Cmap(_))
        ));
        assert!(matches!(
            resolve_predefined("UniCNS-UCS2-H"),
            Some(CidEncoding::Cmap(_))
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
