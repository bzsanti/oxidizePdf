//! CMap and ToUnicode support for text extraction
//!
//! This module implements CMap parsing and ToUnicode mappings according to
//! ISO 32000-1:2008 Section 9.10 (Extraction of Text Content) and Section 9.7.5 (CMaps).
//!
//! CMaps define the mapping from character codes to character selectors (CIDs, character names, or Unicode values).

use crate::parser::{ParseError, ParseResult};
use std::collections::HashMap;

/// CMap type enumeration
#[derive(Debug, Clone, PartialEq)]
pub enum CMapType {
    /// Maps character codes to CIDs (Character IDs)
    CIDMap,
    /// Maps character codes to Unicode values
    ToUnicode,
    /// Predefined CMap (e.g., Identity-H, Identity-V)
    Predefined(String),
}

/// Character code range mapping
#[derive(Debug, Clone)]
pub struct CodeRange {
    /// Start of the code range
    pub start: Vec<u8>,
    /// End of the code range
    pub end: Vec<u8>,
}

impl CodeRange {
    /// Check if a code is within this range
    pub fn contains(&self, code: &[u8]) -> bool {
        if code.len() != self.start.len() || code.len() != self.end.len() {
            return false;
        }

        code >= &self.start[..] && code <= &self.end[..]
    }
}

/// CMap mapping entry
#[derive(Debug, Clone)]
pub enum CMapEntry {
    /// Single character mapping
    Single {
        /// Source character code
        src: Vec<u8>,
        /// Destination (CID or Unicode)
        dst: Vec<u8>,
    },
    /// Range mapping
    Range {
        /// Start of source range
        src_start: Vec<u8>,
        /// End of source range
        src_end: Vec<u8>,
        /// Start of destination range
        dst_start: Vec<u8>,
    },
}

/// CMap structure for character code mappings
#[derive(Debug, Clone)]
pub struct CMap {
    /// CMap name
    pub name: Option<String>,
    /// CMap type
    pub cmap_type: CMapType,
    /// Writing mode (0 = horizontal, 1 = vertical)
    pub wmode: u8,
    /// Code space ranges
    pub codespace_ranges: Vec<CodeRange>,
    /// Character mappings
    pub mappings: Vec<CMapEntry>,
    /// Cached single mappings for fast lookup
    single_mappings: HashMap<Vec<u8>, Vec<u8>>,
    /// Predefined parent CMap inherited via `usecmap`. When set to
    /// `"Identity-H"` or `"Identity-V"`, `map()` falls back to
    /// returning the input code as-is for any code the child CMap
    /// did not map explicitly, and `is_valid_code()` accepts the full
    /// 2-byte (Identity-H) or 1-byte (Identity-V) space. External
    /// CMap chaining (non-predefined parents) is recorded for
    /// observability but does not enable any fallback.
    pub inherited_predefined: Option<String>,
}

impl Default for CMap {
    fn default() -> Self {
        Self::new()
    }
}

impl CMap {
    /// Create a new empty CMap
    pub fn new() -> Self {
        Self {
            name: None,
            cmap_type: CMapType::ToUnicode,
            wmode: 0,
            codespace_ranges: Vec::new(),
            mappings: Vec::new(),
            single_mappings: HashMap::new(),
            inherited_predefined: None,
        }
    }

    /// Create a predefined Identity CMap
    pub fn identity_h() -> Self {
        Self {
            name: Some("Identity-H".to_string()),
            cmap_type: CMapType::Predefined("Identity-H".to_string()),
            wmode: 0,
            codespace_ranges: vec![CodeRange {
                start: vec![0x00, 0x00],
                end: vec![0xFF, 0xFF],
            }],
            mappings: Vec::new(),
            single_mappings: HashMap::new(),
            inherited_predefined: None,
        }
    }

    /// Create a predefined Identity-V CMap
    pub fn identity_v() -> Self {
        Self {
            name: Some("Identity-V".to_string()),
            cmap_type: CMapType::Predefined("Identity-V".to_string()),
            wmode: 1,
            codespace_ranges: vec![CodeRange {
                start: vec![0x00, 0x00],
                end: vec![0xFF, 0xFF],
            }],
            mappings: Vec::new(),
            single_mappings: HashMap::new(),
            inherited_predefined: None,
        }
    }

    /// Parse a CMap from data.
    ///
    /// Adobe CMaps are PostScript, not line-oriented. Same-line forms
    /// like `1 begincodespacerange <0000><00D1> endcodespacerange` are
    /// legal (and shipped by BOE / various other producers — see issue
    /// #272). The previous line-based scanner could get stuck in a state
    /// flag because `endcodespacerange` never appeared as its own line.
    /// This implementation tokenises the input first and then consumes
    /// tokens with a state machine that is whitespace-agnostic.
    pub fn parse(data: &[u8]) -> ParseResult<Self> {
        let mut cmap = Self::new();
        let content =
            std::str::from_utf8(data).map_err(|e| ParseError::CharacterEncodingError {
                position: 0,
                message: format!("Invalid UTF-8 in CMap: {e}"),
            })?;

        let tokens = tokenize_cmap(content);
        let mut i = 0;

        while i < tokens.len() {
            match &tokens[i] {
                Token::Name(n) if n == "CMapName" => {
                    if let Some(Token::Name(name)) = tokens.get(i + 1) {
                        cmap.name = Some(name.clone());
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
                // `usecmap` directive: the immediately preceding Name token
                // (e.g. `/Identity-H usecmap`) names the parent CMap whose
                // mappings the child inherits. For the two predefined
                // Identity CMaps the codebase can synthesise (Identity-H,
                // Identity-V), this enables an identity fallback in `map()`
                // for codes the child doesn't explicitly cover. External
                // CMap names are recorded but produce no fallback (a real
                // chain resolver would need access to the document's CMap
                // resources, which this parser cannot reach).
                Token::Keyword(k) if k == "usecmap" => {
                    let mut j = i;
                    while j > 0 {
                        j -= 1;
                        if let Token::Name(parent) = &tokens[j] {
                            cmap.inherited_predefined = Some(parent.clone());
                            break;
                        }
                    }
                    i += 1;
                }
                Token::Name(n) if n == "WMode" => {
                    if let Some(Token::Integer(w)) = tokens.get(i + 1) {
                        cmap.wmode = *w as u8;
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
                Token::Keyword(k) if k == "begincodespacerange" => {
                    i += 1;
                    while i < tokens.len() {
                        match &tokens[i] {
                            Token::Keyword(k) if k == "endcodespacerange" => {
                                i += 1;
                                break;
                            }
                            Token::Hex(start) => {
                                if let Some(Token::Hex(end)) = tokens.get(i + 1) {
                                    cmap.codespace_ranges.push(CodeRange {
                                        start: start.clone(),
                                        end: end.clone(),
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
                Token::Keyword(k) if k == "beginbfchar" => {
                    i += 1;
                    while i < tokens.len() {
                        match &tokens[i] {
                            Token::Keyword(k) if k == "endbfchar" => {
                                i += 1;
                                break;
                            }
                            Token::Hex(src) => {
                                if let Some(Token::Hex(dst)) = tokens.get(i + 1) {
                                    cmap.single_mappings.insert(src.clone(), dst.clone());
                                    cmap.mappings.push(CMapEntry::Single {
                                        src: src.clone(),
                                        dst: dst.clone(),
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
                Token::Keyword(k) if k == "beginbfrange" => {
                    i += 1;
                    while i < tokens.len() {
                        match &tokens[i] {
                            Token::Keyword(k) if k == "endbfrange" => {
                                i += 1;
                                break;
                            }
                            Token::Hex(src_start) => {
                                // Need src_end + (Hex dst_start | Array of dst)
                                let src_end = match tokens.get(i + 1) {
                                    Some(Token::Hex(e)) => e.clone(),
                                    _ => {
                                        i += 1;
                                        continue;
                                    }
                                };
                                match tokens.get(i + 2) {
                                    Some(Token::Hex(dst_start)) => {
                                        cmap.mappings.push(CMapEntry::Range {
                                            src_start: src_start.clone(),
                                            src_end,
                                            dst_start: dst_start.clone(),
                                        });
                                        i += 3;
                                    }
                                    Some(Token::Array(dsts)) => {
                                        // Array form: each dst replaces one src code,
                                        // walking src_start..=src_end in lockstep. The
                                        // increment is big-endian with carry — wrapping
                                        // only the last byte (`<00FE> + 1 = <0000>`) would
                                        // silently insert into the wrong slot when the
                                        // range crosses a byte boundary.
                                        let mut current_src = src_start.clone();
                                        for dst in dsts {
                                            cmap.single_mappings
                                                .insert(current_src.clone(), dst.clone());
                                            cmap.mappings.push(CMapEntry::Single {
                                                src: current_src.clone(),
                                                dst: dst.clone(),
                                            });
                                            if current_src.as_slice() >= src_end.as_slice() {
                                                break;
                                            }
                                            increment_be(&mut current_src);
                                        }
                                        i += 3;
                                    }
                                    _ => {
                                        i += 1;
                                    }
                                }
                            }
                            _ => i += 1,
                        }
                    }
                }
                // Top-level fall-through covers PostScript constructs the
                // state machine deliberately ignores: the integer operand
                // count before `begin*` keywords (`14 beginbfchar`), the
                // `def` / `dict` / `findresource` / `begincmap` / `endcmap`
                // boilerplate, and any unrecognised name token. Dropping
                // these keeps the parser robust against producer-specific
                // headers without coupling the state machine to them.
                _ => i += 1,
            }
        }

        Ok(cmap)
    }

    /// Map a character code to its destination
    pub fn map(&self, code: &[u8]) -> Option<Vec<u8>> {
        // Check if code is in valid codespace
        if !self.is_valid_code(code) {
            return None;
        }

        // For predefined Identity CMaps
        if let CMapType::Predefined(name) = &self.cmap_type {
            if name.starts_with("Identity") {
                return Some(code.to_vec());
            }
        }

        // Check single mappings first (cached)
        if let Some(dst) = self.single_mappings.get(code) {
            return Some(dst.clone());
        }

        // Check range mappings
        for mapping in &self.mappings {
            if let CMapEntry::Range {
                src_start,
                src_end,
                dst_start,
            } = mapping
            {
                if code.len() == src_start.len() && code >= &src_start[..] && code <= &src_end[..] {
                    // Calculate offset within range
                    let offset = calculate_offset(code, src_start);
                    let mut result = dst_start.clone();

                    // Add offset to the destination treating it as a
                    // big-endian multi-byte integer, propagating carry.
                    let mut carry = offset;
                    for byte in result.iter_mut().rev() {
                        let sum = *byte as usize + carry;
                        *byte = (sum & 0xFF) as u8;
                        carry = sum >> 8;
                        if carry == 0 {
                            break;
                        }
                    }

                    return Some(result);
                }
            }
        }

        // Identity fallback inherited via `usecmap`. If the child CMap
        // didn't map this code explicitly and the parent is Identity-H
        // or Identity-V (both 2-byte CID encodings), pass the code
        // through unchanged. The downstream `to_unicode` then interprets
        // the bytes as UTF-16BE.
        if code.len() == 2 && self.identity_inherited() {
            return Some(code.to_vec());
        }

        None
    }

    /// Check if a code is in valid codespace
    pub fn is_valid_code(&self, code: &[u8]) -> bool {
        for range in &self.codespace_ranges {
            if range.contains(code) {
                return true;
            }
        }
        // No explicit codespace covers this code, but `usecmap`
        // inheritance from a predefined Identity CMap means the full
        // 2-byte space is valid.
        if code.len() == 2
            && (self.inherited_predefined_is("Identity-H")
                || self.inherited_predefined_is("Identity-V"))
        {
            return true;
        }
        false
    }

    /// If this CMap inherits (via `usecmap`) from a predefined Adobe
    /// `*-UCS2` CMap, return the matching CID collection ordering.
    /// Used by ToUnicode decoding to resolve codes the child CMap did
    /// not map explicitly (the code is treated as a CID into the table).
    pub(crate) fn inherited_ordering(&self) -> Option<&'static str> {
        match self.inherited_predefined.as_deref()? {
            "Adobe-GB1-UCS2" => Some("GB1"),
            "Adobe-CNS1-UCS2" => Some("CNS1"),
            "Adobe-Japan1-UCS2" => Some("Japan1"),
            "Adobe-Korea1-UCS2" | "Adobe-KR-UCS2" => Some("Korea1"),
            _ => None,
        }
    }

    /// `true` iff this CMap inherits identity-mapping semantics from a
    /// predefined parent via `usecmap`.
    fn identity_inherited(&self) -> bool {
        self.inherited_predefined_is("Identity-H") || self.inherited_predefined_is("Identity-V")
    }

    /// `true` iff the inherited parent (set by `usecmap`) matches the
    /// given predefined CMap name exactly.
    fn inherited_predefined_is(&self, name: &str) -> bool {
        self.inherited_predefined
            .as_deref()
            .map(|p| p == name)
            .unwrap_or(false)
    }

    /// Convert mapped value to Unicode string
    pub fn to_unicode(&self, mapped: &[u8]) -> Option<String> {
        match self.cmap_type {
            CMapType::ToUnicode => {
                // Interpret as UTF-16BE
                if mapped.len() % 2 == 0 {
                    let utf16_values: Vec<u16> = mapped
                        .chunks(2)
                        .map(|chunk| u16::from_be_bytes([chunk[0], chunk[1]]))
                        .collect();
                    String::from_utf16(&utf16_values).ok()
                } else {
                    // Try as UTF-8
                    String::from_utf8(mapped.to_vec()).ok()
                }
            }
            _ => None,
        }
    }
}

/// Increment a big-endian byte sequence by 1 in place, propagating carry
/// across byte boundaries. Returns `true` on success, `false` on
/// overflow (e.g. `<FFFF> + 1`). Used by the `bfrange` array form to
/// walk `src_start..=src_end` in lockstep with the dst array.
fn increment_be(bytes: &mut [u8]) -> bool {
    let mut carry = 1u32;
    for byte in bytes.iter_mut().rev() {
        let sum = *byte as u32 + carry;
        *byte = (sum & 0xFF) as u8;
        carry = sum >> 8;
        if carry == 0 {
            return true;
        }
    }
    carry == 0
}

/// Parse hex string `<...>` bytes into a `Vec<u8>`. Whitespace inside
/// the angle brackets is permitted (PostScript allows it); odd-length
/// strings or non-hex characters return `None`.
fn parse_hex(s: &str) -> Option<Vec<u8>> {
    let s = s.trim_start_matches('<').trim_end_matches('>');
    let clean: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    if clean.len() % 2 != 0 {
        return None;
    }

    let mut bytes = Vec::new();
    for i in (0..clean.len()).step_by(2) {
        if let Ok(byte) = u8::from_str_radix(&clean[i..i + 2], 16) {
            bytes.push(byte);
        } else {
            return None;
        }
    }
    Some(bytes)
}

/// A single PostScript token extracted from a CMap stream. Only the
/// shapes the CMap state machine consumes are represented; everything
/// else (dictionary `<<...>>` markers, literal strings, unknown
/// keywords) is either skipped at tokenisation time or ignored by the
/// state machine.
#[derive(Debug, Clone)]
enum Token {
    /// Hex string `<00D1>` → `vec![0x00, 0xD1]`.
    Hex(Vec<u8>),
    /// Array `[ <abcd> <ef01> ... ]` of hex strings, used by the
    /// `beginbfrange` array form `<srcStart> <srcEnd> [<dst0> <dst1> ...]`.
    Array(Vec<Vec<u8>>),
    /// PostScript name `/CMapName`, `/WMode`, etc. — the leading `/`
    /// is stripped.
    Name(String),
    /// Decimal integer such as the operand count before `begin*` or the
    /// `0` in `/WMode 0`.
    Integer(i64),
    /// Bare identifier such as `begincmap`, `endbfchar`, `def`. We treat
    /// every non-delimited identifier as a keyword and let the parser
    /// state machine pick the ones it cares about.
    Keyword(String),
}

/// Tokenise a CMap PostScript stream into [`Token`]s. The scanner is
/// whitespace-agnostic so that minified CMaps (`begin... <a><b> end...`
/// all on one line, BOE-style) are parsed identically to the multi-line
/// canonical form. Unknown PostScript constructs (literal strings,
/// `<<` ... `>>` dictionaries, comments) are silently skipped.
fn tokenize_cmap(content: &str) -> Vec<Token> {
    let bytes = content.as_bytes();
    let mut tokens = Vec::new();
    let mut i = 0;

    while i < bytes.len() {
        let b = bytes[i];

        // Whitespace
        if b.is_ascii_whitespace() {
            i += 1;
            continue;
        }

        // Comments: `% ... \n`
        if b == b'%' {
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
            continue;
        }

        // Dictionary `<<` / `>>` — skip the markers, the state machine
        // does not need dict contents.
        if b == b'<' && bytes.get(i + 1) == Some(&b'<') {
            i += 2;
            continue;
        }
        if b == b'>' && bytes.get(i + 1) == Some(&b'>') {
            i += 2;
            continue;
        }

        // Hex string `<...>`. Bail out resiliently if the closing `>` is
        // absent OR if a stray `<` appears before it (meaning the
        // original `<` was unterminated and we are about to greedily
        // consume the *next* valid hex string instead). Skipping just
        // the lone byte preserves any following well-formed mappings.
        if b == b'<' {
            let start = i + 1;
            let mut end_pos: Option<usize> = None;
            for (off, &c) in bytes[start..].iter().enumerate() {
                if c == b'>' {
                    end_pos = Some(start + off);
                    break;
                }
                if c == b'<' {
                    // Another `<` arrived before `>` — original is malformed.
                    break;
                }
            }
            if let Some(end) = end_pos {
                let inner: String = bytes[start..end].iter().map(|&c| c as char).collect();
                if let Some(decoded) = parse_hex(&inner) {
                    tokens.push(Token::Hex(decoded));
                }
                i = end + 1;
                continue;
            } else {
                i += 1;
                continue;
            }
        }

        // Array of hex strings `[ <...> <...> ... ]`
        if b == b'[' {
            i += 1;
            let mut values = Vec::new();
            while i < bytes.len() {
                let bb = bytes[i];
                if bb.is_ascii_whitespace() {
                    i += 1;
                } else if bb == b']' {
                    i += 1;
                    break;
                } else if bb == b'<' {
                    let start = i + 1;
                    if let Some(rel) = bytes[start..].iter().position(|&c| c == b'>') {
                        let end = start + rel;
                        let inner: String = bytes[start..end].iter().map(|&c| c as char).collect();
                        if let Some(decoded) = parse_hex(&inner) {
                            values.push(decoded);
                        }
                        i = end + 1;
                    } else {
                        break;
                    }
                } else {
                    // Skip non-hex content inside arrays (defensive: PostScript
                    // permits other token types but bfrange arrays in practice
                    // only contain hex strings).
                    i += 1;
                }
            }
            tokens.push(Token::Array(values));
            continue;
        }

        // Literal string `( ... )` — skipped. PostScript supports balanced
        // parens and `\` escapes; CMaps only use these inside CIDSystemInfo
        // which the state machine doesn't read.
        if b == b'(' {
            let mut depth = 1;
            i += 1;
            while i < bytes.len() && depth > 0 {
                match bytes[i] {
                    b'\\' if i + 1 < bytes.len() => i += 2,
                    b'(' => {
                        depth += 1;
                        i += 1;
                    }
                    b')' => {
                        depth -= 1;
                        i += 1;
                    }
                    _ => i += 1,
                }
            }
            continue;
        }

        // PostScript name `/ident`
        if b == b'/' {
            let start = i + 1;
            let end = bytes[start..]
                .iter()
                .position(|&c| {
                    c.is_ascii_whitespace()
                        || c == b'<'
                        || c == b'>'
                        || c == b'/'
                        || c == b'['
                        || c == b']'
                        || c == b'('
                        || c == b')'
                        || c == b'%'
                })
                .map(|p| start + p)
                .unwrap_or(bytes.len());
            if end > start {
                let name: String = bytes[start..end].iter().map(|&c| c as char).collect();
                tokens.push(Token::Name(name));
            }
            i = end;
            continue;
        }

        // Integer (decimal). Negative numbers permitted with leading `-`.
        if b.is_ascii_digit()
            || (b == b'-'
                && bytes
                    .get(i + 1)
                    .map(|c| c.is_ascii_digit())
                    .unwrap_or(false))
        {
            let start = i;
            i += 1;
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                i += 1;
            }
            let s: String = bytes[start..i].iter().map(|&c| c as char).collect();
            if let Ok(n) = s.parse::<i64>() {
                tokens.push(Token::Integer(n));
            }
            continue;
        }

        // Keyword: bare identifier (everything until whitespace / delimiter).
        let start = i;
        while i < bytes.len() {
            let c = bytes[i];
            if c.is_ascii_whitespace()
                || c == b'<'
                || c == b'>'
                || c == b'/'
                || c == b'['
                || c == b']'
                || c == b'('
                || c == b')'
                || c == b'%'
            {
                break;
            }
            i += 1;
        }
        if i > start {
            let kw: String = bytes[start..i].iter().map(|&c| c as char).collect();
            tokens.push(Token::Keyword(kw));
        } else {
            // The byte at `i` is a stray close-delimiter that no earlier
            // branch consumed: a lone `>` (not `>>`), or an unmatched `]`
            // or `)`. The keyword loop above `break`s on it immediately
            // without advancing, so we must skip it explicitly to
            // guarantee forward progress. Without this, the tokeniser
            // spins forever on such a byte (regression seen on
            // pdf.js corpus issue11651.pdf).
            i += 1;
        }
    }

    tokens
}

/// Calculate the offset between two big-endian byte sequences of equal length.
///
/// Both inputs are interpreted as unsigned big-endian integers and the
/// difference is returned as `usize`. Caller must ensure `code >= start`
/// (checked in `map_code`); this function saturates to 0 if not, to avoid
/// panicking on malformed input.
///
/// The naive byte-by-byte subtraction is wrong when any single byte
/// position has `code[i] < start[i]` (which is legal as long as the overall
/// big-endian value is still `>=`) — it underflows. Reducing each side to
/// its integer value first avoids the issue.
fn calculate_offset(code: &[u8], start: &[u8]) -> usize {
    let code_val: usize = code.iter().fold(0, |acc, &b| acc * 256 + b as usize);
    let start_val: usize = start.iter().fold(0, |acc, &b| acc * 256 + b as usize);
    code_val.saturating_sub(start_val)
}

/// ToUnicode CMap builder for creating custom mappings
#[derive(Debug, Clone)]
pub struct ToUnicodeCMapBuilder {
    /// Character to Unicode mappings
    mappings: HashMap<Vec<u8>, String>,
    /// Code length in bytes
    code_length: usize,
}

impl ToUnicodeCMapBuilder {
    /// Create a new ToUnicode CMap builder
    pub fn new(code_length: usize) -> Self {
        Self {
            mappings: HashMap::new(),
            code_length,
        }
    }

    /// Add a character mapping
    pub fn add_mapping(&mut self, char_code: Vec<u8>, unicode: &str) {
        self.mappings.insert(char_code, unicode.to_string());
    }

    /// Add a mapping from a single byte code
    pub fn add_single_byte_mapping(&mut self, char_code: u8, unicode: char) {
        let code = if self.code_length == 1 {
            vec![char_code]
        } else {
            // Pad with zeros for multi-byte codes
            let mut code = vec![0; self.code_length - 1];
            code.push(char_code);
            code
        };
        self.mappings.insert(code, unicode.to_string());
    }

    /// Build the ToUnicode CMap content
    pub fn build(&self) -> Vec<u8> {
        let mut content = String::new();

        // CMap header
        content.push_str("/CIDInit /ProcSet findresource begin\n");
        content.push_str("12 dict begin\n");
        content.push_str("begincmap\n");
        content.push_str("/CIDSystemInfo\n");
        content.push_str("<< /Registry (Adobe)\n");
        content.push_str("   /Ordering (UCS)\n");
        content.push_str("   /Supplement 0\n");
        content.push_str(">> def\n");
        content.push_str("/CMapName /Adobe-Identity-UCS def\n");
        content.push_str("/CMapType 2 def\n");

        // Code space range
        content.push_str("1 begincodespacerange\n");
        if self.code_length == 1 {
            content.push_str("<00> <FF>\n");
        } else {
            let start = vec![0x00; self.code_length];
            let end = vec![0xFF; self.code_length];
            content.push_str(&format!(
                "<{}> <{}>\n",
                hex_string(&start),
                hex_string(&end)
            ));
        }
        content.push_str("endcodespacerange\n");

        // Character mappings
        if !self.mappings.is_empty() {
            // Group mappings by consecutive ranges
            let mut sorted_mappings: Vec<_> = self.mappings.iter().collect();
            sorted_mappings.sort_by_key(|(k, _)| *k);

            // Output single character mappings
            let mut single_mappings = Vec::new();
            for (code, unicode) in &sorted_mappings {
                let utf16_bytes = string_to_utf16_be_bytes(unicode);
                single_mappings.push((code, utf16_bytes));
            }

            // Write bfchar mappings in chunks of 100
            for chunk in single_mappings.chunks(100) {
                content.push_str(&format!("{} beginbfchar\n", chunk.len()));
                for (code, unicode_bytes) in chunk {
                    content.push_str(&format!(
                        "<{}> <{}>\n",
                        hex_string(code),
                        hex_string(unicode_bytes)
                    ));
                }
                content.push_str("endbfchar\n");
            }
        }

        // CMap footer
        content.push_str("endcmap\n");
        content.push_str("CMapName currentdict /CMap defineresource pop\n");
        content.push_str("end\n");
        content.push_str("end\n");

        content.into_bytes()
    }
}

/// Convert string to UTF-16BE bytes
pub fn string_to_utf16_be_bytes(s: &str) -> Vec<u8> {
    let mut bytes = Vec::new();
    for ch in s.encode_utf16() {
        bytes.extend(&ch.to_be_bytes());
    }
    bytes
}

/// Convert bytes to hex string
pub fn hex_string(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02X}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_code_range() {
        let range = CodeRange {
            start: vec![0x00],
            end: vec![0xFF],
        };

        assert!(range.contains(&[0x00]));
        assert!(range.contains(&[0x80]));
        assert!(range.contains(&[0xFF]));
        assert!(!range.contains(&[0x00, 0x00])); // Wrong length
    }

    #[test]
    fn test_identity_cmap() {
        let cmap = CMap::identity_h();
        assert_eq!(cmap.name, Some("Identity-H".to_string()));
        assert_eq!(cmap.wmode, 0);

        // Identity mapping returns the same code
        let code = vec![0x00, 0x41];
        assert_eq!(cmap.map(&code), Some(code.clone()));
    }

    #[test]
    fn test_parse_hex() {
        assert_eq!(parse_hex("<00>"), Some(vec![0x00]));
        assert_eq!(parse_hex("<FF>"), Some(vec![0xFF]));
        assert_eq!(parse_hex("<0041>"), Some(vec![0x00, 0x41]));
        assert_eq!(parse_hex("<FEFF>"), Some(vec![0xFE, 0xFF]));
        assert_eq!(parse_hex("invalid"), None);
    }

    #[test]
    fn test_calculate_offset() {
        assert_eq!(calculate_offset(&[0x00, 0x05], &[0x00, 0x00]), 5);
        assert_eq!(calculate_offset(&[0x01, 0x00], &[0x00, 0x00]), 256);
        assert_eq!(calculate_offset(&[0xFF], &[0x00]), 255);
    }

    /// Regression: the byte-by-byte subtraction underflowed whenever
    /// code[i] < start[i] in any single byte position, even though
    /// `code >= start` in the big-endian sense. This triggered panics
    /// extracting text from PDFs with CJK punctuation (e.g. U+3001 `、`
    /// in a ToUnicode bfrange spanning U+2FFF → U+3002).
    #[test]
    fn test_calculate_offset_with_byte_borrow() {
        // 0x0100 − 0x00FF = 1 (individual byte 0x00 < 0xFF → borrow)
        assert_eq!(calculate_offset(&[0x01, 0x00], &[0x00, 0xFF]), 1);
        // 0x3001 − 0x2FFF = 2 (real-world CJK punctuation case)
        assert_eq!(calculate_offset(&[0x30, 0x01], &[0x2F, 0xFF]), 2);
        // 0xFF02 − 0xFEFF = 3 (high byte stays equal, low byte wraps)
        assert_eq!(calculate_offset(&[0xFF, 0x02], &[0xFE, 0xFF]), 3);
        // Same start and end → zero offset.
        assert_eq!(calculate_offset(&[0x12, 0x34], &[0x12, 0x34]), 0);
    }

    #[test]
    fn test_tounicode_builder() {
        let mut builder = ToUnicodeCMapBuilder::new(1);
        builder.add_single_byte_mapping(0x41, 'A');
        builder.add_single_byte_mapping(0x42, 'B');

        let content = builder.build();
        let content_str = String::from_utf8(content).unwrap();

        assert!(content_str.contains("/CMapName /Adobe-Identity-UCS def"));
        assert!(content_str.contains("begincodespacerange"));
        assert!(content_str.contains("<00> <FF>"));
        assert!(content_str.contains("beginbfchar"));
    }

    #[test]
    fn test_simple_cmap_parsing() {
        let cmap_data = br#"
%!PS-Adobe-3.0 Resource-CMap
%%DocumentNeededResources: ProcSet (CIDInit)
%%IncludeResource: ProcSet (CIDInit)
%%BeginResource: CMap (Custom)
%%Title: (Custom Adobe UCS 0)
%%Version: 1.000
%%EndComments

/CIDInit /ProcSet findresource begin
12 dict begin
begincmap
/CIDSystemInfo
<< /Registry (Adobe)
   /Ordering (UCS)
   /Supplement 0
>> def
/CMapName /Custom def
/CMapType 2 def
1 begincodespacerange
<00> <FF>
endcodespacerange
2 beginbfchar
<20> <0020>
<41> <0041>
endbfchar
endcmap
"#;

        let cmap = CMap::parse(cmap_data).unwrap();
        assert_eq!(cmap.name, Some("Custom".to_string()));
        assert_eq!(cmap.codespace_ranges.len(), 1);
        assert_eq!(cmap.map(&[0x20]), Some(vec![0x00, 0x20]));
        assert_eq!(cmap.map(&[0x41]), Some(vec![0x00, 0x41]));
    }

    #[test]
    fn test_cmap_to_unicode() {
        let mut cmap = CMap::new();
        cmap.cmap_type = CMapType::ToUnicode;

        // UTF-16BE for 'A'
        let unicode_a = vec![0x00, 0x41];
        assert_eq!(cmap.to_unicode(&unicode_a), Some("A".to_string()));

        // UTF-16BE for '中' (U+4E2D)
        let unicode_cjk = vec![0x4E, 0x2D];
        assert_eq!(cmap.to_unicode(&unicode_cjk), Some("中".to_string()));
    }

    #[test]
    fn test_bf_range_mapping() {
        let mut cmap = CMap::new();
        cmap.codespace_ranges.push(CodeRange {
            start: vec![0x00],
            end: vec![0xFF],
        });
        cmap.mappings.push(CMapEntry::Range {
            src_start: vec![0x20],
            src_end: vec![0x7E],
            dst_start: vec![0x00, 0x20],
        });

        // Test range mapping
        assert_eq!(cmap.map(&[0x20]), Some(vec![0x00, 0x20])); // Space
        assert_eq!(cmap.map(&[0x41]), Some(vec![0x00, 0x41])); // 'A'
        assert_eq!(cmap.map(&[0x7E]), Some(vec![0x00, 0x7E])); // '~'
        assert_eq!(cmap.map(&[0x7F]), None); // Out of range
    }

    #[test]
    fn test_multibyte_mapping() {
        let mut builder = ToUnicodeCMapBuilder::new(2);
        builder.add_mapping(vec![0x00, 0x41], "A");
        builder.add_mapping(vec![0x00, 0x42], "B");

        let content = builder.build();
        let content_str = String::from_utf8(content).unwrap();

        assert!(content_str.contains("<0000> <FFFF>"));
        assert!(content_str.contains("<0041>"));
        assert!(content_str.contains("<0042>"));
    }

    // ------------------------------------------------------------------
    // Issue #272 (Bug A) — minified / single-line CMap directives.
    //
    // BOE (Spanish official gazette) PDFs ship ToUnicode CMaps with
    // `begin*` and `end*` operators on the SAME line as the entries:
    //
    //   1 begincodespacerange <0000><00D1> endcodespacerange
    //
    // PostScript permits this (CMaps are tokens, not lines). The original
    // parser was line-based and got stuck in `in_codespace_range = true`
    // forever, so subsequent `beginbfchar` / `beginbfrange` lines were
    // discarded and the CMap produced 0 mappings. Encoding fallback
    // (PdfDocEncoding) then leaked each 2-byte CID as two ASCII bytes
    // ("M" → "\0" + "0" → " 0" after sanitization).
    // ------------------------------------------------------------------

    /// The actual BOE F0 CMap, verbatim from
    /// `corpus_cache/6320a941c903a04f.pdf` (Boletín Oficial del Estado,
    /// sumario 2025-01-15). Single-line `begincodespacerange ...
    /// endcodespacerange`, then multi-line bfchar and bfrange blocks.
    /// Must parse to 1 codespace, 14 bfchar singles, and 8 bfrange
    /// entries (22 mappings total).
    #[test]
    fn boe_single_line_codespacerange_parses_full_cmap() {
        let cmap_data = b"/CIDInit /ProcSet findresource begin 12 dict begin begincmap \n\
/CIDSystemInfo <</Registry (F0+0) /Ordering (F0) /Supplement 0>> def\n\
/CMapName /F0+0 def\n\
/CMapType 2 def\n\
1 begincodespacerange <0000><00D1> endcodespacerange\n\
14 beginbfchar\n\
<0000><0000>\n\
<0003><0020>\n\
<005C><0079>\n\
<0066><00D1>\n\
<0069><00E1>\n\
<0070><00E9>\n\
<0074><00ED>\n\
<0078><00F1>\n\
<0079><00F3>\n\
<007E><00FA>\n\
<00C7><00C1>\n\
<00CA><00CD>\n\
<00CE><00D3>\n\
<00D1><00DA>\n\
endbfchar\n\
8 beginbfrange\n\
<000F><001D><002C>\n\
<0024><002D><0041>\n\
<002F><0033><004C>\n\
<0035><0039><0052>\n\
<003B><003D><0058>\n\
<0044><004C><0061>\n\
<004F><0053><006C>\n\
<0055><005A><0072>\n\
endbfrange\n\
endcmap CMapName currentdict /CMap defineresource pop end end\n";

        let cmap = CMap::parse(cmap_data).expect("CMap parse must succeed");

        assert_eq!(
            cmap.codespace_ranges.len(),
            1,
            "single-line begincodespacerange must produce 1 codespace; got {} ({:?})",
            cmap.codespace_ranges.len(),
            cmap.codespace_ranges
        );
        let cs = &cmap.codespace_ranges[0];
        assert_eq!(cs.start, vec![0x00, 0x00]);
        assert_eq!(cs.end, vec![0x00, 0xD1]);

        // 14 bfchar singles + 8 bfrange entries = 22 entries.
        assert_eq!(
            cmap.mappings.len(),
            22,
            "expected 22 entries (14 bfchar + 8 bfrange); got {}",
            cmap.mappings.len()
        );

        // Concrete decoding probe: CID <0030> via bfrange <002F><0033><004C>
        // (the 'M' glyph in this font) → offset 1 → 0x004D → 'M'.
        let mapped = cmap.map(&[0x00, 0x30]).expect("CID 0030 must map");
        assert_eq!(mapped, vec![0x00, 0x4D]);
        assert_eq!(cmap.to_unicode(&mapped).as_deref(), Some("M"));

        // bfchar single: CID <0003> → <0020> (space).
        let mapped = cmap.map(&[0x00, 0x03]).expect("CID 0003 must map");
        assert_eq!(mapped, vec![0x00, 0x20]);
    }

    /// Defensive coverage for the same minification pattern on `beginbfchar`
    /// and `beginbfrange` blocks. Any of the three Adobe CMap operators
    /// may legally appear with the `end*` on the same line — the parser
    /// must handle all three uniformly.
    #[test]
    fn single_line_beginbfchar_parses() {
        let cmap_data = b"begincmap\n\
1 begincodespacerange <00><FF> endcodespacerange\n\
2 beginbfchar <20><0020> <41><0041> endbfchar\n\
endcmap\n";

        let cmap = CMap::parse(cmap_data).expect("parse");
        assert_eq!(cmap.codespace_ranges.len(), 1);
        assert_eq!(cmap.mappings.len(), 2, "got {:?}", cmap.mappings);
        assert_eq!(cmap.map(&[0x20]), Some(vec![0x00, 0x20]));
        assert_eq!(cmap.map(&[0x41]), Some(vec![0x00, 0x41]));
    }

    #[test]
    fn single_line_beginbfrange_parses() {
        let cmap_data = b"begincmap\n\
1 begincodespacerange <0000><00FF> endcodespacerange\n\
1 beginbfrange <0020><007E><0020> endbfrange\n\
endcmap\n";

        let cmap = CMap::parse(cmap_data).expect("parse");
        assert_eq!(cmap.codespace_ranges.len(), 1);
        assert_eq!(cmap.mappings.len(), 1, "got {:?}", cmap.mappings);
        // CID 0x0041 should map via the range (offset 0x21) to 0x0041 (UTF-16BE 'A').
        let mapped = cmap.map(&[0x00, 0x41]).expect("CID 0041 must map");
        assert_eq!(mapped, vec![0x00, 0x41]);
    }

    /// `bfrange` array form must use big-endian carry when incrementing
    /// `current_src` between dst entries. A naive last-byte-only increment
    /// (`<00FE> + 1 → <0000>`) would silently insert into the wrong slot
    /// whenever the range crosses a byte boundary, producing bogus
    /// mappings without any error.
    #[test]
    fn bfrange_array_form_increments_src_with_big_endian_carry() {
        // 4 dsts over <00FE>..=<0101>: the second increment crosses
        // from 0x00FF to 0x0100, which requires the carry to land in
        // the high byte.
        let cmap_data = b"begincmap\n\
1 begincodespacerange <0000><FFFF> endcodespacerange\n\
1 beginbfrange\n\
<00FE> <0101> [<0041> <0042> <0043> <0044>]\n\
endbfrange\n\
endcmap\n";

        let cmap = CMap::parse(cmap_data).expect("parse");

        // Expect exactly four Single entries at <00FE>, <00FF>, <0100>, <0101>.
        assert_eq!(
            cmap.mappings.len(),
            4,
            "expected 4 mappings across the byte boundary; got {:?}",
            cmap.mappings
        );

        // Each src code must map to the matching dst from the array.
        assert_eq!(cmap.map(&[0x00, 0xFE]), Some(vec![0x00, 0x41]));
        assert_eq!(cmap.map(&[0x00, 0xFF]), Some(vec![0x00, 0x42]));
        assert_eq!(
            cmap.map(&[0x01, 0x00]),
            Some(vec![0x00, 0x43]),
            "carry across byte boundary: <0100> must map to dsts[2] (0x0043)"
        );
        assert_eq!(cmap.map(&[0x01, 0x01]), Some(vec![0x00, 0x44]));

        // Crucially: <0000> (the would-be result of naive last-byte
        // wrap of <00FF>) must NOT have inherited dsts[2].
        assert_eq!(
            cmap.map(&[0x00, 0x00]),
            None,
            "<0000> must not be populated; naive wraparound would have put 0x0043 here"
        );
    }

    /// `increment_be` direct unit test: a focused regression guard for
    /// the carry helper used by the bfrange array form.
    #[test]
    fn increment_be_carries_across_byte_boundary() {
        let mut bytes = vec![0x00, 0xFF];
        assert!(increment_be(&mut bytes));
        assert_eq!(bytes, vec![0x01, 0x00]);

        let mut bytes = vec![0x00, 0x00, 0xFF, 0xFF];
        assert!(increment_be(&mut bytes));
        assert_eq!(bytes, vec![0x00, 0x01, 0x00, 0x00]);

        // Overflow at the top: <FFFF> + 1 returns false and wraps.
        let mut bytes = vec![0xFF, 0xFF];
        assert!(!increment_be(&mut bytes));
        assert_eq!(bytes, vec![0x00, 0x00]);
    }

    /// Mixed: some `begin*` directives on their own line, others combined.
    /// The parser must not let one form pollute the state of another.
    #[test]
    fn mixed_single_line_and_multi_line_directives_parse() {
        let cmap_data = b"begincmap\n\
1 begincodespacerange\n\
<0000><FFFF>\n\
endcodespacerange\n\
2 beginbfchar <0001><0041> <0002><0042> endbfchar\n\
1 beginbfrange\n\
<0010> <0012> <0050>\n\
endbfrange\n\
endcmap\n";

        let cmap = CMap::parse(cmap_data).expect("parse");
        assert_eq!(cmap.codespace_ranges.len(), 1);
        assert_eq!(cmap.mappings.len(), 3);
        assert_eq!(cmap.map(&[0x00, 0x01]), Some(vec![0x00, 0x41]));
        assert_eq!(cmap.map(&[0x00, 0x02]), Some(vec![0x00, 0x42]));
        assert_eq!(cmap.map(&[0x00, 0x10]), Some(vec![0x00, 0x50]));
        assert_eq!(cmap.map(&[0x00, 0x12]), Some(vec![0x00, 0x52]));
    }

    /// `bfrange` array form with an empty `[]` is legal but degenerate:
    /// it should produce zero entries and must not panic. This locks in
    /// the early-exit behaviour of the `for dst in dsts` body when the
    /// array vector is empty.
    #[test]
    fn bfrange_array_form_with_empty_array_emits_zero_mappings_no_panic() {
        let cmap_data = b"begincmap\n\
1 begincodespacerange <0000><FFFF> endcodespacerange\n\
1 beginbfrange\n\
<0010> <0012> []\n\
endbfrange\n\
endcmap\n";

        let cmap = CMap::parse(cmap_data).expect("parse must not panic on empty array");
        assert_eq!(
            cmap.mappings.len(),
            0,
            "empty bfrange array must yield zero mappings; got {:?}",
            cmap.mappings
        );
        assert_eq!(cmap.map(&[0x00, 0x10]), None);
    }

    /// Regression for an infinite loop introduced by the token-based
    /// rewrite: a stray close-delimiter (`>` that is not part of `>>`,
    /// or a lone `]` / `)`) reached the keyword branch of the tokeniser,
    /// which `break`s immediately without advancing the cursor. The
    /// scanner then spun forever on that byte. issue11651.pdf from the
    /// pdf.js corpus contained exactly such a byte in a CMap-adjacent
    /// stream and hung text extraction. The tokeniser must always make
    /// forward progress; this test must complete (not hang) and parse
    /// the surrounding valid mappings.
    #[test]
    fn stray_close_delimiters_do_not_hang_tokenizer() {
        // A lone `>`, `]`, and `)` interspersed with valid content.
        let cmap_data = b"begincmap\n\
1 begincodespacerange <0000><FFFF> endcodespacerange\n\
> ] )\n\
2 beginbfchar\n\
<0041><0061>\n\
<0042><0062>\n\
endbfchar\n\
endcmap\n";

        // The assertion that matters is that parse() RETURNS at all.
        let cmap = CMap::parse(cmap_data).expect("parse must terminate, not hang");
        assert_eq!(cmap.map(&[0x00, 0x41]), Some(vec![0x00, 0x61]));
        assert_eq!(cmap.map(&[0x00, 0x42]), Some(vec![0x00, 0x62]));
    }

    /// A CMap consisting solely of a stray `>` must terminate (empty CMap).
    #[test]
    fn lone_gt_delimiter_terminates() {
        let cmap = CMap::parse(b">").expect("must terminate");
        assert!(cmap.mappings.is_empty());
    }

    /// `usecmap` directive must inherit the codespace + identity fallback
    /// from a predefined parent (`Identity-H` / `Identity-V`). Codes
    /// that the child CMap doesn't explicitly map should pass through
    /// as their UTF-16BE code units (Identity behaviour), while
    /// explicit mappings still override.
    #[test]
    fn usecmap_identity_h_inheritance_provides_fallback() {
        let cmap_data = b"begincmap\n\
/CMapName /CustomChild def\n\
/CMapType 2 def\n\
/Identity-H usecmap\n\
1 beginbfchar\n\
<0041><0061>\n\
endbfchar\n\
endcmap\n";

        let cmap = CMap::parse(cmap_data).expect("parse");

        // Explicit override survives.
        assert_eq!(cmap.map(&[0x00, 0x41]), Some(vec![0x00, 0x61]));
        assert_eq!(cmap.to_unicode(&[0x00, 0x61]).as_deref(), Some("a"));

        // Unmapped code falls back to identity: <0042> → CID <0042>
        // → ToUnicode interprets as UTF-16BE 'B'.
        let mapped = cmap
            .map(&[0x00, 0x42])
            .expect("identity fallback must map <0042>");
        assert_eq!(mapped, vec![0x00, 0x42]);
        assert_eq!(cmap.to_unicode(&mapped).as_deref(), Some("B"));

        // Inherited codespace covers the full 2-byte range even though
        // no explicit `begincodespacerange` appears in the child CMap.
        assert!(cmap.is_valid_code(&[0x12, 0x34]));
    }

    /// `usecmap Identity-V` mirrors Identity-H semantics for vertical
    /// writing-mode CMaps. Same fallback behaviour.
    #[test]
    fn usecmap_identity_v_inheritance_provides_fallback() {
        let cmap_data = b"begincmap\n\
/Identity-V usecmap\n\
endcmap\n";

        let cmap = CMap::parse(cmap_data).expect("parse");

        // No explicit mappings, but identity fallback covers everything.
        assert_eq!(cmap.map(&[0x4E, 0x2D]), Some(vec![0x4E, 0x2D]));
        assert_eq!(
            cmap.to_unicode(&[0x4E, 0x2D]).as_deref(),
            Some("中"),
            "identity fallback should let CJK code <4E2D> decode to U+4E2D"
        );
    }

    /// A non-Identity parent (`/Foo usecmap`) must NOT enable the
    /// identity fallback. This locks in that the inheritance is only
    /// honoured for the two predefined Identity CMaps the codebase
    /// can synthesise internally — external CMap chaining is out of
    /// scope and must remain absent (no resolver yet).
    #[test]
    fn usecmap_non_identity_parent_does_not_enable_identity_fallback() {
        let cmap_data = b"begincmap\n\
/SomeOtherCMap usecmap\n\
endcmap\n";

        let cmap = CMap::parse(cmap_data).expect("parse");
        assert_eq!(
            cmap.map(&[0x00, 0x41]),
            None,
            "non-Identity usecmap must not provide identity fallback"
        );
    }

    /// Unterminated hex string (`<00FF` without `>`) used to silently
    /// truncate the entire token stream. The resilience fix keeps the
    /// scanner advancing past the lone `<` so that subsequent valid
    /// mappings still land in the CMap.
    #[test]
    fn unterminated_hex_string_does_not_discard_following_mappings() {
        // The `<00FF` (no closing `>`) appears between two valid mappings.
        // Pre-fix the entire trailing `<0042><0043> endbfchar endcmap`
        // would have been silently dropped; post-fix only the lone `<` is
        // skipped and the second mapping survives.
        let cmap_data = b"begincmap\n\
1 begincodespacerange <0000><FFFF> endcodespacerange\n\
2 beginbfchar\n\
<0041><0061>\n\
<00FF\n\
<0042><0062>\n\
endbfchar\n\
endcmap\n";

        let cmap = CMap::parse(cmap_data).expect("parse");
        // At least the two well-formed mappings must survive.
        assert!(
            cmap.single_mappings.contains_key(&vec![0x00, 0x41]),
            "first valid mapping (<0041>→<0061>) must survive unterminated hex"
        );
        assert!(
            cmap.single_mappings.contains_key(&vec![0x00, 0x42]),
            "second valid mapping after the malformed `<00FF` must survive"
        );
        assert_eq!(cmap.map(&[0x00, 0x41]), Some(vec![0x00, 0x61]));
        assert_eq!(cmap.map(&[0x00, 0x42]), Some(vec![0x00, 0x62]));
    }

    #[test]
    fn usecmap_external_ucs2_parent_maps_to_ordering() {
        let data = b"begincmap\n/Adobe-Korea1-UCS2 usecmap\n\
1 begincodespacerange <0000> <FFFF> endcodespacerange\n\
endcmap";
        let cmap = CMap::parse(data).expect("parse");
        assert_eq!(cmap.inherited_ordering(), Some("Korea1"));
    }
}
