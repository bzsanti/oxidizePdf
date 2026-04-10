use crate::error::Result;
use crate::graphics::Image;
use crate::layout::image_utils::fit_image_dimensions;
use crate::layout::RichText;
use crate::page::Margins;
use crate::page_tables::PageTables;
use crate::text::text_block::measure_text_block;
use crate::text::{Font, Table, TextAlign, TextFlowContext};
use crate::{Document, Page};
use std::sync::Arc;

/// Page dimensions and margins for FlowLayout.
#[derive(Debug, Clone)]
pub struct PageConfig {
    pub width: f64,
    pub height: f64,
    pub margin_left: f64,
    pub margin_right: f64,
    pub margin_top: f64,
    pub margin_bottom: f64,
}

impl PageConfig {
    /// Create a config with explicit dimensions and margins.
    pub fn new(
        width: f64,
        height: f64,
        margin_left: f64,
        margin_right: f64,
        margin_top: f64,
        margin_bottom: f64,
    ) -> Self {
        let config = Self {
            width,
            height,
            margin_left,
            margin_right,
            margin_top,
            margin_bottom,
        };
        debug_assert!(
            config.content_width() > 0.0,
            "margins ({} + {}) exceed page width ({})",
            margin_left,
            margin_right,
            width
        );
        debug_assert!(
            config.usable_height() > 0.0,
            "margins ({} + {}) exceed page height ({})",
            margin_top,
            margin_bottom,
            height
        );
        config
    }

    /// A4 page (595×842 pts) with default 72pt margins.
    pub fn a4() -> Self {
        Self::new(595.0, 842.0, 72.0, 72.0, 72.0, 72.0)
    }

    /// A4 page with custom uniform margins on all sides.
    pub fn a4_with_margins(left: f64, right: f64, top: f64, bottom: f64) -> Self {
        Self::new(595.0, 842.0, left, right, top, bottom)
    }

    /// Available width for content (page width minus left and right margins).
    pub fn content_width(&self) -> f64 {
        self.width - self.margin_left - self.margin_right
    }

    /// Available height for content (page height minus top and bottom margins).
    pub fn usable_height(&self) -> f64 {
        self.height - self.margin_top - self.margin_bottom
    }

    fn start_y(&self) -> f64 {
        self.height - self.margin_top
    }

    fn create_page(&self) -> Page {
        let mut page = Page::new(self.width, self.height);
        page.set_margins(
            self.margin_left,
            self.margin_right,
            self.margin_top,
            self.margin_bottom,
        );
        page
    }

    fn to_margins(&self) -> Margins {
        Margins {
            left: self.margin_left,
            right: self.margin_right,
            top: self.margin_top,
            bottom: self.margin_bottom,
        }
    }
}

/// An element that can be placed in a FlowLayout.
#[derive(Debug)]
pub enum FlowElement {
    /// A block of word-wrapped text.
    Text {
        text: String,
        font: Font,
        font_size: f64,
        line_height: f64,
    },
    /// Vertical space in points.
    Spacer(f64),
    /// A simple table.
    Table(Table),
    /// A single line of mixed-style text.
    RichText { rich: RichText, line_height: f64 },
    /// An image scaled to fit within max dimensions, preserving aspect ratio.
    /// Uses `Arc<Image>` to avoid cloning the pixel buffer when building.
    Image {
        name: String,
        image: Arc<Image>,
        max_width: f64,
        max_height: f64,
        center: bool,
    },
}

impl FlowElement {
    /// Calculate the height this element will occupy.
    fn measure_height(&self, content_width: f64) -> f64 {
        match self {
            FlowElement::Text {
                text,
                font,
                font_size,
                line_height,
            } => {
                let metrics =
                    measure_text_block(text, font, *font_size, *line_height, content_width);
                metrics.height
            }
            FlowElement::Spacer(h) => *h,
            FlowElement::Table(table) => table.get_height(),
            FlowElement::RichText { rich, line_height } => rich.max_font_size() * line_height,
            FlowElement::Image {
                image,
                max_width,
                max_height,
                ..
            } => {
                let (_, h) =
                    fit_image_dimensions(image.width(), image.height(), *max_width, *max_height);
                h
            }
        }
    }
}

/// Automatic flow layout engine with page break support.
///
/// Manages a vertical cursor and a list of elements. When an element
/// would overflow the current page's bottom margin, a new page is
/// created automatically.
///
/// # Example
///
/// ```rust,no_run
/// use oxidize_pdf::{Document, Font};
/// use oxidize_pdf::layout::{FlowLayout, PageConfig};
///
/// let config = PageConfig::a4_with_margins(50.0, 50.0, 50.0, 50.0);
/// let mut layout = FlowLayout::new(config);
/// layout.add_text("Hello World", Font::Helvetica, 12.0);
/// layout.add_spacer(20.0);
/// layout.add_text("Second paragraph", Font::Helvetica, 12.0);
///
/// let mut doc = Document::new();
/// layout.build_into(&mut doc).unwrap();
/// ```
pub struct FlowLayout {
    config: PageConfig,
    elements: Vec<FlowElement>,
}

impl FlowLayout {
    /// Create a new FlowLayout with the given page configuration.
    pub fn new(config: PageConfig) -> Self {
        Self {
            config,
            elements: Vec::new(),
        }
    }

    /// Add a text block. Uses default line_height of 1.2.
    pub fn add_text(&mut self, text: &str, font: Font, font_size: f64) -> &mut Self {
        self.elements.push(FlowElement::Text {
            text: text.to_string(),
            font,
            font_size,
            line_height: 1.2,
        });
        self
    }

    /// Add a text block with custom line height.
    pub fn add_text_with_line_height(
        &mut self,
        text: &str,
        font: Font,
        font_size: f64,
        line_height: f64,
    ) -> &mut Self {
        self.elements.push(FlowElement::Text {
            text: text.to_string(),
            font,
            font_size,
            line_height,
        });
        self
    }

    /// Add vertical spacing in points.
    pub fn add_spacer(&mut self, points: f64) -> &mut Self {
        self.elements.push(FlowElement::Spacer(points));
        self
    }

    /// Add a table.
    pub fn add_table(&mut self, table: Table) -> &mut Self {
        self.elements.push(FlowElement::Table(table));
        self
    }

    /// Add an image scaled to fit within max dimensions, left-aligned.
    /// Wraps the image in `Arc` internally to avoid expensive buffer clones.
    pub fn add_image(
        &mut self,
        name: &str,
        image: Arc<Image>,
        max_width: f64,
        max_height: f64,
    ) -> &mut Self {
        self.elements.push(FlowElement::Image {
            name: name.to_string(),
            image,
            max_width,
            max_height,
            center: false,
        });
        self
    }

    /// Add an image scaled to fit within max dimensions, centered horizontally.
    /// Wraps the image in `Arc` internally to avoid expensive buffer clones.
    pub fn add_image_centered(
        &mut self,
        name: &str,
        image: Arc<Image>,
        max_width: f64,
        max_height: f64,
    ) -> &mut Self {
        self.elements.push(FlowElement::Image {
            name: name.to_string(),
            image,
            max_width,
            max_height,
            center: true,
        });
        self
    }

    /// Add a single line of mixed-style text.
    pub fn add_rich_text(&mut self, rich: RichText) -> &mut Self {
        self.elements.push(FlowElement::RichText {
            rich,
            line_height: 1.2,
        });
        self
    }

    /// Build all elements into the document, creating pages as needed.
    ///
    /// **Limitation**: Elements taller than `PageConfig::usable_height()` (e.g., a very
    /// large table) will overflow past the bottom margin on a single page. They are not
    /// split across pages.
    pub fn build_into(&self, doc: &mut Document) -> Result<()> {
        let content_width = self.config.content_width();
        let mut current_page = self.config.create_page();
        let mut cursor_y = self.config.start_y();

        for element in &self.elements {
            let needed_height = element.measure_height(content_width);

            // Page break: if element doesn't fit and we've already placed something
            if cursor_y - needed_height < self.config.margin_bottom
                && cursor_y < self.config.start_y()
            {
                doc.add_page(current_page);
                current_page = self.config.create_page();
                cursor_y = self.config.start_y();
            }

            match element {
                FlowElement::Text {
                    text,
                    font,
                    font_size,
                    line_height,
                } => {
                    let mut text_flow = TextFlowContext::new(
                        self.config.width,
                        self.config.height,
                        self.config.to_margins(),
                    );
                    text_flow
                        .set_font(font.clone(), *font_size)
                        .set_line_height(*line_height)
                        .set_alignment(TextAlign::Left)
                        .at(self.config.margin_left, cursor_y - font_size * line_height);
                    text_flow.write_wrapped(text)?;
                    current_page.add_text_flow(&text_flow);
                }
                FlowElement::Spacer(_) => {
                    // Spacers only consume vertical space, no rendering needed
                }
                FlowElement::Table(table) => {
                    current_page.add_simple_table(
                        table,
                        self.config.margin_left,
                        cursor_y - needed_height,
                    )?;
                }
                FlowElement::RichText { rich, line_height } => {
                    let ops = rich.render_operations(
                        self.config.margin_left,
                        cursor_y - rich.max_font_size() * line_height,
                    );
                    current_page.append_raw_content(ops.as_bytes());
                }
                FlowElement::Image {
                    name,
                    image,
                    max_width,
                    max_height,
                    center,
                } => {
                    let (w, h) = fit_image_dimensions(
                        image.width(),
                        image.height(),
                        *max_width,
                        *max_height,
                    );
                    let x = if *center {
                        crate::layout::image_utils::centered_image_x(
                            self.config.margin_left,
                            content_width,
                            w,
                        )
                    } else {
                        self.config.margin_left
                    };
                    current_page.add_image(name.clone(), Image::clone(image));
                    current_page.draw_image(name, x, cursor_y - h, w, h)?;
                }
            }

            cursor_y -= needed_height;
        }

        doc.add_page(current_page);
        Ok(())
    }
}
