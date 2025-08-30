//! Chart Generation System for PDF Reports
//!
//! This module provides a comprehensive chart system for creating professional
//! visual representations of data in PDF documents. It includes support for
//! bar charts, pie charts, line graphs, and other common chart types.
//!
//! # Features
//! - Bar charts (horizontal and vertical)
//! - Pie charts with customizable segments
//! - Progress bars and statistics bars
//! - Line graphs with multiple series
//! - Customizable colors, fonts, and styling
//! - Automatic legends and labels
//! - Professional chart layouts
//!
//! # Example
//! ```rust
//! use oxidize_pdf::charts::{BarChartBuilder, ChartData};
//! use oxidize_pdf::graphics::Color;
//!
//! let data = vec![
//!     ChartData::new("Q1", 25.0),
//!     ChartData::new("Q2", 35.0),
//!     ChartData::new("Q3", 20.0),
//!     ChartData::new("Q4", 45.0),
//! ];
//!
//! let chart = BarChartBuilder::new()
//!     .title("Quarterly Sales")
//!     .data(data)
//!     .colors(vec![Color::blue(), Color::green(), Color::yellow(), Color::red()])
//!     .build();
//! ```

mod bar_chart;
mod chart_builder;
mod chart_renderer;
mod line_chart;
mod pie_chart;

pub use bar_chart::{BarChart, BarChartBuilder, BarOrientation};
pub use chart_builder::{Chart, ChartBuilder, ChartData, ChartType, LegendPosition};
pub use chart_renderer::ChartRenderer;
pub use line_chart::{DataSeries, LineChart, LineChartBuilder};
pub use pie_chart::{PieChart, PieChartBuilder, PieSegment};

use crate::error::PdfError;
use crate::page::Page;

/// Extension trait to add chart capabilities to PDF pages
pub trait ChartExt {
    /// Add a chart to the page at the specified position
    fn add_chart(
        &mut self,
        chart: &Chart,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
    ) -> Result<(), PdfError>;

    /// Add a bar chart with automatic sizing
    fn add_bar_chart(
        &mut self,
        chart: &BarChart,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
    ) -> Result<(), PdfError>;

    /// Add a pie chart with automatic sizing
    fn add_pie_chart(
        &mut self,
        chart: &PieChart,
        x: f64,
        y: f64,
        radius: f64,
    ) -> Result<(), PdfError>;

    /// Add a line chart with automatic sizing
    fn add_line_chart(
        &mut self,
        chart: &LineChart,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
    ) -> Result<(), PdfError>;
}

impl ChartExt for Page {
    fn add_chart(
        &mut self,
        chart: &Chart,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
    ) -> Result<(), PdfError> {
        let renderer = ChartRenderer::with_coordinate_system(self.coordinate_system());
        renderer.render_chart(self, chart, x, y, width, height)
    }

    fn add_bar_chart(
        &mut self,
        chart: &BarChart,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
    ) -> Result<(), PdfError> {
        let renderer = ChartRenderer::with_coordinate_system(self.coordinate_system());
        renderer.render_bar_chart(self, chart, x, y, width, height)
    }

    fn add_pie_chart(
        &mut self,
        chart: &PieChart,
        x: f64,
        y: f64,
        radius: f64,
    ) -> Result<(), PdfError> {
        let renderer = ChartRenderer::with_coordinate_system(self.coordinate_system());
        renderer.render_pie_chart(self, chart, x, y, radius)
    }

    fn add_line_chart(
        &mut self,
        chart: &LineChart,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
    ) -> Result<(), PdfError> {
        let renderer = ChartRenderer::with_coordinate_system(self.coordinate_system());
        renderer.render_line_chart(self, chart, x, y, width, height)
    }
}
