//! Tests for CMap and ToUnicode functionality

#[cfg(test)]
mod tests {
    use crate::text::cmap::*;
    use crate::text::cmap::{hex_string, string_to_utf16_be_bytes};

    #[test]
    fn test_code_range_basic() {
        let range = CodeRange {
            start: vec![0x20],
            end: vec![0x7E],
        };

        assert!(range.contains(&[0x20])); // Space
        assert!(range.contains(&[0x41])); // 'A'
        assert!(range.contains(&[0x7E])); // '~'
        assert!(!range.contains(&[0x1F])); // Before range
        assert!(!range.contains(&[0x7F])); // After range
        assert!(!range.contains(&[0x20, 0x00])); // Wrong length
    }

    #[test]
    fn test_code_range_multibyte() {
        let range = CodeRange {
            start: vec![0x00, 0x00],
            end: vec![0xFF, 0xFF],
        };

        assert!(range.contains(&[0x00, 0x00]));
        assert!(range.contains(&[0x12, 0x34]));
        assert!(range.contains(&[0xFF, 0xFF]));
        assert!(!range.contains(&[0x00])); // Wrong length
        assert!(!range.contains(&[0x00, 0x00, 0x00])); // Wrong length
    }

    #[test]
    fn test_cmap_identity_h() {
        let cmap = CMap::identity_h();

        assert_eq!(cmap.name, Some("Identity-H".to_string()));
        assert_eq!(cmap.wmode, 0);
        assert!(matches!(cmap.cmap_type, CMapType::Predefined(_)));

        // Identity mapping returns same code
        let code = vec![0x12, 0x34];
        assert_eq!(cmap.map(&code), Some(code.clone()));
    }

    #[test]
    fn test_cmap_identity_v() {
        let cmap = CMap::identity_v();

        assert_eq!(cmap.name, Some("Identity-V".to_string()));
        assert_eq!(cmap.wmode, 1);

        let code = vec![0xAB, 0xCD];
        assert_eq!(cmap.map(&code), Some(code.clone()));
    }

    #[test]
    fn test_tounicode_builder_single_byte() {
        let mut builder = ToUnicodeCMapBuilder::new(1);
        builder.add_single_byte_mapping(0x41, 'A');
        builder.add_single_byte_mapping(0x42, 'B');
        builder.add_single_byte_mapping(0x80, '€');

        let cmap_data = builder.build();
        let cmap_str = String::from_utf8(cmap_data).unwrap();

        // Check required elements
        assert!(cmap_str.contains("/CMapName /Adobe-Identity-UCS def"));
        assert!(cmap_str.contains("/CMapType 2 def"));
        assert!(cmap_str.contains("begincodespacerange"));
        assert!(cmap_str.contains("<00> <FF>"));
        assert!(cmap_str.contains("endcodespacerange"));
        assert!(cmap_str.contains("beginbfchar"));
        assert!(cmap_str.contains("endbfchar"));
    }

    #[test]
    fn test_tounicode_builder_multibyte() {
        let mut builder = ToUnicodeCMapBuilder::new(2);
        builder.add_mapping(vec![0x00, 0x41], "A");
        builder.add_mapping(vec![0x00, 0x42], "B");
        builder.add_mapping(vec![0x4E, 0x2D], "中"); // CJK character

        let cmap_data = builder.build();
        let cmap_str = String::from_utf8(cmap_data).unwrap();

        assert!(cmap_str.contains("<0000> <FFFF>")); // 2-byte codespace
        assert!(cmap_str.contains("<0041>")); // Code for 'A'
        assert!(cmap_str.contains("<4E2D>")); // Code for '中'
    }

    #[test]
    fn test_cmap_parse_simple() {
        let cmap_content = r#"
/CIDInit /ProcSet findresource begin
12 dict begin
begincmap
/CMapName /Test-UCS def
/CMapType 2 def
/WMode 0 def
1 begincodespacerange
<00> <FF>
endcodespacerange
3 beginbfchar
<20> <0020>
<41> <0041>
<42> <0042>
endbfchar
endcmap
"#;

        let cmap = CMap::parse(cmap_content.as_bytes()).unwrap();

        assert_eq!(cmap.name, Some("Test-UCS".to_string()));
        assert_eq!(cmap.wmode, 0);
        assert_eq!(cmap.codespace_ranges.len(), 1);

        // Test mappings
        assert_eq!(cmap.map(&[0x20]), Some(vec![0x00, 0x20])); // Space
        assert_eq!(cmap.map(&[0x41]), Some(vec![0x00, 0x41])); // 'A'
        assert_eq!(cmap.map(&[0x42]), Some(vec![0x00, 0x42])); // 'B'
        assert_eq!(cmap.map(&[0x43]), None); // Not mapped
    }

    #[test]
    fn test_cmap_parse_with_ranges() {
        let cmap_content = r#"
begincmap
1 begincodespacerange
<00> <FF>
endcodespacerange
1 beginbfrange
<20> <7E> <0020>
endbfrange
endcmap
"#;

        let cmap = CMap::parse(cmap_content.as_bytes()).unwrap();

        // Test range mapping
        assert_eq!(cmap.map(&[0x20]), Some(vec![0x00, 0x20])); // First in range
        assert_eq!(cmap.map(&[0x41]), Some(vec![0x00, 0x41])); // Middle of range
        assert_eq!(cmap.map(&[0x7E]), Some(vec![0x00, 0x7E])); // Last in range
        assert_eq!(cmap.map(&[0x1F]), None); // Before range
        assert_eq!(cmap.map(&[0x7F]), None); // After range
    }

    #[test]
    fn test_cmap_to_unicode_utf16() {
        let mut cmap = CMap::new();
        cmap.cmap_type = CMapType::ToUnicode;

        // Test ASCII
        let ascii_bytes = vec![0x00, 0x41]; // UTF-16BE for 'A'
        assert_eq!(cmap.to_unicode(&ascii_bytes), Some("A".to_string()));

        // Test Unicode
        let unicode_bytes = vec![0x20, 0xAC]; // UTF-16BE for '€' (U+20AC)
        assert_eq!(cmap.to_unicode(&unicode_bytes), Some("€".to_string()));

        // Test CJK
        let cjk_bytes = vec![0x4E, 0x2D]; // UTF-16BE for '中' (U+4E2D)
        assert_eq!(cmap.to_unicode(&cjk_bytes), Some("中".to_string()));
    }

    #[test]
    fn test_cmap_entry_single() {
        let entry = CMapEntry::Single {
            src: vec![0x80],
            dst: vec![0x20, 0xAC], // Euro sign in UTF-16BE
        };

        match entry {
            CMapEntry::Single { src, dst } => {
                assert_eq!(src, vec![0x80]);
                assert_eq!(dst, vec![0x20, 0xAC]);
            }
            _ => panic!("Wrong entry type"),
        }
    }

    #[test]
    fn test_cmap_entry_range() {
        let entry = CMapEntry::Range {
            src_start: vec![0x20],
            src_end: vec![0x7E],
            dst_start: vec![0x00, 0x20],
        };

        match entry {
            CMapEntry::Range {
                src_start,
                src_end,
                dst_start,
            } => {
                assert_eq!(src_start, vec![0x20]);
                assert_eq!(src_end, vec![0x7E]);
                assert_eq!(dst_start, vec![0x00, 0x20]);
            }
            _ => panic!("Wrong entry type"),
        }
    }

    #[test]
    fn test_string_to_utf16_be() {
        // ASCII
        let ascii = string_to_utf16_be_bytes("ABC");
        assert_eq!(ascii, vec![0x00, 0x41, 0x00, 0x42, 0x00, 0x43]);

        // Unicode
        let unicode = string_to_utf16_be_bytes("€");
        assert_eq!(unicode, vec![0x20, 0xAC]);

        // CJK
        let cjk = string_to_utf16_be_bytes("中文");
        assert_eq!(cjk, vec![0x4E, 0x2D, 0x65, 0x87]);
    }

    #[test]
    fn test_hex_string() {
        assert_eq!(hex_string(&[0x00]), "00");
        assert_eq!(hex_string(&[0xFF]), "FF");
        assert_eq!(hex_string(&[0x12, 0x34]), "1234");
        assert_eq!(hex_string(&[0xAB, 0xCD, 0xEF]), "ABCDEF");
    }

    #[test]
    fn test_cmap_builder_large_mapping() {
        let mut builder = ToUnicodeCMapBuilder::new(1);

        // Add 150 mappings (should create 2 bfchar sections)
        for i in 0..150u8 {
            builder.add_single_byte_mapping(i, char::from(i.min(127)));
        }

        let cmap_data = builder.build();
        let cmap_str = String::from_utf8(cmap_data).unwrap();

        // Should have multiple bfchar sections (max 100 per section)
        let bfchar_count = cmap_str.matches("beginbfchar").count();
        assert_eq!(bfchar_count, 2); // 100 + 50
    }

    #[test]
    fn test_cmap_valid_codespace() {
        let mut cmap = CMap::new();
        cmap.codespace_ranges.push(CodeRange {
            start: vec![0x00],
            end: vec![0x7F],
        });
        cmap.codespace_ranges.push(CodeRange {
            start: vec![0x80, 0x00],
            end: vec![0xFF, 0xFF],
        });

        // Valid single-byte codes
        assert!(cmap.is_valid_code(&[0x00]));
        assert!(cmap.is_valid_code(&[0x7F]));
        assert!(!cmap.is_valid_code(&[0x80])); // Not in single-byte range
        assert!(!cmap.is_valid_code(&[0xFF])); // Not in single-byte range

        // Valid two-byte codes
        assert!(cmap.is_valid_code(&[0x80, 0x00]));
        assert!(cmap.is_valid_code(&[0xFF, 0xFF]));
        assert!(!cmap.is_valid_code(&[0x7F, 0xFF])); // Not in two-byte range
    }

    // Remove test_calculate_offset as it's a private function
}
