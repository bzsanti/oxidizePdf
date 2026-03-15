use std::cmp::Ordering;

#[cfg(feature = "semantic")]
use serde::{Deserialize, Serialize};

/// A typed document element extracted from a PDF page.
///
/// Each variant carries its specific data plus shared [`ElementMetadata`]
/// for page number, bounding box, confidence, and optional font info.
#[derive(Debug, Clone)]
#[cfg_attr(
    feature = "semantic",
    derive(Serialize, Deserialize),
    serde(tag = "type", rename_all = "snake_case")
)]
pub enum Element {
    Title(ElementData),
    Paragraph(ElementData),
    Table(TableElementData),
    Header(ElementData),
    Footer(ElementData),
    ListItem(ElementData),
    Image(ImageElementData),
    CodeBlock(ElementData),
    KeyValue(KeyValueElementData),
}

impl Element {
    /// Returns the primary text content of this element.
    pub fn text(&self) -> &str {
        match self {
            Self::Title(d)
            | Self::Paragraph(d)
            | Self::Header(d)
            | Self::Footer(d)
            | Self::ListItem(d)
            | Self::CodeBlock(d) => &d.text,
            Self::Table(t) => {
                // Tables don't have a single text — return empty.
                // Use row_count()/cell() for structured access.
                let _ = t;
                ""
            }
            Self::Image(img) => img.alt_text.as_deref().unwrap_or(""),
            Self::KeyValue(kv) => &kv.value,
        }
    }

    /// Returns a human-readable text representation of this element.
    ///
    /// Unlike [`text()`](Self::text) which returns raw content (empty for tables,
    /// value-only for KV pairs), this method produces a complete display form:
    /// - Tables: pipe-separated rows
    /// - Key-Value: "key: value"
    /// - All others: same as `text()`
    pub fn display_text(&self) -> String {
        match self {
            Self::Table(t) => t
                .rows
                .iter()
                .map(|row| row.join(" | "))
                .collect::<Vec<_>>()
                .join("\n"),
            Self::Image(img) => img.alt_text.clone().unwrap_or_default(),
            Self::KeyValue(kv) => format!("{}: {}", kv.key, kv.value),
            _ => self.text().to_string(),
        }
    }

    /// Returns the page number (0-indexed) where this element appears.
    pub fn page(&self) -> u32 {
        self.metadata().page
    }

    /// Returns the bounding box of this element on the page.
    pub fn bbox(&self) -> &ElementBBox {
        &self.metadata().bbox
    }

    /// Returns the full metadata for this element.
    pub fn metadata(&self) -> &ElementMetadata {
        match self {
            Self::Title(d)
            | Self::Paragraph(d)
            | Self::Header(d)
            | Self::Footer(d)
            | Self::ListItem(d)
            | Self::CodeBlock(d) => &d.metadata,
            Self::Table(t) => &t.metadata,
            Self::Image(img) => &img.metadata,
            Self::KeyValue(kv) => &kv.metadata,
        }
    }

    /// Returns a mutable reference to the metadata of this element.
    pub fn metadata_mut(&mut self) -> &mut ElementMetadata {
        match self {
            Self::Title(d)
            | Self::Paragraph(d)
            | Self::Header(d)
            | Self::Footer(d)
            | Self::ListItem(d)
            | Self::CodeBlock(d) => &mut d.metadata,
            Self::Table(t) => &mut t.metadata,
            Self::Image(img) => &mut img.metadata,
            Self::KeyValue(kv) => &mut kv.metadata,
        }
    }

    /// Returns the snake_case type name of this element variant.
    ///
    /// Useful for logging, serialization, and metadata tagging.
    /// Returns one of: `"title"`, `"paragraph"`, `"table"`, `"header"`,
    /// `"footer"`, `"list_item"`, `"image"`, `"code_block"`, `"key_value"`.
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::Title(_) => "title",
            Self::Paragraph(_) => "paragraph",
            Self::Table(_) => "table",
            Self::Header(_) => "header",
            Self::Footer(_) => "footer",
            Self::ListItem(_) => "list_item",
            Self::Image(_) => "image",
            Self::CodeBlock(_) => "code_block",
            Self::KeyValue(_) => "key_value",
        }
    }

    /// Set the parent heading for this element.
    pub fn set_parent_heading(&mut self, heading: Option<String>) {
        self.metadata_mut().parent_heading = heading;
    }

    /// Returns the number of rows if this is a Table element.
    pub fn row_count(&self) -> Option<usize> {
        match self {
            Self::Table(t) => Some(t.rows.len()),
            _ => None,
        }
    }

    /// Returns the number of columns if this is a Table element.
    pub fn column_count(&self) -> Option<usize> {
        match self {
            Self::Table(t) => t.rows.first().map(|r| r.len()),
            _ => None,
        }
    }

    /// Returns the cell text at (row, col) if this is a Table element.
    pub fn cell(&self, row: usize, col: usize) -> Option<&str> {
        match self {
            Self::Table(t) => t.rows.get(row).and_then(|r| r.get(col)).map(|s| s.as_str()),
            _ => None,
        }
    }
}

/// Comparator for sorting elements in natural reading order:
/// page ASC, then Y DESC (top-to-bottom in PDF coordinates), then X ASC (left-to-right).
///
/// Use with `elements.sort_by(element_reading_order)` instead of `elements.sort()`.
pub fn element_reading_order(a: &Element, b: &Element) -> Ordering {
    let page_cmp = a.page().cmp(&b.page());
    if page_cmp != Ordering::Equal {
        return page_cmp;
    }
    // Higher Y = higher on page in PDF coords → should come first
    let y_cmp = b.bbox().y.total_cmp(&a.bbox().y);
    if y_cmp != Ordering::Equal {
        return y_cmp;
    }
    a.bbox().x.total_cmp(&b.bbox().x)
}

impl PartialEq for Element {
    fn eq(&self, other: &Self) -> bool {
        std::mem::discriminant(self) == std::mem::discriminant(other) && self.content_eq(other)
    }
}

impl Eq for Element {}

impl Element {
    /// Content-based equality: compares text/data content, ignoring metadata (position, font, etc.).
    fn content_eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Title(a), Self::Title(b))
            | (Self::Paragraph(a), Self::Paragraph(b))
            | (Self::Header(a), Self::Header(b))
            | (Self::Footer(a), Self::Footer(b))
            | (Self::ListItem(a), Self::ListItem(b))
            | (Self::CodeBlock(a), Self::CodeBlock(b)) => a.text == b.text,
            (Self::Table(a), Self::Table(b)) => a.rows == b.rows,
            (Self::Image(a), Self::Image(b)) => a.alt_text == b.alt_text,
            (Self::KeyValue(a), Self::KeyValue(b)) => a.key == b.key && a.value == b.value,
            _ => false,
        }
    }
}

/// Shared data for text-based element variants.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "semantic", derive(Serialize, Deserialize))]
pub struct ElementData {
    pub text: String,
    pub metadata: ElementMetadata,
}

/// Data specific to table elements.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "semantic", derive(Serialize, Deserialize))]
pub struct TableElementData {
    /// Row-major cell data. Each inner Vec is one row.
    pub rows: Vec<Vec<String>>,
    pub metadata: ElementMetadata,
}

/// Data specific to image elements.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "semantic", derive(Serialize, Deserialize))]
pub struct ImageElementData {
    pub alt_text: Option<String>,
    pub metadata: ElementMetadata,
}

/// Data specific to key-value pair elements.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "semantic", derive(Serialize, Deserialize))]
pub struct KeyValueElementData {
    pub key: String,
    pub value: String,
    pub metadata: ElementMetadata,
}

/// Metadata common to all element types.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "semantic", derive(Serialize, Deserialize))]
pub struct ElementMetadata {
    /// Page number (0-indexed).
    pub page: u32,
    /// Bounding box on the page.
    pub bbox: ElementBBox,
    /// Classification confidence (0.0–1.0).
    pub confidence: f64,
    /// Font name if detected.
    pub font_name: Option<String>,
    /// Font size in points if detected.
    pub font_size: Option<f64>,
    /// Whether the text is bold.
    pub is_bold: bool,
    /// Whether the text is italic.
    pub is_italic: bool,
    /// The text of the nearest preceding Title element, if any.
    pub parent_heading: Option<String>,
}

impl Default for ElementMetadata {
    fn default() -> Self {
        Self {
            page: 0,
            bbox: ElementBBox::ZERO,
            confidence: 1.0,
            font_name: None,
            font_size: None,
            is_bold: false,
            is_italic: false,
            parent_heading: None,
        }
    }
}

/// Axis-aligned bounding box for an element on a PDF page.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "semantic", derive(Serialize, Deserialize))]
pub struct ElementBBox {
    /// Left edge X coordinate.
    pub x: f64,
    /// Bottom edge Y coordinate (PDF coordinate system).
    pub y: f64,
    /// Width of the bounding box.
    pub width: f64,
    /// Height of the bounding box.
    pub height: f64,
}

impl ElementBBox {
    /// A zero-sized bounding box at the origin.
    pub const ZERO: Self = Self {
        x: 0.0,
        y: 0.0,
        width: 0.0,
        height: 0.0,
    };

    /// Creates a new bounding box.
    pub fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Right edge X coordinate (x + width).
    pub fn right(&self) -> f64 {
        self.x + self.width
    }

    /// Top edge Y coordinate (y + height) in PDF coordinate system.
    pub fn top(&self) -> f64 {
        self.y + self.height
    }
}
