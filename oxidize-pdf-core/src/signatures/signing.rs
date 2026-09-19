//! Provider-neutral two-phase incremental PDF signing.

use super::{
    ensure_modification_allowed, IncrementalModification, SignatureError, SignatureResult,
};
use crate::parser::objects::{PdfArray, PdfDictionary, PdfName, PdfObject, PdfStream, PdfString};
use crate::parser::{PdfDocument, PdfReader};
use crate::text::{escape_pdf_string_literal, TextEncoding};
use crate::writer::IncrementalUpdate;
use std::collections::HashSet;
use std::io::Cursor;
use std::ops::Range;

const BYTE_RANGE_SENTINEL: i64 = i64::MAX;
const BYTE_RANGE_WIDTH: usize = 19;
const MIN_PLACEHOLDER_BYTES: usize = 256;
const MAX_PLACEHOLDER_BYTES: usize = 16 * 1024 * 1024;
const MAX_WATERMARK_PIXELS: usize = 16 * 1024 * 1024;

/// Rectangle for a visible signature widget.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SignatureRect {
    pub left: f64,
    pub bottom: f64,
    pub right: f64,
    pub top: f64,
}

/// RGB raster image drawn as a watermark in a visible signature appearance.
///
/// `rgb` contains exactly `width * height * 3` bytes in row-major RGB order.
/// The image is embedded in the signature appearance before the detached
/// signature's byte range is computed.
#[derive(Debug, Clone, PartialEq)]
pub struct SignatureWatermark {
    pub width: u32,
    pub height: u32,
    pub rgb: Vec<u8>,
    /// Opacity in the inclusive range `0.0..=1.0`.
    pub opacity: f32,
}

/// Content for a generated visible signature appearance.
///
/// The appearance is rendered in widget-local coordinates. Consequently the
/// page's crop box and rotation are applied by the PDF viewer together with
/// the widget annotation, rather than being baked into the artwork.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SignatureAppearance {
    pub signer_name: Option<String>,
    pub signing_date: Option<String>,
    /// Additional lines shown below the signer and date.
    pub text: Vec<String>,
    pub watermark: Option<SignatureWatermark>,
}

impl SignatureRect {
    pub(super) fn validate(self) -> SignatureResult<()> {
        let values = [self.left, self.bottom, self.right, self.top];
        if values.iter().any(|value| !value.is_finite())
            || self.right <= self.left
            || self.top <= self.bottom
        {
            return Err(invalid("signature rectangle must be finite and non-empty"));
        }
        Ok(())
    }

    pub(super) fn object(self) -> PdfObject {
        PdfObject::Array(PdfArray(vec![
            PdfObject::Real(self.left),
            PdfObject::Real(self.bottom),
            PdfObject::Real(self.right),
            PdfObject::Real(self.top),
        ]))
    }
}

/// Select an empty signature field or create a new widget.
#[derive(Debug, Clone, PartialEq)]
pub enum SignatureTarget {
    Existing {
        field_name: String,
        /// Child widget to update. `None` selects a combined field/widget.
        widget_index: Option<usize>,
        /// Optional replacement rectangle and generated normal appearance.
        rect: Option<SignatureRect>,
    },
    New {
        field_name: String,
        page_index: usize,
        rect: Option<SignatureRect>,
    },
}

/// Generic DocMDP permission written by a certification signature.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CertificationPermission {
    NoChanges = 1,
    FormFillAndSign = 2,
    FormFillSignAndAnnotate = 3,
}

/// Generic FieldMDP field selection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldLock {
    All,
    Include(Vec<String>),
    Exclude(Vec<String>),
}

/// Input for the prepare phase. No key or signer handle is accepted.
#[derive(Debug, Clone, PartialEq)]
pub struct SignaturePreparationOptions {
    pub target: SignatureTarget,
    pub placeholder_bytes: usize,
    /// PDF signature handler name, normally `Adobe.PPKLite`.
    pub filter: String,
    /// Signature container profile, for example `adbe.pkcs7.detached`.
    pub sub_filter: String,
    pub reason: Option<String>,
    pub location: Option<String>,
    pub contact_info: Option<String>,
    pub signing_time: Option<String>,
    pub certification: Option<CertificationPermission>,
    pub field_lock: Option<FieldLock>,
    /// Profile-specific signature dictionary entries.
    ///
    /// Structural entries owned by the signing engine cannot be overridden.
    pub additional_signature_entries: PdfDictionary,
}

impl SignaturePreparationOptions {
    pub fn invisible(field_name: impl Into<String>) -> Self {
        Self {
            target: SignatureTarget::New {
                field_name: field_name.into(),
                page_index: 0,
                rect: None,
            },
            placeholder_bytes: 16_384,
            filter: "Adobe.PPKLite".to_string(),
            sub_filter: "adbe.pkcs7.detached".to_string(),
            reason: None,
            location: None,
            contact_info: None,
            signing_time: None,
            certification: None,
            field_lock: None,
            additional_signature_entries: PdfDictionary::new(),
        }
    }

    /// Select an existing signature field without changing its widget appearance.
    pub fn existing(field_name: impl Into<String>) -> Self {
        let mut options = Self::invisible(field_name);
        let field_name = match options.target {
            SignatureTarget::New { field_name, .. } => field_name,
            SignatureTarget::Existing { .. } => unreachable!(),
        };
        options.target = SignatureTarget::Existing {
            field_name,
            widget_index: None,
            rect: None,
        };
        options
    }
}

/// Prepared PDF and exact detached-signature coverage.
#[derive(Debug, Clone)]
pub struct PreparedSignature {
    pdf: Vec<u8>,
    byte_range: super::ByteRange,
    contents_hex: Range<usize>,
    placeholder_bytes: usize,
}

impl PreparedSignature {
    pub fn prepared_pdf(&self) -> &[u8] {
        &self.pdf
    }

    pub fn byte_range(&self) -> &super::ByteRange {
        &self.byte_range
    }

    /// Concatenate the exact bytes that an external signer must digest.
    pub fn bytes_to_digest(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(self.byte_range.total_bytes() as usize);
        for &(offset, length) in self.byte_range.ranges() {
            let start = offset as usize;
            bytes.extend_from_slice(&self.pdf[start..start + length as usize]);
        }
        bytes
    }

    /// Embed a caller-produced DER CMS container without rewriting prior bytes.
    pub fn finalize(mut self, cms: &[u8]) -> SignatureResult<Vec<u8>> {
        validate_cms_container(cms)?;
        if cms.len() > self.placeholder_bytes {
            return Err(SignatureError::ContentsExtractionFailed {
                details: format!(
                    "CMS container needs {} bytes but the placeholder reserves {}",
                    cms.len(),
                    self.placeholder_bytes
                ),
            });
        }
        let slot = &mut self.pdf[self.contents_hex.clone()];
        slot.fill(b'0');
        for (index, byte) in cms.iter().enumerate() {
            let encoded = format!("{byte:02X}");
            slot[index * 2..index * 2 + 2].copy_from_slice(encoded.as_bytes());
        }
        Ok(self.pdf)
    }
}

/// Prepare an incremental revision for an external CMS signer.
pub fn prepare_incremental_signature(
    base: &[u8],
    options: &SignaturePreparationOptions,
) -> SignatureResult<PreparedSignature> {
    prepare_incremental_signature_inner(base, options, None)
}

/// Prepare an incremental revision with generated visible signature artwork.
///
/// A visible target rectangle is required. The artwork, including its font and
/// optional image resource, is part of the prepared revision and is therefore
/// covered by the caller-produced CMS signature.
pub fn prepare_incremental_signature_with_appearance(
    base: &[u8],
    options: &SignaturePreparationOptions,
    appearance: &SignatureAppearance,
) -> SignatureResult<PreparedSignature> {
    prepare_incremental_signature_inner(base, options, Some(appearance))
}

fn prepare_incremental_signature_inner(
    base: &[u8],
    options: &SignaturePreparationOptions,
    appearance: Option<&SignatureAppearance>,
) -> SignatureResult<PreparedSignature> {
    validate_options(options, appearance)?;
    let mut reader =
        PdfReader::new(Cursor::new(base)).map_err(|error| invalid(error.to_string()))?;
    if reader.is_encrypted() {
        return Err(invalid(
            "incremental signing does not support encrypted PDFs",
        ));
    }
    let mut catalog = reader
        .catalog()
        .map_err(|error| invalid(error.to_string()))?
        .clone();
    ensure_modification_allowed(&mut reader, &catalog, IncrementalModification::AddSignature)?;
    let target = &options.target;
    let target_field_name = match target {
        SignatureTarget::Existing { field_name, .. } | SignatureTarget::New { field_name, .. } => {
            field_name
        }
    };
    ensure_fieldmdp_allows_signature(&mut reader, target_field_name)?;
    let mut update = IncrementalUpdate::from_base(base)?;
    let signature_id = update.allocate_id()?;
    let mut signature = signature_dictionary(options);
    signature.insert(
        "ByteRange".to_string(),
        PdfObject::Array(PdfArray(vec![
            PdfObject::Integer(0),
            PdfObject::Integer(BYTE_RANGE_SENTINEL),
            PdfObject::Integer(BYTE_RANGE_SENTINEL),
            PdfObject::Integer(BYTE_RANGE_SENTINEL),
        ])),
    );
    signature.insert(
        "Contents".to_string(),
        PdfObject::String(PdfString::new(vec![0xA5; options.placeholder_bytes])),
    );
    update.replace(signature_id, PdfObject::Dictionary(signature))?;

    match target {
        SignatureTarget::Existing {
            field_name,
            widget_index,
            rect,
        } => {
            let field = find_signature_field(&mut reader, &catalog, field_name)?;
            let mut dictionary = reader
                .get_object(field.0, field.1)
                .map_err(|error| invalid(format!("resolve signature field: {error}")))?
                .as_dict()
                .cloned()
                .ok_or_else(|| invalid("signature field is not a dictionary"))?;
            if dictionary.contains_key("V") {
                return Err(invalid("selected signature field is already signed"));
            }
            dictionary.insert("V".to_string(), reference(signature_id));
            if rect.is_some() {
                let widget = select_widget(&mut reader, field, *widget_index)?;
                let appearance_id = update.allocate_id()?;
                let rect = (*rect).ok_or_else(|| invalid("visible widget requires a rectangle"))?;
                let watermark_id = appearance
                    .and_then(|appearance| appearance.watermark.as_ref())
                    .map(|_| update.allocate_id())
                    .transpose()?;
                let mut appearance_entry = PdfDictionary::new();
                appearance_entry.insert("N".to_string(), reference(appearance_id));
                if widget == field {
                    // A combined field/widget is one indirect object.  Its value and
                    // appearance must be emitted as one incremental replacement.
                    dictionary.insert("Rect".to_string(), rect.object());
                    dictionary.insert("AP".to_string(), PdfObject::Dictionary(appearance_entry));
                    update.replace(field, PdfObject::Dictionary(dictionary))?;
                } else {
                    update.replace(field, PdfObject::Dictionary(dictionary))?;
                    let mut widget_dictionary = reader
                        .get_object(widget.0, widget.1)
                        .map_err(|error| invalid(format!("resolve signature widget: {error}")))?
                        .as_dict()
                        .cloned()
                        .ok_or_else(|| invalid("signature widget is not a dictionary"))?;
                    widget_dictionary.insert("Rect".to_string(), rect.object());
                    widget_dictionary
                        .insert("AP".to_string(), PdfObject::Dictionary(appearance_entry));
                    update.replace(widget, PdfObject::Dictionary(widget_dictionary))?;
                }
                if let (Some(watermark), Some(watermark_id)) = (
                    appearance.and_then(|appearance| appearance.watermark.as_ref()),
                    watermark_id,
                ) {
                    update.replace(watermark_id, watermark_object(watermark))?;
                }
                update.replace(
                    appearance_id,
                    visible_appearance(rect, appearance, watermark_id),
                )?;
            } else if widget_index.is_some() {
                select_widget(&mut reader, field, *widget_index)?;
                update.replace(field, PdfObject::Dictionary(dictionary))?;
            } else {
                update.replace(field, PdfObject::Dictionary(dictionary))?;
            }
        }
        SignatureTarget::New {
            field_name,
            page_index,
            rect,
        } => {
            ensure_field_name_available(&mut reader, &catalog, field_name)?;
            let page_reader = PdfReader::new(Cursor::new(base))
                .map_err(|error| invalid(format!("open signature page snapshot: {error}")))?;
            let page_document = PdfDocument::new(page_reader);
            let page = page_document
                .get_page(*page_index as u32)
                .map_err(|error| invalid(format!("select signature page: {error}")))?;
            let field_id = update.allocate_id()?;
            let appearance_id = rect.map(|_| update.allocate_id()).transpose()?;
            let watermark_id = if rect.is_some() {
                appearance
                    .and_then(|appearance| appearance.watermark.as_ref())
                    .map(|_| update.allocate_id())
                    .transpose()?
            } else {
                None
            };
            let mut field = PdfDictionary::new();
            field.insert("Type".to_string(), name("Annot"));
            field.insert("Subtype".to_string(), name("Widget"));
            field.insert("FT".to_string(), name("Sig"));
            field.insert("T".to_string(), text(field_name));
            field.insert("F".to_string(), PdfObject::Integer(4));
            field.insert("P".to_string(), reference(page.obj_ref));
            field.insert("V".to_string(), reference(signature_id));
            field.insert(
                "Rect".to_string(),
                rect.map(SignatureRect::object)
                    .unwrap_or_else(|| PdfObject::Array(PdfArray(vec![PdfObject::Integer(0); 4]))),
            );
            if let (Some(rect), Some(appearance_id)) = (rect, appearance_id) {
                field.insert("AP".to_string(), {
                    let mut ap = PdfDictionary::new();
                    ap.insert("N".to_string(), reference(appearance_id));
                    PdfObject::Dictionary(ap)
                });
                if let (Some(watermark), Some(watermark_id)) = (
                    appearance.and_then(|appearance| appearance.watermark.as_ref()),
                    watermark_id,
                ) {
                    update.replace(watermark_id, watermark_object(watermark))?;
                }
                update.replace(
                    appearance_id,
                    visible_appearance(*rect, appearance, watermark_id),
                )?;
            }
            update.replace(field_id, PdfObject::Dictionary(field))?;
            append_page_annotation(&mut reader, &mut update, page.obj_ref, &page.dict, field_id)?;
            append_acroform_field(&mut reader, &mut update, &mut catalog, field_id)?;
        }
    }

    if options.certification.is_some() {
        if catalog.contains_key("Perms") {
            return Err(invalid(
                "catalog already contains certification permissions",
            ));
        }
        let mut perms = PdfDictionary::new();
        perms.insert("DocMDP".to_string(), reference(signature_id));
        catalog.insert("Perms".to_string(), PdfObject::Dictionary(perms));
    }
    let root = reader
        .trailer()
        .root()
        .map_err(|error| invalid(error.to_string()))?;
    update.replace(root, PdfObject::Dictionary(catalog))?;
    let prepared = update.finish()?;
    locate_and_patch_placeholders(prepared, base.len(), options.placeholder_bytes)
}

fn validate_options(
    options: &SignaturePreparationOptions,
    appearance: Option<&SignatureAppearance>,
) -> SignatureResult<()> {
    if !(MIN_PLACEHOLDER_BYTES..=MAX_PLACEHOLDER_BYTES).contains(&options.placeholder_bytes) {
        return Err(invalid(format!(
            "placeholder_bytes must be between {MIN_PLACEHOLDER_BYTES} and {MAX_PLACEHOLDER_BYTES}"
        )));
    }
    let name = match &options.target {
        SignatureTarget::Existing { field_name, .. } | SignatureTarget::New { field_name, .. } => {
            field_name
        }
    };
    if name.is_empty() || name.as_bytes().contains(&0) {
        return Err(invalid("signature field name is empty or contains NUL"));
    }
    validate_pdf_name(&options.filter, "filter")?;
    validate_pdf_name(&options.sub_filter, "sub_filter")?;
    const RESERVED: [&str; 7] = [
        "Type",
        "Filter",
        "SubFilter",
        "ByteRange",
        "Contents",
        "Reference",
        "M",
    ];
    if let Some(key) = RESERVED
        .iter()
        .find(|key| options.additional_signature_entries.contains_key(**key))
    {
        return Err(invalid(format!(
            "additional signature entry {key:?} is owned by the signing engine"
        )));
    }
    let rect = match &options.target {
        SignatureTarget::Existing { rect, .. } | SignatureTarget::New { rect, .. } => rect,
    };
    if let Some(rect) = rect {
        rect.validate()?;
    }
    if let Some(appearance) = appearance {
        if rect.is_none() {
            return Err(invalid(
                "a custom signature appearance requires a visible target rectangle",
            ));
        }
        for value in appearance
            .signer_name
            .iter()
            .chain(appearance.signing_date.iter())
            .chain(appearance.text.iter())
        {
            if value.is_empty() || TextEncoding::WinAnsiEncoding.encode_strict(value).is_err() {
                return Err(invalid(
                    "signature appearance text must be non-empty and representable in WinAnsiEncoding",
                ));
            }
        }
        if let Some(watermark) = &appearance.watermark {
            let pixels = (watermark.width as usize)
                .checked_mul(watermark.height as usize)
                .ok_or_else(|| invalid("signature watermark dimensions overflow"))?;
            let bytes = pixels
                .checked_mul(3)
                .ok_or_else(|| invalid("signature watermark byte count overflows"))?;
            if watermark.width == 0
                || watermark.height == 0
                || pixels > MAX_WATERMARK_PIXELS
                || watermark.rgb.len() != bytes
            {
                return Err(invalid(
                    "signature watermark must be a bounded non-empty RGB raster",
                ));
            }
            if !watermark.opacity.is_finite() || !(0.0..=1.0).contains(&watermark.opacity) {
                return Err(invalid(
                    "signature watermark opacity must be finite and between zero and one",
                ));
            }
        }
    }
    Ok(())
}

fn signature_dictionary(options: &SignaturePreparationOptions) -> PdfDictionary {
    let mut dictionary = PdfDictionary::new();
    dictionary.insert("Type".to_string(), name("Sig"));
    dictionary.insert("Filter".to_string(), name(&options.filter));
    dictionary.insert("SubFilter".to_string(), name(&options.sub_filter));
    dictionary
        .0
        .extend(options.additional_signature_entries.0.clone());
    for (key, value) in [
        ("Reason", &options.reason),
        ("Location", &options.location),
        ("ContactInfo", &options.contact_info),
        ("M", &options.signing_time),
    ] {
        if let Some(value) = value {
            dictionary.insert(key.to_string(), text(value));
        }
    }
    let mut transforms = Vec::new();
    if let Some(permission) = options.certification {
        let mut params = PdfDictionary::new();
        params.insert("Type".to_string(), name("TransformParams"));
        params.insert("P".to_string(), PdfObject::Integer(permission as i64));
        params.insert("V".to_string(), name("1.2"));
        transforms.push(transform("DocMDP", params));
    }
    if let Some(lock) = &options.field_lock {
        let mut params = PdfDictionary::new();
        params.insert("Type".to_string(), name("TransformParams"));
        params.insert("V".to_string(), name("1.2"));
        match lock {
            FieldLock::All => {
                params.insert("Action".to_string(), name("All"));
            }
            FieldLock::Include(fields) | FieldLock::Exclude(fields) => {
                params.insert(
                    "Action".to_string(),
                    name(if matches!(lock, FieldLock::Include(_)) {
                        "Include"
                    } else {
                        "Exclude"
                    }),
                );
                params.insert(
                    "Fields".to_string(),
                    PdfObject::Array(PdfArray(fields.iter().map(|field| text(field)).collect())),
                );
            }
        }
        transforms.push(transform("FieldMDP", params));
    }
    if !transforms.is_empty() {
        dictionary.insert(
            "Reference".to_string(),
            PdfObject::Array(PdfArray(transforms)),
        );
    }
    dictionary
}

fn transform(method: &str, params: PdfDictionary) -> PdfObject {
    let mut dictionary = PdfDictionary::new();
    dictionary.insert("Type".to_string(), name("SigRef"));
    dictionary.insert("TransformMethod".to_string(), name(method));
    dictionary.insert("TransformParams".to_string(), PdfObject::Dictionary(params));
    PdfObject::Dictionary(dictionary)
}

pub(super) fn append_page_annotation(
    reader: &mut PdfReader<Cursor<&[u8]>>,
    update: &mut IncrementalUpdate<'_>,
    page_id: (u32, u16),
    original: &PdfDictionary,
    field_id: (u32, u16),
) -> SignatureResult<()> {
    let mut page = original.clone();
    let mut annotations = match page.get("Annots") {
        None => Vec::new(),
        Some(PdfObject::Array(array)) => array.0.clone(),
        Some(PdfObject::Reference(n, g)) => reader
            .get_object(*n, *g)
            .map_err(|error| invalid(error.to_string()))?
            .as_array()
            .ok_or_else(|| invalid("page /Annots reference is not an array"))?
            .0
            .clone(),
        _ => return Err(invalid("page /Annots is not an array")),
    };
    annotations.push(reference(field_id));
    page.insert(
        "Annots".to_string(),
        PdfObject::Array(PdfArray(annotations)),
    );
    update.replace(page_id, PdfObject::Dictionary(page))?;
    Ok(())
}

pub(super) fn append_acroform_field(
    reader: &mut PdfReader<Cursor<&[u8]>>,
    update: &mut IncrementalUpdate<'_>,
    catalog: &mut PdfDictionary,
    field_id: (u32, u16),
) -> SignatureResult<()> {
    let mut form = match catalog.get("AcroForm") {
        None => PdfDictionary::new(),
        Some(PdfObject::Dictionary(dictionary)) => dictionary.clone(),
        Some(PdfObject::Reference(n, g)) => {
            let id = (*n, *g);
            let mut dictionary = reader
                .get_object(*n, *g)
                .map_err(|error| invalid(error.to_string()))?
                .as_dict()
                .cloned()
                .ok_or_else(|| invalid("/AcroForm reference is not a dictionary"))?;
            append_field_array(reader, &mut dictionary, field_id)?;
            update.replace(id, PdfObject::Dictionary(dictionary))?;
            return Ok(());
        }
        _ => return Err(invalid("catalog /AcroForm is not a dictionary")),
    };
    append_field_array(reader, &mut form, field_id)?;
    form.insert("SigFlags".to_string(), PdfObject::Integer(3));
    catalog.insert("AcroForm".to_string(), PdfObject::Dictionary(form));
    Ok(())
}

fn append_field_array(
    reader: &mut PdfReader<Cursor<&[u8]>>,
    form: &mut PdfDictionary,
    field_id: (u32, u16),
) -> SignatureResult<()> {
    let mut fields = match form.get("Fields") {
        None => Vec::new(),
        Some(PdfObject::Array(array)) => array.0.clone(),
        Some(PdfObject::Reference(n, g)) => reader
            .get_object(*n, *g)
            .map_err(|error| invalid(error.to_string()))?
            .as_array()
            .ok_or_else(|| invalid("/AcroForm /Fields reference is not an array"))?
            .0
            .clone(),
        _ => return Err(invalid("/AcroForm /Fields is not an array")),
    };
    fields.push(reference(field_id));
    form.insert("Fields".to_string(), PdfObject::Array(PdfArray(fields)));
    form.insert("SigFlags".to_string(), PdfObject::Integer(3));
    Ok(())
}

pub(super) fn root_field_references(
    reader: &mut PdfReader<Cursor<&[u8]>>,
    catalog: &PdfDictionary,
) -> SignatureResult<Vec<(u32, u16)>> {
    let Some(form) = catalog.get("AcroForm") else {
        return Ok(Vec::new());
    };
    let form = match form {
        PdfObject::Dictionary(d) => d.clone(),
        PdfObject::Reference(n, g) => reader
            .get_object(*n, *g)
            .map_err(|e| invalid(e.to_string()))?
            .as_dict()
            .cloned()
            .ok_or_else(|| invalid("/AcroForm is not a dictionary"))?,
        _ => return Err(invalid("/AcroForm is not a dictionary")),
    };
    let Some(fields) = form.get("Fields") else {
        return Ok(Vec::new());
    };
    Ok(resolve_array(reader, fields, "/AcroForm /Fields")?
        .iter()
        .map(|field| {
            field
                .as_reference()
                .ok_or_else(|| invalid("/AcroForm /Fields entries must be indirect references"))
        })
        .collect::<SignatureResult<Vec<_>>>()?)
}

pub(super) fn ensure_field_name_available(
    reader: &mut PdfReader<Cursor<&[u8]>>,
    catalog: &PdfDictionary,
    name: &str,
) -> SignatureResult<()> {
    if find_named_field(reader, catalog, name)?.is_some() {
        return Err(invalid(format!("field name {name:?} already exists")));
    }
    Ok(())
}

pub(super) fn find_signature_field(
    reader: &mut PdfReader<Cursor<&[u8]>>,
    catalog: &PdfDictionary,
    name: &str,
) -> SignatureResult<(u32, u16)> {
    let field = find_named_field(reader, catalog, name)?
        .ok_or_else(|| invalid(format!("signature field {name:?} was not found")))?;
    if field.field_type.as_deref() != Some("Sig") {
        return Err(invalid("selected field is not a signature field"));
    }
    Ok(field.id)
}

fn select_widget(
    reader: &mut PdfReader<Cursor<&[u8]>>,
    field: (u32, u16),
    widget_index: Option<usize>,
) -> SignatureResult<(u32, u16)> {
    let dictionary = reader
        .get_object(field.0, field.1)
        .map_err(|error| invalid(format!("resolve signature field widgets: {error}")))?
        .as_dict()
        .cloned()
        .ok_or_else(|| invalid("signature field is not a dictionary"))?;
    let Some(index) = widget_index else {
        if dictionary
            .get("Subtype")
            .and_then(PdfObject::as_name)
            .is_some_and(|subtype| subtype.0 == "Widget")
        {
            return Ok(field);
        }
        return Err(invalid(
            "existing field is not a combined widget; specify widget_index",
        ));
    };
    let kids = dictionary
        .get("Kids")
        .ok_or_else(|| invalid("existing field has no child widgets"))?;
    let kids = resolve_array(reader, kids, "signature field /Kids")?;
    let widget = kids
        .get(index)
        .and_then(PdfObject::as_reference)
        .ok_or_else(|| invalid(format!("signature widget index {index} is out of bounds")))?;
    let widget_dictionary = reader
        .get_object(widget.0, widget.1)
        .map_err(|error| invalid(format!("resolve signature widget: {error}")))?
        .as_dict()
        .ok_or_else(|| invalid("signature widget is not a dictionary"))?;
    if widget_dictionary
        .get("Subtype")
        .and_then(PdfObject::as_name)
        .is_none_or(|subtype| subtype.0 != "Widget")
    {
        return Err(invalid("selected child is not a widget annotation"));
    }
    Ok(widget)
}

#[derive(Clone, Debug)]
struct NamedField {
    id: (u32, u16),
    field_type: Option<String>,
}

fn find_named_field(
    reader: &mut PdfReader<Cursor<&[u8]>>,
    catalog: &PdfDictionary,
    wanted: &str,
) -> SignatureResult<Option<NamedField>> {
    let mut stack: Vec<_> = root_field_references(reader, catalog)?
        .into_iter()
        .map(|id| (id, String::new(), None::<String>))
        .collect();
    let mut visited = HashSet::new();
    let mut found = None;
    while let Some((id, parent_name, inherited_type)) = stack.pop() {
        if !visited.insert(id) || visited.len() > 100_000 {
            continue;
        }
        let dictionary = reader
            .get_object(id.0, id.1)
            .map_err(|e| invalid(e.to_string()))?
            .as_dict()
            .cloned()
            .ok_or_else(|| invalid("field tree node is not a dictionary"))?;
        let partial_name = dictionary
            .get("T")
            .and_then(PdfObject::as_string)
            .map(|value| value.to_text());
        let qualified_name = match (parent_name.as_str(), partial_name.as_deref()) {
            ("", Some(partial)) => partial.to_string(),
            (_, Some(partial)) => format!("{parent_name}.{partial}"),
            (_, None) => parent_name.clone(),
        };
        let field_type = dictionary
            .get("FT")
            .and_then(PdfObject::as_name)
            .map(|value| value.0.clone())
            .or(inherited_type);
        if partial_name.is_some() && qualified_name == wanted {
            if found
                .replace(NamedField {
                    id,
                    field_type: field_type.clone(),
                })
                .is_some()
            {
                return Err(invalid(format!("field identity {wanted:?} is ambiguous")));
            }
        }
        if let Some(kids) = dictionary.get("Kids") {
            for kid in resolve_array(reader, kids, "field /Kids")? {
                let kid = kid
                    .as_reference()
                    .ok_or_else(|| invalid("field /Kids entries must be indirect references"))?;
                stack.push((kid, qualified_name.clone(), field_type.clone()));
            }
        }
    }
    Ok(found)
}

pub(super) fn ensure_fieldmdp_allows_signature(
    reader: &mut PdfReader<Cursor<&[u8]>>,
    target: &str,
) -> SignatureResult<()> {
    for id in reader.object_references() {
        // Match DocMDP discovery: an unreferenced classic xref entry at byte
        // zero is not an object and cannot establish a FieldMDP policy.
        if reader.object_storage_offset(id.0) == Some(0) {
            continue;
        }
        let object = reader
            .get_object(id.0, id.1)
            .map_err(|error| invalid(format!("inspect FieldMDP signature: {error}")))?
            .clone();
        let Some(signature) = object.as_dict() else {
            continue;
        };
        if signature.get_type() != Some("Sig")
            || !signature.contains_key("ByteRange")
            || !signature.contains_key("Contents")
        {
            continue;
        }
        ensure_signature_definition_is_covered(reader, id, signature)?;
        let Some(references) = signature.get("Reference") else {
            continue;
        };
        for transform in resolve_array(reader, references, "signature /Reference")? {
            let transform = resolve_dictionary(reader, &transform, "FieldMDP transform")?;
            if transform
                .get("TransformMethod")
                .and_then(PdfObject::as_name)
                .is_none_or(|method| method.0 != "FieldMDP")
            {
                continue;
            }
            let params = transform
                .get("TransformParams")
                .ok_or_else(|| invalid("FieldMDP transform has no /TransformParams"))?;
            let params = resolve_dictionary(reader, params, "FieldMDP /TransformParams")?;
            let action = params
                .get("Action")
                .and_then(PdfObject::as_name)
                .ok_or_else(|| invalid("FieldMDP /TransformParams requires /Action"))?;
            let fields = match params.get("Fields") {
                None => Vec::new(),
                Some(value) => resolve_array(reader, value, "FieldMDP /Fields")?
                    .into_iter()
                    .map(|field| {
                        field
                            .as_string()
                            .map(|value| value.to_text())
                            .ok_or_else(|| invalid("FieldMDP /Fields must contain strings"))
                    })
                    .collect::<SignatureResult<Vec<_>>>()?,
            };
            let locked = match action.0.as_str() {
                "All" if fields.is_empty() => true,
                "Include" => fields.iter().any(|field| field == target),
                "Exclude" => !fields.iter().any(|field| field == target),
                "All" => return Err(invalid("FieldMDP /All must not specify /Fields")),
                other => return Err(invalid(format!("unsupported FieldMDP action /{other}"))),
            };
            if locked {
                return Err(invalid(format!(
                    "FieldMDP locks signature field {target:?}"
                )));
            }
        }
    }
    Ok(())
}

fn ensure_signature_definition_is_covered(
    reader: &PdfReader<Cursor<&[u8]>>,
    id: (u32, u16),
    signature: &PdfDictionary,
) -> SignatureResult<()> {
    let values = signature
        .get("ByteRange")
        .and_then(PdfObject::as_array)
        .ok_or_else(|| invalid("FieldMDP signature requires an array /ByteRange"))?
        .0
        .iter()
        .map(|value| {
            value
                .as_integer()
                .ok_or_else(|| invalid("FieldMDP /ByteRange must contain integers"))
        })
        .collect::<SignatureResult<Vec<_>>>()?;
    let byte_range = super::ByteRange::from_array(&values)
        .map_err(|error| invalid(format!("invalid FieldMDP /ByteRange: {error}")))?;
    byte_range
        .validate()
        .map_err(|error| invalid(format!("invalid FieldMDP /ByteRange: {error}")))?;
    let offset = reader
        .object_storage_offset(id.0)
        .ok_or_else(|| invalid("cannot locate FieldMDP signature definition"))?;
    if !byte_range.ranges().iter().any(|(start, length)| {
        start
            .checked_add(*length)
            .is_some_and(|end| offset >= *start && offset < end)
    }) {
        return Err(invalid(
            "latest FieldMDP signature definition is outside its signed byte ranges",
        ));
    }
    Ok(())
}

fn resolve_dictionary(
    reader: &mut PdfReader<Cursor<&[u8]>>,
    value: &PdfObject,
    role: &str,
) -> SignatureResult<PdfDictionary> {
    let value = resolve_object(reader, value, role)?;
    value
        .as_dict()
        .cloned()
        .ok_or_else(|| invalid(format!("{role} is not a dictionary")))
}

fn resolve_array(
    reader: &mut PdfReader<Cursor<&[u8]>>,
    value: &PdfObject,
    role: &str,
) -> SignatureResult<Vec<PdfObject>> {
    let value = resolve_object(reader, value, role)?;
    value
        .as_array()
        .map(|array| array.0.clone())
        .ok_or_else(|| invalid(format!("{role} is not an array")))
}

fn resolve_object(
    reader: &mut PdfReader<Cursor<&[u8]>>,
    value: &PdfObject,
    role: &str,
) -> SignatureResult<PdfObject> {
    match value {
        PdfObject::Reference(number, generation) => reader
            .get_object(*number, *generation)
            .cloned()
            .map_err(|error| invalid(format!("resolve {role}: {error}"))),
        value => Ok(value.clone()),
    }
}

fn visible_appearance(
    rect: SignatureRect,
    appearance: Option<&SignatureAppearance>,
    watermark_id: Option<(u32, u16)>,
) -> PdfObject {
    let width = rect.right - rect.left;
    let height = rect.top - rect.bottom;
    let mut data = format!("q 0 0 {width} {height} re S Q\n");
    if let Some(appearance) = appearance {
        if let (Some(watermark), Some(_)) = (&appearance.watermark, watermark_id) {
            let image_width = width * 0.36;
            let image_height = height * 0.76;
            let scale =
                (image_width / watermark.width as f64).min(image_height / watermark.height as f64);
            let draw_width = watermark.width as f64 * scale;
            let draw_height = watermark.height as f64 * scale;
            let x = width - draw_width - width.min(height) * 0.08;
            let y = (height - draw_height) / 2.0;
            data.push_str(&format!(
                "q /GS1 gs {draw_width} 0 0 {draw_height} {x} {y} cm /Im0 Do Q\n"
            ));
        }
        let text_width = if appearance.watermark.is_some() {
            width * 0.58
        } else {
            width
        };
        let margin = (width.min(height) * 0.08).clamp(3.0, 10.0);
        // Text remains inside the widget even if a caller supplies more lines
        // than its geometry can show.
        data.push_str(&format!("q 0 0 {text_width} {height} re W n\n"));
        let mut lines = Vec::new();
        if let Some(signer) = &appearance.signer_name {
            lines.push(signer.as_str());
        }
        if let Some(date) = &appearance.signing_date {
            lines.push(date.as_str());
        }
        lines.extend(appearance.text.iter().map(String::as_str));
        if !lines.is_empty() {
            let font_size = ((height - 2.0 * margin) / (lines.len() as f64 + 0.5)).clamp(6.0, 12.0);
            let mut y = height - margin - font_size;
            for line in lines {
                if y < margin {
                    break;
                }
                data.push_str(&format!(
                    "BT /F1 {font_size} Tf 0 g {margin} {y} Td ({}) Tj ET\n",
                    appearance_text_literal(line)
                ));
                y -= font_size * 1.25;
            }
        }
        data.push_str("Q\n");
    }
    let mut dictionary = PdfDictionary::new();
    dictionary.insert("Type".to_string(), name("XObject"));
    dictionary.insert("Subtype".to_string(), name("Form"));
    dictionary.insert(
        "BBox".to_string(),
        PdfObject::Array(PdfArray(vec![
            PdfObject::Integer(0),
            PdfObject::Integer(0),
            PdfObject::Real(width),
            PdfObject::Real(height),
        ])),
    );
    let mut resources = PdfDictionary::new();
    if appearance.is_some() {
        let mut font = PdfDictionary::new();
        font.insert("Type".to_string(), name("Font"));
        font.insert("Subtype".to_string(), name("Type1"));
        font.insert("BaseFont".to_string(), name("Helvetica"));
        font.insert("Encoding".to_string(), name("WinAnsiEncoding"));
        let mut fonts = PdfDictionary::new();
        fonts.insert("F1".to_string(), PdfObject::Dictionary(font));
        resources.insert("Font".to_string(), PdfObject::Dictionary(fonts));
    }
    if let (Some(appearance), Some(watermark_id)) = (appearance, watermark_id) {
        if appearance.watermark.is_some() {
            let mut xobjects = PdfDictionary::new();
            xobjects.insert("Im0".to_string(), reference(watermark_id));
            resources.insert("XObject".to_string(), PdfObject::Dictionary(xobjects));
            let mut state = PdfDictionary::new();
            let opacity = appearance.watermark.as_ref().unwrap().opacity as f64;
            state.insert("Type".to_string(), name("ExtGState"));
            state.insert("ca".to_string(), PdfObject::Real(opacity));
            state.insert("CA".to_string(), PdfObject::Real(opacity));
            let mut states = PdfDictionary::new();
            states.insert("GS1".to_string(), PdfObject::Dictionary(state));
            resources.insert("ExtGState".to_string(), PdfObject::Dictionary(states));
        }
    }
    dictionary.insert("Resources".to_string(), PdfObject::Dictionary(resources));
    PdfObject::Stream(PdfStream {
        dict: dictionary,
        data: data.into_bytes(),
    })
}

fn watermark_object(watermark: &SignatureWatermark) -> PdfObject {
    let mut dictionary = PdfDictionary::new();
    dictionary.insert("Type".to_string(), name("XObject"));
    dictionary.insert("Subtype".to_string(), name("Image"));
    dictionary.insert(
        "Width".to_string(),
        PdfObject::Integer(watermark.width as i64),
    );
    dictionary.insert(
        "Height".to_string(),
        PdfObject::Integer(watermark.height as i64),
    );
    dictionary.insert("ColorSpace".to_string(), name("DeviceRGB"));
    dictionary.insert("BitsPerComponent".to_string(), PdfObject::Integer(8));
    PdfObject::Stream(PdfStream {
        dict: dictionary,
        data: watermark.rgb.clone(),
    })
}

fn appearance_text_literal(value: &str) -> String {
    let encoded = TextEncoding::WinAnsiEncoding
        .encode_strict(value)
        .expect("appearance text was validated before rendering");
    escape_pdf_string_literal(&encoded)
}

fn locate_and_patch_placeholders(
    mut pdf: Vec<u8>,
    base_len: usize,
    placeholder_bytes: usize,
) -> SignatureResult<PreparedSignature> {
    let marker = format!("<{}>", "A5".repeat(placeholder_bytes));
    let appended = pdf
        .get(base_len..)
        .ok_or_else(|| invalid("incremental revision is shorter than its source"))?;
    let contents_start =
        base_len + find_unique(appended, marker.as_bytes(), "/Contents placeholder")?;
    let contents_end = contents_start + marker.len();
    let ranges = [
        0u64,
        contents_start as u64,
        contents_end as u64,
        (pdf.len() - contents_end) as u64,
    ];
    let sentinel = BYTE_RANGE_SENTINEL.to_string();
    let locations: Vec<_> = find_all(&pdf[base_len..], sentinel.as_bytes())
        .into_iter()
        .map(|location| base_len + location)
        .collect();
    if locations.len() != 3 {
        return Err(invalid(
            "could not locate the three ByteRange placeholders uniquely",
        ));
    }
    for (location, value) in locations.into_iter().zip(ranges[1..].iter()) {
        let encoded = format!("{value:0BYTE_RANGE_WIDTH$}");
        if encoded.len() != BYTE_RANGE_WIDTH {
            return Err(invalid("PDF offsets exceed the reserved ByteRange width"));
        }
        pdf[location..location + BYTE_RANGE_WIDTH].copy_from_slice(encoded.as_bytes());
    }
    Ok(PreparedSignature {
        pdf,
        byte_range: super::ByteRange::new(vec![(0, ranges[1]), (ranges[2], ranges[3])]),
        contents_hex: contents_start + 1..contents_end - 1,
        placeholder_bytes,
    })
}

fn validate_cms_container(cms: &[u8]) -> SignatureResult<()> {
    if cms.len() < 2 || cms[0] != 0x30 {
        return Err(SignatureError::CmsParsingFailed {
            details: "CMS must be a non-empty DER SEQUENCE".to_string(),
        });
    }
    let (header, content_len) = if cms[1] & 0x80 == 0 {
        (2, cms[1] as usize)
    } else {
        let count = (cms[1] & 0x7f) as usize;
        if count == 0 || count > 8 || cms.len() < 2 + count {
            return Err(SignatureError::CmsParsingFailed {
                details: "invalid DER length".to_string(),
            });
        }
        let mut length = 0usize;
        for byte in &cms[2..2 + count] {
            length = length
                .checked_mul(256)
                .and_then(|v| v.checked_add(*byte as usize))
                .ok_or_else(|| SignatureError::CmsParsingFailed {
                    details: "DER length overflow".to_string(),
                })?;
        }
        (2 + count, length)
    };
    if header.checked_add(content_len) != Some(cms.len()) {
        return Err(SignatureError::CmsParsingFailed {
            details: "DER sequence length does not match CMS size".to_string(),
        });
    }
    Ok(())
}

fn find_unique(haystack: &[u8], needle: &[u8], role: &str) -> SignatureResult<usize> {
    let matches = find_all(haystack, needle);
    if matches.len() != 1 {
        return Err(invalid(format!("{role} occurs {} times", matches.len())));
    }
    Ok(matches[0])
}
fn find_all(haystack: &[u8], needle: &[u8]) -> Vec<usize> {
    haystack
        .windows(needle.len())
        .enumerate()
        .filter_map(|(index, value)| (value == needle).then_some(index))
        .collect()
}
pub(super) fn reference(id: (u32, u16)) -> PdfObject {
    PdfObject::Reference(id.0, id.1)
}
pub(super) fn name(value: &str) -> PdfObject {
    PdfObject::Name(PdfName::new(value.to_string()))
}
pub(super) fn text(value: &str) -> PdfObject {
    PdfObject::String(PdfString::new(value.as_bytes().to_vec()))
}
pub(super) fn invalid(details: impl Into<String>) -> SignatureError {
    SignatureError::InvalidSignatureDict {
        details: details.into(),
    }
}

fn validate_pdf_name(value: &str, role: &str) -> SignatureResult<()> {
    if value.is_empty()
        || value.bytes().any(|byte| {
            byte.is_ascii_whitespace()
                || matches!(
                    byte,
                    b'(' | b')' | b'<' | b'>' | b'[' | b']' | b'{' | b'}' | b'/' | b'%' | b'#'
                )
        })
    {
        return Err(invalid(format!("{role} is not a safe PDF name")));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pdf_with_unused_zero_offset_entry(in_use: bool, docmdp_points_to_entry: bool) -> Vec<u8> {
        let catalog = if docmdp_points_to_entry {
            "<< /Type /Catalog /Pages 2 0 R /Perms << /DocMDP 4 0 R >> >>"
        } else {
            "<< /Type /Catalog /Pages 2 0 R >>"
        };
        let mut pdf = b"%PDF-1.7\n".to_vec();
        let mut offsets = Vec::new();
        for (index, body) in [
            catalog,
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Resources << >> >>",
        ]
        .iter()
        .enumerate()
        {
            offsets.push(pdf.len());
            pdf.extend_from_slice(format!("{} 0 obj\n{}\nendobj\n", index + 1, body).as_bytes());
        }
        let xref = pdf.len();
        pdf.extend_from_slice(b"xref\n0 5\n0000000000 65535 f \n");
        for offset in offsets {
            pdf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
        }
        pdf.extend_from_slice(if in_use {
            b"0000000000 00000 n \n"
        } else {
            b"0000000000 00000 f \n"
        });
        pdf.extend_from_slice(
            format!("trailer\n<< /Size 5 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n").as_bytes(),
        );
        pdf
    }

    #[test]
    fn incremental_signature_ignores_unreferenced_in_use_zero_offset_entry() {
        let options = SignaturePreparationOptions::invisible("Signature");
        assert!(prepare_incremental_signature(
            &pdf_with_unused_zero_offset_entry(true, false),
            &options,
        )
        .is_ok());
    }

    #[test]
    fn incremental_signature_accepts_free_zero_offset_entry_control() {
        let options = SignaturePreparationOptions::invisible("Signature");
        assert!(prepare_incremental_signature(
            &pdf_with_unused_zero_offset_entry(false, false),
            &options,
        )
        .is_ok());
    }

    #[test]
    fn incremental_signature_rejects_docmdp_reference_to_zero_offset_entry() {
        let options = SignaturePreparationOptions::invisible("Signature");
        let error =
            prepare_incremental_signature(&pdf_with_unused_zero_offset_entry(true, true), &options)
                .expect_err("a DocMDP reference to a zero-offset entry must be rejected");
        assert!(error
            .to_string()
            .contains("resolve certification signature"));
    }
}
