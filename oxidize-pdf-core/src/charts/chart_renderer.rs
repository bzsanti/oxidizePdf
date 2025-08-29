//! Chart renderer for converting chart configurations to PDF graphics

use super::bar_chart::{BarChart, BarOrientation};
use super::chart_builder::Chart;
use super::line_chart::LineChart;
use super::pie_chart::PieChart;
use crate::error::PdfError;
use crate::graphics::Color;
use crate::page::Page;

/// Renderer for various chart types
pub struct ChartRenderer {
    /// Default margin around charts
    pub margin: f64,
    /// Default grid line opacity
    pub grid_opacity: f64,
}

impl ChartRenderer {
    /// Create a new chart renderer
    pub fn new() -> Self {
        Self {
            margin: 20.0,
            grid_opacity: 0.3,
        }
    }

    /// Render a generic chart
    pub fn render_chart(
        &self,
        page: &mut Page,
        chart: &Chart,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
    ) -> Result<(), PdfError> {
        match chart.chart_type {
            super::chart_builder::ChartType::VerticalBar => {
                // Convert to BarChart and render
                let bar_chart = self.convert_to_bar_chart(chart, BarOrientation::Vertical);
                self.render_bar_chart(page, &bar_chart, x, y, width, height)
            }
            super::chart_builder::ChartType::HorizontalBar => {
                let bar_chart = self.convert_to_bar_chart(chart, BarOrientation::Horizontal);
                self.render_bar_chart(page, &bar_chart, x, y, width, height)
            }
            super::chart_builder::ChartType::Pie => {
                let pie_chart = self.convert_to_pie_chart(chart);
                let radius = (width.min(height) / 2.0) - self.margin;
                self.render_pie_chart(page, &pie_chart, x + width / 2.0, y + height / 2.0, radius)
            }
            _ => {
                // For other types, render as vertical bar for now
                let bar_chart = self.convert_to_bar_chart(chart, BarOrientation::Vertical);
                self.render_bar_chart(page, &bar_chart, x, y, width, height)
            }
        }
    }

    /// Render a bar chart
    pub fn render_bar_chart(
        &self,
        page: &mut Page,
        chart: &BarChart,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
    ) -> Result<(), PdfError> {
        if chart.data.is_empty() {
            return Ok(());
        }

        // Calculate chart area (excluding title and margins)
        let title_height = if chart.title.is_empty() {
            0.0
        } else {
            chart.title_font_size + 10.0
        };
        let chart_area = self.calculate_chart_area(x, y, width, height, title_height);

        // Draw background
        if let Some(bg_color) = chart.background_color {
            page.graphics()
                .save_state()
                .set_fill_color(bg_color)
                .rectangle(x, y, width, height)
                .fill()
                .restore_state();
        }

        // Draw title
        if !chart.title.is_empty() {
            page.text()
                .set_font(chart.title_font.clone(), chart.title_font_size)
                .set_fill_color(Color::black())
                .at(x + width / 2.0, y + height - title_height / 2.0)
                .write(&chart.title)?;
        }

        match chart.orientation {
            BarOrientation::Vertical => {
                self.render_vertical_bars(page, chart, &chart_area)?;
            }
            BarOrientation::Horizontal => {
                self.render_horizontal_bars(page, chart, &chart_area)?;
            }
        }

        Ok(())
    }

    /// Render a pie chart
    pub fn render_pie_chart(
        &self,
        page: &mut Page,
        chart: &PieChart,
        center_x: f64,
        center_y: f64,
        radius: f64,
    ) -> Result<(), PdfError> {
        if chart.segments.is_empty() {
            return Ok(());
        }

        let total_value = chart.total_value();
        if total_value <= 0.0 {
            return Ok(());
        }

        let mut current_angle = chart.start_angle;

        // Draw each segment
        for segment in &chart.segments {
            let segment_angle = segment.angle_radians(total_value);
            if segment_angle <= 0.0 {
                continue;
            }

            // Calculate center point (with explosion if needed)
            let (seg_center_x, seg_center_y) = if segment.exploded {
                let middle_angle = current_angle + segment_angle / 2.0;
                let explosion_distance = radius * segment.explosion_distance;
                (
                    center_x + explosion_distance * middle_angle.cos(),
                    center_y + explosion_distance * middle_angle.sin(),
                )
            } else {
                (center_x, center_y)
            };

            // Draw the segment
            self.draw_pie_segment(
                page,
                seg_center_x,
                seg_center_y,
                radius,
                current_angle,
                current_angle + segment_angle,
                segment.color,
            )?;

            // Draw border if enabled
            if chart.draw_borders {
                self.draw_pie_segment_border(
                    page,
                    seg_center_x,
                    seg_center_y,
                    radius,
                    current_angle,
                    current_angle + segment_angle,
                    chart.border_color,
                    chart.border_width,
                )?;
            }

            current_angle += segment_angle;
        }

        // Draw title if present
        if !chart.title.is_empty() {
            page.text()
                .set_font(chart.title_font.clone(), chart.title_font_size)
                .set_fill_color(Color::black())
                .at(center_x, center_y + radius + 30.0)
                .write(&chart.title)?;
        }

        Ok(())
    }

    /// Render a line chart
    pub fn render_line_chart(
        &self,
        page: &mut Page,
        chart: &LineChart,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
    ) -> Result<(), PdfError> {
        if chart.series.is_empty() {
            return Ok(());
        }

        // Calculate chart area
        let title_height = if chart.title.is_empty() {
            0.0
        } else {
            chart.title_font_size + 10.0
        };
        let chart_area = self.calculate_chart_area(x, y, width, height, title_height);

        // Draw background
        if let Some(bg_color) = chart.background_color {
            page.graphics()
                .save_state()
                .set_fill_color(bg_color)
                .rectangle(x, y, width, height)
                .fill()
                .restore_state();
        }

        // Get combined ranges
        let (x_min, x_max) = chart.combined_x_range();
        let (y_min, y_max) = chart.combined_y_range();

        // Draw grid if enabled
        if chart.show_grid {
            self.draw_line_chart_grid(page, &chart_area, chart.grid_lines, chart.grid_color)?;
        }

        // Draw each series
        for series in &chart.series {
            if series.data.len() < 2 {
                continue; // Need at least 2 points for a line
            }

            // Convert data points to screen coordinates
            let screen_points: Vec<(f64, f64)> = series
                .data
                .iter()
                .map(|(data_x, data_y)| {
                    let screen_x =
                        chart_area.x + ((data_x - x_min) / (x_max - x_min)) * chart_area.width;
                    let screen_y =
                        chart_area.y + ((data_y - y_min) / (y_max - y_min)) * chart_area.height;
                    (screen_x, screen_y)
                })
                .collect();

            // Draw area fill if enabled
            if series.fill_area && screen_points.len() >= 2 {
                self.draw_area_fill(page, &screen_points, &chart_area, series)?;
            }

            // Draw the line
            self.draw_line_series(page, &screen_points, series)?;

            // Draw markers if enabled
            if series.show_markers {
                self.draw_line_markers(page, &screen_points, series)?;
            }
        }

        // Draw title
        if !chart.title.is_empty() {
            page.text()
                .set_font(chart.title_font.clone(), chart.title_font_size)
                .set_fill_color(Color::black())
                .at(x + width / 2.0, y + height - title_height / 2.0)
                .write(&chart.title)?;
        }

        Ok(())
    }

    // Helper methods

    fn calculate_chart_area(
        &self,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        title_height: f64,
    ) -> ChartArea {
        ChartArea {
            x: x + self.margin,
            y: y + self.margin,
            width: width - 2.0 * self.margin,
            height: height - 2.0 * self.margin - title_height,
        }
    }

    fn render_vertical_bars(
        &self,
        page: &mut Page,
        chart: &BarChart,
        area: &ChartArea,
    ) -> Result<(), PdfError> {
        let max_value = chart.max_value();
        if max_value <= 0.0 {
            return Ok(());
        }

        let bar_width = chart.calculate_bar_width(area.width);
        let spacing = bar_width * chart.bar_spacing;

        for (i, data) in chart.data.iter().enumerate() {
            let bar_height = (data.value / max_value) * area.height;
            let bar_x = area.x + i as f64 * (bar_width + spacing);
            let bar_y = area.y;

            let color = chart.color_for_index(i);

            // Draw bar
            page.graphics()
                .save_state()
                .set_fill_color(color)
                .rectangle(bar_x, bar_y, bar_width, bar_height)
                .fill()
                .restore_state();

            // Draw border if specified
            if let Some(border_color) = chart.bar_border_color {
                page.graphics()
                    .save_state()
                    .set_stroke_color(border_color)
                    .set_line_width(chart.bar_border_width)
                    .rectangle(bar_x, bar_y, bar_width, bar_height)
                    .stroke()
                    .restore_state();
            }

            // Draw value if enabled
            if chart.show_values {
                let value_text = format!("{:.1}", data.value);
                page.text()
                    .set_font(chart.value_font.clone(), chart.value_font_size)
                    .set_fill_color(Color::black())
                    .at(bar_x + bar_width / 2.0, bar_y + bar_height + 5.0)
                    .write(&value_text)?;
            }

            // Draw label
            page.text()
                .set_font(chart.label_font.clone(), chart.label_font_size)
                .set_fill_color(Color::black())
                .at(bar_x + bar_width / 2.0, area.y - 15.0)
                .write(&data.label)?;
        }

        Ok(())
    }

    fn render_horizontal_bars(
        &self,
        page: &mut Page,
        chart: &BarChart,
        area: &ChartArea,
    ) -> Result<(), PdfError> {
        let max_value = chart.max_value();
        if max_value <= 0.0 {
            return Ok(());
        }

        let bar_height = area.height / chart.data.len() as f64;
        let spacing = bar_height * chart.bar_spacing;
        let actual_bar_height = bar_height - spacing;

        for (i, data) in chart.data.iter().enumerate() {
            let bar_width = (data.value / max_value) * area.width;
            let bar_x = area.x;
            let bar_y = area.y + area.height - (i as f64 + 1.0) * bar_height + spacing / 2.0;

            let color = chart.color_for_index(i);

            // Draw bar
            page.graphics()
                .save_state()
                .set_fill_color(color)
                .rectangle(bar_x, bar_y, bar_width, actual_bar_height)
                .fill()
                .restore_state();

            // Draw value if enabled
            if chart.show_values {
                let value_text = format!("{:.1}", data.value);
                page.text()
                    .set_font(chart.value_font.clone(), chart.value_font_size)
                    .set_fill_color(Color::black())
                    .at(bar_x + bar_width + 5.0, bar_y + actual_bar_height / 2.0)
                    .write(&value_text)?;
            }

            // Draw label
            page.text()
                .set_font(chart.label_font.clone(), chart.label_font_size)
                .set_fill_color(Color::black())
                .at(bar_x - 10.0, bar_y + actual_bar_height / 2.0)
                .write(&data.label)?;
        }

        Ok(())
    }

    fn draw_pie_segment(
        &self,
        page: &mut Page,
        center_x: f64,
        center_y: f64,
        radius: f64,
        start_angle: f64,
        end_angle: f64,
        color: Color,
    ) -> Result<(), PdfError> {
        if (end_angle - start_angle).abs() < 0.001 {
            return Ok(()); // Skip very small segments
        }

        let graphics = page.graphics();

        graphics
            .save_state()
            .set_fill_color(color)
            .move_to(center_x, center_y);

        // Draw arc
        let start_x = center_x + radius * start_angle.cos();
        let start_y = center_y + radius * start_angle.sin();
        graphics.line_to(start_x, start_y);

        // Simple arc approximation using line segments
        let segments = 20;
        let angle_step = (end_angle - start_angle) / segments as f64;

        for i in 0..=segments {
            let angle = start_angle + i as f64 * angle_step;
            let x = center_x + radius * angle.cos();
            let y = center_y + radius * angle.sin();
            graphics.line_to(x, y);
        }

        graphics.line_to(center_x, center_y).fill().restore_state();

        Ok(())
    }

    fn draw_pie_segment_border(
        &self,
        page: &mut Page,
        center_x: f64,
        center_y: f64,
        radius: f64,
        start_angle: f64,
        end_angle: f64,
        color: Color,
        width: f64,
    ) -> Result<(), PdfError> {
        let graphics = page.graphics();

        graphics
            .save_state()
            .set_stroke_color(color)
            .set_line_width(width);

        // Draw the arc border
        let segments = 20;
        let angle_step = (end_angle - start_angle) / segments as f64;

        let start_x = center_x + radius * start_angle.cos();
        let start_y = center_y + radius * start_angle.sin();
        graphics.move_to(start_x, start_y);

        for i in 1..=segments {
            let angle = start_angle + i as f64 * angle_step;
            let x = center_x + radius * angle.cos();
            let y = center_y + radius * angle.sin();
            graphics.line_to(x, y);
        }

        graphics.stroke().restore_state();

        Ok(())
    }

    fn draw_line_chart_grid(
        &self,
        page: &mut Page,
        area: &ChartArea,
        grid_lines: usize,
        color: Color,
    ) -> Result<(), PdfError> {
        let graphics = page.graphics();

        graphics
            .save_state()
            .set_stroke_color(color)
            .set_line_width(0.5);

        // Vertical grid lines
        for i in 0..=grid_lines {
            let x = area.x + (i as f64 / grid_lines as f64) * area.width;
            graphics.move_to(x, area.y).line_to(x, area.y + area.height);
        }

        // Horizontal grid lines
        for i in 0..=grid_lines {
            let y = area.y + (i as f64 / grid_lines as f64) * area.height;
            graphics.move_to(area.x, y).line_to(area.x + area.width, y);
        }

        graphics.stroke().restore_state();

        Ok(())
    }

    fn draw_line_series(
        &self,
        page: &mut Page,
        points: &[(f64, f64)],
        series: &super::line_chart::DataSeries,
    ) -> Result<(), PdfError> {
        if points.len() < 2 {
            return Ok(());
        }

        let graphics = page.graphics();

        graphics
            .save_state()
            .set_stroke_color(series.color)
            .set_line_width(series.line_width)
            .move_to(points[0].0, points[0].1);

        for point in &points[1..] {
            graphics.line_to(point.0, point.1);
        }

        graphics.stroke().restore_state();

        Ok(())
    }

    fn draw_line_markers(
        &self,
        page: &mut Page,
        points: &[(f64, f64)],
        series: &super::line_chart::DataSeries,
    ) -> Result<(), PdfError> {
        let graphics = page.graphics();

        graphics.save_state().set_fill_color(series.color);

        for &(x, y) in points {
            graphics.circle(x, y, series.marker_size);
        }

        graphics.fill().restore_state();

        Ok(())
    }

    fn draw_area_fill(
        &self,
        page: &mut Page,
        points: &[(f64, f64)],
        area: &ChartArea,
        series: &super::line_chart::DataSeries,
    ) -> Result<(), PdfError> {
        if points.len() < 2 {
            return Ok(());
        }

        let fill_color = series.fill_color.unwrap_or_else(|| {
            // PDF doesn't support alpha, use a lighter version of the line color
            series.color
        });

        let graphics = page.graphics();

        graphics
            .save_state()
            .set_fill_color(fill_color)
            .move_to(points[0].0, area.y);

        for &(x, y) in points {
            graphics.line_to(x, y);
        }

        graphics
            .line_to(points.last().unwrap().0, area.y)
            .fill()
            .restore_state();

        Ok(())
    }

    // Conversion helpers
    fn convert_to_bar_chart(&self, chart: &Chart, orientation: BarOrientation) -> BarChart {
        use super::bar_chart::BarChartBuilder;

        let mut builder = BarChartBuilder::new()
            .title(chart.title.clone())
            .orientation(orientation)
            .colors(chart.colors.clone());

        for data in &chart.data {
            builder = builder.add_data(super::chart_builder::ChartData::new(
                data.label.clone(),
                data.value,
            ));
        }

        builder.build()
    }

    fn convert_to_pie_chart(&self, chart: &Chart) -> PieChart {
        use super::pie_chart::PieChartBuilder;

        PieChartBuilder::new()
            .title(chart.title.clone())
            .data(chart.data.clone())
            .build()
    }
}

impl Default for ChartRenderer {
    fn default() -> Self {
        Self::new()
    }
}

/// Chart area definition
struct ChartArea {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}
