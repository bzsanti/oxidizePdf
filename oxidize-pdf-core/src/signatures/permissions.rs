//! Certification-policy checks shared by incremental editors.

use crate::error::PdfError;
use crate::parser::{PdfDictionary, PdfObject, PdfReader};
use crate::signatures::ByteRange;
use std::io::{Read, Seek};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)] // Additional variants are the shared contract for upcoming editors.
pub(crate) enum IncrementalModification {
    PageTreeReorder,
    PageTreeMutation,
    FormFill,
    AddSignature,
    AddAnnotation,
    OcrTextLayer,
    TaggedStructure,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DocMdpPermission {
    NoChanges,
    FormFillAndSign,
    FormFillSignAndAnnotate,
}

/// Enforce the effective certification policy for an incremental edit.
///
/// Approval signatures do not establish a DocMDP policy. Certification
/// metadata is accepted only when the catalog and the signature's transform
/// reference identify one unambiguous permission level.
pub(crate) fn ensure_modification_allowed<R: Read + Seek>(
    reader: &mut PdfReader<R>,
    catalog: &PdfDictionary,
    modification: IncrementalModification,
) -> Result<(), PdfError> {
    let catalog_certification = catalog_certification_reference(reader, catalog)?;
    let mut discovered = Vec::new();

    for reference in reader.object_references() {
        let object = reader
            .get_object(reference.0, reference.1)
            .map_err(|error| invalid_policy(format!("inspect signature object: {error}")))?
            .clone();
        let mut signatures = Vec::new();
        let top_level_signature = signature_dictionary(&object).is_some();
        collect_signature_dictionaries(&object, 0, &mut signatures)?;
        for (index, dictionary) in signatures.into_iter().enumerate() {
            if let Some((permission, byte_range)) = parse_docmdp_permission(reader, &dictionary)? {
                let location = (top_level_signature && index == 0)
                    .then_some(reference)
                    .ok_or_else(|| {
                        invalid_policy("a DocMDP signature must be an indirect object")
                    })?;
                ensure_definition_is_signed(reader, location, &byte_range)?;
                discovered.push((Some(location), permission));
            }
        }
    }

    match catalog_certification {
        None if discovered.is_empty() => Ok(()),
        None => Err(invalid_policy(
            "a DocMDP transform exists without catalog /Perms /DocMDP",
        )),
        Some(reference) => {
            if discovered.len() != 1 || discovered[0].0 != Some(reference) {
                return Err(invalid_policy(
                    "catalog /Perms and discovered DocMDP transforms are inconsistent",
                ));
            }
            enforce_permission(modification, discovered[0].1)
        }
    }
}

fn catalog_certification_reference<R: Read + Seek>(
    reader: &mut PdfReader<R>,
    catalog: &PdfDictionary,
) -> Result<Option<(u32, u16)>, PdfError> {
    let Some(perms) = catalog.get("Perms") else {
        return Ok(None);
    };
    let perms = resolve_dictionary(reader, perms, "catalog /Perms")?;
    let Some(docmdp) = perms.get("DocMDP") else {
        return Ok(None);
    };
    let reference = docmdp
        .as_reference()
        .ok_or_else(|| invalid_policy("catalog /Perms /DocMDP must be an indirect reference"))?;
    let signature = reader
        .get_object(reference.0, reference.1)
        .map_err(|error| invalid_policy(format!("resolve certification signature: {error}")))?;
    if signature_dictionary(signature).is_none() {
        return Err(invalid_policy(
            "catalog /Perms /DocMDP does not reference a signature dictionary",
        ));
    }
    Ok(Some(reference))
}

fn signature_dictionary(object: &PdfObject) -> Option<&PdfDictionary> {
    let dictionary = match object {
        PdfObject::Dictionary(dictionary) => dictionary,
        PdfObject::Stream(stream) => &stream.dict,
        _ => return None,
    };
    let is_signature = dictionary.get_type() == Some("Sig")
        || (dictionary.contains_key("ByteRange") && dictionary.contains_key("Contents"));
    is_signature.then_some(dictionary)
}

fn collect_signature_dictionaries(
    object: &PdfObject,
    depth: usize,
    signatures: &mut Vec<PdfDictionary>,
) -> Result<(), PdfError> {
    if depth > 128 {
        return Err(invalid_policy(
            "object nesting is too deep to inspect safely for signatures",
        ));
    }
    if let Some(signature) = signature_dictionary(object) {
        signatures.push(signature.clone());
        return Ok(());
    }
    match object {
        PdfObject::Array(array) => {
            for value in &array.0 {
                collect_signature_dictionaries(value, depth + 1, signatures)?;
            }
        }
        PdfObject::Dictionary(dictionary) => {
            for value in dictionary.0.values() {
                collect_signature_dictionaries(value, depth + 1, signatures)?;
            }
        }
        PdfObject::Stream(stream) => {
            for value in stream.dict.0.values() {
                collect_signature_dictionaries(value, depth + 1, signatures)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn parse_docmdp_permission<R: Read + Seek>(
    reader: &mut PdfReader<R>,
    signature: &PdfDictionary,
) -> Result<Option<(DocMdpPermission, ByteRange)>, PdfError> {
    let Some(references) = signature.get("Reference") else {
        return Ok(None);
    };
    let references = resolve_array(reader, references, "signature /Reference")?;
    let mut permission = None;
    for item in &references {
        let transform = resolve_dictionary(reader, item, "signature transform reference")?;
        let Some(method) = transform
            .get("TransformMethod")
            .and_then(PdfObject::as_name)
        else {
            return Err(invalid_policy(
                "signature transform reference has no /TransformMethod name",
            ));
        };
        if method.0 != "DocMDP" {
            continue;
        }
        if permission.is_some() {
            return Err(invalid_policy(
                "certification signature contains multiple DocMDP transforms",
            ));
        }
        if let Some(kind) = transform.get("Type").and_then(PdfObject::as_name) {
            if kind.0 != "SigRef" {
                return Err(invalid_policy("DocMDP transform /Type must be /SigRef"));
            }
        }
        let params = transform
            .get("TransformParams")
            .ok_or_else(|| invalid_policy("DocMDP transform has no /TransformParams"))?;
        let params = resolve_dictionary(reader, params, "DocMDP /TransformParams")?;
        if let Some(kind) = params.get("Type").and_then(PdfObject::as_name) {
            if kind.0 != "TransformParams" {
                return Err(invalid_policy(
                    "DocMDP /TransformParams has an invalid /Type",
                ));
            }
        }
        if let Some(version) = params.get("V").and_then(PdfObject::as_name) {
            if version.0 != "1.2" {
                return Err(invalid_policy(format!(
                    "unsupported DocMDP transform parameter version /{}",
                    version.0
                )));
            }
        }
        permission = Some(match params.get("P").and_then(PdfObject::as_integer) {
            Some(1) => DocMdpPermission::NoChanges,
            Some(2) => DocMdpPermission::FormFillAndSign,
            Some(3) => DocMdpPermission::FormFillSignAndAnnotate,
            Some(value) => {
                return Err(invalid_policy(format!(
                    "unsupported DocMDP permission level {value}"
                )))
            }
            None => {
                return Err(invalid_policy(
                    "DocMDP /TransformParams requires integer /P",
                ))
            }
        });
    }
    permission
        .map(|permission| {
            validate_certification_signature(signature).map(|range| (permission, range))
        })
        .transpose()
}

fn validate_certification_signature(signature: &PdfDictionary) -> Result<ByteRange, PdfError> {
    let byte_range = signature
        .get("ByteRange")
        .and_then(PdfObject::as_array)
        .ok_or_else(|| invalid_policy("certification signature requires an array /ByteRange"))?;
    let values: Vec<_> = byte_range
        .0
        .iter()
        .map(|value| {
            value.as_integer().ok_or_else(|| {
                invalid_policy("certification signature /ByteRange must contain only integers")
            })
        })
        .collect::<Result<_, _>>()?;
    let range = ByteRange::from_array(&values)
        .map_err(|error| invalid_policy(format!("invalid certification /ByteRange: {error}")))?;
    range
        .validate()
        .map_err(|error| invalid_policy(format!("invalid certification /ByteRange: {error}")))?;
    if signature
        .get("Contents")
        .and_then(PdfObject::as_string)
        .is_none()
    {
        return Err(invalid_policy(
            "certification signature requires string /Contents",
        ));
    }
    Ok(range)
}

fn ensure_definition_is_signed<R: Read + Seek>(
    reader: &PdfReader<R>,
    reference: (u32, u16),
    byte_range: &ByteRange,
) -> Result<(), PdfError> {
    let offset = reader
        .object_storage_offset(reference.0)
        .ok_or_else(|| invalid_policy("cannot locate the certification signature definition"))?;
    let covered = byte_range.ranges().iter().any(|(start, length)| {
        start
            .checked_add(*length)
            .map(|end| offset >= *start && offset < end)
            .unwrap_or(false)
    });
    if !covered {
        return Err(invalid_policy(
            "the latest certification signature definition is outside its signed byte ranges",
        ));
    }
    Ok(())
}

fn resolve_dictionary<R: Read + Seek>(
    reader: &mut PdfReader<R>,
    value: &PdfObject,
    role: &str,
) -> Result<PdfDictionary, PdfError> {
    let object = resolve_object(reader, value, role)?;
    object
        .as_dict()
        .cloned()
        .ok_or_else(|| invalid_policy(format!("{role} must be a dictionary")))
}

fn resolve_array<R: Read + Seek>(
    reader: &mut PdfReader<R>,
    value: &PdfObject,
    role: &str,
) -> Result<Vec<PdfObject>, PdfError> {
    let object = resolve_object(reader, value, role)?;
    object
        .as_array()
        .map(|array| array.0.clone())
        .ok_or_else(|| invalid_policy(format!("{role} must be an array")))
}

fn resolve_object<R: Read + Seek>(
    reader: &mut PdfReader<R>,
    value: &PdfObject,
    role: &str,
) -> Result<PdfObject, PdfError> {
    match value {
        PdfObject::Reference(number, generation) => reader
            .get_object(*number, *generation)
            .cloned()
            .map_err(|error| invalid_policy(format!("resolve {role}: {error}"))),
        object => Ok(object.clone()),
    }
}

fn enforce_permission(
    modification: IncrementalModification,
    permission: DocMdpPermission,
) -> Result<(), PdfError> {
    let description = match permission {
        DocMdpPermission::NoChanges => "P=1 (no changes)",
        DocMdpPermission::FormFillAndSign => "P=2 (form filling and signing)",
        DocMdpPermission::FormFillSignAndAnnotate => "P=3 (form filling, signing, and annotations)",
    };
    let allowed = match permission {
        DocMdpPermission::NoChanges => false,
        DocMdpPermission::FormFillAndSign => matches!(
            modification,
            IncrementalModification::FormFill | IncrementalModification::AddSignature
        ),
        DocMdpPermission::FormFillSignAndAnnotate => matches!(
            modification,
            IncrementalModification::FormFill
                | IncrementalModification::AddSignature
                | IncrementalModification::AddAnnotation
        ),
    };
    if allowed {
        return Ok(());
    }
    let edit = match modification {
        IncrementalModification::PageTreeReorder => "page-tree reordering",
        IncrementalModification::PageTreeMutation => "page-tree mutation",
        IncrementalModification::FormFill => "form filling",
        IncrementalModification::AddSignature => "adding signatures",
        IncrementalModification::AddAnnotation => "adding annotations",
        IncrementalModification::OcrTextLayer => "adding an OCR text layer",
        IncrementalModification::TaggedStructure => "editing tagged-PDF structure",
    };
    Err(PdfError::PermissionDenied(format!(
        "DocMDP {description} does not permit {edit}"
    )))
}

fn invalid_policy(message: impl Into<String>) -> PdfError {
    PdfError::InvalidStructure(format!("invalid DocMDP policy: {}", message.into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn permission_levels_classify_reusable_incremental_edits() {
        assert!(enforce_permission(
            IncrementalModification::FormFill,
            DocMdpPermission::NoChanges
        )
        .is_err());
        assert!(enforce_permission(
            IncrementalModification::FormFill,
            DocMdpPermission::FormFillAndSign
        )
        .is_ok());
        assert!(enforce_permission(
            IncrementalModification::AddSignature,
            DocMdpPermission::FormFillAndSign
        )
        .is_ok());
        assert!(enforce_permission(
            IncrementalModification::AddAnnotation,
            DocMdpPermission::FormFillAndSign
        )
        .is_err());
        assert!(enforce_permission(
            IncrementalModification::AddAnnotation,
            DocMdpPermission::FormFillSignAndAnnotate
        )
        .is_ok());
    }

    #[test]
    fn page_tree_reordering_is_forbidden_at_every_permission_level() {
        for permission in [
            DocMdpPermission::NoChanges,
            DocMdpPermission::FormFillAndSign,
            DocMdpPermission::FormFillSignAndAnnotate,
        ] {
            assert!(
                enforce_permission(IncrementalModification::PageTreeReorder, permission).is_err()
            );
        }
    }

    #[test]
    fn page_tree_mutations_are_forbidden_at_every_permission_level() {
        for permission in [
            DocMdpPermission::NoChanges,
            DocMdpPermission::FormFillAndSign,
            DocMdpPermission::FormFillSignAndAnnotate,
        ] {
            assert!(
                enforce_permission(IncrementalModification::PageTreeMutation, permission).is_err()
            );
        }
    }

    #[test]
    fn ocr_page_content_is_forbidden_at_every_permission_level() {
        for permission in [
            DocMdpPermission::NoChanges,
            DocMdpPermission::FormFillAndSign,
            DocMdpPermission::FormFillSignAndAnnotate,
        ] {
            assert!(enforce_permission(IncrementalModification::OcrTextLayer, permission).is_err());
        }
    }

    #[test]
    fn tagged_structure_edits_are_forbidden_at_every_permission_level() {
        for permission in [
            DocMdpPermission::NoChanges,
            DocMdpPermission::FormFillAndSign,
            DocMdpPermission::FormFillSignAndAnnotate,
        ] {
            assert!(
                enforce_permission(IncrementalModification::TaggedStructure, permission).is_err()
            );
        }
    }
}
