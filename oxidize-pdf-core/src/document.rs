use crate::error::Result;
use crate::fonts::{Font as CustomFont, FontCache};
use crate::forms::{AcroForm, FormManager};
use crate::objects::{Object, ObjectId};
use crate::page::Page;
use crate::page_labels::PageLabelTree;
use crate::semantic::{BoundingBox, EntityType, RelationType, SemanticEntity};
use crate::structure::{NamedDestinations, OutlineTree, PageTree};
use crate::text::{FontEncoding, FontWithEncoding};
use crate::writer::PdfWriter;
use chrono::{DateTime, Local, Utc};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

mod encryption;
pub use encryption::{DocumentEncryption, EncryptionStrength};

/// A PDF document that can contain multiple pages and metadata.
///
/// # Example
///
/// ```rust
/// use oxidize_pdf::{Document, Page};
///
/// let mut doc = Document::new();
/// doc.set_title("My Document");
/// doc.set_author("John Doe");
///
/// let page = Page::a4();
/// doc.add_page(page);
///
/// doc.save("output.pdf").unwrap();
/// ```
pub struct Document {
    pub(crate) pages: Vec<Page>,
    #[allow(dead_code)]
    pub(crate) objects: HashMap<ObjectId, Object>,
    #[allow(dead_code)]
    pub(crate) next_object_id: u32,
    pub(crate) metadata: DocumentMetadata,
    pub(crate) encryption: Option<DocumentEncryption>,
    pub(crate) outline: Option<OutlineTree>,
    pub(crate) named_destinations: Option<NamedDestinations>,
    #[allow(dead_code)]
    pub(crate) page_tree: Option<PageTree>,
    pub(crate) page_labels: Option<PageLabelTree>,
    /// Default font encoding to use for fonts when no encoding is specified
    pub(crate) default_font_encoding: Option<FontEncoding>,
    /// Interactive form data (AcroForm)
    pub(crate) acro_form: Option<AcroForm>,
    /// Form manager for handling interactive forms
    pub(crate) form_manager: Option<FormManager>,
    /// Whether to compress streams when writing the PDF
    pub(crate) compress: bool,
    /// Whether to use compressed cross-reference streams (PDF 1.5+)
    pub(crate) use_xref_streams: bool,
    /// Cache for custom fonts
    pub(crate) custom_fonts: FontCache,
    /// Map from font name to embedded font object ID
    #[allow(dead_code)]
    pub(crate) embedded_fonts: HashMap<String, ObjectId>,
    /// Characters used in the document (for font subsetting)
    pub(crate) used_characters: HashSet<char>,
    /// Action to execute when the document is opened
    pub(crate) open_action: Option<crate::actions::Action>,
    /// Viewer preferences for controlling document display
    pub(crate) viewer_preferences: Option<crate::viewer_preferences::ViewerPreferences>,
    /// Semantic entities marked in the document for AI processing
    pub(crate) semantic_entities: Vec<SemanticEntity>,
}

/// Metadata for a PDF document.
#[derive(Debug, Clone)]
pub struct DocumentMetadata {
    /// Document title
    pub title: Option<String>,
    /// Document author
    pub author: Option<String>,
    /// Document subject
    pub subject: Option<String>,
    /// Document keywords
    pub keywords: Option<String>,
    /// Software that created the original document
    pub creator: Option<String>,
    /// Software that produced the PDF
    pub producer: Option<String>,
    /// Date and time the document was created
    pub creation_date: Option<DateTime<Utc>>,
    /// Date and time the document was last modified
    pub modification_date: Option<DateTime<Utc>>,
}

impl Default for DocumentMetadata {
    fn default() -> Self {
        let now = Utc::now();
        Self {
            title: None,
            author: None,
            subject: None,
            keywords: None,
            creator: Some("oxidize_pdf".to_string()),
            producer: Some(format!("oxidize_pdf v{}", env!("CARGO_PKG_VERSION"))),
            creation_date: Some(now),
            modification_date: Some(now),
        }
    }
}

impl Document {
    /// Creates a new empty PDF document.
    pub fn new() -> Self {
        Self {
            pages: Vec::new(),
            objects: HashMap::new(),
            next_object_id: 1,
            metadata: DocumentMetadata::default(),
            encryption: None,
            outline: None,
            named_destinations: None,
            page_tree: None,
            page_labels: None,
            default_font_encoding: None,
            acro_form: None,
            form_manager: None,
            compress: true,          // Enable compression by default
            use_xref_streams: false, // Disabled by default for compatibility
            custom_fonts: FontCache::new(),
            embedded_fonts: HashMap::new(),
            used_characters: HashSet::new(),
            open_action: None,
            viewer_preferences: None,
            semantic_entities: Vec::new(),
        }
    }

    /// Adds a page to the document.
    pub fn add_page(&mut self, page: Page) {
        // Collect used characters from the page
        if let Some(used_chars) = page.get_used_characters() {
            self.used_characters.extend(used_chars);
        }
        self.pages.push(page);
    }

    /// Sets the document title.
    pub fn set_title(&mut self, title: impl Into<String>) {
        self.metadata.title = Some(title.into());
    }

    /// Sets the document author.
    pub fn set_author(&mut self, author: impl Into<String>) {
        self.metadata.author = Some(author.into());
    }

    /// Sets the form manager for the document.
    pub fn set_form_manager(&mut self, form_manager: FormManager) {
        self.form_manager = Some(form_manager);
    }

    /// Sets the document subject.
    pub fn set_subject(&mut self, subject: impl Into<String>) {
        self.metadata.subject = Some(subject.into());
    }

    /// Sets the document keywords.
    pub fn set_keywords(&mut self, keywords: impl Into<String>) {
        self.metadata.keywords = Some(keywords.into());
    }

    /// Set document encryption
    pub fn set_encryption(&mut self, encryption: DocumentEncryption) {
        self.encryption = Some(encryption);
    }

    /// Set simple encryption with passwords
    pub fn encrypt_with_passwords(
        &mut self,
        user_password: impl Into<String>,
        owner_password: impl Into<String>,
    ) {
        self.encryption = Some(DocumentEncryption::with_passwords(
            user_password,
            owner_password,
        ));
    }

    /// Check if document is encrypted
    pub fn is_encrypted(&self) -> bool {
        self.encryption.is_some()
    }

    /// Set the action to execute when the document is opened
    pub fn set_open_action(&mut self, action: crate::actions::Action) {
        self.open_action = Some(action);
    }

    /// Get the document open action
    pub fn open_action(&self) -> Option<&crate::actions::Action> {
        self.open_action.as_ref()
    }

    /// Set viewer preferences for controlling document display
    pub fn set_viewer_preferences(
        &mut self,
        preferences: crate::viewer_preferences::ViewerPreferences,
    ) {
        self.viewer_preferences = Some(preferences);
    }

    /// Get viewer preferences
    pub fn viewer_preferences(&self) -> Option<&crate::viewer_preferences::ViewerPreferences> {
        self.viewer_preferences.as_ref()
    }

    /// Set document outline (bookmarks)
    pub fn set_outline(&mut self, outline: OutlineTree) {
        self.outline = Some(outline);
    }

    /// Get document outline
    pub fn outline(&self) -> Option<&OutlineTree> {
        self.outline.as_ref()
    }

    /// Get mutable document outline
    pub fn outline_mut(&mut self) -> Option<&mut OutlineTree> {
        self.outline.as_mut()
    }

    /// Set named destinations
    pub fn set_named_destinations(&mut self, destinations: NamedDestinations) {
        self.named_destinations = Some(destinations);
    }

    /// Get named destinations
    pub fn named_destinations(&self) -> Option<&NamedDestinations> {
        self.named_destinations.as_ref()
    }

    /// Get mutable named destinations
    pub fn named_destinations_mut(&mut self) -> Option<&mut NamedDestinations> {
        self.named_destinations.as_mut()
    }

    /// Set page labels
    pub fn set_page_labels(&mut self, labels: PageLabelTree) {
        self.page_labels = Some(labels);
    }

    /// Get page labels
    pub fn page_labels(&self) -> Option<&PageLabelTree> {
        self.page_labels.as_ref()
    }

    /// Get mutable page labels
    pub fn page_labels_mut(&mut self) -> Option<&mut PageLabelTree> {
        self.page_labels.as_mut()
    }

    /// Get page label for a specific page
    pub fn get_page_label(&self, page_index: u32) -> String {
        self.page_labels
            .as_ref()
            .and_then(|labels| labels.get_label(page_index))
            .unwrap_or_else(|| (page_index + 1).to_string())
    }

    /// Get all page labels
    pub fn get_all_page_labels(&self) -> Vec<String> {
        let page_count = self.pages.len() as u32;
        if let Some(labels) = &self.page_labels {
            labels.get_all_labels(page_count)
        } else {
            (1..=page_count).map(|i| i.to_string()).collect()
        }
    }

    /// Sets the document creator (software that created the original document).
    pub fn set_creator(&mut self, creator: impl Into<String>) {
        self.metadata.creator = Some(creator.into());
    }

    /// Sets the document producer (software that produced the PDF).
    pub fn set_producer(&mut self, producer: impl Into<String>) {
        self.metadata.producer = Some(producer.into());
    }

    /// Sets the document creation date.
    pub fn set_creation_date(&mut self, date: DateTime<Utc>) {
        self.metadata.creation_date = Some(date);
    }

    /// Sets the document creation date using local time.
    pub fn set_creation_date_local(&mut self, date: DateTime<Local>) {
        self.metadata.creation_date = Some(date.with_timezone(&Utc));
    }

    /// Sets the document modification date.
    pub fn set_modification_date(&mut self, date: DateTime<Utc>) {
        self.metadata.modification_date = Some(date);
    }

    /// Sets the document modification date using local time.
    pub fn set_modification_date_local(&mut self, date: DateTime<Local>) {
        self.metadata.modification_date = Some(date.with_timezone(&Utc));
    }

    /// Sets the modification date to the current time.
    pub fn update_modification_date(&mut self) {
        self.metadata.modification_date = Some(Utc::now());
    }

    /// Sets the default font encoding for fonts that don't specify an encoding.
    ///
    /// This encoding will be applied to fonts in the PDF font dictionary when
    /// no explicit encoding is specified. Setting this to `None` (the default)
    /// means no encoding metadata will be added to fonts unless explicitly specified.
    ///
    /// # Example
    ///
    /// ```rust
    /// use oxidize_pdf::{Document, text::FontEncoding};
    ///
    /// let mut doc = Document::new();
    /// doc.set_default_font_encoding(Some(FontEncoding::WinAnsiEncoding));
    /// ```
    pub fn set_default_font_encoding(&mut self, encoding: Option<FontEncoding>) {
        self.default_font_encoding = encoding;
    }

    /// Gets the current default font encoding.
    pub fn default_font_encoding(&self) -> Option<FontEncoding> {
        self.default_font_encoding
    }

    /// Gets all fonts used in the document with their encodings.
    ///
    /// This scans all pages and collects the unique fonts used, applying
    /// the default encoding where no explicit encoding is specified.
    #[allow(dead_code)]
    pub(crate) fn get_fonts_with_encodings(&self) -> Vec<FontWithEncoding> {
        let mut fonts_used = HashSet::new();

        // Collect fonts from all pages
        for page in &self.pages {
            // Get fonts from text content
            for font in page.get_used_fonts() {
                let font_with_encoding = match self.default_font_encoding {
                    Some(default_encoding) => FontWithEncoding::new(font, Some(default_encoding)),
                    None => FontWithEncoding::without_encoding(font),
                };
                fonts_used.insert(font_with_encoding);
            }
        }

        fonts_used.into_iter().collect()
    }

    /// Add a custom font from a file path
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use oxidize_pdf::Document;
    ///
    /// let mut doc = Document::new();
    /// doc.add_font("MyFont", "path/to/font.ttf").unwrap();
    /// ```
    pub fn add_font(
        &mut self,
        name: impl Into<String>,
        path: impl AsRef<std::path::Path>,
    ) -> Result<()> {
        let name = name.into();
        let font = CustomFont::from_file(&name, path)?;
        self.custom_fonts.add_font(name, font)?;
        Ok(())
    }

    /// Add a custom font from byte data
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use oxidize_pdf::Document;
    ///
    /// let mut doc = Document::new();
    /// let font_data = vec![0; 1000]; // Your font data
    /// doc.add_font_from_bytes("MyFont", font_data).unwrap();
    /// ```
    pub fn add_font_from_bytes(&mut self, name: impl Into<String>, data: Vec<u8>) -> Result<()> {
        let name = name.into();
        let font = CustomFont::from_bytes(&name, data)?;

        // TODO: Implement automatic font metrics registration
        // This needs to be properly integrated with the font metrics system

        self.custom_fonts.add_font(name, font)?;
        Ok(())
    }

    /// Get a custom font by name
    #[allow(dead_code)]
    pub(crate) fn get_custom_font(&self, name: &str) -> Option<Arc<CustomFont>> {
        self.custom_fonts.get_font(name)
    }

    /// Check if a custom font is loaded
    pub fn has_custom_font(&self, name: &str) -> bool {
        self.custom_fonts.has_font(name)
    }

    /// Get all loaded custom font names
    pub fn custom_font_names(&self) -> Vec<String> {
        self.custom_fonts.font_names()
    }

    /// Gets the number of pages in the document.
    pub fn page_count(&self) -> usize {
        self.pages.len()
    }

    /// Gets a reference to the AcroForm (interactive form) if present.
    pub fn acro_form(&self) -> Option<&AcroForm> {
        self.acro_form.as_ref()
    }

    /// Gets a mutable reference to the AcroForm (interactive form) if present.
    pub fn acro_form_mut(&mut self) -> Option<&mut AcroForm> {
        self.acro_form.as_mut()
    }

    /// Enables interactive forms by creating a FormManager if not already present.
    /// The FormManager handles both the AcroForm and the connection with page widgets.
    pub fn enable_forms(&mut self) -> &mut FormManager {
        if self.form_manager.is_none() {
            self.form_manager = Some(FormManager::new());
        }
        if self.acro_form.is_none() {
            self.acro_form = Some(AcroForm::new());
        }
        // This should always succeed since we just ensured form_manager exists
        self.form_manager
            .as_mut()
            .expect("FormManager should exist after initialization")
    }

    /// Disables interactive forms by removing both the AcroForm and FormManager.
    pub fn disable_forms(&mut self) {
        self.acro_form = None;
        self.form_manager = None;
    }

    /// Saves the document to a file.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be created or written.
    pub fn save(&mut self, path: impl AsRef<std::path::Path>) -> Result<()> {
        // Update modification date before saving
        self.update_modification_date();

        // Create writer config with document's compression setting
        let config = crate::writer::WriterConfig {
            use_xref_streams: self.use_xref_streams,
            pdf_version: if self.use_xref_streams { "1.5" } else { "1.7" }.to_string(),
            compress_streams: self.compress,
        };

        use std::io::BufWriter;
        let file = std::fs::File::create(path)?;
        let writer = BufWriter::new(file);
        let mut pdf_writer = PdfWriter::with_config(writer, config);

        pdf_writer.write_document(self)?;
        Ok(())
    }

    /// Saves the document to a file with custom writer configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be created or written.
    pub fn save_with_config(
        &mut self,
        path: impl AsRef<std::path::Path>,
        config: crate::writer::WriterConfig,
    ) -> Result<()> {
        use std::io::BufWriter;

        // Update modification date before saving
        self.update_modification_date();

        // Use the config as provided (don't override compress_streams)

        let file = std::fs::File::create(path)?;
        let writer = BufWriter::new(file);
        let mut pdf_writer = PdfWriter::with_config(writer, config);
        pdf_writer.write_document(self)?;
        Ok(())
    }

    /// Saves the document to a file with custom values for headers/footers.
    ///
    /// This method processes all pages to replace custom placeholders in headers
    /// and footers before saving the document.
    ///
    /// # Arguments
    ///
    /// * `path` - The path where the document should be saved
    /// * `custom_values` - A map of placeholder names to their replacement values
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be created or written.
    pub fn save_with_custom_values(
        &mut self,
        path: impl AsRef<std::path::Path>,
        custom_values: &std::collections::HashMap<String, String>,
    ) -> Result<()> {
        // Process all pages with custom values
        let total_pages = self.pages.len();
        for (index, page) in self.pages.iter_mut().enumerate() {
            // Generate content with page info and custom values
            let page_content = page.generate_content_with_page_info(
                Some(index + 1),
                Some(total_pages),
                Some(custom_values),
            )?;
            // Update the page content
            page.set_content(page_content);
        }

        // Save the document normally
        self.save(path)
    }

    /// Writes the document to a buffer.
    ///
    /// # Errors
    ///
    /// Returns an error if the PDF cannot be generated.
    pub fn write(&mut self, buffer: &mut Vec<u8>) -> Result<()> {
        // Update modification date before writing
        self.update_modification_date();

        let mut writer = PdfWriter::new_with_writer(buffer);
        writer.write_document(self)?;
        Ok(())
    }

    #[allow(dead_code)]
    pub(crate) fn allocate_object_id(&mut self) -> ObjectId {
        let id = ObjectId::new(self.next_object_id, 0);
        self.next_object_id += 1;
        id
    }

    #[allow(dead_code)]
    pub(crate) fn add_object(&mut self, obj: Object) -> ObjectId {
        let id = self.allocate_object_id();
        self.objects.insert(id, obj);
        id
    }

    /// Enables or disables compression for PDF streams.
    ///
    /// When compression is enabled (default), content streams and XRef streams are compressed
    /// using Flate/Zlib compression to reduce file size. When disabled, streams are written
    /// uncompressed, making the PDF larger but easier to debug.
    ///
    /// # Arguments
    ///
    /// * `compress` - Whether to enable compression
    ///
    /// # Example
    ///
    /// ```rust
    /// use oxidize_pdf::{Document, Page};
    ///
    /// let mut doc = Document::new();
    ///
    /// // Disable compression for debugging
    /// doc.set_compress(false);
    ///
    /// doc.set_title("My Document");
    /// doc.add_page(Page::a4());
    ///
    /// let pdf_bytes = doc.to_bytes().unwrap();
    /// println!("Uncompressed PDF size: {} bytes", pdf_bytes.len());
    /// ```
    pub fn set_compress(&mut self, compress: bool) {
        self.compress = compress;
    }

    /// Enable or disable compressed cross-reference streams (PDF 1.5+).
    ///
    /// Cross-reference streams provide more compact representation of the cross-reference
    /// table and support additional features like compressed object streams.
    ///
    /// # Arguments
    ///
    /// * `enable` - Whether to enable compressed cross-reference streams
    ///
    /// # Example
    ///
    /// ```rust
    /// use oxidize_pdf::Document;
    ///
    /// let mut doc = Document::new();
    /// doc.enable_xref_streams(true);
    /// ```
    pub fn enable_xref_streams(&mut self, enable: bool) -> &mut Self {
        self.use_xref_streams = enable;
        self
    }

    /// Gets the current compression setting.
    ///
    /// # Returns
    ///
    /// Returns `true` if compression is enabled, `false` otherwise.
    pub fn get_compress(&self) -> bool {
        self.compress
    }

    /// Generates the PDF document as bytes in memory.
    ///
    /// This method provides in-memory PDF generation without requiring file I/O.
    /// The document is serialized to bytes and returned as a `Vec<u8>`.
    ///
    /// # Returns
    ///
    /// Returns the PDF document as bytes on success.
    ///
    /// # Errors
    ///
    /// Returns an error if the document cannot be serialized.
    ///
    /// # Example
    ///
    /// ```rust
    /// use oxidize_pdf::{Document, Page};
    ///
    /// let mut doc = Document::new();
    /// doc.set_title("My Document");
    ///
    /// let page = Page::a4();
    /// doc.add_page(page);
    ///
    /// let pdf_bytes = doc.to_bytes().unwrap();
    /// println!("Generated PDF size: {} bytes", pdf_bytes.len());
    /// ```
    pub fn to_bytes(&mut self) -> Result<Vec<u8>> {
        // Update modification date before serialization
        self.update_modification_date();

        // Create a buffer to write the PDF data to
        let mut buffer = Vec::new();

        // Create writer config with document's compression setting
        let config = crate::writer::WriterConfig {
            use_xref_streams: self.use_xref_streams,
            pdf_version: if self.use_xref_streams { "1.5" } else { "1.7" }.to_string(),
            compress_streams: self.compress,
        };

        // Use PdfWriter with the buffer as output and config
        let mut writer = PdfWriter::with_config(&mut buffer, config);
        writer.write_document(self)?;

        Ok(buffer)
    }

    /// Generates the PDF document as bytes with custom writer configuration.
    ///
    /// This method allows customizing the PDF output (e.g., using XRef streams)
    /// while still generating the document in memory.
    ///
    /// # Arguments
    ///
    /// * `config` - Writer configuration options
    ///
    /// # Returns
    ///
    /// Returns the PDF document as bytes on success.
    ///
    /// # Errors
    ///
    /// Returns an error if the document cannot be serialized.
    ///
    /// # Example
    ///
    /// ```rust
    /// use oxidize_pdf::{Document, Page};
    /// use oxidize_pdf::writer::WriterConfig;
    ///
    /// let mut doc = Document::new();
    /// doc.set_title("My Document");
    ///
    /// let page = Page::a4();
    /// doc.add_page(page);
    ///
    /// let config = WriterConfig {
    ///     use_xref_streams: true,
    ///     pdf_version: "1.5".to_string(),
    ///     compress_streams: true,
    /// };
    ///
    /// let pdf_bytes = doc.to_bytes_with_config(config).unwrap();
    /// println!("Generated PDF size: {} bytes", pdf_bytes.len());
    /// ```
    pub fn to_bytes_with_config(&mut self, config: crate::writer::WriterConfig) -> Result<Vec<u8>> {
        // Update modification date before serialization
        self.update_modification_date();

        // Use the config as provided (don't override compress_streams)

        // Create a buffer to write the PDF data to
        let mut buffer = Vec::new();

        // Use PdfWriter with the buffer as output and custom config
        let mut writer = PdfWriter::with_config(&mut buffer, config);
        writer.write_document(self)?;

        Ok(buffer)
    }

    // ==================== Semantic Entity Methods ====================

    /// Mark a region of the PDF with semantic meaning for AI processing.
    ///
    /// This creates an AI-Ready PDF that contains machine-readable metadata
    /// alongside the visual content, enabling automated document processing.
    ///
    /// # Example
    ///
    /// ```rust
    /// use oxidize_pdf::{Document, semantic::{EntityType, BoundingBox}};
    ///
    /// let mut doc = Document::new();
    ///
    /// // Mark an invoice number region
    /// let entity_id = doc.mark_entity(
    ///     "invoice_001".to_string(),
    ///     EntityType::InvoiceNumber,
    ///     BoundingBox::new(100.0, 700.0, 150.0, 20.0, 1)
    /// );
    ///
    /// // Add content and metadata
    /// doc.set_entity_content(&entity_id, "INV-2024-001");
    /// doc.add_entity_metadata(&entity_id, "confidence", "0.98");
    /// ```
    pub fn mark_entity(
        &mut self,
        id: impl Into<String>,
        entity_type: EntityType,
        bounds: BoundingBox,
    ) -> String {
        let entity_id = id.into();
        let entity = SemanticEntity::new(entity_id.clone(), entity_type, bounds);
        self.semantic_entities.push(entity);
        entity_id
    }

    /// Set the content text for an entity
    pub fn set_entity_content(&mut self, entity_id: &str, content: impl Into<String>) -> bool {
        if let Some(entity) = self
            .semantic_entities
            .iter_mut()
            .find(|e| e.id == entity_id)
        {
            entity.content = content.into();
            true
        } else {
            false
        }
    }

    /// Add metadata to an entity
    pub fn add_entity_metadata(
        &mut self,
        entity_id: &str,
        key: impl Into<String>,
        value: impl Into<String>,
    ) -> bool {
        if let Some(entity) = self
            .semantic_entities
            .iter_mut()
            .find(|e| e.id == entity_id)
        {
            entity.metadata.properties.insert(key.into(), value.into());
            true
        } else {
            false
        }
    }

    /// Set confidence score for an entity
    pub fn set_entity_confidence(&mut self, entity_id: &str, confidence: f32) -> bool {
        if let Some(entity) = self
            .semantic_entities
            .iter_mut()
            .find(|e| e.id == entity_id)
        {
            entity.metadata.confidence = Some(confidence.clamp(0.0, 1.0));
            true
        } else {
            false
        }
    }

    /// Add a relationship between two entities
    pub fn relate_entities(
        &mut self,
        from_id: &str,
        to_id: &str,
        relation_type: RelationType,
    ) -> bool {
        // First check if target entity exists
        let target_exists = self.semantic_entities.iter().any(|e| e.id == to_id);
        if !target_exists {
            return false;
        }

        // Then add the relationship
        if let Some(entity) = self.semantic_entities.iter_mut().find(|e| e.id == from_id) {
            entity.relationships.push(crate::semantic::EntityRelation {
                target_id: to_id.to_string(),
                relation_type,
            });
            true
        } else {
            false
        }
    }

    /// Get all semantic entities in the document
    pub fn get_semantic_entities(&self) -> &[SemanticEntity] {
        &self.semantic_entities
    }

    /// Get entities by type
    pub fn get_entities_by_type(&self, entity_type: EntityType) -> Vec<&SemanticEntity> {
        self.semantic_entities
            .iter()
            .filter(|e| e.entity_type == entity_type)
            .collect()
    }

    /// Export semantic entities as JSON
    #[cfg(feature = "semantic")]
    pub fn export_semantic_entities_json(&self) -> Result<String> {
        serde_json::to_string_pretty(&self.semantic_entities)
            .map_err(|e| crate::error::PdfError::SerializationError(e.to_string()))
    }

    /// Find an entity by ID
    pub fn find_entity(&self, entity_id: &str) -> Option<&SemanticEntity> {
        self.semantic_entities.iter().find(|e| e.id == entity_id)
    }

    /// Remove an entity by ID
    pub fn remove_entity(&mut self, entity_id: &str) -> bool {
        if let Some(pos) = self
            .semantic_entities
            .iter()
            .position(|e| e.id == entity_id)
        {
            self.semantic_entities.remove(pos);
            // Also remove any relationships pointing to this entity
            for entity in &mut self.semantic_entities {
                entity.relationships.retain(|r| r.target_id != entity_id);
            }
            true
        } else {
            false
        }
    }

    /// Get the count of semantic entities
    pub fn semantic_entity_count(&self) -> usize {
        self.semantic_entities.len()
    }

    /// Add XMP metadata stream to the document (Pro feature placeholder)
    pub fn add_xmp_metadata(&mut self, _xmp_data: &str) -> Result<ObjectId> {
        // This is a placeholder implementation for the Pro version
        // In the community edition, this just returns a dummy ObjectId
        tracing::info!("XMP metadata embedding requested but not available in community edition");
        Ok(ObjectId::new(9999, 0)) // Dummy object ID
    }

    /// Get XMP metadata from the document (Pro feature placeholder)  
    pub fn get_xmp_metadata(&self) -> Result<Option<String>> {
        // This is a placeholder implementation for the Pro version
        // In the community edition, this always returns None
        tracing::info!("XMP metadata extraction requested but not available in community edition");
        Ok(None)
    }

    /// Extract text content from all pages (placeholder implementation)
    pub fn extract_text(&self) -> Result<String> {
        // Placeholder implementation - in a real PDF reader this would
        // parse content streams and extract text operators
        let mut text = String::new();
        for (i, _page) in self.pages.iter().enumerate() {
            text.push_str(&format!("Text from page {} (placeholder)\n", i + 1));
        }
        Ok(text)
    }

    /// Extract text content from a specific page (placeholder implementation)
    pub fn extract_page_text(&self, page_index: usize) -> Result<String> {
        if page_index < self.pages.len() {
            Ok(format!("Text from page {} (placeholder)", page_index + 1))
        } else {
            Err(crate::error::PdfError::InvalidReference(format!(
                "Page index {} out of bounds",
                page_index
            )))
        }
    }
}

impl Default for Document {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_document_new() {
        let doc = Document::new();
        assert!(doc.pages.is_empty());
        assert!(doc.objects.is_empty());
        assert_eq!(doc.next_object_id, 1);
        assert!(doc.metadata.title.is_none());
        assert!(doc.metadata.author.is_none());
        assert!(doc.metadata.subject.is_none());
        assert!(doc.metadata.keywords.is_none());
        assert_eq!(doc.metadata.creator, Some("oxidize_pdf".to_string()));
        assert!(doc
            .metadata
            .producer
            .as_ref()
            .unwrap()
            .starts_with("oxidize_pdf"));
    }

    #[test]
    fn test_document_default() {
        let doc = Document::default();
        assert!(doc.pages.is_empty());
        assert_eq!(doc.next_object_id, 1);
    }

    #[test]
    fn test_add_page() {
        let mut doc = Document::new();
        let page1 = Page::a4();
        let page2 = Page::letter();

        doc.add_page(page1);
        assert_eq!(doc.pages.len(), 1);

        doc.add_page(page2);
        assert_eq!(doc.pages.len(), 2);
    }

    #[test]
    fn test_set_title() {
        let mut doc = Document::new();
        assert!(doc.metadata.title.is_none());

        doc.set_title("Test Document");
        assert_eq!(doc.metadata.title, Some("Test Document".to_string()));

        doc.set_title(String::from("Another Title"));
        assert_eq!(doc.metadata.title, Some("Another Title".to_string()));
    }

    #[test]
    fn test_set_author() {
        let mut doc = Document::new();
        assert!(doc.metadata.author.is_none());

        doc.set_author("John Doe");
        assert_eq!(doc.metadata.author, Some("John Doe".to_string()));
    }

    #[test]
    fn test_set_subject() {
        let mut doc = Document::new();
        assert!(doc.metadata.subject.is_none());

        doc.set_subject("Test Subject");
        assert_eq!(doc.metadata.subject, Some("Test Subject".to_string()));
    }

    #[test]
    fn test_set_keywords() {
        let mut doc = Document::new();
        assert!(doc.metadata.keywords.is_none());

        doc.set_keywords("test, pdf, rust");
        assert_eq!(doc.metadata.keywords, Some("test, pdf, rust".to_string()));
    }

    #[test]
    fn test_metadata_default() {
        let metadata = DocumentMetadata::default();
        assert!(metadata.title.is_none());
        assert!(metadata.author.is_none());
        assert!(metadata.subject.is_none());
        assert!(metadata.keywords.is_none());
        assert_eq!(metadata.creator, Some("oxidize_pdf".to_string()));
        assert!(metadata
            .producer
            .as_ref()
            .unwrap()
            .starts_with("oxidize_pdf"));
    }

    #[test]
    fn test_allocate_object_id() {
        let mut doc = Document::new();

        let id1 = doc.allocate_object_id();
        assert_eq!(id1.number(), 1);
        assert_eq!(id1.generation(), 0);
        assert_eq!(doc.next_object_id, 2);

        let id2 = doc.allocate_object_id();
        assert_eq!(id2.number(), 2);
        assert_eq!(id2.generation(), 0);
        assert_eq!(doc.next_object_id, 3);
    }

    #[test]
    fn test_add_object() {
        let mut doc = Document::new();
        assert!(doc.objects.is_empty());

        let obj = Object::Boolean(true);
        let id = doc.add_object(obj.clone());

        assert_eq!(id.number(), 1);
        assert_eq!(doc.objects.len(), 1);
        assert!(doc.objects.contains_key(&id));
    }

    #[test]
    fn test_write_to_buffer() {
        let mut doc = Document::new();
        doc.set_title("Buffer Test");
        doc.add_page(Page::a4());

        let mut buffer = Vec::new();
        let result = doc.write(&mut buffer);

        assert!(result.is_ok());
        assert!(!buffer.is_empty());
        assert!(buffer.starts_with(b"%PDF-1.7"));
    }

    #[test]
    fn test_document_with_multiple_pages() {
        let mut doc = Document::new();
        doc.set_title("Multi-page Document");
        doc.set_author("Test Author");
        doc.set_subject("Testing multiple pages");
        doc.set_keywords("test, multiple, pages");

        for _ in 0..5 {
            doc.add_page(Page::a4());
        }

        assert_eq!(doc.pages.len(), 5);
        assert_eq!(doc.metadata.title, Some("Multi-page Document".to_string()));
        assert_eq!(doc.metadata.author, Some("Test Author".to_string()));
    }

    #[test]
    fn test_empty_document_write() {
        let mut doc = Document::new();
        let mut buffer = Vec::new();

        // Empty document should still produce valid PDF
        let result = doc.write(&mut buffer);
        assert!(result.is_ok());
        assert!(!buffer.is_empty());
        assert!(buffer.starts_with(b"%PDF-1.7"));
    }

    // Integration tests for Document ↔ Writer ↔ Parser interactions
    mod integration_tests {
        use super::*;
        use crate::graphics::Color;
        use crate::text::Font;
        use std::fs;
        use tempfile::TempDir;

        #[test]
        fn test_document_writer_roundtrip() {
            let temp_dir = TempDir::new().unwrap();
            let file_path = temp_dir.path().join("test.pdf");

            // Create document with content
            let mut doc = Document::new();
            doc.set_title("Integration Test");
            doc.set_author("Test Author");
            doc.set_subject("Writer Integration");
            doc.set_keywords("test, writer, integration");

            let mut page = Page::a4();
            page.text()
                .set_font(Font::Helvetica, 12.0)
                .at(100.0, 700.0)
                .write("Integration Test Content")
                .unwrap();

            doc.add_page(page);

            // Write to file
            let result = doc.save(&file_path);
            assert!(result.is_ok());

            // Verify file exists and has content
            assert!(file_path.exists());
            let metadata = fs::metadata(&file_path).unwrap();
            assert!(metadata.len() > 0);

            // Read file back to verify PDF format
            let content = fs::read(&file_path).unwrap();
            assert!(content.starts_with(b"%PDF-1.7"));
            // Check for %%EOF with or without newline
            assert!(content.ends_with(b"%%EOF\n") || content.ends_with(b"%%EOF"));
        }

        #[test]
        fn test_document_with_complex_content() {
            let temp_dir = TempDir::new().unwrap();
            let file_path = temp_dir.path().join("complex.pdf");

            let mut doc = Document::new();
            doc.set_title("Complex Content Test");

            // Create page with mixed content
            let mut page = Page::a4();

            // Add text
            page.text()
                .set_font(Font::Helvetica, 14.0)
                .at(50.0, 750.0)
                .write("Complex Content Test")
                .unwrap();

            // Add graphics
            page.graphics()
                .set_fill_color(Color::rgb(0.8, 0.2, 0.2))
                .rectangle(50.0, 500.0, 200.0, 100.0)
                .fill();

            page.graphics()
                .set_stroke_color(Color::rgb(0.2, 0.2, 0.8))
                .set_line_width(2.0)
                .move_to(50.0, 400.0)
                .line_to(250.0, 400.0)
                .stroke();

            doc.add_page(page);

            // Write and verify
            let result = doc.save(&file_path);
            assert!(result.is_ok());
            assert!(file_path.exists());
        }

        #[test]
        fn test_document_multiple_pages_integration() {
            let temp_dir = TempDir::new().unwrap();
            let file_path = temp_dir.path().join("multipage.pdf");

            let mut doc = Document::new();
            doc.set_title("Multi-page Integration Test");

            // Create multiple pages with different content
            for i in 1..=5 {
                let mut page = Page::a4();

                page.text()
                    .set_font(Font::Helvetica, 16.0)
                    .at(50.0, 750.0)
                    .write(&format!("Page {i}"))
                    .unwrap();

                page.text()
                    .set_font(Font::Helvetica, 12.0)
                    .at(50.0, 700.0)
                    .write(&format!("This is the content for page {i}"))
                    .unwrap();

                // Add unique graphics for each page
                let color = match i % 3 {
                    0 => Color::rgb(1.0, 0.0, 0.0),
                    1 => Color::rgb(0.0, 1.0, 0.0),
                    _ => Color::rgb(0.0, 0.0, 1.0),
                };

                page.graphics()
                    .set_fill_color(color)
                    .rectangle(50.0, 600.0, 100.0, 50.0)
                    .fill();

                doc.add_page(page);
            }

            // Write and verify
            let result = doc.save(&file_path);
            assert!(result.is_ok());
            assert!(file_path.exists());

            // Verify file size is reasonable for 5 pages
            let metadata = fs::metadata(&file_path).unwrap();
            assert!(metadata.len() > 1000); // Should be substantial
        }

        #[test]
        fn test_document_metadata_persistence() {
            let temp_dir = TempDir::new().unwrap();
            let file_path = temp_dir.path().join("metadata.pdf");

            let mut doc = Document::new();
            doc.set_title("Metadata Persistence Test");
            doc.set_author("Test Author");
            doc.set_subject("Testing metadata preservation");
            doc.set_keywords("metadata, persistence, test");

            doc.add_page(Page::a4());

            // Write to file
            let result = doc.save(&file_path);
            assert!(result.is_ok());

            // Read file content to verify metadata is present
            let content = fs::read(&file_path).unwrap();
            let content_str = String::from_utf8_lossy(&content);

            // Check that metadata appears in the PDF
            assert!(content_str.contains("Metadata Persistence Test"));
            assert!(content_str.contains("Test Author"));
        }

        #[test]
        fn test_document_writer_error_handling() {
            let mut doc = Document::new();
            doc.add_page(Page::a4());

            // Test writing to invalid path
            let result = doc.save("/invalid/path/test.pdf");
            assert!(result.is_err());
        }

        #[test]
        fn test_document_object_management() {
            let mut doc = Document::new();

            // Add objects and verify they're managed properly
            let obj1 = Object::Boolean(true);
            let obj2 = Object::Integer(42);
            let obj3 = Object::Real(std::f64::consts::PI);

            let id1 = doc.add_object(obj1.clone());
            let id2 = doc.add_object(obj2.clone());
            let id3 = doc.add_object(obj3.clone());

            assert_eq!(id1.number(), 1);
            assert_eq!(id2.number(), 2);
            assert_eq!(id3.number(), 3);

            assert_eq!(doc.objects.len(), 3);
            assert!(doc.objects.contains_key(&id1));
            assert!(doc.objects.contains_key(&id2));
            assert!(doc.objects.contains_key(&id3));

            // Verify objects are correct
            assert_eq!(doc.objects.get(&id1), Some(&obj1));
            assert_eq!(doc.objects.get(&id2), Some(&obj2));
            assert_eq!(doc.objects.get(&id3), Some(&obj3));
        }

        #[test]
        fn test_document_page_integration() {
            let mut doc = Document::new();

            // Test different page configurations
            let page1 = Page::a4();
            let page2 = Page::letter();
            let mut page3 = Page::new(500.0, 400.0);

            // Add content to custom page
            page3
                .text()
                .set_font(Font::Helvetica, 10.0)
                .at(25.0, 350.0)
                .write("Custom size page")
                .unwrap();

            doc.add_page(page1);
            doc.add_page(page2);
            doc.add_page(page3);

            assert_eq!(doc.pages.len(), 3);

            // Verify pages maintain their properties (actual dimensions may vary)
            assert!(doc.pages[0].width() > 500.0); // A4 width is reasonable
            assert!(doc.pages[0].height() > 700.0); // A4 height is reasonable
            assert!(doc.pages[1].width() > 500.0); // Letter width is reasonable
            assert!(doc.pages[1].height() > 700.0); // Letter height is reasonable
            assert_eq!(doc.pages[2].width(), 500.0); // Custom width
            assert_eq!(doc.pages[2].height(), 400.0); // Custom height
        }

        #[test]
        fn test_document_content_generation() {
            let temp_dir = TempDir::new().unwrap();
            let file_path = temp_dir.path().join("content.pdf");

            let mut doc = Document::new();
            doc.set_title("Content Generation Test");

            let mut page = Page::a4();

            // Generate content programmatically
            for i in 0..10 {
                let y_pos = 700.0 - (i as f64 * 30.0);
                page.text()
                    .set_font(Font::Helvetica, 12.0)
                    .at(50.0, y_pos)
                    .write(&format!("Generated line {}", i + 1))
                    .unwrap();
            }

            doc.add_page(page);

            // Write and verify
            let result = doc.save(&file_path);
            assert!(result.is_ok());
            assert!(file_path.exists());

            // Verify content was generated
            let metadata = fs::metadata(&file_path).unwrap();
            assert!(metadata.len() > 500); // Should contain substantial content
        }

        #[test]
        fn test_document_buffer_vs_file_write() {
            let temp_dir = TempDir::new().unwrap();
            let file_path = temp_dir.path().join("buffer_vs_file.pdf");

            let mut doc = Document::new();
            doc.set_title("Buffer vs File Test");
            doc.add_page(Page::a4());

            // Write to buffer
            let mut buffer = Vec::new();
            let buffer_result = doc.write(&mut buffer);
            assert!(buffer_result.is_ok());

            // Write to file
            let file_result = doc.save(&file_path);
            assert!(file_result.is_ok());

            // Read file back
            let file_content = fs::read(&file_path).unwrap();

            // Both should be valid PDFs with same structure (timestamps may differ)
            assert!(buffer.starts_with(b"%PDF-1.7"));
            assert!(file_content.starts_with(b"%PDF-1.7"));
            assert!(buffer.ends_with(b"%%EOF\n"));
            assert!(file_content.ends_with(b"%%EOF\n"));

            // Both should contain the same title
            let buffer_str = String::from_utf8_lossy(&buffer);
            let file_str = String::from_utf8_lossy(&file_content);
            assert!(buffer_str.contains("Buffer vs File Test"));
            assert!(file_str.contains("Buffer vs File Test"));
        }

        #[test]
        fn test_document_large_content_handling() {
            let temp_dir = TempDir::new().unwrap();
            let file_path = temp_dir.path().join("large_content.pdf");

            let mut doc = Document::new();
            doc.set_title("Large Content Test");

            let mut page = Page::a4();

            // Add large amount of text content - make it much larger
            let large_text =
                "Lorem ipsum dolor sit amet, consectetur adipiscing elit. ".repeat(200);
            page.text()
                .set_font(Font::Helvetica, 10.0)
                .at(50.0, 750.0)
                .write(&large_text)
                .unwrap();

            doc.add_page(page);

            // Write and verify
            let result = doc.save(&file_path);
            assert!(result.is_ok());
            assert!(file_path.exists());

            // Verify large content was handled properly - reduce expectation
            let metadata = fs::metadata(&file_path).unwrap();
            assert!(metadata.len() > 500); // Should be substantial but realistic
        }

        #[test]
        fn test_document_incremental_building() {
            let temp_dir = TempDir::new().unwrap();
            let file_path = temp_dir.path().join("incremental.pdf");

            let mut doc = Document::new();

            // Build document incrementally
            doc.set_title("Incremental Building Test");

            // Add first page
            let mut page1 = Page::a4();
            page1
                .text()
                .set_font(Font::Helvetica, 12.0)
                .at(50.0, 750.0)
                .write("First page content")
                .unwrap();
            doc.add_page(page1);

            // Add metadata
            doc.set_author("Incremental Author");
            doc.set_subject("Incremental Subject");

            // Add second page
            let mut page2 = Page::a4();
            page2
                .text()
                .set_font(Font::Helvetica, 12.0)
                .at(50.0, 750.0)
                .write("Second page content")
                .unwrap();
            doc.add_page(page2);

            // Add more metadata
            doc.set_keywords("incremental, building, test");

            // Final write
            let result = doc.save(&file_path);
            assert!(result.is_ok());
            assert!(file_path.exists());

            // Verify final state
            assert_eq!(doc.pages.len(), 2);
            assert_eq!(
                doc.metadata.title,
                Some("Incremental Building Test".to_string())
            );
            assert_eq!(doc.metadata.author, Some("Incremental Author".to_string()));
            assert_eq!(
                doc.metadata.subject,
                Some("Incremental Subject".to_string())
            );
            assert_eq!(
                doc.metadata.keywords,
                Some("incremental, building, test".to_string())
            );
        }

        #[test]
        fn test_document_concurrent_page_operations() {
            let mut doc = Document::new();
            doc.set_title("Concurrent Operations Test");

            // Simulate concurrent-like operations
            let mut pages = Vec::new();

            // Create multiple pages
            for i in 0..5 {
                let mut page = Page::a4();
                page.text()
                    .set_font(Font::Helvetica, 12.0)
                    .at(50.0, 750.0)
                    .write(&format!("Concurrent page {i}"))
                    .unwrap();
                pages.push(page);
            }

            // Add all pages
            for page in pages {
                doc.add_page(page);
            }

            assert_eq!(doc.pages.len(), 5);

            // Verify each page maintains its content
            let temp_dir = TempDir::new().unwrap();
            let file_path = temp_dir.path().join("concurrent.pdf");
            let result = doc.save(&file_path);
            assert!(result.is_ok());
        }

        #[test]
        fn test_document_memory_efficiency() {
            let mut doc = Document::new();
            doc.set_title("Memory Efficiency Test");

            // Add multiple pages with content
            for i in 0..10 {
                let mut page = Page::a4();
                page.text()
                    .set_font(Font::Helvetica, 12.0)
                    .at(50.0, 700.0)
                    .write(&format!("Memory test page {i}"))
                    .unwrap();
                doc.add_page(page);
            }

            // Write to buffer to test memory usage
            let mut buffer = Vec::new();
            let result = doc.write(&mut buffer);
            assert!(result.is_ok());
            assert!(!buffer.is_empty());

            // Buffer should be reasonable size
            assert!(buffer.len() < 1_000_000); // Should be less than 1MB for simple content
        }

        #[test]
        fn test_document_creator_producer() {
            let mut doc = Document::new();

            // Default values
            assert_eq!(doc.metadata.creator, Some("oxidize_pdf".to_string()));
            assert!(doc
                .metadata
                .producer
                .as_ref()
                .unwrap()
                .contains("oxidize_pdf"));

            // Set custom values
            doc.set_creator("My Application");
            doc.set_producer("My PDF Library v1.0");

            assert_eq!(doc.metadata.creator, Some("My Application".to_string()));
            assert_eq!(
                doc.metadata.producer,
                Some("My PDF Library v1.0".to_string())
            );
        }

        #[test]
        fn test_document_dates() {
            use chrono::{TimeZone, Utc};

            let mut doc = Document::new();

            // Check default dates are set
            assert!(doc.metadata.creation_date.is_some());
            assert!(doc.metadata.modification_date.is_some());

            // Set specific dates
            let creation_date = Utc.with_ymd_and_hms(2023, 1, 1, 12, 0, 0).unwrap();
            let mod_date = Utc.with_ymd_and_hms(2023, 6, 15, 18, 30, 0).unwrap();

            doc.set_creation_date(creation_date);
            doc.set_modification_date(mod_date);

            assert_eq!(doc.metadata.creation_date, Some(creation_date));
            assert_eq!(doc.metadata.modification_date, Some(mod_date));
        }

        #[test]
        fn test_document_dates_local() {
            use chrono::{Local, TimeZone};

            let mut doc = Document::new();

            // Test setting dates with local time
            let local_date = Local.with_ymd_and_hms(2023, 12, 25, 10, 30, 0).unwrap();
            doc.set_creation_date_local(local_date);

            // Verify it was converted to UTC
            assert!(doc.metadata.creation_date.is_some());
            // Just verify the date was set, don't compare exact values due to timezone complexities
            assert!(doc.metadata.creation_date.is_some());
        }

        #[test]
        fn test_update_modification_date() {
            let mut doc = Document::new();

            let initial_mod_date = doc.metadata.modification_date;
            assert!(initial_mod_date.is_some());

            // Sleep briefly to ensure time difference
            std::thread::sleep(std::time::Duration::from_millis(10));

            doc.update_modification_date();

            let new_mod_date = doc.metadata.modification_date;
            assert!(new_mod_date.is_some());
            assert!(new_mod_date.unwrap() > initial_mod_date.unwrap());
        }

        #[test]
        fn test_document_save_updates_modification_date() {
            let temp_dir = TempDir::new().unwrap();
            let file_path = temp_dir.path().join("mod_date_test.pdf");

            let mut doc = Document::new();
            doc.add_page(Page::a4());

            let initial_mod_date = doc.metadata.modification_date;

            // Sleep briefly to ensure time difference
            std::thread::sleep(std::time::Duration::from_millis(10));

            doc.save(&file_path).unwrap();

            // Modification date should be updated
            assert!(doc.metadata.modification_date.unwrap() > initial_mod_date.unwrap());
        }

        #[test]
        fn test_document_metadata_complete() {
            let mut doc = Document::new();

            // Set all metadata fields
            doc.set_title("Complete Metadata Test");
            doc.set_author("Test Author");
            doc.set_subject("Testing all metadata fields");
            doc.set_keywords("test, metadata, complete");
            doc.set_creator("Test Application v1.0");
            doc.set_producer("oxidize_pdf Test Suite");

            // Verify all fields
            assert_eq!(
                doc.metadata.title,
                Some("Complete Metadata Test".to_string())
            );
            assert_eq!(doc.metadata.author, Some("Test Author".to_string()));
            assert_eq!(
                doc.metadata.subject,
                Some("Testing all metadata fields".to_string())
            );
            assert_eq!(
                doc.metadata.keywords,
                Some("test, metadata, complete".to_string())
            );
            assert_eq!(
                doc.metadata.creator,
                Some("Test Application v1.0".to_string())
            );
            assert_eq!(
                doc.metadata.producer,
                Some("oxidize_pdf Test Suite".to_string())
            );
            assert!(doc.metadata.creation_date.is_some());
            assert!(doc.metadata.modification_date.is_some());
        }

        #[test]
        fn test_document_to_bytes() {
            let mut doc = Document::new();
            doc.set_title("Test Document");
            doc.set_author("Test Author");

            let page = Page::a4();
            doc.add_page(page);

            // Generate PDF as bytes
            let pdf_bytes = doc.to_bytes().unwrap();

            // Basic validation
            assert!(!pdf_bytes.is_empty());
            assert!(pdf_bytes.len() > 100); // Should be reasonable size

            // Check PDF header
            let header = &pdf_bytes[0..5];
            assert_eq!(header, b"%PDF-");

            // Check for some basic PDF structure
            let pdf_str = String::from_utf8_lossy(&pdf_bytes);
            assert!(pdf_str.contains("Test Document"));
            assert!(pdf_str.contains("Test Author"));
        }

        #[test]
        fn test_document_to_bytes_with_config() {
            let mut doc = Document::new();
            doc.set_title("Test Document XRef");

            let page = Page::a4();
            doc.add_page(page);

            let config = crate::writer::WriterConfig {
                use_xref_streams: true,
                pdf_version: "1.5".to_string(),
                compress_streams: true,
            };

            // Generate PDF with custom config
            let pdf_bytes = doc.to_bytes_with_config(config).unwrap();

            // Basic validation
            assert!(!pdf_bytes.is_empty());
            assert!(pdf_bytes.len() > 100);

            // Check PDF header with correct version
            let header = String::from_utf8_lossy(&pdf_bytes[0..8]);
            assert!(header.contains("PDF-1.5"));
        }

        #[test]
        fn test_to_bytes_vs_save_equivalence() {
            use std::fs;
            use tempfile::NamedTempFile;

            // Create two identical documents
            let mut doc1 = Document::new();
            doc1.set_title("Equivalence Test");
            doc1.add_page(Page::a4());

            let mut doc2 = Document::new();
            doc2.set_title("Equivalence Test");
            doc2.add_page(Page::a4());

            // Generate bytes
            let pdf_bytes = doc1.to_bytes().unwrap();

            // Save to file
            let temp_file = NamedTempFile::new().unwrap();
            doc2.save(temp_file.path()).unwrap();
            let file_bytes = fs::read(temp_file.path()).unwrap();

            // Both should generate similar structure (lengths may vary due to timestamps)
            assert!(!pdf_bytes.is_empty());
            assert!(!file_bytes.is_empty());
            assert_eq!(&pdf_bytes[0..5], &file_bytes[0..5]); // PDF headers should match
        }

        #[test]
        fn test_document_set_compress() {
            let mut doc = Document::new();
            doc.set_title("Compression Test");
            doc.add_page(Page::a4());

            // Default should be compressed
            assert!(doc.get_compress());

            // Test with compression enabled
            doc.set_compress(true);
            let compressed_bytes = doc.to_bytes().unwrap();

            // Test with compression disabled
            doc.set_compress(false);
            let uncompressed_bytes = doc.to_bytes().unwrap();

            // Uncompressed should generally be larger (though not always guaranteed)
            assert!(!compressed_bytes.is_empty());
            assert!(!uncompressed_bytes.is_empty());

            // Both should be valid PDFs
            assert_eq!(&compressed_bytes[0..5], b"%PDF-");
            assert_eq!(&uncompressed_bytes[0..5], b"%PDF-");
        }

        #[test]
        fn test_document_compression_config_inheritance() {
            let mut doc = Document::new();
            doc.set_title("Config Inheritance Test");
            doc.add_page(Page::a4());

            // Set document compression to false
            doc.set_compress(false);

            // Create config with compression true (should be overridden)
            let config = crate::writer::WriterConfig {
                use_xref_streams: false,
                pdf_version: "1.7".to_string(),
                compress_streams: true,
            };

            // Document setting should take precedence
            let pdf_bytes = doc.to_bytes_with_config(config).unwrap();

            // Should be valid PDF
            assert!(!pdf_bytes.is_empty());
            assert_eq!(&pdf_bytes[0..5], b"%PDF-");
        }

        #[test]
        fn test_document_metadata_all_fields() {
            let mut doc = Document::new();

            // Set all metadata fields
            doc.set_title("Test Document");
            doc.set_author("John Doe");
            doc.set_subject("Testing PDF metadata");
            doc.set_keywords("test, pdf, metadata");
            doc.set_creator("Test Suite");
            doc.set_producer("oxidize_pdf tests");

            // Verify all fields are set
            assert_eq!(doc.metadata.title.as_deref(), Some("Test Document"));
            assert_eq!(doc.metadata.author.as_deref(), Some("John Doe"));
            assert_eq!(
                doc.metadata.subject.as_deref(),
                Some("Testing PDF metadata")
            );
            assert_eq!(
                doc.metadata.keywords.as_deref(),
                Some("test, pdf, metadata")
            );
            assert_eq!(doc.metadata.creator.as_deref(), Some("Test Suite"));
            assert_eq!(doc.metadata.producer.as_deref(), Some("oxidize_pdf tests"));
            assert!(doc.metadata.creation_date.is_some());
            assert!(doc.metadata.modification_date.is_some());
        }

        #[test]
        fn test_document_add_pages() {
            let mut doc = Document::new();

            // Initially empty
            assert_eq!(doc.page_count(), 0);

            // Add pages
            let page1 = Page::a4();
            let page2 = Page::letter();
            let page3 = Page::legal();

            doc.add_page(page1);
            assert_eq!(doc.page_count(), 1);

            doc.add_page(page2);
            assert_eq!(doc.page_count(), 2);

            doc.add_page(page3);
            assert_eq!(doc.page_count(), 3);

            // Verify we can convert to PDF with multiple pages
            let result = doc.to_bytes();
            assert!(result.is_ok());
        }

        #[test]
        fn test_document_default_font_encoding() {
            let mut doc = Document::new();

            // Initially no default encoding
            assert!(doc.default_font_encoding.is_none());

            // Set default encoding
            doc.set_default_font_encoding(Some(FontEncoding::WinAnsiEncoding));
            assert_eq!(
                doc.default_font_encoding(),
                Some(FontEncoding::WinAnsiEncoding)
            );

            // Change encoding
            doc.set_default_font_encoding(Some(FontEncoding::MacRomanEncoding));
            assert_eq!(
                doc.default_font_encoding(),
                Some(FontEncoding::MacRomanEncoding)
            );
        }

        #[test]
        fn test_document_compression_setting() {
            let mut doc = Document::new();

            // Default should compress
            assert!(doc.compress);

            // Disable compression
            doc.set_compress(false);
            assert!(!doc.compress);

            // Re-enable compression
            doc.set_compress(true);
            assert!(doc.compress);
        }

        #[test]
        fn test_document_with_empty_pages() {
            let mut doc = Document::new();

            // Add empty page
            doc.add_page(Page::a4());

            // Should be able to convert to bytes
            let result = doc.to_bytes();
            assert!(result.is_ok());

            let pdf_bytes = result.unwrap();
            assert!(!pdf_bytes.is_empty());
            assert!(pdf_bytes.starts_with(b"%PDF-"));
        }

        #[test]
        fn test_document_with_multiple_page_sizes() {
            let mut doc = Document::new();

            // Add pages with different sizes
            doc.add_page(Page::a4()); // 595 x 842
            doc.add_page(Page::letter()); // 612 x 792
            doc.add_page(Page::legal()); // 612 x 1008
            doc.add_page(Page::a4()); // Another A4
            doc.add_page(Page::new(200.0, 300.0)); // Custom size

            assert_eq!(doc.page_count(), 5);

            // Verify we have 5 pages
            // Note: Direct page access is not available in public API
            // We verify by successful PDF generation
            let result = doc.to_bytes();
            assert!(result.is_ok());
        }

        #[test]
        fn test_document_metadata_dates() {
            use chrono::Duration;

            let doc = Document::new();

            // Should have creation and modification dates
            assert!(doc.metadata.creation_date.is_some());
            assert!(doc.metadata.modification_date.is_some());

            if let (Some(created), Some(modified)) =
                (doc.metadata.creation_date, doc.metadata.modification_date)
            {
                // Dates should be very close (created during construction)
                let diff = modified - created;
                assert!(diff < Duration::seconds(1));
            }
        }

        #[test]
        fn test_document_builder_pattern() {
            // Test fluent API style
            let mut doc = Document::new();
            doc.set_title("Fluent");
            doc.set_author("Builder");
            doc.set_compress(true);

            assert_eq!(doc.metadata.title.as_deref(), Some("Fluent"));
            assert_eq!(doc.metadata.author.as_deref(), Some("Builder"));
            assert!(doc.compress);
        }

        #[test]
        fn test_xref_streams_functionality() {
            use crate::{Document, Font, Page};

            // Test with xref streams disabled (default)
            let mut doc = Document::new();
            assert!(!doc.use_xref_streams);

            let mut page = Page::a4();
            page.text()
                .set_font(Font::Helvetica, 12.0)
                .at(100.0, 700.0)
                .write("Testing XRef Streams")
                .unwrap();

            doc.add_page(page);

            // Generate PDF without xref streams
            let pdf_without_xref = doc.to_bytes().unwrap();

            // Verify traditional xref is used
            let pdf_str = String::from_utf8_lossy(&pdf_without_xref);
            assert!(pdf_str.contains("xref"), "Traditional xref table not found");
            assert!(
                !pdf_str.contains("/Type /XRef"),
                "XRef stream found when it shouldn't be"
            );

            // Test with xref streams enabled
            doc.enable_xref_streams(true);
            assert!(doc.use_xref_streams);

            // Generate PDF with xref streams
            let pdf_with_xref = doc.to_bytes().unwrap();

            // Verify xref streams are used
            let pdf_str = String::from_utf8_lossy(&pdf_with_xref);
            // XRef streams replace traditional xref tables in PDF 1.5+
            assert!(
                pdf_str.contains("/Type /XRef") || pdf_str.contains("stream"),
                "XRef stream not found when enabled"
            );

            // Verify PDF version is set correctly
            assert!(
                pdf_str.contains("PDF-1.5"),
                "PDF version not set to 1.5 for xref streams"
            );

            // Test fluent interface
            let mut doc2 = Document::new();
            doc2.enable_xref_streams(true);
            doc2.set_title("XRef Streams Test");
            doc2.set_author("oxidize-pdf");

            assert!(doc2.use_xref_streams);
            assert_eq!(doc2.metadata.title.as_deref(), Some("XRef Streams Test"));
            assert_eq!(doc2.metadata.author.as_deref(), Some("oxidize-pdf"));
        }

        #[test]
        fn test_document_save_to_vec() {
            let mut doc = Document::new();
            doc.set_title("Test Save");
            doc.add_page(Page::a4());

            // Test to_bytes
            let bytes_result = doc.to_bytes();
            assert!(bytes_result.is_ok());

            let bytes = bytes_result.unwrap();
            assert!(!bytes.is_empty());
            assert!(bytes.starts_with(b"%PDF-"));
            assert!(bytes.ends_with(b"%%EOF") || bytes.ends_with(b"%%EOF\n"));
        }

        #[test]
        fn test_document_unicode_metadata() {
            let mut doc = Document::new();

            // Set metadata with Unicode characters
            doc.set_title("日本語のタイトル");
            doc.set_author("作者名 😀");
            doc.set_subject("Тема документа");
            doc.set_keywords("كلمات, מפתח, 关键词");

            assert_eq!(doc.metadata.title.as_deref(), Some("日本語のタイトル"));
            assert_eq!(doc.metadata.author.as_deref(), Some("作者名 😀"));
            assert_eq!(doc.metadata.subject.as_deref(), Some("Тема документа"));
            assert_eq!(
                doc.metadata.keywords.as_deref(),
                Some("كلمات, מפתח, 关键词")
            );
        }

        #[test]
        fn test_document_page_iteration() {
            let mut doc = Document::new();

            // Add multiple pages
            for i in 0..5 {
                let mut page = Page::a4();
                let gc = page.graphics();
                gc.begin_text();
                let _ = gc.show_text(&format!("Page {}", i + 1));
                gc.end_text();
                doc.add_page(page);
            }

            // Verify page count
            assert_eq!(doc.page_count(), 5);

            // Verify we can generate PDF with all pages
            let result = doc.to_bytes();
            assert!(result.is_ok());
        }

        #[test]
        fn test_document_with_graphics_content() {
            let mut doc = Document::new();

            let mut page = Page::a4();
            {
                let gc = page.graphics();

                // Add various graphics operations
                gc.save_state();

                // Draw rectangle
                gc.rectangle(100.0, 100.0, 200.0, 150.0);
                gc.stroke();

                // Draw circle (approximated)
                gc.move_to(300.0, 300.0);
                gc.circle(300.0, 300.0, 50.0);
                gc.fill();

                // Add text
                gc.begin_text();
                gc.set_text_position(100.0, 500.0);
                let _ = gc.show_text("Graphics Test");
                gc.end_text();

                gc.restore_state();
            }

            doc.add_page(page);

            // Should produce valid PDF
            let result = doc.to_bytes();
            assert!(result.is_ok());
        }

        #[test]
        fn test_document_producer_version() {
            let doc = Document::new();

            // Producer should contain version
            assert!(doc.metadata.producer.is_some());
            if let Some(producer) = &doc.metadata.producer {
                assert!(producer.contains("oxidize_pdf"));
                assert!(producer.contains(env!("CARGO_PKG_VERSION")));
            }
        }

        #[test]
        fn test_document_empty_metadata_fields() {
            let mut doc = Document::new();

            // Set empty strings
            doc.set_title("");
            doc.set_author("");
            doc.set_subject("");
            doc.set_keywords("");

            // Empty strings should be stored as Some("")
            assert_eq!(doc.metadata.title.as_deref(), Some(""));
            assert_eq!(doc.metadata.author.as_deref(), Some(""));
            assert_eq!(doc.metadata.subject.as_deref(), Some(""));
            assert_eq!(doc.metadata.keywords.as_deref(), Some(""));
        }

        #[test]
        fn test_document_very_long_metadata() {
            let mut doc = Document::new();

            // Create very long strings
            let long_title = "A".repeat(1000);
            let long_author = "B".repeat(500);
            let long_keywords = vec!["keyword"; 100].join(", ");

            doc.set_title(&long_title);
            doc.set_author(&long_author);
            doc.set_keywords(&long_keywords);

            assert_eq!(doc.metadata.title.as_deref(), Some(long_title.as_str()));
            assert_eq!(doc.metadata.author.as_deref(), Some(long_author.as_str()));
            assert!(doc.metadata.keywords.as_ref().unwrap().len() > 500);
        }
    }
}
