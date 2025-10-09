use crate::document::Document;
use crate::error::Result;
use crate::objects::{Dictionary, Object, ObjectId};
use crate::text::fonts::embedding::CjkFontType;
use crate::writer::{ObjectStreamConfig, ObjectStreamWriter, XRefStreamWriter};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::io::{BufWriter, Write};
use std::path::Path;

/// Configuration for PDF writer
#[derive(Debug, Clone)]
pub struct WriterConfig {
    /// Use XRef streams instead of traditional XRef tables (PDF 1.5+)
    pub use_xref_streams: bool,
    /// Use Object Streams for compressing multiple objects together (PDF 1.5+)
    pub use_object_streams: bool,
    /// PDF version to write (default: 1.7)
    pub pdf_version: String,
    /// Enable compression for streams (default: true)
    pub compress_streams: bool,
}

impl Default for WriterConfig {
    fn default() -> Self {
        Self {
            use_xref_streams: false,
            use_object_streams: false,
            pdf_version: "1.7".to_string(),
            compress_streams: true,
        }
    }
}

impl WriterConfig {
    /// Create a modern PDF 1.5+ configuration with all compression features enabled
    pub fn modern() -> Self {
        Self {
            use_xref_streams: true,
            use_object_streams: true,
            pdf_version: "1.5".to_string(),
            compress_streams: true,
        }
    }

    /// Create a legacy PDF 1.4 configuration without modern compression
    pub fn legacy() -> Self {
        Self {
            use_xref_streams: false,
            use_object_streams: false,
            pdf_version: "1.4".to_string(),
            compress_streams: true,
        }
    }
}

pub struct PdfWriter<W: Write> {
    writer: W,
    xref_positions: HashMap<ObjectId, u64>,
    current_position: u64,
    next_object_id: u32,
    // Maps for tracking object IDs during writing
    catalog_id: Option<ObjectId>,
    pages_id: Option<ObjectId>,
    info_id: Option<ObjectId>,
    // Maps for tracking form fields and their widgets
    #[allow(dead_code)]
    field_widget_map: HashMap<String, Vec<ObjectId>>, // field name -> widget IDs
    #[allow(dead_code)]
    field_id_map: HashMap<String, ObjectId>, // field name -> field ID
    form_field_ids: Vec<ObjectId>, // form field IDs to add to page annotations
    page_ids: Vec<ObjectId>,       // page IDs for form field references
    // Configuration
    config: WriterConfig,
    // Characters used in document (for font subsetting)
    document_used_chars: Option<std::collections::HashSet<char>>,
    // Object stream buffering (when use_object_streams is enabled)
    buffered_objects: HashMap<ObjectId, Vec<u8>>,
    compressed_object_map: HashMap<ObjectId, (ObjectId, u32)>, // obj_id -> (stream_id, index)
}

impl<W: Write> PdfWriter<W> {
    pub fn new_with_writer(writer: W) -> Self {
        Self::with_config(writer, WriterConfig::default())
    }

    pub fn with_config(writer: W, config: WriterConfig) -> Self {
        Self {
            writer,
            xref_positions: HashMap::new(),
            current_position: 0,
            next_object_id: 1, // Start at 1 for sequential numbering
            catalog_id: None,
            pages_id: None,
            info_id: None,
            field_widget_map: HashMap::new(),
            field_id_map: HashMap::new(),
            form_field_ids: Vec::new(),
            page_ids: Vec::new(),
            config,
            document_used_chars: None,
            buffered_objects: HashMap::new(),
            compressed_object_map: HashMap::new(),
        }
    }

    pub fn write_document(&mut self, document: &mut Document) -> Result<()> {
        // Store used characters for font subsetting
        if !document.used_characters.is_empty() {
            self.document_used_chars = Some(document.used_characters.clone());
        }

        self.write_header()?;

        // Reserve object IDs for fixed objects (written in order)
        self.catalog_id = Some(self.allocate_object_id());
        self.pages_id = Some(self.allocate_object_id());
        self.info_id = Some(self.allocate_object_id());

        // Write custom fonts first (so pages can reference them)
        let font_refs = self.write_fonts(document)?;

        // Write pages (they contain widget annotations and font references)
        self.write_pages(document, &font_refs)?;

        // Write form fields (must be after pages so we can track widgets)
        self.write_form_fields(document)?;

        // Write catalog (must be after forms so AcroForm has correct field references)
        self.write_catalog(document)?;

        // Write document info
        self.write_info(document)?;

        // Flush buffered objects as object streams (if enabled)
        if self.config.use_object_streams {
            self.flush_object_streams()?;
        }

        // Write xref table or stream
        let xref_position = self.current_position;
        if self.config.use_xref_streams {
            self.write_xref_stream()?;
        } else {
            self.write_xref()?;
        }

        // Write trailer (only for traditional xref)
        if !self.config.use_xref_streams {
            self.write_trailer(xref_position)?;
        }

        if let Ok(()) = self.writer.flush() {
            // Flush succeeded
        }
        Ok(())
    }

    fn write_header(&mut self) -> Result<()> {
        let header = format!("%PDF-{}\n", self.config.pdf_version);
        self.write_bytes(header.as_bytes())?;
        // Binary comment to ensure file is treated as binary
        self.write_bytes(&[b'%', 0xE2, 0xE3, 0xCF, 0xD3, b'\n'])?;
        Ok(())
    }

    fn write_catalog(&mut self, document: &mut Document) -> Result<()> {
        let catalog_id = self.catalog_id.expect("catalog_id must be set");
        let pages_id = self.pages_id.expect("pages_id must be set");

        let mut catalog = Dictionary::new();
        catalog.set("Type", Object::Name("Catalog".to_string()));
        catalog.set("Pages", Object::Reference(pages_id));

        // Process FormManager if present to update AcroForm
        // We'll write the actual fields after pages are written
        if let Some(_form_manager) = &document.form_manager {
            // Ensure AcroForm exists
            if document.acro_form.is_none() {
                document.acro_form = Some(crate::forms::AcroForm::new());
            }
        }

        // Add AcroForm if present
        if let Some(acro_form) = &document.acro_form {
            // Reserve object ID for AcroForm
            let acro_form_id = self.allocate_object_id();

            // Write AcroForm object
            self.write_object(acro_form_id, Object::Dictionary(acro_form.to_dict()))?;

            // Reference it in catalog
            catalog.set("AcroForm", Object::Reference(acro_form_id));
        }

        // Add Outlines if present
        if let Some(outline_tree) = &document.outline {
            if !outline_tree.items.is_empty() {
                let outline_root_id = self.write_outline_tree(outline_tree)?;
                catalog.set("Outlines", Object::Reference(outline_root_id));
            }
        }

        // Add StructTreeRoot if present (Tagged PDF - ISO 32000-1 §14.8)
        if let Some(struct_tree) = &document.struct_tree {
            if !struct_tree.is_empty() {
                let struct_tree_root_id = self.write_struct_tree(struct_tree)?;
                catalog.set("StructTreeRoot", Object::Reference(struct_tree_root_id));
                // Mark as Tagged PDF
                catalog.set("MarkInfo", {
                    let mut mark_info = Dictionary::new();
                    mark_info.set("Marked", Object::Boolean(true));
                    Object::Dictionary(mark_info)
                });
            }
        }

        // Add XMP Metadata stream (ISO 32000-1 §14.3.2)
        // Generate XMP from document metadata and embed as stream
        let xmp_metadata = document.create_xmp_metadata();
        let xmp_packet = xmp_metadata.to_xmp_packet();
        let metadata_id = self.allocate_object_id();

        // Create metadata stream dictionary
        let mut metadata_dict = Dictionary::new();
        metadata_dict.set("Type", Object::Name("Metadata".to_string()));
        metadata_dict.set("Subtype", Object::Name("XML".to_string()));
        metadata_dict.set("Length", Object::Integer(xmp_packet.len() as i64));

        // Write XMP metadata stream
        self.write_object(
            metadata_id,
            Object::Stream(metadata_dict, xmp_packet.into_bytes()),
        )?;

        // Reference it in catalog
        catalog.set("Metadata", Object::Reference(metadata_id));

        self.write_object(catalog_id, Object::Dictionary(catalog))?;
        Ok(())
    }

    fn write_page_content(&mut self, content_id: ObjectId, page: &crate::page::Page) -> Result<()> {
        let mut page_copy = page.clone();
        let content = page_copy.generate_content()?;

        // Create stream with compression if enabled
        #[cfg(feature = "compression")]
        {
            use crate::objects::Stream;
            let mut stream = Stream::new(content);
            // Only compress if config allows it
            if self.config.compress_streams {
                stream.compress_flate()?;
            }

            self.write_object(
                content_id,
                Object::Stream(stream.dictionary().clone(), stream.data().to_vec()),
            )?;
        }

        #[cfg(not(feature = "compression"))]
        {
            let mut stream_dict = Dictionary::new();
            stream_dict.set("Length", Object::Integer(content.len() as i64));

            self.write_object(content_id, Object::Stream(stream_dict, content))?;
        }

        Ok(())
    }

    fn write_outline_tree(
        &mut self,
        outline_tree: &crate::structure::OutlineTree,
    ) -> Result<ObjectId> {
        // Create root outline dictionary
        let outline_root_id = self.allocate_object_id();

        let mut outline_root = Dictionary::new();
        outline_root.set("Type", Object::Name("Outlines".to_string()));

        if !outline_tree.items.is_empty() {
            // Reserve IDs for all outline items
            let mut item_ids = Vec::new();

            // Count all items and assign IDs
            fn count_items(items: &[crate::structure::OutlineItem]) -> usize {
                let mut count = items.len();
                for item in items {
                    count += count_items(&item.children);
                }
                count
            }

            let total_items = count_items(&outline_tree.items);

            // Reserve IDs for all items
            for _ in 0..total_items {
                item_ids.push(self.allocate_object_id());
            }

            let mut id_index = 0;

            // Write root items
            let first_id = item_ids[0];
            let last_id = item_ids[outline_tree.items.len() - 1];

            outline_root.set("First", Object::Reference(first_id));
            outline_root.set("Last", Object::Reference(last_id));

            // Visible count
            let visible_count = outline_tree.visible_count();
            outline_root.set("Count", Object::Integer(visible_count));

            // Write all items recursively
            let mut written_items = Vec::new();

            for (i, item) in outline_tree.items.iter().enumerate() {
                let item_id = item_ids[id_index];
                id_index += 1;

                let prev_id = if i > 0 { Some(item_ids[i - 1]) } else { None };
                let next_id = if i < outline_tree.items.len() - 1 {
                    Some(item_ids[i + 1])
                } else {
                    None
                };

                // Write this item and its children
                let children_ids = self.write_outline_item(
                    item,
                    item_id,
                    outline_root_id,
                    prev_id,
                    next_id,
                    &mut item_ids,
                    &mut id_index,
                )?;

                written_items.extend(children_ids);
            }
        }

        self.write_object(outline_root_id, Object::Dictionary(outline_root))?;
        Ok(outline_root_id)
    }

    #[allow(clippy::too_many_arguments)]
    fn write_outline_item(
        &mut self,
        item: &crate::structure::OutlineItem,
        item_id: ObjectId,
        parent_id: ObjectId,
        prev_id: Option<ObjectId>,
        next_id: Option<ObjectId>,
        all_ids: &mut Vec<ObjectId>,
        id_index: &mut usize,
    ) -> Result<Vec<ObjectId>> {
        let mut written_ids = vec![item_id];

        // Handle children if any
        let (first_child_id, last_child_id) = if !item.children.is_empty() {
            let first_idx = *id_index;
            let first_id = all_ids[first_idx];
            let last_idx = first_idx + item.children.len() - 1;
            let last_id = all_ids[last_idx];

            // Write children
            for (i, child) in item.children.iter().enumerate() {
                let child_id = all_ids[*id_index];
                *id_index += 1;

                let child_prev = if i > 0 {
                    Some(all_ids[first_idx + i - 1])
                } else {
                    None
                };
                let child_next = if i < item.children.len() - 1 {
                    Some(all_ids[first_idx + i + 1])
                } else {
                    None
                };

                let child_ids = self.write_outline_item(
                    child, child_id, item_id, // This item is the parent
                    child_prev, child_next, all_ids, id_index,
                )?;

                written_ids.extend(child_ids);
            }

            (Some(first_id), Some(last_id))
        } else {
            (None, None)
        };

        // Create item dictionary
        let item_dict = crate::structure::outline_item_to_dict(
            item,
            parent_id,
            first_child_id,
            last_child_id,
            prev_id,
            next_id,
        );

        self.write_object(item_id, Object::Dictionary(item_dict))?;

        Ok(written_ids)
    }

    /// Writes the structure tree for Tagged PDF (ISO 32000-1 §14.8)
    fn write_struct_tree(
        &mut self,
        struct_tree: &crate::structure::StructTree,
    ) -> Result<ObjectId> {
        // Allocate IDs for StructTreeRoot and all elements
        let struct_tree_root_id = self.allocate_object_id();
        let mut element_ids = Vec::new();
        for _ in 0..struct_tree.len() {
            element_ids.push(self.allocate_object_id());
        }

        // Build parent map: element_index -> parent_id
        let mut parent_map: std::collections::HashMap<usize, ObjectId> =
            std::collections::HashMap::new();

        // Root element's parent is StructTreeRoot
        if let Some(root_index) = struct_tree.root_index() {
            parent_map.insert(root_index, struct_tree_root_id);

            // Recursively map all children to their parents
            fn map_children_parents(
                tree: &crate::structure::StructTree,
                parent_index: usize,
                parent_id: ObjectId,
                element_ids: &[ObjectId],
                parent_map: &mut std::collections::HashMap<usize, ObjectId>,
            ) {
                if let Some(parent_elem) = tree.get(parent_index) {
                    for &child_index in &parent_elem.children {
                        parent_map.insert(child_index, parent_id);
                        map_children_parents(
                            tree,
                            child_index,
                            element_ids[child_index],
                            element_ids,
                            parent_map,
                        );
                    }
                }
            }

            map_children_parents(
                struct_tree,
                root_index,
                element_ids[root_index],
                &element_ids,
                &mut parent_map,
            );
        }

        // Write all structure elements with parent references
        for (index, element) in struct_tree.iter().enumerate() {
            let element_id = element_ids[index];
            let mut element_dict = Dictionary::new();

            element_dict.set("Type", Object::Name("StructElem".to_string()));
            element_dict.set("S", Object::Name(element.structure_type.as_pdf_name()));

            // Parent reference (ISO 32000-1 §14.7.2 - required)
            if let Some(&parent_id) = parent_map.get(&index) {
                element_dict.set("P", Object::Reference(parent_id));
            }

            // Element ID (optional)
            if let Some(ref id) = element.id {
                element_dict.set("ID", Object::String(id.clone()));
            }

            // Attributes
            if let Some(ref lang) = element.attributes.lang {
                element_dict.set("Lang", Object::String(lang.clone()));
            }
            if let Some(ref alt) = element.attributes.alt {
                element_dict.set("Alt", Object::String(alt.clone()));
            }
            if let Some(ref actual_text) = element.attributes.actual_text {
                element_dict.set("ActualText", Object::String(actual_text.clone()));
            }
            if let Some(ref title) = element.attributes.title {
                element_dict.set("T", Object::String(title.clone()));
            }
            if let Some(bbox) = element.attributes.bbox {
                element_dict.set(
                    "BBox",
                    Object::Array(vec![
                        Object::Real(bbox[0]),
                        Object::Real(bbox[1]),
                        Object::Real(bbox[2]),
                        Object::Real(bbox[3]),
                    ]),
                );
            }

            // Kids (children elements + marked content references)
            let mut kids = Vec::new();

            // Add child element references
            for &child_index in &element.children {
                kids.push(Object::Reference(element_ids[child_index]));
            }

            // Add marked content references (MCIDs)
            for mcid_ref in &element.mcids {
                let mut mcr = Dictionary::new();
                mcr.set("Type", Object::Name("MCR".to_string()));
                mcr.set("Pg", Object::Integer(mcid_ref.page_index as i64));
                mcr.set("MCID", Object::Integer(mcid_ref.mcid as i64));
                kids.push(Object::Dictionary(mcr));
            }

            if !kids.is_empty() {
                element_dict.set("K", Object::Array(kids));
            }

            self.write_object(element_id, Object::Dictionary(element_dict))?;
        }

        // Create StructTreeRoot dictionary
        let mut struct_tree_root = Dictionary::new();
        struct_tree_root.set("Type", Object::Name("StructTreeRoot".to_string()));

        // Add root element(s) as K entry
        if let Some(root_index) = struct_tree.root_index() {
            struct_tree_root.set("K", Object::Reference(element_ids[root_index]));
        }

        // Add RoleMap if not empty
        if !struct_tree.role_map.mappings().is_empty() {
            let mut role_map = Dictionary::new();
            for (custom_type, standard_type) in struct_tree.role_map.mappings() {
                role_map.set(
                    custom_type.as_str(),
                    Object::Name(standard_type.as_pdf_name().to_string()),
                );
            }
            struct_tree_root.set("RoleMap", Object::Dictionary(role_map));
        }

        self.write_object(struct_tree_root_id, Object::Dictionary(struct_tree_root))?;
        Ok(struct_tree_root_id)
    }

    fn write_form_fields(&mut self, document: &mut Document) -> Result<()> {
        // Add collected form field IDs to AcroForm
        if !self.form_field_ids.is_empty() {
            if let Some(acro_form) = &mut document.acro_form {
                // Clear any existing fields and add the ones we found
                acro_form.fields.clear();
                for field_id in &self.form_field_ids {
                    acro_form.add_field(*field_id);
                }

                // Ensure AcroForm has the right properties
                acro_form.need_appearances = true;
                if acro_form.da.is_none() {
                    acro_form.da = Some("/Helv 12 Tf 0 g".to_string());
                }
            }
        }
        Ok(())
    }

    fn write_info(&mut self, document: &Document) -> Result<()> {
        let info_id = self.info_id.expect("info_id must be set");
        let mut info_dict = Dictionary::new();

        if let Some(ref title) = document.metadata.title {
            info_dict.set("Title", Object::String(title.clone()));
        }
        if let Some(ref author) = document.metadata.author {
            info_dict.set("Author", Object::String(author.clone()));
        }
        if let Some(ref subject) = document.metadata.subject {
            info_dict.set("Subject", Object::String(subject.clone()));
        }
        if let Some(ref keywords) = document.metadata.keywords {
            info_dict.set("Keywords", Object::String(keywords.clone()));
        }
        if let Some(ref creator) = document.metadata.creator {
            info_dict.set("Creator", Object::String(creator.clone()));
        }
        if let Some(ref producer) = document.metadata.producer {
            info_dict.set("Producer", Object::String(producer.clone()));
        }

        // Add creation date
        if let Some(creation_date) = document.metadata.creation_date {
            let date_string = format_pdf_date(creation_date);
            info_dict.set("CreationDate", Object::String(date_string));
        }

        // Add modification date
        if let Some(mod_date) = document.metadata.modification_date {
            let date_string = format_pdf_date(mod_date);
            info_dict.set("ModDate", Object::String(date_string));
        }

        // Add PDF signature (anti-spoofing and licensing)
        // This is written AFTER user-configurable metadata so it cannot be overridden
        let edition = if cfg!(feature = "pro") {
            super::Edition::Pro
        } else if cfg!(feature = "enterprise") {
            super::Edition::Enterprise
        } else {
            super::Edition::Community
        };

        let signature = super::PdfSignature::new(document, edition);
        signature.write_to_info_dict(&mut info_dict);

        self.write_object(info_id, Object::Dictionary(info_dict))?;
        Ok(())
    }

    fn write_fonts(&mut self, document: &Document) -> Result<HashMap<String, ObjectId>> {
        let mut font_refs = HashMap::new();

        // Write custom fonts from the document
        for font_name in document.custom_font_names() {
            if let Some(font) = document.get_custom_font(&font_name) {
                // For now, write all custom fonts as TrueType with Identity-H for Unicode support
                // The font from document is Arc<fonts::Font>, not text::font_manager::CustomFont
                let font_id = self.write_font_with_unicode_support(&font_name, &font)?;
                font_refs.insert(font_name.clone(), font_id);
            }
        }

        Ok(font_refs)
    }

    /// Write font with automatic Unicode support detection
    fn write_font_with_unicode_support(
        &mut self,
        font_name: &str,
        font: &crate::fonts::Font,
    ) -> Result<ObjectId> {
        // Check if any text in the document needs Unicode
        // For simplicity, always use Type0 for full Unicode support
        self.write_type0_font_from_font(font_name, font)
    }

    /// Write a Type0 font with CID support from fonts::Font
    fn write_type0_font_from_font(
        &mut self,
        font_name: &str,
        font: &crate::fonts::Font,
    ) -> Result<ObjectId> {
        // Get used characters from document for subsetting
        let used_chars = self.document_used_chars.clone().unwrap_or_else(|| {
            // If no tracking, include common characters as fallback
            let mut chars = std::collections::HashSet::new();
            for ch in "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789 .,!?".chars()
            {
                chars.insert(ch);
            }
            chars
        });
        // Allocate IDs for all font objects
        let font_id = self.allocate_object_id();
        let descendant_font_id = self.allocate_object_id();
        let descriptor_id = self.allocate_object_id();
        let font_file_id = self.allocate_object_id();
        let to_unicode_id = self.allocate_object_id();

        // Write font file (embedded TTF data with subsetting for large fonts)
        // Keep track of the glyph mapping if we subset the font
        // IMPORTANT: We need the ORIGINAL font for width calculations, not the subset
        let (font_data_to_embed, subset_glyph_mapping, original_font_for_widths) =
            if font.data.len() > 100_000 && !used_chars.is_empty() {
                // Large font - try to subset it
                match crate::text::fonts::truetype_subsetter::subset_font(
                    font.data.clone(),
                    &used_chars,
                ) {
                    Ok(subset_result) => {
                        // Successfully subsetted - keep both font data and mapping
                        // Also keep reference to original font for width calculations
                        (
                            subset_result.font_data,
                            Some(subset_result.glyph_mapping),
                            font.clone(),
                        )
                    }
                    Err(_) => {
                        // Subsetting failed, use original if under 25MB
                        if font.data.len() < 25_000_000 {
                            (font.data.clone(), None, font.clone())
                        } else {
                            // Too large even for fallback
                            (Vec::new(), None, font.clone())
                        }
                    }
                }
            } else {
                // Small font or no character tracking - use as-is
                (font.data.clone(), None, font.clone())
            };

        if !font_data_to_embed.is_empty() {
            let mut font_file_dict = Dictionary::new();
            // Add appropriate properties based on font format
            match font.format {
                crate::fonts::FontFormat::OpenType => {
                    // CFF/OpenType fonts use FontFile3 with OpenType subtype
                    font_file_dict.set("Subtype", Object::Name("OpenType".to_string()));
                    font_file_dict.set("Length1", Object::Integer(font_data_to_embed.len() as i64));
                }
                crate::fonts::FontFormat::TrueType => {
                    // TrueType fonts use FontFile2 with Length1
                    font_file_dict.set("Length1", Object::Integer(font_data_to_embed.len() as i64));
                }
            }
            let font_stream_obj = Object::Stream(font_file_dict, font_data_to_embed);
            self.write_object(font_file_id, font_stream_obj)?;
        } else {
            // No font data to embed
            let font_file_dict = Dictionary::new();
            let font_stream_obj = Object::Stream(font_file_dict, Vec::new());
            self.write_object(font_file_id, font_stream_obj)?;
        }

        // Write font descriptor
        let mut descriptor = Dictionary::new();
        descriptor.set("Type", Object::Name("FontDescriptor".to_string()));
        descriptor.set("FontName", Object::Name(font_name.to_string()));
        descriptor.set("Flags", Object::Integer(4)); // Symbolic font
        descriptor.set(
            "FontBBox",
            Object::Array(vec![
                Object::Integer(font.descriptor.font_bbox[0] as i64),
                Object::Integer(font.descriptor.font_bbox[1] as i64),
                Object::Integer(font.descriptor.font_bbox[2] as i64),
                Object::Integer(font.descriptor.font_bbox[3] as i64),
            ]),
        );
        descriptor.set(
            "ItalicAngle",
            Object::Real(font.descriptor.italic_angle as f64),
        );
        descriptor.set("Ascent", Object::Real(font.descriptor.ascent as f64));
        descriptor.set("Descent", Object::Real(font.descriptor.descent as f64));
        descriptor.set("CapHeight", Object::Real(font.descriptor.cap_height as f64));
        descriptor.set("StemV", Object::Real(font.descriptor.stem_v as f64));
        // Use appropriate FontFile type based on font format
        let font_file_key = match font.format {
            crate::fonts::FontFormat::OpenType => "FontFile3", // CFF/OpenType fonts
            crate::fonts::FontFormat::TrueType => "FontFile2", // TrueType fonts
        };
        descriptor.set(font_file_key, Object::Reference(font_file_id));
        self.write_object(descriptor_id, Object::Dictionary(descriptor))?;

        // Write CIDFont (descendant font)
        let mut cid_font = Dictionary::new();
        cid_font.set("Type", Object::Name("Font".to_string()));
        // Use appropriate CIDFont subtype based on font format
        let cid_font_subtype =
            if CjkFontType::should_use_cidfonttype2_for_preview_compatibility(font_name) {
                "CIDFontType2" // Force CIDFontType2 for CJK fonts to fix Preview.app rendering
            } else {
                match font.format {
                    crate::fonts::FontFormat::OpenType => "CIDFontType0", // CFF/OpenType fonts
                    crate::fonts::FontFormat::TrueType => "CIDFontType2", // TrueType fonts
                }
            };
        cid_font.set("Subtype", Object::Name(cid_font_subtype.to_string()));
        cid_font.set("BaseFont", Object::Name(font_name.to_string()));

        // CIDSystemInfo - Use appropriate values for CJK fonts
        let mut cid_system_info = Dictionary::new();
        let (registry, ordering, supplement) =
            if let Some(cjk_type) = CjkFontType::detect_from_name(font_name) {
                cjk_type.cid_system_info()
            } else {
                ("Adobe", "Identity", 0)
            };

        cid_system_info.set("Registry", Object::String(registry.to_string()));
        cid_system_info.set("Ordering", Object::String(ordering.to_string()));
        cid_system_info.set("Supplement", Object::Integer(supplement as i64));
        cid_font.set("CIDSystemInfo", Object::Dictionary(cid_system_info));

        cid_font.set("FontDescriptor", Object::Reference(descriptor_id));

        // Calculate a better default width based on font metrics
        let default_width = self.calculate_default_width(font);
        cid_font.set("DW", Object::Integer(default_width));

        // Generate proper width array from font metrics
        // IMPORTANT: Use the ORIGINAL font for width calculations, not the subset
        // But pass the subset mapping to know which characters we're using
        let w_array = self.generate_width_array(
            &original_font_for_widths,
            default_width,
            subset_glyph_mapping.as_ref(),
        );
        cid_font.set("W", Object::Array(w_array));

        // CIDToGIDMap - Generate proper mapping from CID (Unicode) to GlyphID
        // This is critical for Type0 fonts to work correctly
        // If we subsetted the font, use the new glyph mapping
        let cid_to_gid_map = self.generate_cid_to_gid_map(font, subset_glyph_mapping.as_ref())?;
        if !cid_to_gid_map.is_empty() {
            // Write the CIDToGIDMap as a stream
            let cid_to_gid_map_id = self.allocate_object_id();
            let mut map_dict = Dictionary::new();
            map_dict.set("Length", Object::Integer(cid_to_gid_map.len() as i64));
            let map_stream = Object::Stream(map_dict, cid_to_gid_map);
            self.write_object(cid_to_gid_map_id, map_stream)?;
            cid_font.set("CIDToGIDMap", Object::Reference(cid_to_gid_map_id));
        } else {
            cid_font.set("CIDToGIDMap", Object::Name("Identity".to_string()));
        }

        self.write_object(descendant_font_id, Object::Dictionary(cid_font))?;

        // Write ToUnicode CMap
        let cmap_data = self.generate_tounicode_cmap_from_font(font);
        let cmap_dict = Dictionary::new();
        let cmap_stream = Object::Stream(cmap_dict, cmap_data);
        self.write_object(to_unicode_id, cmap_stream)?;

        // Write Type0 font (main font)
        let mut type0_font = Dictionary::new();
        type0_font.set("Type", Object::Name("Font".to_string()));
        type0_font.set("Subtype", Object::Name("Type0".to_string()));
        type0_font.set("BaseFont", Object::Name(font_name.to_string()));
        type0_font.set("Encoding", Object::Name("Identity-H".to_string()));
        type0_font.set(
            "DescendantFonts",
            Object::Array(vec![Object::Reference(descendant_font_id)]),
        );
        type0_font.set("ToUnicode", Object::Reference(to_unicode_id));

        self.write_object(font_id, Object::Dictionary(type0_font))?;

        Ok(font_id)
    }

    /// Calculate default width based on common characters
    fn calculate_default_width(&self, font: &crate::fonts::Font) -> i64 {
        use crate::text::fonts::truetype::TrueTypeFont;

        // Try to calculate from actual font metrics
        if let Ok(tt_font) = TrueTypeFont::parse(font.data.clone()) {
            if let Ok(cmap_tables) = tt_font.parse_cmap() {
                if let Some(cmap) = cmap_tables
                    .iter()
                    .find(|t| t.platform_id == 3 && t.encoding_id == 1)
                    .or_else(|| cmap_tables.iter().find(|t| t.platform_id == 0))
                {
                    if let Ok(widths) = tt_font.get_glyph_widths(&cmap.mappings) {
                        // NOTE: get_glyph_widths already returns widths in PDF units (1000 per em)

                        // Calculate average width of common Latin characters
                        let common_chars =
                            "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789 ";
                        let mut total_width = 0;
                        let mut count = 0;

                        for ch in common_chars.chars() {
                            let unicode = ch as u32;
                            if let Some(&pdf_width) = widths.get(&unicode) {
                                total_width += pdf_width as i64;
                                count += 1;
                            }
                        }

                        if count > 0 {
                            return total_width / count;
                        }
                    }
                }
            }
        }

        // Fallback default if we can't calculate
        500
    }

    /// Generate width array for CID font
    fn generate_width_array(
        &self,
        font: &crate::fonts::Font,
        _default_width: i64,
        subset_mapping: Option<&HashMap<u32, u16>>,
    ) -> Vec<Object> {
        use crate::text::fonts::truetype::TrueTypeFont;

        let mut w_array = Vec::new();

        // Try to get actual glyph widths from the font
        if let Ok(tt_font) = TrueTypeFont::parse(font.data.clone()) {
            // IMPORTANT: Always use ORIGINAL mappings for width calculation
            // The subset_mapping has NEW GlyphIDs which don't correspond to the right glyphs
            // in the original font's width table
            let char_to_glyph = {
                // Parse cmap to get original mappings
                if let Ok(cmap_tables) = tt_font.parse_cmap() {
                    if let Some(cmap) = cmap_tables
                        .iter()
                        .find(|t| t.platform_id == 3 && t.encoding_id == 1)
                        .or_else(|| cmap_tables.iter().find(|t| t.platform_id == 0))
                    {
                        // If we have subset_mapping, filter to only include used characters
                        if let Some(subset_map) = subset_mapping {
                            let mut filtered = HashMap::new();
                            for unicode in subset_map.keys() {
                                // Get the ORIGINAL GlyphID for this Unicode
                                if let Some(&orig_glyph) = cmap.mappings.get(unicode) {
                                    filtered.insert(*unicode, orig_glyph);
                                }
                            }
                            filtered
                        } else {
                            cmap.mappings.clone()
                        }
                    } else {
                        HashMap::new()
                    }
                } else {
                    HashMap::new()
                }
            };

            if !char_to_glyph.is_empty() {
                // Get actual widths from the font
                if let Ok(widths) = tt_font.get_glyph_widths(&char_to_glyph) {
                    // NOTE: get_glyph_widths already returns widths scaled to PDF units (1000 per em)
                    // So we DON'T need to scale them again here

                    // Group consecutive characters with same width for efficiency
                    let mut sorted_chars: Vec<_> = widths.iter().collect();
                    sorted_chars.sort_by_key(|(unicode, _)| *unicode);

                    let mut i = 0;
                    while i < sorted_chars.len() {
                        let start_unicode = *sorted_chars[i].0;
                        // Width is already in PDF units from get_glyph_widths
                        let pdf_width = *sorted_chars[i].1 as i64;

                        // Find consecutive characters with same width
                        let mut end_unicode = start_unicode;
                        let mut j = i + 1;
                        while j < sorted_chars.len() && *sorted_chars[j].0 == end_unicode + 1 {
                            let next_pdf_width = *sorted_chars[j].1 as i64;
                            if next_pdf_width == pdf_width {
                                end_unicode = *sorted_chars[j].0;
                                j += 1;
                            } else {
                                break;
                            }
                        }

                        // Add to W array
                        if start_unicode == end_unicode {
                            // Single character
                            w_array.push(Object::Integer(start_unicode as i64));
                            w_array.push(Object::Array(vec![Object::Integer(pdf_width)]));
                        } else {
                            // Range of characters
                            w_array.push(Object::Integer(start_unicode as i64));
                            w_array.push(Object::Integer(end_unicode as i64));
                            w_array.push(Object::Integer(pdf_width));
                        }

                        i = j;
                    }

                    return w_array;
                }
            }
        }

        // Fallback to reasonable default widths if we can't parse the font
        let ranges = vec![
            // Space character should be narrower
            (0x20, 0x20, 250), // Space
            (0x21, 0x2F, 333), // Punctuation
            (0x30, 0x39, 500), // Numbers (0-9)
            (0x3A, 0x40, 333), // More punctuation
            (0x41, 0x5A, 667), // Uppercase letters (A-Z)
            (0x5B, 0x60, 333), // Brackets
            (0x61, 0x7A, 500), // Lowercase letters (a-z)
            (0x7B, 0x7E, 333), // More brackets
            // Extended Latin
            (0xA0, 0xA0, 250), // Non-breaking space
            (0xA1, 0xBF, 333), // Latin-1 punctuation
            (0xC0, 0xD6, 667), // Latin-1 uppercase
            (0xD7, 0xD7, 564), // Multiplication sign
            (0xD8, 0xDE, 667), // More Latin-1 uppercase
            (0xDF, 0xF6, 500), // Latin-1 lowercase
            (0xF7, 0xF7, 564), // Division sign
            (0xF8, 0xFF, 500), // More Latin-1 lowercase
            // Latin Extended-A
            (0x100, 0x17F, 500), // Latin Extended-A
            // Symbols and special characters
            (0x2000, 0x200F, 250), // Various spaces
            (0x2010, 0x2027, 333), // Hyphens and dashes
            (0x2028, 0x202F, 250), // More spaces
            (0x2030, 0x206F, 500), // General Punctuation
            (0x2070, 0x209F, 400), // Superscripts
            (0x20A0, 0x20CF, 600), // Currency symbols
            (0x2100, 0x214F, 700), // Letterlike symbols
            (0x2190, 0x21FF, 600), // Arrows
            (0x2200, 0x22FF, 600), // Mathematical operators
            (0x2300, 0x23FF, 600), // Miscellaneous technical
            (0x2500, 0x257F, 500), // Box drawing
            (0x2580, 0x259F, 500), // Block elements
            (0x25A0, 0x25FF, 600), // Geometric shapes
            (0x2600, 0x26FF, 600), // Miscellaneous symbols
            (0x2700, 0x27BF, 600), // Dingbats
        ];

        // Convert ranges to W array format
        for (start, end, width) in ranges {
            if start == end {
                // Single character
                w_array.push(Object::Integer(start));
                w_array.push(Object::Array(vec![Object::Integer(width)]));
            } else {
                // Range of characters
                w_array.push(Object::Integer(start));
                w_array.push(Object::Integer(end));
                w_array.push(Object::Integer(width));
            }
        }

        w_array
    }

    /// Generate CIDToGIDMap for Type0 font
    fn generate_cid_to_gid_map(
        &mut self,
        font: &crate::fonts::Font,
        subset_mapping: Option<&HashMap<u32, u16>>,
    ) -> Result<Vec<u8>> {
        use crate::text::fonts::truetype::TrueTypeFont;

        // If we have a subset mapping, use it directly
        // Otherwise, parse the font to get the original cmap table
        let cmap_mappings = if let Some(subset_map) = subset_mapping {
            // Use the subset mapping directly
            subset_map.clone()
        } else {
            // Parse the font to get the original cmap table
            let tt_font = TrueTypeFont::parse(font.data.clone())?;
            let cmap_tables = tt_font.parse_cmap()?;

            // Find the best cmap table (Unicode)
            let cmap = cmap_tables
                .iter()
                .find(|t| t.platform_id == 3 && t.encoding_id == 1) // Windows Unicode
                .or_else(|| cmap_tables.iter().find(|t| t.platform_id == 0)) // Unicode
                .ok_or_else(|| {
                    crate::error::PdfError::FontError("No Unicode cmap table found".to_string())
                })?;

            cmap.mappings.clone()
        };

        // Build the CIDToGIDMap
        // Since we use Unicode code points as CIDs, we need to map Unicode → GlyphID
        // The map is a binary array where index = CID (Unicode) * 2, value = GlyphID (big-endian)

        // OPTIMIZATION: Only create map for characters actually used in the document
        // Get used characters from document tracking
        let used_chars = self.document_used_chars.clone().unwrap_or_default();

        // Find the maximum Unicode value from used characters or full font
        let max_unicode = if !used_chars.is_empty() {
            // If we have used chars tracking, only map up to the highest used character
            used_chars
                .iter()
                .map(|ch| *ch as u32)
                .max()
                .unwrap_or(0x00FF) // At least Basic Latin
                .min(0xFFFF) as usize
        } else {
            // Fallback to original behavior if no tracking
            cmap_mappings
                .keys()
                .max()
                .copied()
                .unwrap_or(0xFFFF)
                .min(0xFFFF) as usize
        };

        // Create the map: 2 bytes per entry
        let mut map = vec![0u8; (max_unicode + 1) * 2];

        // Fill in the mappings
        let mut sample_mappings = Vec::new();
        for (&unicode, &glyph_id) in &cmap_mappings {
            if unicode <= max_unicode as u32 {
                let idx = (unicode as usize) * 2;
                // Write glyph_id in big-endian format
                map[idx] = (glyph_id >> 8) as u8;
                map[idx + 1] = (glyph_id & 0xFF) as u8;

                // Collect some sample mappings for debugging
                if unicode == 0x0041 || unicode == 0x0061 || unicode == 0x00E1 || unicode == 0x00F1
                {
                    sample_mappings.push((unicode, glyph_id));
                }
            }
        }

        Ok(map)
    }

    /// Generate ToUnicode CMap for Type0 font from fonts::Font
    fn generate_tounicode_cmap_from_font(&self, font: &crate::fonts::Font) -> Vec<u8> {
        use crate::text::fonts::truetype::TrueTypeFont;

        let mut cmap = String::new();

        // CMap header
        cmap.push_str("/CIDInit /ProcSet findresource begin\n");
        cmap.push_str("12 dict begin\n");
        cmap.push_str("begincmap\n");
        cmap.push_str("/CIDSystemInfo\n");
        cmap.push_str("<< /Registry (Adobe)\n");
        cmap.push_str("   /Ordering (UCS)\n");
        cmap.push_str("   /Supplement 0\n");
        cmap.push_str(">> def\n");
        cmap.push_str("/CMapName /Adobe-Identity-UCS def\n");
        cmap.push_str("/CMapType 2 def\n");
        cmap.push_str("1 begincodespacerange\n");
        cmap.push_str("<0000> <FFFF>\n");
        cmap.push_str("endcodespacerange\n");

        // Try to get actual mappings from the font
        let mut mappings = Vec::new();
        let mut has_font_mappings = false;

        if let Ok(tt_font) = TrueTypeFont::parse(font.data.clone()) {
            if let Ok(cmap_tables) = tt_font.parse_cmap() {
                // Find the best cmap table (Unicode)
                if let Some(cmap_table) = cmap_tables
                    .iter()
                    .find(|t| t.platform_id == 3 && t.encoding_id == 1) // Windows Unicode
                    .or_else(|| cmap_tables.iter().find(|t| t.platform_id == 0))
                // Unicode
                {
                    // For Identity-H encoding, we use Unicode code points as CIDs
                    // So the ToUnicode CMap should map CID (=Unicode) → Unicode
                    for (&unicode, &glyph_id) in &cmap_table.mappings {
                        if glyph_id > 0 && unicode <= 0xFFFF {
                            // Only non-.notdef glyphs
                            // Map CID (which is Unicode value) to Unicode
                            mappings.push((unicode, unicode));
                        }
                    }
                    has_font_mappings = true;
                }
            }
        }

        // If we couldn't get font mappings, use identity mapping for common ranges
        if !has_font_mappings {
            // Basic Latin and Latin-1 Supplement (0x0020-0x00FF)
            for i in 0x0020..=0x00FF {
                mappings.push((i, i));
            }

            // Latin Extended-A (0x0100-0x017F)
            for i in 0x0100..=0x017F {
                mappings.push((i, i));
            }

            // CJK Unicode ranges - CRITICAL for CJK font support
            // Hiragana (Japanese)
            for i in 0x3040..=0x309F {
                mappings.push((i, i));
            }

            // Katakana (Japanese)
            for i in 0x30A0..=0x30FF {
                mappings.push((i, i));
            }

            // CJK Unified Ideographs (Chinese, Japanese, Korean)
            for i in 0x4E00..=0x9FFF {
                mappings.push((i, i));
            }

            // Hangul Syllables (Korean)
            for i in 0xAC00..=0xD7AF {
                mappings.push((i, i));
            }

            // Common symbols and punctuation
            for i in 0x2000..=0x206F {
                mappings.push((i, i));
            }

            // Mathematical symbols
            for i in 0x2200..=0x22FF {
                mappings.push((i, i));
            }

            // Arrows
            for i in 0x2190..=0x21FF {
                mappings.push((i, i));
            }

            // Box drawing
            for i in 0x2500..=0x259F {
                mappings.push((i, i));
            }

            // Geometric shapes
            for i in 0x25A0..=0x25FF {
                mappings.push((i, i));
            }

            // Miscellaneous symbols
            for i in 0x2600..=0x26FF {
                mappings.push((i, i));
            }
        }

        // Sort mappings by CID for better organization
        mappings.sort_by_key(|&(cid, _)| cid);

        // Use more efficient bfrange where possible
        let mut i = 0;
        while i < mappings.len() {
            // Check if we can use a range
            let start_cid = mappings[i].0;
            let start_unicode = mappings[i].1;
            let mut end_idx = i;

            // Find consecutive mappings
            while end_idx + 1 < mappings.len()
                && mappings[end_idx + 1].0 == mappings[end_idx].0 + 1
                && mappings[end_idx + 1].1 == mappings[end_idx].1 + 1
                && end_idx - i < 99
            // Max 100 per block
            {
                end_idx += 1;
            }

            if end_idx > i {
                // Use bfrange for consecutive mappings
                cmap.push_str("1 beginbfrange\n");
                cmap.push_str(&format!(
                    "<{:04X}> <{:04X}> <{:04X}>\n",
                    start_cid, mappings[end_idx].0, start_unicode
                ));
                cmap.push_str("endbfrange\n");
                i = end_idx + 1;
            } else {
                // Use bfchar for individual mappings
                let mut chars = Vec::new();
                let chunk_end = (i + 100).min(mappings.len());

                for item in &mappings[i..chunk_end] {
                    chars.push(*item);
                }

                if !chars.is_empty() {
                    cmap.push_str(&format!("{} beginbfchar\n", chars.len()));
                    for (cid, unicode) in chars {
                        cmap.push_str(&format!("<{:04X}> <{:04X}>\n", cid, unicode));
                    }
                    cmap.push_str("endbfchar\n");
                }

                i = chunk_end;
            }
        }

        // CMap footer
        cmap.push_str("endcmap\n");
        cmap.push_str("CMapName currentdict /CMap defineresource pop\n");
        cmap.push_str("end\n");
        cmap.push_str("end\n");

        cmap.into_bytes()
    }

    /// Write a regular TrueType font
    #[allow(dead_code)]
    fn write_truetype_font(
        &mut self,
        font_name: &str,
        font: &crate::text::font_manager::CustomFont,
    ) -> Result<ObjectId> {
        // Allocate IDs for font objects
        let font_id = self.allocate_object_id();
        let descriptor_id = self.allocate_object_id();
        let font_file_id = self.allocate_object_id();

        // Write font file (embedded TTF data)
        if let Some(ref data) = font.font_data {
            let mut font_file_dict = Dictionary::new();
            font_file_dict.set("Length1", Object::Integer(data.len() as i64));
            let font_stream_obj = Object::Stream(font_file_dict, data.clone());
            self.write_object(font_file_id, font_stream_obj)?;
        }

        // Write font descriptor
        let mut descriptor = Dictionary::new();
        descriptor.set("Type", Object::Name("FontDescriptor".to_string()));
        descriptor.set("FontName", Object::Name(font_name.to_string()));
        descriptor.set("Flags", Object::Integer(32)); // Non-symbolic font
        descriptor.set(
            "FontBBox",
            Object::Array(vec![
                Object::Integer(-1000),
                Object::Integer(-1000),
                Object::Integer(2000),
                Object::Integer(2000),
            ]),
        );
        descriptor.set("ItalicAngle", Object::Integer(0));
        descriptor.set("Ascent", Object::Integer(font.descriptor.ascent as i64));
        descriptor.set("Descent", Object::Integer(font.descriptor.descent as i64));
        descriptor.set(
            "CapHeight",
            Object::Integer(font.descriptor.cap_height as i64),
        );
        descriptor.set("StemV", Object::Integer(font.descriptor.stem_v as i64));
        descriptor.set("FontFile2", Object::Reference(font_file_id));
        self.write_object(descriptor_id, Object::Dictionary(descriptor))?;

        // Write font dictionary
        let mut font_dict = Dictionary::new();
        font_dict.set("Type", Object::Name("Font".to_string()));
        font_dict.set("Subtype", Object::Name("TrueType".to_string()));
        font_dict.set("BaseFont", Object::Name(font_name.to_string()));
        font_dict.set("FirstChar", Object::Integer(0));
        font_dict.set("LastChar", Object::Integer(255));

        // Create widths array (simplified - all 600)
        let widths: Vec<Object> = (0..256).map(|_| Object::Integer(600)).collect();
        font_dict.set("Widths", Object::Array(widths));
        font_dict.set("FontDescriptor", Object::Reference(descriptor_id));

        // Use WinAnsiEncoding for regular TrueType
        font_dict.set("Encoding", Object::Name("WinAnsiEncoding".to_string()));

        self.write_object(font_id, Object::Dictionary(font_dict))?;

        Ok(font_id)
    }

    fn write_pages(
        &mut self,
        document: &Document,
        font_refs: &HashMap<String, ObjectId>,
    ) -> Result<()> {
        let pages_id = self.pages_id.expect("pages_id must be set");
        let mut pages_dict = Dictionary::new();
        pages_dict.set("Type", Object::Name("Pages".to_string()));
        pages_dict.set("Count", Object::Integer(document.pages.len() as i64));

        let mut kids = Vec::new();

        // Allocate page object IDs sequentially
        let mut page_ids = Vec::new();
        let mut content_ids = Vec::new();
        for _ in 0..document.pages.len() {
            page_ids.push(self.allocate_object_id());
            content_ids.push(self.allocate_object_id());
        }

        for page_id in &page_ids {
            kids.push(Object::Reference(*page_id));
        }

        pages_dict.set("Kids", Object::Array(kids));

        self.write_object(pages_id, Object::Dictionary(pages_dict))?;

        // Store page IDs for form field references
        self.page_ids = page_ids.clone();

        // Write individual pages with font references
        for (i, page) in document.pages.iter().enumerate() {
            let page_id = page_ids[i];
            let content_id = content_ids[i];

            self.write_page_with_fonts(page_id, pages_id, content_id, page, document, font_refs)?;
            self.write_page_content(content_id, page)?;
        }

        Ok(())
    }

    /// Compatibility alias for `write_pages` to maintain backwards compatibility
    #[allow(dead_code)]
    fn write_pages_with_fonts(
        &mut self,
        document: &Document,
        font_refs: &HashMap<String, ObjectId>,
    ) -> Result<()> {
        self.write_pages(document, font_refs)
    }

    fn write_page_with_fonts(
        &mut self,
        page_id: ObjectId,
        parent_id: ObjectId,
        content_id: ObjectId,
        page: &crate::page::Page,
        _document: &Document,
        font_refs: &HashMap<String, ObjectId>,
    ) -> Result<()> {
        // Start with the page's dictionary which includes annotations
        let mut page_dict = page.to_dict();

        page_dict.set("Type", Object::Name("Page".to_string()));
        page_dict.set("Parent", Object::Reference(parent_id));
        page_dict.set("Contents", Object::Reference(content_id));

        // Get resources dictionary or create new one
        let mut resources = if let Some(Object::Dictionary(res)) = page_dict.get("Resources") {
            res.clone()
        } else {
            Dictionary::new()
        };

        // Add font resources
        let mut font_dict = Dictionary::new();

        // Add ALL standard PDF fonts (Type1) with WinAnsiEncoding
        // This fixes the text rendering issue in dashboards where HelveticaBold was missing

        // Helvetica family
        let mut helvetica_dict = Dictionary::new();
        helvetica_dict.set("Type", Object::Name("Font".to_string()));
        helvetica_dict.set("Subtype", Object::Name("Type1".to_string()));
        helvetica_dict.set("BaseFont", Object::Name("Helvetica".to_string()));
        helvetica_dict.set("Encoding", Object::Name("WinAnsiEncoding".to_string()));
        font_dict.set("Helvetica", Object::Dictionary(helvetica_dict));

        let mut helvetica_bold_dict = Dictionary::new();
        helvetica_bold_dict.set("Type", Object::Name("Font".to_string()));
        helvetica_bold_dict.set("Subtype", Object::Name("Type1".to_string()));
        helvetica_bold_dict.set("BaseFont", Object::Name("Helvetica-Bold".to_string()));
        helvetica_bold_dict.set("Encoding", Object::Name("WinAnsiEncoding".to_string()));
        font_dict.set("Helvetica-Bold", Object::Dictionary(helvetica_bold_dict));

        let mut helvetica_oblique_dict = Dictionary::new();
        helvetica_oblique_dict.set("Type", Object::Name("Font".to_string()));
        helvetica_oblique_dict.set("Subtype", Object::Name("Type1".to_string()));
        helvetica_oblique_dict.set("BaseFont", Object::Name("Helvetica-Oblique".to_string()));
        helvetica_oblique_dict.set("Encoding", Object::Name("WinAnsiEncoding".to_string()));
        font_dict.set(
            "Helvetica-Oblique",
            Object::Dictionary(helvetica_oblique_dict),
        );

        let mut helvetica_bold_oblique_dict = Dictionary::new();
        helvetica_bold_oblique_dict.set("Type", Object::Name("Font".to_string()));
        helvetica_bold_oblique_dict.set("Subtype", Object::Name("Type1".to_string()));
        helvetica_bold_oblique_dict.set(
            "BaseFont",
            Object::Name("Helvetica-BoldOblique".to_string()),
        );
        helvetica_bold_oblique_dict.set("Encoding", Object::Name("WinAnsiEncoding".to_string()));
        font_dict.set(
            "Helvetica-BoldOblique",
            Object::Dictionary(helvetica_bold_oblique_dict),
        );

        // Times family
        let mut times_dict = Dictionary::new();
        times_dict.set("Type", Object::Name("Font".to_string()));
        times_dict.set("Subtype", Object::Name("Type1".to_string()));
        times_dict.set("BaseFont", Object::Name("Times-Roman".to_string()));
        times_dict.set("Encoding", Object::Name("WinAnsiEncoding".to_string()));
        font_dict.set("Times-Roman", Object::Dictionary(times_dict));

        let mut times_bold_dict = Dictionary::new();
        times_bold_dict.set("Type", Object::Name("Font".to_string()));
        times_bold_dict.set("Subtype", Object::Name("Type1".to_string()));
        times_bold_dict.set("BaseFont", Object::Name("Times-Bold".to_string()));
        times_bold_dict.set("Encoding", Object::Name("WinAnsiEncoding".to_string()));
        font_dict.set("Times-Bold", Object::Dictionary(times_bold_dict));

        let mut times_italic_dict = Dictionary::new();
        times_italic_dict.set("Type", Object::Name("Font".to_string()));
        times_italic_dict.set("Subtype", Object::Name("Type1".to_string()));
        times_italic_dict.set("BaseFont", Object::Name("Times-Italic".to_string()));
        times_italic_dict.set("Encoding", Object::Name("WinAnsiEncoding".to_string()));
        font_dict.set("Times-Italic", Object::Dictionary(times_italic_dict));

        let mut times_bold_italic_dict = Dictionary::new();
        times_bold_italic_dict.set("Type", Object::Name("Font".to_string()));
        times_bold_italic_dict.set("Subtype", Object::Name("Type1".to_string()));
        times_bold_italic_dict.set("BaseFont", Object::Name("Times-BoldItalic".to_string()));
        times_bold_italic_dict.set("Encoding", Object::Name("WinAnsiEncoding".to_string()));
        font_dict.set(
            "Times-BoldItalic",
            Object::Dictionary(times_bold_italic_dict),
        );

        // Courier family
        let mut courier_dict = Dictionary::new();
        courier_dict.set("Type", Object::Name("Font".to_string()));
        courier_dict.set("Subtype", Object::Name("Type1".to_string()));
        courier_dict.set("BaseFont", Object::Name("Courier".to_string()));
        courier_dict.set("Encoding", Object::Name("WinAnsiEncoding".to_string()));
        font_dict.set("Courier", Object::Dictionary(courier_dict));

        let mut courier_bold_dict = Dictionary::new();
        courier_bold_dict.set("Type", Object::Name("Font".to_string()));
        courier_bold_dict.set("Subtype", Object::Name("Type1".to_string()));
        courier_bold_dict.set("BaseFont", Object::Name("Courier-Bold".to_string()));
        courier_bold_dict.set("Encoding", Object::Name("WinAnsiEncoding".to_string()));
        font_dict.set("Courier-Bold", Object::Dictionary(courier_bold_dict));

        let mut courier_oblique_dict = Dictionary::new();
        courier_oblique_dict.set("Type", Object::Name("Font".to_string()));
        courier_oblique_dict.set("Subtype", Object::Name("Type1".to_string()));
        courier_oblique_dict.set("BaseFont", Object::Name("Courier-Oblique".to_string()));
        courier_oblique_dict.set("Encoding", Object::Name("WinAnsiEncoding".to_string()));
        font_dict.set("Courier-Oblique", Object::Dictionary(courier_oblique_dict));

        let mut courier_bold_oblique_dict = Dictionary::new();
        courier_bold_oblique_dict.set("Type", Object::Name("Font".to_string()));
        courier_bold_oblique_dict.set("Subtype", Object::Name("Type1".to_string()));
        courier_bold_oblique_dict.set("BaseFont", Object::Name("Courier-BoldOblique".to_string()));
        courier_bold_oblique_dict.set("Encoding", Object::Name("WinAnsiEncoding".to_string()));
        font_dict.set(
            "Courier-BoldOblique",
            Object::Dictionary(courier_bold_oblique_dict),
        );

        // Add custom fonts (Type0 fonts for Unicode support)
        for (font_name, font_id) in font_refs {
            font_dict.set(font_name, Object::Reference(*font_id));
        }

        resources.set("Font", Object::Dictionary(font_dict));

        // Add images as XObjects
        if !page.images().is_empty() {
            let mut xobject_dict = Dictionary::new();

            for (name, image) in page.images() {
                // Use sequential ObjectId allocation to avoid conflicts
                let image_id = self.allocate_object_id();

                // Check if image has transparency (alpha channel)
                if image.has_transparency() {
                    // Handle transparent images with SMask
                    let (mut main_obj, smask_obj) = image.to_pdf_object_with_transparency();

                    // If we have a soft mask, write it as a separate object and reference it
                    if let Some(smask_stream) = smask_obj {
                        let smask_id = self.allocate_object_id();
                        self.write_object(smask_id, smask_stream)?;

                        // Add SMask reference to the main image dictionary
                        if let Object::Stream(ref mut dict, _) = main_obj {
                            dict.set("SMask", Object::Reference(smask_id));
                        }
                    }

                    // Write the main image XObject (now with SMask reference if applicable)
                    self.write_object(image_id, main_obj)?;
                } else {
                    // Write the image XObject without transparency
                    self.write_object(image_id, image.to_pdf_object())?;
                }

                // Add reference to XObject dictionary
                xobject_dict.set(name, Object::Reference(image_id));
            }

            resources.set("XObject", Object::Dictionary(xobject_dict));
        }

        // Add ExtGState resources for transparency
        if let Some(extgstate_states) = page.get_extgstate_resources() {
            let mut extgstate_dict = Dictionary::new();
            for (name, state) in extgstate_states {
                let mut state_dict = Dictionary::new();
                state_dict.set("Type", Object::Name("ExtGState".to_string()));

                // Add transparency parameters
                if let Some(alpha_stroke) = state.alpha_stroke {
                    state_dict.set("CA", Object::Real(alpha_stroke));
                }
                if let Some(alpha_fill) = state.alpha_fill {
                    state_dict.set("ca", Object::Real(alpha_fill));
                }

                // Add other parameters as needed
                if let Some(line_width) = state.line_width {
                    state_dict.set("LW", Object::Real(line_width));
                }
                if let Some(line_cap) = state.line_cap {
                    state_dict.set("LC", Object::Integer(line_cap as i64));
                }
                if let Some(line_join) = state.line_join {
                    state_dict.set("LJ", Object::Integer(line_join as i64));
                }
                if let Some(dash_pattern) = &state.dash_pattern {
                    let dash_objects: Vec<Object> = dash_pattern
                        .array
                        .iter()
                        .map(|&d| Object::Real(d))
                        .collect();
                    state_dict.set(
                        "D",
                        Object::Array(vec![
                            Object::Array(dash_objects),
                            Object::Real(dash_pattern.phase),
                        ]),
                    );
                }

                extgstate_dict.set(name, Object::Dictionary(state_dict));
            }
            if !extgstate_dict.is_empty() {
                resources.set("ExtGState", Object::Dictionary(extgstate_dict));
            }
        }

        page_dict.set("Resources", Object::Dictionary(resources));

        // Handle form widget annotations
        if let Some(Object::Array(annots)) = page_dict.get("Annots") {
            let mut new_annots = Vec::new();

            for annot in annots {
                if let Object::Dictionary(ref annot_dict) = annot {
                    if let Some(Object::Name(subtype)) = annot_dict.get("Subtype") {
                        if subtype == "Widget" {
                            // Process widget annotation
                            let widget_id = self.allocate_object_id();
                            self.write_object(widget_id, annot.clone())?;
                            new_annots.push(Object::Reference(widget_id));

                            // Track widget for form fields
                            if let Some(Object::Name(_ft)) = annot_dict.get("FT") {
                                if let Some(Object::String(field_name)) = annot_dict.get("T") {
                                    self.field_widget_map
                                        .entry(field_name.clone())
                                        .or_default()
                                        .push(widget_id);
                                    self.field_id_map.insert(field_name.clone(), widget_id);
                                    self.form_field_ids.push(widget_id);
                                }
                            }
                            continue;
                        }
                    }
                }
                new_annots.push(annot.clone());
            }

            if !new_annots.is_empty() {
                page_dict.set("Annots", Object::Array(new_annots));
            }
        }

        self.write_object(page_id, Object::Dictionary(page_dict))?;
        Ok(())
    }
}

impl PdfWriter<BufWriter<std::fs::File>> {
    pub fn new(path: impl AsRef<Path>) -> Result<Self> {
        let file = std::fs::File::create(path)?;
        let writer = BufWriter::new(file);

        Ok(Self {
            writer,
            xref_positions: HashMap::new(),
            current_position: 0,
            next_object_id: 1,
            catalog_id: None,
            pages_id: None,
            info_id: None,
            field_widget_map: HashMap::new(),
            field_id_map: HashMap::new(),
            form_field_ids: Vec::new(),
            page_ids: Vec::new(),
            config: WriterConfig::default(),
            document_used_chars: None,
            buffered_objects: HashMap::new(),
            compressed_object_map: HashMap::new(),
        })
    }
}

impl<W: Write> PdfWriter<W> {
    fn allocate_object_id(&mut self) -> ObjectId {
        let id = ObjectId::new(self.next_object_id, 0);
        self.next_object_id += 1;
        id
    }

    fn write_object(&mut self, id: ObjectId, object: Object) -> Result<()> {
        use crate::writer::ObjectStreamWriter;

        // If object streams enabled and object is compressible, buffer it
        if self.config.use_object_streams && ObjectStreamWriter::can_compress(&object) {
            let mut buffer = Vec::new();
            self.write_object_value_to_buffer(&object, &mut buffer)?;
            self.buffered_objects.insert(id, buffer);
            return Ok(());
        }

        // Otherwise write immediately (streams, encryption dicts, etc.)
        self.xref_positions.insert(id, self.current_position);

        // Pre-format header to count exact bytes once
        let header = format!("{} {} obj\n", id.number(), id.generation());
        self.write_bytes(header.as_bytes())?;

        self.write_object_value(&object)?;

        self.write_bytes(b"\nendobj\n")?;
        Ok(())
    }

    fn write_object_value(&mut self, object: &Object) -> Result<()> {
        match object {
            Object::Null => self.write_bytes(b"null")?,
            Object::Boolean(b) => self.write_bytes(if *b { b"true" } else { b"false" })?,
            Object::Integer(i) => self.write_bytes(i.to_string().as_bytes())?,
            Object::Real(f) => self.write_bytes(
                format!("{f:.6}")
                    .trim_end_matches('0')
                    .trim_end_matches('.')
                    .as_bytes(),
            )?,
            Object::String(s) => {
                self.write_bytes(b"(")?;
                self.write_bytes(s.as_bytes())?;
                self.write_bytes(b")")?;
            }
            Object::Name(n) => {
                self.write_bytes(b"/")?;
                self.write_bytes(n.as_bytes())?;
            }
            Object::Array(arr) => {
                self.write_bytes(b"[")?;
                for (i, obj) in arr.iter().enumerate() {
                    if i > 0 {
                        self.write_bytes(b" ")?;
                    }
                    self.write_object_value(obj)?;
                }
                self.write_bytes(b"]")?;
            }
            Object::Dictionary(dict) => {
                self.write_bytes(b"<<")?;
                for (key, value) in dict.entries() {
                    self.write_bytes(b"\n/")?;
                    self.write_bytes(key.as_bytes())?;
                    self.write_bytes(b" ")?;
                    self.write_object_value(value)?;
                }
                self.write_bytes(b"\n>>")?;
            }
            Object::Stream(dict, data) => {
                self.write_object_value(&Object::Dictionary(dict.clone()))?;
                self.write_bytes(b"\nstream\n")?;
                self.write_bytes(data)?;
                self.write_bytes(b"\nendstream")?;
            }
            Object::Reference(id) => {
                let ref_str = format!("{} {} R", id.number(), id.generation());
                self.write_bytes(ref_str.as_bytes())?;
            }
        }
        Ok(())
    }

    /// Write object value to a buffer (for object streams)
    fn write_object_value_to_buffer(&self, object: &Object, buffer: &mut Vec<u8>) -> Result<()> {
        match object {
            Object::Null => buffer.extend_from_slice(b"null"),
            Object::Boolean(b) => buffer.extend_from_slice(if *b { b"true" } else { b"false" }),
            Object::Integer(i) => buffer.extend_from_slice(i.to_string().as_bytes()),
            Object::Real(f) => buffer.extend_from_slice(
                format!("{f:.6}")
                    .trim_end_matches('0')
                    .trim_end_matches('.')
                    .as_bytes(),
            ),
            Object::String(s) => {
                buffer.push(b'(');
                buffer.extend_from_slice(s.as_bytes());
                buffer.push(b')');
            }
            Object::Name(n) => {
                buffer.push(b'/');
                buffer.extend_from_slice(n.as_bytes());
            }
            Object::Array(arr) => {
                buffer.push(b'[');
                for (i, obj) in arr.iter().enumerate() {
                    if i > 0 {
                        buffer.push(b' ');
                    }
                    self.write_object_value_to_buffer(obj, buffer)?;
                }
                buffer.push(b']');
            }
            Object::Dictionary(dict) => {
                buffer.extend_from_slice(b"<<");
                for (key, value) in dict.entries() {
                    buffer.extend_from_slice(b"\n/");
                    buffer.extend_from_slice(key.as_bytes());
                    buffer.push(b' ');
                    self.write_object_value_to_buffer(value, buffer)?;
                }
                buffer.extend_from_slice(b"\n>>");
            }
            Object::Stream(_, _) => {
                // Streams should never be compressed in object streams
                return Err(crate::error::PdfError::ObjectStreamError(
                    "Cannot compress stream objects in object streams".to_string(),
                ));
            }
            Object::Reference(id) => {
                let ref_str = format!("{} {} R", id.number(), id.generation());
                buffer.extend_from_slice(ref_str.as_bytes());
            }
        }
        Ok(())
    }

    /// Flush buffered objects as compressed object streams
    fn flush_object_streams(&mut self) -> Result<()> {
        if self.buffered_objects.is_empty() {
            return Ok(());
        }

        // Create object stream writer
        let config = ObjectStreamConfig {
            max_objects_per_stream: 100,
            compression_level: 6,
            enabled: true,
        };
        let mut os_writer = ObjectStreamWriter::new(config);

        // Sort buffered objects by ID for deterministic output
        let mut buffered: Vec<_> = self.buffered_objects.iter().collect();
        buffered.sort_by_key(|(id, _)| id.number());

        // Add all buffered objects to the stream writer
        for (id, data) in buffered {
            os_writer.add_object(*id, data.clone())?;
        }

        // Finalize and get completed streams
        let streams = os_writer.finalize()?;

        // Write each object stream to the PDF
        for mut stream in streams {
            let stream_id = stream.stream_id;

            // Generate compressed stream data
            let compressed_data = stream.generate_stream_data(6)?;

            // Generate stream dictionary
            let dict = stream.generate_dictionary(&compressed_data);

            // Track compressed object mapping for xref
            for (index, (obj_id, _)) in stream.objects.iter().enumerate() {
                self.compressed_object_map
                    .insert(*obj_id, (stream_id, index as u32));
            }

            // Write the object stream itself
            self.xref_positions.insert(stream_id, self.current_position);

            let header = format!("{} {} obj\n", stream_id.number(), stream_id.generation());
            self.write_bytes(header.as_bytes())?;

            self.write_object_value(&Object::Dictionary(dict))?;

            self.write_bytes(b"\nstream\n")?;
            self.write_bytes(&compressed_data)?;
            self.write_bytes(b"\nendstream\nendobj\n")?;
        }

        Ok(())
    }

    fn write_xref(&mut self) -> Result<()> {
        self.write_bytes(b"xref\n")?;

        // Sort by object number and write entries
        let mut entries: Vec<_> = self
            .xref_positions
            .iter()
            .map(|(id, pos)| (*id, *pos))
            .collect();
        entries.sort_by_key(|(id, _)| id.number());

        // Find the highest object number to determine size
        let max_obj_num = entries.iter().map(|(id, _)| id.number()).max().unwrap_or(0);

        // Write subsection header - PDF 1.7 spec allows multiple subsections
        // For simplicity, write one subsection from 0 to max
        self.write_bytes(b"0 ")?;
        self.write_bytes((max_obj_num + 1).to_string().as_bytes())?;
        self.write_bytes(b"\n")?;

        // Write free object entry
        self.write_bytes(b"0000000000 65535 f \n")?;

        // Write entries for all object numbers from 1 to max
        // Fill in gaps with free entries
        for obj_num in 1..=max_obj_num {
            let _obj_id = ObjectId::new(obj_num, 0);
            if let Some((_, position)) = entries.iter().find(|(id, _)| id.number() == obj_num) {
                let entry = format!("{:010} {:05} n \n", position, 0);
                self.write_bytes(entry.as_bytes())?;
            } else {
                // Free entry for gap
                self.write_bytes(b"0000000000 00000 f \n")?;
            }
        }

        Ok(())
    }

    fn write_xref_stream(&mut self) -> Result<()> {
        let catalog_id = self.catalog_id.expect("catalog_id must be set");
        let info_id = self.info_id.expect("info_id must be set");

        // Allocate object ID for the xref stream
        let xref_stream_id = self.allocate_object_id();
        let xref_position = self.current_position;

        // Create XRef stream writer with trailer information
        let mut xref_writer = XRefStreamWriter::new(xref_stream_id);
        xref_writer.set_trailer_info(catalog_id, info_id);

        // Add free entry for object 0
        xref_writer.add_free_entry(0, 65535);

        // Sort entries by object number
        let mut entries: Vec<_> = self
            .xref_positions
            .iter()
            .map(|(id, pos)| (*id, *pos))
            .collect();
        entries.sort_by_key(|(id, _)| id.number());

        // Find the highest object number (including the xref stream itself)
        let max_obj_num = entries
            .iter()
            .map(|(id, _)| id.number())
            .max()
            .unwrap_or(0)
            .max(xref_stream_id.number());

        // Add entries for all objects (including compressed objects)
        for obj_num in 1..=max_obj_num {
            let obj_id = ObjectId::new(obj_num, 0);

            if obj_num == xref_stream_id.number() {
                // The xref stream entry will be added with the correct position
                xref_writer.add_in_use_entry(xref_position, 0);
            } else if let Some((stream_id, index)) = self.compressed_object_map.get(&obj_id) {
                // Type 2: Object is compressed in an object stream
                xref_writer.add_compressed_entry(stream_id.number(), *index);
            } else if let Some((id, position)) =
                entries.iter().find(|(id, _)| id.number() == obj_num)
            {
                // Type 1: Regular in-use entry
                xref_writer.add_in_use_entry(*position, id.generation());
            } else {
                // Type 0: Free entry for gap
                xref_writer.add_free_entry(0, 0);
            }
        }

        // Mark position for xref stream object
        self.xref_positions.insert(xref_stream_id, xref_position);

        // Write object header
        self.write_bytes(
            format!(
                "{} {} obj\n",
                xref_stream_id.number(),
                xref_stream_id.generation()
            )
            .as_bytes(),
        )?;

        // Get the encoded data
        let uncompressed_data = xref_writer.encode_entries();
        let final_data = if self.config.compress_streams {
            crate::compression::compress(&uncompressed_data)?
        } else {
            uncompressed_data
        };

        // Create and write dictionary
        let mut dict = xref_writer.create_dictionary(None);
        dict.set("Length", Object::Integer(final_data.len() as i64));

        // Add filter if compression is enabled
        if self.config.compress_streams {
            dict.set("Filter", Object::Name("FlateDecode".to_string()));
        }
        self.write_bytes(b"<<")?;
        for (key, value) in dict.iter() {
            self.write_bytes(b"\n/")?;
            self.write_bytes(key.as_bytes())?;
            self.write_bytes(b" ")?;
            self.write_object_value(value)?;
        }
        self.write_bytes(b"\n>>\n")?;

        // Write stream
        self.write_bytes(b"stream\n")?;
        self.write_bytes(&final_data)?;
        self.write_bytes(b"\nendstream\n")?;
        self.write_bytes(b"endobj\n")?;

        // Write startxref and EOF
        self.write_bytes(b"\nstartxref\n")?;
        self.write_bytes(xref_position.to_string().as_bytes())?;
        self.write_bytes(b"\n%%EOF\n")?;

        Ok(())
    }

    fn write_trailer(&mut self, xref_position: u64) -> Result<()> {
        let catalog_id = self.catalog_id.expect("catalog_id must be set");
        let info_id = self.info_id.expect("info_id must be set");
        // Find the highest object number to determine size
        let max_obj_num = self
            .xref_positions
            .keys()
            .map(|id| id.number())
            .max()
            .unwrap_or(0);

        let mut trailer = Dictionary::new();
        trailer.set("Size", Object::Integer((max_obj_num + 1) as i64));
        trailer.set("Root", Object::Reference(catalog_id));
        trailer.set("Info", Object::Reference(info_id));

        self.write_bytes(b"trailer\n")?;
        self.write_object_value(&Object::Dictionary(trailer))?;
        self.write_bytes(b"\nstartxref\n")?;
        self.write_bytes(xref_position.to_string().as_bytes())?;
        self.write_bytes(b"\n%%EOF\n")?;

        Ok(())
    }

    fn write_bytes(&mut self, data: &[u8]) -> Result<()> {
        self.writer.write_all(data)?;
        self.current_position += data.len() as u64;
        Ok(())
    }

    #[allow(dead_code)]
    fn create_widget_appearance_stream(&mut self, widget_dict: &Dictionary) -> Result<ObjectId> {
        // Get widget rectangle
        let rect = if let Some(Object::Array(rect_array)) = widget_dict.get("Rect") {
            if rect_array.len() >= 4 {
                if let (
                    Some(Object::Real(x1)),
                    Some(Object::Real(y1)),
                    Some(Object::Real(x2)),
                    Some(Object::Real(y2)),
                ) = (
                    rect_array.first(),
                    rect_array.get(1),
                    rect_array.get(2),
                    rect_array.get(3),
                ) {
                    (*x1, *y1, *x2, *y2)
                } else {
                    (0.0, 0.0, 100.0, 20.0) // Default
                }
            } else {
                (0.0, 0.0, 100.0, 20.0) // Default
            }
        } else {
            (0.0, 0.0, 100.0, 20.0) // Default
        };

        let width = rect.2 - rect.0;
        let height = rect.3 - rect.1;

        // Create appearance stream content
        let mut content = String::new();

        // Set graphics state
        content.push_str("q\n");

        // Draw border (black)
        content.push_str("0 0 0 RG\n"); // Black stroke color
        content.push_str("1 w\n"); // 1pt line width

        // Draw rectangle border
        content.push_str(&format!("0 0 {width} {height} re\n"));
        content.push_str("S\n"); // Stroke

        // Fill with white background
        content.push_str("1 1 1 rg\n"); // White fill color
        content.push_str(&format!("0.5 0.5 {} {} re\n", width - 1.0, height - 1.0));
        content.push_str("f\n"); // Fill

        // Restore graphics state
        content.push_str("Q\n");

        // Create stream dictionary
        let mut stream_dict = Dictionary::new();
        stream_dict.set("Type", Object::Name("XObject".to_string()));
        stream_dict.set("Subtype", Object::Name("Form".to_string()));
        stream_dict.set(
            "BBox",
            Object::Array(vec![
                Object::Real(0.0),
                Object::Real(0.0),
                Object::Real(width),
                Object::Real(height),
            ]),
        );
        stream_dict.set("Resources", Object::Dictionary(Dictionary::new()));
        stream_dict.set("Length", Object::Integer(content.len() as i64));

        // Write the appearance stream
        let stream_id = self.allocate_object_id();
        self.write_object(stream_id, Object::Stream(stream_dict, content.into_bytes()))?;

        Ok(stream_id)
    }

    #[allow(dead_code)]
    fn create_field_appearance_stream(
        &mut self,
        field_dict: &Dictionary,
        widget: &crate::forms::Widget,
    ) -> Result<ObjectId> {
        let width = widget.rect.upper_right.x - widget.rect.lower_left.x;
        let height = widget.rect.upper_right.y - widget.rect.lower_left.y;

        // Create appearance stream content
        let mut content = String::new();

        // Set graphics state
        content.push_str("q\n");

        // Draw background if specified
        if let Some(bg_color) = &widget.appearance.background_color {
            match bg_color {
                crate::graphics::Color::Gray(g) => {
                    content.push_str(&format!("{g} g\n"));
                }
                crate::graphics::Color::Rgb(r, g, b) => {
                    content.push_str(&format!("{r} {g} {b} rg\n"));
                }
                crate::graphics::Color::Cmyk(c, m, y, k) => {
                    content.push_str(&format!("{c} {m} {y} {k} k\n"));
                }
            }
            content.push_str(&format!("0 0 {width} {height} re\n"));
            content.push_str("f\n");
        }

        // Draw border
        if let Some(border_color) = &widget.appearance.border_color {
            match border_color {
                crate::graphics::Color::Gray(g) => {
                    content.push_str(&format!("{g} G\n"));
                }
                crate::graphics::Color::Rgb(r, g, b) => {
                    content.push_str(&format!("{r} {g} {b} RG\n"));
                }
                crate::graphics::Color::Cmyk(c, m, y, k) => {
                    content.push_str(&format!("{c} {m} {y} {k} K\n"));
                }
            }
            content.push_str(&format!("{} w\n", widget.appearance.border_width));
            content.push_str(&format!("0 0 {width} {height} re\n"));
            content.push_str("S\n");
        }

        // For checkboxes, add a checkmark if checked
        if let Some(Object::Name(ft)) = field_dict.get("FT") {
            if ft == "Btn" {
                if let Some(Object::Name(v)) = field_dict.get("V") {
                    if v == "Yes" {
                        // Draw checkmark
                        content.push_str("0 0 0 RG\n"); // Black
                        content.push_str("2 w\n");
                        let margin = width * 0.2;
                        content.push_str(&format!("{} {} m\n", margin, height / 2.0));
                        content.push_str(&format!("{} {} l\n", width / 2.0, margin));
                        content.push_str(&format!("{} {} l\n", width - margin, height - margin));
                        content.push_str("S\n");
                    }
                }
            }
        }

        // Restore graphics state
        content.push_str("Q\n");

        // Create stream dictionary
        let mut stream_dict = Dictionary::new();
        stream_dict.set("Type", Object::Name("XObject".to_string()));
        stream_dict.set("Subtype", Object::Name("Form".to_string()));
        stream_dict.set(
            "BBox",
            Object::Array(vec![
                Object::Real(0.0),
                Object::Real(0.0),
                Object::Real(width),
                Object::Real(height),
            ]),
        );
        stream_dict.set("Resources", Object::Dictionary(Dictionary::new()));
        stream_dict.set("Length", Object::Integer(content.len() as i64));

        // Write the appearance stream
        let stream_id = self.allocate_object_id();
        self.write_object(stream_id, Object::Stream(stream_dict, content.into_bytes()))?;

        Ok(stream_id)
    }
}

/// Format a DateTime as a PDF date string (D:YYYYMMDDHHmmSSOHH'mm)
fn format_pdf_date(date: DateTime<Utc>) -> String {
    // Format the UTC date according to PDF specification
    // D:YYYYMMDDHHmmSSOHH'mm where O is the relationship of local time to UTC (+ or -)
    let formatted = date.format("D:%Y%m%d%H%M%S");

    // For UTC, the offset is always +00'00
    format!("{formatted}+00'00")
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod rigorous_tests;
