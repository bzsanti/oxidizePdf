//! Dashboard Templates - Pre-built Dashboard Layouts
//!
//! This module provides ready-to-use dashboard templates for common use cases.
//! Templates provide a quick way to create professional dashboards by simply
//! providing data, without needing to configure layout and components manually.
//!
//! # Examples
//!
//! ```rust,ignore
//! use oxidize_pdf::dashboard::templates::{SalesDashboardTemplate, TemplateData};
//!
//! let data = TemplateData::new()
//!     .with_kpi("revenue", "$2.5M", 12.5)
//!     .with_kpi("orders", "1,247", 8.3)
//!     .with_chart_data("monthly_sales", vec![100.0, 150.0, 200.0]);
//!
//! let dashboard = SalesDashboardTemplate::new()
//!     .title("Q4 2024 Sales Report")
//!     .build(data)?;
//! ```

use super::{
    Dashboard, DashboardBuilder, HeatMap, HeatMapData, KpiCard, KpiCardBuilder, PivotTable,
    TrendDirection,
};
use crate::charts::{
    BarChartBuilder, DashboardBarChart, DashboardLineChart, DashboardPieChart, DataSeries,
    LineChartBuilder, PieChartBuilder, PieSegment,
};
use crate::error::PdfError;
use crate::graphics::Color;
use std::collections::HashMap;

/// Template data container for building dashboards
#[derive(Debug, Clone, Default)]
pub struct TemplateData {
    /// KPI metrics with name, value, and trend
    pub kpis: Vec<KpiData>,
    /// Chart data series
    pub charts: HashMap<String, ChartData>,
    /// Tabular data for pivot tables
    pub tables: HashMap<String, Vec<HashMap<String, String>>>,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

/// KPI metric data
#[derive(Debug, Clone)]
pub struct KpiData {
    pub name: String,
    pub value: String,
    pub subtitle: Option<String>,
    pub trend_value: Option<f64>,
    pub trend_direction: Option<TrendDirection>,
    pub color: Option<Color>,
    pub sparkline: Option<Vec<f64>>,
}

/// Chart data for templates
#[derive(Debug, Clone)]
pub enum ChartData {
    /// Bar chart data with labels
    Bar {
        labels: Vec<String>,
        values: Vec<f64>,
        colors: Option<Vec<Color>>,
    },
    /// Line chart with multiple series
    Line { series: Vec<SeriesData> },
    /// Pie chart segments
    Pie { segments: Vec<PieSegmentData> },
    /// Heatmap data
    HeatMap {
        values: Vec<Vec<f64>>,
        row_labels: Vec<String>,
        column_labels: Vec<String>,
    },
}

/// Line chart series data
#[derive(Debug, Clone)]
pub struct SeriesData {
    pub name: String,
    pub data: Vec<(f64, f64)>,
    pub color: Color,
}

/// Pie chart segment data
#[derive(Debug, Clone)]
pub struct PieSegmentData {
    pub label: String,
    pub value: f64,
    pub color: Color,
}

impl TemplateData {
    /// Create new empty template data
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a KPI metric
    pub fn with_kpi(
        mut self,
        name: impl Into<String>,
        value: impl Into<String>,
        trend: f64,
    ) -> Self {
        self.kpis.push(KpiData {
            name: name.into(),
            value: value.into(),
            subtitle: None,
            trend_value: Some(trend),
            trend_direction: Some(if trend >= 0.0 {
                TrendDirection::Up
            } else {
                TrendDirection::Down
            }),
            color: None,
            sparkline: None,
        });
        self
    }

    /// Add KPI with full configuration
    pub fn add_kpi(mut self, kpi: KpiData) -> Self {
        self.kpis.push(kpi);
        self
    }

    /// Add chart data
    pub fn with_chart(mut self, name: impl Into<String>, chart_data: ChartData) -> Self {
        self.charts.insert(name.into(), chart_data);
        self
    }

    /// Add table data
    pub fn with_table(
        mut self,
        name: impl Into<String>,
        data: Vec<HashMap<String, String>>,
    ) -> Self {
        self.tables.insert(name.into(), data);
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

/// Sales Dashboard Template
///
/// Pre-configured dashboard for sales reporting with:
/// - KPI cards for key metrics
/// - Monthly sales trend chart
/// - Product/category breakdown
/// - Regional performance heatmap
#[derive(Debug, Clone)]
pub struct SalesDashboardTemplate {
    title: String,
    subtitle: Option<String>,
    theme: String,
}

impl SalesDashboardTemplate {
    /// Create new sales dashboard template
    pub fn new() -> Self {
        Self {
            title: "Sales Dashboard".to_string(),
            subtitle: None,
            theme: "corporate".to_string(),
        }
    }

    /// Set dashboard title
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    /// Set dashboard subtitle
    pub fn subtitle(mut self, subtitle: impl Into<String>) -> Self {
        self.subtitle = Some(subtitle.into());
        self
    }

    /// Set theme
    pub fn theme(mut self, theme: impl Into<String>) -> Self {
        self.theme = theme.into();
        self
    }

    /// Build dashboard from template data
    pub fn build(self, data: TemplateData) -> Result<Dashboard, PdfError> {
        let mut builder = DashboardBuilder::new()
            .title(self.title)
            .theme_by_name(&self.theme);

        if let Some(subtitle) = self.subtitle {
            builder = builder.subtitle(subtitle);
        }

        // Add KPI cards
        if !data.kpis.is_empty() {
            let kpi_cards: Vec<KpiCard> = data
                .kpis
                .into_iter()
                .map(|kpi| {
                    let mut card_builder = KpiCardBuilder::new(&kpi.name, &kpi.value);

                    if let Some(subtitle) = kpi.subtitle {
                        card_builder = card_builder.subtitle(&subtitle);
                    }

                    if let (Some(trend_value), Some(trend_dir)) =
                        (kpi.trend_value, kpi.trend_direction)
                    {
                        card_builder = card_builder.trend(trend_value, trend_dir);
                    }

                    if let Some(color) = kpi.color {
                        card_builder = card_builder.color(color);
                    }

                    if let Some(sparkline) = kpi.sparkline {
                        card_builder = card_builder.sparkline(sparkline);
                    }

                    card_builder.build()
                })
                .collect();

            builder = builder.add_kpi_row(kpi_cards);
        }

        // Add charts
        builder = builder.start_row();

        // Monthly sales chart (bar or line)
        if let Some(ChartData::Bar {
            labels,
            values,
            colors,
        }) = data.charts.get("monthly_sales")
        {
            let labeled_values: Vec<(&str, f64)> = labels
                .iter()
                .map(|s| s.as_str())
                .zip(values.iter().copied())
                .collect();

            let mut chart_builder = BarChartBuilder::new()
                .title("Monthly Sales")
                .labeled_data(labeled_values)
                .show_grid(true)
                .show_values(true);

            if let Some(color_list) = colors {
                chart_builder = chart_builder.colors(color_list.clone());
            }

            let chart = chart_builder.build();
            builder = builder.add_to_row(Box::new(DashboardBarChart::new(chart).span(6)));
        }

        // Product breakdown (pie chart)
        if let Some(ChartData::Pie { segments }) = data.charts.get("product_breakdown") {
            let mut chart_builder = PieChartBuilder::new().title("Product Distribution");

            for segment in segments {
                chart_builder = chart_builder.add_segment(PieSegment::new(
                    &segment.label,
                    segment.value,
                    segment.color,
                ));
            }

            let chart = chart_builder.show_percentages(true).build();
            builder = builder.add_to_row(Box::new(DashboardPieChart::new(chart).span(6)));
        }

        builder = builder.finish_row();

        // Regional heatmap
        if let Some(ChartData::HeatMap {
            values,
            row_labels,
            column_labels,
        }) = data.charts.get("regional_performance")
        {
            let heatmap_data = HeatMapData {
                values: values.clone(),
                row_labels: row_labels.clone(),
                column_labels: column_labels.clone(),
            };
            builder = builder.add_component(Box::new(HeatMap::new(heatmap_data)));
        }

        // Data table
        if let Some(table_data) = data.tables.get("sales_detail") {
            let pivot = PivotTable::new(table_data.clone());
            builder = builder.add_component(Box::new(pivot));
        }

        builder.build()
    }
}

impl Default for SalesDashboardTemplate {
    fn default() -> Self {
        Self::new()
    }
}

/// Financial Report Template
///
/// Designed for financial reporting with:
/// - Financial KPIs (revenue, profit, margins)
/// - Revenue trend over time
/// - Expense breakdown
/// - Financial ratios table
#[derive(Debug, Clone)]
pub struct FinancialReportTemplate {
    title: String,
    subtitle: Option<String>,
    theme: String,
}

impl FinancialReportTemplate {
    /// Create new financial report template
    pub fn new() -> Self {
        Self {
            title: "Financial Report".to_string(),
            subtitle: None,
            theme: "corporate".to_string(),
        }
    }

    /// Set dashboard title
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    /// Set dashboard subtitle
    pub fn subtitle(mut self, subtitle: impl Into<String>) -> Self {
        self.subtitle = Some(subtitle.into());
        self
    }

    /// Set theme
    pub fn theme(mut self, theme: impl Into<String>) -> Self {
        self.theme = theme.into();
        self
    }

    /// Build dashboard from template data
    pub fn build(self, data: TemplateData) -> Result<Dashboard, PdfError> {
        let mut builder = DashboardBuilder::new()
            .title(self.title)
            .theme_by_name(&self.theme);

        if let Some(subtitle) = self.subtitle {
            builder = builder.subtitle(subtitle);
        }

        // Add financial KPIs
        if !data.kpis.is_empty() {
            let kpi_cards: Vec<KpiCard> = data
                .kpis
                .into_iter()
                .map(|kpi| {
                    let mut card_builder = KpiCardBuilder::new(&kpi.name, &kpi.value);

                    if let Some(subtitle) = kpi.subtitle {
                        card_builder = card_builder.subtitle(&subtitle);
                    }

                    if let (Some(trend_value), Some(trend_dir)) =
                        (kpi.trend_value, kpi.trend_direction)
                    {
                        card_builder = card_builder.trend(trend_value, trend_dir);
                    }

                    if let Some(color) = kpi.color {
                        card_builder = card_builder.color(color);
                    }

                    card_builder.build()
                })
                .collect();

            builder = builder.add_kpi_row(kpi_cards);
        }

        // Revenue trend (line chart)
        if let Some(ChartData::Line { series }) = data.charts.get("revenue_trend") {
            let mut chart_builder = LineChartBuilder::new()
                .title("Revenue Trend")
                .axis_labels("Period", "Revenue")
                .grid(true, Color::gray(0.8), 5);

            for series_data in series {
                let data_series = DataSeries::new(&series_data.name, series_data.color)
                    .xy_data(series_data.data.clone());
                chart_builder = chart_builder.add_series(data_series);
            }

            let chart = chart_builder.build();
            builder = builder.add_component(Box::new(DashboardLineChart::new(chart).span(12)));
        }

        // Expense breakdown
        builder = builder.start_row();

        if let Some(ChartData::Pie { segments }) = data.charts.get("expense_breakdown") {
            let mut chart_builder = PieChartBuilder::new().title("Expense Breakdown");

            for segment in segments {
                chart_builder = chart_builder.add_segment(PieSegment::new(
                    &segment.label,
                    segment.value,
                    segment.color,
                ));
            }

            let chart = chart_builder.show_percentages(true).build();
            builder = builder.add_to_row(Box::new(DashboardPieChart::new(chart).span(6)));
        }

        // Cost structure
        if let Some(ChartData::Bar {
            labels,
            values,
            colors,
        }) = data.charts.get("cost_structure")
        {
            let labeled_values: Vec<(&str, f64)> = labels
                .iter()
                .map(|s| s.as_str())
                .zip(values.iter().copied())
                .collect();

            let mut chart_builder = BarChartBuilder::new()
                .title("Cost Structure")
                .labeled_data(labeled_values)
                .show_grid(true)
                .show_values(true);

            if let Some(color_list) = colors {
                chart_builder = chart_builder.colors(color_list.clone());
            }

            let chart = chart_builder.build();
            builder = builder.add_to_row(Box::new(DashboardBarChart::new(chart).span(6)));
        }

        builder = builder.finish_row();

        // Financial ratios table
        if let Some(table_data) = data.tables.get("financial_ratios") {
            let pivot = PivotTable::new(table_data.clone());
            builder = builder.add_component(Box::new(pivot));
        }

        builder.build()
    }
}

impl Default for FinancialReportTemplate {
    fn default() -> Self {
        Self::new()
    }
}

/// Analytics Dashboard Template
///
/// For data analytics and metrics with:
/// - Key performance indicators
/// - Trend analysis over time
/// - Comparison charts
/// - Data distribution heatmap
#[derive(Debug, Clone)]
pub struct AnalyticsDashboardTemplate {
    title: String,
    subtitle: Option<String>,
    theme: String,
}

impl AnalyticsDashboardTemplate {
    /// Create new analytics dashboard template
    pub fn new() -> Self {
        Self {
            title: "Analytics Dashboard".to_string(),
            subtitle: None,
            theme: "colorful".to_string(),
        }
    }

    /// Set dashboard title
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    /// Set dashboard subtitle
    pub fn subtitle(mut self, subtitle: impl Into<String>) -> Self {
        self.subtitle = Some(subtitle.into());
        self
    }

    /// Set theme
    pub fn theme(mut self, theme: impl Into<String>) -> Self {
        self.theme = theme.into();
        self
    }

    /// Build dashboard from template data
    pub fn build(self, data: TemplateData) -> Result<Dashboard, PdfError> {
        let mut builder = DashboardBuilder::new()
            .title(self.title)
            .theme_by_name(&self.theme);

        if let Some(subtitle) = self.subtitle {
            builder = builder.subtitle(subtitle);
        }

        // Add KPIs
        if !data.kpis.is_empty() {
            let kpi_cards: Vec<KpiCard> = data
                .kpis
                .into_iter()
                .map(|kpi| {
                    let mut card_builder = KpiCardBuilder::new(&kpi.name, &kpi.value);

                    if let Some(subtitle) = kpi.subtitle {
                        card_builder = card_builder.subtitle(&subtitle);
                    }

                    if let (Some(trend_value), Some(trend_dir)) =
                        (kpi.trend_value, kpi.trend_direction)
                    {
                        card_builder = card_builder.trend(trend_value, trend_dir);
                    }

                    if let Some(sparkline) = kpi.sparkline {
                        card_builder = card_builder.sparkline(sparkline);
                    }

                    card_builder.build()
                })
                .collect();

            builder = builder.add_kpi_row(kpi_cards);
        }

        // Trend analysis (line chart - full width)
        if let Some(ChartData::Line { series }) = data.charts.get("trends") {
            let mut chart_builder = LineChartBuilder::new()
                .title("Trend Analysis")
                .axis_labels("Time", "Value")
                .grid(true, Color::gray(0.8), 5);

            for series_data in series {
                let data_series = DataSeries::new(&series_data.name, series_data.color)
                    .xy_data(series_data.data.clone());
                chart_builder = chart_builder.add_series(data_series);
            }

            let chart = chart_builder.build();
            builder = builder.add_component(Box::new(DashboardLineChart::new(chart).span(12)));
        }

        // Comparison charts
        builder = builder.start_row();

        if let Some(ChartData::Bar {
            labels,
            values,
            colors,
        }) = data.charts.get("comparison")
        {
            let labeled_values: Vec<(&str, f64)> = labels
                .iter()
                .map(|s| s.as_str())
                .zip(values.iter().copied())
                .collect();

            let mut chart_builder = BarChartBuilder::new()
                .title("Comparison")
                .labeled_data(labeled_values)
                .show_grid(true)
                .show_values(true);

            if let Some(color_list) = colors {
                chart_builder = chart_builder.colors(color_list.clone());
            }

            let chart = chart_builder.build();
            builder = builder.add_to_row(Box::new(DashboardBarChart::new(chart).span(6)));
        }

        if let Some(ChartData::Pie { segments }) = data.charts.get("distribution") {
            let mut chart_builder = PieChartBuilder::new().title("Distribution");

            for segment in segments {
                chart_builder = chart_builder.add_segment(PieSegment::new(
                    &segment.label,
                    segment.value,
                    segment.color,
                ));
            }

            let chart = chart_builder.show_percentages(true).build();
            builder = builder.add_to_row(Box::new(DashboardPieChart::new(chart).span(6)));
        }

        builder = builder.finish_row();

        // Heatmap
        if let Some(ChartData::HeatMap {
            values,
            row_labels,
            column_labels,
        }) = data.charts.get("heatmap")
        {
            let heatmap_data = HeatMapData {
                values: values.clone(),
                row_labels: row_labels.clone(),
                column_labels: column_labels.clone(),
            };
            builder = builder.add_component(Box::new(HeatMap::new(heatmap_data)));
        }

        builder.build()
    }
}

impl Default for AnalyticsDashboardTemplate {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_template_data_builder() {
        let data = TemplateData::new()
            .with_kpi("Revenue", "$1.5M", 12.5)
            .with_kpi("Orders", "1,234", 8.0)
            .with_metadata("period", "Q4 2024");

        assert_eq!(data.kpis.len(), 2);
        assert_eq!(data.metadata.get("period"), Some(&"Q4 2024".to_string()));
    }

    #[test]
    fn test_sales_dashboard_template_creation() {
        let template = SalesDashboardTemplate::new()
            .title("Test Sales Dashboard")
            .subtitle("Test Subtitle")
            .theme("corporate");

        assert_eq!(template.title, "Test Sales Dashboard");
        assert_eq!(template.subtitle, Some("Test Subtitle".to_string()));
    }

    #[test]
    fn test_financial_report_template_creation() {
        let template = FinancialReportTemplate::new()
            .title("Q4 Financial Report")
            .theme("minimal");

        assert_eq!(template.title, "Q4 Financial Report");
        assert_eq!(template.theme, "minimal");
    }

    #[test]
    fn test_analytics_dashboard_template_creation() {
        let template = AnalyticsDashboardTemplate::new().title("Analytics Overview");

        assert_eq!(template.title, "Analytics Overview");
        assert_eq!(template.theme, "colorful");
    }

    #[test]
    fn test_sales_dashboard_build_with_kpis() {
        let data = TemplateData::new()
            .with_kpi("Revenue", "$2.5M", 12.5)
            .with_kpi("Orders", "1,247", 8.3);

        let dashboard = SalesDashboardTemplate::new()
            .title("Test Dashboard")
            .build(data);

        assert!(dashboard.is_ok());
        let dashboard = dashboard.unwrap();
        assert_eq!(dashboard.title, "Test Dashboard");
        assert!(dashboard.components.len() >= 2); // At least 2 KPI cards
    }

    #[test]
    fn test_chart_data_variants() {
        let bar_data = ChartData::Bar {
            labels: vec!["A".to_string(), "B".to_string()],
            values: vec![100.0, 200.0],
            colors: None,
        };

        let line_data = ChartData::Line {
            series: vec![SeriesData {
                name: "Series 1".to_string(),
                data: vec![(0.0, 100.0), (1.0, 200.0)],
                color: Color::blue(),
            }],
        };

        let pie_data = ChartData::Pie {
            segments: vec![PieSegmentData {
                label: "Segment A".to_string(),
                value: 50.0,
                color: Color::red(),
            }],
        };

        // Just verify they can be created
        match bar_data {
            ChartData::Bar { .. } => (),
            _ => panic!("Expected Bar variant"),
        }

        match line_data {
            ChartData::Line { .. } => (),
            _ => panic!("Expected Line variant"),
        }

        match pie_data {
            ChartData::Pie { .. } => (),
            _ => panic!("Expected Pie variant"),
        }
    }
}
