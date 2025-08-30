//! Generic chart builder and data structures

use crate::graphics::Color;
use crate::text::Font;

/// Position of the chart legend
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LegendPosition {
    /// No legend
    None,
    /// Legend on the right side
    Right,
    /// Legend at the bottom
    Bottom,
    /// Legend at the top
    Top,
    /// Legend on the left side
    Left,
}

impl Default for LegendPosition {
    fn default() -> Self {
        LegendPosition::Right
    }
}

/// Chart type enumeration
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ChartType {
    /// Vertical bar chart
    VerticalBar,
    /// Horizontal bar chart
    HorizontalBar,
    /// Pie chart
    Pie,
    /// Line chart
    Line,
    /// Area chart
    Area,
}

/// Data point for charts
#[derive(Debug, Clone)]
pub struct ChartData {
    /// Label for this data point
    pub label: String,
    /// Value for this data point
    pub value: f64,
    /// Custom color for this data point
    pub color: Option<Color>,
    /// Whether this data point should be highlighted
    pub highlighted: bool,
}

impl ChartData {
    /// Create a new chart data point
    pub fn new<S: Into<String>>(label: S, value: f64) -> Self {
        Self {
            label: label.into(),
            value,
            color: None,
            highlighted: false,
        }
    }

    /// Set custom color for this data point
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// Mark this data point as highlighted
    pub fn highlighted(mut self) -> Self {
        self.highlighted = true;
        self
    }
}

/// Generic chart configuration
#[derive(Debug, Clone)]
pub struct Chart {
    /// Chart title
    pub title: String,
    /// Chart type
    pub chart_type: ChartType,
    /// Chart data
    pub data: Vec<ChartData>,
    /// Chart colors (used if data points don't have custom colors)
    pub colors: Vec<Color>,
    /// Title font
    pub title_font: Font,
    /// Title font size
    pub title_font_size: f64,
    /// Label font
    pub label_font: Font,
    /// Label font size
    pub label_font_size: f64,
    /// Legend position
    pub legend_position: LegendPosition,
    /// Background color
    pub background_color: Option<Color>,
    /// Whether to show values on chart elements
    pub show_values: bool,
    /// Whether to show grid lines
    pub show_grid: bool,
    /// Grid color
    pub grid_color: Color,
    /// Border color
    pub border_color: Color,
    /// Border width
    pub border_width: f64,
}

impl Chart {
    /// Create a new chart
    pub fn new(chart_type: ChartType) -> Self {
        Self {
            title: String::new(),
            chart_type,
            data: Vec::new(),
            colors: default_colors(),
            title_font: Font::HelveticaBold,
            title_font_size: 14.0,
            label_font: Font::Helvetica,
            label_font_size: 10.0,
            legend_position: LegendPosition::Right,
            background_color: None,
            show_values: true,
            show_grid: true,
            grid_color: Color::rgb(0.9, 0.9, 0.9),
            border_color: Color::black(),
            border_width: 1.0,
        }
    }

    /// Get the maximum value in the dataset
    pub fn max_value(&self) -> f64 {
        self.data.iter().map(|d| d.value).fold(0.0, f64::max)
    }

    /// Get the total value of all data points (useful for pie charts)
    pub fn total_value(&self) -> f64 {
        self.data.iter().map(|d| d.value).sum()
    }

    /// Get color for a data point at the given index
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
}

/// Builder for creating charts with fluent API
pub struct ChartBuilder {
    chart: Chart,
}

impl ChartBuilder {
    /// Create a new chart builder
    pub fn new(chart_type: ChartType) -> Self {
        Self {
            chart: Chart::new(chart_type),
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

    /// Set chart colors
    pub fn colors(mut self, colors: Vec<Color>) -> Self {
        self.chart.colors = colors;
        self
    }

    /// Set title font
    pub fn title_font(mut self, font: Font, size: f64) -> Self {
        self.chart.title_font = font;
        self.chart.title_font_size = size;
        self
    }

    /// Set label font
    pub fn label_font(mut self, font: Font, size: f64) -> Self {
        self.chart.label_font = font;
        self.chart.label_font_size = size;
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

    /// Show or hide values on chart elements
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

    /// Set border style
    pub fn border(mut self, color: Color, width: f64) -> Self {
        self.chart.border_color = color;
        self.chart.border_width = width;
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

    /// Create a financial chart style
    pub fn financial_style(mut self) -> Self {
        self.chart.colors = vec![
            Color::rgb(0.2, 0.6, 0.2), // Green
            Color::rgb(0.8, 0.2, 0.2), // Red
            Color::rgb(0.2, 0.4, 0.8), // Blue
            Color::rgb(0.9, 0.6, 0.1), // Orange
            Color::rgb(0.6, 0.2, 0.8), // Purple
        ];
        self.chart.show_grid = true;
        self.chart.grid_color = Color::rgb(0.95, 0.95, 0.95);
        self.chart.title_font_size = 16.0;
        self
    }

    /// Create a minimal chart style
    pub fn minimal_style(mut self) -> Self {
        self.chart.show_grid = false;
        self.chart.border_width = 0.0;
        self.chart.background_color = None;
        self
    }

    /// Build the final chart
    pub fn build(self) -> Chart {
        self.chart
    }
}

/// Default color palette for charts
fn default_colors() -> Vec<Color> {
    vec![
        Color::rgb(0.26, 0.45, 0.76), // Blue
        Color::rgb(0.85, 0.37, 0.0),  // Orange
        Color::rgb(0.18, 0.55, 0.34), // Green
        Color::rgb(0.84, 0.15, 0.16), // Red
        Color::rgb(0.58, 0.4, 0.74),  // Purple
        Color::rgb(0.55, 0.34, 0.29), // Brown
        Color::rgb(0.89, 0.47, 0.76), // Pink
        Color::rgb(0.5, 0.5, 0.5),    // Gray
        Color::rgb(0.74, 0.74, 0.13), // Olive
        Color::rgb(0.09, 0.75, 0.81), // Cyan
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chart_data_creation() {
        let data = ChartData::new("Test", 42.0);
        assert_eq!(data.label, "Test");
        assert_eq!(data.value, 42.0);
        assert_eq!(data.color, None);
        assert_eq!(data.highlighted, false);
    }

    #[test]
    fn test_chart_data_with_color() {
        let data = ChartData::new("Test", 42.0)
            .color(Color::red())
            .highlighted();

        assert_eq!(data.color, Some(Color::red()));
        assert_eq!(data.highlighted, true);
    }

    #[test]
    fn test_chart_builder() {
        let chart = ChartBuilder::new(ChartType::VerticalBar)
            .title("Test Chart")
            .simple_data(vec![10.0, 20.0, 30.0])
            .build();

        assert_eq!(chart.title, "Test Chart");
        assert_eq!(chart.chart_type, ChartType::VerticalBar);
        assert_eq!(chart.data.len(), 3);
        assert_eq!(chart.max_value(), 30.0);
        assert_eq!(chart.total_value(), 60.0);
    }

    #[test]
    fn test_color_for_index() {
        let chart = ChartBuilder::new(ChartType::Pie)
            .data(vec![
                ChartData::new("A", 10.0).color(Color::red()),
                ChartData::new("B", 20.0), // No custom color
            ])
            .build();

        assert_eq!(chart.color_for_index(0), Color::red());
        assert_eq!(chart.color_for_index(1), chart.colors[1]); // Default color
    }
}
