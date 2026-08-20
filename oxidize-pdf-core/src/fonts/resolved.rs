//! Parser-side resolved font resources for downstream renderers.

use crate::fonts::{Type3Font, MAX_FONT_STREAM_SIZE};
use crate::parser::document::PdfDocument;
use crate::parser::objects::{PdfDictionary, PdfObject};
use crate::parser::{ParseError, ParseResult};
use crate::text::cmap::CMap;
use crate::text::encoding::TextEncoding;
use crate::text::encoding_cmap::{decode_utf16be, resolve_predefined, CidEncoding, EncodingCMap};
use std::collections::{BTreeMap, HashMap};
use std::io::{Read, Seek};

const MAX_CMAP_STREAM_SIZE: usize = 8 * 1024 * 1024;
const MAX_CID_TO_GID_MAP_SIZE: usize = (u16::MAX as usize + 1) * 2;

/// Concrete PDF font subtype used for rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontSubtype {
    Type1,
    TrueType,
    CidFontType0,
    CidFontType2,
    Type3,
}

/// Direction declared by a composite font's encoding CMap.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WritingMode {
    Horizontal,
    Vertical,
}

/// Decoded embedded font program format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmbeddedFontFormat {
    Type1,
    TrueType,
    Type1C,
    CidFontType0C,
    OpenType,
}

/// Embedded font bytes and their PDF-declared format.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmbeddedFont {
    /// Container/program format declared by the PDF font descriptor.
    pub format: EmbeddedFontFormat,
    /// Decoded, size-bounded font-program bytes.
    pub data: Vec<u8>,
}

/// One renderer-ready glyph decoded from a `Tj` string or a string item in `TJ`.
#[derive(Debug, Clone, PartialEq)]
pub struct DecodedGlyph {
    /// Exact bytes consumed for this PDF character code.
    pub source_code: Vec<u8>,
    /// CID selected by a composite encoding, or `None` for simple fonts and unmapped codes.
    pub cid: Option<u32>,
    /// Glyph identifier selected through `CIDToGIDMap`, when applicable.
    pub gid: Option<u16>,
    /// Best available Unicode mapping, preferring `ToUnicode`.
    pub unicode: Option<String>,
    /// Horizontal or vertical advance from the font's width tables.
    pub advance: f64,
}

#[derive(Debug, Clone)]
enum CodeEncoding {
    Simple,
    Identity,
    Composite(CidEncoding),
}

#[derive(Debug, Clone)]
enum CidToGid {
    Identity,
    Table(Vec<u16>),
}

#[derive(Debug, Clone)]
struct CidWidths {
    explicit: HashMap<u32, f64>,
    ranges: Vec<(u32, u32, f64)>,
    default: f64,
}

impl CidWidths {
    fn width(&self, cid: u32) -> f64 {
        self.explicit
            .get(&cid)
            .copied()
            .or_else(|| {
                self.ranges
                    .iter()
                    .rev()
                    .find(|(first, last, _)| cid >= *first && cid <= *last)
                    .map(|(_, _, width)| *width)
            })
            .unwrap_or(self.default)
    }
}

/// A font entry resolved entirely through [`PdfDocument`]'s parser object model.
#[derive(Debug, Clone)]
pub struct ResolvedFontResource {
    /// Name used for the font in the page resource dictionary.
    pub resource_name: String,
    /// PDF `BaseFont`/Type3 `Name`, when present.
    pub base_font: Option<String>,
    /// Concrete simple, CID descendant, or Type3 subtype.
    pub subtype: FontSubtype,
    /// Named PDF encoding, when the font uses one.
    pub encoding: Option<String>,
    /// Writing direction selected by a composite encoding CMap.
    pub writing_mode: WritingMode,
    /// Simple-font character-code overrides from `Differences`.
    pub differences: BTreeMap<u8, String>,
    /// Decoded embedded font program, when present.
    pub embedded_font: Option<EmbeddedFont>,
    /// Parsed Type3 glyph programs, only for Type3 resources.
    pub type3: Option<Type3Font>,
    code_encoding: CodeEncoding,
    to_unicode: Option<CMap>,
    cid_to_gid: Option<CidToGid>,
    cid_widths: Option<CidWidths>,
    first_char: u32,
    simple_widths: Vec<f64>,
    missing_width: f64,
    symbolic: bool,
}

impl ResolvedFontResource {
    /// Resolve a named font from a page's effective, inherited resources.
    ///
    /// # Errors
    ///
    /// Returns a parser error when the page/resource hierarchy or font data is
    /// missing, malformed, cyclic, unsupported, or exceeds a stream limit.
    pub fn from_page<R: Read + Seek>(
        document: &PdfDocument<R>,
        page_index: u32,
        resource_name: &str,
    ) -> ParseResult<Self> {
        let page = document.get_page(page_index)?;
        let resources = document
            .get_page_resources(&page)?
            .ok_or_else(|| missing("page Resources"))?;
        let fonts = resolve_required(document, resources, "Font")?;
        let fonts = expect_dict(&fonts, "page Font resources")?;
        let font_object = fonts
            .get(resource_name)
            .ok_or_else(|| missing(&format!("font resource /{resource_name}")))?;
        Self::resolve(document, resource_name, font_object)
    }

    /// Resolve a direct or indirect font dictionary.
    ///
    /// # Errors
    ///
    /// Returns a parser error for malformed, cyclic, unsupported, or oversized
    /// font resources.
    pub fn resolve<R: Read + Seek>(
        document: &PdfDocument<R>,
        resource_name: &str,
        font_object: &PdfObject,
    ) -> ParseResult<Self> {
        let resolved = document.resolve(font_object)?;
        let font = expect_dict(&resolved, "font resource")?;
        let declared_subtype = required_name(font, "Subtype")?;
        if declared_subtype == "Type3" {
            let type3 = Type3Font::resolve(font_object, document)?;
            let to_unicode = resolve_cmap(document, font.get("ToUnicode"))?;
            let mut differences = BTreeMap::new();
            let mut widths = vec![0.0; 256];
            for glyph in type3.glyphs() {
                differences.insert(glyph.code, glyph.name.clone());
                widths[glyph.code as usize] = glyph.width;
            }
            return Ok(Self {
                resource_name: resource_name.into(),
                base_font: type3.name.clone(),
                subtype: FontSubtype::Type3,
                encoding: None,
                writing_mode: WritingMode::Horizontal,
                differences,
                embedded_font: None,
                type3: Some(type3),
                code_encoding: CodeEncoding::Simple,
                to_unicode,
                cid_to_gid: None,
                cid_widths: None,
                first_char: 0,
                simple_widths: widths,
                missing_width: 0.0,
                symbolic: false,
            });
        }

        let base_font = font
            .get("BaseFont")
            .and_then(PdfObject::as_name)
            .map(|name| name.0.clone());
        let to_unicode = resolve_cmap(document, font.get("ToUnicode"))?;
        let (encoding, differences, code_encoding, writing_mode) =
            resolve_encoding(document, font.get("Encoding"), declared_subtype == "Type0")?;

        let descendant_storage;
        let concrete = if declared_subtype == "Type0" {
            let descendants = resolve_required(document, font, "DescendantFonts")?;
            let descendants = descendants
                .as_array()
                .ok_or_else(|| syntax("DescendantFonts must be an array"))?;
            let descendant = descendants
                .0
                .first()
                .ok_or_else(|| syntax("DescendantFonts must not be empty"))?;
            descendant_storage = document.resolve(descendant)?;
            expect_dict(&descendant_storage, "CID descendant font")?
        } else {
            font
        };
        let concrete_name = required_name(concrete, "Subtype")?;
        let subtype = match concrete_name {
            "Type1" | "MMType1" => FontSubtype::Type1,
            "TrueType" => FontSubtype::TrueType,
            "CIDFontType0" => FontSubtype::CidFontType0,
            "CIDFontType2" => FontSubtype::CidFontType2,
            other => return Err(syntax(&format!("unsupported font subtype /{other}"))),
        };
        let composite = declared_subtype == "Type0";
        let cid_subtype = matches!(
            subtype,
            FontSubtype::CidFontType0 | FontSubtype::CidFontType2
        );
        if composite != cid_subtype {
            return Err(syntax(if composite {
                "Type0 descendant must be CIDFontType0 or CIDFontType2"
            } else {
                "CID font subtype must be a descendant of Type0"
            }));
        }

        let descriptor = resolve_optional_dict(document, concrete.get("FontDescriptor"))?;
        let embedded_font = descriptor
            .as_ref()
            .map(|dict| resolve_embedded_font(document, dict))
            .transpose()?
            .flatten();
        let missing_width = descriptor
            .as_ref()
            .and_then(|dict| dict.get("MissingWidth"))
            .and_then(PdfObject::as_real)
            .unwrap_or(0.0);
        let symbolic = descriptor
            .as_ref()
            .and_then(|dict| dict.get("Flags"))
            .and_then(PdfObject::as_integer)
            .is_some_and(|flags| flags & 4 != 0)
            || base_font.as_deref() == Some("Symbol");

        let cid_widths = matches!(
            subtype,
            FontSubtype::CidFontType0 | FontSubtype::CidFontType2
        )
        .then(|| parse_cid_widths(document, concrete))
        .transpose()?;
        let cid_to_gid = if subtype == FontSubtype::CidFontType2 {
            resolve_cid_to_gid(document, concrete.get("CIDToGIDMap"))?
        } else {
            None
        };
        let first_char = font
            .get("FirstChar")
            .and_then(PdfObject::as_integer)
            .and_then(|value| u32::try_from(value).ok())
            .unwrap_or(0);
        let simple_widths = resolve_number_array(document, font.get("Widths"))?;

        Ok(Self {
            resource_name: resource_name.into(),
            base_font,
            subtype,
            encoding,
            writing_mode,
            differences,
            embedded_font,
            type3: None,
            code_encoding,
            to_unicode,
            cid_to_gid,
            cid_widths,
            first_char,
            simple_widths,
            missing_width,
            symbolic,
        })
    }

    /// Decode a PDF string into renderer-ready glyph identities and advances.
    ///
    /// # Errors
    ///
    /// Returns a parser error when a character code is truncated or wider than
    /// the supported four-byte PDF code space.
    pub fn decode_glyphs(&self, bytes: &[u8]) -> ParseResult<Vec<DecodedGlyph>> {
        let mut glyphs = Vec::new();
        let mut position = 0;
        while position < bytes.len() {
            let code_len = match &self.code_encoding {
                CodeEncoding::Simple => 1,
                CodeEncoding::Identity | CodeEncoding::Composite(CidEncoding::Utf16Be) => 2,
                CodeEncoding::Composite(CidEncoding::Cmap(cmap)) => {
                    cmap.code_len_at(bytes, position)
                }
            };
            if position + code_len > bytes.len() {
                return Err(syntax("truncated character code"));
            }
            let code = bytes[position..position + code_len].to_vec();
            position += code_len;
            let cid = match &self.code_encoding {
                CodeEncoding::Simple => None,
                CodeEncoding::Identity | CodeEncoding::Composite(CidEncoding::Utf16Be) => {
                    Some(be_code(&code)?)
                }
                CodeEncoding::Composite(CidEncoding::Cmap(cmap)) => cmap
                    .map_code_to_cid(&code)
                    .or_else(|| cmap.map_notdef(&code))
                    .map(u32::from),
            };
            let gid = cid.and_then(|cid| self.gid_for(cid));
            let unicode = self.unicode_for(&code);
            let advance = match &self.cid_widths {
                Some(widths) => cid.map_or(widths.default, |cid| widths.width(cid)),
                None => self.simple_width(be_code(&code)?),
            };
            glyphs.push(DecodedGlyph {
                source_code: code,
                cid,
                gid,
                unicode,
                advance,
            });
        }
        Ok(glyphs)
    }

    fn gid_for(&self, cid: u32) -> Option<u16> {
        match self.cid_to_gid.as_ref()? {
            CidToGid::Identity => u16::try_from(cid).ok(),
            CidToGid::Table(table) => table.get(cid as usize).copied(),
        }
    }

    fn unicode_for(&self, code: &[u8]) -> Option<String> {
        if let Some(cmap) = &self.to_unicode {
            if let Some(mapped) = cmap.map(code) {
                return cmap.to_unicode(&mapped);
            }
        }
        if matches!(
            self.code_encoding,
            CodeEncoding::Composite(CidEncoding::Utf16Be)
        ) {
            return Some(decode_utf16be(code));
        }
        if matches!(self.code_encoding, CodeEncoding::Simple) {
            let value = code[0];
            if let Some(name) = self.differences.get(&value) {
                return glyph_name_to_unicode(name).map(str::to_owned);
            }
            if self.symbolic {
                return symbol_code_to_unicode(value).map(str::to_owned);
            }
            let decoder = match self.encoding.as_deref() {
                Some("WinAnsiEncoding") => TextEncoding::WinAnsiEncoding,
                Some("MacRomanEncoding") => TextEncoding::MacRomanEncoding,
                _ => TextEncoding::StandardEncoding,
            };
            return Some(decoder.decode(&[value]));
        }
        None
    }

    fn simple_width(&self, code: u32) -> f64 {
        code.checked_sub(self.first_char)
            .and_then(|offset| self.simple_widths.get(offset as usize))
            .copied()
            .unwrap_or(self.missing_width)
    }
}

fn resolve_encoding<R: Read + Seek>(
    document: &PdfDocument<R>,
    object: Option<&PdfObject>,
    composite: bool,
) -> ParseResult<(
    Option<String>,
    BTreeMap<u8, String>,
    CodeEncoding,
    WritingMode,
)> {
    let Some(object) = object else {
        if composite {
            return Err(syntax("Type0 font is missing its Encoding"));
        }
        return Ok((
            None,
            BTreeMap::new(),
            CodeEncoding::Simple,
            WritingMode::Horizontal,
        ));
    };
    let resolved = document.resolve(object)?;
    match resolved {
        PdfObject::Name(name) if composite => {
            let vertical = name.0.ends_with("-V");
            let code_encoding =
                if matches!(name.0.as_str(), "Identity-H" | "Identity-V") {
                    CodeEncoding::Identity
                } else {
                    CodeEncoding::Composite(resolve_predefined(&name.0).ok_or_else(|| {
                        syntax(&format!("unsupported predefined CMap /{}", name.0))
                    })?)
                };
            Ok((
                Some(name.0),
                BTreeMap::new(),
                code_encoding,
                if vertical {
                    WritingMode::Vertical
                } else {
                    WritingMode::Horizontal
                },
            ))
        }
        PdfObject::Name(name) => Ok((
            Some(name.0),
            BTreeMap::new(),
            CodeEncoding::Simple,
            WritingMode::Horizontal,
        )),
        PdfObject::Dictionary(dict) if !composite => {
            let base = dict
                .get("BaseEncoding")
                .and_then(PdfObject::as_name)
                .map(|name| name.0.clone());
            Ok((
                base,
                parse_differences(&dict)?,
                CodeEncoding::Simple,
                WritingMode::Horizontal,
            ))
        }
        PdfObject::Stream(stream) if composite => {
            let data = document.decode_stream_with_limit(&stream, MAX_CMAP_STREAM_SIZE)?;
            let cmap = EncodingCMap::parse(&data)?;
            let mode = if cmap.wmode == 1 {
                WritingMode::Vertical
            } else {
                WritingMode::Horizontal
            };
            Ok((
                None,
                BTreeMap::new(),
                CodeEncoding::Composite(CidEncoding::Cmap(cmap)),
                mode,
            ))
        }
        _ => Err(syntax("invalid font Encoding")),
    }
}

fn resolve_cmap<R: Read + Seek>(
    document: &PdfDocument<R>,
    object: Option<&PdfObject>,
) -> ParseResult<Option<CMap>> {
    let Some(object) = object else {
        return Ok(None);
    };
    let resolved = document.resolve(object)?;
    let stream = resolved
        .as_stream()
        .ok_or_else(|| syntax("ToUnicode must be a stream"))?;
    let data = document.decode_stream_with_limit(stream, MAX_CMAP_STREAM_SIZE)?;
    CMap::parse(&data).map(Some)
}

fn resolve_cid_to_gid<R: Read + Seek>(
    document: &PdfDocument<R>,
    object: Option<&PdfObject>,
) -> ParseResult<Option<CidToGid>> {
    let Some(object) = object else {
        return Ok(Some(CidToGid::Identity));
    };
    let resolved = document.resolve(object)?;
    match resolved {
        PdfObject::Name(name) if name.0 == "Identity" => Ok(Some(CidToGid::Identity)),
        PdfObject::Stream(stream) => {
            let data = document.decode_stream_with_limit(&stream, MAX_CID_TO_GID_MAP_SIZE)?;
            if data.len() % 2 != 0 {
                return Err(syntax("CIDToGIDMap stream has odd length"));
            }
            Ok(Some(CidToGid::Table(
                data.chunks_exact(2)
                    .map(|pair| u16::from_be_bytes([pair[0], pair[1]]))
                    .collect(),
            )))
        }
        _ => Err(syntax("CIDToGIDMap must be /Identity or a stream")),
    }
}

fn parse_cid_widths<R: Read + Seek>(
    document: &PdfDocument<R>,
    dict: &PdfDictionary,
) -> ParseResult<CidWidths> {
    let default = dict
        .get("DW")
        .and_then(PdfObject::as_real)
        .unwrap_or(1000.0);
    let entries = resolve_number_or_array_object(document, dict.get("W"))?;
    let Some(entries) = entries.and_then(|object| object.as_array().cloned()) else {
        return Ok(CidWidths {
            explicit: HashMap::new(),
            ranges: Vec::new(),
            default,
        });
    };
    let mut explicit = HashMap::new();
    let mut ranges = Vec::new();
    let mut index = 0;
    while index < entries.0.len() {
        let first = entries.0[index]
            .as_integer()
            .and_then(|value| u32::try_from(value).ok())
            .ok_or_else(|| syntax("invalid first CID in W"))?;
        match entries.0.get(index + 1) {
            Some(PdfObject::Array(widths)) => {
                for (offset, width) in widths.0.iter().enumerate() {
                    let cid = first
                        .checked_add(u32::try_from(offset).map_err(|_| syntax("W range overflow"))?)
                        .ok_or_else(|| syntax("W range overflow"))?;
                    explicit.insert(
                        cid,
                        width.as_real().ok_or_else(|| syntax("invalid W width"))?,
                    );
                }
                index += 2;
            }
            Some(last) => {
                let last = last
                    .as_integer()
                    .and_then(|value| u32::try_from(value).ok())
                    .ok_or_else(|| syntax("invalid last CID in W"))?;
                let width = entries
                    .0
                    .get(index + 2)
                    .and_then(PdfObject::as_real)
                    .ok_or_else(|| syntax("missing W range width"))?;
                if last < first {
                    return Err(syntax("reversed CID width range"));
                }
                ranges.push((first, last, width));
                index += 3;
            }
            None => return Err(syntax("truncated W array")),
        }
    }
    Ok(CidWidths {
        explicit,
        ranges,
        default,
    })
}

fn resolve_embedded_font<R: Read + Seek>(
    document: &PdfDocument<R>,
    descriptor: &PdfDictionary,
) -> ParseResult<Option<EmbeddedFont>> {
    for (key, default_format) in [
        ("FontFile", EmbeddedFontFormat::Type1),
        ("FontFile2", EmbeddedFontFormat::TrueType),
        ("FontFile3", EmbeddedFontFormat::Type1C),
    ] {
        let Some(object) = descriptor.get(key) else {
            continue;
        };
        let resolved = document.resolve(object)?;
        let stream = resolved
            .as_stream()
            .ok_or_else(|| syntax(&format!("{key} must be a stream")))?;
        let format = if key == "FontFile3" {
            match stream
                .dict
                .get("Subtype")
                .and_then(PdfObject::as_name)
                .map(|n| n.0.as_str())
            {
                Some("Type1C") => EmbeddedFontFormat::Type1C,
                Some("CIDFontType0C") => EmbeddedFontFormat::CidFontType0C,
                Some("OpenType") => EmbeddedFontFormat::OpenType,
                _ => return Err(syntax("FontFile3 has unsupported or missing Subtype")),
            }
        } else {
            default_format
        };
        let data = document.decode_stream_with_limit(stream, MAX_FONT_STREAM_SIZE)?;
        return Ok(Some(EmbeddedFont { format, data }));
    }
    Ok(None)
}

fn parse_differences(dict: &PdfDictionary) -> ParseResult<BTreeMap<u8, String>> {
    let mut result = BTreeMap::new();
    let Some(array) = dict.get("Differences").and_then(PdfObject::as_array) else {
        return Ok(result);
    };
    let mut code = None;
    for object in &array.0 {
        if let Some(value) = object.as_integer() {
            code =
                Some(u8::try_from(value).map_err(|_| syntax("Differences code outside 0..=255"))?);
        } else if let Some(name) = object.as_name() {
            let current = code.ok_or_else(|| syntax("Differences name without a code"))?;
            result.insert(current, name.0.clone());
            code = current.checked_add(1);
        } else {
            return Err(syntax("invalid Differences entry"));
        }
    }
    Ok(result)
}

fn resolve_number_array<R: Read + Seek>(
    document: &PdfDocument<R>,
    object: Option<&PdfObject>,
) -> ParseResult<Vec<f64>> {
    let Some(object) = resolve_number_or_array_object(document, object)? else {
        return Ok(Vec::new());
    };
    let array = object
        .as_array()
        .ok_or_else(|| syntax("Widths must be an array"))?;
    array
        .0
        .iter()
        .map(|value| {
            value
                .as_real()
                .ok_or_else(|| syntax("Widths must be numeric"))
        })
        .collect()
}

fn resolve_number_or_array_object<R: Read + Seek>(
    document: &PdfDocument<R>,
    object: Option<&PdfObject>,
) -> ParseResult<Option<PdfObject>> {
    object.map(|value| document.resolve(value)).transpose()
}

fn resolve_optional_dict<R: Read + Seek>(
    document: &PdfDocument<R>,
    object: Option<&PdfObject>,
) -> ParseResult<Option<PdfDictionary>> {
    let Some(object) = object else {
        return Ok(None);
    };
    let resolved = document.resolve(object)?;
    resolved
        .as_dict()
        .cloned()
        .ok_or_else(|| syntax("FontDescriptor must be a dictionary"))
        .map(Some)
}

fn resolve_required<R: Read + Seek>(
    document: &PdfDocument<R>,
    dict: &PdfDictionary,
    key: &str,
) -> ParseResult<PdfObject> {
    document.resolve(dict.get(key).ok_or_else(|| missing(key))?)
}

fn expect_dict<'a>(object: &'a PdfObject, description: &str) -> ParseResult<&'a PdfDictionary> {
    object
        .as_dict()
        .ok_or_else(|| syntax(&format!("{description} must be a dictionary")))
}

fn required_name<'a>(dict: &'a PdfDictionary, key: &str) -> ParseResult<&'a str> {
    dict.get(key)
        .and_then(PdfObject::as_name)
        .map(|name| name.0.as_str())
        .ok_or_else(|| missing(key))
}

fn be_code(code: &[u8]) -> ParseResult<u32> {
    if code.len() > 4 {
        return Err(syntax("character code exceeds four bytes"));
    }
    Ok(code
        .iter()
        .fold(0u32, |value, byte| (value << 8) | u32::from(*byte)))
}

fn glyph_name_to_unicode(name: &str) -> Option<&'static str> {
    match name {
        "bullet" => Some("•"),
        "space" => Some(" "),
        "hyphen" => Some("-"),
        "minus" => Some("−"),
        "checkmark" => Some("✓"),
        _ => None,
    }
}

fn symbol_code_to_unicode(code: u8) -> Option<&'static str> {
    match code {
        0xB7 => Some("•"),
        b'x' => Some("ξ"),
        _ => None,
    }
}

fn missing(key: &str) -> ParseError {
    ParseError::MissingKey(key.into())
}

fn syntax(message: &str) -> ParseError {
    ParseError::SyntaxError {
        position: 0,
        message: message.into(),
    }
}
