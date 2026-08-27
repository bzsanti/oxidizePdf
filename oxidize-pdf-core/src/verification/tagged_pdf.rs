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
    InvalidClassMap,
    InvalidReadingOrder,
    InvalidTableStructure,
    InvalidListStructure,
    InvalidLinkStructure,
    InvalidArtifact,
    MissingDocumentLanguage,
    MissingAlternateText,
    InvalidHeadingOrder,
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
#[derive(Debug, Clone, PartialEq)]
pub struct TaggedPdfStructureElement {
    pub object: TaggedPdfObjectRef,
    pub structure_type: Option<String>,
    pub parent: Option<TaggedPdfObjectRef>,
    pub child_count: usize,
    pub marked_content_count: usize,
    pub language: Option<String>,
    pub alternate_text: Option<String>,
    pub actual_text: Option<String>,
    pub title: Option<String>,
    /// Every key present in the original dictionary, including unknown keys.
    pub dictionary_keys: Vec<String>,
    /// Complete source dictionary, preserving unknown attributes and namespaces.
    pub dictionary: PdfDictionary,
}

/// One `/Artifact` marked-content marker found in a page or form stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaggedPdfArtifact {
    pub context: TaggedPdfObjectRef,
    pub operation_index: usize,
    pub subtype: Option<String>,
}

/// Machine-readable result of tagged-PDF inspection.
#[derive(Debug, Clone, PartialEq)]
pub struct TaggedPdfValidationReport {
    pub tagged: bool,
    pub valid: bool,
    pub structure_tree_root: Option<TaggedPdfObjectRef>,
    pub role_map: BTreeMap<String, String>,
    pub class_names: Vec<String>,
    /// Lossless parsed ClassMap values, including unknown attributes and namespaces.
    pub class_map: BTreeMap<String, PdfObject>,
    pub artifacts: Vec<TaggedPdfArtifact>,
    pub elements: Vec<TaggedPdfStructureElement>,
    pub parent_tree_entries: usize,
    /// Lossless ParentTree values keyed by their integer number-tree key.
    pub parent_tree: BTreeMap<i64, PdfObject>,
    /// Every MCID found in each inspected page or form content context.
    pub content_mcids: BTreeMap<TaggedPdfObjectRef, Vec<i64>>,
    pub findings: Vec<TaggedPdfFinding>,
}

/// Inspect and validate the tagged structure of an existing PDF.
///
/// The traversal validates structure-element parent links and the page
/// `/StructParents` -> structure `/ParentTree` -> MCID owner association. It
/// also reports initial PDF/UA-oriented rules for catalog language, numbered
/// heading order, and text alternatives for figures and formulas. It does not
/// claim complete PDF/UA conformance; annotation semantics, complex table
/// headers, and complete visual-vs-logical reading-order rules are not yet implemented.
///
/// # Errors
///
/// Returns an error when the PDF cannot be parsed or decoded safely, is locked,
/// or exceeds a configured traversal or decoded-content limit. Structural and
/// PDF/UA findings are returned in the report rather than as function errors.
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
    class_map: BTreeMap<String, PdfObject>,
    parent_tree: BTreeMap<i64, PdfObject>,
    direct_mcid_counts: HashMap<TaggedPdfObjectRef, usize>,
    page_mcids: HashMap<TaggedPdfObjectRef, HashSet<i64>>,
    artifacts: Vec<TaggedPdfArtifact>,
    current_content_context: Option<TaggedPdfObjectRef>,
    active_content: HashSet<TaggedPdfObjectRef>,
    last_mcid_by_owner: HashMap<TaggedPdfObjectRef, i64>,
    last_mcid_by_context: HashMap<TaggedPdfObjectRef, i64>,
    objr_counts: HashMap<TaggedPdfObjectRef, usize>,
    decoded_content_bytes: usize,
    catalog_language: bool,
    last_heading_level: Option<u8>,
    saw_generic_heading: bool,
    saw_numbered_heading: bool,
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
            class_map: BTreeMap::new(),
            parent_tree: BTreeMap::new(),
            direct_mcid_counts: HashMap::new(),
            page_mcids: HashMap::new(),
            artifacts: Vec::new(),
            current_content_context: None,
            active_content: HashSet::new(),
            last_mcid_by_owner: HashMap::new(),
            last_mcid_by_context: HashMap::new(),
            objr_counts: HashMap::new(),
            decoded_content_bytes: 0,
            catalog_language: false,
            last_heading_level: None,
            saw_generic_heading: false,
            saw_numbered_heading: false,
        }
    }

    fn run(mut self) -> Result<TaggedPdfValidationReport> {
        let catalog = self.reader.catalog()?.clone();
        self.catalog_language = text_value(catalog.get("Lang")).is_some();
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
        if root_ref.is_none() {
            self.finding(
                TaggedPdfFindingSeverity::Error,
                TaggedPdfFindingCode::InvalidStructureTreeRoot,
                "/Catalog/StructTreeRoot",
                "StructTreeRoot must be an indirect object",
                None,
            );
        }
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
        self.read_role_map(root.get("RoleMap").cloned())?;
        self.read_class_map(root.get("ClassMap").cloned())?;
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
            self.walk_structure_value(&kids, root_ref, None, None, 0, "/StructTreeRoot/K")?;
        }
        self.scan_all_pages()?;
        if !self.catalog_language {
            self.finding(
                TaggedPdfFindingSeverity::Error,
                TaggedPdfFindingCode::MissingDocumentLanguage,
                "/Catalog/Lang",
                "tagged document catalog has no non-empty Lang entry",
                root_ref,
            );
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
        let content_mcids = self
            .page_mcids
            .iter()
            .map(|(context, values)| {
                let mut values: Vec<_> = values.iter().copied().collect();
                values.sort_unstable();
                (*context, values)
            })
            .collect();
        TaggedPdfValidationReport {
            tagged,
            valid,
            structure_tree_root: root,
            role_map: self.role_map,
            class_names: self.class_names,
            class_map: self.class_map,
            artifacts: self.artifacts,
            elements: self.elements,
            parent_tree_entries: self.parent_tree.len(),
            parent_tree: self.parent_tree,
            content_mcids,
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

    fn read_role_map(&mut self, value: Option<PdfObject>) -> Result<()> {
        let Some(value) = value else { return Ok(()) };
        let Some(dict) = self.resolve_dict(&value)? else {
            self.finding(
                TaggedPdfFindingSeverity::Error,
                TaggedPdfFindingCode::InvalidRoleMap,
                "/StructTreeRoot/RoleMap",
                "RoleMap must be a dictionary",
                value.as_reference().map(Into::into),
            );
            return Ok(());
        };
        for (name, value) in &dict.0 {
            if let Some(target) = value.as_name() {
                self.role_map
                    .insert(name.0.clone(), target.as_str().to_string());
            } else {
                self.finding(
                    TaggedPdfFindingSeverity::Error,
                    TaggedPdfFindingCode::InvalidRoleMap,
                    format!("/StructTreeRoot/RoleMap/{}", name.0),
                    "RoleMap values must be names",
                    None,
                );
            }
        }
        Ok(())
    }

    fn read_class_map(&mut self, value: Option<PdfObject>) -> Result<()> {
        let Some(value) = value else { return Ok(()) };
        let Some(dict) = self.resolve_dict(&value)? else {
            self.finding(
                TaggedPdfFindingSeverity::Error,
                TaggedPdfFindingCode::InvalidClassMap,
                "/StructTreeRoot/ClassMap",
                "ClassMap must be a dictionary",
                value.as_reference().map(Into::into),
            );
            return Ok(());
        };
        for (name, value) in dict.0 {
            self.class_names.push(name.0.clone());
            self.class_map.insert(name.0, value);
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
        expected_parent_type: Option<&str>,
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
                    expected_parent_type,
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
            self.validate_objr(
                &dict,
                expected_parent,
                expected_parent_type,
                inherited_page,
                path,
            )?;
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
        let language = text_value(dict.get("Lang"));
        let alternate_text = text_value(dict.get("Alt"));
        let actual_text = text_value(dict.get("ActualText"));
        let title = text_value(dict.get("T"));
        let resolved_type = structure_type
            .as_deref()
            .and_then(|name| self.resolve_role(name).ok());
        self.validate_parent_child(
            expected_parent_type,
            resolved_type.as_deref(),
            path,
            object_ref,
        );
        if matches!(resolved_type.as_deref(), Some("TH" | "TD")) {
            self.validate_table_cell(&dict, resolved_type.as_deref().unwrap(), path, object_ref);
        }
        if matches!(resolved_type.as_deref(), Some("Figure" | "Formula"))
            && alternate_text.is_none()
            && actual_text.is_none()
        {
            self.finding(
                TaggedPdfFindingSeverity::Error,
                TaggedPdfFindingCode::MissingAlternateText,
                format!("{path}/Alt"),
                "figure or formula has neither Alt nor ActualText",
                object_ref,
            );
        }
        if resolved_type.as_deref() == Some("H") {
            self.saw_generic_heading = true;
            if self.saw_numbered_heading {
                self.finding(
                    TaggedPdfFindingSeverity::Error,
                    TaggedPdfFindingCode::InvalidHeadingOrder,
                    format!("{path}/S"),
                    "generic H and numbered H1-H6 headings must not be mixed",
                    object_ref,
                );
            }
        } else if let Some(level) = resolved_type.as_deref().and_then(heading_level) {
            self.saw_numbered_heading = true;
            let invalid = if let Some(previous) = self.last_heading_level {
                level > previous + 1
            } else {
                level != 1
            };
            if invalid || self.saw_generic_heading {
                self.finding(
                    TaggedPdfFindingSeverity::Error,
                    TaggedPdfFindingCode::InvalidHeadingOrder,
                    format!("{path}/S"),
                    if self.saw_generic_heading {
                        "generic H and numbered H1-H6 headings must not be mixed"
                    } else {
                        "numbered headings must start at H1 and not skip levels"
                    },
                    object_ref,
                );
            }
            self.last_heading_level = Some(level);
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
            self.walk_structure_value(
                kids,
                object_ref,
                resolved_type.as_deref(),
                page,
                depth + 1,
                &format!("{path}/K"),
            )?;
        }
        if resolved_type.as_deref() == Some("Link")
            && object_ref
                .and_then(|reference| self.objr_counts.get(&reference).copied())
                .unwrap_or(0)
                == 0
        {
            self.finding(
                TaggedPdfFindingSeverity::Error,
                TaggedPdfFindingCode::InvalidLinkStructure,
                format!("{path}/K"),
                "Link structure element has no OBJR child",
                object_ref,
            );
        }
        if let Some(reference) = object_ref {
            let marked_content_count = self.direct_mcid_counts.remove(&reference).unwrap_or(0);
            let mut dictionary_keys: Vec<_> = dict.0.keys().map(|key| key.0.clone()).collect();
            dictionary_keys.sort();
            self.elements.push(TaggedPdfStructureElement {
                object: reference,
                structure_type,
                parent,
                child_count,
                marked_content_count,
                language,
                alternate_text,
                actual_text,
                title,
                dictionary_keys,
                dictionary: dict,
            });
            self.active.remove(&reference);
        }
        Ok(0)
    }

    fn validate_parent_child(
        &mut self,
        parent: Option<&str>,
        child: Option<&str>,
        path: &str,
        object: Option<TaggedPdfObjectRef>,
    ) {
        let Some((parent, child)) = parent.zip(child) else {
            return;
        };
        let violation = match parent {
            "Table" => (!matches!(child, "Caption" | "THead" | "TBody" | "TFoot" | "TR"))
                .then_some((
                    TaggedPdfFindingCode::InvalidTableStructure,
                    "Table children must be Caption, THead, TBody, TFoot, or TR",
                )),
            "THead" | "TBody" | "TFoot" => (child != "TR").then_some((
                TaggedPdfFindingCode::InvalidTableStructure,
                "table row groups may contain only TR elements",
            )),
            "TR" => (!matches!(child, "TH" | "TD")).then_some((
                TaggedPdfFindingCode::InvalidTableStructure,
                "TR elements may contain only TH or TD cells",
            )),
            "L" => (!matches!(child, "Caption" | "LI")).then_some((
                TaggedPdfFindingCode::InvalidListStructure,
                "L elements may contain only Caption or LI elements",
            )),
            "LI" => (!matches!(child, "Lbl" | "LBody")).then_some((
                TaggedPdfFindingCode::InvalidListStructure,
                "LI elements may contain only Lbl or LBody elements",
            )),
            _ => None,
        };
        if let Some((code, message)) = violation {
            self.finding(
                TaggedPdfFindingSeverity::Error,
                code,
                format!("{path}/S"),
                message,
                object,
            );
        }
    }

    fn validate_table_cell(
        &mut self,
        dict: &PdfDictionary,
        cell_type: &str,
        path: &str,
        object: Option<TaggedPdfObjectRef>,
    ) {
        for key in ["RowSpan", "ColSpan"] {
            if table_attribute(dict, key)
                .and_then(PdfObject::as_integer)
                .is_some_and(|span| span < 1)
            {
                self.finding(
                    TaggedPdfFindingSeverity::Error,
                    TaggedPdfFindingCode::InvalidTableStructure,
                    format!("{path}/A/{key}"),
                    format!("table-cell {key} must be a positive integer"),
                    object,
                );
            }
        }
        if cell_type != "TH" {
            return;
        }
        let scope = table_attribute(dict, "Scope").and_then(PdfObject::as_name);
        if scope.is_some_and(|scope| !matches!(scope.as_str(), "Row" | "Column" | "Both")) {
            self.finding(
                TaggedPdfFindingSeverity::Error,
                TaggedPdfFindingCode::InvalidTableStructure,
                format!("{path}/A/Scope"),
                "TH Scope must be Row, Column, or Both",
                object,
            );
        }
        let has_headers = table_attribute(dict, "Headers")
            .and_then(PdfObject::as_array)
            .is_some_and(|headers| !headers.0.is_empty());
        if scope.is_none() && !has_headers {
            self.finding(
                TaggedPdfFindingSeverity::Warning,
                TaggedPdfFindingCode::InvalidTableStructure,
                format!("{path}/A"),
                "TH should define Scope or Headers so data cells can identify their headers",
                object,
            );
        }
    }

    fn validate_objr(
        &mut self,
        dict: &PdfDictionary,
        owner: Option<TaggedPdfObjectRef>,
        owner_type: Option<&str>,
        inherited_page: Option<TaggedPdfObjectRef>,
        path: &str,
    ) -> Result<()> {
        if let Some(owner) = owner {
            *self.objr_counts.entry(owner).or_default() += 1;
        }
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
        if owner_type == Some("Link") {
            let annotation = referenced
                .as_dict()
                .or_else(|| referenced.as_stream().map(|stream| &stream.dict));
            if annotation
                .and_then(|dictionary| dictionary.get("Subtype"))
                .and_then(PdfObject::as_name)
                .is_none_or(|subtype| subtype.as_str() != "Link")
            {
                self.finding(
                    TaggedPdfFindingSeverity::Error,
                    TaggedPdfFindingCode::InvalidLinkStructure,
                    format!("{path}/Obj/Subtype"),
                    "OBJR child of Link must reference a Link annotation",
                    owner,
                );
            } else if annotation.is_some_and(|dictionary| {
                !dictionary.contains_key("A") && !dictionary.contains_key("Dest")
            }) {
                self.finding(
                    TaggedPdfFindingSeverity::Error,
                    TaggedPdfFindingCode::InvalidLinkStructure,
                    format!("{path}/Obj"),
                    "Link annotation has neither an action nor a destination",
                    owner,
                );
            }
        }
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
            if self
                .last_mcid_by_owner
                .insert(owner, mcid)
                .is_some_and(|previous| mcid <= previous)
            {
                self.finding(
                    TaggedPdfFindingSeverity::Error,
                    TaggedPdfFindingCode::InvalidReadingOrder,
                    format!("{path}/MCID"),
                    "MCIDs in one structure element must follow increasing content order",
                    Some(owner),
                );
            }
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
        if dict.contains_key("Stm") && dict.get("Stm").and_then(PdfObject::as_reference).is_none() {
            self.finding(
                TaggedPdfFindingSeverity::Error,
                TaggedPdfFindingCode::InvalidMarkedContentReference,
                format!("{path}/Stm"),
                "MCR Stm must be an indirect stream reference",
                owner,
            );
            return Ok(true);
        }
        let context = dict
            .get("Stm")
            .and_then(PdfObject::as_reference)
            .map(TaggedPdfObjectRef::from)
            .unwrap_or(page);
        if self
            .last_mcid_by_context
            .insert(context, mcid)
            .is_some_and(|previous| mcid <= previous)
        {
            self.finding(
                TaggedPdfFindingSeverity::Error,
                TaggedPdfFindingCode::InvalidReadingOrder,
                format!("{path}/MCID"),
                "logical structure order does not follow marked-content order in its content context",
                owner,
            );
        }
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
        if !self.active_content.insert(context) {
            return Ok(HashSet::new());
        }
        let resources = self.effective_resources(object, 0)?;
        let previous_context = self.current_content_context.replace(context);
        let mut mcids = HashSet::new();
        if let Some(stream) = object.as_stream() {
            let bytes = self.decode_content_stream(stream)?;
            self.process_content_bytes(&bytes, resources.as_ref(), &mut mcids)?;
        } else if let Some(contents) = object.as_dict().and_then(|dict| dict.get("Contents")) {
            self.process_content_objects(contents, resources.as_ref(), &mut mcids, 0)?;
        }
        self.page_mcids.insert(context, mcids.clone());
        self.active_content.remove(&context);
        self.current_content_context = previous_context;
        Ok(mcids)
    }

    fn process_content_bytes(
        &mut self,
        bytes: &[u8],
        resources: Option<&PdfDictionary>,
        mcids: &mut HashSet<i64>,
    ) -> Result<()> {
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
        for (operation_index, operation) in ContentParser::parse(bytes)?.into_iter().enumerate() {
            match operation {
                ContentOperation::BeginMarkedContent(tag) if tag == "Artifact" => {
                    self.record_artifact(operation_index, None);
                }
                ContentOperation::BeginMarkedContentWithProps(tag, properties)
                    if tag == "Artifact" =>
                {
                    if self.mcid_from_properties(&properties, resources)?.is_some() {
                        self.finding(
                            TaggedPdfFindingSeverity::Error,
                            TaggedPdfFindingCode::InvalidArtifact,
                            "/Contents/Artifact",
                            "Artifact marked content must not carry an MCID",
                            self.current_content_context,
                        );
                    }
                    self.record_artifact(operation_index, artifact_subtype(&properties));
                }
                ContentOperation::BeginMarkedContentWithProps(_, properties) => {
                    if let Some(mcid) = self.mcid_from_properties(&properties, resources)? {
                        if mcid >= 0 && !mcids.insert(mcid) {
                            self.finding(
                                TaggedPdfFindingSeverity::Error,
                                TaggedPdfFindingCode::InvalidReadingOrder,
                                "/Contents/MCID",
                                "content context contains a duplicate MCID",
                                self.current_content_context,
                            );
                        }
                    }
                }
                ContentOperation::PaintXObject(name) => {
                    self.scan_form_xobject(&name, resources)?;
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn decode_content_stream(&self, stream: &crate::parser::objects::PdfStream) -> Result<Vec<u8>> {
        let remaining = self
            .limits
            .max_decoded_content_bytes
            .saturating_sub(self.decoded_content_bytes);
        stream
            .decode_with_limit(self.reader.options(), remaining)
            .map_err(|error| {
                let message = error.to_string();
                if message.contains("limit") || message.contains("exceeds") {
                    self.limit_error(
                        "decoded content bytes",
                        self.limits.max_decoded_content_bytes,
                    )
                } else {
                    error.into()
                }
            })
    }

    fn record_artifact(&mut self, operation_index: usize, subtype: Option<String>) {
        if let Some(context) = self.current_content_context {
            self.artifacts.push(TaggedPdfArtifact {
                context,
                operation_index,
                subtype,
            });
        }
    }

    fn scan_form_xobject(&mut self, name: &str, resources: Option<&PdfDictionary>) -> Result<()> {
        let Some(xobjects) = resources
            .and_then(|resources| resources.get("XObject"))
            .cloned()
        else {
            return Ok(());
        };
        let xobjects = self.resolve(&xobjects)?;
        let Some(value) = xobjects.as_dict().and_then(|xobjects| xobjects.get(name)) else {
            return Ok(());
        };
        let Some(reference) = value.as_reference().map(TaggedPdfObjectRef::from) else {
            return Ok(());
        };
        let object = self
            .reader
            .get_object(reference.object_number, reference.generation)?
            .clone();
        if object
            .as_stream()
            .and_then(|stream| stream.dict.get("Subtype"))
            .and_then(PdfObject::as_name)
            .is_some_and(|subtype| subtype.0 == "Form")
        {
            self.mcids_for_context(reference, &object)?;
        }
        Ok(())
    }

    fn scan_all_pages(&mut self) -> Result<()> {
        let pages = self.reader.catalog()?.get("Pages").cloned();
        let mut references = Vec::new();
        let mut seen = HashSet::new();
        if let Some(pages) = pages {
            self.collect_page_references(&pages, 0, &mut seen, &mut references)?;
        }
        for context in references {
            if !self.page_mcids.contains_key(&context) {
                let object = self
                    .reader
                    .get_object(context.object_number, context.generation)?
                    .clone();
                self.mcids_for_context(context, &object)?;
            }
        }
        Ok(())
    }

    fn collect_page_references(
        &mut self,
        value: &PdfObject,
        depth: usize,
        seen: &mut HashSet<TaggedPdfObjectRef>,
        pages: &mut Vec<TaggedPdfObjectRef>,
    ) -> Result<()> {
        self.check_depth(depth)?;
        let reference = value.as_reference().map(TaggedPdfObjectRef::from);
        if reference.is_some_and(|reference| !seen.insert(reference)) {
            return Ok(());
        }
        let Some(dictionary) = self.resolve_dict(value)? else {
            return Ok(());
        };
        if dictionary.get_type() == Some("Page") {
            if let Some(reference) = reference {
                pages.push(reference);
            }
            return Ok(());
        }
        if let Some(kids) = dictionary.get("Kids") {
            let kids = self.resolve(kids)?;
            if let Some(array) = kids.as_array() {
                for kid in &array.0 {
                    self.collect_page_references(kid, depth + 1, seen, pages)?;
                }
            }
        }
        Ok(())
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

    fn process_content_objects(
        &mut self,
        value: &PdfObject,
        resources: Option<&PdfDictionary>,
        mcids: &mut HashSet<i64>,
        depth: usize,
    ) -> Result<()> {
        self.check_depth(depth)?;
        let resolved = self.resolve(value)?;
        if let Some(stream) = resolved.as_stream() {
            let bytes = self.decode_content_stream(stream)?;
            return self.process_content_bytes(&bytes, resources, mcids);
        }
        if let Some(array) = resolved.as_array() {
            for child in &array.0 {
                self.bump_entry()?;
                self.process_content_objects(child, resources, mcids, depth + 1)?;
            }
            return Ok(());
        }
        Err(PdfError::InvalidStructure(
            "page /Contents is neither a stream nor an array of streams".to_string(),
        ))
    }

    fn walk_number_tree(&mut self, value: &PdfObject, depth: usize) -> Result<Option<(i64, i64)>> {
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
                return Ok(None);
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
            return Ok(None);
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
        let mut effective_limits = None;
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
                return Ok(None);
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
            effective_limits = actual_limits;
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
                let mut previous = None;
                for kid in &array.0 {
                    self.bump_entry()?;
                    if let Some(range) = self.walk_number_tree(kid, depth + 1)? {
                        if previous.is_some_and(|(_, last)| range.0 <= last) {
                            self.finding(
                                TaggedPdfFindingSeverity::Error,
                                TaggedPdfFindingCode::InvalidParentTree,
                                "/StructTreeRoot/ParentTree/Kids",
                                "number-tree child ranges overlap or are out of order",
                                reference,
                            );
                        }
                        effective_limits = Some(match effective_limits {
                            Some((first, _)) => (first, range.1),
                            None => range,
                        });
                        previous = Some(range);
                    }
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
        if declared_limits.is_some() && declared_limits != effective_limits {
            self.finding(
                TaggedPdfFindingSeverity::Error,
                TaggedPdfFindingCode::InvalidParentTree,
                "/StructTreeRoot/ParentTree/Limits",
                "number-tree Limits do not match the effective subtree range",
                reference,
            );
        }
        Ok(effective_limits)
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

fn text_value(value: Option<&PdfObject>) -> Option<String> {
    value
        .and_then(PdfObject::as_string)
        .map(|value| value.to_text())
        .filter(|value| !value.trim().is_empty())
}

fn table_attribute<'a>(dictionary: &'a PdfDictionary, key: &str) -> Option<&'a PdfObject> {
    dictionary.get(key).or_else(|| match dictionary.get("A") {
        Some(PdfObject::Dictionary(attributes)) => attributes.get(key),
        Some(PdfObject::Array(attributes)) => attributes
            .0
            .iter()
            .find_map(|value| value.as_dict().and_then(|attributes| attributes.get(key))),
        _ => None,
    })
}

fn artifact_subtype(properties: &MarkedContentProps) -> Option<String> {
    match properties {
        MarkedContentProps::Inline(values) => values.get("Subtype").and_then(|value| match value {
            MarkedContentValue::Name(name) => Some(name.clone()),
            _ => None,
        }),
        MarkedContentProps::ResourceRef(_) => None,
    }
}

fn heading_level(name: &str) -> Option<u8> {
    match name {
        "H1" => Some(1),
        "H2" => Some(2),
        "H3" => Some(3),
        "H4" => Some(4),
        "H5" => Some(5),
        "H6" => Some(6),
        _ => None,
    }
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
            "<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 5 0 R /Lang (en-US) >>",
            "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /StructParents 0 /Contents 4 0 R >>",
            "<< /Length 22 >>\nstream\n/P <</MCID 0>> BDC EMC\nendstream",
            "<< /Type /StructTreeRoot /K [6 0 R] /ParentTree 7 0 R /RoleMap << /CustomP /P >> /ClassMap << /C1 << /O /Layout >> >> >>",
            "<< /Type /StructElem /S /CustomP /P 5 0 R /Pg 3 0 R /Lang (en-US) /K [<< /Type /MCR /Pg 3 0 R /MCID 0 >>] >>",
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
        assert!(matches!(
            report.class_map.get("C1"),
            Some(PdfObject::Dictionary(_))
        ));
    }

    #[test]
    fn reports_parent_tree_owner_mismatch() {
        let bytes = pdf(&[
            "<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 5 0 R /Lang (en-US) >>",
            "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /StructParents 0 /Contents 4 0 R >>",
            "<< /Length 22 >>\nstream\n/P <</MCID 0>> BDC EMC\nendstream",
            "<< /Type /StructTreeRoot /K [6 0 R] /ParentTree 7 0 R >>",
            "<< /Type /StructElem /S /P /P 5 0 R /Pg 3 0 R /Lang (en-US) /K 0 >>",
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
            "<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 4 0 R /Lang (en-US) >>",
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
            "<< /Type /StructElem /S /P /P 5 0 R /Pg 3 0 R /Lang (en-US) /K 0 >>",
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
            "<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 5 0 R /Lang (en-US) >>",
            "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /StructParents 0 /Contents 4 0 R /Resources << /Properties << /MC0 << /MCID 0 >> >> >> >>",
            "<< /Length 15 >>\nstream\n/P /MC0 BDC EMC\nendstream",
            "<< /Type /StructTreeRoot /K 6 0 R /ParentTree 7 0 R >>",
            "<< /Type /StructElem /S /P /P 5 0 R /Pg 3 0 R /Lang (en-US) /K 0 >>",
            "<< /Nums [0 [6 0 R]] >>",
        ]);
        let report = validate_tagged_pdf(&bytes, &Default::default()).unwrap();
        assert!(report.valid, "{:?}", report.findings);
    }

    #[test]
    fn validates_objr_through_struct_parent() {
        let bytes = pdf(&[
            "<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 4 0 R /Lang (en-US) >>",
            "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] >>",
            "<< /Type /StructTreeRoot /K 5 0 R /ParentTree 6 0 R >>",
            "<< /Type /StructElem /S /Annot /P 4 0 R /Pg 3 0 R /Lang (en-US) /K << /Type /OBJR /Obj 7 0 R >> >>",
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

    #[test]
    fn reports_missing_language_for_tagged_documents() {
        let bytes = pdf(&[
            "<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 4 0 R >>",
            "<< /Type /Pages /Count 0 /Kids [] >>",
            "null",
            "<< /Type /StructTreeRoot /K 5 0 R /ParentTree 6 0 R >>",
            "<< /Type /StructElem /S /Document /P 4 0 R >>",
            "<< /Nums [] >>",
        ]);
        let report = validate_tagged_pdf(&bytes, &Default::default()).unwrap();
        assert!(report
            .findings
            .iter()
            .any(|finding| { finding.code == TaggedPdfFindingCode::MissingDocumentLanguage }));
    }

    #[test]
    fn reports_figure_without_text_alternative() {
        let bytes = pdf(&[
            "<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 4 0 R /Lang (en-US) >>",
            "<< /Type /Pages /Count 0 /Kids [] >>",
            "null",
            "<< /Type /StructTreeRoot /K 5 0 R /ParentTree 6 0 R >>",
            "<< /Type /StructElem /S /Figure /P 4 0 R >>",
            "<< /Nums [] >>",
        ]);
        let report = validate_tagged_pdf(&bytes, &Default::default()).unwrap();
        assert!(report
            .findings
            .iter()
            .any(|finding| finding.code == TaggedPdfFindingCode::MissingAlternateText));
    }

    #[test]
    fn reports_skipped_heading_levels_in_structure_order() {
        let bytes = pdf(&[
            "<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 4 0 R /Lang (en-US) >>",
            "<< /Type /Pages /Count 0 /Kids [] >>",
            "null",
            "<< /Type /StructTreeRoot /K [5 0 R 6 0 R] /ParentTree 7 0 R >>",
            "<< /Type /StructElem /S /H1 /P 4 0 R >>",
            "<< /Type /StructElem /S /H3 /P 4 0 R >>",
            "<< /Nums [] >>",
        ]);
        let report = validate_tagged_pdf(&bytes, &Default::default()).unwrap();
        assert!(report
            .findings
            .iter()
            .any(|finding| finding.code == TaggedPdfFindingCode::InvalidHeadingOrder));
    }

    #[test]
    fn rejects_a_direct_structure_tree_root() {
        let bytes = pdf(&[
            "<< /Type /Catalog /Pages 2 0 R /Lang (en-US) /StructTreeRoot << /Type /StructTreeRoot /K [] /ParentTree << /Nums [] >> >> >>",
            "<< /Type /Pages /Count 0 /Kids [] >>",
        ]);
        let report = validate_tagged_pdf(&bytes, &Default::default()).unwrap();
        assert!(report
            .findings
            .iter()
            .any(|finding| finding.code == TaggedPdfFindingCode::InvalidStructureTreeRoot));
    }

    #[test]
    fn reports_malformed_role_and_class_maps() {
        let bytes = pdf(&[
            "<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 4 0 R /Lang (en-US) >>",
            "<< /Type /Pages /Count 0 /Kids [] >>",
            "null",
            "<< /Type /StructTreeRoot /K [] /ParentTree 5 0 R /RoleMap << /Custom 42 >> /ClassMap 7 >>",
            "<< /Nums [] >>",
        ]);
        let report = validate_tagged_pdf(&bytes, &Default::default()).unwrap();
        assert!(report
            .findings
            .iter()
            .any(|finding| finding.code == TaggedPdfFindingCode::InvalidRoleMap));
        assert!(report
            .findings
            .iter()
            .any(|finding| finding.code == TaggedPdfFindingCode::InvalidClassMap));
    }

    #[test]
    fn element_language_does_not_replace_catalog_language() {
        let bytes = pdf(&[
            "<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 4 0 R >>",
            "<< /Type /Pages /Count 0 /Kids [] >>",
            "null",
            "<< /Type /StructTreeRoot /K 5 0 R /ParentTree 6 0 R >>",
            "<< /Type /StructElem /S /Document /P 4 0 R /Lang (en-US) >>",
            "<< /Nums [] >>",
        ]);
        let report = validate_tagged_pdf(&bytes, &Default::default()).unwrap();
        assert!(report
            .findings
            .iter()
            .any(|finding| finding.code == TaggedPdfFindingCode::MissingDocumentLanguage));
    }

    #[test]
    fn applies_semantic_rules_after_role_mapping() {
        let bytes = pdf(&[
            "<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 4 0 R /Lang (en-US) >>",
            "<< /Type /Pages /Count 0 /Kids [] >>",
            "null",
            "<< /Type /StructTreeRoot /K 5 0 R /ParentTree 6 0 R /RoleMap << /Illustration /Figure >> >>",
            "<< /Type /StructElem /S /Illustration /P 4 0 R >>",
            "<< /Nums [] >>",
        ]);
        let report = validate_tagged_pdf(&bytes, &Default::default()).unwrap();
        assert!(report
            .findings
            .iter()
            .any(|finding| finding.code == TaggedPdfFindingCode::MissingAlternateText));
    }

    #[test]
    fn numbered_headings_must_start_at_h1_and_not_mix_with_h() {
        let bytes = pdf(&[
            "<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 4 0 R /Lang (en-US) >>",
            "<< /Type /Pages /Count 0 /Kids [] >>",
            "null",
            "<< /Type /StructTreeRoot /K [5 0 R 6 0 R] /ParentTree 7 0 R >>",
            "<< /Type /StructElem /S /H3 /P 4 0 R >>",
            "<< /Type /StructElem /S /H /P 4 0 R >>",
            "<< /Nums [] >>",
        ]);
        let report = validate_tagged_pdf(&bytes, &Default::default()).unwrap();
        assert!(
            report
                .findings
                .iter()
                .filter(|finding| finding.code == TaggedPdfFindingCode::InvalidHeadingOrder)
                .count()
                >= 2
        );
    }

    #[test]
    fn rejects_overlapping_parent_tree_child_ranges() {
        let bytes = pdf(&[
            "<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 4 0 R /Lang (en-US) >>",
            "<< /Type /Pages /Count 0 /Kids [] >>",
            "null",
            "<< /Type /StructTreeRoot /K [] /ParentTree 5 0 R >>",
            "<< /Limits [0 2] /Kids [6 0 R 7 0 R] >>",
            "<< /Limits [0 1] /Nums [0 null 1 null] >>",
            "<< /Limits [1 2] /Nums [1 null 2 null] >>",
        ]);
        let report = validate_tagged_pdf(&bytes, &Default::default()).unwrap();
        assert!(report.findings.iter().any(|finding| {
            finding.code == TaggedPdfFindingCode::InvalidParentTree
                && finding.message.contains("overlap")
        }));
    }

    #[test]
    fn rejects_direct_mcr_stream_associations() {
        let bytes = pdf(&[
            "<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 5 0 R /Lang (en-US) >>",
            "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /StructParents 0 /Contents 4 0 R >>",
            "<< /Length 22 >>\nstream\n/P <</MCID 0>> BDC EMC\nendstream",
            "<< /Type /StructTreeRoot /K 6 0 R /ParentTree 7 0 R >>",
            "<< /Type /StructElem /S /P /P 5 0 R /K << /Type /MCR /Pg 3 0 R /Stm << /Length 0 >> /MCID 0 >> >>",
            "<< /Nums [0 [6 0 R]] >>",
        ]);
        let report = validate_tagged_pdf(&bytes, &Default::default()).unwrap();
        assert!(report
            .findings
            .iter()
            .any(|finding| finding.code == TaggedPdfFindingCode::InvalidMarkedContentReference));
    }

    #[test]
    fn inventories_artifact_markers_and_subtypes() {
        let bytes = pdf(&[
            "<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 5 0 R /Lang (en-US) >>",
            "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R >>",
            "<< /Length 47 >>\nstream\n/Artifact <</Subtype /Pagination>> BDC EMC\nendstream",
            "<< /Type /StructTreeRoot /K [] /ParentTree 6 0 R >>",
            "<< /Nums [] >>",
        ]);
        let report = validate_tagged_pdf(&bytes, &Default::default()).unwrap();
        assert_eq!(report.artifacts.len(), 1);
        assert_eq!(report.artifacts[0].subtype.as_deref(), Some("Pagination"));
    }

    #[test]
    fn validates_table_list_and_link_structure() {
        let bytes = pdf(&[
            "<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 4 0 R /Lang (en-US) >>",
            "<< /Type /Pages /Count 0 /Kids [] >>",
            "null",
            "<< /Type /StructTreeRoot /K [5 0 R 7 0 R 9 0 R] /ParentTree 11 0 R >>",
            "<< /Type /StructElem /S /Table /P 4 0 R /K 6 0 R >>",
            "<< /Type /StructElem /S /P /P 5 0 R >>",
            "<< /Type /StructElem /S /L /P 4 0 R /K 8 0 R >>",
            "<< /Type /StructElem /S /P /P 7 0 R >>",
            "<< /Type /StructElem /S /Link /P 4 0 R >>",
            "null",
            "<< /Nums [] >>",
        ]);
        let report = validate_tagged_pdf(&bytes, &Default::default()).unwrap();
        for expected in [
            TaggedPdfFindingCode::InvalidTableStructure,
            TaggedPdfFindingCode::InvalidListStructure,
            TaggedPdfFindingCode::InvalidLinkStructure,
        ] {
            assert!(report
                .findings
                .iter()
                .any(|finding| finding.code == expected));
        }
    }

    #[test]
    fn rejects_duplicate_content_mcids() {
        let bytes = pdf(&[
            "<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 5 0 R /Lang (en-US) >>",
            "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /StructParents 0 /Contents 4 0 R >>",
            "<< /Length 44 >>\nstream\n/P <</MCID 0>> BDC EMC /P <</MCID 0>> BDC EMC\nendstream",
            "<< /Type /StructTreeRoot /K 6 0 R /ParentTree 7 0 R >>",
            "<< /Type /StructElem /S /P /P 5 0 R /Pg 3 0 R /K 0 >>",
            "<< /Nums [0 [6 0 R]] >>",
        ]);
        let report = validate_tagged_pdf(&bytes, &Default::default()).unwrap();
        assert!(report
            .findings
            .iter()
            .any(|finding| finding.code == TaggedPdfFindingCode::InvalidReadingOrder));
    }

    #[test]
    fn rejects_logical_order_that_reverses_page_content() {
        let bytes = pdf(&[
            "<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 5 0 R /Lang (en-US) >>",
            "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] /StructParents 0 /Contents 4 0 R >>",
            "<< /Length 44 >>\nstream\n/P <</MCID 0>> BDC EMC /P <</MCID 1>> BDC EMC\nendstream",
            "<< /Type /StructTreeRoot /K [7 0 R 6 0 R] /ParentTree 8 0 R >>",
            "<< /Type /StructElem /S /P /P 5 0 R /Pg 3 0 R /K 0 >>",
            "<< /Type /StructElem /S /P /P 5 0 R /Pg 3 0 R /K 1 >>",
            "<< /Nums [0 [6 0 R 7 0 R]] >>",
        ]);
        let report = validate_tagged_pdf(&bytes, &Default::default()).unwrap();
        assert!(report.findings.iter().any(|finding| {
            finding.code == TaggedPdfFindingCode::InvalidReadingOrder
                && finding.message.contains("logical structure order")
        }));
    }

    #[test]
    fn validates_table_header_attributes_and_link_annotation_semantics() {
        let bytes = pdf(&[
            "<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 4 0 R /Lang (en-US) >>",
            "<< /Type /Pages /Count 0 /Kids [] >>",
            "null",
            "<< /Type /StructTreeRoot /K [5 0 R 6 0 R] /ParentTree 9 0 R >>",
            "<< /Type /StructElem /S /TH /P 4 0 R /A << /O /Table /Scope /Invalid /RowSpan 0 >> >>",
            "<< /Type /StructElem /S /Link /P 4 0 R /K << /Type /OBJR /Obj 7 0 R >> >>",
            "<< /Type /Annot /Subtype /Text /StructParent 0 >>",
            "null",
            "<< /Nums [0 6 0 R] >>",
        ]);
        let report = validate_tagged_pdf(&bytes, &Default::default()).unwrap();
        assert!(report.findings.iter().any(|finding| {
            finding.code == TaggedPdfFindingCode::InvalidTableStructure
                && finding.path.ends_with("RowSpan")
        }));
        assert!(report.findings.iter().any(|finding| {
            finding.code == TaggedPdfFindingCode::InvalidLinkStructure
                && finding.path.ends_with("Subtype")
        }));
    }
}
