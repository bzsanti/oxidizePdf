//! Enhanced text extraction with CMap/ToUnicode support
//!
//! This module extends the basic text extraction to properly handle
//! CMap and ToUnicode mappings for accurate character decoding.

use crate::parser::document::PdfDocument;
use crate::parser::objects::{PdfDictionary, PdfName, PdfObject, PdfStream};
use crate::parser::{ParseError, ParseOptions, ParseResult};
use crate::text::cid_to_unicode::AdobeCidCollection;
use crate::text::cmap::CMap;
use crate::text::pdf_simple_encodings;
use std::collections::HashMap;
use std::io::{Read, Seek};
use std::sync::OnceLock;

/// Only Adobe registries may use the bundled Adobe collection mappings.
pub(crate) fn resolve_cid_ordering<R: Read + Seek>(
    document: &PdfDocument<R>,
    font: &PdfDictionary,
) -> ParseResult<Option<String>> {
    let Some(info) = font.get("CIDSystemInfo") else {
        return Ok(None);
    };
    let info = document.resolve(info)?;
    let Some(info) = info.as_dict() else {
        return Err(ParseError::SyntaxError {
            position: 0,
            message: "CIDSystemInfo must be a dictionary".into(),
        });
    };
    let string = |key: &str| -> ParseResult<Option<String>> {
        info.get(key)
            .map(|object| {
                document
                    .resolve(object)
                    .map(|value| value.as_string().map(|s| s.to_text()))
            })
            .transpose()
            .map(Option::flatten)
    };
    if string("Registry")?.as_deref() != Some("Adobe") {
        return Ok(None);
    }
    if let Some(value) = info.get("Supplement") {
        let value = document.resolve(value)?;
        if !value.as_integer().is_some_and(|value| value >= 0) {
            // Malformed explicit metadata cannot authorize Adobe Unicode.
            // Missing Supplement retains the existing recovery policy.
            return Ok(None);
        }
    }
    string("Ordering")
}

/// A named/known Encoding CMap cannot feed another collection's Unicode table.
/// Supplement numbers are cumulative and need not be identical (Adobe TN5099).
/// Unannotated custom maps retain their existing recovery until collection
/// evidence is available; explicit ToUnicode remains independently authoritative.
pub(crate) fn compatible_cid_ordering<'a>(
    ordering: Option<&'a str>,
    encoding: Option<&crate::text::encoding_cmap::CidEncoding>,
) -> Option<&'a str> {
    let ordering = ordering?;
    if let Some(crate::text::encoding_cmap::CidEncoding::Cmap(map)) = encoding {
        if map.invalid_collection {
            return None;
        }
        if let Some((registry, expected)) = &map.collection {
            if registry != "Adobe" || expected != ordering {
                return None;
            }
        }
    }
    Some(ordering)
}

fn metric_error(message: &str) -> ParseError {
    ParseError::SyntaxError {
        position: 0,
        message: message.into(),
    }
}

fn finite_metric(value: &PdfObject) -> ParseResult<f64> {
    value
        .as_real()
        .filter(|value| value.is_finite())
        .ok_or_else(|| metric_error("font metric must be a finite number"))
}

/// Optional metric dictionary entries with null values are absent (§7.3.9).
/// Arrays are left intact: a null array element is not an omitted entry.
pub(crate) fn without_null_metrics<'a, R: Read + Seek>(
    dict: &'a PdfDictionary,
    document: &PdfDocument<R>,
) -> std::borrow::Cow<'a, PdfDictionary> {
    let mut result = std::borrow::Cow::Borrowed(dict);
    for key in [
        "FirstChar",
        "LastChar",
        "Widths",
        "W",
        "DW",
        "MissingWidth",
        "FontDescriptor",
    ] {
        let absent = match dict.get(key) {
            Some(PdfObject::Null) => true,
            Some(value @ PdfObject::Reference(_, _)) => matches!(
                document.resolve(value),
                Ok(PdfObject::Null) | Err(ParseError::InvalidReference(_, _))
            ),
            _ => false,
        };
        if absent {
            result.to_mut().0.remove(&PdfName(key.into()));
        }
    }
    result
}

/// Validate simple-font byte bounds without silently wrapping malformed values.
pub(crate) fn simple_font_bounds<R: Read + Seek>(
    document: &PdfDocument<R>,
    dict: &PdfDictionary,
) -> ParseResult<(Option<u32>, Option<u32>)> {
    let bound = |key| {
        dict.get(key)
            .map(|value| {
                document
                    .resolve(value)?
                    .as_integer()
                    .and_then(|value| u8::try_from(value).ok())
                    .map(u32::from)
                    .ok_or_else(|| metric_error("Simple font bounds must be byte integers"))
            })
            .transpose()
    };
    let first = bound("FirstChar")?;
    let last = bound("LastChar")?;
    if first.zip(last).is_some_and(|(first, last)| first > last) {
        return Err(metric_error("FirstChar exceeds LastChar"));
    }
    Ok((first, last))
}

/// Shared strict reader for horizontal CID metrics. Recovery belongs to the caller.
pub(crate) fn parse_cid_widths<R: Read + Seek>(
    document: &PdfDocument<R>,
    dict: &PdfDictionary,
) -> ParseResult<CidWidths> {
    let default = dict
        .get("DW")
        .map(|value| document.resolve(value))
        .transpose()?
        .as_ref()
        .map(finite_metric)
        .transpose()?
        .unwrap_or(1000.0);
    let entries = dict
        .get("W")
        .map(|value| document.resolve(value))
        .transpose()?;
    let Some(entries) = entries else {
        return Ok(CidWidths {
            widths: HashMap::new(),
            ranges: Vec::new(),
            default_width: default,
            vertical: CidVerticalWidths::default(),
        });
    };
    let entries = entries
        .as_array()
        .ok_or_else(|| metric_error("W must be an array"))?;
    let mut explicit = HashMap::new();
    let mut ranges = Vec::new();
    let mut index = 0;
    while index < entries.0.len() {
        let first = document
            .resolve(&entries.0[index])?
            .as_integer()
            .and_then(|value| u16::try_from(value).ok())
            .map(u32::from)
            .ok_or_else(|| metric_error("invalid first CID in W"))?;
        let next = entries
            .0
            .get(index + 1)
            .map(|value| document.resolve(value))
            .transpose()?;
        match next.as_ref() {
            Some(PdfObject::Array(widths)) => {
                if widths.0.len() > usize::from(u16::MAX) + 1 - first as usize {
                    return Err(metric_error("W array exceeds CID 65535"));
                }
                for (offset, width) in widths.0.iter().enumerate() {
                    let cid = first
                        .checked_add(
                            u32::try_from(offset).map_err(|_| metric_error("W range overflow"))?,
                        )
                        .ok_or_else(|| metric_error("W range overflow"))?;
                    explicit.insert(cid, finite_metric(&document.resolve(width)?)?);
                }
                index += 2;
            }
            Some(last) => {
                let last = last
                    .as_integer()
                    .and_then(|value| u16::try_from(value).ok())
                    .map(u32::from)
                    .ok_or_else(|| metric_error("invalid last CID in W"))?;
                let width = entries
                    .0
                    .get(index + 2)
                    .ok_or_else(|| metric_error("missing W range width"))?;
                let width = finite_metric(&document.resolve(width)?)?;
                if last < first {
                    return Err(metric_error("reversed CID width range"));
                }
                ranges.push((first, last, width));
                index += 3;
            }
            None => return Err(metric_error("truncated W array")),
        }
    }
    Ok(CidWidths {
        widths: explicit,
        ranges,
        default_width: default,
        vertical: CidVerticalWidths::default(),
    })
}

/// A CIDFont's `/W` + `/DW` glyph-space widths (ISO 32000-1 §9.7.4.3),
/// indexed by **CID** — an arbitrary font-internal identifier unrelated to
/// any decoded Unicode codepoint. Only meaningful on a `CIDFontType0`/
/// `CIDFontType2` descendant font's [`FontMetrics`]; simple fonts use the
/// `first_char`/`last_char`/`widths` triple instead.
#[derive(Debug, Clone, Default)]
pub struct CidWidths {
    /// Per-CID widths (1/1000 em), parsed from `/W`'s mixed
    /// `c [w1 w2 ...]` and `cFirst cLast w` forms.
    pub widths: HashMap<u32, f64>,
    /// Uniform `/W` ranges stored without materializing every CID. Keeping
    /// ranges compact bounds parser work by input size for untrusted PDFs.
    pub ranges: Vec<(u32, u32, f64)>,
    /// `/DW`, the width (1/1000 em) for any CID absent from `widths`.
    /// Defaults to 1000 per spec when `/DW` is not present.
    pub default_width: f64,
    /// Vertical pen advances from W2/DW2, independent of glyph drawing offsets.
    pub vertical: CidVerticalWidths,
}

#[derive(Debug, Clone)]
pub struct CidVerticalWidths {
    pub advances: HashMap<u32, f64>,
    pub ranges: Vec<(u32, u32, f64)>,
    pub default_advance: f64,
}

impl Default for CidVerticalWidths {
    fn default() -> Self {
        Self {
            advances: HashMap::new(),
            ranges: Vec::new(),
            default_advance: -1000.0,
        }
    }
}

impl CidVerticalWidths {
    pub fn advance_for(&self, cid: u32) -> f64 {
        self.advances
            .get(&cid)
            .copied()
            .or_else(|| {
                self.ranges
                    .iter()
                    .rev()
                    .find(|(first, last, _)| (*first..=*last).contains(&cid))
                    .map(|(_, _, advance)| *advance)
            })
            .unwrap_or(self.default_advance)
    }
}

impl CidWidths {
    /// Width (1/1000 em) for `cid`: its `/W` entry, or `/DW` otherwise.
    pub fn width_for(&self, cid: u32) -> f64 {
        self.widths
            .get(&cid)
            .copied()
            .or_else(|| {
                self.ranges
                    .iter()
                    .rev()
                    .find(|(first, last, _)| cid >= *first && cid <= *last)
                    .map(|(_, _, width)| *width)
            })
            .unwrap_or(self.default_width)
    }

    /// Narrowest positive width declared by `/W`, or `/DW` when `/W` has no
    /// usable entries. This is not assumed to be the space CID; it is only a
    /// conservative lower bound for a font-derived implicit-space heuristic.
    pub(crate) fn narrowest_positive_width(&self) -> Option<f64> {
        self.widths
            .values()
            .copied()
            .chain(self.ranges.iter().map(|(_, _, width)| *width))
            .filter(|width| width.is_finite() && *width > 0.0)
            .min_by(f64::total_cmp)
            .or_else(|| {
                (self.default_width.is_finite() && self.default_width > 0.0)
                    .then_some(self.default_width)
            })
    }
}

/// Font metrics for accurate text width calculation
#[derive(Debug, Clone)]
pub struct FontMetrics {
    /// First character code in the Widths array
    pub first_char: Option<u32>,
    /// Last character code in the Widths array
    pub last_char: Option<u32>,
    /// Character widths (in glyph space units, typically 1/1000)
    pub widths: Option<Vec<f64>>,
    /// Missing width (default width for characters not in Widths array)
    pub missing_width: Option<f64>,
    /// Kerning pairs: (char1, char2) -> adjustment
    pub kerning: Option<HashMap<(u32, u32), f64>>,
    /// CID-indexed widths (`/W`/`/DW`), for a `CIDFontType0`/`CIDFontType2`
    /// descendant font. `None` for simple fonts and for CID fonts with
    /// neither `/W` nor `/DW` present.
    pub cid_widths: Option<CidWidths>,
}

impl Default for FontMetrics {
    fn default() -> Self {
        Self {
            first_char: None,
            last_char: None,
            widths: None,
            missing_width: Some(500.0), // Default to 500 units (typical average)
            kerning: None,
            cid_widths: None,
        }
    }
}

/// Font information with CMap support
#[derive(Debug, Clone)]
pub struct FontInfo {
    /// Font name
    pub name: String,
    /// Font type (Type1, TrueType, Type0, etc.)
    pub font_type: String,
    /// Base encoding (if any)
    pub encoding: Option<String>,
    /// ToUnicode CMap (if present)
    pub to_unicode: Option<CMap>,
    /// Encoding differences
    pub differences: Option<HashMap<u8, String>>,
    /// For Type0 fonts: descendant font
    pub descendant_font: Option<Box<FontInfo>>,
    /// For CIDFonts: CIDSystemInfo Ordering (e.g., "CNS1", "GB1", "Japan1", "Korea1")
    pub cid_ordering: Option<String>,
    /// Font metrics (widths, kerning)
    pub metrics: FontMetrics,
    /// Resolved non-Identity CID encoding (code→CID), if any. Only consulted in
    /// the Type0 decode path in `decode_text_with_font`; ignored for other font
    /// types (it may be populated for them but is never read).
    pub cid_encoding: Option<crate::text::encoding_cmap::CidEncoding>,
}

/// Parser for a PDF font dictionary into the [`FontInfo`] the decoders read.
///
/// Stateless: it carries no cache and extracts no text. `R` names the reader
/// type of the document whose indirect objects the parse resolves.
pub struct CMapTextExtractor<R: Read + Seek> {
    /// PDF document reference for resource lookup
    _phantom: std::marker::PhantomData<R>,
}

impl<R: Read + Seek> CMapTextExtractor<R> {
    /// Create a new CMap-aware font parser
    pub fn new() -> Self {
        Self {
            _phantom: std::marker::PhantomData,
        }
    }

    /// Extract font information from a font dictionary
    pub fn extract_font_info(
        &mut self,
        font_dict: &PdfDictionary,
        document: &PdfDocument<R>,
    ) -> ParseResult<FontInfo> {
        let font_type = font_dict
            .get("Subtype")
            .and_then(|obj| obj.as_name())
            .ok_or_else(|| ParseError::MissingKey("Font Subtype".to_string()))?;

        let default_name = PdfName("Unknown".to_string());
        let name = font_dict
            .get("BaseFont")
            .and_then(|obj| obj.as_name())
            .unwrap_or(&default_name);

        let mut font_info = FontInfo {
            name: name.0.clone(),
            font_type: font_type.0.clone(),
            encoding: None,
            to_unicode: None,
            differences: None,
            descendant_font: None,
            cid_ordering: None,
            metrics: FontMetrics::default(),
            cid_encoding: None,
        };

        font_info.cid_ordering = resolve_cid_ordering(document, font_dict)?;

        // Extract encoding
        if let Some(encoding_obj) = font_dict.get("Encoding") {
            let resolved_encoding = document.resolve(encoding_obj).ok();
            let target_obj = resolved_encoding.as_ref().unwrap_or(encoding_obj);
            match target_obj {
                PdfObject::Name(enc_name) => {
                    font_info.encoding = Some(enc_name.0.clone());
                    if enc_name.0 != "Identity-H" && enc_name.0 != "Identity-V" {
                        font_info.cid_encoding =
                            crate::text::encoding_cmap::resolve_predefined(&enc_name.0);
                    }
                }
                PdfObject::Dictionary(enc_dict) => {
                    // Handle encoding with differences
                    if let Some(base_enc_obj) = enc_dict.get("BaseEncoding") {
                        let base_enc_resolved = document.resolve(base_enc_obj).ok();
                        let target_base = base_enc_resolved.as_ref().unwrap_or(base_enc_obj);
                        if let Some(base_enc) = target_base.as_name() {
                            font_info.encoding = Some(base_enc.0.clone());
                        }
                    }

                    if let Some(diff_obj) = enc_dict.get("Differences") {
                        let diff_resolved = document.resolve(diff_obj).ok();
                        let diff_target = diff_resolved.as_ref().unwrap_or(diff_obj);
                        if let Some(differences) = diff_target.as_array() {
                            font_info.differences =
                                Some(self.parse_encoding_differences(&differences.0, document)?);
                        }
                    }
                }
                PdfObject::Stream(stream) => {
                    let enc = Self::parse_encoding_stream(stream, document, 0)?;
                    font_info.cid_encoding =
                        Some(crate::text::encoding_cmap::CidEncoding::Cmap(enc));
                }
                _ => {}
            }
        }

        if font_info.font_type == "TrueType" && font_info.encoding.is_none() {
            let symbolic = font_dict
                .get("FontDescriptor")
                .and_then(|object| document.resolve(object).ok())
                .and_then(|object| object.as_dict().and_then(|dict| dict.get("Flags")).cloned())
                .and_then(|object| document.resolve(&object).ok())
                .and_then(|object| object.as_integer())
                .is_some_and(|flags| flags & 4 != 0);
            if symbolic {
                // Symbolic cmap codes are glyph selectors, not Unicode and
                // not necessarily Adobe Symbol. Use the existing intrinsic
                // name layer to retain one replacement per unknown byte,
                // without changing the public FontInfo struct. ToUnicode
                // still wins; explicit Differences names remain evidence.
                let mut names: HashMap<u8, String> =
                    (0..=255).map(|code| (code, "uniFFFD".to_owned())).collect();
                if let Some(differences) = font_info.differences.take() {
                    names.extend(differences.into_iter().map(|(code, name)| {
                        let name = if glyph_name_to_unicode_sequence(&name).is_some() {
                            name
                        } else {
                            "uniFFFD".to_owned()
                        };
                        (code, name)
                    }));
                }
                font_info.differences = Some(names);
            }
        }

        if font_info.font_type == "Type1" && font_info.encoding.is_none() {
            if let Some(mut intrinsic) = Self::embedded_intrinsic_encoding(font_dict, document) {
                // Unknown glyph names have no Unicode assignment; never turn
                // them into a byte-as-Unicode assertion. Explicit Differences
                // still override the intrinsic base, and ToUnicode wins later.
                for name in intrinsic.values_mut() {
                    if glyph_name_to_unicode_sequence(name).is_none() {
                        *name = "uniFFFD".to_owned();
                    }
                }
                if let Some(differences) = font_info.differences.take() {
                    intrinsic.extend(differences);
                }
                font_info.differences = Some(intrinsic);
            }
        }

        // A non-embedded standard Type1 font supplies its built-in encoding
        // when Encoding (or BaseEncoding within a Differences dictionary) is
        // absent. Do not guess an embedded font's intrinsic encoding by name.
        if font_info.encoding.is_none() && font_info.font_type == "Type1" {
            let has_embedded_program = font_dict
                .get("FontDescriptor")
                .and_then(|obj| document.resolve(obj).ok())
                .is_some_and(|obj| {
                    obj.as_dict().is_some_and(|descriptor| {
                        ["FontFile", "FontFile2", "FontFile3"]
                            .iter()
                            .any(|key| descriptor.get(key).is_some())
                    })
                });
            if !has_embedded_program {
                // PDF subset prefixes contain exactly six uppercase letters.
                let base_name = without_subset_prefix(&font_info.name);
                font_info.encoding = match base_name {
                    "Symbol" => Some("SymbolEncoding".to_owned()),
                    "ZapfDingbats" => Some("ZapfDingbatsEncoding".to_owned()),
                    _ => None,
                };
            }
        }

        // A declared ToUnicode remains authoritative even if it is damaged.
        // Resolve the whole reference chain instead of inspecting one object.
        if let Some(object) = font_dict.get("ToUnicode") {
            let parsed = Self::resolve_tounicode(object, document);
            font_info.to_unicode = match parsed {
                Ok(map) => map,
                Err(error) if document.options().strict_mode => return Err(error),
                Err(_) => {
                    // Preserve source-code boundaries while marking every
                    // undecodable code. Do not re-enable a base Unicode map.
                    let mut empty = CMap::new();
                    if font_info.font_type == "Type0" {
                        use crate::text::encoding_cmap::{CidEncoding, EncodingCMap};
                        empty.codespace_ranges = match font_info.cid_encoding.as_ref() {
                            Some(CidEncoding::Cmap(map)) => map.codespace_ranges.clone(),
                            Some(CidEncoding::Utf16Be) => {
                                EncodingCMap::identity(false).codespace_ranges
                            }
                            None if matches!(
                                font_info.encoding.as_deref(),
                                Some("Identity-H" | "Identity-V")
                            ) =>
                            {
                                EncodingCMap::identity(false).codespace_ranges
                            }
                            None => Vec::new(),
                        };
                    }
                    Some(empty)
                }
            };
        }

        // Extract font metrics (Widths, FirstChar, LastChar)
        font_info.metrics = self.extract_font_metrics(font_dict, document)?;

        // Handle Type0 (composite) fonts
        if font_type.as_str() == "Type0" {
            // The DescendantFonts value and the CIDFont element inside it may
            // each be written inline or as an indirect reference: ISO 32000-1
            // Table 121 types the entry as "array" with no reference
            // requirement, §7.3.6 lets array elements be dictionaries, and
            // §7.3.7 lets a dictionary value be any kind of object — where
            // the spec wants a reference it says so (Table 117 marks the
            // CIDFont's FontDescriptor "shall be an indirect reference"), and
            // the DescendantFonts row carries no such words. The same argument
            // as #463, which fixed this for /Font resource entries. Producers
            // do use the inline form (ReportLab's UnicodeCIDFont writes the
            // CIDFont as a direct dictionary). Reading only the
            // reference-inside-direct-array combination left `descendant_font`
            // empty, which silently skipped the `cid_encoding` branch in
            // `decode_text_with_font` and fell back to byte-wise decoding.
            let resolved_array;
            let descendant_fonts = match font_dict.get("DescendantFonts") {
                Some(PdfObject::Array(array)) => Some(array),
                Some(PdfObject::Reference(num, gen)) => match document.get_object(*num, *gen) {
                    Ok(PdfObject::Array(array)) => {
                        resolved_array = array;
                        Some(&resolved_array)
                    }
                    _ => None,
                },
                _ => None,
            };
            let descendant = match descendant_fonts.and_then(|array| array.0.first()) {
                Some(PdfObject::Dictionary(dict)) => Some(self.extract_font_info(dict, document)?),
                Some(PdfObject::Reference(num, gen)) => match document.get_object(*num, *gen) {
                    Ok(PdfObject::Dictionary(dict)) => {
                        Some(self.extract_font_info(&dict, document)?)
                    }
                    _ => None,
                },
                _ => None,
            };
            if let Some(descendant) = descendant {
                font_info.descendant_font = Some(Box::new(descendant));
            }
        }

        Ok(font_info)
    }

    pub(crate) fn embedded_intrinsic_encoding(
        font: &PdfDictionary,
        document: &PdfDocument<R>,
    ) -> Option<HashMap<u8, String>> {
        let descriptor = document.resolve(font.get("FontDescriptor")?).ok()?;
        let descriptor = descriptor.as_dict()?;
        if let Some(object) = descriptor.get("FontFile") {
            let object = document.resolve(object).ok()?;
            if let PdfObject::Stream(stream) = object {
                let data = document
                    .decode_stream_with_limit(&stream, 8 * 1024 * 1024)
                    .ok()?;
                let clear_length = document
                    .resolve(stream.dict.get("Length1")?)
                    .ok()?
                    .as_integer()
                    .and_then(|length| usize::try_from(length).ok())?;
                return super::intrinsic_encoding::type1(data.get(..clear_length)?);
            }
        }
        let object = document.resolve(descriptor.get("FontFile3")?).ok()?;
        let PdfObject::Stream(stream) = object else {
            return None;
        };
        let data = document
            .decode_stream_with_limit(&stream, 8 * 1024 * 1024)
            .ok()?;
        match stream.dict.get("Subtype")?.as_name()?.as_str() {
            "Type1C" => super::intrinsic_encoding::cff(&data),
            "OpenType" => {
                super::intrinsic_encoding::cff(super::intrinsic_encoding::opentype_cff(&data)?)
            }
            _ => None,
        }
    }

    /// Parse encoding differences array
    fn parse_encoding_differences(
        &self,
        differences: &[PdfObject],
        document: &PdfDocument<R>,
    ) -> ParseResult<HashMap<u8, String>> {
        let mut diff_map = HashMap::new();
        let mut current_code = None;

        for item in differences {
            let resolved = document.resolve(item).ok();
            let target = resolved.as_ref().unwrap_or(item);
            match target {
                PdfObject::Integer(code) => {
                    current_code = u8::try_from(*code).ok();
                }
                PdfObject::Name(name) => {
                    if let Some(code) = current_code {
                        diff_map.insert(code, name.0.clone());
                        current_code = code.checked_add(1);
                    }
                }
                _ => {}
            }
        }

        Ok(diff_map)
    }

    /// Resolve at most sixteen stream levels. This also bounds cycles without
    /// allocating a graph or expanding ranges into per-code entries.
    fn cmap_error(message: String) -> ParseError {
        ParseError::SyntaxError {
            position: 0,
            message,
        }
    }

    fn cmap_depth(depth: usize) -> ParseResult<()> {
        if depth >= 16 {
            return Err(Self::cmap_error(
                "UseCMap inheritance exceeds 16 stream levels (possible cycle)".into(),
            ));
        }
        Ok(())
    }

    pub(crate) fn parse_encoding_stream(
        stream: &PdfStream,
        document: &PdfDocument<R>,
        depth: usize,
    ) -> ParseResult<crate::text::encoding_cmap::EncodingCMap> {
        use crate::text::encoding_cmap::{resolve_predefined, CidEncoding, EncodingCMap};
        Self::cmap_depth(depth)?;
        let mut child =
            EncodingCMap::parse(&document.decode_stream_with_limit(stream, 8 * 1024 * 1024)?)?;
        if let Some(info) = stream.dict.get("CIDSystemInfo") {
            let info = document.resolve(info)?;
            let collection = if let Some(info) = info.as_dict() {
                let string = |name: &str| -> ParseResult<Option<String>> {
                    info.get(name)
                        .map(|value| {
                            document
                                .resolve(value)
                                .map(|value| value.as_string().map(|s| s.to_text()))
                        })
                        .transpose()
                        .map(Option::flatten)
                };
                let supplement = info
                    .get("Supplement")
                    .map(|value| document.resolve(value))
                    .transpose()?;
                if supplement
                    .as_ref()
                    .and_then(PdfObject::as_integer)
                    .is_some_and(|n| n >= 0)
                {
                    string("Registry")?.zip(string("Ordering")?)
                } else {
                    None
                }
            } else {
                None
            };
            child.declare_collection(collection);
        }
        if let Some(mode) = stream.dict.get("WMode").and_then(PdfObject::as_integer) {
            child.wmode = mode.clamp(0, 1) as u8;
            child.explicit_wmode = true;
        }
        let parent = stream.dict.get("UseCMap").cloned().or_else(|| {
            child
                .usecmap_parent
                .as_ref()
                .map(|name| PdfObject::Name(PdfName(name.clone())))
        });
        if let Some(parent) = parent {
            let result = (|| match document.resolve(&parent)? {
                PdfObject::Stream(stream) => {
                    Self::parse_encoding_stream(&stream, document, depth + 1)
                }
                PdfObject::Name(name) => match name.0.as_str() {
                    "Identity-H" => Ok(EncodingCMap::identity(false)),
                    "Identity-V" => Ok(EncodingCMap::identity(true)),
                    name => match resolve_predefined(name) {
                        Some(CidEncoding::Cmap(map)) => Ok(map),
                        _ => Err(Self::cmap_error(format!(
                            "Unsupported Encoding UseCMap parent {name}"
                        ))),
                    },
                },
                _ => Err(Self::cmap_error(
                    "UseCMap parent must be a name or stream".into(),
                )),
            })();
            match result {
                Ok(parent) => child.inherit(parent),
                Err(error) if document.options().strict_mode => return Err(error),
                // Recovery keeps the child's explicit mappings, never invents
                // identity semantics for an unresolved or cyclic parent.
                Err(_) => {}
            }
        }
        Ok(child)
    }

    /// Resolve a bounded ToUnicode resource chain; cycles never lose authority.
    pub(crate) fn resolve_tounicode(
        object: &PdfObject,
        document: &PdfDocument<R>,
    ) -> ParseResult<Option<CMap>> {
        let mut object = object.clone();
        let mut seen = std::collections::HashSet::new();
        while let PdfObject::Reference(number, generation) = object {
            if seen.len() >= 16 || !seen.insert((number, generation)) {
                return Err(Self::cmap_error(
                    "ToUnicode reference cycle or depth limit".into(),
                ));
            }
            object = match document.get_object(number, generation) {
                Err(ParseError::InvalidReference(_, _)) => PdfObject::Null,
                other => other?,
            };
        }
        if matches!(object, PdfObject::Null) {
            return Ok(None);
        }
        let stream = object
            .as_stream()
            .ok_or_else(|| Self::cmap_error("ToUnicode must be a stream".into()))?;
        Self::parse_unicode_chain(stream, document, 0).map(Some)
    }

    pub(crate) fn parse_unicode_chain(
        stream: &PdfStream,
        document: &PdfDocument<R>,
        depth: usize,
    ) -> ParseResult<CMap> {
        Self::cmap_depth(depth)?;
        let mut child = CMap::parse(&document.decode_stream_with_limit(stream, 8 * 1024 * 1024)?)?;
        // Named operator parents already have the established CMap fallback.
        // A dictionary UseCMap takes precedence over that operator.
        if let Some(parent) = stream.dict.get("UseCMap") {
            child.inherited_predefined = None;
            let result = (|| match document.resolve(parent)? {
                PdfObject::Stream(stream) => {
                    Self::parse_unicode_chain(&stream, document, depth + 1)
                }
                PdfObject::Name(name) => match name.0.as_str() {
                    "Identity-H" => Ok(CMap::identity_h()),
                    "Identity-V" => Ok(CMap::identity_v()),
                    _ => {
                        // A decoded PDF name is data, not PostScript source.
                        let mut parent = CMap::new();
                        parent.inherited_predefined = Some(name.0);
                        Ok(parent)
                    }
                },
                _ => Err(Self::cmap_error(
                    "UseCMap parent must be a name or stream".into(),
                )),
            })();
            match result {
                Ok(parent) => child.inherit(parent),
                Err(error) if document.options().strict_mode => return Err(error),
                Err(_) => {}
            }
        }
        Ok(child)
    }

    /// Extract font metrics (widths, kerning) from font dictionary
    pub(crate) fn extract_font_metrics(
        &self,
        font_dict: &PdfDictionary,
        document: &PdfDocument<R>,
    ) -> ParseResult<FontMetrics> {
        let normalized = without_null_metrics(font_dict, document);
        let font_dict = normalized.as_ref();
        let mut metrics = FontMetrics::default();

        let bounds_valid = match simple_font_bounds(document, font_dict) {
            Ok((first, last)) => {
                metrics.first_char = first;
                metrics.last_char = last;
                true
            }
            Err(error) if document.options().strict_mode => return Err(error),
            Err(_) => false,
        };

        // Extract FontMatrix scaling factor (relevant for Type 3 fonts).
        // Standard simple fonts have an implicit FontMatrix [0.001 0 0 0.001 0 0].
        // When FontMatrix is present, glyph-space widths are transformed to text space
        // by FontMatrix[0]. We scale by FontMatrix[0] * 1000.0 to normalize to standard
        // milli-em units (1/1000 of text space).
        let font_matrix = font_dict
            .get("FontMatrix")
            .and_then(|obj| document.resolve(obj).ok())
            .and_then(|obj| {
                if let PdfObject::Array(arr) = obj {
                    let mut matrix = Vec::new();
                    for elem in &arr.0 {
                        if let Ok(elem_resolved) = document.resolve(elem) {
                            if let Some(val) = elem_resolved.as_real() {
                                matrix.push(val);
                            }
                        }
                    }
                    if matrix.len() == 6 {
                        Some(matrix)
                    } else {
                        None
                    }
                } else {
                    None
                }
            });

        let width_scale = font_matrix.as_ref().map_or(1.0, |m| m[0] * 1000.0);

        // Extract Widths array
        metrics.widths = font_dict
            .get("Widths")
            .map(|obj| {
                let obj = document.resolve(obj)?;
                let array = obj
                    .as_array()
                    .ok_or_else(|| Self::cmap_error("Widths must be an array".into()))?;
                array
                    .0
                    .iter()
                    .map(|width| {
                        let value = document
                            .resolve(width)?
                            .as_real()
                            .map(|value| value * width_scale)
                            .filter(|value| value.is_finite())
                            .ok_or_else(|| {
                                Self::cmap_error("Widths must contain finite numbers".into())
                            })?;
                        Ok(value)
                    })
                    .collect::<ParseResult<Vec<_>>>()
            })
            .transpose()
            .or_else(|error| {
                if document.options().strict_mode {
                    Err(error)
                } else {
                    Ok(None)
                }
            })?;

        if !bounds_valid {
            metrics.widths = None;
        }

        // Extract MissingWidth from font descriptor
        metrics.missing_width = font_dict
            .get("FontDescriptor")
            .and_then(|o| document.resolve(o).ok())
            .and_then(|o| match o {
                PdfObject::Dictionary(d) => without_null_metrics(&d, document)
                    .get("MissingWidth")
                    .cloned(),
                _ => None,
            })
            .map(|o| {
                document
                    .resolve(&o)?
                    .as_real()
                    .filter(|value| value.is_finite())
                    .ok_or_else(|| Self::cmap_error("MissingWidth must be a finite number".into()))
            })
            .transpose()
            .or_else(|error| {
                if document.options().strict_mode {
                    Err(error)
                } else {
                    Ok(None)
                }
            })?;

        // Extract CIDFont `/W` + `/DW` (ISO 32000-1 §9.7.4.3). Only present
        // on a CIDFontType0/CIDFontType2 descendant font dictionary; a
        // simple font's dict has no `/W` key, so this is a no-op there.
        let is_cid_font = font_dict
            .get("Subtype")
            .and_then(|o| document.resolve(o).ok())
            .is_some_and(|o| match o {
                PdfObject::Name(n) => matches!(n.0.as_str(), "CIDFontType0" | "CIDFontType2"),
                _ => false,
            });
        let dw = font_dict
            .get("DW")
            .map(|o| {
                document
                    .resolve(o)?
                    .as_real()
                    .filter(|value| value.is_finite())
                    .ok_or_else(|| Self::cmap_error("DW must be a finite number".into()))
            })
            .transpose()
            .or_else(|error| {
                if document.options().strict_mode {
                    Err(error)
                } else {
                    Ok(None)
                }
            })?;
        if is_cid_font {
            metrics.cid_widths = Some(match parse_cid_widths(document, font_dict) {
                Ok(widths) => widths,
                Err(error) if document.options().strict_mode => return Err(error),
                // A malformed W table has no certified ranges. Keep valid DW
                // and the font's Unicode metadata for bounded lenient recovery.
                Err(_) => CidWidths {
                    widths: HashMap::new(),
                    ranges: Vec::new(),
                    default_width: dw.unwrap_or(1000.0),
                    vertical: CidVerticalWidths::default(),
                },
            });
        }

        if let Some(widths) = metrics.cid_widths.as_mut().filter(|_| is_cid_font) {
            let number = |object: &PdfObject| {
                document
                    .resolve(object)
                    .ok()
                    .and_then(|object| object.as_real())
                    .filter(|value| value.is_finite())
            };
            if let Some(PdfObject::Array(values)) = font_dict
                .get("DW2")
                .and_then(|object| document.resolve(object).ok())
            {
                if let Some(advance) = values.0.get(1).and_then(number) {
                    widths.vertical.default_advance = advance;
                }
            }
            if let Some(PdfObject::Array(values)) = font_dict
                .get("W2")
                .and_then(|object| document.resolve(object).ok())
            {
                let entries = values.0;
                let mut index = 0;
                while index + 1 < entries.len() {
                    let first = document
                        .resolve(&entries[index])
                        .ok()
                        .and_then(|object| object.as_integer())
                        .and_then(|value| u16::try_from(value).ok())
                        .map(u32::from);
                    let Some(first) = first else {
                        break;
                    };
                    let next = document.resolve(&entries[index + 1]).ok();
                    if let Some(PdfObject::Array(values)) = next {
                        for (offset, triple) in values.0.chunks_exact(3).enumerate() {
                            let cid = u32::try_from(offset)
                                .ok()
                                .and_then(|offset| first.checked_add(offset))
                                .filter(|cid| *cid <= u32::from(u16::MAX));
                            if let (Some(cid), Some(advance), Some(_), Some(_)) = (
                                cid,
                                number(&triple[0]),
                                number(&triple[1]),
                                number(&triple[2]),
                            ) {
                                widths.vertical.advances.insert(cid, advance);
                            }
                        }
                        index += 2;
                    } else {
                        let last = next
                            .and_then(|object| object.as_integer())
                            .and_then(|value| u16::try_from(value).ok())
                            .map(u32::from);
                        let triple = entries.get(index + 2..index + 5);
                        if let (Some(last), Some(triple)) = (last, triple) {
                            if let (Some(advance), Some(_), Some(_)) =
                                (number(&triple[0]), number(&triple[1]), number(&triple[2]))
                            {
                                if last >= first {
                                    widths.vertical.ranges.push((first, last, advance));
                                }
                            }
                        }
                        index += 5;
                    }
                }
            }
        }

        // Extract kerning from TrueType fonts (if embedded)
        if let Some(desc_obj) = font_dict.get("FontDescriptor") {
            if let Ok(PdfObject::Dictionary(desc_dict)) = document.resolve(desc_obj) {
                // Look for embedded TrueType font (FontFile2)
                if let Some(font_file_obj) = desc_dict.get("FontFile2") {
                    if let Ok(PdfObject::Stream(font_stream)) = document.resolve(font_file_obj) {
                        // Try to extract kerning from TrueType font
                        if let Ok(kerning_pairs) = extract_truetype_kerning(&font_stream) {
                            if !kerning_pairs.is_empty() {
                                metrics.kerning = Some(kerning_pairs);
                            }
                        }
                    }
                }
            }
        }

        Ok(metrics)
    }
}

/// Extract kerning pairs from TrueType font stream (kern table)
///
/// # Kerning Support
///
/// **Implemented:**
/// - TrueType fonts (FontFile2): Extracts kerning from embedded `kern` table
///
/// **NOT Implemented (by design):**
/// - Type1 fonts (FontFile): Type1 PFB (PostScript Font Binary) files embedded in PDFs
///   only contain glyph outlines, NOT font metrics. Kerning data for Type1 fonts is stored
///   separately in .afm (Adobe Font Metrics) or .pfm (PostScript Font Metrics) files,
///   which are NOT embedded in PDF documents.
///
/// For Type1 fonts requiring kerning, PDFs use TJ array position adjustments in content
/// streams (already handled by text extraction). There is no kerning data to extract
/// from embedded Type1 font programs.
///
/// If a real-world edge case emerges where Type1 fonts DO embed kerning data, this can
/// be revisited. Current implementation handles 99.9% of PDFs correctly.
pub(crate) fn extract_truetype_kerning(
    font_stream: &PdfStream,
) -> ParseResult<HashMap<(u32, u32), f64>> {
    // Decode the font stream
    let font_data = match font_stream.decode(&ParseOptions::default()) {
        Ok(data) => data,
        Err(_) => return Ok(HashMap::new()), // Silently fail if can't decode
    };

    // Parse TrueType font tables
    match parse_truetype_kern_table(&font_data) {
        Ok(pairs) => Ok(pairs),
        Err(_) => Ok(HashMap::new()), // Silently fail if parsing fails
    }
}

/// Parse TrueType kern table (Format 0 only)
pub(crate) fn parse_truetype_kern_table(font_data: &[u8]) -> ParseResult<HashMap<(u32, u32), f64>> {
    // TrueType fonts start with a table directory
    if font_data.len() < 12 {
        return Err(ParseError::SyntaxError {
            position: 0,
            message: "Font data too short for TrueType header".to_string(),
        });
    }

    // Read table directory offset (offset 12 + 16 * numTables)
    let num_tables = u16::from_be_bytes([font_data[4], font_data[5]]) as usize;

    // Find 'kern' table in table directory
    let mut kern_offset = None;
    let mut kern_length = None;

    for i in 0..num_tables {
        let table_offset = 12 + i * 16;
        if table_offset + 16 > font_data.len() {
            break;
        }

        // Read table tag (4 bytes)
        let tag = &font_data[table_offset..table_offset + 4];

        if tag == b"kern" {
            // Read table offset and length
            kern_offset = Some(u32::from_be_bytes([
                font_data[table_offset + 8],
                font_data[table_offset + 9],
                font_data[table_offset + 10],
                font_data[table_offset + 11],
            ]) as usize);

            kern_length = Some(u32::from_be_bytes([
                font_data[table_offset + 12],
                font_data[table_offset + 13],
                font_data[table_offset + 14],
                font_data[table_offset + 15],
            ]) as usize);

            break;
        }
    }

    // If no kern table found, return empty map
    let (offset, length) = match (kern_offset, kern_length) {
        (Some(o), Some(l)) => (o, l),
        _ => return Ok(HashMap::new()),
    };

    if offset + length > font_data.len() {
        return Err(ParseError::SyntaxError {
            position: offset,
            message: "Invalid kern table offset".to_string(),
        });
    }

    // Parse kern table header
    let kern_data = &font_data[offset..offset + length];
    if kern_data.len() < 4 {
        return Ok(HashMap::new());
    }

    // Version and nTables
    // nTables is a u16 at bytes 2-3
    let n_tables = u16::from_be_bytes([kern_data[2], kern_data[3]]) as usize;

    let mut kerning_pairs = HashMap::new();
    let mut table_offset = 4; // After header

    // Parse each subtable (we only support Format 0)
    for _ in 0..n_tables {
        if table_offset + 6 > kern_data.len() {
            break;
        }

        // Subtable header
        let subtable_length = u32::from_be_bytes([
            0,
            0,
            kern_data[table_offset + 2],
            kern_data[table_offset + 3],
        ]) as usize;

        let coverage =
            u16::from_be_bytes([kern_data[table_offset + 4], kern_data[table_offset + 5]]);

        // Format is in the lower byte per TrueType spec (ISO 14496-22:2019)
        let format = coverage & 0xFF;

        // Only process Format 0 (ordered pair list)
        if format == 0 && table_offset + subtable_length <= kern_data.len() {
            let subtable_data = &kern_data[table_offset + 6..table_offset + subtable_length];

            if subtable_data.len() >= 8 {
                let n_pairs = u16::from_be_bytes([subtable_data[0], subtable_data[1]]) as usize;

                // Skip searchRange, entrySelector, rangeShift (6 bytes)
                let mut pair_offset = 8;

                for _ in 0..n_pairs {
                    if pair_offset + 6 > subtable_data.len() {
                        break;
                    }

                    let left_glyph = u16::from_be_bytes([
                        subtable_data[pair_offset],
                        subtable_data[pair_offset + 1],
                    ]) as u32;

                    let right_glyph = u16::from_be_bytes([
                        subtable_data[pair_offset + 2],
                        subtable_data[pair_offset + 3],
                    ]) as u32;

                    let value = i16::from_be_bytes([
                        subtable_data[pair_offset + 4],
                        subtable_data[pair_offset + 5],
                    ]) as f64;

                    // Store kerning pair (value is in FUnits, typically 1/1000)
                    kerning_pairs.insert((left_glyph, right_glyph), value);

                    pair_offset += 6;
                }
            }
        }

        table_offset += subtable_length;
    }

    Ok(kerning_pairs)
}

/// The whitespace `sanitize_extracted_text` keeps downstream. `decode_is_usable`
/// uses the same set on purpose: accepting a decode that the next stage then
/// deletes would turn "usable" into an empty string — worse than the guessed
/// fallback it was accepted over.
fn is_preservable_whitespace(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\n')
}

/// Whether a decode produced usable text, or nothing the caller can act on.
///
/// A decode is unusable only when it yielded no characters at all, or nothing
/// but control codes that carry no text (NUL included) — the signature of
/// reading bytes through the wrong table. Whitespace is real text: a subsetted
/// font's first glyph is routinely the space, mapped by `/ToUnicode` to U+0020,
/// and content streams routinely show it in a text-showing operator of its own.
///
/// Treating such a decode as a failure sends the caller to a *guessed* encoding,
/// and in a subset font with no `/Encoding` the codes mean nothing outside their
/// CMap — so the guess renders the code as its own literal ASCII, replacing every
/// space with `!` or `"` and shredding word boundaries document-wide (#438).
pub(crate) fn decode_is_usable(decoded: &str) -> bool {
    !decoded.is_empty()
        && !decoded
            .chars()
            .all(|c| c.is_control() && !is_preservable_whitespace(c))
}

/// How a page's `/Font` subdictionary names one font.
///
/// Both forms are legal: any dictionary value may be written directly
/// (ISO 32000-1 §7.3.7), and §9.5 imposes no reference requirement on the
/// entries of the font subdictionary. Real producers use both — our own writer
/// emits them inline.
pub(crate) enum FontEntry {
    /// `/F1 12 0 R`. The font dictionary is its own object, so a caller may
    /// cache the parsed font across pages keyed by that object id.
    Indirect(u32, u16),
    /// `/F1 << /Type /Font ... >>`. Written directly into the resources; there
    /// is no object id to key a cross-page cache on, so it is parsed per page.
    Inline(PdfDictionary),
}

/// Resolve a page's `/Font` resource subdictionary into its entries.
///
/// Resolves the `/Font` level (itself either a dictionary or a reference) and
/// classifies each entry, without resolving the entries themselves — a caller
/// holding a font cache keyed by object id can then skip the fetch entirely on
/// a hit.
///
/// Every text extractor used to read only the entries that were indirect
/// references, so a page with inline fonts decoded against an empty cache: no
/// `/ToUnicode`, no `/Encoding`, and the byte-wise fallback turned glyph codes
/// into whatever those bytes mean in WinAnsi.
///
/// Dereferences a single level, the convention throughout the parser: a `/Font`
/// (or an entry) written as a reference *to another reference* dereferences to a
/// `Reference`, falls through, and drops the font. That shape does not occur in
/// the measured corpus and no producer is known to emit it; a double-indirect
/// font resource would need its own handling.
pub(crate) fn resolve_font_entries<R: Read + Seek>(
    resources: &PdfDictionary,
    document: &PdfDocument<R>,
) -> Vec<(String, FontEntry)> {
    let font_dict = match resources.get("Font") {
        Some(PdfObject::Dictionary(dict)) => Some(dict.clone()),
        Some(PdfObject::Reference(num, gen)) => match document.get_object(*num, *gen) {
            Ok(PdfObject::Dictionary(dict)) => Some(dict),
            _ => None,
        },
        _ => None,
    };

    let Some(font_dict) = font_dict else {
        return Vec::new();
    };

    font_dict
        .0
        .iter()
        .filter_map(|(name, obj)| match obj {
            PdfObject::Reference(num, gen) => {
                Some((name.0.clone(), FontEntry::Indirect(*num, *gen)))
            }
            PdfObject::Dictionary(dict) => Some((name.0.clone(), FontEntry::Inline(dict.clone()))),
            _ => None,
        })
        .collect()
}

/// Whether a known Adobe collection intentionally decodes all source codes as .notdef.
pub(crate) fn collection_notdef_only(text_bytes: &[u8], font_info: &FontInfo) -> bool {
    if font_info.font_type != "Type0" || font_info.to_unicode.is_some() {
        return false;
    }
    let Some(descendant) = &font_info.descendant_font else {
        return false;
    };
    if descendant.to_unicode.is_some()
        || descendant
            .cid_ordering
            .as_deref()
            .or(font_info.cid_ordering.as_deref())
            .and_then(AdobeCidCollection::from_ordering)
            .is_none()
    {
        return false;
    }
    match &font_info.cid_encoding {
        None => text_bytes.len() % 2 == 0 && text_bytes.iter().all(|byte| *byte == 0),
        Some(crate::text::encoding_cmap::CidEncoding::Utf16Be) => false,
        Some(crate::text::encoding_cmap::CidEncoding::Cmap(cmap)) => {
            let mut position = 0;
            while position < text_bytes.len() {
                let len = cmap.code_len_at(text_bytes, position);
                let Some(code) = text_bytes.get(position..position + len) else {
                    return false;
                };
                if cmap.map_code_to_cid(code).or_else(|| cmap.map_notdef(code)) != Some(0) {
                    return false;
                }
                position += len;
            }
            true
        }
    }
}

/// Decode text using font information — free function (no allocations).
///
/// Tries ToUnicode CMap first, then CID→Unicode tables for CJK fonts,
/// then descendant fonts (for Type0), then falls back to encoding-based decoding.
pub fn decode_text_with_font(text_bytes: &[u8], font_info: &FontInfo) -> ParseResult<String> {
    // First try ToUnicode CMap if available
    if let Some(ref to_unicode) = font_info.to_unicode {
        let simple = matches!(
            font_info.font_type.as_str(),
            "Type1" | "MMType1" | "TrueType" | "Type3"
        );
        return decode_with_cmap(text_bytes, to_unicode, simple);
    }

    // For Type0 fonts, try CID→Unicode tables before falling back
    if font_info.font_type == "Type0" {
        if let Some(ref descendant) = font_info.descendant_font {
            // Try descendant's ToUnicode first
            if descendant.to_unicode.is_some() {
                return decode_text_with_font(text_bytes, descendant);
            }

            // Non-Identity encoding: map code→CID (or UTF-16BE) before CID→Unicode.
            let ordering = descendant
                .cid_ordering
                .as_deref()
                .or(font_info.cid_ordering.as_deref());

            let ordering = compatible_cid_ordering(ordering, font_info.cid_encoding.as_ref());

            match &font_info.cid_encoding {
                Some(crate::text::encoding_cmap::CidEncoding::Utf16Be) => {
                    return Ok(crate::text::encoding_cmap::decode_utf16be(text_bytes));
                }
                Some(crate::text::encoding_cmap::CidEncoding::Cmap(enc)) => {
                    if let Some(coll) = ordering
                        .and_then(crate::text::cid_to_unicode::AdobeCidCollection::from_ordering)
                    {
                        return Ok(decode_via_encoding_cmap(text_bytes, enc, &coll));
                    }
                    // Non-Identity encoding but the CID collection is unknown
                    // (malformed PDF without CIDSystemInfo/Ordering). Fall through
                    // to the Identity CID-table path below as best-effort.
                }
                None => {}
            }

            // Try CID→Unicode mapping using CIDSystemInfo Ordering
            // This handles fonts with Identity-H encoding and no ToUnicode CMap
            if let Some(ordering) = ordering {
                if let Some(collection) =
                    crate::text::cid_to_unicode::AdobeCidCollection::from_ordering(ordering)
                {
                    let result = decode_with_cid_table(text_bytes, &collection);
                    // Same predicate as the ToUnicode path — one definition so
                    // the two acceptance rules cannot drift apart (#438).
                    // An all-.notdef string has a valid empty collection result.
                    // Do not reinterpret its zero bytes through a simple encoding.
                    if decode_is_usable(&result) || collection_notdef_only(text_bytes, font_info) {
                        return Ok(result);
                    }
                }
            }

            if matches!(
                descendant.font_type.as_str(),
                "CIDFontType0" | "CIDFontType2"
            ) {
                // Private CID charsets and CIDToGIDMap select glyphs, never
                // Unicode. Preserve one unknown marker per source code rather
                // than feeding multibyte CIDs to a simple byte encoding.
                let mut result = String::new();
                let mut position = 0;
                while position < text_bytes.len() {
                    let length = match &font_info.cid_encoding {
                        Some(crate::text::encoding_cmap::CidEncoding::Cmap(cmap)) => {
                            cmap.code_len_at(text_bytes, position).max(1)
                        }
                        _ => 2,
                    };
                    result.push('\u{fffd}');
                    position += length.min(text_bytes.len() - position);
                }
                return Ok(result);
            }
            // Fall through to descendant's encoding-based decoding
            return decode_text_with_font(text_bytes, descendant);
        }
    }

    // Fall back to encoding-based decoding
    decode_with_encoding(text_bytes, font_info)
}

/// Decode using an embedded/predefined encoding CMap (code→CID) followed by a
/// CID→Unicode collection. Walks variable-width codes per the CMap codespace.
fn decode_via_encoding_cmap(
    text_bytes: &[u8],
    enc: &crate::text::encoding_cmap::EncodingCMap,
    collection: &crate::text::cid_to_unicode::AdobeCidCollection,
) -> String {
    let mut result = String::new();
    let mut i = 0;
    while i < text_bytes.len() {
        let len = enc.code_len_at(text_bytes, i).max(1);
        if len > text_bytes.len() - i {
            // A partial source code is not a shorter code, even if a mapping
            // for its prefix exists in a damaged font.
            result.push('\u{FFFD}');
            break;
        }
        let code = &text_bytes[i..i + len];
        match enc.map_code_to_cid(code).or_else(|| enc.map_notdef(code)) {
            Some(cid) => match collection.cid_to_unicode_sequence(cid) {
                Some(ch) => result.push_str(ch),
                None if cid > 0 => result.push('\u{FFFD}'),
                None => {}
            },
            None => result.push('\u{FFFD}'),
        }
        i += len;
    }
    result
}

/// Decode text using CID→Unicode lookup tables (Adobe CMap Resources).
///
/// Interprets text_bytes as pairs of big-endian u16 CIDs and maps each
/// to its Unicode code point using the specified CID collection.
fn decode_with_cid_table(
    text_bytes: &[u8],
    collection: &crate::text::cid_to_unicode::AdobeCidCollection,
) -> String {
    let mut result = String::new();
    let mut i = 0;

    while i + 1 < text_bytes.len() {
        let cid = u16::from_be_bytes([text_bytes[i], text_bytes[i + 1]]);
        if let Some(ch) = collection.cid_to_unicode_sequence(cid) {
            result.push_str(ch);
        } else if cid > 0 {
            // Unknown CID — emit replacement character rather than losing position
            result.push('\u{FFFD}');
        }
        i += 2;
    }
    if i < text_bytes.len() {
        result.push('\u{FFFD}');
    }

    result
}

/// Decode text using a CMap — free function (no allocations).
fn decode_with_cmap(text_bytes: &[u8], cmap: &CMap, simple_font: bool) -> ParseResult<String> {
    let inherited = cmap
        .inherited_ordering()
        .and_then(AdobeCidCollection::from_ordering);

    let mut result = String::new();
    let mut i = 0;

    while i < text_bytes.len() {
        let mut decoded = false;

        for len in 1..=4.min(text_bytes.len() - i) {
            let code = &text_bytes[i..i + len];
            if let Some(mapped) = cmap.map(code) {
                if let Some(unicode_str) = cmap.to_unicode(&mapped) {
                    result.push_str(&unicode_str);
                    i += len;
                    decoded = true;
                    break;
                }
            }
        }

        if !decoded {
            // External usecmap to a predefined Adobe `*-UCS2` parent: treat an
            // unmapped 2-byte code as a CID and resolve via the inherited
            // collection. Explicit child bf* mappings already won above. Advance
            // a full 2 bytes regardless of lookup success to keep the 2-byte
            // stride (matching decode_with_cid_table); emit U+FFFD for an
            // unmapped non-zero CID, nothing for CID 0.
            if let Some(coll) = inherited {
                if text_bytes.len() - i >= 2 {
                    let cid = u16::from_be_bytes([text_bytes[i], text_bytes[i + 1]]);
                    match coll.cid_to_unicode_sequence(cid) {
                        Some(ch) => result.push_str(ch),
                        None if cid > 0 => result.push('\u{FFFD}'),
                        None => {}
                    }
                    i += 2;
                    continue;
                }
            }
            // Keep failed decoding visible and consume the whole source code.
            // Retrying suffix bytes can manufacture valid-looking Unicode from
            // an unmapped multibyte code. A partial tail is one failed code.
            // Successful explicit mappings above still tolerate the common
            // simple-font defect of one-byte entries in a two-byte codespace.
            let remaining = &text_bytes[i..];
            let len = if simple_font {
                // Simple-font source codes are bytes, even when a producer's
                // ToUnicode incorrectly declares a two-byte codespace.
                1
            } else {
                cmap.codespace_ranges
                    .iter()
                    .find_map(|range| {
                        let len = range.start.len();
                        if len == 0 || len > 4 || range.end.len() != len {
                            return None;
                        }
                        let prefix_len = len.min(remaining.len());
                        let prefix = &remaining[..prefix_len];
                        (prefix >= &range.start[..prefix_len] && prefix <= &range.end[..prefix_len])
                            .then_some(len)
                    })
                    .unwrap_or(if inherited.is_some() { 2 } else { 1 })
            };
            result.push('\u{FFFD}');
            i += len.min(remaining.len());
        }
    }

    Ok(result)
}

fn without_subset_prefix(name: &str) -> &str {
    name.split_once('+')
        .filter(|(prefix, _)| prefix.len() == 6 && prefix.bytes().all(|b| b.is_ascii_uppercase()))
        .map_or(name, |(_, name)| name)
}

/// Decode text using encoding differences and base encoding — free function.
fn decode_with_encoding(text_bytes: &[u8], font_info: &FontInfo) -> ParseResult<String> {
    let mut result = String::new();

    for &byte in text_bytes {
        if let Some(ref differences) = font_info.differences {
            if let Some(char_name) = differences.get(&byte) {
                if let Some(unicode) =
                    glyph_name_to_unicode_sequence_in_font(char_name, Some(&font_info.name))
                {
                    result.push_str(&unicode);
                    continue;
                }
                // Differences replaces the base glyph even when its name
                // has no Unicode assignment. A Type3 CharProc is drawing
                // evidence, not permission to resurrect the base character.
                result.push('\u{FFFD}');
                continue;
            }
        }

        let ch = match font_info.encoding.as_deref() {
            Some("WinAnsiEncoding") => decode_winansi(byte),
            Some("MacRomanEncoding") => decode_macroman(byte),
            Some("StandardEncoding") => decode_standard(byte),
            Some("MacExpertEncoding") => pdf_simple_encodings::MAC_EXPERT[usize::from(byte)],
            Some("SymbolEncoding") => pdf_simple_encodings::SYMBOL[usize::from(byte)],
            Some("ZapfDingbatsEncoding") => pdf_simple_encodings::ZAPF_DINGBATS[usize::from(byte)],
            _ => byte as char,
        };

        result.push(ch);
    }

    Ok(result)
}

/// Convert a glyph name to one Unicode character.
///
/// For glyph names that resolve to more than one Unicode scalar, use
/// [`glyph_name_to_unicode_sequence`].
pub fn glyph_name_to_unicode(name: &str) -> Option<char> {
    let value = glyph_name_to_unicode_sequence(name)?;
    let mut chars = value.chars();
    match (chars.next(), chars.next()) {
        (Some(character), None) => Some(character),
        _ => None,
    }
}

/// Convert a glyph name to its complete Unicode sequence.
///
/// The mapping uses Adobe's complete Glyph List. It also implements the
/// `uniXXXX...` and `uXXXX` glyph-name conventions, including `uni` names
/// containing multiple four-digit scalar values.
pub fn glyph_name_to_unicode_sequence(name: &str) -> Option<String> {
    glyph_name_to_unicode_sequence_in_font(name, None)
}

pub(crate) fn glyph_name_to_unicode_sequence_in_font(
    name: &str,
    font_name: Option<&str>,
) -> Option<String> {
    let zapf = font_name.is_some_and(|name| without_subset_prefix(name) == "ZapfDingbats");
    if name.chars().count() == 1 {
        return Some(name.to_owned());
    }

    let lookup_name = name.split_once('.').map_or(name, |(base, _)| base);
    if lookup_name.is_empty() {
        return None;
    }

    let result: String = lookup_name
        .split('_')
        .filter_map(|component| {
            if zapf {
                if let Some(value) = pdf_simple_encodings::zapf_glyph(component) {
                    return Some(value.to_string());
                }
            }
            glyph_component_to_unicode(component)
        })
        .collect();
    (!result.is_empty()).then_some(result)
}

fn glyph_component_to_unicode(lookup_name: &str) -> Option<String> {
    if let Some(scalars) = adobe_glyph_list().get(lookup_name) {
        return decode_unicode_scalars(scalars.split_whitespace());
    }
    if let Some(hex) = lookup_name.strip_prefix("uni") {
        if !hex.is_empty()
            && hex.len() % 4 == 0
            && hex
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'A'..=b'F').contains(&byte))
        {
            return decode_unicode_scalars(
                hex.as_bytes()
                    .chunks_exact(4)
                    .map(|chunk| std::str::from_utf8(chunk).expect("ASCII hexadecimal glyph name")),
            );
        }
        return None;
    }

    if let Some(hex) = lookup_name.strip_prefix('u') {
        if (4..=6).contains(&hex.len())
            && hex
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'A'..=b'F').contains(&byte))
        {
            return decode_unicode_scalars(std::iter::once(hex));
        }
        return None;
    }

    adobe_glyph_list()
        .get(lookup_name)
        .and_then(|scalars| decode_unicode_scalars(scalars.split_whitespace()))
        .or_else(|| {
            legacy_glyph_name_to_unicode(lookup_name).map(|character| character.to_string())
        })
}

fn decode_unicode_scalars<'a>(scalars: impl IntoIterator<Item = &'a str>) -> Option<String> {
    let mut result = String::new();
    for scalar in scalars {
        let value = u32::from_str_radix(scalar, 16).ok()?;
        result.push(char::from_u32(value)?);
    }
    (!result.is_empty()).then_some(result)
}

fn adobe_glyph_list() -> &'static HashMap<&'static str, &'static str> {
    static GLYPHS: OnceLock<HashMap<&'static str, &'static str>> = OnceLock::new();
    GLYPHS.get_or_init(|| {
        include_str!("adobe_glyph_list.txt")
            .lines()
            .filter_map(|line| line.split_once(';'))
            .collect()
    })
}

fn legacy_glyph_name_to_unicode(name: &str) -> Option<char> {
    // 1. Single character glyph name (e.g., 'A', 'a', '1', '+')
    let mut chars = name.chars();
    if let (Some(c), None) = (chars.next(), chars.next()) {
        return Some(c);
    }

    // 2. uniXXXX (4 hex digits)
    if name.len() == 7
        && name.starts_with("uni")
        && name[3..].chars().all(|c| c.is_ascii_hexdigit())
    {
        if let Ok(val) = u32::from_str_radix(&name[3..], 16) {
            if let Some(c) = char::from_u32(val) {
                return Some(c);
            }
        }
    }

    // 3. uXXXX / uXXXXX / uXXXXXX (4..6 hex digits)
    if (5..=7).contains(&name.len())
        && name.starts_with('u')
        && name[1..].chars().all(|c| c.is_ascii_hexdigit())
    {
        if let Ok(val) = u32::from_str_radix(&name[1..], 16) {
            if let Some(c) = char::from_u32(val) {
                return Some(c);
            }
        }
    }

    // Strip variant suffix after period (e.g., "A.swash" -> "A", "aacute.alt" -> "aacute")
    let lookup_name = if let Some((base, _)) = name.split_once('.') {
        if !base.is_empty() {
            if let Some(c) = glyph_name_to_unicode(base) {
                return Some(c);
            }
        }
        base
    } else {
        name
    };

    // 4. Adobe Glyph List (AGL) mapping
    match lookup_name {
        // ASCII / Basic Latin glyph names
        "space" => Some(' '),
        "exclam" => Some('!'),
        "quotedbl" => Some('"'),
        "numbersign" => Some('#'),
        "dollar" => Some('$'),
        "percent" => Some('%'),
        "ampersand" => Some('&'),
        "quotesingle" => Some('\''),
        "parenleft" => Some('('),
        "parenright" => Some(')'),
        "asterisk" => Some('*'),
        "plus" => Some('+'),
        "comma" => Some(','),
        "hyphen" => Some('-'),
        "period" => Some('.'),
        "slash" => Some('/'),
        "zero" => Some('0'),
        "one" => Some('1'),
        "two" => Some('2'),
        "three" => Some('3'),
        "four" => Some('4'),
        "five" => Some('5'),
        "six" => Some('6'),
        "seven" => Some('7'),
        "eight" => Some('8'),
        "nine" => Some('9'),
        "colon" => Some(':'),
        "semicolon" => Some(';'),
        "less" => Some('<'),
        "equal" => Some('='),
        "greater" => Some('>'),
        "question" => Some('?'),
        "at" => Some('@'),
        "bracketleft" => Some('['),
        "backslash" => Some('\\'),
        "bracketright" => Some(']'),
        "asciicircum" => Some('^'),
        "underscore" => Some('_'),
        "grave" => Some('`'),
        "braceleft" => Some('{'),
        "bar" => Some('|'),
        "braceright" => Some('}'),
        "asciitilde" => Some('~'),

        // Latin-1 Supplement & Common PostScript
        "exclamdown" => Some('¡'),
        "cent" => Some('¢'),
        "sterling" => Some('£'),
        "currency" => Some('¤'),
        "yen" => Some('¥'),
        "brokenbar" => Some('¦'),
        "section" => Some('§'),
        "dieresis" => Some('¨'),
        "copyright" => Some('©'),
        "ordfeminine" => Some('ª'),
        "guillemotleft" | "guillemetleft" => Some('«'),
        "logicalnot" => Some('¬'),
        "softhyphen" | "hyphensoft" => Some('\u{00AD}'),
        "registered" => Some('®'),
        "macron" => Some('¯'),
        "degree" => Some('°'),
        "plusminus" => Some('±'),
        "twosuperior" => Some('²'),
        "threesuperior" => Some('³'),
        "acute" => Some('´'),
        "mu" | "micro" => Some('µ'),
        "paragraph" => Some('¶'),
        "periodcentered" | "bulletcentered" => Some('·'),
        "cedilla" => Some('¸'),
        "onesuperior" => Some('¹'),
        "ordmasculine" => Some('º'),
        "guillemotright" | "guillemetright" => Some('»'),
        "onequarter" => Some('¼'),
        "onehalf" => Some('½'),
        "threequarters" => Some('¾'),
        "questiondown" => Some('¿'),

        // Accented uppercase Latin-1
        "Agrave" => Some('À'),
        "Aacute" => Some('Á'),
        "Acircumflex" => Some('Â'),
        "Atilde" => Some('Ã'),
        "Adieresis" => Some('Ä'),
        "Aring" => Some('Å'),
        "AE" => Some('Æ'),
        "Ccedilla" => Some('Ç'),
        "Egrave" => Some('È'),
        "Eacute" => Some('É'),
        "Ecircumflex" => Some('Ê'),
        "Edieresis" => Some('Ë'),
        "Igrave" => Some('Ì'),
        "Iacute" => Some('Í'),
        "Icircumflex" => Some('Î'),
        "Idieresis" => Some('Ï'),
        "Eth" => Some('Ð'),
        "Ntilde" => Some('Ñ'),
        "Ograve" => Some('Ò'),
        "Oacute" => Some('Ó'),
        "Ocircumflex" => Some('Ô'),
        "Otilde" => Some('Õ'),
        "Odieresis" => Some('Ö'),
        "multiply" => Some('×'),
        "Oslash" => Some('Ø'),
        "Ugrave" => Some('Ù'),
        "Uacute" => Some('Ú'),
        "Ucircumflex" => Some('Û'),
        "Udieresis" => Some('Ü'),
        "Yacute" => Some('Ý'),
        "Thorn" => Some('Þ'),
        "germandbls" => Some('ß'),

        // Accented lowercase Latin-1
        "agrave" => Some('à'),
        "aacute" => Some('á'),
        "acircumflex" => Some('â'),
        "atilde" => Some('ã'),
        "adieresis" => Some('ä'),
        "aring" => Some('å'),
        "ae" => Some('æ'),
        "ccedilla" => Some('ç'),
        "egrave" => Some('è'),
        "eacute" => Some('é'),
        "ecircumflex" => Some('ê'),
        "edieresis" => Some('ë'),
        "igrave" => Some('ì'),
        "iacute" => Some('í'),
        "icircumflex" => Some('î'),
        "idieresis" => Some('ï'),
        "eth" => Some('ð'),
        "ntilde" => Some('ñ'),
        "ograve" => Some('ò'),
        "oacute" => Some('ó'),
        "ocircumflex" => Some('ô'),
        "otilde" => Some('õ'),
        "odieresis" => Some('ö'),
        "divide" => Some('÷'),
        "oslash" => Some('ø'),
        "ugrave" => Some('ù'),
        "uacute" => Some('ú'),
        "ucircumflex" => Some('û'),
        "udieresis" => Some('ü'),
        "yacute" => Some('ý'),
        "thorn" => Some('þ'),
        "ydieresis" => Some('ÿ'),

        // Latin Extended-A & B
        "Amacron" => Some('Ā'),
        "amacron" => Some('ā'),
        "Abreve" => Some('Ă'),
        "abreve" => Some('ă'),
        "Aogonek" => Some('Ą'),
        "aogonek" => Some('ą'),
        "Cacute" => Some('Ć'),
        "cacute" => Some('ć'),
        "Ccircumflex" => Some('Ĉ'),
        "ccircumflex" => Some('ĉ'),
        "Cdotaccent" => Some('Ċ'),
        "cdotaccent" => Some('ċ'),
        "Ccaron" => Some('Č'),
        "ccaron" => Some('č'),
        "Dcaron" => Some('Ď'),
        "dcaron" => Some('ď'),
        "Dcroat" => Some('Đ'),
        "dcroat" => Some('đ'),
        "Emacron" => Some('Ē'),
        "emacron" => Some('ē'),
        "Ebreve" => Some('Ĕ'),
        "ebreve" => Some('ĕ'),
        "Edotaccent" => Some('Ė'),
        "edotaccent" => Some('ė'),
        "Eogonek" => Some('Ę'),
        "eogonek" => Some('ę'),
        "Ecaron" => Some('Ě'),
        "ecaron" => Some('ě'),
        "Gcircumflex" => Some('Ĝ'),
        "gcircumflex" => Some('ĝ'),
        "Gbreve" => Some('Ğ'),
        "gbreve" => Some('ğ'),
        "Gdotaccent" => Some('Ġ'),
        "gdotaccent" => Some('ġ'),
        "Gcommaaccent" => Some('Ģ'),
        "gcommaaccent" => Some('ģ'),
        "Hcircumflex" => Some('Ĥ'),
        "hcircumflex" => Some('ĥ'),
        "Hbar" => Some('Ħ'),
        "hbar" => Some('ħ'),
        "Itilde" => Some('Ĩ'),
        "itilde" => Some('ĩ'),
        "Imacron" => Some('Ī'),
        "imacron" => Some('ī'),
        "Ibreve" => Some('Ĭ'),
        "ibreve" => Some('ĭ'),
        "Iogonek" => Some('Į'),
        "iogonek" => Some('į'),
        "Idotaccent" | "Idot" => Some('İ'),
        "dotlessi" => Some('ı'),
        "IJ" => Some('Ĳ'),
        "ij" => Some('ĳ'),
        "Jcircumflex" => Some('Ĵ'),
        "jcircumflex" => Some('ĵ'),
        "Kcommaaccent" => Some('Ķ'),
        "kcommaaccent" => Some('ķ'),
        "kgreenlandic" => Some('ĸ'),
        "Lacute" => Some('Ĺ'),
        "lacute" => Some('ĺ'),
        "Lcommaaccent" => Some('Ļ'),
        "lcommaaccent" => Some('ļ'),
        "Lcaron" => Some('Ľ'),
        "lcaron" => Some('ľ'),
        "Ldot" => Some('Ŀ'),
        "ldot" => Some('ŀ'),
        "Lslash" => Some('Ł'),
        "lslash" => Some('ł'),
        "Nacute" => Some('Ń'),
        "nacute" => Some('ń'),
        "Ncommaaccent" => Some('Ņ'),
        "ncommaaccent" => Some('ņ'),
        "Ncaron" => Some('Ň'),
        "ncaron" => Some('ň'),
        "napostrophe" => Some('ŉ'),
        "Eng" => Some('Ŋ'),
        "eng" => Some('ŋ'),
        "Omacron" => Some('Ō'),
        "omacron" => Some('ō'),
        "Obreve" => Some('Ŏ'),
        "obreve" => Some('ŏ'),
        "Ohungarumlaut" => Some('Ő'),
        "ohungarumlaut" => Some('ő'),
        "OE" => Some('Œ'),
        "oe" => Some('œ'),
        "Racute" => Some('Ŕ'),
        "racute" => Some('ŕ'),
        "Rcommaaccent" => Some('Ŗ'),
        "rcommaaccent" => Some('ŗ'),
        "Rcaron" => Some('Ř'),
        "rcaron" => Some('ř'),
        "Sacute" => Some('Ś'),
        "sacute" => Some('ś'),
        "Scircumflex" => Some('Ŝ'),
        "scircumflex" => Some('ŝ'),
        "Scedilla" => Some('Ş'),
        "scedilla" => Some('ş'),
        "Scaron" => Some('Š'),
        "scaron" => Some('š'),
        "Scommaaccent" => Some('Ș'),
        "scommaaccent" => Some('ș'),
        "Tcommaaccent" => Some('Ț'),
        "tcommaaccent" => Some('ț'),
        "Tcedilla" => Some('Ţ'),
        "tcedilla" => Some('ţ'),
        "Tcaron" => Some('Ť'),
        "tcaron" => Some('ť'),
        "Tbar" => Some('Ŧ'),
        "tbar" => Some('ŧ'),
        "Utilde" => Some('Ũ'),
        "utilde" => Some('ũ'),
        "Umacron" => Some('Ū'),
        "umacron" => Some('ū'),
        "Ubreve" => Some('Ŭ'),
        "ubreve" => Some('ŭ'),
        "Uring" => Some('Ů'),
        "uring" => Some('ů'),
        "Uhungarumlaut" => Some('Ű'),
        "uhungarumlaut" => Some('ű'),
        "Uogonek" => Some('Ų'),
        "uogonek" => Some('ų'),
        "Wcircumflex" => Some('Ŵ'),
        "wcircumflex" => Some('ŵ'),
        "Ycircumflex" => Some('Ŷ'),
        "ycircumflex" => Some('ŷ'),
        "Ydieresis" => Some('Ÿ'),
        "Zacute" => Some('Ź'),
        "zacute" => Some('ź'),
        "Zdotaccent" => Some('Ż'),
        "zdotaccent" => Some('ż'),
        "Zcaron" => Some('Ž'),
        "zcaron" => Some('ž'),
        "longs" => Some('ſ'),
        "florin" => Some('ƒ'),
        "dotlessj" => Some('ȷ'),

        // Spacing Modifiers
        "circumflex" => Some('ˆ'),
        "caron" => Some('ˇ'),
        "breve" => Some('˘'),
        "dotaccent" => Some('˙'),
        "ring" => Some('˚'),
        "ogonek" => Some('˛'),
        "tilde" => Some('˜'),
        "hungarumlaut" => Some('˝'),

        // Punctuation & Typographic symbols
        "quoteleft" | "leftsinglequote" => Some('‘'),
        "quoteright" | "rightsinglequote" => Some('’'),
        "quotesinglbase" | "singlelow9quote" => Some('‚'),
        "quotedblleft" | "leftdoublequote" => Some('“'),
        "quotedblright" | "rightdoublequote" => Some('”'),
        "quotedblbase" | "doublelow9quote" => Some('„'),
        "dagger" => Some('†'),
        "daggerdbl" => Some('‡'),
        "bullet" => Some('•'),
        "ellipsis" => Some('…'),
        "perthousand" => Some('‰'),
        "guilsinglleft" | "singleleftguillemet" => Some('‹'),
        "guilsinglright" | "singlerightguillemet" => Some('›'),
        "fraction" => Some('⁄'),
        "endash" | "figuredash" => Some('–'),
        "emdash" => Some('—'),
        "Euro" | "euro" => Some('€'),
        "trademark" => Some('™'),

        // Math & Other symbols
        "minus" => Some('−'),
        "checkmark" => Some('✓'),
        "partialdiff" => Some('∂'),
        "summation" => Some('∑'),
        "radical" => Some('√'),
        "infinity" => Some('∞'),
        "integral" => Some('∫'),
        "approxequal" => Some('≈'),
        "notequal" => Some('≠'),
        "lessequal" => Some('≤'),
        "greaterequal" => Some('≥'),
        "lozenge" => Some('◊'),
        "apple" => Some('\u{F8FF}'),

        // Ligatures
        "ff" => Some('ﬀ'),
        "fi" => Some('ﬁ'),
        "fl" => Some('ﬂ'),
        "ffi" => Some('ﬃ'),
        "ffl" => Some('ﬄ'),
        "ft" => Some('ﬅ'),
        "st" => Some('ﬆ'),

        // Greek letters (Symbol font / AGL)
        "Alpha" => Some('Α'),
        "Beta" => Some('Β'),
        "Gamma" => Some('Γ'),
        "Delta" => Some('Δ'),
        "Epsilon" => Some('Ε'),
        "Zeta" => Some('Ζ'),
        "Eta" => Some('Η'),
        "Theta" => Some('Θ'),
        "Iota" => Some('Ι'),
        "Kappa" => Some('Κ'),
        "Lambda" => Some('Λ'),
        "Mu" => Some('Μ'),
        "Nu" => Some('Ν'),
        "Xi" => Some('Ξ'),
        "Omicron" => Some('Ο'),
        "Pi" => Some('Π'),
        "Rho" => Some('Ρ'),
        "Sigma" => Some('Σ'),
        "Tau" => Some('Τ'),
        "Upsilon" => Some('Υ'),
        "Phi" => Some('Φ'),
        "Chi" => Some('Χ'),
        "Psi" => Some('Ψ'),
        "Omega" => Some('Ω'),
        "alpha" => Some('α'),
        "beta" => Some('β'),
        "gamma" => Some('γ'),
        "delta" => Some('δ'),
        "epsilon" => Some('ε'),
        "zeta" => Some('ζ'),
        "eta" => Some('η'),
        "theta" => Some('θ'),
        "iota" => Some('ι'),
        "kappa" => Some('κ'),
        "lambda" => Some('λ'),
        "nu" => Some('ν'),
        "xi" => Some('ξ'),
        "omicron" => Some('ο'),
        "pi" => Some('π'),
        "rho" => Some('ρ'),
        "sigma" => Some('σ'),
        "sigma1" => Some('ς'),
        "tau" => Some('τ'),
        "upsilon" => Some('υ'),
        "phi" => Some('φ'),
        "chi" => Some('χ'),
        "psi" => Some('ψ'),
        "omega" => Some('ω'),

        _ => None,
    }
}

/// Decode PDF WinAnsiEncoding, distinct from the Windows-1252 OS codec.
fn decode_winansi(byte: u8) -> char {
    pdf_simple_encodings::WIN_ANSI[usize::from(byte)]
}

/// Decode PDF MacRomanEncoding (ISO 32000-1 Annex D.2, issue #662).
/// Unlike OS Mac Roman, 0xDB is currency and undefined codes recover as U+FFFD.
fn decode_macroman(byte: u8) -> char {
    match byte {
        0x20..=0x7E => byte as char,
        0x80 => 'Ä',
        0x81 => 'Å',
        0x82 => 'Ç',
        0x83 => 'É',
        0x84 => 'Ñ',
        0x85 => 'Ö',
        0x86 => 'Ü',
        0x87 => 'á',
        0x88 => 'à',
        0x89 => 'â',
        0x8A => 'ä',
        0x8B => 'ã',
        0x8C => 'å',
        0x8D => 'ç',
        0x8E => 'é',
        0x8F => 'è',
        0x90 => 'ê',
        0x91 => 'ë',
        0x92 => 'í',
        0x93 => 'ì',
        0x94 => 'î',
        0x95 => 'ï',
        0x96 => 'ñ',
        0x97 => 'ó',
        0x98 => 'ò',
        0x99 => 'ô',
        0x9A => 'ö',
        0x9B => 'õ',
        0x9C => 'ú',
        0x9D => 'ù',
        0x9E => 'û',
        0x9F => 'ü',
        0xA0 => '†',
        0xA1 => '°',
        0xA2 => '¢',
        0xA3 => '£',
        0xA4 => '§',
        0xA5 => '•',
        0xA6 => '¶',
        0xA7 => 'ß',
        0xA8 => '®',
        0xA9 => '©',
        0xAA => '™',
        0xAB => '´',
        0xAC => '¨',
        0xAE => 'Æ',
        0xAF => 'Ø',
        0xB1 => '±',
        0xB4 => '¥',
        0xB5 => 'µ',
        0xBB => 'ª',
        0xBC => 'º',
        0xBE => 'æ',
        0xBF => 'ø',
        0xC0 => '¿',
        0xC1 => '¡',
        0xC2 => '¬',
        0xC4 => 'ƒ',
        0xC7 => '«',
        0xC8 => '»',
        0xC9 => '…',
        0xCA => '\u{00A0}', // Non-breaking space
        0xCB => 'À',
        0xCC => 'Ã',
        0xCD => 'Õ',
        0xCE => 'Œ',
        0xCF => 'œ',
        0xD0 => '–',
        0xD1 => '—',
        0xD2 => '“',
        0xD3 => '”',
        0xD4 => '‘',
        0xD5 => '’',
        0xD6 => '÷',
        0xD8 => 'ÿ',
        0xD9 => 'Ÿ',
        0xDA => '⁄',
        0xDB => '¤',
        0xDC => '‹',
        0xDD => '›',
        0xDE => 'ﬁ',
        0xDF => 'ﬂ',
        0xE0 => '‡',
        0xE1 => '·',
        0xE2 => '‚',
        0xE3 => '„',
        0xE4 => '‰',
        0xE5 => 'Â',
        0xE6 => 'Ê',
        0xE7 => 'Á',
        0xE8 => 'Ë',
        0xE9 => 'È',
        0xEA => 'Í',
        0xEB => 'Î',
        0xEC => 'Ï',
        0xED => 'Ì',
        0xEE => 'Ó',
        0xEF => 'Ô',
        0xF1 => 'Ò',
        0xF2 => 'Ú',
        0xF3 => 'Û',
        0xF4 => 'Ù',
        0xF5 => 'ı',
        0xF6 => 'ˆ',
        0xF7 => '˜',
        0xF8 => '¯',
        0xF9 => '˘',
        0xFA => '˙',
        0xFB => '˚',
        0xFC => '¸',
        0xFD => '˝',
        0xFE => '˛',
        0xFF => 'ˇ',
        // PDF leaves these codes unassigned; preserve their position in recovery.
        _ => '\u{FFFD}',
    }
}

/// Decode StandardEncoding
fn decode_standard(byte: u8) -> char {
    pdf_simple_encodings::STANDARD[usize::from(byte)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_glyph_name_to_unicode() {
        // Single characters
        assert_eq!(glyph_name_to_unicode("A"), Some('A'));
        assert_eq!(glyph_name_to_unicode("a"), Some('a'));
        assert_eq!(glyph_name_to_unicode("0"), Some('0'));
        assert_eq!(glyph_name_to_unicode("+"), Some('+'));
        assert_eq!(glyph_name_to_unicode("?"), Some('?'));

        // uniXXXX escapes (4 hex digits)
        assert_eq!(glyph_name_to_unicode("uni0041"), Some('A'));
        assert_eq!(glyph_name_to_unicode("uni00E9"), Some('é'));
        assert_eq!(glyph_name_to_unicode("uni20AC"), Some('€'));
        assert_eq!(glyph_name_to_unicode("uni00DF"), Some('ß'));

        // uXXXX / uXXXXXX escapes (4..6 hex digits)
        assert_eq!(glyph_name_to_unicode("u0041"), Some('A'));
        assert_eq!(glyph_name_to_unicode("u00E9"), Some('é'));
        assert_eq!(glyph_name_to_unicode("u1F600"), Some('😀'));
        assert_eq!(glyph_name_to_unicode("u000041"), Some('A'));

        // Basic Latin / AGL names
        assert_eq!(glyph_name_to_unicode("space"), Some(' '));
        assert_eq!(glyph_name_to_unicode("zero"), Some('0'));
        assert_eq!(glyph_name_to_unicode("nine"), Some('9'));
        assert_eq!(glyph_name_to_unicode("exclam"), Some('!'));

        // Accented letters
        assert_eq!(glyph_name_to_unicode("aacute"), Some('á'));
        assert_eq!(glyph_name_to_unicode("eacute"), Some('é'));
        assert_eq!(glyph_name_to_unicode("atilde"), Some('ã'));
        assert_eq!(glyph_name_to_unicode("ccedilla"), Some('ç'));
        assert_eq!(glyph_name_to_unicode("Adieresis"), Some('Ä'));
        assert_eq!(glyph_name_to_unicode("Eacute"), Some('É'));
        assert_eq!(glyph_name_to_unicode("ntilde"), Some('ñ'));
        assert_eq!(glyph_name_to_unicode("Oslash"), Some('Ø'));
        assert_eq!(glyph_name_to_unicode("oslash"), Some('ø'));
        assert_eq!(glyph_name_to_unicode("Scaron"), Some('Š'));
        assert_eq!(glyph_name_to_unicode("scaron"), Some('š'));
        assert_eq!(glyph_name_to_unicode("Zcaron"), Some('Ž'));
        assert_eq!(glyph_name_to_unicode("zcaron"), Some('ž'));

        // German sharp s
        assert_eq!(glyph_name_to_unicode("germandbls"), Some('ß'));

        // Typographical & Punctuation
        assert_eq!(glyph_name_to_unicode("periodcentered"), Some('·'));
        assert_eq!(glyph_name_to_unicode("bullet"), Some('•'));
        assert_eq!(glyph_name_to_unicode("hyphen"), Some('-'));
        assert_eq!(glyph_name_to_unicode("endash"), Some('–'));
        assert_eq!(glyph_name_to_unicode("emdash"), Some('—'));
        assert_eq!(glyph_name_to_unicode("quoteleft"), Some('‘'));
        assert_eq!(glyph_name_to_unicode("quoteright"), Some('’'));
        assert_eq!(glyph_name_to_unicode("quotedblleft"), Some('“'));
        assert_eq!(glyph_name_to_unicode("quotedblright"), Some('”'));
        assert_eq!(glyph_name_to_unicode("ellipsis"), Some('…'));

        // Ligatures
        assert_eq!(glyph_name_to_unicode("fi"), Some('ﬁ'));
        assert_eq!(glyph_name_to_unicode("fl"), Some('ﬂ'));
        assert_eq!(glyph_name_to_unicode("ffi"), Some('ﬃ'));
        assert_eq!(glyph_name_to_unicode("ffl"), Some('ﬄ'));
        assert_eq!(glyph_name_to_unicode("ff"), Some('ﬀ'));
        assert_eq!(glyph_name_to_unicode("oe"), Some('œ'));
        assert_eq!(glyph_name_to_unicode("OE"), Some('Œ'));
        assert_eq!(glyph_name_to_unicode("ae"), Some('æ'));
        assert_eq!(glyph_name_to_unicode("AE"), Some('Æ'));

        // Symbols
        assert_eq!(glyph_name_to_unicode("plus"), Some('+'));
        assert_eq!(glyph_name_to_unicode("minus"), Some('−'));
        assert_eq!(glyph_name_to_unicode("slash"), Some('/'));
        assert_eq!(glyph_name_to_unicode("backslash"), Some('\\'));
        assert_eq!(glyph_name_to_unicode("Euro"), Some('€'));
        assert_eq!(glyph_name_to_unicode("trademark"), Some('™'));
        assert_eq!(glyph_name_to_unicode("copyright"), Some('©'));
        assert_eq!(glyph_name_to_unicode("registered"), Some('®'));
        assert_eq!(glyph_name_to_unicode("checkmark"), Some('✓'));

        // Variant suffixes
        assert_eq!(glyph_name_to_unicode("A.swash"), Some('A'));
        assert_eq!(glyph_name_to_unicode("aacute.alt"), Some('á'));

        // Unknown names
        assert_eq!(glyph_name_to_unicode("nonexistent_glyph_xyz"), None);
    }

    #[test]
    fn test_decode_with_encoding_differences() {
        let mut diffs = HashMap::new();
        diffs.insert(1, "aacute".to_string());
        diffs.insert(2, "germandbls".to_string());
        diffs.insert(3, "bullet".to_string());
        diffs.insert(4, "fi".to_string());
        diffs.insert(5, "uni0041".to_string());
        diffs.insert(6, "u00E9".to_string());
        diffs.insert(7, "endash".to_string());
        diffs.insert(8, "minus".to_string());

        let font_info = FontInfo {
            name: "CustomFont".to_string(),
            font_type: "Type1".to_string(),
            encoding: Some("WinAnsiEncoding".to_string()),
            to_unicode: None,
            differences: Some(diffs),
            descendant_font: None,
            cid_ordering: None,
            metrics: FontMetrics::default(),
            cid_encoding: None,
        };

        let decoded = decode_text_with_font(&[1, 2, 3, 4, 5, 6, 7, 8], &font_info).unwrap();
        assert_eq!(decoded, "áß•ﬁAé–−");
    }

    #[test]
    fn test_decode_winansi() {
        assert_eq!(decode_winansi(0x20), ' ');
        assert_eq!(decode_winansi(0x41), 'A');
        assert_eq!(decode_winansi(0x80), '€');
        assert_eq!(decode_winansi(0x99), '™');
    }

    #[test]
    fn test_decode_macroman() {
        assert_eq!(decode_macroman(0x20), ' ');
        assert_eq!(decode_macroman(0x41), 'A');
        assert_eq!(decode_macroman(0x80), 'Ä');
        assert_eq!(decode_macroman(0x87), 'á');
        // Issue #662: MacRoman bytes in 0xA0..=0xFF range
        assert_eq!(decode_macroman(0xA0), '†');
        assert_eq!(decode_macroman(0xBB), 'ª');
        assert_eq!(decode_macroman(0xBC), 'º');
        assert_eq!(decode_macroman(0xDB), '¤');
        assert_eq!(decode_macroman(0xD2), '“');
        assert_eq!(decode_macroman(0xD3), '”');
        assert_eq!(decode_macroman(0xE7), 'Á');
    }

    #[test]
    fn test_font_info_creation() {
        let font_info = FontInfo {
            name: "Helvetica".to_string(),
            font_type: "Type1".to_string(),
            encoding: Some("WinAnsiEncoding".to_string()),
            to_unicode: None,
            differences: None,
            descendant_font: None,
            cid_ordering: None,
            metrics: FontMetrics::default(),
            cid_encoding: None,
        };

        assert_eq!(font_info.name, "Helvetica");
        assert_eq!(font_info.font_type, "Type1");
        assert_eq!(font_info.encoding, Some("WinAnsiEncoding".to_string()));
    }

    #[test]
    fn simple_font_one_byte_tounicode_under_two_byte_codespace_decodes() {
        // #302 symptom 3 end-to-end: a simple TrueType font whose ToUnicode
        // declares the generic 2-byte codespace <0000><FFFF> but maps 1-byte
        // codes. Before the fix `decode_with_cmap` returned "" for any string
        // containing such a code, the result was rejected as garbage, and the
        // wrong base-encoding fallback produced U+FFFD. The full decode chain
        // must now recover the real characters (here U+2019 RIGHT SINGLE
        // QUOTATION MARK for code 0x92, followed by a comma).
        let cmap_src = br#"begincmap
/CMapName /Adobe-Identity-UCS def
/CMapType 2 def
1 begincodespacerange
<0000> <FFFF>
endcodespacerange
2 beginbfchar
<92> <2019>
<2C> <002C>
endbfchar
endcmap
"#;
        let cmap = crate::text::cmap::CMap::parse(cmap_src).unwrap();
        let font_info = FontInfo {
            name: "VUNXGH+ArialMT".to_string(),
            font_type: "TrueType".to_string(),
            encoding: Some("WinAnsiEncoding".to_string()),
            to_unicode: Some(cmap),
            differences: None,
            descendant_font: None,
            cid_ordering: None,
            metrics: FontMetrics::default(),
            cid_encoding: None,
        };
        let out = decode_text_with_font(&[0x92, 0x2C], &font_info).unwrap();
        assert_eq!(out, "\u{2019},");
    }

    #[test]
    fn embedded_encoding_cmap_decodes_via_cid_table() {
        use crate::text::cid_to_unicode::AdobeCidCollection;
        use crate::text::encoding_cmap::{CidEncoding, EncodingCMap};
        // Pick a CID that is present in the GB1 collection; assert decode equals
        // exactly the GB1 table's Unicode for that CID (real content, not shape).
        let coll = AdobeCidCollection::from_ordering("GB1").unwrap();
        // find a small CID that maps (scan upward to be robust to table contents)
        let (cid, expected) = (1u16..2000)
            .find_map(|c| coll.cid_to_unicode(c).map(|ch| (c, ch)))
            .expect("GB1 has at least one mapped CID");

        let enc = EncodingCMap::parse(
            format!(
                "begincmap\n1 begincodespacerange <0000> <FFFF> endcodespacerange\n\
1 begincidchar <0041> {cid} endcidchar\nendcmap"
            )
            .as_bytes(),
        )
        .unwrap();

        let descendant = FontInfo {
            name: "Desc".into(),
            font_type: "CIDFontType0".into(),
            encoding: None,
            to_unicode: None,
            differences: None,
            descendant_font: None,
            cid_ordering: Some("GB1".into()),
            metrics: FontMetrics::default(),
            cid_encoding: None,
        };
        let parent = FontInfo {
            name: "Type0".into(),
            font_type: "Type0".into(),
            encoding: None,
            to_unicode: None,
            differences: None,
            descendant_font: Some(Box::new(descendant)),
            cid_ordering: None,
            metrics: FontMetrics::default(),
            cid_encoding: Some(CidEncoding::Cmap(enc)),
        };

        let out = decode_text_with_font(&[0x00, 0x41], &parent).unwrap();
        assert_eq!(out, expected.to_string());
    }

    #[test]
    fn type0_utf16be_encoding_decodes_through_dispatch() {
        use crate::text::encoding_cmap::CidEncoding;
        let descendant = FontInfo {
            name: "Desc".into(),
            font_type: "CIDFontType2".into(),
            encoding: None,
            to_unicode: None,
            differences: None,
            descendant_font: None,
            cid_ordering: Some("GB1".into()),
            metrics: FontMetrics::default(),
            cid_encoding: None,
        };
        let parent = FontInfo {
            name: "Type0".into(),
            font_type: "Type0".into(),
            encoding: None,
            to_unicode: None,
            differences: None,
            descendant_font: Some(Box::new(descendant)),
            cid_ordering: None,
            metrics: FontMetrics::default(),
            cid_encoding: Some(CidEncoding::Utf16Be),
        };
        // UTF-16BE code 0x4E2D ('中') must decode directly, ignoring the ordering.
        let out = decode_text_with_font(&[0x4E, 0x2D], &parent).unwrap();
        assert_eq!(out, "中");
    }

    #[test]
    fn explicit_bfchar_overrides_usecmap_cid_fallback() {
        use crate::text::cmap::CMap;
        let with_override = CMap::parse(
            b"begincmap\n/Adobe-Korea1-UCS2 usecmap\n\
1 begincodespacerange <0000> <FFFF> endcodespacerange\n\
1 beginbfchar <0041> <AC00> endbfchar\nendcmap",
        )
        .expect("parse with_override");
        let without = CMap::parse(
            b"begincmap\n/Adobe-Korea1-UCS2 usecmap\n\
1 begincodespacerange <0000> <FFFF> endcodespacerange\nendcmap",
        )
        .expect("parse without");
        let got = decode_with_cmap(&[0x00, 0x41], &with_override, false).unwrap();
        let fallback = decode_with_cmap(&[0x00, 0x41], &without, false).unwrap();
        assert_eq!(got, "\u{AC00}", "explicit bfchar must win");
        assert_ne!(
            got, fallback,
            "explicit mapping must override the CID-table fallback"
        );
    }

    #[test]
    fn extract_font_metrics_type3_font_matrix_scaling() {
        use crate::parser::objects::PdfArray;
        use crate::parser::PdfReader;
        use std::io::Cursor;

        // Dummy PDF document for resolving direct objects
        let pdf = b"%PDF-1.4\n1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n2 0 obj\n<< /Type /Pages /Kids [] /Count 0 >>\nendobj\nxref\n0 3\n0000000000 65535 f \n0000000009 00000 n \n0000000058 00000 n \ntrailer\n<< /Size 3 /Root 1 0 R >>\nstartxref\n115\n%%EOF\n";
        let document = PdfDocument::new(PdfReader::new(Cursor::new(&pdf[..])).unwrap());
        let extractor = CMapTextExtractor::new();

        // 1. Type 3 font with FontMatrix [1.0 0 0 1.0 0 0] and Widths [0.5, 0.75]
        let mut font_dict = PdfDictionary::new();
        font_dict.insert("Subtype".into(), PdfObject::Name(PdfName("Type3".into())));
        font_dict.insert("FirstChar".into(), PdfObject::Integer(65));
        font_dict.insert("LastChar".into(), PdfObject::Integer(66));
        font_dict.insert(
            "FontMatrix".into(),
            PdfObject::Array(PdfArray(vec![
                PdfObject::Real(1.0),
                PdfObject::Real(0.0),
                PdfObject::Real(0.0),
                PdfObject::Real(1.0),
                PdfObject::Real(0.0),
                PdfObject::Real(0.0),
            ])),
        );
        font_dict.insert(
            "Widths".into(),
            PdfObject::Array(PdfArray(vec![PdfObject::Real(0.5), PdfObject::Real(0.75)])),
        );

        let metrics = extractor
            .extract_font_metrics(&font_dict, &document)
            .unwrap();
        assert_eq!(metrics.first_char, Some(65));
        assert_eq!(metrics.last_char, Some(66));
        assert_eq!(metrics.widths, Some(vec![500.0, 750.0]));

        // 2. Type 3 font with FontMatrix [0.001 0 0 0.001 0 0] and Widths [500, 750]
        let mut font_dict2 = PdfDictionary::new();
        font_dict2.insert("Subtype".into(), PdfObject::Name(PdfName("Type3".into())));
        font_dict2.insert("FirstChar".into(), PdfObject::Integer(65));
        font_dict2.insert("LastChar".into(), PdfObject::Integer(66));
        font_dict2.insert(
            "FontMatrix".into(),
            PdfObject::Array(PdfArray(vec![
                PdfObject::Real(0.001),
                PdfObject::Real(0.0),
                PdfObject::Real(0.0),
                PdfObject::Real(0.001),
                PdfObject::Real(0.0),
                PdfObject::Real(0.0),
            ])),
        );
        font_dict2.insert(
            "Widths".into(),
            PdfObject::Array(PdfArray(vec![
                PdfObject::Integer(500),
                PdfObject::Integer(750),
            ])),
        );

        let metrics2 = extractor
            .extract_font_metrics(&font_dict2, &document)
            .unwrap();
        assert_eq!(metrics2.widths, Some(vec![500.0, 750.0]));

        // 3. Simple font with no FontMatrix (implicit [0.001 0 0 0.001 0 0])
        let mut font_dict3 = PdfDictionary::new();
        font_dict3.insert(
            "Subtype".into(),
            PdfObject::Name(PdfName("TrueType".into())),
        );
        font_dict3.insert("FirstChar".into(), PdfObject::Integer(32));
        font_dict3.insert("LastChar".into(), PdfObject::Integer(33));
        font_dict3.insert(
            "Widths".into(),
            PdfObject::Array(PdfArray(vec![
                PdfObject::Integer(250),
                PdfObject::Integer(333),
            ])),
        );

        let metrics3 = extractor
            .extract_font_metrics(&font_dict3, &document)
            .unwrap();
        assert_eq!(metrics3.widths, Some(vec![250.0, 333.0]));
    }
}

#[cfg(test)]
#[path = "usecmap_resolution_tests.rs"]
mod usecmap_resolution_tests;

#[cfg(test)]
#[path = "vertical_metrics_tests.rs"]
mod vertical_metrics_tests;
