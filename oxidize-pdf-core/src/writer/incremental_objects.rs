//! Public, transactional object-level incremental editing.

use super::IncrementalUpdate;
use crate::error::{PdfError, Result};
use crate::parser::{ParseOptions, PdfDictionary, PdfObject, PdfReader};
use crate::signatures::{ensure_modification_allowed, IncrementalModification};
use std::collections::BTreeSet;
use std::io::Cursor;

/// Append caller-supplied PDF objects without rewriting the original bytes.
///
/// Unrelated catalog entries, object references and generations are preserved.
/// This is a low-level API: callers own the semantics of supplied objects and
/// evidence (including any DSS construction/merging). It does not validate
/// signatures, timestamps, trust or signing profiles.
///
/// Generic edits cannot be classified as form filling, signing or annotations:
/// **all DocMDP certification levels are rejected**. Use the dedicated editors
/// for permitted operations. Prefix preservation alone does not authorize an edit.
/// Encrypted documents, FieldMDP locks and malformed certification policies are rejected.
///
/// Changes remain in memory; only [`Self::finish`] returns output. An error never
/// writes a partial file. Dropping the editor discards the entire transaction.
pub struct IncrementalObjectEditor<'a> {
    update: IncrementalUpdate<'a>,
    catalog: PdfDictionary,
    root: (u32, u16),
    known: BTreeSet<(u32, u16)>,
    allocated: BTreeSet<(u32, u16)>,
    written: BTreeSet<(u32, u16)>,
    protected: BTreeSet<(u32, u16)>,
    catalog_changed: bool,
}

impl<'a> IncrementalObjectEditor<'a> {
    /// Open an unencrypted PDF with valid xrefs and no certification policy.
    pub fn new(base: &'a [u8]) -> Result<Self> {
        let mut reader = PdfReader::new_with_options(Cursor::new(base), ParseOptions::strict())
            .map_err(|e| invalid(format!("parse base PDF: {e}")))?;
        if reader.is_encrypted() {
            return Err(PdfError::PermissionDenied(
                "incremental object editing does not support encrypted PDFs".into(),
            ));
        }
        let catalog = reader
            .catalog()
            .map_err(|e| invalid(e.to_string()))?
            .clone();
        ensure_modification_allowed(
            &mut reader,
            &catalog,
            IncrementalModification::GenericObjects,
        )?;
        let root = reader
            .trailer()
            .root()
            .map_err(|e| invalid(e.to_string()))?;
        let known: BTreeSet<_> = reader.object_references().into_iter().collect();
        let mut protected = BTreeSet::new();
        for id in &known {
            let object = reader
                .get_object(id.0, id.1)
                .map_err(|e| invalid(e.to_string()))?;
            if let Some(dict) = object.as_dict() {
                if matches!(
                    dict.get_type(),
                    Some("Sig" | "DocTimeStamp" | "XRef" | "ObjStm")
                ) || (dict.contains_key("ByteRange") && dict.contains_key("Contents"))
                {
                    protected.insert(*id);
                }
            }
        }
        let update = IncrementalUpdate::from_reader(base, &reader)?;
        if known
            .iter()
            .any(|id| id.0 >= reader.trailer().size().unwrap_or(0))
        {
            return Err(invalid("trailer /Size does not cover existing objects"));
        }
        Ok(Self {
            update,
            catalog,
            root,
            known,
            allocated: BTreeSet::new(),
            written: BTreeSet::new(),
            protected,
            catalog_changed: false,
        })
    }

    /// Current catalog snapshot, including staged entry changes.
    pub fn catalog(&self) -> &PdfDictionary {
        &self.catalog
    }

    /// Reserve a fresh generation-zero reference. Every reservation must be
    /// populated with [`Self::replace`] before finishing. Forward references work.
    pub fn allocate_id(&mut self) -> Result<(u32, u16)> {
        let id = self.update.allocate_id()?;
        self.known.insert(id);
        self.allocated.insert(id);
        Ok(id)
    }

    /// Stage one existing or allocated object, keeping its exact generation.
    /// Duplicate writes, unknown/stale references and replacement of signature
    /// or storage dictionaries are rejected. Use catalog entry methods for Root.
    pub fn replace(&mut self, id: (u32, u16), object: PdfObject) -> Result<()> {
        if id == self.root || self.protected.contains(&id) || !self.known.contains(&id) {
            return Err(invalid("object reference is unknown, stale, or protected"));
        }
        self.validate_object(&object, 0, true)?;
        self.update.replace(id, object)?;
        self.written.insert(id);
        Ok(())
    }

    /// Set one catalog entry, preserving every other entry. Type and Perms are
    /// engine-controlled here; this API cannot establish certification policy.
    pub fn set_catalog_entry(&mut self, key: impl Into<String>, value: PdfObject) -> Result<()> {
        let key = key.into();
        validate_key(&key)?;
        self.validate_object(&value, 0, false)?;
        self.catalog.insert(key, value);
        self.catalog_changed = true;
        Ok(())
    }

    /// Remove an optional catalog entry. Required Type/Pages and Perms cannot
    /// be removed through this method.
    pub fn remove_catalog_entry(&mut self, key: &str) -> Result<()> {
        validate_key(key)?;
        if key == "Pages" {
            return Err(invalid("cannot remove required catalog /Pages"));
        }
        self.catalog.0.remove(&crate::parser::PdfName(key.into()));
        self.catalog_changed = true;
        Ok(())
    }

    /// Return the complete PDF with one new revision, or the original bytes for
    /// an empty transaction. All reserved objects must have been supplied.
    pub fn finish(mut self) -> Result<Vec<u8>> {
        if !self.allocated.is_subset(&self.written) {
            return Err(invalid("an allocated object has no definition"));
        }
        if self.catalog_changed {
            self.update
                .replace(self.root, PdfObject::Dictionary(self.catalog))?;
        }
        self.update.finish()
    }

    fn validate_object(&self, object: &PdfObject, depth: usize, indirect: bool) -> Result<()> {
        if depth > 128 {
            return Err(invalid("object nesting exceeds 128"));
        }
        match object {
            PdfObject::Reference(n, g) if !self.known.contains(&(*n, *g)) => {
                return Err(invalid("unknown or stale object reference"))
            }
            PdfObject::Real(v) if !v.is_finite() => return Err(invalid("non-finite PDF number")),
            PdfObject::Array(a) => {
                for v in &a.0 {
                    self.validate_object(v, depth + 1, false)?;
                }
            }
            PdfObject::Dictionary(d) => self.validate_dictionary(d, depth)?,
            PdfObject::Stream(s) => {
                if !indirect {
                    return Err(invalid("streams must be indirect objects"));
                }
                self.validate_dictionary(&s.dict, depth)?;
            }
            _ => {}
        }
        Ok(())
    }

    fn validate_dictionary(&self, dict: &PdfDictionary, depth: usize) -> Result<()> {
        if matches!(
            dict.get_type(),
            Some("Sig" | "DocTimeStamp" | "XRef" | "ObjStm")
        ) || dict.contains_key("ByteRange")
            || dict.contains_key("TransformMethod")
        {
            return Err(invalid(
                "use dedicated APIs for signatures and storage dictionaries",
            ));
        }
        for value in dict.0.values() {
            self.validate_object(value, depth + 1, false)?;
        }
        Ok(())
    }
}

fn validate_key(key: &str) -> Result<()> {
    if key.is_empty() || key.contains('\0') || matches!(key, "Type" | "Perms") {
        return Err(invalid("invalid or protected catalog entry"));
    }
    Ok(())
}

fn invalid(message: impl Into<String>) -> PdfError {
    PdfError::InvalidStructure(message.into())
}
