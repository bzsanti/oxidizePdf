//! HeatMap Visualization Component
//!
//! This module implements heat maps for dashboard visualizations, displaying
//! data intensity through color gradients in a matrix format.

use super::{
    component::ComponentConfig, ComponentPosition, ComponentSpan, DashboardComponent,
    DashboardTheme,
};
use crate::error::PdfError;
use crate::graphics::Color;
use crate::page::Page;

/// HeatMap visualization component
#[derive(Debug, Clone)]
pub struct HeatMap {
    /// Component configuration
    config: ComponentConfig,
    /// Heat map data
    data: HeatMapData,
    /// Configuration options
    options: HeatMapOptions,
    /// Color scale for the heat map
    color_scale: ColorScale,
}

impl HeatMap {
    /// Create a new heat map
    pub fn new(data: HeatMapData) -> Self {
        Self {
            config: ComponentConfig::new(ComponentSpan::new(6)), // Half width by default
            data,
            options: HeatMapOptions::default(),
            color_scale: ColorScale::default(),
        }
    }

    /// Set heat map options
    pub fn with_options(mut self, options: HeatMapOptions) -> Self {
        self.options = options;
        self
    }

    /// Set color scale
    pub fn with_color_scale(mut self, color_scale: ColorScale) -> Self {
        self.color_scale = color_scale;
        self
    }

    /// Get min/max values from the data
    fn get_value_range(&self) -> (f64, f64) {
        let min_val = self.color_scale.min_value.unwrap_or_else(|| {
            self.data.values.iter()
                .flat_map(|row| row.iter())
                .copied()
                .fold(f64::INFINITY, f64::min)
        });

        let max_val = self.color_scale.max_value.unwrap_or_else(|| {
            self.data.values.iter()
                .flat_map(|row| row.iter())
                .copied()
                .fold(f64::NEG_INFINITY, f64::max)
        });

        (min_val, max_val)
    }

    /// Interpolate color based on value
    fn interpolate_color(&self, value: f64, min_val: f64, max_val: f64) -> Color {
        if max_val == min_val {
            return self.color_scale.colors[0];
        }

        let normalized = ((value - min_val) / (max_val - min_val)).clamp(0.0, 1.0);

        if self.color_scale.colors.len() == 1 {
            return self.color_scale.colors[0];
        }

        // Interpolate between colors
        let segment_count = self.color_scale.colors.len() - 1;
        let segment = (normalized * segment_count as f64).floor() as usize;
        let segment = segment.min(segment_count - 1);

        let t = (normalized * segment_count as f64) - segment as f64;

        let c1 = &self.color_scale.colors[segment];
        let c2 = &self.color_scale.colors[segment + 1];

        // Extract RGB components from both colors
        let (r1, g1, b1) = match c1 {
            Color::Rgb(r, g, b) => (*r, *g, *b),
            Color::Gray(v) => (*v, *v, *v),
            Color::Cmyk(c, m, y, k) => {
                // Simple CMYK to RGB conversion
                let r = (1.0 - c) * (1.0 - k);
                let g = (1.0 - m) * (1.0 - k);
                let b = (1.0 - y) * (1.0 - k);
                (r, g, b)
            }
        };

        let (r2, g2, b2) = match c2 {
            Color::Rgb(r, g, b) => (*r, *g, *b),
            Color::Gray(v) => (*v, *v, *v),
            Color::Cmyk(c, m, y, k) => {
                let r = (1.0 - c) * (1.0 - k);
                let g = (1.0 - m) * (1.0 - k);
                let b = (1.0 - y) * (1.0 - k);
                (r, g, b)
            }
        };

        Color::rgb(
            r1 + (r2 - r1) * t,
            g1 + (g2 - g1) * t,
            b1 + (b2 - b1) * t,
        )
    }

    /// Check if a color is dark (for text contrast)
    fn is_dark_color(&self, color: &Color) -> bool {
        // Using relative luminance formula
        let (r, g, b) = match color {
            Color::Rgb(r, g, b) => (*r, *g, *b),
            Color::Gray(v) => (*v, *v, *v),
            Color::Cmyk(c, m, y, k) => {
                let r = (1.0 - c) * (1.0 - k);
                let g = (1.0 - m) * (1.0 - k);
                let b = (1.0 - y) * (1.0 - k);
                (r, g, b)
            }
        };
        let luminance = 0.299 * r + 0.587 * g + 0.114 * b;
        luminance < 0.5
    }

    /// Render the color legend
    fn render_legend(
        &self,
        page: &mut Page,
        _position: ComponentPosition,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        min_val: f64,
        max_val: f64,
        theme: &DashboardTheme,
    ) -> Result<(), PdfError> {
        let steps = 20;
        let step_height = height / steps as f64;

        // Draw gradient
        for i in 0..steps {
            let value = min_val + (max_val - min_val) * (i as f64 / steps as f64);
            let color = self.interpolate_color(value, min_val, max_val);
            let step_y = y + (steps - 1 - i) as f64 * step_height;

            page.graphics()
                .set_fill_color(color)
                .rect(x, step_y, width, step_height)
                .fill();
        }

        // Draw border
        page.graphics()
            .set_stroke_color(Color::gray(0.5))
            .set_line_width(1.0)
            .rect(x, y, width, height)
            .stroke();

        // Draw min/max labels
        page.text()
            .set_font(crate::Font::Helvetica, 8.0)
            .set_fill_color(theme.colors.text_secondary)
            .at(x + width + 5.0, y - 5.0)
            .write(&format!("{:.1}", max_val))?;

        page.text()
            .set_font(crate::Font::Helvetica, 8.0)
            .set_fill_color(theme.colors.text_secondary)
            .at(x + width + 5.0, y + height - 10.0)
            .write(&format!("{:.1}", min_val))?;

        Ok(())
    }
}

impl DashboardComponent for HeatMap {
    fn render(
        &self,
        page: &mut Page,
        position: ComponentPosition,
        theme: &DashboardTheme,
    ) -> Result<(), PdfError> {
        let title = self.options.title.as_deref().unwrap_or("HeatMap");

        // Calculate dimensions
        let title_height = 30.0;
        let legend_width = if self.options.show_legend { 60.0 } else { 0.0 };
        let label_width = 80.0;
        let label_height = 30.0;

        let chart_x = position.x + label_width;
        let chart_y = position.y;
        let chart_width = position.width - label_width - legend_width;
        let chart_height = position.height - title_height - label_height;

        // Render title
        page.text()
            .set_font(crate::Font::HelveticaBold, theme.typography.heading_size)
            .set_fill_color(theme.colors.text_primary)
            .at(position.x, position.y + position.height - 15.0)
            .write(title)?;

        // Calculate cell dimensions
        let rows = self.data.values.len();
        let cols = if rows > 0 { self.data.values[0].len() } else { 0 };

        if rows == 0 || cols == 0 {
            return Ok(());
        }

        let cell_width = chart_width / cols as f64;
        let cell_height = chart_height / rows as f64;

        // Find min/max values for color scaling
        let (min_val, max_val) = self.get_value_range();

        // Render cells
        for (row_idx, row) in self.data.values.iter().enumerate() {
            for (col_idx, &value) in row.iter().enumerate() {
                let x = chart_x + col_idx as f64 * cell_width;
                let y = chart_y + title_height + (rows - 1 - row_idx) as f64 * cell_height;

                // Get color for this value
                let color = self.interpolate_color(value, min_val, max_val);

                // Draw cell
                page.graphics()
                    .set_fill_color(color)
                    .rect(
                        x + self.options.cell_padding,
                        y + self.options.cell_padding,
                        cell_width - 2.0 * self.options.cell_padding,
                        cell_height - 2.0 * self.options.cell_padding
                    )
                    .fill();

                // Draw cell border
                page.graphics()
                    .set_stroke_color(Color::gray(0.8))
                    .set_line_width(0.5)
                    .rect(
                        x + self.options.cell_padding,
                        y + self.options.cell_padding,
                        cell_width - 2.0 * self.options.cell_padding,
                        cell_height - 2.0 * self.options.cell_padding
                    )
                    .stroke();

                // Optionally show values
                if self.options.show_values && cell_width > 40.0 && cell_height > 20.0 {
                    let text_color = if self.is_dark_color(&color) {
                        Color::white()
                    } else {
                        Color::black()
                    };

                    page.text()
                        .set_font(crate::Font::Helvetica, 8.0)
                        .set_fill_color(text_color)
                        .at(x + cell_width / 2.0 - 10.0, y + cell_height / 2.0 - 3.0)
                        .write(&format!("{:.1}", value))?;
                }
            }
        }

        // Render row labels
        for (idx, label) in self.data.row_labels.iter().enumerate() {
            let y = chart_y + title_height + (rows - 1 - idx) as f64 * cell_height;
            page.text()
                .set_font(crate::Font::Helvetica, 9.0)
                .set_fill_color(theme.colors.text_secondary)
                .at(position.x + 5.0, y + cell_height / 2.0 - 3.0)
                .write(label)?;
        }

        // Render column labels
        for (idx, label) in self.data.column_labels.iter().enumerate() {
            let x = chart_x + idx as f64 * cell_width;

            // Rotate text for better fit
            page.text()
                .set_font(crate::Font::Helvetica, 9.0)
                .set_fill_color(theme.colors.text_secondary)
                .at(x + cell_width / 2.0 - 5.0, chart_y + 10.0)
                .write(label)?;
        }

        // Render legend
        if self.options.show_legend {
            self.render_legend(page, position, chart_x + chart_width + 10.0, chart_y + title_height, legend_width - 20.0, chart_height, min_val, max_val, theme)?;
        }

        Ok(())
    }

    fn get_span(&self) -> ComponentSpan {
        self.config.span
    }
    fn set_span(&mut self, span: ComponentSpan) {
        self.config.span = span;
    }
    fn preferred_height(&self, _available_width: f64) -> f64 {
        300.0
    }
    fn component_type(&self) -> &'static str {
        "HeatMap"
    }
    fn complexity_score(&self) -> u8 {
        75
    }
}

/// HeatMap data structure
#[derive(Debug, Clone)]
pub struct HeatMapData {
    pub values: Vec<Vec<f64>>,
    pub row_labels: Vec<String>,
    pub column_labels: Vec<String>,
}

/// HeatMap configuration options
#[derive(Debug, Clone)]
pub struct HeatMapOptions {
    pub title: Option<String>,
    pub show_legend: bool,
    pub show_values: bool,
    pub cell_padding: f64,
}

impl Default for HeatMapOptions {
    fn default() -> Self {
        Self {
            title: None,
            show_legend: true,
            show_values: false,
            cell_padding: 2.0,
        }
    }
}

/// Color scale for heat maps
#[derive(Debug, Clone)]
pub struct ColorScale {
    pub colors: Vec<Color>,
    pub min_value: Option<f64>,
    pub max_value: Option<f64>,
}

impl Default for ColorScale {
    fn default() -> Self {
        Self {
            colors: vec![
                Color::hex("#ffffff"), // White for minimum
                Color::hex("#ff0000"), // Red for maximum
            ],
            min_value: None,
            max_value: None,
        }
    }
}

/// Builder for HeatMap
pub struct HeatMapBuilder;

impl HeatMapBuilder {
    pub fn new() -> Self {
        Self
    }
    pub fn build(self) -> HeatMap {
        HeatMap::new(HeatMapData {
            values: vec![],
            row_labels: vec![],
            column_labels: vec![],
        })
    }
}
