//! Line chart implementation with multiple data series support

use super::chart_builder::LegendPosition;
use crate::graphics::Color;
use crate::text::Font;

/// A data series for line charts
#[derive(Debug, Clone)]
pub struct DataSeries {
    /// Series name
    pub name: String,
    /// Data points (x, y) pairs
    pub data: Vec<(f64, f64)>,
    /// Line color
    pub color: Color,
    /// Line width
    pub line_width: f64,
    /// Whether to show markers at data points
    pub show_markers: bool,
    /// Marker size
    pub marker_size: f64,
    /// Whether to fill area under the line
    pub fill_area: bool,
    /// Fill color (if different from line color)
    pub fill_color: Option<Color>,
}

impl DataSeries {
    /// Create a new data series
    pub fn new<S: Into<String>>(name: S, color: Color) -> Self {
        Self {
            name: name.into(),
            data: Vec::new(),
            color,
            line_width: 2.0,
            show_markers: true,
            marker_size: 4.0,
            fill_area: false,
            fill_color: None,
        }
    }

    /// Add data points from y-values (x will be 0, 1, 2, ...)
    pub fn y_data(mut self, values: Vec<f64>) -> Self {
        self.data = values
            .into_iter()
            .enumerate()
            .map(|(i, y)| (i as f64, y))
            .collect();
        self
    }

    /// Add data points from (x, y) pairs
    pub fn xy_data(mut self, data: Vec<(f64, f64)>) -> Self {
        self.data = data;
        self
    }

    /// Set line style
    pub fn line_style(mut self, width: f64) -> Self {
        self.line_width = width;
        self
    }

    /// Enable/disable markers
    pub fn markers(mut self, show: bool, size: f64) -> Self {
        self.show_markers = show;
        self.marker_size = size;
        self
    }

    /// Enable area fill
    pub fn fill_area(mut self, fill_color: Option<Color>) -> Self {
        self.fill_area = true;
        self.fill_color = fill_color;
        self
    }

    /// Get the range of x values
    pub fn x_range(&self) -> (f64, f64) {
        if self.data.is_empty() {
            return (0.0, 1.0);
        }

        let xs: Vec<f64> = self.data.iter().map(|(x, _)| *x).collect();
        let min_x = xs.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        let max_x = xs.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));

        (min_x, max_x)
    }

    /// Get the range of y values
    pub fn y_range(&self) -> (f64, f64) {
        if self.data.is_empty() {
            return (0.0, 1.0);
        }

        let ys: Vec<f64> = self.data.iter().map(|(_, y)| *y).collect();
        let min_y = ys.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        let max_y = ys.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));

        (min_y, max_y)
    }
}

/// Line chart configuration
#[derive(Debug, Clone)]
pub struct LineChart {
    /// Chart title
    pub title: String,
    /// Data series
    pub series: Vec<DataSeries>,
    /// X-axis label
    pub x_axis_label: String,
    /// Y-axis label
    pub y_axis_label: String,
    /// Title font and size
    pub title_font: Font,
    pub title_font_size: f64,
    /// Label font and size
    pub label_font: Font,
    pub label_font_size: f64,
    /// Axis font and size
    pub axis_font: Font,
    pub axis_font_size: f64,
    /// Legend position
    pub legend_position: LegendPosition,
    /// Background color
    pub background_color: Option<Color>,
    /// Show grid lines
    pub show_grid: bool,
    /// Grid color
    pub grid_color: Color,
    /// Axis color
    pub axis_color: Color,
    /// X-axis range (None for auto)
    pub x_range: Option<(f64, f64)>,
    /// Y-axis range (None for auto)
    pub y_range: Option<(f64, f64)>,
    /// Number of grid lines
    pub grid_lines: usize,
}

impl LineChart {
    /// Create a new line chart
    pub fn new() -> Self {
        Self {
            title: String::new(),
            series: Vec::new(),
            x_axis_label: String::new(),
            y_axis_label: String::new(),
            title_font: Font::HelveticaBold,
            title_font_size: 16.0,
            label_font: Font::Helvetica,
            label_font_size: 12.0,
            axis_font: Font::Helvetica,
            axis_font_size: 10.0,
            legend_position: LegendPosition::Right,
            background_color: None,
            show_grid: true,
            grid_color: Color::rgb(0.9, 0.9, 0.9),
            axis_color: Color::black(),
            x_range: None,
            y_range: None,
            grid_lines: 5,
        }
    }

    /// Get the combined X range of all series
    pub fn combined_x_range(&self) -> (f64, f64) {
        if let Some(range) = self.x_range {
            return range;
        }

        if self.series.is_empty() {
            return (0.0, 1.0);
        }

        let mut min_x = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;

        for series in &self.series {
            let (series_min, series_max) = series.x_range();
            min_x = min_x.min(series_min);
            max_x = max_x.max(series_max);
        }

        // Add some padding
        let range = max_x - min_x;
        let padding = range * 0.1;
        (min_x - padding, max_x + padding)
    }

    /// Get the combined Y range of all series
    pub fn combined_y_range(&self) -> (f64, f64) {
        if let Some(range) = self.y_range {
            return range;
        }

        if self.series.is_empty() {
            return (0.0, 1.0);
        }

        let mut min_y = f64::INFINITY;
        let mut max_y = f64::NEG_INFINITY;

        for series in &self.series {
            let (series_min, series_max) = series.y_range();
            min_y = min_y.min(series_min);
            max_y = max_y.max(series_max);
        }

        // Add some padding
        let range = max_y - min_y;
        let padding = range * 0.1;
        (min_y - padding, max_y + padding)
    }
}

impl Default for LineChart {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for creating line charts
pub struct LineChartBuilder {
    chart: LineChart,
}

impl LineChartBuilder {
    /// Create a new line chart builder
    pub fn new() -> Self {
        Self {
            chart: LineChart::new(),
        }
    }

    /// Set chart title
    pub fn title<S: Into<String>>(mut self, title: S) -> Self {
        self.chart.title = title.into();
        self
    }

    /// Add a data series
    pub fn add_series(mut self, series: DataSeries) -> Self {
        self.chart.series.push(series);
        self
    }

    /// Set axis labels
    pub fn axis_labels<S: Into<String>>(mut self, x_label: S, y_label: S) -> Self {
        self.chart.x_axis_label = x_label.into();
        self.chart.y_axis_label = y_label.into();
        self
    }

    /// Set fonts
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

    /// Set axis font
    pub fn axis_font(mut self, font: Font, size: f64) -> Self {
        self.chart.axis_font = font;
        self.chart.axis_font_size = size;
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

    /// Configure grid
    pub fn grid(mut self, show: bool, color: Color, lines: usize) -> Self {
        self.chart.show_grid = show;
        self.chart.grid_color = color;
        self.chart.grid_lines = lines;
        self
    }

    /// Set axis ranges
    pub fn x_range(mut self, min: f64, max: f64) -> Self {
        self.chart.x_range = Some((min, max));
        self
    }

    /// Set Y axis range
    pub fn y_range(mut self, min: f64, max: f64) -> Self {
        self.chart.y_range = Some((min, max));
        self
    }

    /// Add a simple series from Y values
    pub fn add_simple_series<S: Into<String>>(
        mut self,
        name: S,
        values: Vec<f64>,
        color: Color,
    ) -> Self {
        let series = DataSeries::new(name, color).y_data(values);
        self.chart.series.push(series);
        self
    }

    /// Build the final line chart
    pub fn build(self) -> LineChart {
        self.chart
    }
}

impl Default for LineChartBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_data_series_creation() {
        let series = DataSeries::new("Test Series", Color::blue()).y_data(vec![1.0, 2.0, 3.0]);

        assert_eq!(series.name, "Test Series");
        assert_eq!(series.color, Color::blue());
        assert_eq!(series.data.len(), 3);
        assert_eq!(series.data[0], (0.0, 1.0));
        assert_eq!(series.data[2], (2.0, 3.0));
    }

    #[test]
    fn test_data_series_ranges() {
        let series = DataSeries::new("Test", Color::red()).xy_data(vec![
            (0.0, 10.0),
            (5.0, 20.0),
            (10.0, 5.0),
        ]);

        let (min_x, max_x) = series.x_range();
        let (min_y, max_y) = series.y_range();

        assert_eq!(min_x, 0.0);
        assert_eq!(max_x, 10.0);
        assert_eq!(min_y, 5.0);
        assert_eq!(max_y, 20.0);
    }

    #[test]
    fn test_line_chart_creation() {
        let chart = LineChartBuilder::new()
            .title("Test Chart")
            .add_simple_series("Series 1", vec![1.0, 2.0, 3.0], Color::blue())
            .add_simple_series("Series 2", vec![3.0, 2.0, 1.0], Color::red())
            .build();

        assert_eq!(chart.title, "Test Chart");
        assert_eq!(chart.series.len(), 2);

        let (min_y, max_y) = chart.combined_y_range();
        assert!(min_y <= 1.0);
        assert!(max_y >= 3.0);
    }
}
