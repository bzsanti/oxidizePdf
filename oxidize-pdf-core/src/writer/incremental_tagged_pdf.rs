//! Lossless incremental edits for existing tagged-PDF structure trees.

use super::incremental_update::IncrementalUpdate;
use crate::error::{PdfError, Result};
use crate::parser::objects::{PdfArray, PdfDictionary, PdfName, PdfObject};
use crate::parser::PdfReader;
use crate::signatures::{ensure_modification_allowed, IncrementalModification};
use crate::verification::tagged_pdf::{
    validate_tagged_pdf, TaggedPdfObjectRef, TaggedPdfValidationOptions, TaggedPdfValidationReport,
};
use std::collections::{BTreeMap, BTreeSet};
use std::io::Cursor;

/// One controlled tagged-structure mutation.
#[derive(Debug, Clone, PartialEq)]
pub enum TaggedPdfMutation {
    /// Set or remove a non-structural entry on an existing StructElem dictionary.
    SetElementAttribute {
        element: TaggedPdfObjectRef,
        key: String,
        value: Option<PdfObject>,
    },
    /// Associate an MCID already present in page content with a structure element.
    AssociateMcid {
        element: TaggedPdfObjectRef,
        page: TaggedPdfObjectRef,
        mcid: u32,
    },
    /// Set or remove one ParentTree entry while preserving all unrelated entries.
    SetParentTreeEntry { key: i64, value: Option<PdfObject> },
}

/// Role of an indirect object changed by a tagged-PDF revision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TaggedPdfChangedObjectKind {
    StructureElement,
    ParentTree,
    CrossReference,
}

/// Exact object identified by a dry-run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct TaggedPdfChangedObject {
    pub object: TaggedPdfObjectRef,
    pub kind: TaggedPdfChangedObjectKind,
}

/// Dry-run result for one atomic tagged-PDF revision.
#[derive(Debug, Clone, PartialEq)]
pub struct TaggedPdfEditPlan {
    pub changed_objects: Vec<TaggedPdfChangedObject>,
    pub mutations: Vec<TaggedPdfMutation>,
}

/// Machine-readable result of an applied tagged-PDF revision.
#[derive(Debug, Clone, PartialEq)]
pub struct TaggedPdfMutationReport {
    pub pdf_bytes: Vec<u8>,
    pub plan: TaggedPdfEditPlan,
    pub validation_before: TaggedPdfValidationReport,
    pub validation_after: TaggedPdfValidationReport,
}

/// Plans and applies one lossless incremental tagged-structure revision.
pub struct IncrementalTaggedPdfEditor<'a> {
    base_bytes: &'a [u8],
    options: TaggedPdfValidationOptions,
}

impl<'a> IncrementalTaggedPdfEditor<'a> {
    pub fn new(base_bytes: &'a [u8]) -> Self {
        Self {
            base_bytes,
            options: TaggedPdfValidationOptions::default(),
        }
    }

    pub fn with_validation_options(mut self, options: TaggedPdfValidationOptions) -> Self {
        self.options = options;
        self
    }

    /// Validate a request and identify every indirect object that would change.
    ///
    /// # Errors
    ///
    /// Returns an error for malformed requests, encrypted documents, unsupported
    /// direct structure objects, missing content MCIDs, or forbidden DocMDP edits.
    pub fn plan(&self, mutations: &[TaggedPdfMutation]) -> Result<TaggedPdfEditPlan> {
        let validation = validate_tagged_pdf(self.base_bytes, &self.options)?;
        let mut reader = policy_reader(self.base_bytes)?;
        validate_mutations(&mut reader, &validation, mutations)?;
        build_plan(self.base_bytes, &mut reader, &validation, mutations)
    }

    /// Append one incremental revision, preserving the input as an exact prefix.
    ///
    /// # Errors
    ///
    /// Returns an error when planning, serialization, or reopening/validation fails.
    pub fn apply(&self, mutations: &[TaggedPdfMutation]) -> Result<TaggedPdfMutationReport> {
        let validation_before = validate_tagged_pdf(self.base_bytes, &self.options)?;
        let mut reader = policy_reader(self.base_bytes)?;
        validate_mutations(&mut reader, &validation_before, mutations)?;
        let plan = build_plan(self.base_bytes, &mut reader, &validation_before, mutations)?;
        if plan.changed_objects.is_empty() {
            return Ok(TaggedPdfMutationReport {
                pdf_bytes: self.base_bytes.to_vec(),
                plan,
                validation_after: validation_before.clone(),
                validation_before,
            });
        }

        let parent_tree = parent_tree_reference(&mut reader)?;
        let mut replacements = BTreeMap::<TaggedPdfObjectRef, PdfDictionary>::new();
        let mut parent_entries = validation_before.parent_tree.clone();
        let mut touches_parent_tree = false;

        for mutation in &plan.mutations {
            match mutation {
                TaggedPdfMutation::SetElementAttribute {
                    element,
                    key,
                    value,
                } => {
                    let dictionary =
                        replacement_dictionary(&mut reader, &mut replacements, *element)?;
                    set_dictionary_value(dictionary, key, value.clone());
                }
                TaggedPdfMutation::AssociateMcid {
                    element,
                    page,
                    mcid,
                } => {
                    let dictionary =
                        replacement_dictionary(&mut reader, &mut replacements, *element)?;
                    append_mcid(dictionary, *page, *mcid);
                    let page_dictionary = resolve_dictionary(&mut reader, *page)?;
                    let key = page_dictionary
                        .get("StructParents")
                        .and_then(PdfObject::as_integer)
                        .ok_or_else(|| invalid("association page has no integer StructParents"))?;
                    set_parent_owner(&mut reader, &mut parent_entries, key, *mcid, *element)?;
                    touches_parent_tree = true;
                }
                TaggedPdfMutation::SetParentTreeEntry { key, value } => {
                    if let Some(value) = value {
                        parent_entries.insert(*key, value.clone());
                    } else {
                        parent_entries.remove(key);
                    }
                    touches_parent_tree = true;
                }
            }
        }

        if touches_parent_tree {
            let reference =
                parent_tree.ok_or_else(|| invalid("StructTreeRoot ParentTree must be indirect"))?;
            let dictionary = replacement_dictionary(&mut reader, &mut replacements, reference)?;
            write_flat_parent_tree(dictionary, &parent_entries);
        }

        let mut update = IncrementalUpdate::from_reader(self.base_bytes, &reader)?;
        for (reference, dictionary) in replacements {
            update.replace(
                (reference.object_number, reference.generation),
                PdfObject::Dictionary(dictionary),
            )?;
        }
        let pdf_bytes = update.finish()?;
        if !pdf_bytes.starts_with(self.base_bytes) {
            return Err(invalid(
                "incremental result does not preserve the base as an exact prefix",
            ));
        }
        let validation_after = validate_tagged_pdf(&pdf_bytes, &self.options)?;
        Ok(TaggedPdfMutationReport {
            pdf_bytes,
            plan,
            validation_before,
            validation_after,
        })
    }
}

fn policy_reader(bytes: &[u8]) -> Result<PdfReader<Cursor<&[u8]>>> {
    let mut reader = PdfReader::new(Cursor::new(bytes))
        .map_err(|error| invalid(format!("parse tagged PDF: {error}")))?;
    if reader.is_encrypted() {
        return Err(PdfError::PermissionDenied(
            "incremental tagged-PDF editing is not supported on encrypted PDFs".to_string(),
        ));
    }
    let catalog = reader.catalog()?.clone();
    ensure_modification_allowed(
        &mut reader,
        &catalog,
        IncrementalModification::TaggedStructure,
    )?;
    Ok(reader)
}

fn validate_mutations(
    reader: &mut PdfReader<Cursor<&[u8]>>,
    report: &TaggedPdfValidationReport,
    mutations: &[TaggedPdfMutation],
) -> Result<()> {
    let elements: BTreeSet<_> = report
        .elements
        .iter()
        .map(|element| element.object)
        .collect();
    for mutation in mutations {
        match mutation {
            TaggedPdfMutation::SetElementAttribute { element, key, .. } => {
                if !elements.contains(element) {
                    return Err(invalid(
                        "attribute target is not an inspected structure element",
                    ));
                }
                if key.is_empty() || matches!(key.as_str(), "Type" | "S" | "P" | "K" | "Pg") {
                    return Err(invalid(
                        "core structure keys cannot be edited as attributes",
                    ));
                }
            }
            TaggedPdfMutation::AssociateMcid {
                element,
                page,
                mcid,
            } => {
                if !elements.contains(element) {
                    return Err(invalid("MCID owner is not an inspected structure element"));
                }
                if !report
                    .content_mcids
                    .get(page)
                    .is_some_and(|values| values.contains(&i64::from(*mcid)))
                {
                    return Err(invalid("requested MCID is absent from the page content"));
                }
                let page_dictionary = resolve_dictionary(reader, *page)?;
                if page_dictionary.get_type() != Some("Page") {
                    return Err(invalid("MCID association target is not a page"));
                }
                if page_dictionary
                    .get("StructParents")
                    .and_then(PdfObject::as_integer)
                    .is_none()
                {
                    return Err(invalid("association page has no integer StructParents"));
                }
                let key = page_dictionary
                    .get("StructParents")
                    .and_then(PdfObject::as_integer)
                    .expect("validated StructParents");
                if report
                    .parent_tree
                    .get(&key)
                    .and_then(PdfObject::as_array)
                    .and_then(|owners| owners.0.get(*mcid as usize))
                    .and_then(PdfObject::as_reference)
                    == Some((element.object_number, element.generation))
                {
                    return Err(invalid("requested MCID association already exists"));
                }
            }
            TaggedPdfMutation::SetParentTreeEntry { key, .. } if *key < 0 => {
                return Err(invalid("ParentTree keys must be non-negative"));
            }
            TaggedPdfMutation::SetParentTreeEntry { .. } => {}
        }
    }
    Ok(())
}

fn build_plan(
    base_bytes: &[u8],
    reader: &mut PdfReader<Cursor<&[u8]>>,
    report: &TaggedPdfValidationReport,
    mutations: &[TaggedPdfMutation],
) -> Result<TaggedPdfEditPlan> {
    let mut changed = BTreeSet::new();
    let mut parent_tree_changed = false;
    let mut effective = Vec::new();
    for mutation in mutations {
        match mutation {
            TaggedPdfMutation::SetElementAttribute {
                element,
                key,
                value,
            } => {
                let dictionary = resolve_dictionary(reader, *element)?;
                if dictionary.get(key) == value.as_ref() {
                    continue;
                }
                changed.insert(TaggedPdfChangedObject {
                    object: *element,
                    kind: TaggedPdfChangedObjectKind::StructureElement,
                });
                effective.push(mutation.clone());
            }
            TaggedPdfMutation::AssociateMcid { element, .. } => {
                changed.insert(TaggedPdfChangedObject {
                    object: *element,
                    kind: TaggedPdfChangedObjectKind::StructureElement,
                });
                parent_tree_changed = true;
                effective.push(mutation.clone());
            }
            TaggedPdfMutation::SetParentTreeEntry { key, value }
                if report.parent_tree.get(key) == value.as_ref() => {}
            TaggedPdfMutation::SetParentTreeEntry { .. } => {
                parent_tree_changed = true;
                effective.push(mutation.clone());
            }
        }
    }
    if parent_tree_changed {
        let object = parent_tree_reference(reader)?
            .ok_or_else(|| invalid("StructTreeRoot ParentTree must be indirect"))?;
        changed.insert(TaggedPdfChangedObject {
            object,
            kind: TaggedPdfChangedObjectKind::ParentTree,
        });
    }
    if !changed.is_empty() {
        if let Some(reference) = IncrementalUpdate::from_reader(base_bytes, reader)?
            .pending_xref_stream_id()
            .map(TaggedPdfObjectRef::from)
        {
            changed.insert(TaggedPdfChangedObject {
                object: reference,
                kind: TaggedPdfChangedObjectKind::CrossReference,
            });
        }
    }
    Ok(TaggedPdfEditPlan {
        changed_objects: changed.into_iter().collect(),
        mutations: effective,
    })
}

fn parent_tree_reference(
    reader: &mut PdfReader<Cursor<&[u8]>>,
) -> Result<Option<TaggedPdfObjectRef>> {
    let root = reader
        .catalog()?
        .get("StructTreeRoot")
        .and_then(PdfObject::as_reference)
        .map(TaggedPdfObjectRef::from)
        .ok_or_else(|| invalid("Catalog StructTreeRoot must be indirect"))?;
    let root = resolve_dictionary(reader, root)?;
    Ok(root
        .get("ParentTree")
        .and_then(PdfObject::as_reference)
        .map(TaggedPdfObjectRef::from))
}

fn resolve_dictionary(
    reader: &mut PdfReader<Cursor<&[u8]>>,
    reference: TaggedPdfObjectRef,
) -> Result<PdfDictionary> {
    reader
        .get_object(reference.object_number, reference.generation)?
        .as_dict()
        .cloned()
        .ok_or_else(|| {
            invalid(format!(
                "object {} is not a dictionary",
                reference.object_number
            ))
        })
}

fn replacement_dictionary<'a>(
    reader: &mut PdfReader<Cursor<&[u8]>>,
    replacements: &'a mut BTreeMap<TaggedPdfObjectRef, PdfDictionary>,
    reference: TaggedPdfObjectRef,
) -> Result<&'a mut PdfDictionary> {
    if !replacements.contains_key(&reference) {
        replacements.insert(reference, resolve_dictionary(reader, reference)?);
    }
    Ok(replacements
        .get_mut(&reference)
        .expect("inserted replacement"))
}

fn set_dictionary_value(dictionary: &mut PdfDictionary, key: &str, value: Option<PdfObject>) {
    if let Some(value) = value {
        dictionary.insert(key.to_string(), value);
    } else {
        dictionary.0.remove(&PdfName(key.to_string()));
    }
}

fn append_mcid(dictionary: &mut PdfDictionary, page: TaggedPdfObjectRef, mcid: u32) {
    let mut mcr = PdfDictionary::new();
    mcr.insert(
        "Type".to_string(),
        PdfObject::Name(PdfName("MCR".to_string())),
    );
    mcr.insert(
        "Pg".to_string(),
        PdfObject::Reference(page.object_number, page.generation),
    );
    mcr.insert("MCID".to_string(), PdfObject::Integer(i64::from(mcid)));
    let mcr = PdfObject::Dictionary(mcr);
    match dictionary.get("K").cloned() {
        None => dictionary.insert("K".to_string(), mcr),
        Some(PdfObject::Array(mut kids)) => {
            kids.push(mcr);
            dictionary.insert("K".to_string(), PdfObject::Array(kids));
        }
        Some(existing) => dictionary.insert(
            "K".to_string(),
            PdfObject::Array(PdfArray(vec![existing, mcr])),
        ),
    }
}

fn set_parent_owner(
    reader: &mut PdfReader<Cursor<&[u8]>>,
    entries: &mut BTreeMap<i64, PdfObject>,
    key: i64,
    mcid: u32,
    owner: TaggedPdfObjectRef,
) -> Result<()> {
    let entry = entries
        .entry(key)
        .or_insert_with(|| PdfObject::Array(PdfArray::new()));
    if let Some((number, generation)) = entry.as_reference() {
        *entry = reader.get_object(number, generation)?.clone();
    }
    let array = match entry {
        PdfObject::Array(array) => array,
        _ => return Err(invalid("page ParentTree entry is not an owner array")),
    };
    let index = usize::try_from(mcid).map_err(|_| invalid("MCID does not fit in memory"))?;
    array.0.resize(index + 1, PdfObject::Null);
    array.0[index] = PdfObject::Reference(owner.object_number, owner.generation);
    Ok(())
}

fn write_flat_parent_tree(dictionary: &mut PdfDictionary, entries: &BTreeMap<i64, PdfObject>) {
    let mut nums = Vec::with_capacity(entries.len() * 2);
    for (key, value) in entries {
        nums.push(PdfObject::Integer(*key));
        nums.push(value.clone());
    }
    dictionary.0.remove(&PdfName("Kids".to_string()));
    dictionary.insert("Nums".to_string(), PdfObject::Array(PdfArray(nums)));
    if let (Some(first), Some(last)) = (entries.keys().next(), entries.keys().next_back()) {
        dictionary.insert(
            "Limits".to_string(),
            PdfObject::Array(PdfArray(vec![
                PdfObject::Integer(*first),
                PdfObject::Integer(*last),
            ])),
        );
    } else {
        dictionary.0.remove(&PdfName("Limits".to_string()));
    }
}

fn invalid(message: impl Into<String>) -> PdfError {
    PdfError::InvalidStructure(format!("tagged-PDF edit: {}", message.into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pdf(objects: &[&str]) -> Vec<u8> {
        let mut bytes = b"%PDF-1.7\n".to_vec();
        let mut offsets = vec![0];
        for (index, object) in objects.iter().enumerate() {
            offsets.push(bytes.len());
            bytes.extend_from_slice(format!("{} 0 obj\n{object}\nendobj\n", index + 1).as_bytes());
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

    #[test]
    fn association_resolves_an_indirect_parent_owner_array() {
        let base = pdf(&[
            "<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 5 0 R /Lang (en-US) >>",
            "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] /StructParents 0 /Contents 4 0 R >>",
            "<< /Length 44 >>\nstream\n/P <</MCID 0>> BDC EMC /P <</MCID 1>> BDC EMC\nendstream",
            "<< /Type /StructTreeRoot /K 6 0 R /ParentTree 7 0 R >>",
            "<< /Type /StructElem /S /P /P 5 0 R /Pg 3 0 R /K 0 >>",
            "<< /Nums [0 8 0 R] >>",
            "[6 0 R]",
        ]);
        let mutation = TaggedPdfMutation::AssociateMcid {
            element: TaggedPdfObjectRef::from((6, 0)),
            page: TaggedPdfObjectRef::from((3, 0)),
            mcid: 1,
        };
        let update = IncrementalTaggedPdfEditor::new(&base)
            .apply(&[mutation])
            .unwrap();
        assert!(
            update.validation_after.valid,
            "{:?}",
            update.validation_after.findings
        );
        assert_eq!(
            update.validation_after.parent_tree.get(&0),
            Some(&PdfObject::Array(PdfArray(vec![
                PdfObject::Reference(6, 0),
                PdfObject::Reference(6, 0),
            ])))
        );
    }
}
