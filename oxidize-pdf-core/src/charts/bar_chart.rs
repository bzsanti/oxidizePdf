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

    // ==================== BarOrientation Tests ====================

    #[test]
    fn test_bar_orientation_default() {
        let orientation: BarOrientation = Default::default();
        assert_eq!(orientation, BarOrientation::Vertical);
    }

    #[test]
    fn test_bar_orientation_variants() {
        assert_eq!(BarOrientation::Vertical, BarOrientation::Vertical);
        assert_eq!(BarOrientation::Horizontal, BarOrientation::Horizontal);
        assert_ne!(BarOrientation::Vertical, BarOrientation::Horizontal);
    }

    #[test]
    fn test_bar_orientation_clone() {
        let orientation = BarOrientation::Horizontal;
        let cloned = orientation;
        assert_eq!(orientation, cloned);
    }

    #[test]
    fn test_bar_orientation_debug() {
        let debug_str = format!("{:?}", BarOrientation::Vertical);
        assert!(debug_str.contains("Vertical"));
    }

    // ==================== BarChart Tests ====================

    #[test]
    fn test_bar_chart_new_defaults() {
        let chart = BarChart::new();
        assert!(chart.title.is_empty());
        assert!(chart.data.is_empty());
        assert_eq!(chart.orientation, BarOrientation::Vertical);
        assert_eq!(chart.title_font, Font::HelveticaBold);
        assert_eq!(chart.title_font_size, 16.0);
        assert_eq!(chart.label_font, Font::Helvetica);
        assert_eq!(chart.label_font_size, 12.0);
        assert_eq!(chart.value_font, Font::Helvetica);
        assert_eq!(chart.value_font_size, 10.0);
        assert_eq!(chart.legend_position, LegendPosition::None);
        assert_eq!(chart.background_color, None);
        assert!(chart.show_values);
        assert!(chart.show_grid);
        assert_eq!(chart.bar_spacing, 0.2);
        assert_eq!(chart.bar_border_color, None);
        assert_eq!(chart.bar_border_width, 1.0);
        assert_eq!(chart.min_bar_width, 20.0);
        assert_eq!(chart.max_bar_width, None);
    }

    #[test]
    fn test_bar_chart_default_trait() {
        let chart: BarChart = Default::default();
        assert!(chart.title.is_empty());
        assert!(chart.data.is_empty());
    }

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
    fn test_bar_chart_max_value_empty() {
        let chart = BarChart::new();
        assert_eq!(chart.max_value(), 0.0);
    }

    #[test]
    fn test_bar_chart_min_value_empty() {
        let chart = BarChart::new();
        // Empty data returns INFINITY.min(0.0) = 0.0
        assert_eq!(chart.min_value(), 0.0);
    }

    #[test]
    fn test_bar_chart_min_value_all_positive() {
        let chart = BarChartBuilder::new()
            .simple_data(vec![5.0, 10.0, 15.0])
            .build();
        // min_value always includes 0 in range
        assert_eq!(chart.min_value(), 0.0);
    }

    #[test]
    fn test_bar_chart_color_for_index() {
        let chart = BarChartBuilder::new()
            .simple_data(vec![10.0, 20.0, 30.0])
            .build();

        // Should return colors from default palette
        let color0 = chart.color_for_index(0);
        let color1 = chart.color_for_index(1);
        assert_ne!(color0, color1);
    }

    #[test]
    fn test_bar_chart_color_for_index_custom() {
        let chart = BarChartBuilder::new()
            .add_data(ChartData::new("A", 10.0).color(Color::red()))
            .add_data(ChartData::new("B", 20.0))
            .build();

        assert_eq!(chart.color_for_index(0), Color::red());
        // Second item uses default colors
        assert_eq!(chart.color_for_index(1), chart.colors[1]);
    }

    #[test]
    fn test_bar_chart_color_for_index_wraps() {
        let chart = BarChart::new();
        // With default colors (8 colors), index 100 wraps around
        let color = chart.color_for_index(100);
        // Should not panic and return a valid color
        assert!(color.r() >= 0.0 && color.r() <= 1.0);
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
    fn test_bar_width_calculation_empty_data() {
        let chart = BarChart::new();
        let width = chart.calculate_bar_width(400.0);
        assert_eq!(width, chart.min_bar_width);
    }

    #[test]
    fn test_bar_width_calculation_respects_min() {
        let chart = BarChartBuilder::new()
            .simple_data(vec![1.0; 100]) // Many bars
            .bar_width_range(30.0, None)
            .build();

        let width = chart.calculate_bar_width(100.0); // Very narrow space
        assert!(width >= 30.0);
    }

    #[test]
    fn test_bar_width_calculation_respects_max() {
        let chart = BarChartBuilder::new()
            .simple_data(vec![1.0, 2.0]) // Few bars
            .bar_width_range(10.0, Some(50.0))
            .build();

        let width = chart.calculate_bar_width(1000.0); // Very wide space
        assert!(width <= 50.0);
    }

    #[test]
    fn test_bar_chart_value_range_empty() {
        let chart = BarChart::new();
        let (min, max) = chart.value_range();
        // With empty data: max=0, min=0, range=0, padding=0
        assert_eq!(min, 0.0);
        assert_eq!(max, 0.0);
    }

    #[test]
    fn test_bar_chart_value_range_single_value() {
        let chart = BarChartBuilder::new().simple_data(vec![100.0]).build();

        let (min, max) = chart.value_range();
        assert!(min < 0.0); // Should have padding
        assert!(max > 100.0); // Should have padding
    }

    // ==================== BarChartBuilder Tests ====================

    #[test]
    fn test_bar_chart_builder_default() {
        let builder: BarChartBuilder = Default::default();
        let chart = builder.build();
        assert!(chart.title.is_empty());
    }

    #[test]
    fn test_bar_chart_builder_title() {
        let chart = BarChartBuilder::new().title("My Chart").build();
        assert_eq!(chart.title, "My Chart");
    }

    #[test]
    fn test_bar_chart_builder_title_from_string() {
        let chart = BarChartBuilder::new()
            .title(String::from("Dynamic Title"))
            .build();
        assert_eq!(chart.title, "Dynamic Title");
    }

    #[test]
    fn test_bar_chart_builder_add_data() {
        let chart = BarChartBuilder::new()
            .add_data(ChartData::new("A", 10.0))
            .add_data(ChartData::new("B", 20.0))
            .build();

        assert_eq!(chart.data.len(), 2);
        assert_eq!(chart.data[0].label, "A");
        assert_eq!(chart.data[1].label, "B");
    }

    #[test]
    fn test_bar_chart_builder_data_replaces() {
        let chart = BarChartBuilder::new()
            .add_data(ChartData::new("Old", 1.0))
            .data(vec![ChartData::new("New", 2.0)])
            .build();

        assert_eq!(chart.data.len(), 1);
        assert_eq!(chart.data[0].label, "New");
    }

    #[test]
    fn test_bar_chart_builder_orientation() {
        let chart = BarChartBuilder::new()
            .orientation(BarOrientation::Horizontal)
            .build();
        assert_eq!(chart.orientation, BarOrientation::Horizontal);
    }

    #[test]
    fn test_bar_chart_builder_colors() {
        let colors = vec![Color::red(), Color::blue()];
        let chart = BarChartBuilder::new().colors(colors).build();

        assert_eq!(chart.colors.len(), 2);
        assert_eq!(chart.colors[0], Color::red());
    }

    #[test]
    fn test_bar_chart_builder_title_font() {
        let chart = BarChartBuilder::new()
            .title_font(Font::TimesBold, 24.0)
            .build();

        assert_eq!(chart.title_font, Font::TimesBold);
        assert_eq!(chart.title_font_size, 24.0);
    }

    #[test]
    fn test_bar_chart_builder_label_font() {
        let chart = BarChartBuilder::new()
            .label_font(Font::Courier, 8.0)
            .build();

        assert_eq!(chart.label_font, Font::Courier);
        assert_eq!(chart.label_font_size, 8.0);
    }

    #[test]
    fn test_bar_chart_builder_value_font() {
        let chart = BarChartBuilder::new()
            .value_font(Font::CourierBold, 14.0)
            .build();

        assert_eq!(chart.value_font, Font::CourierBold);
        assert_eq!(chart.value_font_size, 14.0);
    }

    #[test]
    fn test_bar_chart_builder_legend_position() {
        let chart = BarChartBuilder::new()
            .legend_position(LegendPosition::Bottom)
            .build();
        assert_eq!(chart.legend_position, LegendPosition::Bottom);
    }

    #[test]
    fn test_bar_chart_builder_background_color() {
        let chart = BarChartBuilder::new()
            .background_color(Color::white())
            .build();
        assert_eq!(chart.background_color, Some(Color::white()));
    }

    #[test]
    fn test_bar_chart_builder_show_values() {
        let chart = BarChartBuilder::new().show_values(false).build();
        assert!(!chart.show_values);
    }

    #[test]
    fn test_bar_chart_builder_show_grid() {
        let chart = BarChartBuilder::new().show_grid(false).build();
        assert!(!chart.show_grid);
    }

    #[test]
    fn test_bar_chart_builder_grid_color() {
        let chart = BarChartBuilder::new().grid_color(Color::blue()).build();
        assert_eq!(chart.grid_color, Color::blue());
    }

    #[test]
    fn test_bar_chart_builder_bar_spacing() {
        let chart = BarChartBuilder::new().bar_spacing(0.5).build();
        assert_eq!(chart.bar_spacing, 0.5);
    }

    #[test]
    fn test_bar_chart_builder_bar_spacing_negative_clamped() {
        let chart = BarChartBuilder::new().bar_spacing(-0.5).build();
        // Negative spacing is clamped to 0
        assert_eq!(chart.bar_spacing, 0.0);
    }

    #[test]
    fn test_bar_chart_builder_bar_border() {
        let chart = BarChartBuilder::new()
            .bar_border(Color::black(), 2.0)
            .build();

        assert_eq!(chart.bar_border_color, Some(Color::black()));
        assert_eq!(chart.bar_border_width, 2.0);
    }

    #[test]
    fn test_bar_chart_builder_bar_width_range() {
        let chart = BarChartBuilder::new()
            .bar_width_range(15.0, Some(100.0))
            .build();

        assert_eq!(chart.min_bar_width, 15.0);
        assert_eq!(chart.max_bar_width, Some(100.0));
    }

    #[test]
    fn test_bar_chart_builder_simple_data_labels() {
        let chart = BarChartBuilder::new()
            .simple_data(vec![1.0, 2.0, 3.0])
            .build();

        assert_eq!(chart.data[0].label, "Item 1");
        assert_eq!(chart.data[1].label, "Item 2");
        assert_eq!(chart.data[2].label, "Item 3");
    }

    #[test]
    fn test_bar_chart_builder_labeled_data() {
        let chart = BarChartBuilder::new()
            .labeled_data(vec![("Q1", 100.0), ("Q2", 200.0)])
            .build();

        assert_eq!(chart.data.len(), 2);
        assert_eq!(chart.data[0].label, "Q1");
        assert_eq!(chart.data[0].value, 100.0);
        assert_eq!(chart.data[1].label, "Q2");
        assert_eq!(chart.data[1].value, 200.0);
    }

    #[test]
    fn test_financial_style() {
        let chart = BarChartBuilder::new().financial_style().build();

        assert!(chart.show_grid);
        assert!(chart.bar_border_color.is_some());
        assert_eq!(chart.colors.len(), 4);
    }

    #[test]
    fn test_minimal_style() {
        let chart = BarChartBuilder::new().minimal_style().build();

        assert!(!chart.show_grid);
        assert!(!chart.show_values);
        assert_eq!(chart.background_color, None);
        assert_eq!(chart.bar_border_color, None);
    }

    #[test]
    fn test_progress_style() {
        let chart = BarChartBuilder::new()
            .progress_style(Color::green())
            .build();

        assert_eq!(chart.orientation, BarOrientation::Horizontal);
        assert_eq!(chart.colors.len(), 1);
        assert_eq!(chart.colors[0], Color::green());
        assert!(!chart.show_grid);
        assert!(chart.show_values);
        assert_eq!(chart.legend_position, LegendPosition::None);
        assert_eq!(chart.bar_spacing, 0.0);
    }

    #[test]
    fn test_bar_chart_builder_chained() {
        let chart = BarChartBuilder::new()
            .title("Sales Report")
            .orientation(BarOrientation::Horizontal)
            .labeled_data(vec![("2023", 1000.0), ("2024", 1500.0)])
            .financial_style()
            .show_values(true)
            .legend_position(LegendPosition::Right)
            .build();

        assert_eq!(chart.title, "Sales Report");
        assert_eq!(chart.orientation, BarOrientation::Horizontal);
        assert_eq!(chart.data.len(), 2);
        assert!(chart.show_values);
        assert_eq!(chart.legend_position, LegendPosition::Right);
    }

    // ==================== default_bar_colors Tests ====================

    #[test]
    fn test_default_bar_colors_count() {
        let colors = default_bar_colors();
        assert_eq!(colors.len(), 8);
    }

    #[test]
    fn test_default_bar_colors_valid_rgb() {
        let colors = default_bar_colors();
        for color in colors {
            assert!(color.r() >= 0.0 && color.r() <= 1.0);
            assert!(color.g() >= 0.0 && color.g() <= 1.0);
            assert!(color.b() >= 0.0 && color.b() <= 1.0);
        }
    }

    #[test]
    fn test_default_bar_colors_unique() {
        let colors = default_bar_colors();
        for i in 0..colors.len() {
            for j in (i + 1)..colors.len() {
                assert_ne!(
                    colors[i], colors[j],
                    "Colors at {} and {} should be different",
                    i, j
                );
            }
        }
    }

    // ==================== Edge Cases ====================

    #[test]
    fn test_empty_bar_chart() {
        let chart = BarChartBuilder::new().build();
        assert!(chart.data.is_empty());
        assert_eq!(chart.max_value(), 0.0);
        assert_eq!(chart.min_value(), 0.0);
    }

    #[test]
    fn test_single_bar() {
        let chart = BarChartBuilder::new()
            .add_data(ChartData::new("Only", 42.0))
            .build();

        assert_eq!(chart.data.len(), 1);
        assert_eq!(chart.max_value(), 42.0);
    }

    #[test]
    fn test_many_bars() {
        let mut builder = BarChartBuilder::new();
        for i in 0..100 {
            builder = builder.add_data(ChartData::new(format!("Item {}", i), i as f64));
        }
        let chart = builder.build();

        assert_eq!(chart.data.len(), 100);
        assert_eq!(chart.max_value(), 99.0);
    }

    #[test]
    fn test_bar_chart_clone() {
        let chart = BarChartBuilder::new()
            .title("Original")
            .simple_data(vec![1.0, 2.0])
            .build();

        let cloned = chart.clone();
        assert_eq!(cloned.title, "Original");
        assert_eq!(cloned.data.len(), 2);
    }

    #[test]
    fn test_bar_chart_debug() {
        let chart = BarChart::new();
        let debug_str = format!("{:?}", chart);
        assert!(debug_str.contains("BarChart"));
    }
}
