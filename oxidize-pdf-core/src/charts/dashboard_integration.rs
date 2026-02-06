//! Dashboard Integration for Charts
//!
//! This module provides wrappers that allow chart components (BarChart, PieChart, LineChart)
//! to be used within the dashboard framework by implementing the DashboardComponent trait.

use super::{BarChart, LineChart, PieChart};
use crate::dashboard::{ComponentPosition, ComponentSpan, DashboardComponent, DashboardTheme};
use crate::error::PdfError;
use crate::page::Page;

/// Wrapper for BarChart that implements DashboardComponent
#[derive(Debug, Clone)]
pub struct DashboardBarChart {
    chart: BarChart,
    span: ComponentSpan,
}

impl DashboardBarChart {
    /// Create a new dashboard bar chart
    pub fn new(chart: BarChart) -> Self {
        Self {
            chart,
            span: ComponentSpan::new(6), // Half-width by default
        }
    }

    /// Set the column span
    pub fn span(mut self, columns: u8) -> Self {
        self.span = ComponentSpan::new(columns);
        self
    }

    /// Get a reference to the underlying chart
    pub fn chart(&self) -> &BarChart {
        &self.chart
    }

    /// Get a mutable reference to the underlying chart
    pub fn chart_mut(&mut self) -> &mut BarChart {
        &mut self.chart
    }
}

impl DashboardComponent for DashboardBarChart {
    fn render(
        &self,
        page: &mut Page,
        position: ComponentPosition,
        _theme: &DashboardTheme,
    ) -> Result<(), PdfError> {
        // Apply padding to the position
        let padded = position.with_padding(10.0);

        // Use the chart's existing rendering via ChartExt
        use crate::charts::ChartExt;
        page.add_bar_chart(&self.chart, padded.x, padded.y, padded.width, padded.height)
    }

    fn get_span(&self) -> ComponentSpan {
        self.span
    }

    fn set_span(&mut self, span: ComponentSpan) {
        self.span = span;
    }

    fn preferred_height(&self, _available_width: f64) -> f64 {
        250.0 // Reasonable default for bar charts
    }

    fn minimum_width(&self) -> f64 {
        200.0
    }

    fn estimated_render_time_ms(&self) -> u32 {
        20 + (self.chart.data.len() as u32 * 2) // Base + bars
    }

    fn estimated_memory_mb(&self) -> f64 {
        0.2 + (self.chart.data.len() as f64 * 0.01)
    }

    fn complexity_score(&self) -> u8 {
        let base_score = 30;
        let data_complexity = (self.chart.data.len() / 5).min(20) as u8;
        let feature_score =
            if self.chart.show_grid { 10 } else { 0 } + if self.chart.show_values { 5 } else { 0 };

        (base_score + data_complexity + feature_score).min(100)
    }

    fn component_type(&self) -> &'static str {
        "BarChart"
    }
}

/// Wrapper for PieChart that implements DashboardComponent
#[derive(Debug, Clone)]
pub struct DashboardPieChart {
    chart: PieChart,
    span: ComponentSpan,
}

impl DashboardPieChart {
    /// Create a new dashboard pie chart
    pub fn new(chart: PieChart) -> Self {
        Self {
            chart,
            span: ComponentSpan::new(6), // Half-width by default
        }
    }

    /// Set the column span
    pub fn span(mut self, columns: u8) -> Self {
        self.span = ComponentSpan::new(columns);
        self
    }

    /// Get a reference to the underlying chart
    pub fn chart(&self) -> &PieChart {
        &self.chart
    }

    /// Get a mutable reference to the underlying chart
    pub fn chart_mut(&mut self) -> &mut PieChart {
        &mut self.chart
    }
}

impl DashboardComponent for DashboardPieChart {
    fn render(
        &self,
        page: &mut Page,
        position: ComponentPosition,
        _theme: &DashboardTheme,
    ) -> Result<(), PdfError> {
        // Calculate center position and radius based on available space
        let padded = position.with_padding(20.0);
        let radius = (padded.width.min(padded.height) / 2.0) - 20.0;
        let center_x = padded.x + padded.width / 2.0;
        let center_y = padded.y + padded.height / 2.0;

        // Use the chart's existing rendering via ChartExt
        use crate::charts::ChartExt;
        page.add_pie_chart(&self.chart, center_x, center_y, radius)
    }

    fn get_span(&self) -> ComponentSpan {
        self.span
    }

    fn set_span(&mut self, span: ComponentSpan) {
        self.span = span;
    }

    fn preferred_height(&self, _available_width: f64) -> f64 {
        250.0 // Square aspect for pie charts
    }

    fn minimum_width(&self) -> f64 {
        200.0
    }

    fn estimated_render_time_ms(&self) -> u32 {
        25 + (self.chart.segments.len() as u32 * 3) // Segments are more complex
    }

    fn estimated_memory_mb(&self) -> f64 {
        0.15 + (self.chart.segments.len() as f64 * 0.02)
    }

    fn complexity_score(&self) -> u8 {
        let base_score = 35; // Pie charts are slightly more complex than bar charts
        let segment_complexity = (self.chart.segments.len() / 3).min(25) as u8;
        let feature_score = if self.chart.show_percentages { 5 } else { 0 };

        (base_score + segment_complexity + feature_score).min(100)
    }

    fn component_type(&self) -> &'static str {
        "PieChart"
    }
}

/// Wrapper for LineChart that implements DashboardComponent
#[derive(Debug, Clone)]
pub struct DashboardLineChart {
    chart: LineChart,
    span: ComponentSpan,
}

impl DashboardLineChart {
    /// Create a new dashboard line chart
    pub fn new(chart: LineChart) -> Self {
        Self {
            chart,
            span: ComponentSpan::new(6), // Half-width by default
        }
    }

    /// Set the column span
    pub fn span(mut self, columns: u8) -> Self {
        self.span = ComponentSpan::new(columns);
        self
    }

    /// Get a reference to the underlying chart
    pub fn chart(&self) -> &LineChart {
        &self.chart
    }

    /// Get a mutable reference to the underlying chart
    pub fn chart_mut(&mut self) -> &mut LineChart {
        &mut self.chart
    }
}

impl DashboardComponent for DashboardLineChart {
    fn render(
        &self,
        page: &mut Page,
        position: ComponentPosition,
        _theme: &DashboardTheme,
    ) -> Result<(), PdfError> {
        // Apply padding to the position
        let padded = position.with_padding(10.0);

        // Use the chart's existing rendering via ChartExt
        use crate::charts::ChartExt;
        page.add_line_chart(&self.chart, padded.x, padded.y, padded.width, padded.height)
    }

    fn get_span(&self) -> ComponentSpan {
        self.span
    }

    fn set_span(&mut self, span: ComponentSpan) {
        self.span = span;
    }

    fn preferred_height(&self, _available_width: f64) -> f64 {
        220.0 // Reasonable default for line charts
    }

    fn minimum_width(&self) -> f64 {
        250.0 // Line charts need more width for x-axis
    }

    fn estimated_render_time_ms(&self) -> u32 {
        let total_points: u32 = self.chart.series.iter().map(|s| s.data.len() as u32).sum();
        30 + (total_points * 2) // Base + data points
    }

    fn estimated_memory_mb(&self) -> f64 {
        let total_points = self
            .chart
            .series
            .iter()
            .map(|s| s.data.len())
            .sum::<usize>();
        0.25 + (total_points as f64 * 0.01)
    }

    fn complexity_score(&self) -> u8 {
        let base_score = 40; // Line charts are more complex
        let series_complexity = (self.chart.series.len() * 10).min(30) as u8;
        let feature_score = if self.chart.show_grid { 10 } else { 0 };

        (base_score + series_complexity + feature_score).min(100)
    }

    fn component_type(&self) -> &'static str {
        "LineChart"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::charts::{BarChartBuilder, DataSeries, LineChartBuilder, PieChartBuilder};
    use crate::graphics::Color;

    // ==================== DashboardBarChart Tests ====================

    #[test]
    fn test_dashboard_bar_chart_creation() {
        let chart = BarChartBuilder::new()
            .simple_data(vec![10.0, 20.0, 30.0])
            .build();

        let dashboard_chart = DashboardBarChart::new(chart);
        assert_eq!(dashboard_chart.get_span().columns, 6);
        assert_eq!(dashboard_chart.component_type(), "BarChart");
    }

    #[test]
    fn test_dashboard_bar_chart_span() {
        let chart = BarChartBuilder::new().simple_data(vec![10.0, 20.0]).build();

        let dashboard_chart = DashboardBarChart::new(chart).span(12);
        assert_eq!(dashboard_chart.get_span().columns, 12);
        assert!(dashboard_chart.get_span().is_full_width());
    }

    #[test]
    fn test_dashboard_bar_chart_chart_accessor() {
        let chart = BarChartBuilder::new()
            .simple_data(vec![1.0, 2.0, 3.0])
            .build();
        let dashboard_chart = DashboardBarChart::new(chart);

        assert_eq!(dashboard_chart.chart().data.len(), 3);
    }

    #[test]
    fn test_dashboard_bar_chart_chart_mut() {
        let chart = BarChartBuilder::new().simple_data(vec![1.0, 2.0]).build();
        let mut dashboard_chart = DashboardBarChart::new(chart);

        dashboard_chart.chart_mut().show_grid = true;
        assert!(dashboard_chart.chart().show_grid);
    }

    #[test]
    fn test_dashboard_bar_chart_set_span() {
        let chart = BarChartBuilder::new().simple_data(vec![10.0]).build();
        let mut dashboard_chart = DashboardBarChart::new(chart);

        dashboard_chart.set_span(ComponentSpan::new(4));
        assert_eq!(dashboard_chart.get_span().columns, 4);
    }

    #[test]
    fn test_dashboard_bar_chart_preferred_height() {
        let chart = BarChartBuilder::new().simple_data(vec![10.0]).build();
        let dashboard_chart = DashboardBarChart::new(chart);

        assert_eq!(dashboard_chart.preferred_height(500.0), 250.0);
    }

    #[test]
    fn test_dashboard_bar_chart_minimum_width() {
        let chart = BarChartBuilder::new().simple_data(vec![10.0]).build();
        let dashboard_chart = DashboardBarChart::new(chart);

        assert_eq!(dashboard_chart.minimum_width(), 200.0);
    }

    #[test]
    fn test_dashboard_bar_chart_estimated_render_time() {
        let chart = BarChartBuilder::new()
            .simple_data(vec![10.0, 20.0, 30.0, 40.0, 50.0])
            .build();
        let dashboard_chart = DashboardBarChart::new(chart);

        // 20 base + (5 bars * 2) = 30
        assert_eq!(dashboard_chart.estimated_render_time_ms(), 30);
    }

    #[test]
    fn test_dashboard_bar_chart_estimated_memory() {
        let chart = BarChartBuilder::new()
            .simple_data(vec![10.0, 20.0, 30.0])
            .build();
        let dashboard_chart = DashboardBarChart::new(chart);

        // 0.2 base + (3 * 0.01) = 0.23
        let expected = 0.2 + (3.0 * 0.01);
        assert!((dashboard_chart.estimated_memory_mb() - expected).abs() < 0.001);
    }

    #[test]
    fn test_dashboard_bar_chart_clone() {
        let chart = BarChartBuilder::new().simple_data(vec![10.0]).build();
        let dashboard_chart = DashboardBarChart::new(chart).span(8);
        let cloned = dashboard_chart.clone();

        assert_eq!(cloned.get_span().columns, 8);
    }

    #[test]
    fn test_dashboard_bar_chart_debug() {
        let chart = BarChartBuilder::new().simple_data(vec![10.0]).build();
        let dashboard_chart = DashboardBarChart::new(chart);

        let debug_str = format!("{:?}", dashboard_chart);
        assert!(debug_str.contains("DashboardBarChart"));
    }

    // ==================== DashboardPieChart Tests ====================

    #[test]
    fn test_dashboard_pie_chart_creation() {
        let chart = PieChartBuilder::new()
            .simple_data(vec![25.0, 35.0, 40.0])
            .build();

        let dashboard_chart = DashboardPieChart::new(chart);
        assert_eq!(dashboard_chart.component_type(), "PieChart");
        assert!(dashboard_chart.complexity_score() > 30);
    }

    #[test]
    fn test_dashboard_pie_chart_span() {
        let chart = PieChartBuilder::new().simple_data(vec![50.0, 50.0]).build();
        let dashboard_chart = DashboardPieChart::new(chart).span(4);

        assert_eq!(dashboard_chart.get_span().columns, 4);
    }

    #[test]
    fn test_dashboard_pie_chart_chart_accessor() {
        let chart = PieChartBuilder::new()
            .simple_data(vec![20.0, 30.0, 50.0])
            .build();
        let dashboard_chart = DashboardPieChart::new(chart);

        assert_eq!(dashboard_chart.chart().segments.len(), 3);
    }

    #[test]
    fn test_dashboard_pie_chart_chart_mut() {
        let chart = PieChartBuilder::new().simple_data(vec![50.0, 50.0]).build();
        let mut dashboard_chart = DashboardPieChart::new(chart);

        dashboard_chart.chart_mut().show_percentages = true;
        assert!(dashboard_chart.chart().show_percentages);
    }

    #[test]
    fn test_dashboard_pie_chart_set_span() {
        let chart = PieChartBuilder::new().simple_data(vec![100.0]).build();
        let mut dashboard_chart = DashboardPieChart::new(chart);

        dashboard_chart.set_span(ComponentSpan::new(3));
        assert_eq!(dashboard_chart.get_span().columns, 3);
    }

    #[test]
    fn test_dashboard_pie_chart_preferred_height() {
        let chart = PieChartBuilder::new().simple_data(vec![50.0, 50.0]).build();
        let dashboard_chart = DashboardPieChart::new(chart);

        assert_eq!(dashboard_chart.preferred_height(400.0), 250.0);
    }

    #[test]
    fn test_dashboard_pie_chart_minimum_width() {
        let chart = PieChartBuilder::new().simple_data(vec![100.0]).build();
        let dashboard_chart = DashboardPieChart::new(chart);

        assert_eq!(dashboard_chart.minimum_width(), 200.0);
    }

    #[test]
    fn test_dashboard_pie_chart_estimated_render_time() {
        let chart = PieChartBuilder::new()
            .simple_data(vec![25.0, 25.0, 25.0, 25.0])
            .build();
        let dashboard_chart = DashboardPieChart::new(chart);

        // 25 base + (4 segments * 3) = 37
        assert_eq!(dashboard_chart.estimated_render_time_ms(), 37);
    }

    #[test]
    fn test_dashboard_pie_chart_estimated_memory() {
        let chart = PieChartBuilder::new().simple_data(vec![50.0, 50.0]).build();
        let dashboard_chart = DashboardPieChart::new(chart);

        // 0.15 base + (2 * 0.02) = 0.19
        let expected = 0.15 + (2.0 * 0.02);
        assert!((dashboard_chart.estimated_memory_mb() - expected).abs() < 0.001);
    }

    #[test]
    fn test_dashboard_pie_chart_complexity_with_percentages() {
        let chart = PieChartBuilder::new()
            .simple_data(vec![50.0, 50.0])
            .show_percentages(true)
            .build();
        let dashboard_chart = DashboardPieChart::new(chart);

        // Should include feature_score of 5 for percentages
        assert!(dashboard_chart.complexity_score() >= 35);
    }

    #[test]
    fn test_dashboard_pie_chart_clone() {
        let chart = PieChartBuilder::new().simple_data(vec![100.0]).build();
        let dashboard_chart = DashboardPieChart::new(chart).span(5);
        let cloned = dashboard_chart.clone();

        assert_eq!(cloned.get_span().columns, 5);
    }

    #[test]
    fn test_dashboard_pie_chart_debug() {
        let chart = PieChartBuilder::new().simple_data(vec![100.0]).build();
        let dashboard_chart = DashboardPieChart::new(chart);

        let debug_str = format!("{:?}", dashboard_chart);
        assert!(debug_str.contains("DashboardPieChart"));
    }

    // ==================== DashboardLineChart Tests ====================

    #[test]
    fn test_dashboard_line_chart_creation() {
        let series = DataSeries::new("Series 1", Color::blue()).xy_data(vec![
            (0.0, 10.0),
            (1.0, 20.0),
            (2.0, 15.0),
        ]);
        let chart = LineChartBuilder::new()
            .title("Test Line Chart")
            .add_series(series)
            .build();

        let dashboard_chart = DashboardLineChart::new(chart);
        assert_eq!(dashboard_chart.component_type(), "LineChart");
        assert_eq!(dashboard_chart.preferred_height(500.0), 220.0);
    }

    #[test]
    fn test_dashboard_line_chart_span() {
        let series = DataSeries::new("Series", Color::red()).xy_data(vec![(0.0, 5.0)]);
        let chart = LineChartBuilder::new().add_series(series).build();
        let dashboard_chart = DashboardLineChart::new(chart).span(10);

        assert_eq!(dashboard_chart.get_span().columns, 10);
    }

    #[test]
    fn test_dashboard_line_chart_chart_accessor() {
        let series = DataSeries::new("Test", Color::green()).xy_data(vec![(0.0, 1.0), (1.0, 2.0)]);
        let chart = LineChartBuilder::new().add_series(series).build();
        let dashboard_chart = DashboardLineChart::new(chart);

        assert_eq!(dashboard_chart.chart().series.len(), 1);
    }

    #[test]
    fn test_dashboard_line_chart_chart_mut() {
        let series = DataSeries::new("Test", Color::blue()).xy_data(vec![(0.0, 1.0)]);
        let chart = LineChartBuilder::new().add_series(series).build();
        let mut dashboard_chart = DashboardLineChart::new(chart);

        dashboard_chart.chart_mut().show_grid = true;
        assert!(dashboard_chart.chart().show_grid);
    }

    #[test]
    fn test_dashboard_line_chart_set_span() {
        let series = DataSeries::new("Test", Color::blue()).xy_data(vec![(0.0, 1.0)]);
        let chart = LineChartBuilder::new().add_series(series).build();
        let mut dashboard_chart = DashboardLineChart::new(chart);

        dashboard_chart.set_span(ComponentSpan::new(8));
        assert_eq!(dashboard_chart.get_span().columns, 8);
    }

    #[test]
    fn test_dashboard_line_chart_minimum_width() {
        let series = DataSeries::new("Test", Color::blue()).xy_data(vec![(0.0, 1.0)]);
        let chart = LineChartBuilder::new().add_series(series).build();
        let dashboard_chart = DashboardLineChart::new(chart);

        assert_eq!(dashboard_chart.minimum_width(), 250.0);
    }

    #[test]
    fn test_dashboard_line_chart_estimated_render_time() {
        let series1 =
            DataSeries::new("S1", Color::blue()).xy_data(vec![(0.0, 1.0), (1.0, 2.0), (2.0, 3.0)]);
        let series2 = DataSeries::new("S2", Color::red()).xy_data(vec![(0.0, 2.0), (1.0, 1.0)]);
        let chart = LineChartBuilder::new()
            .add_series(series1)
            .add_series(series2)
            .build();
        let dashboard_chart = DashboardLineChart::new(chart);

        // 30 base + (5 points * 2) = 40
        assert_eq!(dashboard_chart.estimated_render_time_ms(), 40);
    }

    #[test]
    fn test_dashboard_line_chart_estimated_memory() {
        let series = DataSeries::new("Test", Color::blue()).xy_data(vec![
            (0.0, 1.0),
            (1.0, 2.0),
            (2.0, 3.0),
            (3.0, 4.0),
        ]);
        let chart = LineChartBuilder::new().add_series(series).build();
        let dashboard_chart = DashboardLineChart::new(chart);

        // 0.25 base + (4 points * 0.01) = 0.29
        let expected = 0.25 + (4.0 * 0.01);
        assert!((dashboard_chart.estimated_memory_mb() - expected).abs() < 0.001);
    }

    #[test]
    fn test_dashboard_line_chart_complexity_with_grid() {
        let series = DataSeries::new("Test", Color::blue()).xy_data(vec![(0.0, 1.0)]);
        let chart = LineChartBuilder::new()
            .add_series(series)
            .grid(true, Color::gray(0.8), 10)
            .build();
        let dashboard_chart = DashboardLineChart::new(chart);

        // 40 base + 10 series complexity + 10 grid = at least 50
        assert!(dashboard_chart.complexity_score() >= 50);
    }

    #[test]
    fn test_dashboard_line_chart_complexity_multiple_series() {
        let series1 = DataSeries::new("S1", Color::blue()).xy_data(vec![(0.0, 1.0)]);
        let series2 = DataSeries::new("S2", Color::red()).xy_data(vec![(0.0, 2.0)]);
        let series3 = DataSeries::new("S3", Color::green()).xy_data(vec![(0.0, 3.0)]);
        let chart = LineChartBuilder::new()
            .add_series(series1)
            .add_series(series2)
            .add_series(series3)
            .build();
        let dashboard_chart = DashboardLineChart::new(chart);

        // More series = higher complexity
        assert!(dashboard_chart.complexity_score() >= 40);
    }

    #[test]
    fn test_dashboard_line_chart_clone() {
        let series = DataSeries::new("Test", Color::blue()).xy_data(vec![(0.0, 1.0)]);
        let chart = LineChartBuilder::new().add_series(series).build();
        let dashboard_chart = DashboardLineChart::new(chart).span(7);
        let cloned = dashboard_chart.clone();

        assert_eq!(cloned.get_span().columns, 7);
    }

    #[test]
    fn test_dashboard_line_chart_debug() {
        let series = DataSeries::new("Test", Color::blue()).xy_data(vec![(0.0, 1.0)]);
        let chart = LineChartBuilder::new().add_series(series).build();
        let dashboard_chart = DashboardLineChart::new(chart);

        let debug_str = format!("{:?}", dashboard_chart);
        assert!(debug_str.contains("DashboardLineChart"));
    }

    // ==================== Comparison Tests ====================

    #[test]
    fn test_complexity_scores() {
        // Bar chart with many bars should have higher complexity
        let simple_bar = BarChartBuilder::new().simple_data(vec![10.0, 20.0]).build();
        let complex_bar = BarChartBuilder::new()
            .simple_data(vec![10.0, 20.0, 30.0, 40.0, 50.0, 60.0, 70.0, 80.0])
            .show_grid(true)
            .show_values(true)
            .build();

        let simple_dashboard = DashboardBarChart::new(simple_bar);
        let complex_dashboard = DashboardBarChart::new(complex_bar);

        assert!(complex_dashboard.complexity_score() > simple_dashboard.complexity_score());
    }

    #[test]
    fn test_all_chart_types_component_types() {
        let bar = DashboardBarChart::new(BarChartBuilder::new().simple_data(vec![1.0]).build());
        let pie = DashboardPieChart::new(PieChartBuilder::new().simple_data(vec![1.0]).build());
        let series = DataSeries::new("T", Color::black()).xy_data(vec![(0.0, 1.0)]);
        let line = DashboardLineChart::new(LineChartBuilder::new().add_series(series).build());

        assert_eq!(bar.component_type(), "BarChart");
        assert_eq!(pie.component_type(), "PieChart");
        assert_eq!(line.component_type(), "LineChart");
    }

    #[test]
    fn test_empty_data_estimated_values() {
        // Even with minimal data, estimates should be reasonable
        let bar = DashboardBarChart::new(BarChartBuilder::new().simple_data(vec![]).build());
        let pie = DashboardPieChart::new(PieChartBuilder::new().simple_data(vec![]).build());

        assert!(bar.estimated_render_time_ms() >= 20); // Base time
        assert!(pie.estimated_render_time_ms() >= 25); // Base time
        assert!(bar.estimated_memory_mb() >= 0.2); // Base memory
        assert!(pie.estimated_memory_mb() >= 0.15); // Base memory
    }
}
