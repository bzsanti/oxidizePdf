//! Tagged PDF Support - ISO 32000-1 Section 14.8 (Experimental)
//!
//! Tagged PDF is a framework for document structure and accessibility.
//! This module provides **structure tree generation** with parent references,
//! attributes, and role mapping.
//!
//! # ⚠️ Status (v1.4.0)
//!
//! - ✅ **Structure tree hierarchy** - Fully implemented
//! - ✅ **Parent references** - ISO 32000-1 §14.7.2 compliant
//! - ✅ **Attributes** - Lang, Alt, ActualText, Title, BBox
//! - ✅ **RoleMap** - Custom to standard type mapping
//! - ❌ **Marked content operators** - Not yet implemented (v1.5.0)
//! - ❌ **Automatic MCID assignment** - Not yet implemented (v1.5.0)
//! - ❌ **PDF/UA compliance** - Not yet achieved (v1.5.0)
//!
//! **Note**: While the structure tree is generated correctly, the content is NOT
//! automatically marked with BMC/BDC/EMC operators. This means screen readers
//! will see the structure but cannot navigate the actual content. For full
//! accessibility, wait for v1.5.0 or add marked content manually.
//!
//! # Key Components
//!
//! - **Structure Tree Root** (`/StructTreeRoot` in catalog): Root of structure hierarchy
//! - **Structure Elements** (`/StructElem`): Elements forming the document tree
//! - **Role Map**: Maps custom structure types to standard types
//! - **Marked Content** (v1.5.0): Associates content with structure elements via MCIDs
//!
//! # Standard Structure Types
//!
//! ISO 32000-1 Table 337 defines standard structure types:
//! - Grouping: Document, Part, Sect, Div, Art, BlockQuote
//! - Paragraphs: P, H, H1-H6
//! - Lists: L, LI, Lbl, LBody
//! - Tables: Table, TR, TH, TD
//! - Inline: Span, Quote, Note, Reference, Code
//! - Illustration: Figure, Formula, Form
//!
//! # Example
//!
//! ```rust,no_run
//! use oxidize_pdf::structure::{StructTree, StructureElement, StandardStructureType};
//!
//! # fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let mut tree = StructTree::new();
//!
//! // Create document root
//! let doc_elem = StructureElement::new(StandardStructureType::Document);
//! let doc_idx = tree.set_root(doc_elem);
//!
//! // Add heading with language attribute
//! let heading = StructureElement::new(StandardStructureType::H1)
//!     .with_language("en-US")
//!     .with_actual_text("Welcome to Tagged PDF");
//! let h1_idx = tree.add_child(doc_idx, heading)?;
//!
//! // Add paragraph
//! let para = StructureElement::new(StandardStructureType::P);
//! tree.add_child(doc_idx, para)?;
//!
//! # Ok(())
//! # }
//! ```

use std::collections::HashMap;

/// Standard structure types defined in ISO 32000-1 Table 337
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum StandardStructureType {
    // Grouping Elements
    /// Document root element
    Document,
    /// Part of a document
    Part,
    /// Section (generic division)
    Sect,
    /// Generic block-level division
    Div,
    /// Article
    Art,
    /// Block quotation
    BlockQuote,
    /// Caption (for figures, tables, etc.)
    Caption,
    /// Table of contents
    TOC,
    /// Table of contents item
    TOCI,
    /// Index
    Index,

    // Paragraph-like Elements
    /// Generic paragraph
    P,
    /// Generic heading (when level unknown)
    H,
    /// Heading level 1
    H1,
    /// Heading level 2
    H2,
    /// Heading level 3
    H3,
    /// Heading level 4
    H4,
    /// Heading level 5
    H5,
    /// Heading level 6
    H6,

    // List Elements
    /// List
    L,
    /// List item
    LI,
    /// Label for list item (bullet or number)
    Lbl,
    /// List item body
    LBody,

    // Table Elements
    /// Table
    Table,
    /// Table row
    TR,
    /// Table header cell
    TH,
    /// Table data cell
    TD,
    /// Table header row group
    THead,
    /// Table body row group
    TBody,
    /// Table footer row group
    TFoot,

    // Inline Elements
    /// Generic inline span
    Span,
    /// Quotation
    Quote,
    /// Note or footnote
    Note,
    /// Reference to external content
    Reference,
    /// Bibliographic entry
    BibEntry,
    /// Computer code
    Code,
    /// Hyperlink
    Link,
    /// Annotation reference
    Annot,

    // Illustration Elements
    /// Figure or illustration
    Figure,
    /// Mathematical formula
    Formula,
    /// Interactive form element
    Form,

    // Ruby and Warichu (for Asian languages)
    /// Ruby annotation (Asian text)
    Ruby,
    /// Ruby base text
    RB,
    /// Ruby text
    RT,
    /// Ruby punctuation
    RP,
    /// Warichu annotation
    Warichu,
    /// Warichu text
    WT,
    /// Warichu punctuation
    WP,

    // Special
    /// Non-structural element (decorative content)
    NonStruct,
    /// Private element (application-specific)
    Private,
}

impl StandardStructureType {
    /// Returns the PDF name for this structure type
    pub fn as_pdf_name(&self) -> &'static str {
        match self {
            Self::Document => "Document",
            Self::Part => "Part",
            Self::Sect => "Sect",
            Self::Div => "Div",
            Self::Art => "Art",
            Self::BlockQuote => "BlockQuote",
            Self::Caption => "Caption",
            Self::TOC => "TOC",
            Self::TOCI => "TOCI",
            Self::Index => "Index",
            Self::P => "P",
            Self::H => "H",
            Self::H1 => "H1",
            Self::H2 => "H2",
            Self::H3 => "H3",
            Self::H4 => "H4",
            Self::H5 => "H5",
            Self::H6 => "H6",
            Self::L => "L",
            Self::LI => "LI",
            Self::Lbl => "Lbl",
            Self::LBody => "LBody",
            Self::Table => "Table",
            Self::TR => "TR",
            Self::TH => "TH",
            Self::TD => "TD",
            Self::THead => "THead",
            Self::TBody => "TBody",
            Self::TFoot => "TFoot",
            Self::Span => "Span",
            Self::Quote => "Quote",
            Self::Note => "Note",
            Self::Reference => "Reference",
            Self::BibEntry => "BibEntry",
            Self::Code => "Code",
            Self::Link => "Link",
            Self::Annot => "Annot",
            Self::Figure => "Figure",
            Self::Formula => "Formula",
            Self::Form => "Form",
            Self::Ruby => "Ruby",
            Self::RB => "RB",
            Self::RT => "RT",
            Self::RP => "RP",
            Self::Warichu => "Warichu",
            Self::WT => "WT",
            Self::WP => "WP",
            Self::NonStruct => "NonStruct",
            Self::Private => "Private",
        }
    }

    /// Parses a PDF name into a standard structure type
    pub fn from_pdf_name(name: &str) -> Option<Self> {
        match name {
            "Document" => Some(Self::Document),
            "Part" => Some(Self::Part),
            "Sect" => Some(Self::Sect),
            "Div" => Some(Self::Div),
            "Art" => Some(Self::Art),
            "BlockQuote" => Some(Self::BlockQuote),
            "Caption" => Some(Self::Caption),
            "TOC" => Some(Self::TOC),
            "TOCI" => Some(Self::TOCI),
            "Index" => Some(Self::Index),
            "P" => Some(Self::P),
            "H" => Some(Self::H),
            "H1" => Some(Self::H1),
            "H2" => Some(Self::H2),
            "H3" => Some(Self::H3),
            "H4" => Some(Self::H4),
            "H5" => Some(Self::H5),
            "H6" => Some(Self::H6),
            "L" => Some(Self::L),
            "LI" => Some(Self::LI),
            "Lbl" => Some(Self::Lbl),
            "LBody" => Some(Self::LBody),
            "Table" => Some(Self::Table),
            "TR" => Some(Self::TR),
            "TH" => Some(Self::TH),
            "TD" => Some(Self::TD),
            "THead" => Some(Self::THead),
            "TBody" => Some(Self::TBody),
            "TFoot" => Some(Self::TFoot),
            "Span" => Some(Self::Span),
            "Quote" => Some(Self::Quote),
            "Note" => Some(Self::Note),
            "Reference" => Some(Self::Reference),
            "BibEntry" => Some(Self::BibEntry),
            "Code" => Some(Self::Code),
            "Link" => Some(Self::Link),
            "Annot" => Some(Self::Annot),
            "Figure" => Some(Self::Figure),
            "Formula" => Some(Self::Formula),
            "Form" => Some(Self::Form),
            "Ruby" => Some(Self::Ruby),
            "RB" => Some(Self::RB),
            "RT" => Some(Self::RT),
            "RP" => Some(Self::RP),
            "Warichu" => Some(Self::Warichu),
            "WT" => Some(Self::WT),
            "WP" => Some(Self::WP),
            "NonStruct" => Some(Self::NonStruct),
            "Private" => Some(Self::Private),
            _ => None,
        }
    }
}

/// Attributes that can be attached to structure elements
///
/// These attributes provide additional semantic information for accessibility
/// and document understanding.
#[derive(Debug, Clone, Default)]
pub struct StructureAttributes {
    /// Language of the element content (e.g., "en-US", "es-ES", "zh-CN")
    pub lang: Option<String>,

    /// Alternate description (for accessibility - used when content cannot be extracted)
    pub alt: Option<String>,

    /// Actual text representation (replacement text for abbreviations, symbols, etc.)
    pub actual_text: Option<String>,

    /// Expansion of an abbreviation
    pub expanded: Option<String>,

    /// Title or label for the element
    pub title: Option<String>,

    /// Bounding box (Left, Bottom, Right, Top)
    pub bbox: Option<[f64; 4]>,

    /// Custom attributes (for application-specific metadata)
    pub custom: HashMap<String, String>,
}

impl StructureAttributes {
    /// Creates a new empty attributes set
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the language attribute
    pub fn with_language(mut self, lang: impl Into<String>) -> Self {
        self.lang = Some(lang.into());
        self
    }

    /// Sets the alt text attribute (for accessibility)
    pub fn with_alt_text(mut self, alt: impl Into<String>) -> Self {
        self.alt = Some(alt.into());
        self
    }

    /// Sets the actual text attribute
    pub fn with_actual_text(mut self, text: impl Into<String>) -> Self {
        self.actual_text = Some(text.into());
        self
    }

    /// Sets the title attribute
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Sets the bounding box attribute
    pub fn with_bbox(mut self, bbox: [f64; 4]) -> Self {
        self.bbox = Some(bbox);
        self
    }
}

/// A structure element in the document structure tree
///
/// Structure elements form a hierarchical tree that describes the logical
/// organization of the document content.
#[derive(Debug, Clone)]
pub struct StructureElement {
    /// The structure type (either standard or custom)
    pub structure_type: StructureType,

    /// Element ID (optional, used for referencing)
    pub id: Option<String>,

    /// Attributes for this element
    pub attributes: StructureAttributes,

    /// Child elements (element IDs)
    pub children: Vec<usize>,

    /// Marked content references (MCIDs) associated with this element
    pub mcids: Vec<MarkedContentReference>,
}

/// Structure type - either standard or custom (mapped via RoleMap)
#[derive(Debug, Clone, PartialEq)]
pub enum StructureType {
    /// Standard structure type defined by PDF spec
    Standard(StandardStructureType),
    /// Custom structure type (must be mapped in RoleMap)
    Custom(String),
}

impl StructureType {
    /// Returns the PDF name for this structure type
    pub fn as_pdf_name(&self) -> String {
        match self {
            Self::Standard(std_type) => std_type.as_pdf_name().to_string(),
            Self::Custom(name) => name.clone(),
        }
    }
}

/// Reference to marked content in a content stream
///
/// Marked content is delimited by BMC/BDC (begin) and EMC (end) operators,
/// and associated with structure elements via MCIDs.
#[derive(Debug, Clone, PartialEq)]
pub struct MarkedContentReference {
    /// Page index where the marked content appears
    pub page_index: usize,

    /// Marked Content ID within the page's content stream
    pub mcid: u32,
}

impl StructureElement {
    /// Creates a new structure element with the given type
    pub fn new(structure_type: StandardStructureType) -> Self {
        Self {
            structure_type: StructureType::Standard(structure_type),
            id: None,
            attributes: StructureAttributes::new(),
            children: Vec::new(),
            mcids: Vec::new(),
        }
    }

    /// Creates a new structure element with a custom type
    pub fn new_custom(type_name: impl Into<String>) -> Self {
        Self {
            structure_type: StructureType::Custom(type_name.into()),
            id: None,
            attributes: StructureAttributes::new(),
            children: Vec::new(),
            mcids: Vec::new(),
        }
    }

    /// Sets the element ID
    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Sets the language attribute
    pub fn with_language(mut self, lang: impl Into<String>) -> Self {
        self.attributes.lang = Some(lang.into());
        self
    }

    /// Sets the alt text attribute
    pub fn with_alt_text(mut self, alt: impl Into<String>) -> Self {
        self.attributes.alt = Some(alt.into());
        self
    }

    /// Sets the actual text attribute
    pub fn with_actual_text(mut self, text: impl Into<String>) -> Self {
        self.attributes.actual_text = Some(text.into());
        self
    }

    /// Sets the title attribute
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.attributes.title = Some(title.into());
        self
    }

    /// Adds a marked content reference to this element
    pub fn add_mcid(&mut self, page_index: usize, mcid: u32) {
        self.mcids.push(MarkedContentReference { page_index, mcid });
    }

    /// Adds a child element (by index in the structure tree)
    pub fn add_child(&mut self, child_index: usize) {
        self.children.push(child_index);
    }
}

/// Role map - maps custom structure types to standard types
///
/// Allows extending the structure type system while maintaining
/// compatibility with standard types.
#[derive(Debug, Clone, Default)]
pub struct RoleMap {
    mappings: HashMap<String, StandardStructureType>,
}

impl RoleMap {
    /// Creates a new empty role map
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a mapping from a custom type to a standard type
    pub fn add_mapping(
        &mut self,
        custom_type: impl Into<String>,
        standard_type: StandardStructureType,
    ) {
        self.mappings.insert(custom_type.into(), standard_type);
    }

    /// Gets the standard type for a custom type (if mapped)
    pub fn get_mapping(&self, custom_type: &str) -> Option<&StandardStructureType> {
        self.mappings.get(custom_type)
    }

    /// Returns all mappings
    pub fn mappings(&self) -> &HashMap<String, StandardStructureType> {
        &self.mappings
    }
}

/// Structure tree - hierarchical organization of document structure
///
/// The structure tree describes the logical organization of content in
/// a tagged PDF document.
#[derive(Debug, Clone)]
pub struct StructTree {
    /// All structure elements in the tree (indexed by position)
    elements: Vec<StructureElement>,

    /// Index of the root element (typically Document)
    root_index: Option<usize>,

    /// Role map for custom structure types
    pub role_map: RoleMap,

    /// ID tree for quick lookup by element ID
    id_map: HashMap<String, usize>,
}

impl Default for StructTree {
    fn default() -> Self {
        Self::new()
    }
}

impl StructTree {
    /// Creates a new empty structure tree
    pub fn new() -> Self {
        Self {
            elements: Vec::new(),
            root_index: None,
            role_map: RoleMap::new(),
            id_map: HashMap::new(),
        }
    }

    /// Adds a root element to the tree (typically Document)
    pub fn set_root(&mut self, element: StructureElement) -> usize {
        let index = self.elements.len();

        // Update ID map if element has an ID
        if let Some(ref id) = element.id {
            self.id_map.insert(id.clone(), index);
        }

        self.elements.push(element);
        self.root_index = Some(index);
        index
    }

    /// Adds an element as a child of another element
    pub fn add_child(
        &mut self,
        parent_index: usize,
        element: StructureElement,
    ) -> Result<usize, String> {
        if parent_index >= self.elements.len() {
            return Err(format!("Parent index {} out of bounds", parent_index));
        }

        let child_index = self.elements.len();

        // Update ID map if element has an ID
        if let Some(ref id) = element.id {
            self.id_map.insert(id.clone(), child_index);
        }

        self.elements.push(element);
        self.elements[parent_index].add_child(child_index);

        Ok(child_index)
    }

    /// Gets an element by index
    pub fn get(&self, index: usize) -> Option<&StructureElement> {
        self.elements.get(index)
    }

    /// Gets a mutable reference to an element by index
    pub fn get_mut(&mut self, index: usize) -> Option<&mut StructureElement> {
        self.elements.get_mut(index)
    }

    /// Gets an element by ID
    pub fn get_by_id(&self, id: &str) -> Option<&StructureElement> {
        self.id_map.get(id).and_then(|&index| self.get(index))
    }

    /// Gets the root element index
    pub fn root_index(&self) -> Option<usize> {
        self.root_index
    }

    /// Gets the root element
    pub fn root(&self) -> Option<&StructureElement> {
        self.root_index.and_then(|index| self.get(index))
    }

    /// Returns the total number of elements in the tree
    pub fn len(&self) -> usize {
        self.elements.len()
    }

    /// Returns true if the tree is empty
    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }

    /// Returns an iterator over all elements
    pub fn iter(&self) -> impl Iterator<Item = &StructureElement> {
        self.elements.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_standard_structure_type_names() {
        assert_eq!(StandardStructureType::Document.as_pdf_name(), "Document");
        assert_eq!(StandardStructureType::H1.as_pdf_name(), "H1");
        assert_eq!(StandardStructureType::P.as_pdf_name(), "P");
        assert_eq!(StandardStructureType::Figure.as_pdf_name(), "Figure");
        assert_eq!(StandardStructureType::Table.as_pdf_name(), "Table");
    }

    #[test]
    fn test_standard_structure_type_parsing() {
        assert_eq!(
            StandardStructureType::from_pdf_name("Document"),
            Some(StandardStructureType::Document)
        );
        assert_eq!(
            StandardStructureType::from_pdf_name("H1"),
            Some(StandardStructureType::H1)
        );
        assert_eq!(StandardStructureType::from_pdf_name("Invalid"), None);
    }

    #[test]
    fn test_structure_element_creation() {
        let elem = StructureElement::new(StandardStructureType::H1)
            .with_id("heading1")
            .with_language("en-US")
            .with_actual_text("Chapter One");

        assert_eq!(elem.id, Some("heading1".to_string()));
        assert_eq!(elem.attributes.lang, Some("en-US".to_string()));
        assert_eq!(elem.attributes.actual_text, Some("Chapter One".to_string()));
    }

    #[test]
    fn test_structure_attributes_builder() {
        let attrs = StructureAttributes::new()
            .with_language("es-ES")
            .with_alt_text("Imagen de ejemplo")
            .with_bbox([0.0, 0.0, 100.0, 100.0]);

        assert_eq!(attrs.lang, Some("es-ES".to_string()));
        assert_eq!(attrs.alt, Some("Imagen de ejemplo".to_string()));
        assert_eq!(attrs.bbox, Some([0.0, 0.0, 100.0, 100.0]));
    }

    #[test]
    fn test_role_map() {
        let mut role_map = RoleMap::new();
        role_map.add_mapping("MyHeading", StandardStructureType::H1);
        role_map.add_mapping("MyParagraph", StandardStructureType::P);

        assert_eq!(
            role_map.get_mapping("MyHeading"),
            Some(&StandardStructureType::H1)
        );
        assert_eq!(
            role_map.get_mapping("MyParagraph"),
            Some(&StandardStructureType::P)
        );
        assert_eq!(role_map.get_mapping("Unknown"), None);
    }

    #[test]
    fn test_struct_tree_creation() {
        let mut tree = StructTree::new();

        // Add root document element
        let doc = StructureElement::new(StandardStructureType::Document);
        let doc_idx = tree.set_root(doc);

        assert_eq!(tree.root_index(), Some(doc_idx));
        assert_eq!(tree.len(), 1);
    }

    #[test]
    fn test_struct_tree_hierarchy() {
        let mut tree = StructTree::new();

        // Create document root
        let doc = StructureElement::new(StandardStructureType::Document).with_id("doc1");
        let doc_idx = tree.set_root(doc);

        // Add heading
        let h1 = StructureElement::new(StandardStructureType::H1)
            .with_id("h1")
            .with_actual_text("Title");
        let h1_idx = tree.add_child(doc_idx, h1).unwrap();

        // Add paragraph
        let para = StructureElement::new(StandardStructureType::P).with_id("p1");
        let p_idx = tree.add_child(doc_idx, para).unwrap();

        assert_eq!(tree.len(), 3);
        assert_eq!(tree.get(doc_idx).unwrap().children.len(), 2);
        assert_eq!(tree.get(doc_idx).unwrap().children[0], h1_idx);
        assert_eq!(tree.get(doc_idx).unwrap().children[1], p_idx);

        // Test ID lookup
        assert!(tree.get_by_id("h1").is_some());
        assert!(tree.get_by_id("p1").is_some());
        assert!(tree.get_by_id("unknown").is_none());
    }

    #[test]
    fn test_marked_content_references() {
        let mut elem = StructureElement::new(StandardStructureType::P);
        elem.add_mcid(0, 1);
        elem.add_mcid(0, 2);

        assert_eq!(elem.mcids.len(), 2);
        assert_eq!(elem.mcids[0].page_index, 0);
        assert_eq!(elem.mcids[0].mcid, 1);
        assert_eq!(elem.mcids[1].mcid, 2);
    }

    #[test]
    fn test_custom_structure_type() {
        let elem = StructureElement::new_custom("MyCustomType");

        match elem.structure_type {
            StructureType::Custom(ref name) => assert_eq!(name, "MyCustomType"),
            _ => panic!("Expected custom structure type"),
        }
    }

    #[test]
    fn test_struct_tree_error_handling() {
        let mut tree = StructTree::new();

        // Try to add child to non-existent parent
        let elem = StructureElement::new(StandardStructureType::P);
        let result = tree.add_child(999, elem);

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("out of bounds"));
    }
}
