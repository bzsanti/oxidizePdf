//! Chart renderer for converting chart configurations to PDF graphics

use super::bar_chart::{BarChart, BarOrientation};
use super::chart_builder::Chart;
use super::line_chart::LineChart;
use super::pie_chart::PieChart;
use crate::coordinate_system::CoordinateSystem;
use crate::error::PdfError;
use crate::graphics::Color;
use crate::page::Page;
use crate::text::metrics::measure_text;

/// Renderer for various chart types
pub struct ChartRenderer {
    /// Default margin around charts
    pub margin: f64,
    /// Default grid line opacity
    pub grid_opacity: f64,
    /// Coordinate system to use for rendering
    pub coordinate_system: CoordinateSystem,
}

impl ChartRenderer {
    /// Create a new chart renderer with PDF standard coordinates (default)
    pub fn new() -> Self {
        Self {
            margin: 20.0,
            grid_opacity: 0.3,
            coordinate_system: CoordinateSystem::PdfStandard,
        }
    }

    /// Create a new chart renderer with specific coordinate system
    pub fn with_coordinate_system(coordinate_system: CoordinateSystem) -> Self {
        Self {
            margin: 20.0,
            grid_opacity: 0.3,
            coordinate_system,
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

    // Coordinate transformation methods

    /// Transform Y coordinate based on the active coordinate system
    #[allow(dead_code)]
    fn transform_y(&self, y: f64, chart_height: f64, page_height: f64) -> f64 {
        match self.coordinate_system {
            CoordinateSystem::PdfStandard => y, // No transformation needed
            CoordinateSystem::ScreenSpace => {
                // Convert screen coordinates (origin top-left) to PDF coordinates (origin bottom-left)
                page_height - y - chart_height
            }
            CoordinateSystem::Custom(matrix) => {
                // Apply custom transformation matrix
                let point = crate::geometry::Point::new(0.0, y);
                matrix.transform_point(point).y
            }
        }
    }

    /// Transform bar coordinates for vertical bars based on coordinate system
    fn transform_vertical_bar(
        &self,
        bar_x: f64,
        bar_y: f64,
        bar_height: f64,
        _chart_area_height: f64,
        _page_height: f64,
    ) -> (f64, f64, f64) {
        match self.coordinate_system {
            CoordinateSystem::PdfStandard => {
                // PDF coordinates: bars grow upward from base
                (bar_x, bar_y, bar_height)
            }
            CoordinateSystem::ScreenSpace => {
                // Screen coordinates: bars should be positioned correctly within chart area
                // In screen coordinates, Y=0 is at the top, so we need to flip the bar position
                // but keep bars growing upward visually (which is actually downward in screen space)
                (bar_x, bar_y, bar_height)
            }
            CoordinateSystem::Custom(matrix) => {
                // For custom matrices, apply basic transformation
                let start_point = matrix.transform_point(crate::geometry::Point::new(bar_x, bar_y));
                let end_point =
                    matrix.transform_point(crate::geometry::Point::new(bar_x, bar_y + bar_height));
                let transformed_height = (end_point.y - start_point.y).abs();
                (start_point.x, start_point.y, transformed_height)
            }
        }
    }

    /// Transform bar coordinates for horizontal bars based on coordinate system  
    fn transform_horizontal_bar(
        &self,
        bar_x: f64,
        bar_y: f64,
        bar_width: f64,
        bar_height: f64,
        chart_area: &ChartArea,
    ) -> (f64, f64, f64, f64) {
        match self.coordinate_system {
            CoordinateSystem::PdfStandard => {
                // PDF coordinates: no transformation needed
                (bar_x, bar_y, bar_width, bar_height)
            }
            CoordinateSystem::ScreenSpace => {
                // Screen coordinates: Y positions need to be flipped within chart area
                let screen_bar_y =
                    chart_area.y + chart_area.height - bar_y - bar_height + chart_area.y;
                (bar_x, screen_bar_y, bar_width, bar_height)
            }
            CoordinateSystem::Custom(matrix) => {
                // For custom matrices, apply transformation
                let start_point = matrix.transform_point(crate::geometry::Point::new(bar_x, bar_y));
                let end_point = matrix.transform_point(crate::geometry::Point::new(
                    bar_x + bar_width,
                    bar_y + bar_height,
                ));
                let transformed_width = (end_point.x - start_point.x).abs();
                let transformed_height = (end_point.y - start_point.y).abs();
                (
                    start_point.x,
                    start_point.y,
                    transformed_width,
                    transformed_height,
                )
            }
        }
    }

    /// Transform line chart data points based on coordinate system
    fn transform_line_points(
        &self,
        points: &[(f64, f64)],
        chart_area: &ChartArea,
    ) -> Vec<(f64, f64)> {
        match self.coordinate_system {
            CoordinateSystem::PdfStandard => {
                // PDF coordinates: no transformation needed for data points
                points.to_vec()
            }
            CoordinateSystem::ScreenSpace => {
                // Screen coordinates: flip Y coordinates within chart area
                points
                    .iter()
                    .map(|(x, y)| {
                        let flipped_y = chart_area.y + chart_area.height - (y - chart_area.y);
                        (*x, flipped_y)
                    })
                    .collect()
            }
            CoordinateSystem::Custom(matrix) => {
                // Apply custom transformation
                points
                    .iter()
                    .map(|(x, y)| {
                        let transformed =
                            matrix.transform_point(crate::geometry::Point::new(*x, *y));
                        (transformed.x, transformed.y)
                    })
                    .collect()
            }
        }
    }

    /// Transform text position for labels based on coordinate system
    fn transform_label_position(&self, x: f64, y: f64, chart_area: &ChartArea) -> (f64, f64) {
        match self.coordinate_system {
            CoordinateSystem::PdfStandard => {
                // Labels go below the chart area (negative offset)
                (x, y - 15.0)
            }
            CoordinateSystem::ScreenSpace => {
                // Labels go below the chart area (positive offset in screen space)
                (x, chart_area.y + chart_area.height + 15.0)
            }
            CoordinateSystem::Custom(matrix) => {
                // Apply custom transformation
                let point = matrix.transform_point(crate::geometry::Point::new(x, y));
                (point.x, point.y)
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
            let title_width = measure_text(
                &chart.title,
                chart.title_font.clone(),
                chart.title_font_size,
            );
            page.text()
                .set_font(chart.title_font.clone(), chart.title_font_size)
                .set_fill_color(Color::black())
                .at(
                    x + width / 2.0 - title_width / 2.0,
                    y + height - title_height / 2.0,
                )
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
            let title_width = measure_text(
                &chart.title,
                chart.title_font.clone(),
                chart.title_font_size,
            );
            page.text()
                .set_font(chart.title_font.clone(), chart.title_font_size)
                .set_fill_color(Color::black())
                .at(center_x - title_width / 2.0, center_y + radius + 30.0)
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

            // Convert data points to chart coordinates
            let chart_points: Vec<(f64, f64)> = series
                .data
                .iter()
                .map(|(data_x, data_y)| {
                    let chart_x =
                        chart_area.x + ((data_x - x_min) / (x_max - x_min)) * chart_area.width;
                    let chart_y =
                        chart_area.y + ((data_y - y_min) / (y_max - y_min)) * chart_area.height;
                    (chart_x, chart_y)
                })
                .collect();

            // Transform points based on coordinate system
            let final_points = self.transform_line_points(&chart_points, &chart_area);

            // Draw area fill if enabled
            if series.fill_area && final_points.len() >= 2 {
                self.draw_area_fill(page, &final_points, &chart_area, series)?;
            }

            // Draw the line
            self.draw_line_series(page, &final_points, series)?;

            // Draw markers if enabled
            if series.show_markers {
                self.draw_line_markers(page, &final_points, series)?;
            }
        }

        // Draw title
        if !chart.title.is_empty() {
            let title_width = measure_text(
                &chart.title,
                chart.title_font.clone(),
                chart.title_font_size,
            );
            page.text()
                .set_font(chart.title_font.clone(), chart.title_font_size)
                .set_fill_color(Color::black())
                .at(
                    x + width / 2.0 - title_width / 2.0,
                    y + height - title_height / 2.0,
                )
                .write(&chart.title)?;
        }

        // Draw axis labels if present
        if !chart.x_axis_label.is_empty() {
            let x_label_width = measure_text(
                &chart.x_axis_label,
                chart.axis_font.clone(),
                chart.axis_font_size,
            );
            page.text()
                .set_font(chart.axis_font.clone(), chart.axis_font_size)
                .set_fill_color(Color::black())
                .at(x + width / 2.0 - x_label_width / 2.0, y - 20.0)
                .write(&chart.x_axis_label)?;
        }

        if !chart.y_axis_label.is_empty() {
            // Position Y axis label inside the chart area to ensure visibility
            page.text()
                .set_font(chart.axis_font.clone(), chart.axis_font_size)
                .set_fill_color(Color::black())
                .at(x + 10.0, y + height - 20.0)
                .write(&chart.y_axis_label)?;
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
            let bar_y_original = area.y;

            // Transform bar coordinates based on coordinate system
            let (final_bar_x, final_bar_y, final_bar_height) = self.transform_vertical_bar(
                bar_x,
                bar_y_original,
                bar_height,
                area.height,
                page.height(),
            );

            let color = chart.color_for_index(i);

            // Draw bar
            page.graphics()
                .save_state()
                .set_fill_color(color)
                .rectangle(final_bar_x, final_bar_y, bar_width, final_bar_height)
                .fill()
                .restore_state();

            // Draw border if specified
            if let Some(border_color) = chart.bar_border_color {
                page.graphics()
                    .save_state()
                    .set_stroke_color(border_color)
                    .set_line_width(chart.bar_border_width)
                    .rectangle(final_bar_x, final_bar_y, bar_width, final_bar_height)
                    .stroke()
                    .restore_state();
            }

            // Draw value if enabled
            if chart.show_values {
                let value_text = format!("{:.1}", data.value);
                let value_y = match self.coordinate_system {
                    CoordinateSystem::PdfStandard => final_bar_y + final_bar_height + 5.0,
                    CoordinateSystem::ScreenSpace => final_bar_y - 5.0, // Above bars in screen space
                    CoordinateSystem::Custom(_) => final_bar_y + final_bar_height + 5.0,
                };

                let value_width =
                    measure_text(&value_text, chart.value_font.clone(), chart.value_font_size);
                page.text()
                    .set_font(chart.value_font.clone(), chart.value_font_size)
                    .set_fill_color(Color::black())
                    .at(final_bar_x + bar_width / 2.0 - value_width / 2.0, value_y)
                    .write(&value_text)?;
            }

            // Draw label using coordinate system transformation
            let (label_x, label_y) =
                self.transform_label_position(bar_x + bar_width / 2.0, bar_y_original, area);

            let label_width =
                measure_text(&data.label, chart.label_font.clone(), chart.label_font_size);
            page.text()
                .set_font(chart.label_font.clone(), chart.label_font_size)
                .set_fill_color(Color::black())
                .at(label_x - label_width / 2.0, label_y)
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
            let bar_x_original = area.x;
            let bar_y_original =
                area.y + area.height - (i as f64 + 1.0) * bar_height + spacing / 2.0;

            // Transform bar coordinates based on coordinate system
            let (final_bar_x, final_bar_y, final_bar_width, final_bar_height) = self
                .transform_horizontal_bar(
                    bar_x_original,
                    bar_y_original,
                    bar_width,
                    actual_bar_height,
                    area,
                );

            let color = chart.color_for_index(i);

            // Draw bar
            page.graphics()
                .save_state()
                .set_fill_color(color)
                .rectangle(final_bar_x, final_bar_y, final_bar_width, final_bar_height)
                .fill()
                .restore_state();

            // Draw border if specified
            if let Some(border_color) = chart.bar_border_color {
                page.graphics()
                    .save_state()
                    .set_stroke_color(border_color)
                    .set_line_width(chart.bar_border_width)
                    .rectangle(final_bar_x, final_bar_y, final_bar_width, final_bar_height)
                    .stroke()
                    .restore_state();
            }

            // Draw value if enabled
            if chart.show_values {
                let value_text = format!("{:.1}", data.value);
                let value_x = final_bar_x + final_bar_width + 5.0;
                let value_y = final_bar_y + final_bar_height / 2.0;

                // Note: For horizontal bars, values are positioned to the right of the bar
                // No need to center horizontally as they are left-aligned from the edge
                page.text()
                    .set_font(chart.value_font.clone(), chart.value_font_size)
                    .set_fill_color(Color::black())
                    .at(value_x, value_y)
                    .write(&value_text)?;
            }

            // Draw label - for horizontal bars, labels go to the left
            let label_width =
                measure_text(&data.label, chart.label_font.clone(), chart.label_font_size);
            let label_x = final_bar_x - 10.0 - label_width; // Right-align to the left of the bar
            let label_y = final_bar_y + final_bar_height / 2.0;

            page.text()
                .set_font(chart.label_font.clone(), chart.label_font_size)
                .set_fill_color(Color::black())
                .at(label_x, label_y)
                .write(&data.label)?;
        }

        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
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

    #[allow(clippy::too_many_arguments)]
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

        let fill_color = series.fill_color.unwrap_or({
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

        // Safe to unwrap: points.len() >= 2 is guaranteed by check at line 833
        if let Some(last_point) = points.last() {
            graphics
                .line_to(last_point.0, area.y)
                .fill()
                .restore_state();
        }

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
