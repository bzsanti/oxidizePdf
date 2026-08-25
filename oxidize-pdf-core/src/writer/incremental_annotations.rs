//! Shared page and annotation discovery for incremental annotation editors.

use crate::error::{PdfError, Result};
use crate::parser::objects::{PdfDictionary, PdfObject};
use crate::parser::{PdfDocument, PdfReader};
use std::collections::HashMap;
use std::io::Cursor;

#[derive(Clone)]
pub(super) struct PageState {
    pub(super) reference: (u32, u16),
    pub(super) dictionary: PdfDictionary,
    pub(super) bounds: [f64; 4],
    pub(super) container: AnnotationContainer,
    pub(super) annotations: Vec<PdfObject>,
}

#[derive(Clone, Copy)]
pub(super) enum AnnotationContainer {
    Page,
    Indirect((u32, u16)),
}

pub(super) struct AnnotationState {
    pub(super) page_index: u32,
    pub(super) dictionary: PdfDictionary,
}

pub(super) struct AnnotationSnapshot {
    pub(super) pages: Vec<PageState>,
    pub(super) annotations: HashMap<(u32, u16), AnnotationState>,
}

impl AnnotationSnapshot {
    pub(super) fn parse(bytes: &[u8], target_subtype: &str, feature: &str) -> Result<Self> {
        let reader = PdfReader::new(Cursor::new(bytes))
            .map_err(|e| PdfError::InvalidStructure(format!("parse base PDF: {e}")))?;
        if reader.is_encrypted() {
            return Err(PdfError::PermissionDenied(format!(
                "incremental {feature} editing is not supported on encrypted PDFs"
            )));
        }
        let document = PdfDocument::new(reader);
        let page_count = document
            .page_count()
            .map_err(|e| PdfError::InvalidStructure(format!("read page tree: {e}")))?;
        let mut pages = Vec::with_capacity(page_count as usize);
        let mut annotations = HashMap::new();
        let mut indirect_annots_pages = HashMap::new();

        for page_index in 0..page_count {
            let page = document
                .get_page(page_index)
                .map_err(|e| PdfError::InvalidStructure(format!("read page {page_index}: {e}")))?;
            let bounds = page.crop_box.unwrap_or(page.media_box);
            let (container, values) = match page.dict.get("Annots") {
                None => (AnnotationContainer::Page, Vec::new()),
                Some(PdfObject::Array(array)) => (AnnotationContainer::Page, array.0.clone()),
                Some(PdfObject::Reference(number, generation)) => {
                    if let Some(first_page) =
                        indirect_annots_pages.insert((*number, *generation), page_index)
                    {
                        return Err(PdfError::InvalidStructure(format!(
                            "page /Annots array {number} {generation} is shared by multiple pages ({first_page} and {page_index})"
                        )));
                    }
                    let object = document.get_object(*number, *generation).map_err(|e| {
                        PdfError::InvalidStructure(format!("resolve page /Annots: {e}"))
                    })?;
                    let array = object.as_array().ok_or_else(|| {
                        PdfError::InvalidStructure("indirect /Annots is not an array".to_string())
                    })?;
                    (
                        AnnotationContainer::Indirect((*number, *generation)),
                        array.0.clone(),
                    )
                }
                Some(_) => {
                    return Err(PdfError::InvalidStructure(
                        "page /Annots must be an array or indirect array".to_string(),
                    ))
                }
            };

            for value in &values {
                match value {
                    PdfObject::Reference(number, generation) => {
                        let object = document.get_object(*number, *generation).map_err(|e| {
                            PdfError::InvalidStructure(format!(
                                "resolve annotation {number} {generation}: {e}"
                            ))
                        })?;
                        let dictionary = object.as_dict().cloned().ok_or_else(|| {
                            PdfError::InvalidStructure(format!(
                                "annotation {number} {generation} is not a dictionary"
                            ))
                        })?;
                        let id = (*number, *generation);
                        if annotations
                            .get(&id)
                            .is_some_and(|existing: &AnnotationState| {
                                existing.page_index != page_index
                            })
                        {
                            return Err(PdfError::InvalidStructure(format!(
                                "annotation {number} {generation} is referenced from multiple pages"
                            )));
                        }
                        annotations.insert(
                            id,
                            AnnotationState {
                                page_index,
                                dictionary,
                            },
                        );
                    }
                    PdfObject::Dictionary(dictionary)
                        if subtype(dictionary) == Some(target_subtype) =>
                    {
                        return Err(PdfError::InvalidStructure(format!(
                            "inline /{target_subtype} annotations have no stable object identity"
                        )));
                    }
                    _ => {}
                }
            }
            pages.push(PageState {
                reference: page.obj_ref,
                dictionary: page.dict,
                bounds,
                container,
                annotations: values,
            });
        }
        Ok(Self { pages, annotations })
    }
}

pub(super) fn subtype(dictionary: &PdfDictionary) -> Option<&str> {
    dictionary
        .get("Subtype")
        .and_then(PdfObject::as_name)
        .map(|name| name.0.as_str())
}
