//! PDF Template System with Variable Substitution
//!
//! This module provides a template system for creating dynamic PDFs with variable substitution.
//! It supports placeholders in the format `{{variable_name}}` and provides context management
//! for template rendering.
//!
//! # Examples
//!
//! ```rust
//! use oxidize_pdf::templates::{Template, TemplateContext};
//! use oxidize_pdf::{Document, Page, Font};
//!
//! fn example() -> Result<(), Box<dyn std::error::Error>> {
//!     let mut context = TemplateContext::new();
//!     context.set("name", "John Doe");
//!     context.set("date", "2024-01-15");
//!     context.set("total", "$1,234.56");
//!
//!     let template_text = "Invoice for {{name}} - Date: {{date}} - Total: {{total}}";
//!     let rendered = Template::render(template_text, &context)?;
//!     Ok(())
//! }
//! ```

mod context;
mod error;
mod parser;
mod renderer;

#[cfg(test)]
mod integration_test;

pub use context::{TemplateContext, TemplateValue};
pub use error::{TemplateError, TemplateResult};
pub use parser::{Placeholder, TemplateParser};
pub use renderer::{Template, TemplateRenderer};

/// Re-export for convenience
pub type Result<T> = std::result::Result<T, TemplateError>;
