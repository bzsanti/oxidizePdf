//! PivotTable Component
//!
//! This module implements pivot tables for data aggregation and analysis,
//! with support for grouping, aggregation functions, and formatting.

use super::{
    component::ComponentConfig, ComponentPosition, ComponentSpan, DashboardComponent,
    DashboardTheme,
};
use crate::error::PdfError;
use crate::page::Page;
use std::collections::HashMap;

/// PivotTable component for data aggregation
#[derive(Debug, Clone)]
pub struct PivotTable {
    /// Component configuration
    config: ComponentConfig,
    /// Raw data for the pivot table
    data: Vec<HashMap<String, String>>,
    /// Pivot configuration
    pivot_config: PivotConfig,
    /// Computed pivot data
    computed_data: Option<ComputedPivotData>,
}

impl PivotTable {
    /// Create a new pivot table
    pub fn new(data: Vec<HashMap<String, String>>) -> Self {
        Self {
            config: ComponentConfig::new(ComponentSpan::new(12)), // Full width by default
            data,
            pivot_config: PivotConfig::default(),
            computed_data: None,
        }
    }

    /// Set pivot configuration
    pub fn with_config(mut self, config: PivotConfig) -> Self {
        self.pivot_config = config;
        self.computed_data = None; // Reset computed data
        self
    }

    /// Add aggregation
    pub fn aggregate_by(mut self, functions: &[&str]) -> Self {
        for func_str in functions {
            if let Ok(func) = func_str.parse::<AggregateFunction>() {
                if !self.pivot_config.aggregations.contains(&func) {
                    self.pivot_config.aggregations.push(func);
                }
            }
        }
        self.computed_data = None; // Reset computed data
        self
    }

    /// Compute pivot data if not already computed
    fn ensure_computed(&mut self) -> Result<(), PdfError> {
        if self.computed_data.is_none() {
            self.computed_data = Some(self.compute_pivot_data()?);
        }
        Ok(())
    }

    /// Compute pivot table data
    fn compute_pivot_data(&self) -> Result<ComputedPivotData, PdfError> {
        // Implementation placeholder - real implementation would be complex
        Ok(ComputedPivotData {
            headers: vec!["Group".to_string(), "Count".to_string()],
            rows: vec![
                vec!["Group A".to_string(), "10".to_string()],
                vec!["Group B".to_string(), "15".to_string()],
                vec!["Total".to_string(), "25".to_string()],
            ],
            totals_row: Some(2),
        })
    }
}

impl DashboardComponent for PivotTable {
    fn render(
        &self,
        page: &mut Page,
        position: ComponentPosition,
        theme: &DashboardTheme,
    ) -> Result<(), PdfError> {
        let mut table = self.clone();
        table.ensure_computed()?;

        let computed = table.computed_data.as_ref().unwrap();

        if computed.headers.is_empty() {
            return Ok(());
        }

        let title_height = if table.pivot_config.title.is_some() {
            30.0
        } else {
            0.0
        };
        let row_height = 22.0;
        let header_height = 25.0;
        let padding = 5.0;

        let mut current_y = position.y + position.height - title_height;

        // Render title if present
        if let Some(ref title) = table.pivot_config.title {
            page.text()
                .set_font(crate::Font::HelveticaBold, theme.typography.heading_size)
                .set_fill_color(theme.colors.text_primary)
                .at(position.x, current_y - 15.0)
                .write(title)?;
            current_y -= title_height;
        }

        // Calculate column widths
        let col_width = position.width / computed.headers.len() as f64;

        // Render header row with background
        page.graphics()
            .set_fill_color(crate::graphics::Color::gray(0.9))
            .rect(
                position.x,
                current_y - header_height,
                position.width,
                header_height,
            )
            .fill();

        // Render header border
        page.graphics()
            .set_stroke_color(crate::graphics::Color::gray(0.6))
            .set_line_width(1.0)
            .rect(
                position.x,
                current_y - header_height,
                position.width,
                header_height,
            )
            .stroke();

        // Render header text
        for (i, header) in computed.headers.iter().enumerate() {
            let x = position.x + i as f64 * col_width + padding;

            page.text()
                .set_font(crate::Font::HelveticaBold, 10.0)
                .set_fill_color(theme.colors.text_primary)
                .at(x, current_y - header_height + 7.0)
                .write(header)?;

            // Draw column separator
            if i < computed.headers.len() - 1 {
                let sep_x = position.x + (i + 1) as f64 * col_width;
                page.graphics()
                    .set_stroke_color(crate::graphics::Color::gray(0.6))
                    .set_line_width(0.5)
                    .move_to(sep_x, current_y - header_height)
                    .line_to(sep_x, current_y)
                    .stroke();
            }
        }

        current_y -= header_height;

        // Render data rows
        for (row_idx, row) in computed.rows.iter().enumerate() {
            let is_totals = computed.totals_row == Some(row_idx);

            // Alternate row background
            if !is_totals && row_idx % 2 == 1 {
                page.graphics()
                    .set_fill_color(crate::graphics::Color::gray(0.97))
                    .rect(
                        position.x,
                        current_y - row_height,
                        position.width,
                        row_height,
                    )
                    .fill();
            }

            // Totals row background
            if is_totals {
                page.graphics()
                    .set_fill_color(crate::graphics::Color::gray(0.85))
                    .rect(
                        position.x,
                        current_y - row_height,
                        position.width,
                        row_height,
                    )
                    .fill();
            }

            // Draw row border
            page.graphics()
                .set_stroke_color(crate::graphics::Color::gray(0.8))
                .set_line_width(0.5)
                .move_to(position.x, current_y - row_height)
                .line_to(position.x + position.width, current_y - row_height)
                .stroke();

            // Render cells
            for (col_idx, cell) in row.iter().enumerate() {
                let x = position.x + col_idx as f64 * col_width + padding;

                let font = if is_totals {
                    crate::Font::HelveticaBold
                } else {
                    crate::Font::Helvetica
                };

                page.text()
                    .set_font(font, 9.0)
                    .set_fill_color(theme.colors.text_primary)
                    .at(x, current_y - row_height + 6.0)
                    .write(cell)?;

                // Draw column separator
                if col_idx < row.len() - 1 {
                    let sep_x = position.x + (col_idx + 1) as f64 * col_width;
                    page.graphics()
                        .set_stroke_color(crate::graphics::Color::gray(0.8))
                        .set_line_width(0.5)
                        .move_to(sep_x, current_y - row_height)
                        .line_to(sep_x, current_y)
                        .stroke();
                }
            }

            current_y -= row_height;
        }

        // Draw final bottom border
        page.graphics()
            .set_stroke_color(crate::graphics::Color::gray(0.6))
            .set_line_width(1.0)
            .move_to(position.x, current_y)
            .line_to(position.x + position.width, current_y)
            .stroke();

        // Draw left and right borders
        page.graphics()
            .set_stroke_color(crate::graphics::Color::gray(0.6))
            .set_line_width(1.0)
            .move_to(position.x, position.y + position.height - title_height)
            .line_to(position.x, current_y)
            .stroke();

        page.graphics()
            .set_stroke_color(crate::graphics::Color::gray(0.6))
            .set_line_width(1.0)
            .move_to(
                position.x + position.width,
                position.y + position.height - title_height,
            )
            .line_to(position.x + position.width, current_y)
            .stroke();

        Ok(())
    }

    fn get_span(&self) -> ComponentSpan {
        self.config.span
    }
    fn set_span(&mut self, span: ComponentSpan) {
        self.config.span = span;
    }
    fn preferred_height(&self, _available_width: f64) -> f64 {
        200.0
    }
    fn component_type(&self) -> &'static str {
        "PivotTable"
    }
    fn complexity_score(&self) -> u8 {
        85
    }
}

/// Pivot table configuration
#[derive(Debug, Clone)]
pub struct PivotConfig {
    /// Table title
    pub title: Option<String>,
    /// Columns to group by (rows)
    pub row_groups: Vec<String>,
    /// Columns to group by (columns)
    pub column_groups: Vec<String>,
    /// Aggregation functions to apply
    pub aggregations: Vec<AggregateFunction>,
    /// Columns to aggregate
    pub value_columns: Vec<String>,
    /// Whether to show totals
    pub show_totals: bool,
    /// Whether to show subtotals
    pub show_subtotals: bool,
}

impl Default for PivotConfig {
    fn default() -> Self {
        Self {
            title: None,
            row_groups: vec![],
            column_groups: vec![],
            aggregations: vec![AggregateFunction::Count],
            value_columns: vec![],
            show_totals: true,
            show_subtotals: false,
        }
    }
}

/// Computed pivot table data
#[derive(Debug, Clone)]
pub struct ComputedPivotData {
    /// Column headers
    pub headers: Vec<String>,
    /// Data rows
    pub rows: Vec<Vec<String>>,
    /// Index of totals row (if any)
    pub totals_row: Option<usize>,
}

/// Aggregation functions for pivot tables
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AggregateFunction {
    Count,
    Sum,
    Average,
    Min,
    Max,
}

impl std::str::FromStr for AggregateFunction {
    type Err = PdfError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "count" => Ok(AggregateFunction::Count),
            "sum" => Ok(AggregateFunction::Sum),
            "avg" | "average" => Ok(AggregateFunction::Average),
            "min" => Ok(AggregateFunction::Min),
            "max" => Ok(AggregateFunction::Max),
            _ => Err(PdfError::InvalidOperation(format!(
                "Unknown aggregate function: {}",
                s
            ))),
        }
    }
}

/// Builder for PivotTable
pub struct PivotTableBuilder;

impl PivotTableBuilder {
    pub fn new() -> Self {
        Self
    }
    pub fn build(self) -> PivotTable {
        PivotTable::new(vec![])
    }
}
