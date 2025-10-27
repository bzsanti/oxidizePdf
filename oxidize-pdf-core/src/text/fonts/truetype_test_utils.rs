//! Test utilities for TrueType font testing

#[cfg(test)]
pub mod test_utils {
    use super::super::truetype::*;
    use std::collections::HashSet;

    /// Create a minimal but valid TrueType font for testing
    pub fn create_test_font() -> Vec<u8> {
        let mut font = Vec::new();

        // TTF Header (Offset 0x00)
        font.extend(&[0x00, 0x01, 0x00, 0x00]); // version 1.0
        font.extend(&[0x00, 0x07]); // numTables = 7
        font.extend(&[0x00, 0x80]); // searchRange = 128
        font.extend(&[0x00, 0x03]); // entrySelector = 3
        font.extend(&[0x00, 0x70]); // rangeShift = 112

        // Calculate table offsets
        let table_dir_size = 12 + (7 * 16); // header + 7 table entries
        let mut current_offset = table_dir_size;

        // Table directory entries
        let tables = [
            (b"cmap", 256), // Character to glyph mapping
            (b"glyf", 128), // Glyph data
            (b"head", 54),  // Font header
            (b"hhea", 36),  // Horizontal header
            (b"hmtx", 16),  // Horizontal metrics
            (b"loca", 10),  // Index to location
            (b"maxp", 32),  // Maximum profile
        ];

        // Write table directory
        for (tag, size) in &tables {
            font.extend(*tag);
            font.extend(&[0x00, 0x00, 0x00, 0x00]); // checksum
            font.extend(&(current_offset as u32).to_be_bytes()); // offset
            font.extend(&(*size as u32).to_be_bytes()); // length
            current_offset += size;
        }

        // head table
        font.extend(&[0x00, 0x01, 0x00, 0x00]); // version
        font.extend(&[0x00, 0x01, 0x00, 0x00]); // fontRevision
        font.extend(&[0x00, 0x00, 0x00, 0x00]); // checkSumAdjustment
        font.extend(&[0x5F, 0x0F, 0x3C, 0xF5]); // magicNumber
        font.extend(&[0x00, 0x00]); // flags
        font.extend(&[0x04, 0x00]); // unitsPerEm = 1024
        font.extend(&[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]); // created
        font.extend(&[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]); // modified
        font.extend(&[0x00, 0x00]); // xMin
        font.extend(&[0x00, 0x00]); // yMin
        font.extend(&[0x04, 0x00]); // xMax = 1024
        font.extend(&[0x04, 0x00]); // yMax = 1024
        font.extend(&[0x00, 0x00]); // macStyle
        font.extend(&[0x00, 0x08]); // lowestRecPPEM
        font.extend(&[0x00, 0x02]); // fontDirectionHint
        font.extend(&[0x00, 0x00]); // indexToLocFormat = 0 (short)
        font.extend(&[0x00, 0x00]); // glyphDataFormat

        // hhea table
        font.extend(&[0x00, 0x01, 0x00, 0x00]); // version
        font.extend(&[0x03, 0x00]); // ascender = 768
        font.extend(&[0xFF, 0x00]); // descender = -256
        font.extend(&[0x00, 0x00]); // lineGap
        font.extend(&[0x04, 0x00]); // advanceWidthMax = 1024
        font.extend(&[0x00, 0x00]); // minLeftSideBearing
        font.extend(&[0x00, 0x00]); // minRightSideBearing
        font.extend(&[0x04, 0x00]); // xMaxExtent = 1024
        font.extend(&[0x00, 0x01]); // caretSlopeRise
        font.extend(&[0x00, 0x00]); // caretSlopeRun
        font.extend(&[0x00, 0x00]); // caretOffset
        font.extend(&[0x00, 0x00]); // reserved
        font.extend(&[0x00, 0x00]); // reserved
        font.extend(&[0x00, 0x00]); // reserved
        font.extend(&[0x00, 0x00]); // reserved
        font.extend(&[0x00, 0x00]); // metricDataFormat
        font.extend(&[0x00, 0x04]); // numberOfHMetrics = 4

        // maxp table
        font.extend(&[0x00, 0x01, 0x00, 0x00]); // version 1.0
        font.extend(&[0x00, 0x04]); // numGlyphs = 4
        font.extend(&[0x00, 0x00]); // maxPoints
        font.extend(&[0x00, 0x00]); // maxContours
        font.extend(&[0x00, 0x00]); // maxCompositePoints
        font.extend(&[0x00, 0x00]); // maxCompositeContours
        font.extend(&[0x00, 0x02]); // maxZones
        font.extend(&[0x00, 0x00]); // maxTwilightPoints
        font.extend(&[0x00, 0x00]); // maxStorage
        font.extend(&[0x00, 0x00]); // maxFunctionDefs
        font.extend(&[0x00, 0x00]); // maxInstructionDefs
        font.extend(&[0x00, 0x00]); // maxStackElements
        font.extend(&[0x00, 0x00]); // maxSizeOfInstructions
        font.extend(&[0x00, 0x00]); // maxComponentElements
        font.extend(&[0x00, 0x00]); // maxComponentDepth

        // cmap table
        font.extend(&[0x00, 0x00]); // version
        font.extend(&[0x00, 0x01]); // numTables
                                    // encoding record
        font.extend(&[0x00, 0x03]); // platformID = 3 (Windows)
        font.extend(&[0x00, 0x01]); // encodingID = 1 (Unicode)
        font.extend(&[0x00, 0x00, 0x00, 0x0C]); // offset = 12

        // Format 4 subtable
        font.extend(&[0x00, 0x04]); // format = 4
        font.extend(&[0x00, 0x20]); // length = 32
        font.extend(&[0x00, 0x00]); // language
        font.extend(&[0x00, 0x04]); // segCountX2 = 4
        font.extend(&[0x00, 0x04]); // searchRange
        font.extend(&[0x00, 0x01]); // entrySelector
        font.extend(&[0x00, 0x00]); // rangeShift
                                    // End codes
        font.extend(&[0x00, 0x7F]); // End code for segment
        font.extend(&[0xFF, 0xFF]); // End code 0xFFFF
        font.extend(&[0x00, 0x00]); // Reserved pad
                                    // Start codes
        font.extend(&[0x00, 0x20]); // Start code 0x20
        font.extend(&[0xFF, 0xFF]); // Start code 0xFFFF
                                    // ID deltas
        font.extend(&[0x00, 0x00]); // Delta = 0
        font.extend(&[0x00, 0x01]); // Delta = 1
                                    // ID range offsets
        font.extend(&[0x00, 0x00]); // Offset = 0
        font.extend(&[0x00, 0x00]); // Offset = 0

        // Pad cmap to full size
        while font.len() < table_dir_size + 256 {
            font.push(0);
        }

        // glyf table - empty glyphs
        for _ in 0..128 {
            font.push(0);
        }

        // Ensure we've written head table at the right offset
        while font.len() < table_dir_size + 256 + 128 {
            font.push(0);
        }

        // Already written head table above, skip to hhea offset
        while font.len() < table_dir_size + 256 + 128 + 54 {
            font.push(0);
        }

        // Already written hhea table above, skip to hmtx
        while font.len() < table_dir_size + 256 + 128 + 54 + 36 {
            font.push(0);
        }

        // hmtx table
        font.extend(&[0x02, 0x00, 0x00, 0x00]); // glyph 0: width=512, lsb=0
        font.extend(&[0x02, 0x00, 0x00, 0x00]); // glyph 1
        font.extend(&[0x02, 0x00, 0x00, 0x00]); // glyph 2
        font.extend(&[0x02, 0x00, 0x00, 0x00]); // glyph 3

        // loca table (short format)
        font.extend(&[0x00, 0x00]); // glyph 0 offset
        font.extend(&[0x00, 0x20]); // glyph 1 offset
        font.extend(&[0x00, 0x40]); // glyph 2 offset
        font.extend(&[0x00, 0x60]); // glyph 3 offset
        font.extend(&[0x00, 0x80]); // glyph 4 offset (end)

        font
    }

    /// Verify a TrueType font has expected properties
    pub fn verify_test_font(font: &TrueTypeFont) {
        assert_eq!(font.num_glyphs, 4);
        assert_eq!(font.units_per_em, 1024);
        assert_eq!(font.loca_format, 0);
    }

    /// Create a minimal glyph set for testing
    pub fn create_test_glyph_set() -> HashSet<u16> {
        let mut glyphs = HashSet::new();
        glyphs.insert(0); // .notdef
        glyphs.insert(1); // First real glyph
        glyphs.insert(3); // Skip glyph 2
        glyphs
    }
}
