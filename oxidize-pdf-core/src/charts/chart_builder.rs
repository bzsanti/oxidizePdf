//! Generic chart builder and data structures

use crate::graphics::Color;
use crate::text::Font;

/// Position of the chart legend
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum LegendPosition {
    /// No legend
    None,
    /// Legend on the right side
    #[default]
    Right,
    /// Legend at the bottom
    Bottom,
    /// Legend at the top
    Top,
    /// Legend on the left side
    Left,
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

    // ==================== LegendPosition Tests ====================

    #[test]
    fn test_legend_position_default() {
        let pos: LegendPosition = Default::default();
        assert_eq!(pos, LegendPosition::Right);
    }

    #[test]
    fn test_legend_position_variants() {
        assert_eq!(LegendPosition::None, LegendPosition::None);
        assert_eq!(LegendPosition::Right, LegendPosition::Right);
        assert_eq!(LegendPosition::Bottom, LegendPosition::Bottom);
        assert_eq!(LegendPosition::Top, LegendPosition::Top);
        assert_eq!(LegendPosition::Left, LegendPosition::Left);
    }

    #[test]
    fn test_legend_position_clone() {
        let pos = LegendPosition::Bottom;
        let cloned = pos;
        assert_eq!(pos, cloned);
    }

    #[test]
    fn test_legend_position_debug() {
        let debug_str = format!("{:?}", LegendPosition::Top);
        assert!(debug_str.contains("Top"));
    }

    // ==================== ChartType Tests ====================

    #[test]
    fn test_chart_type_variants() {
        assert_eq!(ChartType::VerticalBar, ChartType::VerticalBar);
        assert_eq!(ChartType::HorizontalBar, ChartType::HorizontalBar);
        assert_eq!(ChartType::Pie, ChartType::Pie);
        assert_eq!(ChartType::Line, ChartType::Line);
        assert_eq!(ChartType::Area, ChartType::Area);
    }

    #[test]
    fn test_chart_type_not_equal() {
        assert_ne!(ChartType::VerticalBar, ChartType::HorizontalBar);
        assert_ne!(ChartType::Pie, ChartType::Line);
    }

    #[test]
    fn test_chart_type_clone() {
        let ct = ChartType::Area;
        let cloned = ct;
        assert_eq!(ct, cloned);
    }

    #[test]
    fn test_chart_type_debug() {
        let debug_str = format!("{:?}", ChartType::Line);
        assert!(debug_str.contains("Line"));
    }

    // ==================== ChartData Tests ====================

    #[test]
    fn test_chart_data_creation() {
        let data = ChartData::new("Test", 42.0);
        assert_eq!(data.label, "Test");
        assert_eq!(data.value, 42.0);
        assert_eq!(data.color, None);
        assert!(!data.highlighted);
    }

    #[test]
    fn test_chart_data_with_color() {
        let data = ChartData::new("Test", 42.0)
            .color(Color::red())
            .highlighted();

        assert_eq!(data.color, Some(Color::red()));
        assert!(data.highlighted);
    }

    #[test]
    fn test_chart_data_from_string() {
        let data = ChartData::new(String::from("Dynamic"), 99.9);
        assert_eq!(data.label, "Dynamic");
        assert_eq!(data.value, 99.9);
    }

    #[test]
    fn test_chart_data_negative_value() {
        let data = ChartData::new("Negative", -25.5);
        assert_eq!(data.value, -25.5);
    }

    #[test]
    fn test_chart_data_zero_value() {
        let data = ChartData::new("Zero", 0.0);
        assert_eq!(data.value, 0.0);
    }

    #[test]
    fn test_chart_data_clone() {
        let data = ChartData::new("Clone", 10.0).color(Color::blue());
        let cloned = data.clone();
        assert_eq!(cloned.label, "Clone");
        assert_eq!(cloned.value, 10.0);
        assert_eq!(cloned.color, Some(Color::blue()));
    }

    #[test]
    fn test_chart_data_debug() {
        let data = ChartData::new("Debug", 5.0);
        let debug_str = format!("{:?}", data);
        assert!(debug_str.contains("Debug"));
        assert!(debug_str.contains("5"));
    }

    // ==================== Chart Tests ====================

    #[test]
    fn test_chart_new_vertical_bar() {
        let chart = Chart::new(ChartType::VerticalBar);
        assert_eq!(chart.chart_type, ChartType::VerticalBar);
        assert!(chart.title.is_empty());
        assert!(chart.data.is_empty());
        assert_eq!(chart.legend_position, LegendPosition::Right);
    }

    #[test]
    fn test_chart_new_pie() {
        let chart = Chart::new(ChartType::Pie);
        assert_eq!(chart.chart_type, ChartType::Pie);
    }

    #[test]
    fn test_chart_new_line() {
        let chart = Chart::new(ChartType::Line);
        assert_eq!(chart.chart_type, ChartType::Line);
    }

    #[test]
    fn test_chart_new_defaults() {
        let chart = Chart::new(ChartType::Area);
        assert_eq!(chart.title_font, Font::HelveticaBold);
        assert_eq!(chart.title_font_size, 14.0);
        assert_eq!(chart.label_font, Font::Helvetica);
        assert_eq!(chart.label_font_size, 10.0);
        assert!(chart.show_values);
        assert!(chart.show_grid);
        assert_eq!(chart.border_width, 1.0);
        assert_eq!(chart.background_color, None);
    }

    #[test]
    fn test_chart_max_value_empty() {
        let chart = Chart::new(ChartType::VerticalBar);
        assert_eq!(chart.max_value(), 0.0);
    }

    #[test]
    fn test_chart_total_value_empty() {
        let chart = Chart::new(ChartType::Pie);
        assert_eq!(chart.total_value(), 0.0);
    }

    #[test]
    fn test_chart_max_value_with_negatives() {
        let mut chart = Chart::new(ChartType::Line);
        chart.data = vec![
            ChartData::new("A", -10.0),
            ChartData::new("B", -5.0),
            ChartData::new("C", -20.0),
        ];
        // Note: max_value uses fold(0.0, f64::max), so all-negative data returns 0.0
        // This is the current behavior - chart assumes non-negative values
        assert_eq!(chart.max_value(), 0.0);
    }

    #[test]
    fn test_chart_total_value_with_negatives() {
        let mut chart = Chart::new(ChartType::VerticalBar);
        chart.data = vec![ChartData::new("A", 10.0), ChartData::new("B", -5.0)];
        assert_eq!(chart.total_value(), 5.0);
    }

    #[test]
    fn test_chart_color_for_index_out_of_bounds() {
        let chart = Chart::new(ChartType::Pie);
        // Should wrap around using modulo
        let color = chart.color_for_index(100);
        // Verify it returns a valid color (doesn't panic)
        assert!(color.r() >= 0.0 && color.r() <= 1.0);
    }

    #[test]
    #[should_panic(expected = "attempt to calculate the remainder with a divisor of zero")]
    fn test_chart_color_for_index_with_empty_colors_panics() {
        let mut chart = Chart::new(ChartType::Pie);
        chart.colors = vec![];
        // Current behavior: panics with empty colors array
        // This documents the edge case - users should not empty the colors array
        let _ = chart.color_for_index(0);
    }

    #[test]
    fn test_chart_color_for_index_custom_overrides_default() {
        let mut chart = Chart::new(ChartType::Pie);
        chart.data = vec![ChartData::new("Custom", 10.0).color(Color::green())];
        assert_eq!(chart.color_for_index(0), Color::green());
    }

    // ==================== ChartBuilder Tests ====================

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
    fn test_chart_builder_add_data() {
        let chart = ChartBuilder::new(ChartType::Pie)
            .add_data(ChartData::new("A", 10.0))
            .add_data(ChartData::new("B", 20.0))
            .build();

        assert_eq!(chart.data.len(), 2);
        assert_eq!(chart.data[0].label, "A");
        assert_eq!(chart.data[1].label, "B");
    }

    #[test]
    fn test_chart_builder_data_replaces() {
        let chart = ChartBuilder::new(ChartType::Line)
            .add_data(ChartData::new("Old", 1.0))
            .data(vec![ChartData::new("New", 2.0)])
            .build();

        assert_eq!(chart.data.len(), 1);
        assert_eq!(chart.data[0].label, "New");
    }

    #[test]
    fn test_chart_builder_colors() {
        let custom_colors = vec![Color::red(), Color::blue()];
        let chart = ChartBuilder::new(ChartType::VerticalBar)
            .colors(custom_colors.clone())
            .build();

        assert_eq!(chart.colors.len(), 2);
        assert_eq!(chart.colors[0], Color::red());
        assert_eq!(chart.colors[1], Color::blue());
    }

    #[test]
    fn test_chart_builder_title_font() {
        let chart = ChartBuilder::new(ChartType::Pie)
            .title_font(Font::TimesBold, 24.0)
            .build();

        assert_eq!(chart.title_font, Font::TimesBold);
        assert_eq!(chart.title_font_size, 24.0);
    }

    #[test]
    fn test_chart_builder_label_font() {
        let chart = ChartBuilder::new(ChartType::Line)
            .label_font(Font::Courier, 8.0)
            .build();

        assert_eq!(chart.label_font, Font::Courier);
        assert_eq!(chart.label_font_size, 8.0);
    }

    #[test]
    fn test_chart_builder_legend_position() {
        let chart = ChartBuilder::new(ChartType::Pie)
            .legend_position(LegendPosition::Bottom)
            .build();

        assert_eq!(chart.legend_position, LegendPosition::Bottom);
    }

    #[test]
    fn test_chart_builder_legend_none() {
        let chart = ChartBuilder::new(ChartType::VerticalBar)
            .legend_position(LegendPosition::None)
            .build();

        assert_eq!(chart.legend_position, LegendPosition::None);
    }

    #[test]
    fn test_chart_builder_background_color() {
        let chart = ChartBuilder::new(ChartType::Line)
            .background_color(Color::white())
            .build();

        assert_eq!(chart.background_color, Some(Color::white()));
    }

    #[test]
    fn test_chart_builder_show_values() {
        let chart = ChartBuilder::new(ChartType::VerticalBar)
            .show_values(false)
            .build();

        assert!(!chart.show_values);
    }

    #[test]
    fn test_chart_builder_show_grid() {
        let chart = ChartBuilder::new(ChartType::Line).show_grid(false).build();

        assert!(!chart.show_grid);
    }

    #[test]
    fn test_chart_builder_grid_color() {
        let chart = ChartBuilder::new(ChartType::Area)
            .grid_color(Color::gray(0.5))
            .build();

        assert_eq!(chart.grid_color, Color::gray(0.5));
    }

    #[test]
    fn test_chart_builder_border() {
        let chart = ChartBuilder::new(ChartType::VerticalBar)
            .border(Color::black(), 2.5)
            .build();

        assert_eq!(chart.border_color, Color::black());
        assert_eq!(chart.border_width, 2.5);
    }

    #[test]
    fn test_chart_builder_simple_data_labels() {
        let chart = ChartBuilder::new(ChartType::VerticalBar)
            .simple_data(vec![1.0, 2.0, 3.0])
            .build();

        assert_eq!(chart.data[0].label, "Item 1");
        assert_eq!(chart.data[1].label, "Item 2");
        assert_eq!(chart.data[2].label, "Item 3");
    }

    #[test]
    fn test_chart_builder_labeled_data() {
        let chart = ChartBuilder::new(ChartType::Pie)
            .labeled_data(vec![("Q1", 100.0), ("Q2", 200.0)])
            .build();

        assert_eq!(chart.data.len(), 2);
        assert_eq!(chart.data[0].label, "Q1");
        assert_eq!(chart.data[0].value, 100.0);
        assert_eq!(chart.data[1].label, "Q2");
        assert_eq!(chart.data[1].value, 200.0);
    }

    #[test]
    fn test_chart_builder_financial_style() {
        let chart = ChartBuilder::new(ChartType::VerticalBar)
            .financial_style()
            .build();

        assert_eq!(chart.colors.len(), 5);
        assert!(chart.show_grid);
        assert_eq!(chart.title_font_size, 16.0);
    }

    #[test]
    fn test_chart_builder_minimal_style() {
        let chart = ChartBuilder::new(ChartType::Line).minimal_style().build();

        assert!(!chart.show_grid);
        assert_eq!(chart.border_width, 0.0);
        assert_eq!(chart.background_color, None);
    }

    #[test]
    fn test_chart_builder_chained_styles() {
        let chart = ChartBuilder::new(ChartType::VerticalBar)
            .title("Financial Report")
            .financial_style()
            .labeled_data(vec![("2023", 1000.0), ("2024", 1500.0)])
            .legend_position(LegendPosition::Bottom)
            .build();

        assert_eq!(chart.title, "Financial Report");
        assert_eq!(chart.data.len(), 2);
        assert_eq!(chart.legend_position, LegendPosition::Bottom);
        assert!(chart.show_grid);
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

    // ==================== default_colors Tests ====================

    #[test]
    fn test_default_colors_count() {
        let colors = default_colors();
        assert_eq!(colors.len(), 10);
    }

    #[test]
    fn test_default_colors_valid_rgb() {
        let colors = default_colors();
        for color in colors {
            assert!(color.r() >= 0.0 && color.r() <= 1.0);
            assert!(color.g() >= 0.0 && color.g() <= 1.0);
            assert!(color.b() >= 0.0 && color.b() <= 1.0);
        }
    }

    #[test]
    fn test_default_colors_unique() {
        let colors = default_colors();
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
    fn test_empty_chart() {
        let chart = ChartBuilder::new(ChartType::Pie).build();
        assert!(chart.data.is_empty());
        assert_eq!(chart.max_value(), 0.0);
        assert_eq!(chart.total_value(), 0.0);
    }

    #[test]
    fn test_single_data_point() {
        let chart = ChartBuilder::new(ChartType::VerticalBar)
            .add_data(ChartData::new("Only", 42.0))
            .build();

        assert_eq!(chart.data.len(), 1);
        assert_eq!(chart.max_value(), 42.0);
        assert_eq!(chart.total_value(), 42.0);
    }

    #[test]
    fn test_large_dataset() {
        let mut builder = ChartBuilder::new(ChartType::Line);
        for i in 0..1000 {
            builder = builder.add_data(ChartData::new(format!("Item {}", i), i as f64));
        }
        let chart = builder.build();

        assert_eq!(chart.data.len(), 1000);
        assert_eq!(chart.max_value(), 999.0);
        assert_eq!(chart.total_value(), (0..1000).sum::<i32>() as f64);
    }

    #[test]
    fn test_special_float_values() {
        let chart = ChartBuilder::new(ChartType::VerticalBar)
            .simple_data(vec![f64::MIN_POSITIVE, f64::MAX / 2.0])
            .build();

        assert_eq!(chart.data.len(), 2);
        assert!(chart.max_value() > 0.0);
    }
}
