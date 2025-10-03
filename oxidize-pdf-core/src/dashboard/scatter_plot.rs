//! ScatterPlot Visualization Component
//!
//! This module implements scatter plots for showing correlations and distributions
//! in two-dimensional data.

use super::{
    component::ComponentConfig, ComponentPosition, ComponentSpan, DashboardComponent,
    DashboardTheme,
};
use crate::error::PdfError;
use crate::graphics::Color;
use crate::page::Page;

/// ScatterPlot visualization component
#[derive(Debug, Clone)]
pub struct ScatterPlot {
    /// Component configuration
    config: ComponentConfig,
    /// Scatter plot data
    data: Vec<ScatterPoint>,
    /// Configuration options
    options: ScatterPlotOptions,
}

impl ScatterPlot {
    /// Create a new scatter plot
    pub fn new(data: Vec<ScatterPoint>) -> Self {
        Self {
            config: ComponentConfig::new(ComponentSpan::new(6)), // Half width by default
            data,
            options: ScatterPlotOptions::default(),
        }
    }

    /// Set scatter plot options
    pub fn with_options(mut self, options: ScatterPlotOptions) -> Self {
        self.options = options;
        self
    }

    /// Get data bounds (min/max for x and y)
    fn get_bounds(&self) -> (f64, f64, f64, f64) {
        if self.data.is_empty() {
            return (0.0, 100.0, 0.0, 100.0);
        }

        let mut min_x = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_y = f64::NEG_INFINITY;

        for point in &self.data {
            min_x = min_x.min(point.x);
            max_x = max_x.max(point.x);
            min_y = min_y.min(point.y);
            max_y = max_y.max(point.y);
        }

        // Add 10% padding
        let x_range = max_x - min_x;
        let y_range = max_y - min_y;
        let x_padding = x_range * 0.1;
        let y_padding = y_range * 0.1;

        (
            min_x - x_padding,
            max_x + x_padding,
            min_y - y_padding,
            max_y + y_padding,
        )
    }

    /// Map data coordinate to plot coordinate
    fn map_to_plot(&self, value: f64, min: f64, max: f64, plot_min: f64, plot_max: f64) -> f64 {
        if max == min {
            return (plot_min + plot_max) / 2.0;
        }
        plot_min + (value - min) / (max - min) * (plot_max - plot_min)
    }
}

impl DashboardComponent for ScatterPlot {
    fn render(
        &self,
        page: &mut Page,
        position: ComponentPosition,
        theme: &DashboardTheme,
    ) -> Result<(), PdfError> {
        let title = self.options.title.as_deref().unwrap_or("Scatter Plot");

        // Calculate layout
        let title_height = 30.0;
        let axis_label_space = 40.0;
        let margin = 10.0;

        let plot_x = position.x + axis_label_space + margin;
        let plot_y = position.y + axis_label_space;
        let plot_width = position.width - axis_label_space - 2.0 * margin;
        let plot_height = position.height - title_height - axis_label_space - margin;

        // Render title
        page.text()
            .set_font(crate::Font::HelveticaBold, theme.typography.heading_size)
            .set_fill_color(theme.colors.text_primary)
            .at(position.x, position.y + position.height - 15.0)
            .write(title)?;

        // Get data bounds
        let (min_x, max_x, min_y, max_y) = self.get_bounds();

        // Draw plot background
        page.graphics()
            .set_fill_color(Color::white())
            .rect(plot_x, plot_y, plot_width, plot_height)
            .fill();

        // Draw grid lines
        let grid_color = Color::gray(0.9);
        let num_grid_lines = 5;

        for i in 0..=num_grid_lines {
            let t = i as f64 / num_grid_lines as f64;

            // Vertical grid lines
            let x = plot_x + t * plot_width;
            page.graphics()
                .set_stroke_color(grid_color)
                .set_line_width(0.5)
                .move_to(x, plot_y)
                .line_to(x, plot_y + plot_height)
                .stroke();

            // Horizontal grid lines
            let y = plot_y + t * plot_height;
            page.graphics()
                .set_stroke_color(grid_color)
                .set_line_width(0.5)
                .move_to(plot_x, y)
                .line_to(plot_x + plot_width, y)
                .stroke();
        }

        // Draw axes
        page.graphics()
            .set_stroke_color(Color::black())
            .set_line_width(1.5)
            .move_to(plot_x, plot_y)
            .line_to(plot_x, plot_y + plot_height)
            .stroke();

        page.graphics()
            .set_stroke_color(Color::black())
            .set_line_width(1.5)
            .move_to(plot_x, plot_y)
            .line_to(plot_x + plot_width, plot_y)
            .stroke();

        // Draw axis labels
        if let Some(ref x_label) = self.options.x_label {
            page.text()
                .set_font(crate::Font::Helvetica, 10.0)
                .set_fill_color(theme.colors.text_secondary)
                .at(plot_x + plot_width / 2.0 - 20.0, plot_y - 25.0)
                .write(x_label)?;
        }

        if let Some(ref y_label) = self.options.y_label {
            page.text()
                .set_font(crate::Font::Helvetica, 10.0)
                .set_fill_color(theme.colors.text_secondary)
                .at(position.x + 5.0, plot_y + plot_height / 2.0)
                .write(y_label)?;
        }

        // Draw axis tick labels
        for i in 0..=num_grid_lines {
            let t = i as f64 / num_grid_lines as f64;

            // X-axis labels
            let x_value = min_x + t * (max_x - min_x);
            let x_pos = plot_x + t * plot_width;
            page.text()
                .set_font(crate::Font::Helvetica, 8.0)
                .set_fill_color(theme.colors.text_secondary)
                .at(x_pos - 10.0, plot_y - 15.0)
                .write(&format!("{:.1}", x_value))?;

            // Y-axis labels
            let y_value = min_y + t * (max_y - min_y);
            let y_pos = plot_y + t * plot_height;
            page.text()
                .set_font(crate::Font::Helvetica, 8.0)
                .set_fill_color(theme.colors.text_secondary)
                .at(plot_x - 35.0, y_pos - 3.0)
                .write(&format!("{:.1}", y_value))?;
        }

        // Draw data points
        let default_color = Color::hex("#007bff");
        let default_size = 3.0;

        for point in &self.data {
            let px = self.map_to_plot(point.x, min_x, max_x, plot_x, plot_x + plot_width);
            let py = self.map_to_plot(point.y, min_y, max_y, plot_y, plot_y + plot_height);
            let size = point.size.unwrap_or(default_size);
            let color = point.color.unwrap_or(default_color);

            // Draw point as filled circle
            page.graphics()
                .set_fill_color(color)
                .circle(px, py, size)
                .fill();

            // Draw point border for visibility
            page.graphics()
                .set_stroke_color(Color::white())
                .set_line_width(0.5)
                .circle(px, py, size)
                .stroke();
        }

        // Draw plot border
        page.graphics()
            .set_stroke_color(Color::black())
            .set_line_width(1.0)
            .rect(plot_x, plot_y, plot_width, plot_height)
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
        300.0
    }
    fn component_type(&self) -> &'static str {
        "ScatterPlot"
    }
    fn complexity_score(&self) -> u8 {
        60
    }
}

/// Scatter plot data point
#[derive(Debug, Clone)]
pub struct ScatterPoint {
    pub x: f64,
    pub y: f64,
    pub size: Option<f64>,
    pub color: Option<Color>,
    pub label: Option<String>,
}

/// Scatter plot options
#[derive(Debug, Clone)]
pub struct ScatterPlotOptions {
    pub title: Option<String>,
    pub x_label: Option<String>,
    pub y_label: Option<String>,
    pub show_trend_line: bool,
}

impl Default for ScatterPlotOptions {
    fn default() -> Self {
        Self {
            title: None,
            x_label: None,
            y_label: None,
            show_trend_line: false,
        }
    }
}

/// Builder for ScatterPlot
pub struct ScatterPlotBuilder;

impl ScatterPlotBuilder {
    pub fn new() -> Self {
        Self
    }
    pub fn build(self) -> ScatterPlot {
        ScatterPlot::new(vec![])
    }
}
