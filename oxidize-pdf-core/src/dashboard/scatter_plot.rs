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

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_scatter_data() -> Vec<ScatterPoint> {
        vec![
            ScatterPoint {
                x: 1.0,
                y: 2.0,
                size: None,
                color: None,
                label: Some("Point A".to_string()),
            },
            ScatterPoint {
                x: 3.0,
                y: 4.0,
                size: Some(5.0),
                color: Some(Color::rgb(1.0, 0.0, 0.0)),
                label: None,
            },
            ScatterPoint {
                x: 5.0,
                y: 6.0,
                size: None,
                color: None,
                label: None,
            },
        ]
    }

    #[test]
    fn test_scatter_plot_new() {
        let data = sample_scatter_data();
        let plot = ScatterPlot::new(data.clone());

        assert_eq!(plot.data.len(), 3);
        assert_eq!(plot.data[0].x, 1.0);
        assert_eq!(plot.data[0].y, 2.0);
    }

    #[test]
    fn test_scatter_plot_with_options() {
        let data = sample_scatter_data();
        let options = ScatterPlotOptions {
            title: Some("Test Plot".to_string()),
            x_label: Some("X Axis".to_string()),
            y_label: Some("Y Axis".to_string()),
            show_trend_line: true,
        };

        let plot = ScatterPlot::new(data).with_options(options);

        assert_eq!(plot.options.title, Some("Test Plot".to_string()));
        assert_eq!(plot.options.x_label, Some("X Axis".to_string()));
        assert_eq!(plot.options.y_label, Some("Y Axis".to_string()));
        assert!(plot.options.show_trend_line);
    }

    #[test]
    fn test_scatter_plot_options_default() {
        let options = ScatterPlotOptions::default();

        assert!(options.title.is_none());
        assert!(options.x_label.is_none());
        assert!(options.y_label.is_none());
        assert!(!options.show_trend_line);
    }

    #[test]
    fn test_scatter_plot_builder() {
        let builder = ScatterPlotBuilder::new();
        let plot = builder.build();

        assert!(plot.data.is_empty());
    }

    #[test]
    fn test_scatter_point_creation() {
        let point = ScatterPoint {
            x: 10.0,
            y: 20.0,
            size: Some(4.0),
            color: Some(Color::rgb(0.0, 1.0, 0.0)),
            label: Some("Test".to_string()),
        };

        assert_eq!(point.x, 10.0);
        assert_eq!(point.y, 20.0);
        assert_eq!(point.size, Some(4.0));
        assert!(point.color.is_some());
        assert_eq!(point.label, Some("Test".to_string()));
    }

    #[test]
    fn test_get_bounds_with_data() {
        let data = sample_scatter_data();
        let plot = ScatterPlot::new(data);

        let (min_x, max_x, min_y, max_y) = plot.get_bounds();

        // Original bounds are (1,5) for x and (2,6) for y
        // With 10% padding: x_range=4, y_range=4, padding=0.4
        assert!(min_x < 1.0);
        assert!(max_x > 5.0);
        assert!(min_y < 2.0);
        assert!(max_y > 6.0);
    }

    #[test]
    fn test_get_bounds_empty_data() {
        let plot = ScatterPlot::new(vec![]);

        let (min_x, max_x, min_y, max_y) = plot.get_bounds();

        assert_eq!(min_x, 0.0);
        assert_eq!(max_x, 100.0);
        assert_eq!(min_y, 0.0);
        assert_eq!(max_y, 100.0);
    }

    #[test]
    fn test_get_bounds_single_point() {
        let data = vec![ScatterPoint {
            x: 5.0,
            y: 5.0,
            size: None,
            color: None,
            label: None,
        }];
        let plot = ScatterPlot::new(data);

        let (min_x, max_x, min_y, max_y) = plot.get_bounds();

        // Single point has range 0, so padding is 0
        assert_eq!(min_x, 5.0);
        assert_eq!(max_x, 5.0);
        assert_eq!(min_y, 5.0);
        assert_eq!(max_y, 5.0);
    }

    #[test]
    fn test_map_to_plot_normal() {
        let plot = ScatterPlot::new(vec![]);

        // Map value 50 from range [0,100] to plot range [0,200]
        let result = plot.map_to_plot(50.0, 0.0, 100.0, 0.0, 200.0);
        assert_eq!(result, 100.0);

        // Map value 0 from range [0,100] to plot range [100,200]
        let result = plot.map_to_plot(0.0, 0.0, 100.0, 100.0, 200.0);
        assert_eq!(result, 100.0);

        // Map value 100 from range [0,100] to plot range [100,200]
        let result = plot.map_to_plot(100.0, 0.0, 100.0, 100.0, 200.0);
        assert_eq!(result, 200.0);
    }

    #[test]
    fn test_map_to_plot_same_min_max() {
        let plot = ScatterPlot::new(vec![]);

        // When min == max, should return midpoint
        let result = plot.map_to_plot(5.0, 5.0, 5.0, 0.0, 100.0);
        assert_eq!(result, 50.0);
    }

    #[test]
    fn test_component_span() {
        let data = sample_scatter_data();
        let mut plot = ScatterPlot::new(data);

        // Default span
        let span = plot.get_span();
        assert_eq!(span.columns, 6);

        // Set new span
        plot.set_span(ComponentSpan::new(12));
        assert_eq!(plot.get_span().columns, 12);
    }

    #[test]
    fn test_component_type() {
        let plot = ScatterPlot::new(vec![]);

        assert_eq!(plot.component_type(), "ScatterPlot");
    }

    #[test]
    fn test_complexity_score() {
        let plot = ScatterPlot::new(vec![]);

        assert_eq!(plot.complexity_score(), 60);
    }

    #[test]
    fn test_preferred_height() {
        let plot = ScatterPlot::new(vec![]);

        assert_eq!(plot.preferred_height(1000.0), 300.0);
    }

    #[test]
    fn test_get_bounds_negative_values() {
        let data = vec![
            ScatterPoint {
                x: -10.0,
                y: -5.0,
                size: None,
                color: None,
                label: None,
            },
            ScatterPoint {
                x: 10.0,
                y: 5.0,
                size: None,
                color: None,
                label: None,
            },
        ];
        let plot = ScatterPlot::new(data);

        let (min_x, max_x, min_y, max_y) = plot.get_bounds();

        // Should include negative values with padding
        assert!(min_x < -10.0);
        assert!(max_x > 10.0);
        assert!(min_y < -5.0);
        assert!(max_y > 5.0);
    }

    #[test]
    fn test_map_to_plot_negative_range() {
        let plot = ScatterPlot::new(vec![]);

        // Map value 0 from range [-100,100] to plot range [0,200]
        let result = plot.map_to_plot(0.0, -100.0, 100.0, 0.0, 200.0);
        assert_eq!(result, 100.0);

        // Map value -100 from range [-100,100] to plot range [0,200]
        let result = plot.map_to_plot(-100.0, -100.0, 100.0, 0.0, 200.0);
        assert_eq!(result, 0.0);
    }
}
