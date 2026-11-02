//! Document permissions from signed signature-field lock dictionaries.

use super::{
    enforce_permission, ensure_definition_is_signed, ensure_docmdp_allowed, invalid_policy,
    resolve_array, resolve_dictionary, resolve_object, validate_certification_signature,
    DocMdpPermission, IncrementalModification,
};
use crate::error::PdfError;
use crate::parser::{PdfDictionary, PdfObject, PdfReader};
use std::collections::HashSet;
use std::io::{Read, Seek};

pub(super) fn enforce_history<R: Read + Seek>(
    reader: &mut PdfReader<R>,
    catalog: &PdfDictionary,
    modification: IncrementalModification,
) -> Result<(), PdfError> {
    let revisions = reader.xref_revisions();
    let Some(first) = revisions.first() else {
        ensure_docmdp_allowed(reader, catalog, modification)?;
        return enforce_fields(reader, modification);
    };
    let mut snapshot = reader
        .at_revision(first.xref_offset)
        .map_err(|e| invalid_policy(format!("read signed permission history: {e}")))?;
    for (index, revision) in revisions.iter().enumerate() {
        if index != 0 {
            snapshot
                .advance_revision(revision.xref_offset)
                .map_err(|e| invalid_policy(format!("advance signed permission history: {e}")))?;
        }
        let catalog = snapshot
            .catalog()
            .map_err(|e| invalid_policy(format!("read historical catalog: {e}")))?
            .clone();
        ensure_docmdp_allowed(&mut snapshot, &catalog, modification)?;
        enforce_fields(&mut snapshot, modification)?;
    }
    Ok(())
}

fn enforce_fields<R: Read + Seek>(
    reader: &mut PdfReader<R>,
    modification: IncrementalModification,
) -> Result<(), PdfError> {
    let catalog = reader
        .catalog()
        .map_err(|e| invalid_policy(e.to_string()))?
        .clone();
    let Some(form) = catalog.get("AcroForm") else {
        return Ok(());
    };
    let form = resolve_dictionary(reader, form, "AcroForm")?;
    let Some(fields) = form.get("Fields") else {
        return Ok(());
    };
    let mut stack: Vec<_> = resolve_array(reader, fields, "AcroForm /Fields")?
        .into_iter()
        .map(|field| (field, false, 0usize, Vec::new()))
        .collect();
    let mut seen = HashSet::new();
    while let Some((object, inherited_sig, depth, mut ancestors)) = stack.pop() {
        if depth > 128 {
            return Err(invalid_policy("signature field tree is too deep"));
        }
        let location = object.as_reference();
        if let Some(reference) = location {
            if ancestors.contains(&reference) {
                return Err(invalid_policy("cyclic signature field reference"));
            }
            // Shared children can inherit different field types through aliases.
            if !seen.insert((reference, inherited_sig)) {
                continue;
            }
            ancestors.push(reference);
        }
        let field = resolve_dictionary(reader, &object, "signature field")?;
        let is_sig = match field.get("FT") {
            Some(value) => {
                let kind = resolve_object(reader, value, "field /FT")?;
                kind.as_name()
                    .ok_or_else(|| invalid_policy("field /FT must be a name"))?
                    .0
                    == "Sig"
            }
            None => inherited_sig,
        };
        if is_sig {
            enforce_field(reader, location, &field, modification)?;
        }
        if let Some(kids) = field.get("Kids") {
            for child in resolve_array(reader, kids, "signature field /Kids")? {
                stack.push((child, is_sig, depth + 1, ancestors.clone()));
            }
        }
    }
    Ok(())
}

fn enforce_field<R: Read + Seek>(
    reader: &mut PdfReader<R>,
    field_ref: Option<(u32, u16)>,
    field: &PdfDictionary,
    modification: IncrementalModification,
) -> Result<(), PdfError> {
    let Some(value) = field.get("V").filter(|v| !matches!(v, PdfObject::Null)) else {
        return Ok(());
    };
    let Some(lock_value) = field.get("Lock") else {
        return Ok(());
    };
    let lock = resolve_dictionary(reader, lock_value, "signature field /Lock")?;
    let Some(permission_value) = lock.get("P") else {
        return Ok(());
    };
    if let Some(value) = lock.get("Type") {
        let kind = resolve_object(reader, value, "signature field /Lock /Type")?;
        if kind.as_name().is_none_or(|name| name.0 != "SigFieldLock") {
            return Err(invalid_policy(
                "signature field /Lock /Type must be SigFieldLock",
            ));
        }
    }
    let permission =
        match resolve_object(reader, permission_value, "signature field /Lock /P")?.as_integer() {
            Some(1) => DocMdpPermission::NoChanges,
            Some(2) => DocMdpPermission::FormFillAndSign,
            Some(3) => DocMdpPermission::FormFillSignAndAnnotate,
            _ => {
                return Err(invalid_policy(
                    "signature field /Lock /P must be integer 1, 2 or 3",
                ))
            }
        };
    let signature = resolve_dictionary(reader, value, "signed field /V")?;
    let range = validate_certification_signature(&signature)?;
    let location = field_ref
        .ok_or_else(|| invalid_policy("a signed field lock requires an indirect field"))?;
    ensure_definition_is_signed(reader, location, &range)?;
    let signature_ref = value
        .as_reference()
        .ok_or_else(|| invalid_policy("signed field /V must be indirect"))?;
    ensure_definition_is_signed(reader, signature_ref, &range)?;
    for object in [lock_value, permission_value]
        .into_iter()
        .chain(lock.get("Type"))
    {
        if let Some(reference) = object.as_reference() {
            ensure_definition_is_signed(reader, reference, &range)?;
        }
    }
    enforce_permission(modification, permission)
}
