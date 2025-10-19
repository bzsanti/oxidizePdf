//! Advanced Table System for PDF Generation
//!
//! This module provides a comprehensive table system with advanced styling capabilities,
//! complex headers, cell spanning, and professional formatting options.
//!
//! # Features
//! - CSS-style cell styling (padding, borders, colors)
//! - Complex headers with colspan/rowspan support
//! - Alternating row colors and zebra striping
//! - Flexible column width management
//! - Nested tables support
//! - Professional border styles (solid, dashed, dotted, double)
//!
//! # Example
//! ```rust
//! use oxidize_pdf::advanced_tables::{AdvancedTableBuilder, CellStyle};
//! use oxidize_pdf::graphics::Color;
//!
//! let table = AdvancedTableBuilder::new()
//!     .add_column("Name", 150.0)
//!     .add_column("Age", 80.0)
//!     .add_column("Department", 200.0)
//!     .header_style(CellStyle::new()
//!         .background_color(Color::rgb(0.2, 0.4, 0.8))
//!         .text_color(Color::white())
//!         .font_size(14.0))
//!     .add_row(vec!["John Doe", "32", "Engineering"])
//!     .add_row(vec!["Jane Smith", "28", "Marketing"])
//!     .zebra_striping(Color::rgb(0.95, 0.95, 0.95))
//!     .build();
//! ```

mod cell_style;
mod error;
mod header_builder;
mod table_builder;
mod table_renderer;

pub use cell_style::{BorderStyle, CellAlignment, CellStyle, Padding};
pub use error::TableError;
pub use header_builder::{HeaderBuilder, HeaderCell};
pub use table_builder::{AdvancedTable, AdvancedTableBuilder, Column};
pub use table_renderer::TableRenderer;

use crate::error::PdfError;
use crate::page::Page;

/// Extension trait to add advanced table capabilities to PDF pages
pub trait AdvancedTableExt {
    /// Add an advanced table to the page at the specified position
    fn add_advanced_table(
        &mut self,
        table: &AdvancedTable,
        x: f64,
        y: f64,
    ) -> Result<f64, PdfError>;

    /// Add an advanced table with automatic positioning (below last content)
    fn add_advanced_table_auto(&mut self, table: &AdvancedTable) -> Result<f64, PdfError>;
}

impl AdvancedTableExt for Page {
    fn add_advanced_table(
        &mut self,
        table: &AdvancedTable,
        x: f64,
        y: f64,
    ) -> Result<f64, PdfError> {
        let renderer = TableRenderer::new();
        renderer.render_table(self, table, x, y)
    }

    fn add_advanced_table_auto(&mut self, table: &AdvancedTable) -> Result<f64, PdfError> {
        // Position table with default positioning (top of page with margin)
        let y = 750.0; // Default Y position near top of page
        self.add_advanced_table(table, 50.0, y)
    }
}
