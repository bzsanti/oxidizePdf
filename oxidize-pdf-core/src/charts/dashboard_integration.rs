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
    fn test_dashboard_pie_chart_creation() {
        let chart = PieChartBuilder::new()
            .simple_data(vec![25.0, 35.0, 40.0])
            .build();

        let dashboard_chart = DashboardPieChart::new(chart);
        assert_eq!(dashboard_chart.component_type(), "PieChart");
        assert!(dashboard_chart.complexity_score() > 30);
    }

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
}
