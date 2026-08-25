//! Incremental editing of standard `/Text` annotations in existing PDFs.

use super::incremental_annotations::{subtype, AnnotationContainer, AnnotationSnapshot, PageState};
use super::incremental_update::IncrementalUpdate;
use crate::error::{PdfError, Result};
use crate::geometry::Point;
use crate::parser::objects::{PdfArray, PdfDictionary, PdfName, PdfObject, PdfString};
use std::collections::{HashMap, HashSet};

const NOTE_SIZE: f64 = 20.0;

/// Stable indirect-object identity of a text note.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextNoteId {
    /// PDF object number.
    pub object_number: u32,
    /// PDF generation number.
    pub generation_number: u16,
}

impl TextNoteId {
    /// Construct an identity from a PDF indirect reference.
    pub const fn new(object_number: u32, generation_number: u16) -> Self {
        Self {
            object_number,
            generation_number,
        }
    }

    fn tuple(self) -> (u32, u16) {
        (self.object_number, self.generation_number)
    }
}

/// A standard PDF text-note annotation.
#[derive(Debug, Clone, PartialEq)]
pub struct TextNote {
    /// Stable indirect-object identity.
    pub id: TextNoteId,
    /// Zero-based page index.
    pub page_index: u32,
    /// Lower-left point of the annotation rectangle.
    pub position: Point,
    /// Note contents decoded as Unicode.
    pub contents: String,
}

/// A requested change to the document's text notes.
#[derive(Debug, Clone, PartialEq)]
pub enum TextNoteMutation {
    /// Add a 20×20 point note to a page.
    Add {
        /// Zero-based target page.
        page_index: u32,
        /// Lower-left point of the new note.
        position: Point,
        /// Non-empty note contents.
        contents: String,
    },
    /// Move and edit an existing note while preserving its other keys.
    Update {
        /// Existing text-note identity.
        id: TextNoteId,
        /// New lower-left point. Existing width and height are preserved.
        position: Point,
        /// New non-empty contents.
        contents: String,
    },
    /// Remove an existing note reference from its page.
    Remove {
        /// Existing text-note identity.
        id: TextNoteId,
    },
}

/// Result of one atomic incremental text-note update.
#[derive(Debug, Clone)]
pub struct TextNoteUpdate {
    /// Complete PDF bytes. The original document is an exact prefix.
    pub pdf_bytes: Vec<u8>,
    /// Notes allocated by `Add`, in mutation order.
    pub added_notes: Vec<TextNote>,
}

/// Reads and atomically edits standard text notes in an existing PDF.
pub struct IncrementalTextNoteEditor<'a> {
    base_bytes: &'a [u8],
}

impl<'a> IncrementalTextNoteEditor<'a> {
    /// Create an editor over an existing PDF's bytes.
    pub fn new(base_bytes: &'a [u8]) -> Self {
        Self { base_bytes }
    }

    /// Read every indirect `/Text` annotation with its stable identity.
    ///
    /// # Errors
    ///
    /// Returns an error when the PDF is malformed or encrypted, or when a text
    /// annotation has no stable indirect identity or is shared by pages.
    pub fn notes(&self) -> Result<Vec<TextNote>> {
        let snapshot = Snapshot::parse(self.base_bytes)?;
        let mut notes: Vec<_> = snapshot
            .annotations
            .iter()
            .filter_map(|(id, annotation)| annotation.is_text.then(|| annotation.note(*id)))
            .collect();
        notes.sort_by_key(|note| {
            (
                note.page_index,
                note.id.object_number,
                note.id.generation_number,
            )
        });
        Ok(notes)
    }

    /// Validate and apply a batch as exactly one incremental revision.
    ///
    /// # Errors
    ///
    /// Returns an error for malformed or encrypted PDFs, invalid pages or note
    /// identities, conflicting mutations, invalid contents or coordinates, and
    /// exhausted indirect-object numbers.
    pub fn apply(&self, mutations: &[TextNoteMutation]) -> Result<TextNoteUpdate> {
        let mut snapshot = Snapshot::parse(self.base_bytes)?;
        if mutations.is_empty() {
            return Ok(TextNoteUpdate {
                pdf_bytes: self.base_bytes.to_vec(),
                added_notes: Vec::new(),
            });
        }
        validate_batch(&snapshot, mutations)?;
        let mut update = IncrementalUpdate::from_base(self.base_bytes)?;
        let mut added_notes = Vec::new();
        let mut changed_pages = HashSet::new();
        let mut rewritten_annotations: HashMap<TextNoteId, PdfDictionary> = HashMap::new();

        for mutation in mutations {
            match mutation {
                TextNoteMutation::Add {
                    page_index,
                    position,
                    contents,
                } => {
                    let id_tuple = update.allocate_id()?;
                    let id = TextNoteId::new(id_tuple.0, id_tuple.1);
                    let dictionary = new_note_dictionary(*position, contents);
                    update.replace(id_tuple, PdfObject::Dictionary(dictionary.clone()))?;
                    snapshot.pages[*page_index as usize]
                        .annotations
                        .push(PdfObject::Reference(id_tuple.0, id_tuple.1));
                    changed_pages.insert(*page_index);
                    added_notes.push(TextNote {
                        id,
                        page_index: *page_index,
                        position: *position,
                        contents: contents.clone(),
                    });
                }
                TextNoteMutation::Update {
                    id,
                    position,
                    contents,
                } => {
                    let annotation = target_text_note(&snapshot, *id)?;
                    let mut dictionary = annotation.dictionary.clone();
                    let width = annotation.rect[2] - annotation.rect[0];
                    let height = annotation.rect[3] - annotation.rect[1];
                    dictionary.insert("Rect".to_string(), rectangle(*position, width, height));
                    dictionary.insert("Contents".to_string(), pdf_text(contents));
                    rewritten_annotations.insert(*id, dictionary);
                }
                TextNoteMutation::Remove { id } => {
                    let page_index = target_text_note(&snapshot, *id)?.page_index;
                    let page = &mut snapshot.pages[page_index as usize];
                    page.annotations
                        .retain(|value| value.as_reference() != Some(id.tuple()));
                    changed_pages.insert(page_index);
                }
            }
        }

        for (id, dictionary) in rewritten_annotations {
            update.replace(id.tuple(), PdfObject::Dictionary(dictionary))?;
        }
        for page_index in changed_pages {
            let page = &mut snapshot.pages[page_index as usize];
            match page.container {
                AnnotationContainer::Page => {
                    page.dictionary.insert(
                        "Annots".to_string(),
                        PdfObject::Array(PdfArray(page.annotations.clone())),
                    );
                    update.replace(
                        page.reference,
                        PdfObject::Dictionary(page.dictionary.clone()),
                    )?;
                }
                AnnotationContainer::Indirect(id) => {
                    update.replace(id, PdfObject::Array(PdfArray(page.annotations.clone())))?;
                }
            }
        }

        Ok(TextNoteUpdate {
            pdf_bytes: update.finish()?,
            added_notes,
        })
    }
}

struct AnnotationState {
    page_index: u32,
    dictionary: PdfDictionary,
    rect: [f64; 4],
    contents: String,
    is_text: bool,
}

impl AnnotationState {
    fn note(&self, id: TextNoteId) -> TextNote {
        TextNote {
            id,
            page_index: self.page_index,
            position: Point::new(self.rect[0], self.rect[1]),
            contents: self.contents.clone(),
        }
    }
}

struct Snapshot {
    pages: Vec<PageState>,
    annotations: HashMap<TextNoteId, AnnotationState>,
}

impl Snapshot {
    fn parse(bytes: &[u8]) -> Result<Self> {
        let shared = AnnotationSnapshot::parse(bytes, "Text", "text-note")?;
        let mut annotations = HashMap::new();
        for ((number, generation), state) in shared.annotations {
            let is_text = subtype(&state.dictionary) == Some("Text");
            let rect = if is_text {
                parse_rectangle(&state.dictionary)?
            } else {
                [0.0; 4]
            };
            let contents = state
                .dictionary
                .get("Contents")
                .and_then(PdfObject::as_string)
                .map(PdfString::to_text)
                .unwrap_or_default();
            annotations.insert(
                TextNoteId::new(number, generation),
                AnnotationState {
                    page_index: state.page_index,
                    dictionary: state.dictionary,
                    rect,
                    contents,
                    is_text,
                },
            );
        }
        Ok(Self {
            pages: shared.pages,
            annotations,
        })
    }
}

fn validate_batch(snapshot: &Snapshot, mutations: &[TextNoteMutation]) -> Result<()> {
    let mut targeted = HashSet::new();
    for mutation in mutations {
        match mutation {
            TextNoteMutation::Add {
                page_index,
                position,
                contents,
            } => {
                let page = snapshot.pages.get(*page_index as usize).ok_or_else(|| {
                    PdfError::InvalidStructure(format!("page {page_index} does not exist"))
                })?;
                validate_contents(contents)?;
                validate_position(*position, NOTE_SIZE, NOTE_SIZE, page.bounds)?;
            }
            TextNoteMutation::Update {
                id,
                position,
                contents,
            } => {
                if !targeted.insert(*id) {
                    return Err(PdfError::InvalidStructure(format!(
                        "text note {} {} is targeted more than once",
                        id.object_number, id.generation_number
                    )));
                }
                let annotation = target_text_note(snapshot, *id)?;
                validate_contents(contents)?;
                validate_position(
                    *position,
                    annotation.rect[2] - annotation.rect[0],
                    annotation.rect[3] - annotation.rect[1],
                    snapshot.pages[annotation.page_index as usize].bounds,
                )?;
            }
            TextNoteMutation::Remove { id } => {
                if !targeted.insert(*id) {
                    return Err(PdfError::InvalidStructure(format!(
                        "text note {} {} is targeted more than once",
                        id.object_number, id.generation_number
                    )));
                }
                target_text_note(snapshot, *id)?;
            }
        }
    }
    Ok(())
}

fn target_text_note(snapshot: &Snapshot, id: TextNoteId) -> Result<&AnnotationState> {
    let annotation = snapshot.annotations.get(&id).ok_or_else(|| {
        PdfError::InvalidStructure(format!(
            "annotation {} {} does not exist",
            id.object_number, id.generation_number
        ))
    })?;
    if !annotation.is_text {
        return Err(PdfError::InvalidStructure(format!(
            "annotation {} {} is not a /Text annotation",
            id.object_number, id.generation_number
        )));
    }
    Ok(annotation)
}

fn validate_contents(contents: &str) -> Result<()> {
    if contents.trim().is_empty() {
        return Err(PdfError::InvalidStructure(
            "text-note contents must not be empty".to_string(),
        ));
    }
    Ok(())
}

fn validate_position(position: Point, width: f64, height: f64, bounds: [f64; 4]) -> Result<()> {
    if !position.x.is_finite()
        || !position.y.is_finite()
        || !width.is_finite()
        || !height.is_finite()
        || width <= 0.0
        || height <= 0.0
    {
        return Err(PdfError::InvalidStructure(
            "text-note coordinates and dimensions must be finite and positive".to_string(),
        ));
    }
    if position.x < bounds[0]
        || position.y < bounds[1]
        || position.x + width > bounds[2]
        || position.y + height > bounds[3]
    {
        return Err(PdfError::InvalidStructure(
            "text-note rectangle is outside the page bounds".to_string(),
        ));
    }
    Ok(())
}

fn parse_rectangle(dictionary: &PdfDictionary) -> Result<[f64; 4]> {
    let array = dictionary
        .get("Rect")
        .and_then(PdfObject::as_array)
        .ok_or_else(|| PdfError::InvalidStructure("/Text annotation has no /Rect".to_string()))?;
    if array.0.len() != 4 {
        return Err(PdfError::InvalidStructure(
            "/Text annotation /Rect must have four numbers".to_string(),
        ));
    }
    let mut result = [0.0; 4];
    for (index, value) in array.0.iter().enumerate() {
        result[index] = match value {
            PdfObject::Integer(number) => *number as f64,
            PdfObject::Real(number) if number.is_finite() => *number,
            _ => {
                return Err(PdfError::InvalidStructure(
                    "/Text annotation /Rect contains a non-finite or non-numeric value".to_string(),
                ))
            }
        };
    }
    Ok(result)
}

fn new_note_dictionary(position: Point, contents: &str) -> PdfDictionary {
    let mut dictionary = PdfDictionary::new();
    dictionary.insert(
        "Type".to_string(),
        PdfObject::Name(PdfName("Annot".to_string())),
    );
    dictionary.insert(
        "Subtype".to_string(),
        PdfObject::Name(PdfName("Text".to_string())),
    );
    dictionary.insert(
        "Rect".to_string(),
        rectangle(position, NOTE_SIZE, NOTE_SIZE),
    );
    dictionary.insert("Contents".to_string(), pdf_text(contents));
    dictionary
}

fn rectangle(position: Point, width: f64, height: f64) -> PdfObject {
    PdfObject::Array(PdfArray(vec![
        PdfObject::Real(position.x),
        PdfObject::Real(position.y),
        PdfObject::Real(position.x + width),
        PdfObject::Real(position.y + height),
    ]))
}

fn pdf_text(contents: &str) -> PdfObject {
    let mut bytes = vec![0xFE, 0xFF];
    for unit in contents.encode_utf16() {
        bytes.extend_from_slice(&unit.to_be_bytes());
    }
    PdfObject::String(PdfString(bytes))
}
