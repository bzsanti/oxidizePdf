//! Bounded inspection and validation of tagged-PDF structure trees.
//!
//! This module intentionally starts with a read-only API.  A lossless editor
//! needs a trustworthy inventory of the existing structure and stable findings
//! before it can safely publish an incremental mutation.

use crate::error::{PdfError, Result};
use crate::parser::content::{
    ContentOperation, ContentParser, MarkedContentProps, MarkedContentValue,
};
use crate::parser::objects::{PdfDictionary, PdfObject};
use crate::parser::PdfReader;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::io::{Cursor, Read, Seek};

/// Hard limits for untrusted structure and number trees.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaggedPdfLimits {
    /// Maximum number of indirect structure/number-tree objects visited.
    pub max_objects: usize,
    /// Maximum nesting depth of `/K` and number-tree `/Kids` arrays.
    pub max_depth: usize,
    /// Maximum total children and number-tree entries inspected.
    pub max_entries: usize,
    /// Maximum total decoded page-content bytes inspected for MCIDs.
    pub max_decoded_content_bytes: usize,
}

impl Default for TaggedPdfLimits {
    fn default() -> Self {
        Self {
            max_objects: 100_000,
            max_depth: 256,
            max_entries: 1_000_000,
            max_decoded_content_bytes: 256 * 1024 * 1024,
        }
    }
}

/// Options for tagged-PDF inspection.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TaggedPdfValidationOptions {
    /// Resource limits applied to the input document.
    pub limits: TaggedPdfLimits,
}

/// Severity of a tagged-PDF finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TaggedPdfFindingSeverity {
    /// The structure is malformed or has a broken bidirectional association.
    Error,
    /// The structure is readable, but incomplete for robust PDF/UA processing.
    Warning,
}

/// Stable finding identifiers suitable for machine processing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TaggedPdfFindingCode {
    MissingStructureTree,
    InvalidStructureTreeRoot,
    InvalidStructureElement,
    DirectStructureElement,
    MissingStructureType,
    BrokenParentLink,
    DuplicateStructureReference,
    StructureCycle,
    InvalidMarkedContentReference,
    MissingMarkedContent,
    InvalidObjectReference,
    MissingParentTree,
    InvalidParentTree,
    MissingStructParents,
    MissingParentTreeEntry,
    ParentTreeOwnerMismatch,
    UnmappedCustomRole,
    InvalidRoleMap,
}

/// An indirect object reference exposed without leaking parser internals.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TaggedPdfObjectRef {
    pub object_number: u32,
    pub generation: u16,
}

impl From<(u32, u16)> for TaggedPdfObjectRef {
    fn from(value: (u32, u16)) -> Self {
        Self {
            object_number: value.0,
            generation: value.1,
        }
    }
}

/// One validation finding with a stable logical path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaggedPdfFinding {
    pub severity: TaggedPdfFindingSeverity,
    pub code: TaggedPdfFindingCode,
    pub path: String,
    pub message: String,
    pub object: Option<TaggedPdfObjectRef>,
}

/// Read-only summary of one indirect structure element.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaggedPdfStructureElement {
    pub object: TaggedPdfObjectRef,
    pub structure_type: Option<String>,
    pub parent: Option<TaggedPdfObjectRef>,
    pub child_count: usize,
    pub marked_content_count: usize,
}

/// Machine-readable result of tagged-PDF inspection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaggedPdfValidationReport {
    pub tagged: bool,
    pub valid: bool,
    pub structure_tree_root: Option<TaggedPdfObjectRef>,
    pub role_map: BTreeMap<String, String>,
    pub class_names: Vec<String>,
    pub elements: Vec<TaggedPdfStructureElement>,
    pub parent_tree_entries: usize,
    pub findings: Vec<TaggedPdfFinding>,
}

/// Inspect and validate the tagged structure of an existing PDF.
///
/// The traversal validates structure-element parent links and the page
/// `/StructParents` -> structure `/ParentTree` -> MCID owner association.  It
/// does not claim full PDF/UA conformance: language, headings, tables, lists,
/// links and alternate-text rules will build on this structural foundation.
pub fn validate_tagged_pdf(
    pdf: &[u8],
    options: &TaggedPdfValidationOptions,
) -> Result<TaggedPdfValidationReport> {
    let mut reader = PdfReader::new(Cursor::new(pdf))?;
    if reader.is_encrypted() && !reader.is_unlocked() {
        return Err(PdfError::EncryptionError(
            "tagged-PDF validation requires an unlocked document".to_string(),
        ));
    }
    Validator::new(&mut reader, &options.limits).run()
}

struct Validator<'a, R: Read + Seek> {
    reader: &'a mut PdfReader<R>,
    limits: &'a TaggedPdfLimits,
    visited: HashSet<TaggedPdfObjectRef>,
    active: HashSet<TaggedPdfObjectRef>,
    entry_count: usize,
    elements: Vec<TaggedPdfStructureElement>,
    findings: Vec<TaggedPdfFinding>,
    role_map: BTreeMap<String, String>,
    class_names: Vec<String>,
    parent_tree: BTreeMap<i64, PdfObject>,
    direct_mcid_counts: HashMap<TaggedPdfObjectRef, usize>,
    page_mcids: HashMap<TaggedPdfObjectRef, HashSet<i64>>,
    decoded_content_bytes: usize,
}

impl<'a, R: Read + Seek> Validator<'a, R> {
    fn new(reader: &'a mut PdfReader<R>, limits: &'a TaggedPdfLimits) -> Self {
        Self {
            reader,
            limits,
            visited: HashSet::new(),
            active: HashSet::new(),
            entry_count: 0,
            elements: Vec::new(),
            findings: Vec::new(),
            role_map: BTreeMap::new(),
            class_names: Vec::new(),
            parent_tree: BTreeMap::new(),
            direct_mcid_counts: HashMap::new(),
            page_mcids: HashMap::new(),
            decoded_content_bytes: 0,
        }
    }

    fn run(mut self) -> Result<TaggedPdfValidationReport> {
        let catalog = self.reader.catalog()?.clone();
        let Some(root_value) = catalog.get("StructTreeRoot").cloned() else {
            self.finding(
                TaggedPdfFindingSeverity::Warning,
                TaggedPdfFindingCode::MissingStructureTree,
                "/Catalog/StructTreeRoot",
                "document has no structure tree",
                None,
            );
            return Ok(self.report(false, None));
        };
        let root_ref = root_value.as_reference().map(Into::into);
        let Some(root) = self.resolve_dict(&root_value)? else {
            self.finding(
                TaggedPdfFindingSeverity::Error,
                TaggedPdfFindingCode::InvalidStructureTreeRoot,
                "/Catalog/StructTreeRoot",
                "StructTreeRoot is not a dictionary",
                root_ref,
            );
            return Ok(self.report(true, root_ref));
        };
        if root.get_type() != Some("StructTreeRoot") {
            self.finding(
                TaggedPdfFindingSeverity::Error,
                TaggedPdfFindingCode::InvalidStructureTreeRoot,
                "/StructTreeRoot/Type",
                "structure-tree root has no /Type /StructTreeRoot",
                root_ref,
            );
        }
        self.read_name_map(root.get("RoleMap").cloned(), true)?;
        self.read_name_map(root.get("ClassMap").cloned(), false)?;
        self.validate_role_map();

        if let Some(parent_tree) = root.get("ParentTree").cloned() {
            self.walk_number_tree(&parent_tree, 0)?;
        } else {
            self.finding(
                TaggedPdfFindingSeverity::Error,
                TaggedPdfFindingCode::MissingParentTree,
                "/StructTreeRoot/ParentTree",
                "tagged structure has no ParentTree",
                root_ref,
            );
        }

        if let Some(kids) = root.get("K").cloned() {
            self.walk_structure_value(&kids, root_ref, None, 0, "/StructTreeRoot/K")?;
        }
        self.elements.sort_by_key(|element| element.object);
        self.class_names.sort();
        self.class_names.dedup();
        self.findings.sort_by(|a, b| {
            a.path
                .cmp(&b.path)
                .then(a.severity.cmp(&b.severity))
                .then(a.message.cmp(&b.message))
        });
        Ok(self.report(true, root_ref))
    }

    fn report(self, tagged: bool, root: Option<TaggedPdfObjectRef>) -> TaggedPdfValidationReport {
        let valid = !self
            .findings
            .iter()
            .any(|finding| finding.severity == TaggedPdfFindingSeverity::Error);
        TaggedPdfValidationReport {
            tagged,
            valid,
            structure_tree_root: root,
            role_map: self.role_map,
            class_names: self.class_names,
            elements: self.elements,
            parent_tree_entries: self.parent_tree.len(),
            findings: self.findings,
        }
    }

    fn resolve(&mut self, value: &PdfObject) -> Result<PdfObject> {
        match value.as_reference() {
            Some((number, generation)) => Ok(self.reader.get_object(number, generation)?.clone()),
            None => Ok(value.clone()),
        }
    }

    fn resolve_dict(&mut self, value: &PdfObject) -> Result<Option<PdfDictionary>> {
        let object = self.resolve(value)?;
        Ok(object
            .as_dict()
            .cloned()
            .or_else(|| object.as_stream().map(|stream| stream.dict.clone())))
    }

    fn read_name_map(&mut self, value: Option<PdfObject>, role_map: bool) -> Result<()> {
        let Some(value) = value else { return Ok(()) };
        let Some(dict) = self.resolve_dict(&value)? else {
            return Ok(());
        };
        for (name, value) in &dict.0 {
            if role_map {
                if let Some(target) = value.as_name() {
                    self.role_map
                        .insert(name.0.clone(), target.as_str().to_string());
                }
            } else {
                self.class_names.push(name.0.clone());
            }
        }
        Ok(())
    }

    fn validate_role_map(&mut self) {
        let names: Vec<_> = self.role_map.keys().cloned().collect();
        for name in names {
            if let Err(message) = self.resolve_role(&name) {
                self.finding(
                    TaggedPdfFindingSeverity::Error,
                    TaggedPdfFindingCode::InvalidRoleMap,
                    format!("/StructTreeRoot/RoleMap/{name}"),
                    message,
                    None,
                );
            }
        }
    }

    fn resolve_role(&self, name: &str) -> std::result::Result<String, String> {
        let mut current = name;
        let mut seen = HashSet::new();
        loop {
            if is_standard_structure_type(current) {
                return Ok(current.to_string());
            }
            if !seen.insert(current.to_string()) {
                return Err("RoleMap contains a cycle".to_string());
            }
            let Some(next) = self.role_map.get(current) else {
                return Err(format!(
                    "RoleMap chain does not resolve custom type /{current} to a standard type"
                ));
            };
            current = next;
        }
    }

    fn walk_structure_value(
        &mut self,
        value: &PdfObject,
        expected_parent: Option<TaggedPdfObjectRef>,
        inherited_page: Option<TaggedPdfObjectRef>,
        depth: usize,
        path: &str,
    ) -> Result<usize> {
        self.check_depth(depth)?;
        if let Some(array) = value.as_array() {
            let mut marked = 0;
            for (index, child) in array.0.iter().enumerate() {
                self.bump_entry()?;
                marked += self.walk_structure_value(
                    child,
                    expected_parent,
                    inherited_page,
                    depth + 1,
                    &format!("{path}[{index}]"),
                )?;
            }
            return Ok(marked);
        }
        if let Some(mcid) = value.as_integer() {
            let mut mcr = PdfDictionary::new();
            mcr.insert("MCID".to_string(), PdfObject::Integer(mcid));
            if let Some(page) = inherited_page {
                mcr.insert(
                    "Pg".to_string(),
                    PdfObject::Reference(page.object_number, page.generation),
                );
            }
            return self
                .validate_mcr(&mcr, expected_parent, inherited_page, path)
                .map(usize::from);
        }
        let object_ref = value.as_reference().map(TaggedPdfObjectRef::from);
        if let Some(reference) = object_ref {
            if self.active.contains(&reference) {
                self.finding(
                    TaggedPdfFindingSeverity::Error,
                    TaggedPdfFindingCode::StructureCycle,
                    path,
                    "cycle detected in structure tree",
                    Some(reference),
                );
                return Ok(0);
            }
            if !self.visited.insert(reference) {
                self.finding(
                    TaggedPdfFindingSeverity::Error,
                    TaggedPdfFindingCode::DuplicateStructureReference,
                    path,
                    "structure object is referenced by more than one child position",
                    Some(reference),
                );
                return Ok(0);
            }
            if self.visited.len() > self.limits.max_objects {
                return Err(self.limit_error("structure objects", self.limits.max_objects));
            }
            self.active.insert(reference);
        }
        let Some(dict) = self.resolve_dict(value)? else {
            self.finding(
                TaggedPdfFindingSeverity::Error,
                TaggedPdfFindingCode::InvalidStructureElement,
                path,
                "structure child is not an integer, array, dictionary, or resolvable reference",
                object_ref,
            );
            return Ok(0);
        };
        let kind = dict.get_type();
        if kind == Some("OBJR") {
            self.validate_objr(&dict, expected_parent, inherited_page, path)?;
            if let Some(reference) = object_ref {
                self.active.remove(&reference);
            }
            return Ok(0);
        }
        if kind == Some("MCR") || dict.contains_key("MCID") {
            let marked = self.validate_mcr(&dict, expected_parent, inherited_page, path)?;
            if let Some(reference) = object_ref {
                self.active.remove(&reference);
            }
            return Ok(usize::from(marked));
        }
        if object_ref.is_none() {
            self.finding(
                TaggedPdfFindingSeverity::Error,
                TaggedPdfFindingCode::DirectStructureElement,
                path,
                "structure elements must be indirect objects",
                None,
            );
        }
        let structure_type = dict
            .get("S")
            .and_then(PdfObject::as_name)
            .map(|name| name.as_str().to_string());
        if structure_type.is_none() {
            self.finding(
                TaggedPdfFindingSeverity::Error,
                TaggedPdfFindingCode::MissingStructureType,
                &format!("{path}/S"),
                "structure element has no name-valued /S",
                object_ref,
            );
        } else if let Some(name) = structure_type.as_deref() {
            if !is_standard_structure_type(name) && self.resolve_role(name).is_err() {
                self.finding(
                    TaggedPdfFindingSeverity::Warning,
                    TaggedPdfFindingCode::UnmappedCustomRole,
                    &format!("{path}/S"),
                    "custom structure type is absent from RoleMap",
                    object_ref,
                );
            }
        }
        let parent = dict
            .get("P")
            .and_then(PdfObject::as_reference)
            .map(Into::into);
        if expected_parent.is_some() && parent != expected_parent {
            self.finding(
                TaggedPdfFindingSeverity::Error,
                TaggedPdfFindingCode::BrokenParentLink,
                &format!("{path}/P"),
                "structure element /P does not point to its containing parent",
                object_ref,
            );
        }
        let page = dict
            .get("Pg")
            .and_then(PdfObject::as_reference)
            .map(Into::into)
            .or(inherited_page);
        let child_count = dict
            .get("K")
            .map(|kids| match kids {
                PdfObject::Array(array) => array.0.len(),
                _ => 1,
            })
            .unwrap_or(0);
        if let Some(kids) = dict.get("K") {
            self.walk_structure_value(kids, object_ref, page, depth + 1, &format!("{path}/K"))?;
        }
        if let Some(reference) = object_ref {
            let marked_content_count = self.direct_mcid_counts.remove(&reference).unwrap_or(0);
            self.elements.push(TaggedPdfStructureElement {
                object: reference,
                structure_type,
                parent,
                child_count,
                marked_content_count,
            });
            self.active.remove(&reference);
        }
        Ok(0)
    }

    fn validate_objr(
        &mut self,
        dict: &PdfDictionary,
        owner: Option<TaggedPdfObjectRef>,
        inherited_page: Option<TaggedPdfObjectRef>,
        path: &str,
    ) -> Result<()> {
        let Some(object) = dict
            .get("Obj")
            .and_then(PdfObject::as_reference)
            .map(TaggedPdfObjectRef::from)
        else {
            self.finding(
                TaggedPdfFindingSeverity::Error,
                TaggedPdfFindingCode::InvalidObjectReference,
                format!("{path}/Obj"),
                "OBJR has no indirect /Obj reference",
                owner,
            );
            return Ok(());
        };
        let referenced = self
            .reader
            .get_object(object.object_number, object.generation)?
            .clone();
        let Some(key) = referenced
            .as_dict()
            .or_else(|| referenced.as_stream().map(|stream| &stream.dict))
            .and_then(|dict| dict.get("StructParent"))
            .and_then(PdfObject::as_integer)
        else {
            self.finding(
                TaggedPdfFindingSeverity::Error,
                TaggedPdfFindingCode::InvalidObjectReference,
                format!("{path}/Obj/StructParent"),
                "object referenced by OBJR has no integer StructParent key",
                Some(object),
            );
            return Ok(());
        };
        let actual = self
            .parent_tree
            .get(&key)
            .and_then(PdfObject::as_reference)
            .map(TaggedPdfObjectRef::from);
        if actual != owner {
            self.finding(
                TaggedPdfFindingSeverity::Error,
                TaggedPdfFindingCode::ParentTreeOwnerMismatch,
                format!("/StructTreeRoot/ParentTree/{key}"),
                "ParentTree object-reference entry does not point back to its structure element",
                owner,
            );
        }
        let page = dict
            .get("Pg")
            .and_then(PdfObject::as_reference)
            .map(Into::into)
            .or(inherited_page);
        if page.is_none() {
            self.finding(
                TaggedPdfFindingSeverity::Warning,
                TaggedPdfFindingCode::InvalidObjectReference,
                format!("{path}/Pg"),
                "OBJR has no page association",
                owner,
            );
        }
        Ok(())
    }

    fn validate_mcr(
        &mut self,
        dict: &PdfDictionary,
        owner: Option<TaggedPdfObjectRef>,
        inherited_page: Option<TaggedPdfObjectRef>,
        path: &str,
    ) -> Result<bool> {
        let Some(mcid) = dict.get("MCID").and_then(PdfObject::as_integer) else {
            if dict.get_type() == Some("MCR") {
                self.finding(
                    TaggedPdfFindingSeverity::Error,
                    TaggedPdfFindingCode::InvalidMarkedContentReference,
                    &format!("{path}/MCID"),
                    "marked-content reference has no non-negative integer MCID",
                    owner,
                );
            }
            return Ok(false);
        };
        if mcid < 0 {
            self.finding(
                TaggedPdfFindingSeverity::Error,
                TaggedPdfFindingCode::InvalidMarkedContentReference,
                &format!("{path}/MCID"),
                "MCID must be non-negative",
                owner,
            );
            return Ok(false);
        }
        if let Some(owner) = owner {
            *self.direct_mcid_counts.entry(owner).or_default() += 1;
        }
        let page = dict
            .get("Pg")
            .and_then(PdfObject::as_reference)
            .map(Into::into)
            .or(inherited_page);
        let Some(page) = page else {
            self.finding(
                TaggedPdfFindingSeverity::Error,
                TaggedPdfFindingCode::InvalidMarkedContentReference,
                &format!("{path}/Pg"),
                "MCID has no page association",
                owner,
            );
            return Ok(true);
        };
        let page_object = self
            .reader
            .get_object(page.object_number, page.generation)?
            .clone();
        let context = dict
            .get("Stm")
            .and_then(PdfObject::as_reference)
            .map(TaggedPdfObjectRef::from)
            .unwrap_or(page);
        let context_object = if context == page {
            page_object
        } else {
            self.reader
                .get_object(context.object_number, context.generation)?
                .clone()
        };
        let struct_parent = context_object
            .as_dict()
            .or_else(|| context_object.as_stream().map(|stream| &stream.dict))
            .and_then(|object| object.get("StructParents"))
            .and_then(PdfObject::as_integer);
        let Some(key) = struct_parent else {
            self.finding(
                TaggedPdfFindingSeverity::Error,
                TaggedPdfFindingCode::MissingStructParents,
                &format!("{path}/Pg/StructParents"),
                "page associated with MCID has no integer StructParents key",
                Some(context),
            );
            return Ok(true);
        };
        let Some(parent_entry) = self.parent_tree.get(&key).cloned() else {
            self.finding(
                TaggedPdfFindingSeverity::Error,
                TaggedPdfFindingCode::MissingParentTreeEntry,
                &format!("/StructTreeRoot/ParentTree/{key}"),
                "ParentTree has no entry for the page StructParents key",
                Some(context),
            );
            return Ok(true);
        };
        let resolved = self.resolve(&parent_entry)?;
        let actual = resolved
            .as_array()
            .and_then(|array| array.0.get(mcid as usize))
            .and_then(PdfObject::as_reference)
            .map(TaggedPdfObjectRef::from);
        if actual != owner {
            self.finding(
                TaggedPdfFindingSeverity::Error,
                TaggedPdfFindingCode::ParentTreeOwnerMismatch,
                &format!("/StructTreeRoot/ParentTree/{key}[{mcid}]"),
                "ParentTree MCID slot does not point back to its structure element",
                owner,
            );
        }
        let mcids = self.mcids_for_context(context, &context_object)?;
        if !mcids.contains(&mcid) {
            self.finding(
                TaggedPdfFindingSeverity::Error,
                TaggedPdfFindingCode::MissingMarkedContent,
                format!("{path}/MCID"),
                "MCID is referenced by the structure tree but absent from its content stream",
                owner,
            );
        }
        Ok(true)
    }

    fn mcids_for_context(
        &mut self,
        context: TaggedPdfObjectRef,
        object: &PdfObject,
    ) -> Result<HashSet<i64>> {
        if let Some(cached) = self.page_mcids.get(&context) {
            return Ok(cached.clone());
        }
        let content = if let Some(stream) = object.as_stream() {
            vec![stream.decode(self.reader.options())?]
        } else if let Some(dict) = object.as_dict() {
            match dict.get("Contents") {
                Some(contents) => self.decode_content_objects(contents, 0)?,
                None => Vec::new(),
            }
        } else {
            Vec::new()
        };
        let resources = self.effective_resources(object, 0)?;
        let mut mcids = HashSet::new();
        for bytes in content {
            self.decoded_content_bytes = self
                .decoded_content_bytes
                .checked_add(bytes.len())
                .ok_or_else(|| self.limit_error("decoded content bytes", usize::MAX))?;
            if self.decoded_content_bytes > self.limits.max_decoded_content_bytes {
                return Err(self.limit_error(
                    "decoded content bytes",
                    self.limits.max_decoded_content_bytes,
                ));
            }
            for operation in ContentParser::parse(&bytes)? {
                if let ContentOperation::BeginMarkedContentWithProps(_, properties) = operation {
                    if let Some(mcid) =
                        self.mcid_from_properties(&properties, resources.as_ref())?
                    {
                        if mcid >= 0 {
                            mcids.insert(mcid);
                        }
                    }
                }
            }
        }
        self.page_mcids.insert(context, mcids.clone());
        Ok(mcids)
    }

    fn mcid_from_properties(
        &mut self,
        properties: &MarkedContentProps,
        resources: Option<&PdfDictionary>,
    ) -> Result<Option<i64>> {
        match properties {
            MarkedContentProps::Inline(properties) => {
                Ok(properties.get("MCID").and_then(|value| match value {
                    MarkedContentValue::Integer(mcid) => Some(*mcid),
                    _ => None,
                }))
            }
            MarkedContentProps::ResourceRef(name) => {
                let Some(properties) = resources.and_then(|dict| dict.get("Properties")) else {
                    return Ok(None);
                };
                let Some(properties) = self.resolve_dict(properties)? else {
                    return Ok(None);
                };
                let Some(value) = properties.get(name) else {
                    return Ok(None);
                };
                Ok(self
                    .resolve_dict(value)?
                    .and_then(|dict| dict.get("MCID").and_then(PdfObject::as_integer)))
            }
        }
    }

    fn effective_resources(
        &mut self,
        object: &PdfObject,
        depth: usize,
    ) -> Result<Option<PdfDictionary>> {
        self.check_depth(depth)?;
        let Some(dict) = object
            .as_dict()
            .or_else(|| object.as_stream().map(|stream| &stream.dict))
        else {
            return Ok(None);
        };
        if let Some(resources) = dict.get("Resources") {
            return self.resolve_dict(resources);
        }
        let Some(parent) = dict.get("Parent") else {
            return Ok(None);
        };
        let parent = self.resolve(parent)?;
        self.effective_resources(&parent, depth + 1)
    }

    fn decode_content_objects(&mut self, value: &PdfObject, depth: usize) -> Result<Vec<Vec<u8>>> {
        self.check_depth(depth)?;
        let resolved = self.resolve(value)?;
        if let Some(stream) = resolved.as_stream() {
            return Ok(vec![stream.decode(self.reader.options())?]);
        }
        if let Some(array) = resolved.as_array() {
            let mut streams = Vec::new();
            for child in &array.0 {
                self.bump_entry()?;
                streams.extend(self.decode_content_objects(child, depth + 1)?);
            }
            return Ok(streams);
        }
        Err(PdfError::InvalidStructure(
            "page /Contents is neither a stream nor an array of streams".to_string(),
        ))
    }

    fn walk_number_tree(&mut self, value: &PdfObject, depth: usize) -> Result<()> {
        self.check_depth(depth)?;
        let reference = value.as_reference().map(TaggedPdfObjectRef::from);
        if let Some(reference) = reference {
            if !self.visited.insert(reference) {
                self.finding(
                    TaggedPdfFindingSeverity::Error,
                    TaggedPdfFindingCode::InvalidParentTree,
                    "/StructTreeRoot/ParentTree",
                    "cycle or duplicate node in ParentTree",
                    Some(reference),
                );
                return Ok(());
            }
            if self.visited.len() > self.limits.max_objects {
                return Err(self.limit_error("indirect objects", self.limits.max_objects));
            }
        }
        let Some(dict) = self.resolve_dict(value)? else {
            self.finding(
                TaggedPdfFindingSeverity::Error,
                TaggedPdfFindingCode::InvalidParentTree,
                "/StructTreeRoot/ParentTree",
                "ParentTree node is not a dictionary",
                reference,
            );
            return Ok(());
        };
        if dict.contains_key("Nums") && dict.contains_key("Kids") {
            self.finding(
                TaggedPdfFindingSeverity::Error,
                TaggedPdfFindingCode::InvalidParentTree,
                "/StructTreeRoot/ParentTree",
                "number-tree node must not contain both Nums and Kids",
                reference,
            );
        }
        let declared_limits = if let Some(limits) = dict.get("Limits") {
            let limits = self.resolve(limits)?;
            let parsed = limits
                .as_array()
                .and_then(|array| match array.0.as_slice() {
                    [PdfObject::Integer(first), PdfObject::Integer(last)]
                        if *first >= 0 && first <= last =>
                    {
                        Some((*first, *last))
                    }
                    _ => None,
                });
            if parsed.is_none() {
                self.finding(
                    TaggedPdfFindingSeverity::Error,
                    TaggedPdfFindingCode::InvalidParentTree,
                    "/StructTreeRoot/ParentTree/Limits",
                    "number-tree Limits must be two ordered non-negative integers",
                    reference,
                );
            }
            parsed
        } else {
            None
        };
        if let Some(nums) = dict.get("Nums") {
            let nums = self.resolve(nums)?;
            let Some(array) = nums.as_array() else {
                self.finding(
                    TaggedPdfFindingSeverity::Error,
                    TaggedPdfFindingCode::InvalidParentTree,
                    "/StructTreeRoot/ParentTree/Nums",
                    "number-tree Nums is not an array",
                    reference,
                );
                return Ok(());
            };
            if array.0.len() % 2 != 0 {
                self.finding(
                    TaggedPdfFindingSeverity::Error,
                    TaggedPdfFindingCode::InvalidParentTree,
                    "/StructTreeRoot/ParentTree/Nums",
                    "number-tree Nums must contain key/value pairs",
                    reference,
                );
            }
            let actual_limits = array
                .0
                .first()
                .and_then(PdfObject::as_integer)
                .zip(array.0.iter().rev().nth(1).and_then(PdfObject::as_integer));
            if declared_limits.is_some() && declared_limits != actual_limits {
                self.finding(
                    TaggedPdfFindingSeverity::Error,
                    TaggedPdfFindingCode::InvalidParentTree,
                    "/StructTreeRoot/ParentTree/Limits",
                    "number-tree Limits do not match the first and last Nums keys",
                    reference,
                );
            }
            let mut previous_key = None;
            for pair in array.0.chunks_exact(2) {
                self.bump_entry()?;
                if let Some(key) = pair[0].as_integer() {
                    if key < 0 {
                        self.finding(
                            TaggedPdfFindingSeverity::Error,
                            TaggedPdfFindingCode::InvalidParentTree,
                            "/StructTreeRoot/ParentTree/Nums",
                            "ParentTree keys must be non-negative integers",
                            reference,
                        );
                    }
                    if previous_key.is_some_and(|previous| key <= previous) {
                        self.finding(
                            TaggedPdfFindingSeverity::Error,
                            TaggedPdfFindingCode::InvalidParentTree,
                            "/StructTreeRoot/ParentTree/Nums",
                            "number-tree keys must be strictly increasing",
                            reference,
                        );
                    }
                    previous_key = Some(key);
                    if self.parent_tree.insert(key, pair[1].clone()).is_some() {
                        self.finding(
                            TaggedPdfFindingSeverity::Error,
                            TaggedPdfFindingCode::InvalidParentTree,
                            &format!("/StructTreeRoot/ParentTree/{key}"),
                            "duplicate ParentTree key",
                            reference,
                        );
                    }
                } else {
                    self.finding(
                        TaggedPdfFindingSeverity::Error,
                        TaggedPdfFindingCode::InvalidParentTree,
                        "/StructTreeRoot/ParentTree/Nums",
                        "number-tree key is not an integer",
                        reference,
                    );
                }
            }
        }
        if let Some(kids) = dict.get("Kids") {
            let kids = self.resolve(kids)?;
            if let Some(array) = kids.as_array() {
                for kid in &array.0 {
                    self.bump_entry()?;
                    self.walk_number_tree(kid, depth + 1)?;
                }
            } else {
                self.finding(
                    TaggedPdfFindingSeverity::Error,
                    TaggedPdfFindingCode::InvalidParentTree,
                    "/StructTreeRoot/ParentTree/Kids",
                    "number-tree Kids is not an array",
                    reference,
                );
            }
        }
        Ok(())
    }

    fn check_depth(&self, depth: usize) -> Result<()> {
        if depth > self.limits.max_depth {
            Err(self.limit_error("nesting depth", self.limits.max_depth))
        } else {
            Ok(())
        }
    }

    fn bump_entry(&mut self) -> Result<()> {
        self.entry_count += 1;
        if self.entry_count > self.limits.max_entries {
            Err(self.limit_error("structure entries", self.limits.max_entries))
        } else {
            Ok(())
        }
    }

    fn limit_error(&self, resource: &str, limit: usize) -> PdfError {
        PdfError::InvalidStructure(format!(
            "tagged-PDF {resource} exceed configured limit {limit}"
        ))
    }

    fn finding(
        &mut self,
        severity: TaggedPdfFindingSeverity,
        code: TaggedPdfFindingCode,
        path: impl Into<String>,
        message: impl Into<String>,
        object: Option<TaggedPdfObjectRef>,
    ) {
        self.findings.push(TaggedPdfFinding {
            severity,
            code,
            path: path.into(),
            message: message.into(),
            object,
        });
    }
}

fn is_standard_structure_type(name: &str) -> bool {
    matches!(
        name,
        "Document"
            | "Part"
            | "Art"
            | "Sect"
            | "Div"
            | "BlockQuote"
            | "Caption"
            | "TOC"
            | "TOCI"
            | "Index"
            | "NonStruct"
            | "Private"
            | "P"
            | "H"
            | "H1"
            | "H2"
            | "H3"
            | "H4"
            | "H5"
            | "H6"
            | "L"
            | "LI"
            | "Lbl"
            | "LBody"
            | "Table"
            | "TR"
            | "TH"
            | "TD"
            | "THead"
            | "TBody"
            | "TFoot"
            | "Span"
            | "Quote"
            | "Note"
            | "Reference"
            | "BibEntry"
            | "Code"
            | "Link"
            | "Annot"
            | "Ruby"
            | "RB"
            | "RT"
            | "RP"
            | "Warichu"
            | "WT"
            | "WP"
            | "Figure"
            | "Formula"
            | "Form"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pdf(objects: &[&str]) -> Vec<u8> {
        let mut bytes = b"%PDF-1.7\n".to_vec();
        let mut offsets = vec![0usize; objects.len() + 1];
        for (index, object) in objects.iter().enumerate() {
            offsets[index + 1] = bytes.len();
            bytes
                .extend_from_slice(format!("{} 0 obj\n{}\nendobj\n", index + 1, object).as_bytes());
        }
        let xref = bytes.len();
        bytes.extend_from_slice(
            format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).as_bytes(),
        );
        for offset in offsets.iter().skip(1) {
            bytes.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
        }
        bytes.extend_from_slice(
            format!(
                "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
                objects.len() + 1
            )
            .as_bytes(),
        );
        bytes
    }

    fn valid_tagged_pdf() -> Vec<u8> {
        pdf(&[
            "<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 5 0 R >>",
            "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /StructParents 0 /Contents 4 0 R >>",
            "<< /Length 22 >>\nstream\n/P <</MCID 0>> BDC EMC\nendstream",
            "<< /Type /StructTreeRoot /K [6 0 R] /ParentTree 7 0 R /RoleMap << /CustomP /P >> /ClassMap << /C1 << /O /Layout >> >> >>",
            "<< /Type /StructElem /S /CustomP /P 5 0 R /Pg 3 0 R /K [<< /Type /MCR /Pg 3 0 R /MCID 0 >>] >>",
            "<< /Nums [0 [6 0 R]] >>",
        ])
    }

    #[test]
    fn validates_bidirectional_tagged_structure() {
        let report = validate_tagged_pdf(&valid_tagged_pdf(), &Default::default()).unwrap();
        assert!(report.tagged);
        assert!(report.valid, "{:?}", report.findings);
        assert_eq!(report.elements.len(), 1);
        assert_eq!(report.elements[0].marked_content_count, 1);
        assert_eq!(report.parent_tree_entries, 1);
        assert_eq!(
            report.role_map.get("CustomP").map(String::as_str),
            Some("P")
        );
        assert_eq!(report.class_names, ["C1"]);
    }

    #[test]
    fn reports_parent_tree_owner_mismatch() {
        let bytes = pdf(&[
            "<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 5 0 R >>",
            "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /StructParents 0 /Contents 4 0 R >>",
            "<< /Length 22 >>\nstream\n/P <</MCID 0>> BDC EMC\nendstream",
            "<< /Type /StructTreeRoot /K [6 0 R] /ParentTree 7 0 R >>",
            "<< /Type /StructElem /S /P /P 5 0 R /Pg 3 0 R /K 0 >>",
            "<< /Nums [0 [5 0 R]] >>",
        ]);
        let report = validate_tagged_pdf(&bytes, &Default::default()).unwrap();
        assert!(!report.valid);
        assert!(report
            .findings
            .iter()
            .any(|finding| { finding.code == TaggedPdfFindingCode::ParentTreeOwnerMismatch }));
    }

    #[test]
    fn rejects_structure_cycles_without_recursing_forever() {
        let bytes = pdf(&[
            "<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 4 0 R >>",
            "<< /Type /Pages /Count 0 /Kids [] >>",
            "null",
            "<< /Type /StructTreeRoot /K 5 0 R /ParentTree 6 0 R >>",
            "<< /Type /StructElem /S /Document /P 4 0 R /K 5 0 R >>",
            "<< /Nums [] >>",
        ]);
        let report = validate_tagged_pdf(&bytes, &Default::default()).unwrap();
        assert!(!report.valid);
        assert!(report
            .findings
            .iter()
            .any(|finding| finding.code == TaggedPdfFindingCode::StructureCycle));
    }

    #[test]
    fn reports_untagged_document_without_failing_parse() {
        let bytes = pdf(&[
            "<< /Type /Catalog /Pages 2 0 R >>",
            "<< /Type /Pages /Count 0 /Kids [] >>",
        ]);
        let report = validate_tagged_pdf(&bytes, &Default::default()).unwrap();
        assert!(!report.tagged);
        assert!(report.valid);
        assert_eq!(
            report.findings[0].code,
            TaggedPdfFindingCode::MissingStructureTree
        );
    }

    #[test]
    fn enforces_nesting_limit() {
        let options = TaggedPdfValidationOptions {
            limits: TaggedPdfLimits {
                max_depth: 0,
                ..Default::default()
            },
        };
        let error = validate_tagged_pdf(&valid_tagged_pdf(), &options).unwrap_err();
        assert!(error.to_string().contains("nesting depth"));
    }

    #[test]
    fn rejects_direct_structure_elements() {
        let bytes = pdf(&[
            "<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 4 0 R >>",
            "<< /Type /Pages /Count 0 /Kids [] >>",
            "null",
            "<< /Type /StructTreeRoot /K << /Type /StructElem /S /P /P 4 0 R >> /ParentTree 5 0 R >>",
            "<< /Nums [] >>",
        ]);
        let report = validate_tagged_pdf(&bytes, &Default::default()).unwrap();
        assert!(!report.valid);
        assert!(report
            .findings
            .iter()
            .any(|finding| finding.code == TaggedPdfFindingCode::DirectStructureElement));
    }

    #[test]
    fn rejects_cyclic_role_maps() {
        let bytes = pdf(&[
            "<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 4 0 R >>",
            "<< /Type /Pages /Count 0 /Kids [] >>",
            "null",
            "<< /Type /StructTreeRoot /K 5 0 R /ParentTree 6 0 R /RoleMap << /A /B /B /A >> >>",
            "<< /Type /StructElem /S /A /P 4 0 R >>",
            "<< /Nums [] >>",
        ]);
        let report = validate_tagged_pdf(&bytes, &Default::default()).unwrap();
        assert!(!report.valid);
        assert!(report
            .findings
            .iter()
            .any(|finding| finding.code == TaggedPdfFindingCode::InvalidRoleMap));
    }

    #[test]
    fn rejects_an_mcid_absent_from_page_content() {
        let bytes = pdf(&[
            "<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 5 0 R >>",
            "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /StructParents 0 /Contents 4 0 R >>",
            "<< /Length 22 >>\nstream\n/P <</MCID 1>> BDC EMC\nendstream",
            "<< /Type /StructTreeRoot /K 6 0 R /ParentTree 7 0 R >>",
            "<< /Type /StructElem /S /P /P 5 0 R /Pg 3 0 R /K 0 >>",
            "<< /Nums [0 [6 0 R]] >>",
        ]);
        let report = validate_tagged_pdf(&bytes, &Default::default()).unwrap();
        assert!(!report.valid);
        assert!(report
            .findings
            .iter()
            .any(|finding| finding.code == TaggedPdfFindingCode::MissingMarkedContent));
    }

    #[test]
    fn resolves_mcid_from_a_resource_property_list() {
        let bytes = pdf(&[
            "<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 5 0 R >>",
            "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /StructParents 0 /Contents 4 0 R /Resources << /Properties << /MC0 << /MCID 0 >> >> >> >>",
            "<< /Length 15 >>\nstream\n/P /MC0 BDC EMC\nendstream",
            "<< /Type /StructTreeRoot /K 6 0 R /ParentTree 7 0 R >>",
            "<< /Type /StructElem /S /P /P 5 0 R /Pg 3 0 R /K 0 >>",
            "<< /Nums [0 [6 0 R]] >>",
        ]);
        let report = validate_tagged_pdf(&bytes, &Default::default()).unwrap();
        assert!(report.valid, "{:?}", report.findings);
    }

    #[test]
    fn validates_objr_through_struct_parent() {
        let bytes = pdf(&[
            "<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 4 0 R >>",
            "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] >>",
            "<< /Type /StructTreeRoot /K 5 0 R /ParentTree 6 0 R >>",
            "<< /Type /StructElem /S /Annot /P 4 0 R /Pg 3 0 R /K << /Type /OBJR /Obj 7 0 R >> >>",
            "<< /Nums [1 5 0 R] >>",
            "<< /Type /Annot /Subtype /Link /StructParent 1 >>",
        ]);
        let report = validate_tagged_pdf(&bytes, &Default::default()).unwrap();
        assert!(report.valid, "{:?}", report.findings);
    }

    #[test]
    fn rejects_malformed_number_tree_order_and_limits() {
        let bytes = pdf(&[
            "<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 4 0 R >>",
            "<< /Type /Pages /Count 0 /Kids [] >>",
            "null",
            "<< /Type /StructTreeRoot /K [] /ParentTree 5 0 R >>",
            "<< /Limits [1 0] /Nums [1 null 0 null] >>",
        ]);
        let report = validate_tagged_pdf(&bytes, &Default::default()).unwrap();
        assert!(!report.valid);
        assert!(
            report
                .findings
                .iter()
                .filter(|finding| { finding.code == TaggedPdfFindingCode::InvalidParentTree })
                .count()
                >= 2
        );
    }

    #[test]
    fn enforces_decoded_content_limit() {
        let options = TaggedPdfValidationOptions {
            limits: TaggedPdfLimits {
                max_decoded_content_bytes: 1,
                ..Default::default()
            },
        };
        let error = validate_tagged_pdf(&valid_tagged_pdf(), &options).unwrap_err();
        assert!(error.to_string().contains("decoded content bytes"));
    }
}
