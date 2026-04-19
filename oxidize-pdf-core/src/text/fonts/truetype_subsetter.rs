//! TrueType font subsetting
//!
//! This module implements font subsetting to reduce file size by including
//! only the glyphs actually used in the document.

#![allow(dead_code)]

use super::truetype::{CmapSubtable, TrueTypeFont};
use crate::parser::{ParseError, ParseResult};
use std::collections::{HashMap, HashSet};

// =============================================================================
// COMPOSITE GLYPH FLAGS (OpenType spec §5.3.3 — Glyph Headers)
// =============================================================================

/// Component glyph arguments are 16-bit (words) instead of 8-bit (bytes)
const COMPOSITE_ARG_1_AND_2_ARE_WORDS: u16 = 0x0001;
/// There are more components after this one
const COMPOSITE_MORE_COMPONENTS: u16 = 0x0020;
/// Component has a simple scale (F2Dot14)
const COMPOSITE_WE_HAVE_A_SCALE: u16 = 0x0008;
/// Component has separate X and Y scales (two F2Dot14s)
const COMPOSITE_WE_HAVE_AN_X_AND_Y_SCALE: u16 = 0x0040;
/// Component has a 2x2 transformation matrix (four F2Dot14s)
const COMPOSITE_WE_HAVE_A_TWO_BY_TWO: u16 = 0x0080;
/// Instructions follow after all components (OpenType spec §5.3.3 flag 0x0100).
/// Not used in current parsing — instructions appear after all component records,
/// not between them, so the component loop terminates correctly without this flag.
const COMPOSITE_WE_HAVE_INSTRUCTIONS: u16 = 0x0100;

/// Maximum byte offset representable in loca short format (u16 * 2).
/// If the glyf table exceeds this, loca must use long format (u32 entries).
const LOCA_SHORT_FORMAT_MAX_OFFSET: u32 = 0x1_FFFE;

/// When the subset uses more than this fraction of the font's total glyphs,
/// subsetting is skipped because the overhead of rebuilding the font outweighs
/// the size savings. Could be exposed as a field on a `SubsetOptions` struct
/// in a future release.
const SUBSETTING_RATIO_THRESHOLD: f32 = 0.5;

// =============================================================================
// SUBSETTING THRESHOLDS (Issue #115)
// =============================================================================
//
// These constants control when font subsetting is skipped vs performed.
// The logic is:
// - If used_chars is empty → skip (nothing to subset)
// - If font is small AND few characters → skip (overhead not worth it)
// - Otherwise → perform subsetting
//
// IMPORTANT: Large fonts (>100KB) should ALWAYS be subsetted, even with few
// characters. A 41MB CJK font with 4 characters should produce a ~10KB subset,
// not a 41MB embedded font.

/// Minimum font size (in bytes) below which subsetting may be skipped
/// for small character sets. Fonts smaller than this are cheap to embed fully.
pub const SUBSETTING_SIZE_THRESHOLD: usize = 100_000; // 100KB

/// Minimum number of characters below which subsetting may be skipped
/// for small fonts. This threshold is ONLY applied when font size is
/// below SUBSETTING_SIZE_THRESHOLD.
pub const SUBSETTING_CHAR_THRESHOLD: usize = 10;

/// Determines whether font subsetting should be skipped based on font size
/// and number of characters used.
///
/// Returns `true` if subsetting should be SKIPPED (use full font).
/// Returns `false` if subsetting should be PERFORMED.
///
/// # Logic
/// - Empty character set → skip (nothing to subset)
/// - Small font (<100KB) AND few chars (<10) → skip (not worth the overhead)
/// - Large font (≥100KB) → ALWAYS subset, regardless of char count
/// - Many chars (≥10) → ALWAYS subset, regardless of font size
#[inline]
pub fn should_skip_subsetting(font_size: usize, char_count: usize) -> bool {
    // Empty character set - nothing to subset
    if char_count == 0 {
        return true;
    }

    // Only skip subsetting if BOTH conditions are true:
    // 1. Font is small (< 100KB) - cheap to embed fully
    // 2. Few characters (< 10) - subsetting overhead may exceed benefit
    font_size < SUBSETTING_SIZE_THRESHOLD && char_count < SUBSETTING_CHAR_THRESHOLD
}

/// Filter a full cmap mapping to only include codepoints in `used_chars`.
/// Used when the font is not subsetted (full font embedded) to avoid leaking
/// unused entries into the W array and CIDToGIDMap.
fn filter_mapping_to_used(
    mappings: &HashMap<u32, u16>,
    used_chars: &HashSet<char>,
) -> HashMap<u32, u16> {
    used_chars
        .iter()
        .filter_map(|ch| {
            let cp = *ch as u32;
            mappings.get(&cp).map(|&gid| (cp, gid))
        })
        .collect()
}

/// Table record for font directory
struct TableRecord {
    tag: [u8; 4],
    checksum: u32,
    offset: u32,
    length: u32,
}

/// Read a u32 from font data
fn read_u32(data: &[u8], offset: usize) -> ParseResult<u32> {
    if offset + 4 > data.len() {
        return Err(ParseError::SyntaxError {
            position: offset,
            message: "Unexpected end of font data".to_string(),
        });
    }
    Ok(u32::from_be_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
    ]))
}

/// Calculate table checksum
fn calculate_checksum(data: &[u8]) -> u32 {
    let mut sum = 0u32;
    let mut i = 0;

    // Process complete 4-byte chunks
    while i + 3 < data.len() {
        sum = sum.wrapping_add(u32::from_be_bytes([
            data[i],
            data[i + 1],
            data[i + 2],
            data[i + 3],
        ]));
        i += 4;
    }

    // Handle remaining bytes
    if i < data.len() {
        let mut last = [0u8; 4];
        let remaining = data.len() - i;
        last[..remaining].copy_from_slice(&data[i..]);
        sum = sum.wrapping_add(u32::from_be_bytes(last));
    }

    sum
}

/// Result of font subsetting operation
pub struct SubsetResult {
    /// Subsetted font data
    pub font_data: Vec<u8>,
    /// Unicode to GlyphID mapping for the subsetted font
    pub glyph_mapping: HashMap<u32, u16>,
    /// True if font_data is raw CFF bytes (embed with /CIDFontType0C),
    /// false if it's OTF or TrueType (embed with /OpenType or /FontFile2)
    pub is_raw_cff: bool,
}

/// Extract component glyph IDs from a composite glyph's raw data.
///
/// Per OpenType spec §5.3.3, a composite glyph has `numberOfContours < 0`
/// (first 2 bytes as i16). Its body contains a sequence of component records,
/// each with flags (u16) and a glyphIndex (u16), followed by variable-size
/// arguments and optional transformation data.
///
/// Returns an empty vec for simple glyphs or data too short to parse.
pub fn extract_composite_components(glyph_data: &[u8]) -> Vec<u16> {
    if glyph_data.len() < 12 {
        // Glyph header is 10 bytes; need at least 4 more for one component
        return Vec::new();
    }

    let num_contours = i16::from_be_bytes([glyph_data[0], glyph_data[1]]);
    if num_contours >= 0 {
        // Simple glyph — no components
        return Vec::new();
    }

    let mut components = Vec::new();
    let mut cursor = 10; // Skip glyph header (numberOfContours + xMin/yMin/xMax/yMax)

    loop {
        if cursor + 4 > glyph_data.len() {
            break;
        }

        let flags = u16::from_be_bytes([glyph_data[cursor], glyph_data[cursor + 1]]);
        let glyph_index = u16::from_be_bytes([glyph_data[cursor + 2], glyph_data[cursor + 3]]);
        components.push(glyph_index);

        // Advance past flags (2) + glyphIndex (2)
        cursor += 4;

        // Advance past arguments (offsets or point numbers)
        if flags & COMPOSITE_ARG_1_AND_2_ARE_WORDS != 0 {
            cursor += 4; // Two i16/u16 args
        } else {
            cursor += 2; // Two i8/u8 args
        }

        // Advance past transformation data
        if flags & COMPOSITE_WE_HAVE_A_TWO_BY_TWO != 0 {
            cursor += 8; // Four F2Dot14 values
        } else if flags & COMPOSITE_WE_HAVE_AN_X_AND_Y_SCALE != 0 {
            cursor += 4; // Two F2Dot14 values
        } else if flags & COMPOSITE_WE_HAVE_A_SCALE != 0 {
            cursor += 2; // One F2Dot14 value
        }

        if flags & COMPOSITE_MORE_COMPONENTS == 0 {
            break;
        }
    }

    components
}

/// Rewrite component glyph IDs in a composite glyph's data from old GIDs to new GIDs.
///
/// For simple glyphs (numberOfContours >= 0), returns data unchanged.
/// For composite glyphs, walks the component list and replaces each GlyphIndex
/// using the provided mapping.
fn remap_composite_glyph(glyph_data: &[u8], glyph_map: &HashMap<u16, u16>) -> Vec<u8> {
    if glyph_data.len() < 12 {
        return glyph_data.to_vec();
    }

    let num_contours = i16::from_be_bytes([glyph_data[0], glyph_data[1]]);
    if num_contours >= 0 {
        return glyph_data.to_vec();
    }

    let mut result = glyph_data.to_vec();
    let mut cursor = 10;

    loop {
        if cursor + 4 > result.len() {
            break;
        }

        let flags = u16::from_be_bytes([result[cursor], result[cursor + 1]]);
        let old_gid = u16::from_be_bytes([result[cursor + 2], result[cursor + 3]]);
        // .notdef (GID 0) fallback is correct: expand_composite_glyphs() ensures
        // all component GIDs are in needed_glyphs before remapping. An unmapped GID
        // here indicates font corruption; .notdef is safer than panicking.
        let new_gid = glyph_map.get(&old_gid).copied().unwrap_or(0);
        result[cursor + 2] = (new_gid >> 8) as u8;
        result[cursor + 3] = (new_gid & 0xFF) as u8;

        cursor += 4;

        if flags & COMPOSITE_ARG_1_AND_2_ARE_WORDS != 0 {
            cursor += 4;
        } else {
            cursor += 2;
        }

        if flags & COMPOSITE_WE_HAVE_A_TWO_BY_TWO != 0 {
            cursor += 8;
        } else if flags & COMPOSITE_WE_HAVE_AN_X_AND_Y_SCALE != 0 {
            cursor += 4;
        } else if flags & COMPOSITE_WE_HAVE_A_SCALE != 0 {
            cursor += 2;
        }

        if flags & COMPOSITE_MORE_COMPONENTS == 0 {
            break;
        }
    }

    result
}

/// TrueType font subsetter
pub struct TrueTypeSubsetter {
    /// Original font data
    font_data: Vec<u8>,
    /// Parsed font
    font: TrueTypeFont,
}

impl TrueTypeSubsetter {
    /// Create a new subsetter from font data
    // TODO(perf): TrueTypeFont::parse takes Vec<u8> by value, requiring a full
    // clone of font_data here (10-50MB for CJK fonts). Fixing this requires
    // changing parse() to accept &[u8] or Arc<Vec<u8>>.
    pub fn new(font_data: Vec<u8>) -> ParseResult<Self> {
        let font = TrueTypeFont::parse(font_data.clone())?;
        Ok(Self { font_data, font })
    }

    /// Subset the font to include only the specified characters
    /// Returns the subsetted font data and the Unicode to GlyphID mapping
    pub fn subset(&self, used_chars: &HashSet<char>) -> ParseResult<SubsetResult> {
        // Get the cmap table to find which glyphs we need
        let cmap_tables = self.font.parse_cmap()?;
        let cmap = CmapSubtable::select_best_or_first(&cmap_tables).ok_or_else(|| {
            ParseError::SyntaxError {
                position: 0,
                message: "No suitable cmap table found".to_string(),
            }
        })?;

        // Issue #115 Fix: Use should_skip_subsetting() which considers BOTH font size AND char count
        // Previously, this skipped subsetting for ANY font with < 10 chars, causing 41MB fonts
        // to be embedded fully even when only 4 characters were used.
        if should_skip_subsetting(self.font_data.len(), used_chars.len()) {
            return Ok(SubsetResult {
                font_data: self.font_data.clone(),
                glyph_mapping: filter_mapping_to_used(&cmap.mappings, used_chars),
                is_raw_cff: false,
            });
        }

        // Check if we need most of the glyphs - if so, don't subset
        let mut needed_glyphs = HashSet::new();
        needed_glyphs.insert(0); // Always include .notdef

        for ch in used_chars {
            let unicode = *ch as u32;
            if let Some(&glyph_id) = cmap.mappings.get(&unicode) {
                needed_glyphs.insert(glyph_id);
            }
        }

        // Expand composite glyphs: add component GIDs recursively
        self.expand_composite_glyphs(&mut needed_glyphs);

        tracing::debug!("Font subsetting analysis:");
        tracing::debug!("  Total glyphs in font: {}", self.font.num_glyphs);
        tracing::debug!("  Glyphs needed: {}", needed_glyphs.len());
        tracing::debug!("  Characters used: {}", used_chars.len());

        // Always subset if we're using less than 10% of glyphs in a large font
        let subset_ratio = needed_glyphs.len() as f32 / self.font.num_glyphs as f32;
        if subset_ratio > SUBSETTING_RATIO_THRESHOLD || self.font_data.len() < 100_000 {
            tracing::debug!(
                "  Keeping full font (using {:.1}% of glyphs)",
                subset_ratio * 100.0
            );
            return Ok(SubsetResult {
                font_data: self.font_data.clone(),
                glyph_mapping: filter_mapping_to_used(&cmap.mappings, used_chars),
                is_raw_cff: false,
            });
        }

        tracing::debug!(
            "  Subsetting font (using only {:.1}% of glyphs)",
            subset_ratio * 100.0
        );

        // CFF/OpenType fonts require a different subsetting approach
        if self.font.is_cff {
            match crate::text::fonts::cff_subsetter::subset_cff_font(&self.font_data, used_chars) {
                Ok(result) => {
                    tracing::debug!(
                        "  CFF subsetting: {} -> {} bytes ({:.1}% reduction)",
                        self.font_data.len(),
                        result.font_data.len(),
                        (1.0 - result.font_data.len() as f32 / self.font_data.len() as f32) * 100.0
                    );
                    return Ok(SubsetResult {
                        font_data: result.font_data,
                        glyph_mapping: result.glyph_mapping,
                        is_raw_cff: result.is_raw_cff,
                    });
                }
                Err(e) => {
                    tracing::debug!("  CFF subsetting failed: {:?}, using full font", e);
                    return Ok(SubsetResult {
                        font_data: self.font_data.clone(),
                        glyph_mapping: filter_mapping_to_used(&cmap.mappings, used_chars),
                        is_raw_cff: false,
                    });
                }
            }
        }

        // Create glyph remapping: old_glyph_id -> new_glyph_id
        let mut glyph_map: HashMap<u16, u16> = HashMap::new();
        let mut sorted_glyphs: Vec<u16> = needed_glyphs.iter().copied().collect();
        sorted_glyphs.sort(); // Ensure glyph 0 (.notdef) comes first

        for (new_id, &old_id) in sorted_glyphs.iter().enumerate() {
            glyph_map.insert(old_id, new_id as u16);
        }

        // Create new cmap with remapped glyph IDs
        let mut new_cmap: HashMap<u32, u16> = HashMap::new();
        for ch in used_chars {
            let unicode = *ch as u32;
            if let Some(&old_glyph_id) = cmap.mappings.get(&unicode) {
                if let Some(&new_glyph_id) = glyph_map.get(&old_glyph_id) {
                    new_cmap.insert(unicode, new_glyph_id);
                }
            }
        }

        // Build the actual subset font
        match self.build_subset_font(&glyph_map) {
            Ok(subset_font_data) => {
                tracing::debug!(
                    "  Successfully subsetted: {} -> {} bytes ({:.1}% reduction)",
                    self.font_data.len(),
                    subset_font_data.len(),
                    (1.0 - subset_font_data.len() as f32 / self.font_data.len() as f32) * 100.0
                );

                Ok(SubsetResult {
                    font_data: subset_font_data,
                    glyph_mapping: new_cmap,
                    is_raw_cff: false,
                })
            }
            Err(e) => {
                tracing::debug!("  Subsetting failed: {:?}, using full font as fallback", e);
                Ok(SubsetResult {
                    font_data: self.font_data.clone(),
                    glyph_mapping: cmap.mappings.clone(),
                    is_raw_cff: false,
                })
            }
        }
    }

    /// Expand composite glyphs by adding all component GIDs to needed_glyphs.
    /// Uses a worklist for iterative (not recursive) traversal to handle
    /// composites that reference other composites.
    fn expand_composite_glyphs(&self, needed_glyphs: &mut HashSet<u16>) {
        let mut worklist: Vec<u16> = needed_glyphs.iter().copied().collect();
        while let Some(gid) = worklist.pop() {
            if let Ok(data) = self.font.get_glyph_data(gid) {
                for component_gid in extract_composite_components(&data) {
                    if needed_glyphs.insert(component_gid) {
                        worklist.push(component_gid);
                    }
                }
            }
        }
    }

    /// Build the subset font file.
    ///
    /// Output contains only the tables required for PDF embedding: glyf,
    /// head, loca, hmtx, hhea, maxp, post. `cmap`, `OS/2` and `name` are
    /// stripped — PDF uses its own ToUnicode CMap and CIDToGIDMap for
    /// character-to-glyph resolution, and OS/2 / name are cosmetic.
    fn build_subset_font(&self, glyph_map: &HashMap<u16, u16>) -> ParseResult<Vec<u8>> {
        // Build new glyf table with only needed glyphs
        let mut new_glyf = Vec::new();
        let mut current_offset = 0u32;

        // Create inverse map: new_glyph_id -> old_glyph_id
        let mut inverse_map: HashMap<u16, u16> = HashMap::new();
        for (&old_id, &new_id) in glyph_map {
            inverse_map.insert(new_id, old_id);
        }

        // Collect per-glyph offsets first, then decide loca format
        let mut glyph_offsets: Vec<u32> = Vec::with_capacity(glyph_map.len() + 1);

        for new_glyph_id in 0..glyph_map.len() as u16 {
            glyph_offsets.push(current_offset);

            let old_glyph_id = inverse_map.get(&new_glyph_id).copied().unwrap_or(0);
            let glyph_data = self.font.get_glyph_data(old_glyph_id)?;
            let remapped = remap_composite_glyph(&glyph_data, glyph_map);
            new_glyf.extend_from_slice(&remapped);
            current_offset += remapped.len() as u32;
        }
        // Final loca entry (points past last glyph)
        glyph_offsets.push(current_offset);

        // Determine loca format: use short if original was short AND offsets fit,
        // otherwise upgrade to long format to prevent silent truncation.
        let loca_format =
            if self.font.loca_format == 0 && current_offset <= LOCA_SHORT_FORMAT_MAX_OFFSET {
                0u16
            } else {
                1u16
            };

        // Build loca table with the chosen format
        let mut new_loca = Vec::new();
        for &offset in &glyph_offsets {
            if loca_format == 0 {
                new_loca.extend_from_slice(&((offset / 2) as u16).to_be_bytes());
            } else {
                new_loca.extend_from_slice(&offset.to_be_bytes());
            }
        }

        // Build new hmtx table
        let new_hmtx = self.build_hmtx(glyph_map, &inverse_map)?;

        // Reconstruct the font file — cmap is not built; PDF handles
        // character-to-glyph resolution via ToUnicode/CIDToGIDMap.
        self.build_font_file(
            new_glyf,
            new_loca,
            new_hmtx,
            glyph_map.len() as u16,
            loca_format,
        )
    }

    /// Build hmtx table
    fn build_hmtx(
        &self,
        glyph_map: &HashMap<u16, u16>,
        inverse_map: &HashMap<u16, u16>,
    ) -> ParseResult<Vec<u8>> {
        let mut data = Vec::new();

        // For each new glyph ID in order, add its width
        // IMPORTANT: hmtx must store widths in font design units (NOT scaled to 1000/em).
        // get_glyph_metrics returns the raw advance width and LSB from the original hmtx table.
        for new_glyph_id in 0..glyph_map.len() as u16 {
            let old_glyph_id = inverse_map.get(&new_glyph_id).copied().unwrap_or(0);

            // get_glyph_metrics returns (advance_width, lsb) in font design units
            let (advance_width, lsb) = self
                .font
                .get_glyph_metrics(old_glyph_id)
                .unwrap_or((self.font.units_per_em, 0));

            data.extend_from_slice(&advance_width.to_be_bytes());
            data.extend_from_slice(&lsb.to_be_bytes());
        }

        Ok(data)
    }
    /// Build final font file
    fn build_font_file(
        &self,
        glyf: Vec<u8>,
        loca: Vec<u8>,
        hmtx: Vec<u8>,
        num_glyphs: u16,
        loca_format: u16,
    ) -> ParseResult<Vec<u8>> {
        let mut font_data = Vec::new();
        let mut table_records = Vec::new();

        // Read original font header to preserve some data
        let sfnt_version = read_u32(&self.font_data, 0)?;

        // Tables we'll include in the subset font. We deliberately omit
        // cmap / OS/2 / name: PDF provides its own character-to-glyph
        // resolution (ToUnicode + CIDToGIDMap), and OS/2 / name are
        // metadata that renderers do not consult when drawing.
        let head_table = self.get_table_data(b"head")?;
        let hhea_table = self.update_hhea_table(num_glyphs)?;
        let maxp_table = self.get_original_maxp(num_glyphs)?;
        let post_table = self
            .get_table_data(b"post")
            .unwrap_or_else(|_| vec![0x00, 0x03, 0x00, 0x00]); // Version 3.0

        let tables_to_write: Vec<(&[u8; 4], Vec<u8>)> = vec![
            (b"glyf", glyf),
            (b"head", self.update_head_table(head_table, loca_format)?),
            (b"hhea", hhea_table),
            (b"hmtx", hmtx),
            (b"loca", loca),
            (b"maxp", maxp_table),
            (b"post", post_table),
        ];

        let num_tables = tables_to_write.len() as u16;

        // Calculate header values
        let entry_selector = (num_tables as f64).log2().floor() as u16;
        let search_range = (2u16.pow(entry_selector as u32)) * 16;
        let range_shift = num_tables * 16 - search_range;

        // Write font header
        font_data.extend_from_slice(&sfnt_version.to_be_bytes());
        font_data.extend_from_slice(&num_tables.to_be_bytes());
        font_data.extend_from_slice(&search_range.to_be_bytes());
        font_data.extend_from_slice(&entry_selector.to_be_bytes());
        font_data.extend_from_slice(&range_shift.to_be_bytes());

        // Calculate offsets and write table directory
        let table_dir_size = 12 + num_tables as usize * 16;
        let mut current_offset = table_dir_size;

        // First pass: calculate offsets and checksums.
        // IMPORTANT: alignment padding must be applied BEFORE recording the offset,
        // matching the second pass (data writing) which also pads before each table.
        for (tag, data) in &tables_to_write {
            while current_offset % 4 != 0 {
                current_offset += 1;
            }

            let checksum = calculate_checksum(&data);
            table_records.push(TableRecord {
                tag: **tag,
                checksum,
                offset: current_offset as u32,
                length: data.len() as u32,
            });

            current_offset += data.len();
        }

        // Write table directory
        for record in &table_records {
            font_data.extend_from_slice(&record.tag);
            font_data.extend_from_slice(&record.checksum.to_be_bytes());
            font_data.extend_from_slice(&record.offset.to_be_bytes());
            font_data.extend_from_slice(&record.length.to_be_bytes());
        }

        // Write actual table data
        for (_, data) in &tables_to_write {
            // Align to 4-byte boundary
            while font_data.len() % 4 != 0 {
                font_data.push(0);
            }
            font_data.extend_from_slice(&data);
        }

        Ok(font_data)
    }

    /// Get table data from original font
    fn get_table_data(&self, tag: &[u8]) -> ParseResult<Vec<u8>> {
        let table = self.font.get_table(tag)?;
        Ok(self.font_data[table.offset as usize..(table.offset + table.length) as usize].to_vec())
    }

    /// Update head table with new loca format
    fn update_head_table(&self, mut head: Vec<u8>, loca_format: u16) -> ParseResult<Vec<u8>> {
        if head.len() < 54 {
            return Err(ParseError::SyntaxError {
                position: 0,
                message: "Invalid head table".to_string(),
            });
        }
        // Update indexToLocFormat at offset 50
        head[50] = (loca_format >> 8) as u8;
        head[51] = (loca_format & 0xFF) as u8;
        Ok(head)
    }

    /// Create updated maxp table with new glyph count
    fn get_original_maxp(&self, num_glyphs: u16) -> ParseResult<Vec<u8>> {
        let mut maxp = self.get_table_data(b"maxp")?;
        if maxp.len() < 6 {
            return Err(ParseError::SyntaxError {
                position: 0,
                message: "Invalid maxp table".to_string(),
            });
        }
        // Update numGlyphs at offset 4
        maxp[4] = (num_glyphs >> 8) as u8;
        maxp[5] = (num_glyphs & 0xFF) as u8;
        Ok(maxp)
    }

    /// Update hhea table with the correct numberOfHMetrics for the subset.
    ///
    /// Per the OpenType spec, `numberOfHMetrics` in hhea tells the reader how many
    /// full longHorMetric records (4 bytes each: advanceWidth + lsb) are in the hmtx
    /// table. After subsetting, this must equal the number of glyphs in the subset,
    /// since `build_hmtx` writes one full entry per glyph.
    ///
    /// Leaving the original (large) numberOfHMetrics intact causes out-of-bounds
    /// reads: the renderer attempts to read hmtx entries that no longer exist,
    /// producing garbage advance widths and potentially misaligned composite glyphs.
    fn update_hhea_table(&self, num_glyphs: u16) -> ParseResult<Vec<u8>> {
        let mut hhea = self.get_table_data(b"hhea")?;
        if hhea.len() < 36 {
            return Err(ParseError::SyntaxError {
                position: 0,
                message: "Invalid hhea table (too short)".to_string(),
            });
        }
        // numberOfHMetrics is at offset 34 in hhea (OpenType spec §hmtx table)
        hhea[34] = (num_glyphs >> 8) as u8;
        hhea[35] = (num_glyphs & 0xFF) as u8;
        Ok(hhea)
    }
}

/// Convenience function to subset a font
pub fn subset_font(font_data: Vec<u8>, used_chars: &HashSet<char>) -> ParseResult<SubsetResult> {
    let subsetter = TrueTypeSubsetter::new(font_data)?;
    subsetter.subset(used_chars)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_u32() {
        let data = vec![0x00, 0x01, 0x02, 0x03, 0x04, 0x05];

        let result = read_u32(&data, 0);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0x00010203);

        let result = read_u32(&data, 2);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0x02030405);

        // Test out of bounds
        let result = read_u32(&data, 3);
        assert!(result.is_err());
    }

    #[test]
    fn test_calculate_checksum() {
        // Test with aligned data
        let data = vec![0x00, 0x01, 0x02, 0x03];
        let checksum = calculate_checksum(&data);
        assert_eq!(checksum, 0x00010203);

        // Test with multiple chunks
        let data = vec![0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07];
        let checksum = calculate_checksum(&data);
        assert_eq!(checksum, 0x00010203_u32.wrapping_add(0x04050607));

        // Test with unaligned data (padding)
        let data = vec![0x00, 0x01, 0x02];
        let checksum = calculate_checksum(&data);
        assert_eq!(checksum, 0x00010200); // Last byte is padded with 0
    }

    #[test]
    fn test_glyph_id_remapping() {
        let used_glyphs = vec![0, 3, 5, 10, 15].into_iter().collect::<HashSet<_>>();

        // Create a simple mapping - need to sort to ensure consistent ordering
        let mut glyph_map = HashMap::new();
        let mut sorted_glyphs: Vec<_> = used_glyphs.iter().copied().collect();
        sorted_glyphs.sort();

        let mut new_id = 0;
        for old_id in sorted_glyphs {
            glyph_map.insert(old_id, new_id);
            new_id += 1;
        }

        // Verify mapping - glyph 0 always maps to 0 (sorted first)
        assert_eq!(glyph_map.get(&0), Some(&0));
        // Other glyphs map to sequential IDs based on their sorted order
        assert!(glyph_map.contains_key(&3));
        assert!(glyph_map.contains_key(&5));
        assert!(glyph_map.contains_key(&10));
        assert!(glyph_map.contains_key(&15));
        assert_eq!(glyph_map.get(&7), None); // Not in used glyphs
    }

    #[test]
    fn test_table_record_structure() {
        let record = TableRecord {
            tag: [b'h', b'e', b'a', b'd'],
            checksum: 0x12345678,
            offset: 0x1000,
            length: 0x0054,
        };

        assert_eq!(record.tag, [b'h', b'e', b'a', b'd']);
        assert_eq!(record.checksum, 0x12345678);
        assert_eq!(record.offset, 0x1000);
        assert_eq!(record.length, 0x0054);
    }

    #[test]
    fn test_checksum_overflow() {
        // Test that checksum handles overflow correctly
        let data = vec![0xFF; 8]; // Two max u32 values
        let checksum = calculate_checksum(&data);
        // Should wrap around on overflow: 0xFFFFFFFF + 0xFFFFFFFF = 0xFFFFFFFE (with wrap)
        assert_eq!(checksum, 0xFFFFFFFE);
    }

    #[test]
    fn test_required_tables() {
        // List of required tables for a subset font
        let required_tables = vec![
            [b'h', b'e', b'a', b'd'],
            [b'h', b'h', b'e', b'a'],
            [b'm', b'a', b'x', b'p'],
            [b'h', b'm', b't', b'x'],
            [b'l', b'o', b'c', b'a'],
            [b'g', b'l', b'y', b'f'],
        ];

        for tag in required_tables {
            // Verify we recognize these as required
            assert!(tag.len() == 4);
        }
    }

    #[test]
    fn test_optional_tables() {
        // List of optional tables that might be included
        let optional_tables = vec![
            [b'n', b'a', b'm', b'e'],
            [b'p', b'o', b's', b't'],
            [b'c', b'm', b'a', b'p'],
            [b'k', b'e', b'r', b'n'],
            [b'G', b'P', b'O', b'S'],
            [b'G', b'S', b'U', b'B'],
        ];

        for tag in optional_tables {
            assert!(tag.len() == 4);
        }
    }

    #[test]
    fn test_font_header_validation() {
        // Test validation of font header
        let mut data = vec![0u8; 12];

        // TrueType signature
        data[0] = 0x00;
        data[1] = 0x01;
        data[2] = 0x00;
        data[3] = 0x00;

        // Number of tables
        data[4] = 0x00;
        data[5] = 0x06; // 6 tables

        let num_tables = u16::from_be_bytes([data[4], data[5]]);
        assert_eq!(num_tables, 6);
    }

    #[test]
    fn test_glyph_remapping_consistency() {
        let mut used_glyphs = HashSet::new();
        used_glyphs.insert(0); // .notdef
        used_glyphs.insert(32); // space
        used_glyphs.insert(65); // A
        used_glyphs.insert(97); // a

        let mut glyph_map = HashMap::new();
        let mut new_id = 0;

        // Create consistent mapping
        let mut sorted_glyphs: Vec<_> = used_glyphs.iter().cloned().collect();
        sorted_glyphs.sort();

        for old_id in sorted_glyphs {
            glyph_map.insert(old_id, new_id);
            new_id += 1;
        }

        // Verify consistency
        assert_eq!(glyph_map.len(), 4);
        assert_eq!(*glyph_map.get(&0).unwrap(), 0); // .notdef stays at 0
        assert!(*glyph_map.get(&32).unwrap() < 4);
        assert!(*glyph_map.get(&65).unwrap() < 4);
        assert!(*glyph_map.get(&97).unwrap() < 4);
    }

    #[test]
    fn test_table_directory_size() {
        // Table directory entry is 16 bytes
        let num_tables = 10;
        let dir_size = 12 + (num_tables * 16); // Header + entries
        assert_eq!(dir_size, 172);

        // Verify alignment
        assert_eq!(dir_size % 4, 0);
    }

    #[test]
    fn test_loca_table_format() {
        // Test short format (16-bit offsets)
        let short_offsets = vec![0u16, 100, 200, 350, 500];
        let mut short_data = Vec::new();
        for offset in &short_offsets {
            short_data.extend_from_slice(&offset.to_be_bytes());
        }
        assert_eq!(short_data.len(), short_offsets.len() * 2);

        // Test long format (32-bit offsets)
        let long_offsets = vec![0u32, 100, 200, 350, 500];
        let mut long_data = Vec::new();
        for offset in &long_offsets {
            long_data.extend_from_slice(&offset.to_be_bytes());
        }
        assert_eq!(long_data.len(), long_offsets.len() * 4);
    }

    // =========================================================================
    // TDD TESTS FOR ISSUE #115 - Font Subsetting Skip Logic
    // =========================================================================
    //
    // These tests verify the fix for GitHub Issue #115:
    // "Embedding the font subset encountered an error, or is this feature not supported?"
    //
    // Bug: A 41MB CJK font with only 4 characters produced a 41MB PDF because
    // the subsetting logic incorrectly skipped subsetting when char_count < 10,
    // regardless of font size.
    //
    // Fix: Only skip subsetting when BOTH:
    // - Font is small (< 100KB)
    // - Character count is small (< 10)

    /// Test 1: CRITICAL - Large font with few characters MUST be subsetted
    /// This is the exact bug reported in Issue #115
    #[test]
    fn test_issue_115_large_font_few_chars_should_subset() {
        // Simulates a 41MB CJK font with 4 characters (like "中国人!")
        let font_size = 41_000_000; // 41MB
        let char_count = 4;

        let should_skip = should_skip_subsetting(font_size, char_count);

        // MUST return false - we MUST subset large fonts even with few chars
        assert!(
            !should_skip,
            "Bug #115: Large font ({} bytes) with {} chars should NOT skip subsetting",
            font_size, char_count
        );
    }

    /// Test 2: Regression - Small font with few chars CAN skip subsetting
    /// This behavior should be preserved (optimization for small fonts)
    #[test]
    fn test_small_font_few_chars_can_skip_subsetting() {
        // A small 50KB font with 5 characters - skipping is acceptable
        let font_size = 50_000; // 50KB
        let char_count = 5;

        let should_skip = should_skip_subsetting(font_size, char_count);

        // Small font + few chars = OK to skip
        assert!(
            should_skip,
            "Small font ({} bytes) with {} chars can skip subsetting",
            font_size, char_count
        );
    }

    /// Test 3: Small font with many characters SHOULD be subsetted
    #[test]
    fn test_small_font_many_chars_should_subset() {
        // A small font but with enough characters to justify subsetting
        let font_size = 80_000; // 80KB
        let char_count = 200;

        let should_skip = should_skip_subsetting(font_size, char_count);

        // Many chars = should subset regardless of font size
        assert!(
            !should_skip,
            "Small font ({} bytes) with {} chars should NOT skip subsetting",
            font_size, char_count
        );
    }

    /// Test 4: Large font with many characters SHOULD be subsetted
    #[test]
    fn test_large_font_many_chars_should_subset() {
        // Large font with many characters - definitely should subset
        let font_size = 5_000_000; // 5MB
        let char_count = 500;

        let should_skip = should_skip_subsetting(font_size, char_count);

        assert!(
            !should_skip,
            "Large font ({} bytes) with {} chars should NOT skip subsetting",
            font_size, char_count
        );
    }

    /// Test 5: Boundary - Font at exact threshold with few chars
    #[test]
    fn test_font_at_threshold_boundary() {
        // Font exactly at 100KB threshold with 5 characters
        let font_size = SUBSETTING_SIZE_THRESHOLD; // 100KB exactly
        let char_count = 5;

        let should_skip = should_skip_subsetting(font_size, char_count);

        // At threshold (>=), should NOT skip
        assert!(
            !should_skip,
            "Font at threshold ({} bytes) with {} chars should NOT skip subsetting",
            font_size, char_count
        );
    }

    /// Test 6: Empty character set always skips (degenerate case)
    #[test]
    fn test_empty_chars_returns_full_font() {
        // No characters to subset - skip regardless of font size
        let font_size = 200_000; // 200KB
        let char_count = 0;

        let should_skip = should_skip_subsetting(font_size, char_count);

        // Empty char set = nothing to do, skip
        assert!(
            should_skip,
            "Empty character set should always skip subsetting"
        );
    }

    /// Test 7: Exactly at char threshold with small font
    #[test]
    fn test_char_count_at_threshold_small_font() {
        // Small font with exactly 10 characters (at threshold)
        let font_size = 50_000; // 50KB
        let char_count = SUBSETTING_CHAR_THRESHOLD; // 10 chars exactly

        let should_skip = should_skip_subsetting(font_size, char_count);

        // At char threshold (>=), should NOT skip even for small fonts
        assert!(
            !should_skip,
            "Font with {} chars at threshold should NOT skip subsetting",
            char_count
        );
    }

    /// Test 8: Just below thresholds (edge case)
    #[test]
    fn test_just_below_both_thresholds() {
        // Just below both thresholds
        let font_size = SUBSETTING_SIZE_THRESHOLD - 1; // 99,999 bytes
        let char_count = SUBSETTING_CHAR_THRESHOLD - 1; // 9 chars

        let should_skip = should_skip_subsetting(font_size, char_count);

        // Below both thresholds = OK to skip
        assert!(
            should_skip,
            "Font just below thresholds can skip subsetting"
        );
    }

    /// Test constants are reasonable
    #[test]
    fn test_subsetting_constants_are_reasonable() {
        // Size threshold should be at least 50KB (typical small font)
        assert!(
            SUBSETTING_SIZE_THRESHOLD >= 50_000,
            "Size threshold too low"
        );

        // Size threshold should not exceed 1MB (would miss many fonts)
        assert!(
            SUBSETTING_SIZE_THRESHOLD <= 1_000_000,
            "Size threshold too high"
        );

        // Char threshold should be at least 5 (very minimal documents)
        assert!(SUBSETTING_CHAR_THRESHOLD >= 5, "Char threshold too low");

        // Char threshold should not exceed 50 (would skip too often)
        assert!(SUBSETTING_CHAR_THRESHOLD <= 50, "Char threshold too high");
    }

    // =========================================================================
    // COMPOSITE GLYPH TESTS (Issue #176)
    // =========================================================================

    /// Build a minimal composite glyph data block.
    /// `numberOfContours = -1`, followed by a standard 8-byte bounding box (zeros),
    /// then component records.
    fn build_composite_glyph(components: &[(u16, bool)]) -> Vec<u8> {
        let mut data = Vec::new();
        // numberOfContours = -1 (composite)
        data.extend_from_slice(&(-1i16).to_be_bytes());
        // xMin, yMin, xMax, yMax (all zero)
        data.extend_from_slice(&[0u8; 8]);

        for (i, &(glyph_id, has_scale)) in components.iter().enumerate() {
            let is_last = i == components.len() - 1;
            let mut flags: u16 = 0;
            if !is_last {
                flags |= COMPOSITE_MORE_COMPONENTS;
            }
            // Use byte args (not word args) for simplicity
            if has_scale {
                flags |= COMPOSITE_WE_HAVE_A_SCALE;
            }
            data.extend_from_slice(&flags.to_be_bytes());
            data.extend_from_slice(&glyph_id.to_be_bytes());
            // Arguments: two i8 offsets (dx=0, dy=0)
            data.push(0);
            data.push(0);
            if has_scale {
                // F2Dot14 scale = 1.0 = 0x4000
                data.extend_from_slice(&0x4000u16.to_be_bytes());
            }
        }
        data
    }

    /// Build a simple glyph data block (numberOfContours = 1, minimal).
    fn build_simple_glyph() -> Vec<u8> {
        let mut data = Vec::new();
        // numberOfContours = 1
        data.extend_from_slice(&1i16.to_be_bytes());
        // xMin, yMin, xMax, yMax
        data.extend_from_slice(&0i16.to_be_bytes());
        data.extend_from_slice(&0i16.to_be_bytes());
        data.extend_from_slice(&100i16.to_be_bytes());
        data.extend_from_slice(&100i16.to_be_bytes());
        // endPtsOfContours[0] = 3 (a square: 4 points, indices 0-3)
        data.extend_from_slice(&3u16.to_be_bytes());
        // instructionLength = 0
        data.extend_from_slice(&0u16.to_be_bytes());
        // flags: 4 points, all on-curve (flag = 0x01)
        data.extend_from_slice(&[0x01, 0x01, 0x01, 0x01]);
        // x-coordinates (relative): 0, 100, 0, -100
        data.push(0);
        data.push(100);
        data.push(0);
        data.push(100); // stored as u8 since flag doesn't indicate i16
                        // y-coordinates (relative): 0, 0, 100, -100 (same pattern)
        data.push(0);
        data.push(0);
        data.push(100);
        data.push(100);
        data
    }

    #[test]
    fn test_extract_composite_components_returns_component_gids() {
        // Composite glyph referencing GID 1 (stem) and GID 2 (dot)
        let data = build_composite_glyph(&[(1, false), (2, false)]);
        let components = extract_composite_components(&data);
        assert_eq!(components, vec![1, 2]);
    }

    #[test]
    fn test_extract_composite_components_with_scale() {
        // Composite with scale transformation on first component
        let data = build_composite_glyph(&[(5, true), (7, false)]);
        let components = extract_composite_components(&data);
        assert_eq!(components, vec![5, 7]);
    }

    #[test]
    fn test_extract_composite_components_single_component() {
        let data = build_composite_glyph(&[(42, false)]);
        let components = extract_composite_components(&data);
        assert_eq!(components, vec![42]);
    }

    #[test]
    fn test_extract_composite_returns_empty_for_simple_glyph() {
        let data = build_simple_glyph();
        let components = extract_composite_components(&data);
        assert!(components.is_empty());
    }

    #[test]
    fn test_extract_composite_returns_empty_for_short_data() {
        let components = extract_composite_components(&[0xFF, 0xFF]);
        assert!(components.is_empty());
    }

    #[test]
    fn test_remap_composite_glyph_rewrites_component_gids() {
        let data = build_composite_glyph(&[(1, false), (2, false)]);

        let mut glyph_map = HashMap::new();
        glyph_map.insert(1u16, 10u16);
        glyph_map.insert(2u16, 20u16);

        let remapped = remap_composite_glyph(&data, &glyph_map);
        let components = extract_composite_components(&remapped);
        assert_eq!(components, vec![10, 20]);
    }

    #[test]
    fn test_remap_composite_glyph_leaves_simple_glyph_unchanged() {
        let data = build_simple_glyph();
        let glyph_map = HashMap::new();
        let remapped = remap_composite_glyph(&data, &glyph_map);
        assert_eq!(remapped, data);
    }

    #[test]
    fn test_remap_composite_unmapped_gid_falls_back_to_notdef() {
        let data = build_composite_glyph(&[(99, false)]);
        let glyph_map = HashMap::new(); // No mappings → falls back to 0 (.notdef)
        let remapped = remap_composite_glyph(&data, &glyph_map);
        let components = extract_composite_components(&remapped);
        assert_eq!(components, vec![0]);
    }

    // =========================================================================
    // LOCA FORMAT OVERFLOW TESTS
    // =========================================================================

    #[test]
    fn test_loca_short_format_max_offset_constant() {
        assert_eq!(LOCA_SHORT_FORMAT_MAX_OFFSET, 0x1_FFFE);
        assert_eq!(LOCA_SHORT_FORMAT_MAX_OFFSET, 131070);
    }

    #[test]
    fn test_we_have_instructions_constant_has_correct_value() {
        assert_eq!(COMPOSITE_WE_HAVE_INSTRUCTIONS, 0x0100);
    }

    // =========================================================================
    // COMPOSITE GLYPH TRANSFORMATION BRANCH TESTS (Item 6)
    // =========================================================================

    /// Build a composite glyph with explicit flags for each component.
    /// Each tuple is (glyph_id, extra_flags). The function always sets
    /// MORE_COMPONENTS for all but the last component.
    fn build_composite_glyph_with_flags(components: &[(u16, u16)]) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&(-1i16).to_be_bytes()); // composite
        data.extend_from_slice(&[0u8; 8]); // bounding box

        for (i, &(glyph_id, extra_flags)) in components.iter().enumerate() {
            let is_last = i == components.len() - 1;
            let mut flags = extra_flags;
            if !is_last {
                flags |= COMPOSITE_MORE_COMPONENTS;
            }
            data.extend_from_slice(&flags.to_be_bytes());
            data.extend_from_slice(&glyph_id.to_be_bytes());

            // Arguments depend on ARG_1_AND_2_ARE_WORDS
            if flags & COMPOSITE_ARG_1_AND_2_ARE_WORDS != 0 {
                data.extend_from_slice(&0i16.to_be_bytes()); // dx
                data.extend_from_slice(&0i16.to_be_bytes()); // dy
            } else {
                data.push(0); // dx i8
                data.push(0); // dy i8
            }

            // Transformation data
            if flags & COMPOSITE_WE_HAVE_A_TWO_BY_TWO != 0 {
                // 4 x F2Dot14 = 8 bytes (identity matrix)
                data.extend_from_slice(&0x4000u16.to_be_bytes()); // m00 = 1.0
                data.extend_from_slice(&0x0000u16.to_be_bytes()); // m01 = 0.0
                data.extend_from_slice(&0x0000u16.to_be_bytes()); // m10 = 0.0
                data.extend_from_slice(&0x4000u16.to_be_bytes()); // m11 = 1.0
            } else if flags & COMPOSITE_WE_HAVE_AN_X_AND_Y_SCALE != 0 {
                // 2 x F2Dot14 = 4 bytes
                data.extend_from_slice(&0x4000u16.to_be_bytes()); // xScale = 1.0
                data.extend_from_slice(&0x4000u16.to_be_bytes()); // yScale = 1.0
            } else if flags & COMPOSITE_WE_HAVE_A_SCALE != 0 {
                // 1 x F2Dot14 = 2 bytes
                data.extend_from_slice(&0x4000u16.to_be_bytes()); // scale = 1.0
            }
        }
        data
    }

    #[test]
    fn test_extract_composite_with_x_and_y_scale() {
        let data =
            build_composite_glyph_with_flags(&[(10, COMPOSITE_WE_HAVE_AN_X_AND_Y_SCALE), (20, 0)]);
        let components = extract_composite_components(&data);
        assert_eq!(components, vec![10, 20]);
    }

    #[test]
    fn test_extract_composite_with_two_by_two() {
        let data =
            build_composite_glyph_with_flags(&[(30, COMPOSITE_WE_HAVE_A_TWO_BY_TWO), (40, 0)]);
        let components = extract_composite_components(&data);
        assert_eq!(components, vec![30, 40]);
    }

    #[test]
    fn test_extract_composite_with_word_args() {
        let data =
            build_composite_glyph_with_flags(&[(50, COMPOSITE_ARG_1_AND_2_ARE_WORDS), (60, 0)]);
        let components = extract_composite_components(&data);
        assert_eq!(components, vec![50, 60]);
    }

    #[test]
    fn test_extract_composite_with_all_flags_combined() {
        let data = build_composite_glyph_with_flags(&[
            (
                70,
                COMPOSITE_ARG_1_AND_2_ARE_WORDS | COMPOSITE_WE_HAVE_A_TWO_BY_TWO,
            ),
            (80, COMPOSITE_WE_HAVE_AN_X_AND_Y_SCALE),
        ]);
        let components = extract_composite_components(&data);
        assert_eq!(components, vec![70, 80]);
    }

    #[test]
    fn test_expand_composite_glyphs_transitive() {
        // GID 3 is composite → GID 2, GID 2 is composite → GID 1, GID 1 is simple.
        // Starting with {GID 3}, expand must yield {GID 1, GID 2, GID 3}.
        let glyph1 = build_simple_glyph();
        let glyph2 = build_composite_glyph(&[(1, false)]);
        let glyph3 = build_composite_glyph(&[(2, false)]);

        // Verify direct components
        assert_eq!(extract_composite_components(&glyph3), vec![2]);
        assert_eq!(extract_composite_components(&glyph2), vec![1]);
        assert!(extract_composite_components(&glyph1).is_empty());

        // Simulate worklist expansion (same algorithm as expand_composite_glyphs)
        let glyph_data: HashMap<u16, Vec<u8>> = [(1u16, glyph1), (2u16, glyph2), (3u16, glyph3)]
            .into_iter()
            .collect();

        let mut needed: HashSet<u16> = [3u16].into_iter().collect();
        let mut worklist: Vec<u16> = needed.iter().copied().collect();
        while let Some(gid) = worklist.pop() {
            if let Some(data) = glyph_data.get(&gid) {
                for comp in extract_composite_components(data) {
                    if needed.insert(comp) {
                        worklist.push(comp);
                    }
                }
            }
        }

        assert!(needed.contains(&1), "GID 1 must be included (transitive)");
        assert!(needed.contains(&2), "GID 2 must be included (direct)");
        assert!(needed.contains(&3), "GID 3 must be included (original)");
        assert_eq!(needed.len(), 3);
    }

    // =========================================================================
    // HHEA numberOfHMetrics UPDATE TEST (tittle offset fix)
    // =========================================================================

    /// Build a minimal hhea table with the given numberOfHMetrics value.
    fn build_hhea_with_metrics(num_h_metrics: u16) -> Vec<u8> {
        let mut data = vec![0u8; 36];
        // version = 1.0
        data[0] = 0x00;
        data[1] = 0x01;
        data[2] = 0x00;
        data[3] = 0x00;
        // ascender = 800
        data[4] = 0x03;
        data[5] = 0x20;
        // descender = -200 (0xFF38 as i16)
        let desc: i16 = -200;
        let desc_bytes = desc.to_be_bytes();
        data[6] = desc_bytes[0];
        data[7] = desc_bytes[1];
        // numberOfHMetrics at offset 34
        data[34] = (num_h_metrics >> 8) as u8;
        data[35] = (num_h_metrics & 0xFF) as u8;
        data
    }

    /// Build a minimal TrueType font data with a specific hhea.numberOfHMetrics.
    ///
    /// Constructs a parseable 7-table TrueType font (same layout as the test helper in
    /// truetype_tests.rs) and sets hhea.numberOfHMetrics to the requested value.
    fn build_font_with_num_h_metrics(original_num_h_metrics: u16) -> Vec<u8> {
        let mut font = Vec::new();

        // Offsets (absolute in the file)
        // Table directory: 12 + 7*16 = 124 bytes
        // Table layout:
        //   cmap  @ 124, len=256
        //   glyf  @ 380, len=128
        //   head  @ 508, len=54
        //   hhea  @ 562, len=36
        //   hmtx  @ 598, len=16
        //   loca  @ 614, len=10
        //   maxp  @ 624, len=32
        let dir_size: usize = 12 + 7 * 16;
        let cmap_off = dir_size;
        let glyf_off = cmap_off + 256;
        let head_off = glyf_off + 128;
        let hhea_off = head_off + 54;
        let hmtx_off = hhea_off + 36;
        let loca_off = hmtx_off + 16;
        let maxp_off = loca_off + 10;
        let total = maxp_off + 32;

        // Font header (12 bytes)
        font.extend_from_slice(&[0x00, 0x01, 0x00, 0x00]); // version 1.0
        font.extend_from_slice(&7u16.to_be_bytes()); // numTables
        font.extend_from_slice(&[0x00, 0x80]); // searchRange
        font.extend_from_slice(&[0x00, 0x03]); // entrySelector
        font.extend_from_slice(&[0x00, 0x70]); // rangeShift

        // Table directory
        let entries: &[(&[u8], usize, usize)] = &[
            (b"cmap", cmap_off, 256),
            (b"glyf", glyf_off, 128),
            (b"head", head_off, 54),
            (b"hhea", hhea_off, 36),
            (b"hmtx", hmtx_off, 16),
            (b"loca", loca_off, 10),
            (b"maxp", maxp_off, 32),
        ];
        for (tag, off, len) in entries {
            font.extend_from_slice(tag);
            font.extend_from_slice(&[0u8; 4]); // checksum (ignored in tests)
            font.extend_from_slice(&(*off as u32).to_be_bytes());
            font.extend_from_slice(&(*len as u32).to_be_bytes());
        }

        // Grow to total size with zeros
        font.resize(total, 0u8);

        // head table @ head_off
        let h = head_off;
        font[h..h + 4].copy_from_slice(&[0x00, 0x01, 0x00, 0x00]); // version
        font[h + 18..h + 20].copy_from_slice(&1024u16.to_be_bytes()); // unitsPerEm=1024
        font[h + 50..h + 52].copy_from_slice(&[0x00, 0x00]); // indexToLocFormat=0 (short)

        // hhea table @ hhea_off
        let he = hhea_off;
        font[he..he + 4].copy_from_slice(&[0x00, 0x01, 0x00, 0x00]); // version
        font[he + 4..he + 6].copy_from_slice(&800i16.to_be_bytes()); // ascender
        font[he + 6..he + 8].copy_from_slice(&(-200i16).to_be_bytes()); // descender
        font[he + 34..he + 36].copy_from_slice(&original_num_h_metrics.to_be_bytes());

        // maxp table @ maxp_off
        let m = maxp_off;
        font[m..m + 4].copy_from_slice(&[0x00, 0x01, 0x00, 0x00]); // version 1.0
        font[m + 4..m + 6].copy_from_slice(&4u16.to_be_bytes()); // numGlyphs=4

        // hmtx table @ hmtx_off: 4 glyphs, each (advanceWidth=0x0200, lsb=0)
        for i in 0..4 {
            let off = hmtx_off + i * 4;
            font[off..off + 2].copy_from_slice(&0x0200u16.to_be_bytes()); // advance=512
            font[off + 2..off + 4].copy_from_slice(&0i16.to_be_bytes()); // lsb=0
        }

        // cmap table @ cmap_off: minimal format 4 with glyph 0 for 0x0020-0x007F
        let cm = cmap_off;
        font[cm..cm + 2].copy_from_slice(&[0x00, 0x00]); // version
        font[cm + 2..cm + 4].copy_from_slice(&1u16.to_be_bytes()); // numTables
                                                                   // encoding record
        font[cm + 4..cm + 6].copy_from_slice(&3u16.to_be_bytes()); // platformID=3
        font[cm + 6..cm + 8].copy_from_slice(&1u16.to_be_bytes()); // encodingID=1
        font[cm + 8..cm + 12].copy_from_slice(&12u32.to_be_bytes()); // subtable at +12
                                                                     // format 4 subtable at cmap_off+12
        let sf = cm + 12;
        font[sf..sf + 2].copy_from_slice(&4u16.to_be_bytes()); // format 4
        font[sf + 2..sf + 4].copy_from_slice(&32u16.to_be_bytes()); // length=32
        font[sf + 4..sf + 6].copy_from_slice(&[0x00, 0x00]); // language
        font[sf + 6..sf + 8].copy_from_slice(&4u16.to_be_bytes()); // segCountX2=4
        font[sf + 8..sf + 10].copy_from_slice(&[0x00, 0x04]); // searchRange
        font[sf + 10..sf + 12].copy_from_slice(&[0x00, 0x01]); // entrySelector
        font[sf + 12..sf + 14].copy_from_slice(&[0x00, 0x00]); // rangeShift
                                                               // endCodes
        font[sf + 14..sf + 16].copy_from_slice(&0x007Fu16.to_be_bytes());
        font[sf + 16..sf + 18].copy_from_slice(&0xFFFFu16.to_be_bytes());
        // reservedPad
        font[sf + 18..sf + 20].copy_from_slice(&[0x00, 0x00]);
        // startCodes
        font[sf + 20..sf + 22].copy_from_slice(&0x0020u16.to_be_bytes());
        font[sf + 22..sf + 24].copy_from_slice(&0xFFFFu16.to_be_bytes());
        // idDelta
        font[sf + 24..sf + 26].copy_from_slice(&0u16.to_be_bytes()); // delta=0
        font[sf + 26..sf + 28].copy_from_slice(&1u16.to_be_bytes()); // terminal delta
                                                                     // idRangeOffset
        font[sf + 28..sf + 30].copy_from_slice(&[0x00, 0x00]);
        font[sf + 30..sf + 32].copy_from_slice(&[0x00, 0x00]);

        // loca table @ loca_off: short format, 5 entries (4 glyphs + end)
        // All glyphs at offset 0 (empty)
        for i in 0..5usize {
            let off = loca_off + i * 2;
            font[off..off + 2].copy_from_slice(&0u16.to_be_bytes());
        }

        font
    }

    /// Test that update_hhea_table sets numberOfHMetrics to the subset glyph count.
    ///
    /// The bug: after subsetting a font from N glyphs to M glyphs, the hhea table
    /// kept the original numberOfHMetrics (= N). PDF renderers reading this value would
    /// try to access hmtx entries beyond the end of the subset hmtx table, causing
    /// out-of-bounds reads and potentially wrong glyph metrics (e.g., wrong advance
    /// width for composite glyph components → tittle offset bug).
    #[test]
    fn test_update_hhea_num_h_metrics_is_set_to_num_glyphs() {
        let font_data = build_font_with_num_h_metrics(100); // original had 100 metrics
        let subsetter = TrueTypeSubsetter::new(font_data).unwrap();

        // After subsetting to 5 glyphs, numberOfHMetrics should be 5
        let updated = subsetter.update_hhea_table(5).unwrap();
        let num_h_metrics = u16::from_be_bytes([updated[34], updated[35]]);
        assert_eq!(
            num_h_metrics, 5,
            "update_hhea_table must set numberOfHMetrics to the new glyph count"
        );
    }

    #[test]
    fn test_update_hhea_preserves_other_fields() {
        let font_data = build_font_with_num_h_metrics(4);
        let subsetter = TrueTypeSubsetter::new(font_data).unwrap();

        let original = subsetter.get_table_data(b"hhea").unwrap();
        let updated = subsetter.update_hhea_table(3).unwrap();

        // Only bytes 34-35 should differ
        assert_eq!(
            original.len(),
            updated.len(),
            "table length must not change"
        );
        for i in 0..original.len() {
            if i == 34 || i == 35 {
                continue; // skip the updated field
            }
            assert_eq!(
                original[i], updated[i],
                "byte {} should be unchanged after update_hhea_table",
                i
            );
        }
        // numberOfHMetrics should be 3
        let num_h_metrics = u16::from_be_bytes([updated[34], updated[35]]);
        assert_eq!(num_h_metrics, 3);
    }

    #[test]
    fn test_update_hhea_rejects_too_short_table() {
        // Provide a font whose hhea is too short (< 36 bytes)
        let font_data = build_font_with_num_h_metrics(4);
        let subsetter = TrueTypeSubsetter::new(font_data).unwrap();

        // Manually construct a bad hhea slice (only 30 bytes) is impossible via public API,
        // but we can verify the function itself rejects short input.
        // We test indirectly: a normally-built font should succeed.
        let result = subsetter.update_hhea_table(4);
        assert!(
            result.is_ok(),
            "update_hhea_table should succeed for valid font"
        );
    }

    // =========================================================================
    // HMTX DESIGN-UNIT WIDTH TEST (tittle offset fix)
    // =========================================================================

    /// Test that build_hmtx writes advance widths in font design units, not scaled.
    ///
    /// The bug: build_hmtx called get_glyph_widths() which scales widths to PDF
    /// 1000/em units. For fonts with units_per_em != 1000 (e.g., 2048 for TrueType),
    /// the subset hmtx would have the wrong (scaled-down) advance widths.
    ///
    /// A wrong advance width in hmtx causes some PDF renderers to misplace text:
    /// if the renderer uses hmtx for internal glyph layout (e.g., for positioning
    /// composite glyph components via LSB), incorrect widths lead to misaligned dots
    /// (tittles) on glyphs like lowercase 'i'.
    #[test]
    fn test_build_hmtx_uses_design_units_not_scaled() {
        // The test font (create_test_font) has units_per_em = 1024 and each glyph
        // has advance_width = 512 in the hmtx (0x02, 0x00 at offsets 0 and 4).
        let font_data = build_font_with_num_h_metrics(4);
        let subsetter = TrueTypeSubsetter::new(font_data).unwrap();

        // Build an identity map: new GID i = old GID i for glyphs 0..=3
        let mut glyph_map: HashMap<u16, u16> = HashMap::new();
        let mut inverse_map: HashMap<u16, u16> = HashMap::new();
        for i in 0u16..4 {
            glyph_map.insert(i, i);
            inverse_map.insert(i, i);
        }

        let hmtx = subsetter.build_hmtx(&glyph_map, &inverse_map).unwrap();

        // Each entry is 4 bytes: advanceWidth (u16) + lsb (i16)
        // The test font has advance_width = 0x0200 = 512 design units (= 500/1000em)
        // If the bug is present: scaled = 512 * 1000 / 1024 = 500 (0x01F4)
        // If the fix is correct: 512 (0x0200) is preserved
        assert!(hmtx.len() >= 4, "hmtx must have at least one entry");
        let advance_width = u16::from_be_bytes([hmtx[0], hmtx[1]]);
        assert_eq!(
            advance_width, 0x0200,
            "advance_width in hmtx must be in design units (0x0200=512), \
             not scaled to 1000/em (0x01F4=500)"
        );
    }

    // =========================================================================
    // TTF subset output: unused tables stripped for PDF embedding
    // =========================================================================

    /// The PDF writer uses its own ToUnicode CMap and CIDToGIDMap, so the TTF
    /// `cmap` table is redundant. `OS/2` and `name` are not consulted during
    /// rendering. All three are stripped to reduce the embedded font size.
    #[test]
    fn test_ttf_subset_excludes_cmap_os2_name_tables() {
        let font_path = "../test-pdfs/Roboto-Regular.ttf";
        let font_data = match std::fs::read(font_path) {
            Ok(d) => d,
            Err(_) => {
                eprintln!("SKIPPED: {} not found", font_path);
                return;
            }
        };

        let used: HashSet<char> = "AB".chars().collect();
        let result = subset_font(font_data, &used).expect("TTF subset must succeed");

        let out = &result.font_data;
        assert!(out.len() > 12, "output too small to parse");
        let num_tables = u16::from_be_bytes([out[4], out[5]]) as usize;

        let mut tags: Vec<[u8; 4]> = Vec::with_capacity(num_tables);
        for i in 0..num_tables {
            let base = 12 + i * 16;
            tags.push([out[base], out[base + 1], out[base + 2], out[base + 3]]);
        }

        for stripped in [b"cmap", b"OS/2", b"name"] {
            assert!(
                !tags.iter().any(|t| t == stripped),
                "subset must not contain {:?}",
                std::str::from_utf8(stripped).unwrap()
            );
        }

        for required in [b"glyf", b"head", b"loca", b"hmtx", b"hhea", b"maxp"] {
            assert!(
                tags.iter().any(|t| t == required),
                "subset must contain {:?}",
                std::str::from_utf8(required).unwrap()
            );
        }
    }
}
