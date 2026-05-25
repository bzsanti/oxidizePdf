//! Non-Identity CID encoding CMap (`code → CID`) for Type0 text extraction.
//! See docs/superpowers/specs/2026-05-25-cid-encoding-cmap-design.md.

use crate::parser::ParseResult;
use crate::text::cmap::{tokenize_cmap, CodeRange, Token};

/// A CID encoding CMap: maps character codes (1–2 bytes, variable width per
/// the codespace) to CIDs. Distinct from `CMap` (ToUnicode), whose
/// destinations are Unicode hex strings.
#[allow(dead_code)]
#[derive(Debug, Clone, Default)]
pub(crate) struct EncodingCMap {
    pub codespace_ranges: Vec<CodeRange>,
    pub single_cid: std::collections::HashMap<Vec<u8>, u16>,
    pub cid_ranges: Vec<CidRange>,
    pub notdef_ranges: Vec<CidRange>,
    pub ordering: Option<String>,
    pub usecmap_parent: Option<String>,
    pub wmode: u8,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub(crate) struct CidRange {
    pub lo: Vec<u8>,
    pub hi: Vec<u8>,
    pub base_cid: u16,
}

#[allow(dead_code)]
impl EncodingCMap {
    /// Parse the codespace ranges and the usecmap parent. Mapping entries
    /// (cidchar/cidrange/notdef) are added in later tasks.
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
                _ => i += 1,
            }
        }
        Ok(cmap)
    }

    /// Determine the byte width of the code starting at `pos` by matching the
    /// first byte against codespace ranges (ISO 32000-1 §9.7.6.2). Falls back
    /// to width 1 when no range matches, guaranteeing forward progress.
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
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
