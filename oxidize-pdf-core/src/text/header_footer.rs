//! Header and footer support for PDF pages.
//!
//! This module provides functionality for adding headers and footers to PDF pages,
//! including support for dynamic placeholders like page numbers.

use crate::text::{Font, TextAlign};
use chrono::Local;
use std::collections::HashMap;

/// Position for headers and footers on a page.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HeaderFooterPosition {
    /// Header at the top of the page
    Header,
    /// Footer at the bottom of the page
    Footer,
}

/// Configuration options for headers and footers.
#[derive(Debug, Clone)]
pub struct HeaderFooterOptions {
    /// Font to use for the header/footer text
    pub font: Font,
    /// Font size in points
    pub font_size: f64,
    /// Text alignment
    pub alignment: TextAlign,
    /// Vertical offset from page edge in points
    pub margin: f64,
    /// Whether to include page numbers
    pub show_page_numbers: bool,
    /// Custom date format (if None, uses default)
    pub date_format: Option<String>,
}

impl Default for HeaderFooterOptions {
    fn default() -> Self {
        Self {
            font: Font::Helvetica,
            font_size: 10.0,
            alignment: TextAlign::Center,
            margin: 36.0, // 0.5 inch
            show_page_numbers: true,
            date_format: None,
        }
    }
}

/// A header or footer that can be added to PDF pages.
#[derive(Debug, Clone)]
pub struct HeaderFooter {
    /// The position of this header/footer
    position: HeaderFooterPosition,
    /// The content template with optional placeholders
    content: String,
    /// Configuration options
    options: HeaderFooterOptions,
}

impl HeaderFooter {
    /// Creates a new header with the given content.
    ///
    /// # Example
    ///
    /// ```rust
    /// use oxidize_pdf::text::{HeaderFooter, HeaderFooterOptions};
    ///
    /// let header = HeaderFooter::new_header("Annual Report {{year}}");
    /// ```
    pub fn new_header(content: impl Into<String>) -> Self {
        Self {
            position: HeaderFooterPosition::Header,
            content: content.into(),
            options: HeaderFooterOptions::default(),
        }
    }

    /// Creates a new footer with the given content.
    ///
    /// # Example
    ///
    /// ```rust
    /// use oxidize_pdf::text::{HeaderFooter, HeaderFooterOptions};
    ///
    /// let footer = HeaderFooter::new_footer("Page {{page_number}} of {{total_pages}}");
    /// ```
    pub fn new_footer(content: impl Into<String>) -> Self {
        Self {
            position: HeaderFooterPosition::Footer,
            content: content.into(),
            options: HeaderFooterOptions::default(),
        }
    }

    /// Sets the options for this header/footer.
    pub fn with_options(mut self, options: HeaderFooterOptions) -> Self {
        self.options = options;
        self
    }

    /// Sets the font for this header/footer.
    pub fn with_font(mut self, font: Font, size: f64) -> Self {
        self.options.font = font;
        self.options.font_size = size;
        self
    }

    /// Sets the text alignment.
    pub fn with_alignment(mut self, alignment: TextAlign) -> Self {
        self.options.alignment = alignment;
        self
    }

    /// Sets the margin from the page edge.
    pub fn with_margin(mut self, margin: f64) -> Self {
        self.options.margin = margin;
        self
    }

    /// Gets the position of this header/footer.
    pub fn position(&self) -> HeaderFooterPosition {
        self.position
    }

    /// Gets the content template.
    pub fn content(&self) -> &str {
        &self.content
    }

    /// Gets the options.
    pub fn options(&self) -> &HeaderFooterOptions {
        &self.options
    }

    /// Renders the header/footer content with placeholder substitution.
    ///
    /// Available placeholders:
    /// - `{{page_number}}` - Current page number
    /// - `{{total_pages}}` - Total number of pages
    /// - `{{date}}` - Current date
    /// - `{{time}}` - Current time
    /// - `{{datetime}}` - Current date and time
    /// - `{{year}}` - Current year
    /// - `{{month}}` - Current month
    /// - `{{day}}` - Current day
    pub fn render(
        &self,
        page_number: usize,
        total_pages: usize,
        custom_values: Option<&HashMap<String, String>>,
    ) -> String {
        let mut result = self.content.clone();

        // Replace standard placeholders
        result = result.replace("{{page_number}}", &page_number.to_string());
        result = result.replace("{{total_pages}}", &total_pages.to_string());

        // Date/time placeholders
        let now = Local::now();
        result = result.replace("{{year}}", &now.format("%Y").to_string());
        result = result.replace("{{month}}", &now.format("%B").to_string());
        result = result.replace("{{day}}", &now.format("%d").to_string());

        if let Some(date_format) = &self.options.date_format {
            result = result.replace("{{date}}", &now.format(date_format).to_string());
        } else {
            result = result.replace("{{date}}", &now.format("%Y-%m-%d").to_string());
        }

        result = result.replace("{{time}}", &now.format("%H:%M:%S").to_string());
        result = result.replace("{{datetime}}", &now.format("%Y-%m-%d %H:%M:%S").to_string());

        // Replace custom values if provided
        if let Some(custom) = custom_values {
            for (key, value) in custom {
                result = result.replace(&format!("{{{{{key}}}}}"), value);
            }
        }

        result
    }

    /// Calculates the Y position for rendering based on page height and position.
    pub fn calculate_y_position(&self, page_height: f64) -> f64 {
        match self.position {
            HeaderFooterPosition::Header => page_height - self.options.margin,
            HeaderFooterPosition::Footer => self.options.margin,
        }
    }

    /// Calculates the X position for rendering based on page width and alignment.
    pub fn calculate_x_position(&self, page_width: f64, text_width: f64) -> f64 {
        match self.options.alignment {
            TextAlign::Left => self.options.margin,
            TextAlign::Center => (page_width - text_width) / 2.0,
            TextAlign::Right => page_width - self.options.margin - text_width,
            TextAlign::Justified => self.options.margin, // Justified acts as left for single line
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_header_creation() {
        let header = HeaderFooter::new_header("Test Header");
        assert_eq!(header.position(), HeaderFooterPosition::Header);
        assert_eq!(header.content(), "Test Header");
        assert_eq!(header.options().font_size, 10.0);
    }

    #[test]
    fn test_footer_creation() {
        let footer = HeaderFooter::new_footer("Test Footer");
        assert_eq!(footer.position(), HeaderFooterPosition::Footer);
        assert_eq!(footer.content(), "Test Footer");
    }

    #[test]
    fn test_with_options() {
        let options = HeaderFooterOptions {
            font: Font::TimesRoman,
            font_size: 12.0,
            alignment: TextAlign::Right,
            margin: 20.0,
            show_page_numbers: false,
            date_format: Some("%d/%m/%Y".to_string()),
        };

        let header = HeaderFooter::new_header("Test").with_options(options.clone());
        assert_eq!(header.options().font_size, 12.0);
        assert_eq!(header.options().margin, 20.0);
    }

    #[test]
    fn test_render_page_numbers() {
        let footer = HeaderFooter::new_footer("Page {{page_number}} of {{total_pages}}");
        let rendered = footer.render(3, 10, None);
        assert_eq!(rendered, "Page 3 of 10");
    }

    #[test]
    fn test_render_date_placeholders() {
        let header = HeaderFooter::new_header("Report {{year}} - {{month}}");
        let rendered = header.render(1, 1, None);

        // Check that year is 4 digits and month is replaced
        assert!(rendered.contains("Report 20"));
        assert!(!rendered.contains("{{year}}"));
        assert!(!rendered.contains("{{month}}"));
    }

    #[test]
    fn test_render_custom_values() {
        let mut custom = HashMap::new();
        custom.insert("title".to_string(), "Annual Report".to_string());
        custom.insert("company".to_string(), "ACME Corp".to_string());

        let header = HeaderFooter::new_header("{{company}} - {{title}}");
        let rendered = header.render(1, 1, Some(&custom));
        assert_eq!(rendered, "ACME Corp - Annual Report");
    }

    #[test]
    fn test_calculate_positions() {
        let header = HeaderFooter::new_header("Test").with_margin(50.0);
        let footer = HeaderFooter::new_footer("Test").with_margin(50.0);

        assert_eq!(header.calculate_y_position(842.0), 792.0); // A4 height - margin
        assert_eq!(footer.calculate_y_position(842.0), 50.0); // margin
    }

    #[test]
    fn test_alignment_positions() {
        let page_width = 595.0; // A4 width
        let text_width = 100.0;
        let margin = 36.0;

        let left = HeaderFooter::new_header("Test")
            .with_alignment(TextAlign::Left)
            .with_margin(margin);
        assert_eq!(left.calculate_x_position(page_width, text_width), margin);

        let center = HeaderFooter::new_header("Test").with_alignment(TextAlign::Center);
        assert_eq!(
            center.calculate_x_position(page_width, text_width),
            (page_width - text_width) / 2.0
        );

        let right = HeaderFooter::new_header("Test")
            .with_alignment(TextAlign::Right)
            .with_margin(margin);
        assert_eq!(
            right.calculate_x_position(page_width, text_width),
            page_width - margin - text_width
        );
    }

    #[test]
    fn test_no_placeholders() {
        let header = HeaderFooter::new_header("Static Header Text");
        let rendered = header.render(1, 10, None);
        assert_eq!(rendered, "Static Header Text");
    }

    #[test]
    fn test_multiple_placeholders() {
        let footer = HeaderFooter::new_footer(
            "{{date}} | Page {{page_number}} of {{total_pages}} | {{time}}",
        );
        let rendered = footer.render(5, 20, None);

        // Check structure is maintained
        assert!(rendered.contains(" | Page 5 of 20 | "));
        assert!(!rendered.contains("{{"));
    }
}
