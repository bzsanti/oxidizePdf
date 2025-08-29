//! Pie chart implementation with customizable segments and labels

use super::chart_builder::{ChartData, LegendPosition};
use crate::graphics::Color;
use crate::text::Font;

/// A segment of a pie chart
#[derive(Debug, Clone)]
pub struct PieSegment {
    /// Segment label
    pub label: String,
    /// Segment value
    pub value: f64,
    /// Segment color
    pub color: Color,
    /// Whether this segment is exploded (separated from the pie)
    pub exploded: bool,
    /// Explosion distance (as a fraction of radius)
    pub explosion_distance: f64,
    /// Whether to show the percentage on this segment
    pub show_percentage: bool,
    /// Whether to show the label on this segment
    pub show_label: bool,
}

impl PieSegment {
    /// Create a new pie segment
    pub fn new<S: Into<String>>(label: S, value: f64, color: Color) -> Self {
        Self {
            label: label.into(),
            value,
            color,
            exploded: false,
            explosion_distance: 0.1, // 10% of radius by default
            show_percentage: true,
            show_label: true,
        }
    }

    /// Make this segment exploded
    pub fn exploded(mut self, distance: f64) -> Self {
        self.exploded = true;
        self.explosion_distance = distance;
        self
    }

    /// Control visibility of percentage on this segment
    pub fn show_percentage(mut self, show: bool) -> Self {
        self.show_percentage = show;
        self
    }

    /// Control visibility of label on this segment
    pub fn show_label(mut self, show: bool) -> Self {
        self.show_label = show;
        self
    }

    /// Calculate the percentage of this segment relative to total
    pub fn percentage(&self, total: f64) -> f64 {
        if total > 0.0 {
            (self.value / total) * 100.0
        } else {
            0.0
        }
    }

    /// Calculate the angle of this segment in radians
    pub fn angle_radians(&self, total: f64) -> f64 {
        if total > 0.0 {
            (self.value / total) * 2.0 * std::f64::consts::PI
        } else {
            0.0
        }
    }
}

/// Pie chart configuration
#[derive(Debug, Clone)]
pub struct PieChart {
    /// Chart title
    pub title: String,
    /// Pie segments
    pub segments: Vec<PieSegment>,
    /// Default colors (used when segments don't have custom colors)
    pub colors: Vec<Color>,
    /// Title font and size
    pub title_font: Font,
    pub title_font_size: f64,
    /// Label font and size
    pub label_font: Font,
    pub label_font_size: f64,
    /// Percentage font and size
    pub percentage_font: Font,
    pub percentage_font_size: f64,
    /// Legend position
    pub legend_position: LegendPosition,
    /// Background color
    pub background_color: Option<Color>,
    /// Show percentages on segments
    pub show_percentages: bool,
    /// Show labels on segments
    pub show_labels: bool,
    /// Start angle in radians (0 = 3 o'clock, π/2 = 12 o'clock)
    pub start_angle: f64,
    /// Whether to draw segment borders
    pub draw_borders: bool,
    /// Border color
    pub border_color: Color,
    /// Border width
    pub border_width: f64,
    /// Minimum segment angle to show label (in radians)
    pub min_label_angle: f64,
    /// Label distance from pie edge (as fraction of radius)
    pub label_distance: f64,
}

impl PieChart {
    /// Create a new pie chart
    pub fn new() -> Self {
        Self {
            title: String::new(),
            segments: Vec::new(),
            colors: default_pie_colors(),
            title_font: Font::HelveticaBold,
            title_font_size: 16.0,
            label_font: Font::Helvetica,
            label_font_size: 10.0,
            percentage_font: Font::Helvetica,
            percentage_font_size: 9.0,
            legend_position: LegendPosition::Right,
            background_color: None,
            show_percentages: true,
            show_labels: true,
            start_angle: -std::f64::consts::PI / 2.0, // Start at 12 o'clock
            draw_borders: true,
            border_color: Color::white(),
            border_width: 2.0,
            min_label_angle: 0.1, // About 5.7 degrees
            label_distance: 1.2,  // 120% of radius
        }
    }

    /// Get the total value of all segments
    pub fn total_value(&self) -> f64 {
        self.segments.iter().map(|s| s.value).sum()
    }

    /// Calculate percentage for a segment at the given index
    pub fn percentage_for_index(&self, index: usize) -> f64 {
        if let Some(segment) = self.segments.get(index) {
            segment.percentage(self.total_value())
        } else {
            0.0
        }
    }

    /// Calculate the cumulative angles for each segment
    pub fn cumulative_angles(&self) -> Vec<f64> {
        let total = self.total_value();
        let mut cumulative = Vec::new();
        let mut current_angle = self.start_angle;

        for segment in &self.segments {
            cumulative.push(current_angle);
            current_angle += segment.angle_radians(total);
        }

        cumulative
    }

    /// Get the middle angle of a segment
    pub fn segment_middle_angle(&self, index: usize) -> f64 {
        let angles = self.cumulative_angles();
        if let Some(start_angle) = angles.get(index) {
            let segment_angle = if let Some(segment) = self.segments.get(index) {
                segment.angle_radians(self.total_value())
            } else {
                0.0
            };
            start_angle + segment_angle / 2.0
        } else {
            0.0
        }
    }
}

impl Default for PieChart {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for creating pie charts with fluent API
pub struct PieChartBuilder {
    chart: PieChart,
}

impl PieChartBuilder {
    /// Create a new pie chart builder
    pub fn new() -> Self {
        Self {
            chart: PieChart::new(),
        }
    }

    /// Set chart title
    pub fn title<S: Into<String>>(mut self, title: S) -> Self {
        self.chart.title = title.into();
        self
    }

    /// Add a single segment
    pub fn add_segment(mut self, segment: PieSegment) -> Self {
        self.chart.segments.push(segment);
        self
    }

    /// Set all segments at once
    pub fn segments(mut self, segments: Vec<PieSegment>) -> Self {
        self.chart.segments = segments;
        self
    }

    /// Set default colors
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

    /// Set percentage font and size
    pub fn percentage_font(mut self, font: Font, size: f64) -> Self {
        self.chart.percentage_font = font;
        self.chart.percentage_font_size = size;
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

    /// Show or hide percentages on segments
    pub fn show_percentages(mut self, show: bool) -> Self {
        self.chart.show_percentages = show;
        self
    }

    /// Show or hide labels on segments
    pub fn show_labels(mut self, show: bool) -> Self {
        self.chart.show_labels = show;
        self
    }

    /// Set start angle in radians
    pub fn start_angle(mut self, angle: f64) -> Self {
        self.chart.start_angle = angle;
        self
    }

    /// Set border style
    pub fn border(mut self, color: Color, width: f64) -> Self {
        self.chart.draw_borders = width > 0.0;
        self.chart.border_color = color;
        self.chart.border_width = width;
        self
    }

    /// Set label positioning
    pub fn label_settings(mut self, distance: f64, min_angle: f64) -> Self {
        self.chart.label_distance = distance;
        self.chart.min_label_angle = min_angle;
        self
    }

    /// Add data from ChartData (converting to segments)
    pub fn data(mut self, data: Vec<ChartData>) -> Self {
        for (i, item) in data.into_iter().enumerate() {
            let color = item.color.unwrap_or_else(|| {
                self.chart
                    .colors
                    .get(i % self.chart.colors.len())
                    .copied()
                    .unwrap_or(Color::gray(0.5))
            });

            let mut segment = PieSegment::new(item.label, item.value, color);
            if item.highlighted {
                segment = segment.exploded(0.15);
            }

            self.chart.segments.push(segment);
        }
        self
    }

    /// Add data from simple values with automatic labels and colors
    pub fn simple_data(mut self, values: Vec<f64>) -> Self {
        for (i, value) in values.into_iter().enumerate() {
            let color = self
                .chart
                .colors
                .get(i % self.chart.colors.len())
                .copied()
                .unwrap_or(Color::gray(0.5));

            self.chart
                .segments
                .push(PieSegment::new(format!("Segment {}", i + 1), value, color));
        }
        self
    }

    /// Add data from label-value pairs with automatic colors
    pub fn labeled_data(mut self, data: Vec<(&str, f64)>) -> Self {
        for (i, (label, value)) in data.into_iter().enumerate() {
            let color = self
                .chart
                .colors
                .get(i % self.chart.colors.len())
                .copied()
                .unwrap_or(Color::gray(0.5));

            self.chart
                .segments
                .push(PieSegment::new(label, value, color));
        }
        self
    }

    /// Create a financial pie chart style
    pub fn financial_style(mut self) -> Self {
        self.chart.colors = vec![
            Color::rgb(0.2, 0.6, 0.2), // Green
            Color::rgb(0.8, 0.2, 0.2), // Red
            Color::rgb(0.2, 0.4, 0.8), // Blue
            Color::rgb(0.9, 0.6, 0.1), // Orange
            Color::rgb(0.6, 0.2, 0.8), // Purple
        ];
        self.chart.border_color = Color::white();
        self.chart.border_width = 2.0;
        self
    }

    /// Create a minimal pie chart style
    pub fn minimal_style(mut self) -> Self {
        self.chart.draw_borders = false;
        self.chart.show_percentages = false;
        self.chart.background_color = None;
        self.chart.legend_position = LegendPosition::None;
        self
    }

    /// Create a donut chart style (with center hole - requires custom renderer)
    pub fn donut_style(mut self) -> Self {
        // Note: Donut rendering would need special handling in the renderer
        self.chart.border_width = 1.0;
        self.chart.border_color = Color::white();
        self
    }

    /// Build the final pie chart
    pub fn build(self) -> PieChart {
        self.chart
    }
}

impl Default for PieChartBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Default color palette for pie charts
fn default_pie_colors() -> Vec<Color> {
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
    fn test_pie_segment_creation() {
        let segment = PieSegment::new("Test", 25.0, Color::blue());
        assert_eq!(segment.label, "Test");
        assert_eq!(segment.value, 25.0);
        assert_eq!(segment.color, Color::blue());
        assert!(!segment.exploded);
    }

    #[test]
    fn test_pie_segment_percentage() {
        let segment = PieSegment::new("Test", 25.0, Color::blue());
        assert_eq!(segment.percentage(100.0), 25.0);
        assert_eq!(segment.percentage(0.0), 0.0);
    }

    #[test]
    fn test_pie_chart_creation() {
        let chart = PieChartBuilder::new()
            .title("Test Pie")
            .simple_data(vec![25.0, 35.0, 40.0])
            .build();

        assert_eq!(chart.title, "Test Pie");
        assert_eq!(chart.segments.len(), 3);
        assert_eq!(chart.total_value(), 100.0);
    }

    #[test]
    fn test_pie_chart_angles() {
        let chart = PieChartBuilder::new()
            .simple_data(vec![50.0, 50.0]) // Half and half
            .build();

        let angles = chart.cumulative_angles();
        assert_eq!(angles.len(), 2);

        // First segment should be π radians (180 degrees)
        let segment_angle = chart.segments[0].angle_radians(chart.total_value());
        assert!((segment_angle - std::f64::consts::PI).abs() < 0.001);
    }

    #[test]
    fn test_exploded_segment() {
        let segment = PieSegment::new("Test", 25.0, Color::blue()).exploded(0.2);

        assert!(segment.exploded);
        assert_eq!(segment.explosion_distance, 0.2);
    }
}
