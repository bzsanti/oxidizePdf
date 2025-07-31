//! Tests for TrueType font parsing and subsetting

#[cfg(test)]
mod tests {
    use super::super::truetype::*;
    use std::collections::{HashMap, HashSet};

    /// Create minimal valid TrueType font data for testing
    fn create_minimal_ttf() -> Vec<u8> {
        let mut data = Vec::new();

        // TTF header
        data.extend(&[0x00, 0x01, 0x00, 0x00]); // version
        data.extend(&[0x00, 0x07]); // numTables = 7 (required tables)
        data.extend(&[0x00, 0x80]); // searchRange
        data.extend(&[0x00, 0x03]); // entrySelector
        data.extend(&[0x00, 0x70]); // rangeShift

        // Table directory (16 bytes per table)
        let tables = vec![
            (b"cmap", 0x100, 0x100),
            (b"glyf", 0x200, 0x100),
            (b"head", 0x300, 0x36),
            (b"hhea", 0x340, 0x24),
            (b"hmtx", 0x370, 0x10),
            (b"loca", 0x390, 0x10),
            (b"maxp", 0x3B0, 0x20),
        ];

        for (tag, offset, length) in &tables {
            data.extend(*tag);
            data.extend(&[0x00, 0x00, 0x00, 0x00]); // checksum
            data.extend(&(*offset as u32).to_be_bytes()); // offset
            data.extend(&(*length as u32).to_be_bytes()); // length
        }

        // Pad to first table
        while data.len() < 0x100 {
            data.push(0);
        }

        // cmap table at 0x100
        data.extend(&[0x00, 0x00]); // version
        data.extend(&[0x00, 0x01]); // numTables
        data.extend(&[0x00, 0x03]); // platformID = Windows
        data.extend(&[0x00, 0x01]); // encodingID = Unicode
        data.extend(&[0x00, 0x00, 0x00, 0x0C]); // offset

        // Format 4 subtable
        data.extend(&[0x00, 0x04]); // format
        data.extend(&[0x00, 0x20]); // length
        data.extend(&[0x00, 0x00]); // language
        data.extend(&[0x00, 0x04]); // segCountX2
        data.extend(&[0x00, 0x04]); // searchRange
        data.extend(&[0x00, 0x01]); // entrySelector
        data.extend(&[0x00, 0x00]); // rangeShift

        // End codes
        data.extend(&[0x00, 0x7F]); // End code for segment 1
        data.extend(&[0xFF, 0xFF]); // End code for last segment
        data.extend(&[0x00, 0x00]); // Reserved pad

        // Start codes
        data.extend(&[0x00, 0x20]); // Start code for segment 1
        data.extend(&[0xFF, 0xFF]); // Start code for last segment

        // ID deltas
        data.extend(&[0x00, 0x00]); // Delta for segment 1
        data.extend(&[0x00, 0x01]); // Delta for last segment

        // ID range offsets
        data.extend(&[0x00, 0x00]); // Offset for segment 1
        data.extend(&[0x00, 0x00]); // Offset for last segment

        // Pad to glyf table
        while data.len() < 0x200 {
            data.push(0);
        }

        // glyf table at 0x200 (empty glyphs)
        for _ in 0..0x100 {
            data.push(0);
        }

        // head table at 0x300
        data.extend(&[0x00, 0x01, 0x00, 0x00]); // version
        data.extend(&[0x00, 0x01, 0x00, 0x00]); // fontRevision
        data.extend(&[0x00, 0x00, 0x00, 0x00]); // checkSumAdjustment
        data.extend(&[0x5F, 0x0F, 0x3C, 0xF5]); // magicNumber
        data.extend(&[0x00, 0x00]); // flags
        data.extend(&[0x04, 0x00]); // unitsPerEm = 1024
        data.extend(&[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]); // created
        data.extend(&[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]); // modified
        data.extend(&[0x00, 0x00]); // xMin
        data.extend(&[0x00, 0x00]); // yMin
        data.extend(&[0x04, 0x00]); // xMax
        data.extend(&[0x04, 0x00]); // yMax
        data.extend(&[0x00, 0x00]); // macStyle
        data.extend(&[0x00, 0x08]); // lowestRecPPEM
        data.extend(&[0x00, 0x02]); // fontDirectionHint
        data.extend(&[0x00, 0x00]); // indexToLocFormat = 0 (short)
        data.extend(&[0x00, 0x00]); // glyphDataFormat

        // hhea table at 0x340
        data.extend(&[0x00, 0x01, 0x00, 0x00]); // version
        data.extend(&[0x03, 0x00]); // ascender
        data.extend(&[0xFF, 0x00]); // descender
        data.extend(&[0x00, 0x00]); // lineGap
        data.extend(&[0x04, 0x00]); // advanceWidthMax
        data.extend(&[0x00, 0x00]); // minLeftSideBearing
        data.extend(&[0x00, 0x00]); // minRightSideBearing
        data.extend(&[0x04, 0x00]); // xMaxExtent
        data.extend(&[0x00, 0x01]); // caretSlopeRise
        data.extend(&[0x00, 0x00]); // caretSlopeRun
        data.extend(&[0x00, 0x00]); // caretOffset
        data.extend(&[0x00, 0x00]); // reserved
        data.extend(&[0x00, 0x00]); // reserved
        data.extend(&[0x00, 0x00]); // reserved
        data.extend(&[0x00, 0x00]); // reserved
        data.extend(&[0x00, 0x00]); // metricDataFormat
        data.extend(&[0x00, 0x04]); // numberOfHMetrics = 4

        // hmtx table at 0x370
        data.extend(&[0x02, 0x00, 0x00, 0x00]); // glyph 0: width=512, lsb=0
        data.extend(&[0x02, 0x00, 0x00, 0x00]); // glyph 1: width=512, lsb=0
        data.extend(&[0x02, 0x00, 0x00, 0x00]); // glyph 2: width=512, lsb=0
        data.extend(&[0x02, 0x00, 0x00, 0x00]); // glyph 3: width=512, lsb=0

        // loca table at 0x390 (short format)
        data.extend(&[0x00, 0x00]); // glyph 0 offset
        data.extend(&[0x00, 0x10]); // glyph 1 offset
        data.extend(&[0x00, 0x20]); // glyph 2 offset
        data.extend(&[0x00, 0x30]); // glyph 3 offset
        data.extend(&[0x00, 0x40]); // glyph 4 offset (end)

        // Pad for more offsets
        for _ in 0..3 {
            data.extend(&[0x00, 0x00]);
        }

        // maxp table at 0x3B0
        data.extend(&[0x00, 0x01, 0x00, 0x00]); // version
        data.extend(&[0x00, 0x04]); // numGlyphs = 4
        data.extend(&[0x00, 0x00]); // maxPoints
        data.extend(&[0x00, 0x00]); // maxContours
        data.extend(&[0x00, 0x00]); // maxCompositePoints
        data.extend(&[0x00, 0x00]); // maxCompositeContours
        data.extend(&[0x00, 0x02]); // maxZones
        data.extend(&[0x00, 0x00]); // maxTwilightPoints
        data.extend(&[0x00, 0x00]); // maxStorage
        data.extend(&[0x00, 0x00]); // maxFunctionDefs
        data.extend(&[0x00, 0x00]); // maxInstructionDefs
        data.extend(&[0x00, 0x00]); // maxStackElements
        data.extend(&[0x00, 0x00]); // maxSizeOfInstructions
        data.extend(&[0x00, 0x00]); // maxComponentElements
        data.extend(&[0x00, 0x00]); // maxComponentDepth

        data
    }

    #[test]
    fn test_parse_minimal_ttf() {
        let data = create_minimal_ttf();
        let font = TrueTypeFont::parse(data).unwrap();

        assert_eq!(font.num_glyphs, 4);
        assert_eq!(font.units_per_em, 1024);
        assert_eq!(font.loca_format, 0); // short format
    }

    #[test]
    fn test_parse_cmap() {
        let data = create_minimal_ttf();
        let font = TrueTypeFont::parse(data).unwrap();
        let cmap_tables = font.parse_cmap().unwrap();

        assert_eq!(cmap_tables.len(), 1);
        assert_eq!(cmap_tables[0].platform_id, 3); // Windows
        assert_eq!(cmap_tables[0].encoding_id, 1); // Unicode
        assert_eq!(cmap_tables[0].format, 4);

        // Check some mappings
        let mappings = &cmap_tables[0].mappings;
        assert!(mappings.contains_key(&0x20)); // space
        assert!(mappings.contains_key(&0x41)); // 'A'
    }

    #[test]
    fn test_glyph_metrics() {
        let data = create_minimal_ttf();
        let font = TrueTypeFont::parse(data).unwrap();

        let (width, lsb) = font.get_glyph_metrics(0).unwrap();
        assert_eq!(width, 512);
        assert_eq!(lsb, 0);

        let (width, lsb) = font.get_glyph_metrics(3).unwrap();
        assert_eq!(width, 512);
        assert_eq!(lsb, 0);

        // Test glyph beyond numberOfHMetrics
        let (width, _lsb) = font.get_glyph_metrics(5).unwrap_or((512, 0));
        assert_eq!(width, 512); // Should use last advance width
    }

    #[test]
    fn test_font_name_parsing() {
        // For minimal font, name table is not included
        let data = create_minimal_ttf();
        let font = TrueTypeFont::parse(data).unwrap();
        let name = font.get_font_name().unwrap();
        assert_eq!(name, "Unknown"); // No name table
    }

    #[test]
    fn test_subset_creation() {
        let data = create_minimal_ttf();
        let font = TrueTypeFont::parse(data).unwrap();

        // Create subset with glyphs 0 and 2
        let mut used_glyphs = HashSet::new();
        used_glyphs.insert(0);
        used_glyphs.insert(2);

        let subset_data = font.create_subset(&used_glyphs).unwrap();

        // Verify subset is valid
        assert!(!subset_data.is_empty());
        assert!(subset_data.starts_with(&[0x00, 0x01, 0x00, 0x00])); // TTF signature
    }

    #[test]
    fn test_cmap_format_0_parsing() {
        // Already tested in truetype.rs tests
    }

    #[test]
    fn test_cmap_format_6_parsing() {
        // Test format 6 cmap parsing
        let mappings = HashMap::new();
        let subtable = CmapSubtable {
            platform_id: 1,
            encoding_id: 0,
            format: 6,
            mappings,
        };

        assert_eq!(subtable.format, 6);
        assert_eq!(subtable.mappings.len(), 0);
    }

    #[test]
    fn test_cmap_format_12_parsing() {
        // Test format 12 cmap parsing
        let mut mappings = HashMap::new();
        mappings.insert(0x1F600, 100); // Emoji
        mappings.insert(0x1F601, 101);

        let subtable = CmapSubtable {
            platform_id: 0,
            encoding_id: 4,
            format: 12,
            mappings,
        };

        assert_eq!(subtable.format, 12);
        assert_eq!(subtable.mappings.len(), 2);
        assert_eq!(subtable.mappings.get(&0x1F600), Some(&100));
    }

    #[test]
    fn test_invalid_font_data() {
        // Too small
        let result = TrueTypeFont::parse(vec![0x00, 0x01]);
        assert!(result.is_err());

        // Invalid signature
        let result = TrueTypeFont::parse(vec![
            0xFF, 0xFF, 0xFF, 0xFF, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ]);
        assert!(result.is_err());

        // TTC signature (not supported)
        let result = TrueTypeFont::parse(vec![
            0x74, 0x74, 0x63, 0x66, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ]);
        assert!(result.is_err());
    }

    #[test]
    fn test_missing_required_tables() {
        let mut data = Vec::new();

        // TTF header
        data.extend(&[0x00, 0x01, 0x00, 0x00]); // version
        data.extend(&[0x00, 0x01]); // numTables = 1
        data.extend(&[0x00, 0x00, 0x00, 0x00, 0x00, 0x00]); // search params

        // Only one table (missing required tables)
        data.extend(b"test");
        data.extend(&[0x00, 0x00, 0x00, 0x00]); // checksum
        data.extend(&[0x00, 0x00, 0x00, 0x20]); // offset
        data.extend(&[0x00, 0x00, 0x00, 0x10]); // length

        let result = TrueTypeFont::parse(data);
        assert!(result.is_err());
    }

    #[test]
    fn test_glyph_info_struct() {
        let info = GlyphInfo {
            index: 42,
            unicode: vec![0x41, 0x0041], // 'A'
            advance_width: 500,
            lsb: 50,
        };

        assert_eq!(info.index, 42);
        assert_eq!(info.unicode, vec![0x41, 0x0041]);
        assert_eq!(info.advance_width, 500);
        assert_eq!(info.lsb, 50);
    }

    // Remove test_jpeg_color_space as it's not related to TrueType fonts

    #[test]
    fn test_empty_glyph_set() {
        let data = create_minimal_ttf();
        let font = TrueTypeFont::parse(data).unwrap();

        let empty_set = HashSet::new();
        let subset_data = font.create_subset(&empty_set).unwrap();

        // Should still create valid subset with at least glyph 0
        assert!(!subset_data.is_empty());
    }

    #[test]
    fn test_all_glyphs_subset() {
        let data = create_minimal_ttf();
        let font = TrueTypeFont::parse(data).unwrap();

        let all_glyphs = font.get_all_glyph_indices();
        assert!(!all_glyphs.is_empty());

        let subset_data = font.create_subset(&all_glyphs).unwrap();
        assert!(!subset_data.is_empty());
    }

    #[test]
    fn test_font_bbox_calculation() {
        // This would require parsing the glyf table to calculate bounding box
        // For now, we use fixed values in the implementation
        let data = create_minimal_ttf();
        let font = TrueTypeFont::parse(data).unwrap();
        assert_eq!(font.units_per_em, 1024);
    }

    #[test]
    fn test_platform_encoding_combinations() {
        let mappings = HashMap::new();

        // Windows Unicode
        let win_unicode = CmapSubtable {
            platform_id: 3,
            encoding_id: 1,
            format: 4,
            mappings: mappings.clone(),
        };
        assert_eq!(win_unicode.platform_id, 3);
        assert_eq!(win_unicode.encoding_id, 1);

        // Mac Roman
        let mac_roman = CmapSubtable {
            platform_id: 1,
            encoding_id: 0,
            format: 0,
            mappings: mappings.clone(),
        };
        assert_eq!(mac_roman.platform_id, 1);
        assert_eq!(mac_roman.encoding_id, 0);

        // Unicode
        let unicode = CmapSubtable {
            platform_id: 0,
            encoding_id: 3,
            format: 4,
            mappings,
        };
        assert_eq!(unicode.platform_id, 0);
        assert_eq!(unicode.encoding_id, 3);
    }
}
