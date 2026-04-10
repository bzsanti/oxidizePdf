use crate::error::Result;
use crate::graphics::Image;
use crate::layout::{FlowLayout, PageConfig, RichText};
use crate::text::{Font, Table};
use crate::Document;
use std::sync::Arc;

/// High-level builder for creating multi-page PDF documents with automatic layout.
///
/// Wraps [`FlowLayout`] with an owned-chaining API so you can build a complete
/// document in a single expression.
///
/// # Example
///
/// ```rust,no_run
/// use oxidize_pdf::layout::DocumentBuilder;
/// use oxidize_pdf::Font;
///
/// let mut doc = DocumentBuilder::a4()
///     .add_text("Invoice #001", Font::HelveticaBold, 18.0)
///     .add_spacer(10.0)
///     .add_text("Date: 2026-04-09", Font::Helvetica, 12.0)
///     .build()
///     .unwrap();
///
/// doc.save("invoice.pdf").unwrap();
/// ```
pub struct DocumentBuilder {
    layout: FlowLayout,
}

impl DocumentBuilder {
    /// Create a builder with A4 page size and default 72pt margins.
    pub fn a4() -> Self {
        Self {
            layout: FlowLayout::new(PageConfig::a4()),
        }
    }

    /// Create a builder with custom page configuration.
    pub fn new(config: PageConfig) -> Self {
        Self {
            layout: FlowLayout::new(config),
        }
    }

    /// Add a text block with default line height (1.2).
    pub fn add_text(mut self, text: &str, font: Font, font_size: f64) -> Self {
        self.layout.add_text(text, font, font_size);
        self
    }

    /// Add a text block with custom line height.
    pub fn add_text_with_line_height(
        mut self,
        text: &str,
        font: Font,
        font_size: f64,
        line_height: f64,
    ) -> Self {
        self.layout
            .add_text_with_line_height(text, font, font_size, line_height);
        self
    }

    /// Add vertical spacing in points.
    pub fn add_spacer(mut self, points: f64) -> Self {
        self.layout.add_spacer(points);
        self
    }

    /// Add a table.
    pub fn add_table(mut self, table: Table) -> Self {
        self.layout.add_table(table);
        self
    }

    /// Add an image scaled to fit within max dimensions, left-aligned.
    pub fn add_image(
        mut self,
        name: &str,
        image: Arc<Image>,
        max_width: f64,
        max_height: f64,
    ) -> Self {
        self.layout.add_image(name, image, max_width, max_height);
        self
    }

    /// Add an image scaled to fit, centered horizontally.
    pub fn add_image_centered(
        mut self,
        name: &str,
        image: Arc<Image>,
        max_width: f64,
        max_height: f64,
    ) -> Self {
        self.layout
            .add_image_centered(name, image, max_width, max_height);
        self
    }

    /// Add a single line of mixed-style text.
    pub fn add_rich_text(mut self, rich: RichText) -> Self {
        self.layout.add_rich_text(rich);
        self
    }

    /// Build the document, creating pages as needed for all added elements.
    pub fn build(self) -> Result<Document> {
        let mut doc = Document::new();
        self.layout.build_into(&mut doc)?;
        Ok(doc)
    }
}
