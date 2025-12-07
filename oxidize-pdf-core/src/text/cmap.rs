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
        }
    }

    /// Parse a CMap from data
    pub fn parse(data: &[u8]) -> ParseResult<Self> {
        let mut cmap = Self::new();
        let content =
            std::str::from_utf8(data).map_err(|e| ParseError::CharacterEncodingError {
                position: 0,
                message: format!("Invalid UTF-8 in CMap: {e}"),
            })?;

        let lines = content.lines();
        let mut in_codespace_range = false;
        let mut in_bf_char = false;
        let mut in_bf_range = false;

        for line in lines {
            let line = line.trim();

            // Skip comments
            if line.starts_with('%') {
                continue;
            }

            // CMap name
            if line.starts_with("/CMapName") {
                if let Some(name) = extract_name(line) {
                    cmap.name = Some(name);
                }
            }
            // Writing mode
            else if line.starts_with("/WMode") {
                if let Some(wmode) = extract_number(line) {
                    cmap.wmode = wmode as u8;
                }
            }
            // Code space range
            else if line.contains("begincodespacerange") {
                in_codespace_range = true;
            } else if line == "endcodespacerange" {
                in_codespace_range = false;
            } else if in_codespace_range {
                if let Some((start, end)) = parse_hex_range(line) {
                    cmap.codespace_ranges.push(CodeRange { start, end });
                }
            }
            // BF char mappings
            else if line.contains("beginbfchar") {
                in_bf_char = true;
            } else if line == "endbfchar" {
                in_bf_char = false;
            } else if in_bf_char {
                if let Some((src, dst)) = parse_bf_char(line) {
                    cmap.single_mappings.insert(src.clone(), dst.clone());
                    cmap.mappings.push(CMapEntry::Single { src, dst });
                }
            }
            // BF range mappings
            else if line.contains("beginbfrange") {
                in_bf_range = true;
            } else if line == "endbfrange" {
                in_bf_range = false;
            } else if in_bf_range {
                // Handle both simple format and array format
                if let Some(entries) = parse_bf_range_entries(line) {
                    for entry in entries {
                        if let CMapEntry::Single { ref src, ref dst } = entry {
                            cmap.single_mappings.insert(src.clone(), dst.clone());
                        }
                        cmap.mappings.push(entry);
                    }
                }
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

                    // Add offset to destination
                    if let Some(last) = result.last_mut() {
                        *last = last.wrapping_add(offset as u8);
                    }

                    return Some(result);
                }
            }
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
        false
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

/// Extract name from a line like "/CMapName /Adobe-Identity-UCS def"
fn extract_name(line: &str) -> Option<String> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() >= 2 && parts[1].starts_with('/') {
        Some(parts[1][1..].to_string())
    } else {
        None
    }
}

/// Extract number from a line like "/WMode 0 def"
fn extract_number(line: &str) -> Option<i32> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() >= 2 {
        parts[1].parse().ok()
    } else {
        None
    }
}

/// Parse hex string to bytes
fn parse_hex(s: &str) -> Option<Vec<u8>> {
    let s = s.trim_start_matches('<').trim_end_matches('>');
    if s.len() % 2 != 0 {
        return None;
    }

    let mut bytes = Vec::new();
    for i in (0..s.len()).step_by(2) {
        if let Ok(byte) = u8::from_str_radix(&s[i..i + 2], 16) {
            bytes.push(byte);
        } else {
            return None;
        }
    }
    Some(bytes)
}

/// Parse a hex range like "<0000> <FFFF>"
fn parse_hex_range(line: &str) -> Option<(Vec<u8>, Vec<u8>)> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() >= 2 {
        if let (Some(start), Some(end)) = (parse_hex(parts[0]), parse_hex(parts[1])) {
            return Some((start, end));
        }
    }
    None
}

/// Parse a bfchar line like "<0001> <0041>"
fn parse_bf_char(line: &str) -> Option<(Vec<u8>, Vec<u8>)> {
    parse_hex_range(line)
}

/// Parse a bfrange line - returns Vec of entries (array format creates multiple entries)
fn parse_bf_range_entries(line: &str) -> Option<Vec<CMapEntry>> {
    // Check if line contains an array (format: <start> <end> [<dst1> <dst2> ...])
    if line.contains('[') {
        // Parse array format: <srcStart> <srcEnd> [<dst1> <dst2> <dst3> ...]
        if let Some(array_start) = line.find('[') {
            let before_array = &line[..array_start];
            let parts: Vec<&str> = before_array.split_whitespace().collect();

            if parts.len() >= 2 {
                if let (Some(src_start), Some(src_end)) = (parse_hex(parts[0]), parse_hex(parts[1]))
                {
                    // Extract array values
                    let after_bracket = &line[array_start + 1..];
                    if let Some(array_end) = after_bracket.find(']') {
                        let array_content = &after_bracket[..array_end];

                        // Parse each hex value in the array
                        let hex_values: Vec<Vec<u8>> = array_content
                            .split_whitespace()
                            .filter_map(parse_hex)
                            .collect();

                        // Create individual Single entries for each mapping
                        let mut entries = Vec::new();
                        let mut current_src = src_start;

                        for dst in hex_values {
                            entries.push(CMapEntry::Single {
                                src: current_src.clone(),
                                dst,
                            });

                            // Increment source code
                            if let Some(last) = current_src.last_mut() {
                                *last = last.wrapping_add(1);
                            }

                            // Stop if we've reached src_end
                            if current_src > src_end {
                                break;
                            }
                        }

                        return Some(entries);
                    }
                }
            }
        }
        return None;
    }

    // Original simple format: <start> <end> <dst>
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() >= 3 {
        if let (Some(start), Some(end), Some(dst)) = (
            parse_hex(parts[0]),
            parse_hex(parts[1]),
            parse_hex(parts[2]),
        ) {
            return Some(vec![CMapEntry::Range {
                src_start: start,
                src_end: end,
                dst_start: dst,
            }]);
        }
    }
    None
}

/// Calculate offset between two byte arrays
fn calculate_offset(code: &[u8], start: &[u8]) -> usize {
    let mut offset = 0;
    for i in (0..code.len()).rev() {
        let diff = code[i] as usize - start[i] as usize;
        offset += diff * (256_usize.pow((code.len() - i - 1) as u32));
    }
    offset
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
}
