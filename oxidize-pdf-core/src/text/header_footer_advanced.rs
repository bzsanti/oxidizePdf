//! Advanced header and footer support with template engine.
//!
//! This module provides enhanced functionality for headers and footers including:
//! - Multi-line support
//! - Different headers/footers for odd/even pages
//! - Left/Center/Right sections in the same line
//! - Advanced template variables and conditionals

use crate::error::PdfError;
use crate::graphics::{Color, GraphicsContext};
use crate::text::{measure_text, Font};
use chrono::{DateTime, Local};
use std::collections::HashMap;

/// Advanced template engine for processing header/footer templates
#[derive(Debug, Clone)]
pub struct TemplateEngine {
    /// Custom variables that can be used in templates
    variables: HashMap<String, String>,
    /// Whether to enable conditional expressions
    enable_conditionals: bool,
}

impl Default for TemplateEngine {
    fn default() -> Self {
        Self {
            variables: HashMap::new(),
            enable_conditionals: true,
        }
    }
}

impl TemplateEngine {
    /// Create a new template engine
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a custom variable
    pub fn add_variable(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.variables.insert(key.into(), value.into());
    }

    /// Add multiple variables at once
    pub fn add_variables(&mut self, vars: HashMap<String, String>) {
        self.variables.extend(vars);
    }

    /// Process a template string with variable substitution
    pub fn process(
        &self,
        template: &str,
        page_number: usize,
        total_pages: usize,
        is_odd_page: bool,
    ) -> String {
        let mut result = template.to_string();

        // Standard page variables
        result = result.replace("{{page}}", &page_number.to_string());
        result = result.replace("{{page_number}}", &page_number.to_string());
        result = result.replace("{{total_pages}}", &total_pages.to_string());
        result = result.replace("{{total}}", &total_pages.to_string());

        // Page type variables
        result = result.replace("{{page_type}}", if is_odd_page { "odd" } else { "even" });
        result = result.replace("{{is_odd}}", if is_odd_page { "true" } else { "false" });
        result = result.replace("{{is_even}}", if !is_odd_page { "true" } else { "false" });

        // Date and time variables
        let now = Local::now();
        result = self.process_date_time(&result, &now);

        // Custom variables
        for (key, value) in &self.variables {
            result = result.replace(&format!("{{{{{}}}}}", key), value);
        }

        // Process conditionals if enabled
        if self.enable_conditionals {
            result = self.process_conditionals(&result, is_odd_page, page_number);
        }

        result
    }

    fn process_date_time(&self, template: &str, now: &DateTime<Local>) -> String {
        let mut result = template.to_string();

        // Date formats
        result = result.replace("{{date}}", &now.format("%Y-%m-%d").to_string());
        result = result.replace("{{date_us}}", &now.format("%m/%d/%Y").to_string());
        result = result.replace("{{date_eu}}", &now.format("%d/%m/%Y").to_string());
        result = result.replace("{{date_long}}", &now.format("%B %d, %Y").to_string());

        // Time formats
        result = result.replace("{{time}}", &now.format("%H:%M:%S").to_string());
        result = result.replace("{{time_12h}}", &now.format("%I:%M %p").to_string());

        // Individual components
        result = result.replace("{{year}}", &now.format("%Y").to_string());
        result = result.replace("{{month}}", &now.format("%m").to_string());
        result = result.replace("{{month_name}}", &now.format("%B").to_string());
        result = result.replace("{{month_short}}", &now.format("%b").to_string());
        result = result.replace("{{day}}", &now.format("%d").to_string());
        result = result.replace("{{weekday}}", &now.format("%A").to_string());
        result = result.replace("{{weekday_short}}", &now.format("%a").to_string());

        result
    }

    fn process_conditionals(
        &self,
        template: &str,
        is_odd_page: bool,
        page_number: usize,
    ) -> String {
        let mut result = template.to_string();

        // Simple odd/even conditionals
        if is_odd_page {
            result = result.replace("{{#if_odd}}", "");
            result = result.replace("{{/if_odd}}", "");
            // Remove even content
            while let Some(start) = result.find("{{#if_even}}") {
                if let Some(end) = result.find("{{/if_even}}") {
                    result.replace_range(start..=end + 11, "");
                } else {
                    break;
                }
            }
        } else {
            result = result.replace("{{#if_even}}", "");
            result = result.replace("{{/if_even}}", "");
            // Remove odd content
            while let Some(start) = result.find("{{#if_odd}}") {
                if let Some(end) = result.find("{{/if_odd}}") {
                    result.replace_range(start..=end + 10, "");
                } else {
                    break;
                }
            }
        }

        // First/last page conditionals
        if page_number == 1 {
            result = result.replace("{{#if_first}}", "");
            result = result.replace("{{/if_first}}", "");
        } else {
            while let Some(start) = result.find("{{#if_first}}") {
                if let Some(end) = result.find("{{/if_first}}") {
                    result.replace_range(start..=end + 12, "");
                } else {
                    break;
                }
            }
        }

        result
    }
}

/// Section layout for headers and footers
#[derive(Debug, Clone, Default)]
pub struct SectionLayout {
    /// Left-aligned content
    pub left: Option<String>,
    /// Center-aligned content
    pub center: Option<String>,
    /// Right-aligned content
    pub right: Option<String>,
}

impl SectionLayout {
    /// Create a new empty section layout
    pub fn new() -> Self {
        Self::default()
    }

    /// Set left content
    pub fn with_left(mut self, content: impl Into<String>) -> Self {
        self.left = Some(content.into());
        self
    }

    /// Set center content
    pub fn with_center(mut self, content: impl Into<String>) -> Self {
        self.center = Some(content.into());
        self
    }

    /// Set right content
    pub fn with_right(mut self, content: impl Into<String>) -> Self {
        self.right = Some(content.into());
        self
    }
}

/// Advanced header/footer configuration
#[derive(Debug, Clone)]
pub struct AdvancedHeaderFooter {
    /// Content for odd pages
    pub odd_pages: Vec<SectionLayout>,
    /// Content for even pages
    pub even_pages: Vec<SectionLayout>,
    /// Font settings
    pub font: Font,
    pub font_size: f64,
    /// Text color
    pub text_color: Color,
    /// Vertical margin from page edge
    pub margin: f64,
    /// Horizontal margins
    pub margin_left: f64,
    pub margin_right: f64,
    /// Line spacing for multi-line headers/footers
    pub line_spacing: f64,
    /// Template engine for processing variables
    pub template_engine: TemplateEngine,
}

impl Default for AdvancedHeaderFooter {
    fn default() -> Self {
        Self {
            odd_pages: Vec::new(),
            even_pages: Vec::new(),
            font: Font::Helvetica,
            font_size: 10.0,
            text_color: Color::black(),
            margin: 36.0,
            margin_left: 36.0,
            margin_right: 36.0,
            line_spacing: 14.0,
            template_engine: TemplateEngine::new(),
        }
    }
}

impl AdvancedHeaderFooter {
    /// Create a simple header/footer with centered text
    pub fn simple(content: impl Into<String>) -> Self {
        let mut hf = Self::default();
        let section = SectionLayout::new().with_center(content.into());
        hf.odd_pages.push(section.clone());
        hf.even_pages.push(section);
        hf
    }

    /// Create a header/footer with page numbering on the right
    pub fn with_page_numbers() -> Self {
        let mut hf = Self::default();
        let section = SectionLayout::new().with_right("Page {{page}} of {{total_pages}}");
        hf.odd_pages.push(section.clone());
        hf.even_pages.push(section);
        hf
    }

    /// Create a professional header with title and page numbers
    pub fn professional(title: impl Into<String>) -> Self {
        let mut hf = Self::default();

        // Odd pages: title on left, page on right
        let odd_section = SectionLayout::new()
            .with_left(title.into())
            .with_right("{{page}}");
        hf.odd_pages.push(odd_section);

        // Even pages: page on left, title on right
        let title_str = hf.odd_pages[0].left.clone().unwrap();
        let even_section = SectionLayout::new()
            .with_left("{{page}}")
            .with_right(title_str);
        hf.even_pages.push(even_section);

        hf
    }

    /// Add a line to odd pages
    pub fn add_odd_line(&mut self, layout: SectionLayout) {
        self.odd_pages.push(layout);
    }

    /// Add a line to even pages
    pub fn add_even_line(&mut self, layout: SectionLayout) {
        self.even_pages.push(layout);
    }

    /// Add the same line to both odd and even pages
    pub fn add_line(&mut self, layout: SectionLayout) {
        self.odd_pages.push(layout.clone());
        self.even_pages.push(layout);
    }

    /// Set a custom variable in the template engine
    pub fn set_variable(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.template_engine.add_variable(key, value);
    }

    /// Render the header/footer to a graphics context
    pub fn render(
        &self,
        graphics: &mut GraphicsContext,
        page_width: f64,
        page_height: f64,
        page_number: usize,
        total_pages: usize,
        is_header: bool,
    ) -> Result<(), PdfError> {
        let is_odd_page = page_number % 2 == 1;
        let sections = if is_odd_page {
            &self.odd_pages
        } else {
            &self.even_pages
        };

        if sections.is_empty() {
            return Ok(());
        }

        // Calculate starting Y position
        let mut y_pos = if is_header {
            page_height - self.margin
        } else {
            self.margin + (sections.len() as f64 - 1.0) * self.line_spacing
        };

        // Render each line
        for section in sections {
            self.render_section(
                graphics,
                section,
                page_width,
                y_pos,
                page_number,
                total_pages,
                is_odd_page,
            )?;

            // Move to next line position
            y_pos -= self.line_spacing;
        }

        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn render_section(
        &self,
        graphics: &mut GraphicsContext,
        section: &SectionLayout,
        page_width: f64,
        y_pos: f64,
        page_number: usize,
        total_pages: usize,
        is_odd_page: bool,
    ) -> Result<(), PdfError> {
        graphics.save_state();
        graphics.set_fill_color(self.text_color);
        graphics.set_font(self.font.clone(), self.font_size);

        // Process and render left section
        if let Some(left_template) = &section.left {
            let text =
                self.template_engine
                    .process(left_template, page_number, total_pages, is_odd_page);
            graphics.begin_text();
            graphics.set_text_position(self.margin_left, y_pos);
            graphics.show_text(&text)?;
            graphics.end_text();
        }

        // Process and render center section
        if let Some(center_template) = &section.center {
            let text = self.template_engine.process(
                center_template,
                page_number,
                total_pages,
                is_odd_page,
            );
            let text_width = measure_text(&text, self.font.clone(), self.font_size);
            let x_pos = (page_width - text_width) / 2.0;

            graphics.begin_text();
            graphics.set_text_position(x_pos, y_pos);
            graphics.show_text(&text)?;
            graphics.end_text();
        }

        // Process and render right section
        if let Some(right_template) = &section.right {
            let text =
                self.template_engine
                    .process(right_template, page_number, total_pages, is_odd_page);
            let text_width = measure_text(&text, self.font.clone(), self.font_size);
            let x_pos = page_width - self.margin_right - text_width;

            graphics.begin_text();
            graphics.set_text_position(x_pos, y_pos);
            graphics.show_text(&text)?;
            graphics.end_text();
        }

        graphics.restore_state();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_template_engine_basic() {
        let engine = TemplateEngine::new();
        let result = engine.process("Page {{page}} of {{total}}", 3, 10, true);
        assert_eq!(result, "Page 3 of 10");
    }

    #[test]
    fn test_template_engine_custom_variables() {
        let mut engine = TemplateEngine::new();
        engine.add_variable("title", "Annual Report");
        engine.add_variable("author", "John Doe");

        let result = engine.process("{{title}} by {{author}}", 1, 1, true);
        assert_eq!(result, "Annual Report by John Doe");
    }

    #[test]
    fn test_template_conditionals_odd() {
        let engine = TemplateEngine::new();
        let template = "{{#if_odd}}Odd page{{/if_odd}}{{#if_even}}Even page{{/if_even}}";

        let odd_result = engine.process(template, 1, 2, true);
        assert_eq!(odd_result, "Odd page");

        let even_result = engine.process(template, 2, 2, false);
        assert_eq!(even_result, "Even page");
    }

    #[test]
    fn test_section_layout() {
        let layout = SectionLayout::new()
            .with_left("Left")
            .with_center("Center")
            .with_right("Right");

        assert_eq!(layout.left, Some("Left".to_string()));
        assert_eq!(layout.center, Some("Center".to_string()));
        assert_eq!(layout.right, Some("Right".to_string()));
    }

    #[test]
    fn test_advanced_header_simple() {
        let hf = AdvancedHeaderFooter::simple("Test Header");
        assert_eq!(hf.odd_pages.len(), 1);
        assert_eq!(hf.even_pages.len(), 1);
        assert_eq!(hf.odd_pages[0].center, Some("Test Header".to_string()));
    }

    #[test]
    fn test_advanced_header_professional() {
        let hf = AdvancedHeaderFooter::professional("My Document");

        // Check odd pages
        assert_eq!(hf.odd_pages[0].left, Some("My Document".to_string()));
        assert_eq!(hf.odd_pages[0].right, Some("{{page}}".to_string()));

        // Check even pages
        assert_eq!(hf.even_pages[0].left, Some("{{page}}".to_string()));
        assert_eq!(hf.even_pages[0].right, Some("My Document".to_string()));
    }

    #[test]
    fn test_date_time_variables() {
        let engine = TemplateEngine::new();
        let result = engine.process("Date: {{date}} Year: {{year}}", 1, 1, true);

        // Just check that placeholders are replaced
        assert!(!result.contains("{{date}}"));
        assert!(!result.contains("{{year}}"));
        assert!(result.contains("Date: "));
        assert!(result.contains("Year: "));
    }
}
