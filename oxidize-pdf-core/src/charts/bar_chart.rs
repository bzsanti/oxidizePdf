//! Bar chart implementation with horizontal and vertical orientations

use super::chart_builder::{ChartData, LegendPosition};
use crate::graphics::Color;
use crate::text::Font;

/// Bar chart orientation
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum BarOrientation {
    /// Vertical bars (default)
    #[default]
    Vertical,
    /// Horizontal bars
    Horizontal,
}

/// Bar chart configuration
#[derive(Debug, Clone)]
pub struct BarChart {
    /// Chart title
    pub title: String,
    /// Chart data
    pub data: Vec<ChartData>,
    /// Bar orientation
    pub orientation: BarOrientation,
    /// Chart colors
    pub colors: Vec<Color>,
    /// Title font and size
    pub title_font: Font,
    pub title_font_size: f64,
    /// Label font and size
    pub label_font: Font,
    pub label_font_size: f64,
    /// Value font and size (for showing values on bars)
    pub value_font: Font,
    pub value_font_size: f64,
    /// Legend position
    pub legend_position: LegendPosition,
    /// Background color
    pub background_color: Option<Color>,
    /// Show values on bars
    pub show_values: bool,
    /// Show grid lines
    pub show_grid: bool,
    /// Grid color
    pub grid_color: Color,
    /// Bar spacing (as a fraction of bar width)
    pub bar_spacing: f64,
    /// Bar border color
    pub bar_border_color: Option<Color>,
    /// Bar border width
    pub bar_border_width: f64,
    /// Minimum bar width in points
    pub min_bar_width: f64,
    /// Maximum bar width in points
    pub max_bar_width: Option<f64>,
}

impl BarChart {
    /// Create a new bar chart
    pub fn new() -> Self {
        Self {
            title: String::new(),
            data: Vec::new(),
            orientation: BarOrientation::Vertical,
            colors: default_bar_colors(),
            title_font: Font::HelveticaBold,
            title_font_size: 16.0,
            label_font: Font::Helvetica,
            label_font_size: 12.0,
            value_font: Font::Helvetica,
            value_font_size: 10.0,
            legend_position: LegendPosition::None,
            background_color: None,
            show_values: true,
            show_grid: true,
            grid_color: Color::rgb(0.9, 0.9, 0.9),
            bar_spacing: 0.2, // 20% spacing between bars
            bar_border_color: None,
            bar_border_width: 1.0,
            min_bar_width: 20.0,
            max_bar_width: None,
        }
    }

    /// Get the maximum value in the dataset
    pub fn max_value(&self) -> f64 {
        self.data.iter().map(|d| d.value).fold(0.0, f64::max)
    }

    /// Get the minimum value in the dataset
    pub fn min_value(&self) -> f64 {
        self.data
            .iter()
            .map(|d| d.value)
            .fold(f64::INFINITY, f64::min)
            .min(0.0) // Always include 0 in the range
    }

    /// Get color for a bar at the given index
    pub fn color_for_index(&self, index: usize) -> Color {
        if let Some(data_point) = self.data.get(index) {
            if let Some(color) = data_point.color {
                return color;
            }
        }

        self.colors
            .get(index % self.colors.len())
            .copied()
            .unwrap_or(Color::rgb(0.5, 0.5, 0.5))
    }

    /// Calculate bar width for the given chart dimensions
    pub fn calculate_bar_width(&self, available_width: f64) -> f64 {
        if self.data.is_empty() {
            return self.min_bar_width;
        }

        let spacing_factor = 1.0 + self.bar_spacing;
        let total_spacing = spacing_factor * self.data.len() as f64 - self.bar_spacing;
        let calculated_width = available_width / total_spacing;

        let width = calculated_width.max(self.min_bar_width);

        if let Some(max_width) = self.max_bar_width {
            width.min(max_width)
        } else {
            width
        }
    }

    /// Calculate the value range for scaling
    pub fn value_range(&self) -> (f64, f64) {
        let min = self.min_value();
        let max = self.max_value();

        // Add some padding to the range
        let range = max - min;
        let padding = range * 0.1; // 10% padding

        (min - padding, max + padding)
    }
}

impl Default for BarChart {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for creating bar charts with fluent API
pub struct BarChartBuilder {
    chart: BarChart,
}

impl BarChartBuilder {
    /// Create a new bar chart builder
    pub fn new() -> Self {
        Self {
            chart: BarChart::new(),
        }
    }

    /// Set chart title
    pub fn title<S: Into<String>>(mut self, title: S) -> Self {
        self.chart.title = title.into();
        self
    }

    /// Add a single data point
    pub fn add_data(mut self, data: ChartData) -> Self {
        self.chart.data.push(data);
        self
    }

    /// Set all data at once
    pub fn data(mut self, data: Vec<ChartData>) -> Self {
        self.chart.data = data;
        self
    }

    /// Set bar orientation
    pub fn orientation(mut self, orientation: BarOrientation) -> Self {
        self.chart.orientation = orientation;
        self
    }

    /// Set chart colors
    pub fn colors(mut self, colors: Vec<Color>) -> Self {
        self.chart.colors = colors;
        self
    }

    /// Set title font and size
    pub fn title_font(mut self, font: Font, size: f64) -> Self {
        self.chart.title_font = font;
        self.chart.title_font_size = size;
        self
    }

    /// Set label font and size
    pub fn label_font(mut self, font: Font, size: f64) -> Self {
        self.chart.label_font = font;
        self.chart.label_font_size = size;
        self
    }

    /// Set value font and size
    pub fn value_font(mut self, font: Font, size: f64) -> Self {
        self.chart.value_font = font;
        self.chart.value_font_size = size;
        self
    }

    /// Set legend position
    pub fn legend_position(mut self, position: LegendPosition) -> Self {
        self.chart.legend_position = position;
        self
    }

    /// Set background color
    pub fn background_color(mut self, color: Color) -> Self {
        self.chart.background_color = Some(color);
        self
    }

    /// Show or hide values on bars
    pub fn show_values(mut self, show: bool) -> Self {
        self.chart.show_values = show;
        self
    }

    /// Show or hide grid lines
    pub fn show_grid(mut self, show: bool) -> Self {
        self.chart.show_grid = show;
        self
    }

    /// Set grid color
    pub fn grid_color(mut self, color: Color) -> Self {
        self.chart.grid_color = color;
        self
    }

    /// Set bar spacing
    pub fn bar_spacing(mut self, spacing: f64) -> Self {
        self.chart.bar_spacing = spacing.max(0.0);
        self
    }

    /// Set bar border
    pub fn bar_border(mut self, color: Color, width: f64) -> Self {
        self.chart.bar_border_color = Some(color);
        self.chart.bar_border_width = width;
        self
    }

    /// Set bar width constraints
    pub fn bar_width_range(mut self, min_width: f64, max_width: Option<f64>) -> Self {
        self.chart.min_bar_width = min_width;
        self.chart.max_bar_width = max_width;
        self
    }

    /// Add data from simple values with automatic labels
    pub fn simple_data(mut self, values: Vec<f64>) -> Self {
        for (i, value) in values.into_iter().enumerate() {
            self.chart
                .data
                .push(ChartData::new(format!("Item {}", i + 1), value));
        }
        self
    }

    /// Add data from label-value pairs
    pub fn labeled_data(mut self, data: Vec<(&str, f64)>) -> Self {
        for (label, value) in data {
            self.chart.data.push(ChartData::new(label, value));
        }
        self
    }

    /// Create a financial bar chart style
    pub fn financial_style(mut self) -> Self {
        self.chart.colors = vec![
            Color::rgb(0.2, 0.6, 0.2), // Green for positive
            Color::rgb(0.8, 0.2, 0.2), // Red for negative
            Color::rgb(0.2, 0.4, 0.8), // Blue
            Color::rgb(0.9, 0.6, 0.1), // Orange
        ];
        self.chart.show_grid = true;
        self.chart.grid_color = Color::rgb(0.95, 0.95, 0.95);
        self.chart.bar_border_color = Some(Color::rgb(0.8, 0.8, 0.8));
        self.chart.bar_border_width = 0.5;
        self
    }

    /// Create a minimal bar chart style
    pub fn minimal_style(mut self) -> Self {
        self.chart.show_grid = false;
        self.chart.show_values = false;
        self.chart.background_color = None;
        self.chart.bar_border_color = None;
        self
    }

    /// Create a progress bar style (single color, horizontal)
    pub fn progress_style(mut self, color: Color) -> Self {
        self.chart.orientation = BarOrientation::Horizontal;
        self.chart.colors = vec![color];
        self.chart.show_grid = false;
        self.chart.show_values = true;
        self.chart.legend_position = LegendPosition::None;
        self.chart.bar_spacing = 0.0;
        self
    }

    /// Build the final bar chart
    pub fn build(self) -> BarChart {
        self.chart
    }
}

impl Default for BarChartBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Default color palette for bar charts
fn default_bar_colors() -> Vec<Color> {
    vec![
        Color::rgb(0.31, 0.78, 0.47), // Green
        Color::rgb(0.26, 0.45, 0.76), // Blue
        Color::rgb(0.85, 0.37, 0.0),  // Orange
        Color::rgb(0.84, 0.15, 0.16), // Red
        Color::rgb(0.58, 0.4, 0.74),  // Purple
        Color::rgb(0.55, 0.34, 0.29), // Brown
        Color::rgb(0.89, 0.47, 0.76), // Pink
        Color::rgb(0.5, 0.5, 0.5),    // Gray
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bar_chart_creation() {
        let chart = BarChartBuilder::new()
            .title("Test Chart")
            .simple_data(vec![10.0, 20.0, 30.0])
            .build();

        assert_eq!(chart.title, "Test Chart");
        assert_eq!(chart.data.len(), 3);
        assert_eq!(chart.max_value(), 30.0);
        assert_eq!(chart.min_value(), 0.0); // Always includes 0 for bar charts
    }

    #[test]
    fn test_bar_chart_with_negative_values() {
        let chart = BarChartBuilder::new()
            .simple_data(vec![-10.0, 20.0, -5.0])
            .build();

        assert_eq!(chart.max_value(), 20.0);
        assert_eq!(chart.min_value(), -10.0);

        let (min_range, max_range) = chart.value_range();
        assert!(min_range < -10.0); // Should have padding
        assert!(max_range > 20.0); // Should have padding
    }

    #[test]
    fn test_bar_width_calculation() {
        let chart = BarChartBuilder::new()
            .simple_data(vec![10.0, 20.0, 30.0])
            .bar_spacing(0.2)
            .build();

        let width = chart.calculate_bar_width(400.0);
        assert!(width >= chart.min_bar_width);
    }

    #[test]
    fn test_financial_style() {
        let chart = BarChartBuilder::new().financial_style().build();

        assert_eq!(chart.show_grid, true);
        assert!(chart.bar_border_color.is_some());
    }
}
