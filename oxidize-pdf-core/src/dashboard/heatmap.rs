//! HeatMap Visualization Component
//!
//! This module implements heat maps for dashboard visualizations, displaying
//! data intensity through color gradients in a matrix format.

use super::{
    component::ComponentConfig, ComponentPosition, ComponentSpan, DashboardComponent,
    DashboardTheme,
};
use crate::error::PdfError;
use crate::graphics::Color;
use crate::page::Page;

/// HeatMap visualization component
#[derive(Debug, Clone)]
pub struct HeatMap {
    /// Component configuration
    config: ComponentConfig,
    /// Heat map data
    data: HeatMapData,
    /// Configuration options
    options: HeatMapOptions,
    /// Color scale for the heat map
    color_scale: ColorScale,
}

impl HeatMap {
    /// Create a new heat map
    pub fn new(data: HeatMapData) -> Self {
        Self {
            config: ComponentConfig::new(ComponentSpan::new(6)), // Half width by default
            data,
            options: HeatMapOptions::default(),
            color_scale: ColorScale::default(),
        }
    }

    /// Set heat map options
    pub fn with_options(mut self, options: HeatMapOptions) -> Self {
        self.options = options;
        self
    }

    /// Set color scale
    pub fn with_color_scale(mut self, color_scale: ColorScale) -> Self {
        self.color_scale = color_scale;
        self
    }

    /// Get min/max values from the data
    fn get_value_range(&self) -> (f64, f64) {
        let min_val = self.color_scale.min_value.unwrap_or_else(|| {
            self.data
                .values
                .iter()
                .flat_map(|row| row.iter())
                .copied()
                .fold(f64::INFINITY, f64::min)
        });

        let max_val = self.color_scale.max_value.unwrap_or_else(|| {
            self.data
                .values
                .iter()
                .flat_map(|row| row.iter())
                .copied()
                .fold(f64::NEG_INFINITY, f64::max)
        });

        (min_val, max_val)
    }

    /// Interpolate color based on value
    fn interpolate_color(&self, value: f64, min_val: f64, max_val: f64) -> Color {
        if max_val == min_val {
            return self.color_scale.colors[0];
        }

        let normalized = ((value - min_val) / (max_val - min_val)).clamp(0.0, 1.0);

        if self.color_scale.colors.len() == 1 {
            return self.color_scale.colors[0];
        }

        // Interpolate between colors
        let segment_count = self.color_scale.colors.len() - 1;
        let segment = (normalized * segment_count as f64).floor() as usize;
        let segment = segment.min(segment_count - 1);

        let t = (normalized * segment_count as f64) - segment as f64;

        let c1 = &self.color_scale.colors[segment];
        let c2 = &self.color_scale.colors[segment + 1];

        // Extract RGB components from both colors
        let (r1, g1, b1) = match c1 {
            Color::Rgb(r, g, b) => (*r, *g, *b),
            Color::Gray(v) => (*v, *v, *v),
            Color::Cmyk(c, m, y, k) => {
                // Simple CMYK to RGB conversion
                let r = (1.0 - c) * (1.0 - k);
                let g = (1.0 - m) * (1.0 - k);
                let b = (1.0 - y) * (1.0 - k);
                (r, g, b)
            }
        };

        let (r2, g2, b2) = match c2 {
            Color::Rgb(r, g, b) => (*r, *g, *b),
            Color::Gray(v) => (*v, *v, *v),
            Color::Cmyk(c, m, y, k) => {
                let r = (1.0 - c) * (1.0 - k);
                let g = (1.0 - m) * (1.0 - k);
                let b = (1.0 - y) * (1.0 - k);
                (r, g, b)
            }
        };

        Color::rgb(r1 + (r2 - r1) * t, g1 + (g2 - g1) * t, b1 + (b2 - b1) * t)
    }

    /// Check if a color is dark (for text contrast)
    fn is_dark_color(&self, color: &Color) -> bool {
        // Using relative luminance formula
        let (r, g, b) = match color {
            Color::Rgb(r, g, b) => (*r, *g, *b),
            Color::Gray(v) => (*v, *v, *v),
            Color::Cmyk(c, m, y, k) => {
                let r = (1.0 - c) * (1.0 - k);
                let g = (1.0 - m) * (1.0 - k);
                let b = (1.0 - y) * (1.0 - k);
                (r, g, b)
            }
        };
        let luminance = 0.299 * r + 0.587 * g + 0.114 * b;
        luminance < 0.5
    }

    /// Render the color legend
    fn render_legend(
        &self,
        page: &mut Page,
        _position: ComponentPosition,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        min_val: f64,
        max_val: f64,
        theme: &DashboardTheme,
    ) -> Result<(), PdfError> {
        let steps = 20;
        let step_height = height / steps as f64;

        // Draw gradient
        for i in 0..steps {
            let value = min_val + (max_val - min_val) * (i as f64 / steps as f64);
            let color = self.interpolate_color(value, min_val, max_val);
            let step_y = y + (steps - 1 - i) as f64 * step_height;

            page.graphics()
                .set_fill_color(color)
                .rect(x, step_y, width, step_height)
                .fill();
        }

        // Draw border
        page.graphics()
            .set_stroke_color(Color::gray(0.5))
            .set_line_width(1.0)
            .rect(x, y, width, height)
            .stroke();

        // Draw min/max labels
        page.text()
            .set_font(crate::Font::Helvetica, 8.0)
            .set_fill_color(theme.colors.text_secondary)
            .at(x + width + 5.0, y - 5.0)
            .write(&format!("{:.1}", max_val))?;

        page.text()
            .set_font(crate::Font::Helvetica, 8.0)
            .set_fill_color(theme.colors.text_secondary)
            .at(x + width + 5.0, y + height - 10.0)
            .write(&format!("{:.1}", min_val))?;

        Ok(())
    }
}

impl DashboardComponent for HeatMap {
    fn render(
        &self,
        page: &mut Page,
        position: ComponentPosition,
        theme: &DashboardTheme,
    ) -> Result<(), PdfError> {
        let title = self.options.title.as_deref().unwrap_or("HeatMap");

        // Calculate dimensions
        let title_height = 30.0;
        let legend_width = if self.options.show_legend { 60.0 } else { 0.0 };
        let label_width = 80.0;
        let label_height = 30.0;

        let chart_x = position.x + label_width;
        let chart_y = position.y;
        let chart_width = position.width - label_width - legend_width;
        let chart_height = position.height - title_height - label_height;

        // Render title
        page.text()
            .set_font(crate::Font::HelveticaBold, theme.typography.heading_size)
            .set_fill_color(theme.colors.text_primary)
            .at(position.x, position.y + position.height - 15.0)
            .write(title)?;

        // Calculate cell dimensions
        let rows = self.data.values.len();
        let cols = if rows > 0 {
            self.data.values[0].len()
        } else {
            0
        };

        if rows == 0 || cols == 0 {
            return Ok(());
        }

        let cell_width = chart_width / cols as f64;
        let cell_height = chart_height / rows as f64;

        // Find min/max values for color scaling
        let (min_val, max_val) = self.get_value_range();

        // Render cells
        for (row_idx, row) in self.data.values.iter().enumerate() {
            for (col_idx, &value) in row.iter().enumerate() {
                let x = chart_x + col_idx as f64 * cell_width;
                let y = chart_y + title_height + (rows - 1 - row_idx) as f64 * cell_height;

                // Get color for this value
                let color = self.interpolate_color(value, min_val, max_val);

                // Draw cell
                page.graphics()
                    .set_fill_color(color)
                    .rect(
                        x + self.options.cell_padding,
                        y + self.options.cell_padding,
                        cell_width - 2.0 * self.options.cell_padding,
                        cell_height - 2.0 * self.options.cell_padding,
                    )
                    .fill();

                // Draw cell border
                page.graphics()
                    .set_stroke_color(Color::gray(0.8))
                    .set_line_width(0.5)
                    .rect(
                        x + self.options.cell_padding,
                        y + self.options.cell_padding,
                        cell_width - 2.0 * self.options.cell_padding,
                        cell_height - 2.0 * self.options.cell_padding,
                    )
                    .stroke();

                // Optionally show values
                if self.options.show_values && cell_width > 40.0 && cell_height > 20.0 {
                    let text_color = if self.is_dark_color(&color) {
                        Color::white()
                    } else {
                        Color::black()
                    };

                    page.text()
                        .set_font(crate::Font::Helvetica, 8.0)
                        .set_fill_color(text_color)
                        .at(x + cell_width / 2.0 - 10.0, y + cell_height / 2.0 - 3.0)
                        .write(&format!("{:.1}", value))?;
                }
            }
        }

        // Render row labels
        for (idx, label) in self.data.row_labels.iter().enumerate() {
            let y = chart_y + title_height + (rows - 1 - idx) as f64 * cell_height;
            page.text()
                .set_font(crate::Font::Helvetica, 9.0)
                .set_fill_color(theme.colors.text_secondary)
                .at(position.x + 5.0, y + cell_height / 2.0 - 3.0)
                .write(label)?;
        }

        // Render column labels
        for (idx, label) in self.data.column_labels.iter().enumerate() {
            let x = chart_x + idx as f64 * cell_width;

            // Rotate text for better fit
            page.text()
                .set_font(crate::Font::Helvetica, 9.0)
                .set_fill_color(theme.colors.text_secondary)
                .at(x + cell_width / 2.0 - 5.0, chart_y + 10.0)
                .write(label)?;
        }

        // Render legend
        if self.options.show_legend {
            self.render_legend(
                page,
                position,
                chart_x + chart_width + 10.0,
                chart_y + title_height,
                legend_width - 20.0,
                chart_height,
                min_val,
                max_val,
                theme,
            )?;
        }

        Ok(())
    }

    fn get_span(&self) -> ComponentSpan {
        self.config.span
    }
    fn set_span(&mut self, span: ComponentSpan) {
        self.config.span = span;
    }
    fn preferred_height(&self, _available_width: f64) -> f64 {
        300.0
    }
    fn component_type(&self) -> &'static str {
        "HeatMap"
    }
    fn complexity_score(&self) -> u8 {
        75
    }
}

/// HeatMap data structure
#[derive(Debug, Clone)]
pub struct HeatMapData {
    pub values: Vec<Vec<f64>>,
    pub row_labels: Vec<String>,
    pub column_labels: Vec<String>,
}

/// HeatMap configuration options
#[derive(Debug, Clone)]
pub struct HeatMapOptions {
    pub title: Option<String>,
    pub show_legend: bool,
    pub show_values: bool,
    pub cell_padding: f64,
}

impl Default for HeatMapOptions {
    fn default() -> Self {
        Self {
            title: None,
            show_legend: true,
            show_values: false,
            cell_padding: 2.0,
        }
    }
}

/// Color scale for heat maps
#[derive(Debug, Clone)]
pub struct ColorScale {
    pub colors: Vec<Color>,
    pub min_value: Option<f64>,
    pub max_value: Option<f64>,
}

impl Default for ColorScale {
    fn default() -> Self {
        Self {
            colors: vec![
                Color::hex("#ffffff"), // White for minimum
                Color::hex("#ff0000"), // Red for maximum
            ],
            min_value: None,
            max_value: None,
        }
    }
}

/// Builder for HeatMap
pub struct HeatMapBuilder;

impl HeatMapBuilder {
    pub fn new() -> Self {
        Self
    }
    pub fn build(self) -> HeatMap {
        HeatMap::new(HeatMapData {
            values: vec![],
            row_labels: vec![],
            column_labels: vec![],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_heatmap_data() -> HeatMapData {
        HeatMapData {
            values: vec![
                vec![1.0, 2.0, 3.0],
                vec![4.0, 5.0, 6.0],
                vec![7.0, 8.0, 9.0],
            ],
            row_labels: vec!["Row1".to_string(), "Row2".to_string(), "Row3".to_string()],
            column_labels: vec!["Col1".to_string(), "Col2".to_string(), "Col3".to_string()],
        }
    }

    #[test]
    fn test_heatmap_new() {
        let data = sample_heatmap_data();
        let heatmap = HeatMap::new(data.clone());

        assert_eq!(heatmap.data.values.len(), 3);
        assert_eq!(heatmap.data.row_labels.len(), 3);
        assert_eq!(heatmap.data.column_labels.len(), 3);
    }

    #[test]
    fn test_heatmap_with_options() {
        let data = sample_heatmap_data();
        let options = HeatMapOptions {
            title: Some("Test HeatMap".to_string()),
            show_legend: false,
            show_values: true,
            cell_padding: 5.0,
        };

        let heatmap = HeatMap::new(data).with_options(options.clone());

        assert_eq!(heatmap.options.title, Some("Test HeatMap".to_string()));
        assert!(!heatmap.options.show_legend);
        assert!(heatmap.options.show_values);
        assert_eq!(heatmap.options.cell_padding, 5.0);
    }

    #[test]
    fn test_heatmap_with_color_scale() {
        let data = sample_heatmap_data();
        let color_scale = ColorScale {
            colors: vec![Color::rgb(0.0, 0.0, 1.0), Color::rgb(1.0, 0.0, 0.0)],
            min_value: Some(0.0),
            max_value: Some(10.0),
        };

        let heatmap = HeatMap::new(data).with_color_scale(color_scale);

        assert_eq!(heatmap.color_scale.colors.len(), 2);
        assert_eq!(heatmap.color_scale.min_value, Some(0.0));
        assert_eq!(heatmap.color_scale.max_value, Some(10.0));
    }

    #[test]
    fn test_heatmap_options_default() {
        let options = HeatMapOptions::default();

        assert!(options.title.is_none());
        assert!(options.show_legend);
        assert!(!options.show_values);
        assert_eq!(options.cell_padding, 2.0);
    }

    #[test]
    fn test_color_scale_default() {
        let scale = ColorScale::default();

        assert_eq!(scale.colors.len(), 2);
        assert!(scale.min_value.is_none());
        assert!(scale.max_value.is_none());
    }

    #[test]
    fn test_heatmap_builder() {
        let builder = HeatMapBuilder::new();
        let heatmap = builder.build();

        assert!(heatmap.data.values.is_empty());
        assert!(heatmap.data.row_labels.is_empty());
        assert!(heatmap.data.column_labels.is_empty());
    }

    #[test]
    fn test_get_value_range_auto() {
        let data = sample_heatmap_data();
        let heatmap = HeatMap::new(data);

        let (min, max) = heatmap.get_value_range();

        assert_eq!(min, 1.0);
        assert_eq!(max, 9.0);
    }

    #[test]
    fn test_get_value_range_with_explicit_values() {
        let data = sample_heatmap_data();
        let color_scale = ColorScale {
            colors: vec![Color::white(), Color::rgb(1.0, 0.0, 0.0)],
            min_value: Some(-10.0),
            max_value: Some(20.0),
        };
        let heatmap = HeatMap::new(data).with_color_scale(color_scale);

        let (min, max) = heatmap.get_value_range();

        assert_eq!(min, -10.0);
        assert_eq!(max, 20.0);
    }

    #[test]
    fn test_interpolate_color_at_minimum() {
        let data = sample_heatmap_data();
        let heatmap = HeatMap::new(data);

        let color = heatmap.interpolate_color(0.0, 0.0, 100.0);

        // Should be close to first color in default scale (white)
        match color {
            Color::Rgb(r, g, b) => {
                assert!(r >= 0.9, "Red component should be high for white");
                assert!(g >= 0.9, "Green component should be high for white");
                assert!(b >= 0.9, "Blue component should be high for white");
            }
            _ => panic!("Expected RGB color"),
        }
    }

    #[test]
    fn test_interpolate_color_at_maximum() {
        let data = sample_heatmap_data();
        let heatmap = HeatMap::new(data);

        let color = heatmap.interpolate_color(100.0, 0.0, 100.0);

        // Should be close to last color in default scale (red)
        match color {
            Color::Rgb(r, g, b) => {
                assert!(r >= 0.9, "Red component should be high for red");
                assert!(g <= 0.1, "Green component should be low for red");
                assert!(b <= 0.1, "Blue component should be low for red");
            }
            _ => panic!("Expected RGB color"),
        }
    }

    #[test]
    fn test_interpolate_color_at_midpoint() {
        let data = sample_heatmap_data();
        let heatmap = HeatMap::new(data);

        let color = heatmap.interpolate_color(50.0, 0.0, 100.0);

        // Should be interpolated between white and red
        match color {
            Color::Rgb(r, g, b) => {
                assert!(r >= 0.9, "Red component should remain high");
                assert!(g >= 0.4 && g <= 0.6, "Green should be around 0.5");
                assert!(b >= 0.4 && b <= 0.6, "Blue should be around 0.5");
            }
            _ => panic!("Expected RGB color"),
        }
    }

    #[test]
    fn test_interpolate_color_same_min_max() {
        let data = sample_heatmap_data();
        let heatmap = HeatMap::new(data);

        // When min == max, should return first color
        let color = heatmap.interpolate_color(5.0, 5.0, 5.0);

        // Should be the first color in the scale
        assert!(matches!(color, Color::Rgb(_, _, _)));
    }

    #[test]
    fn test_interpolate_color_single_color_scale() {
        let data = sample_heatmap_data();
        let color_scale = ColorScale {
            colors: vec![Color::rgb(0.5, 0.5, 0.5)],
            min_value: None,
            max_value: None,
        };
        let heatmap = HeatMap::new(data).with_color_scale(color_scale);

        let color = heatmap.interpolate_color(50.0, 0.0, 100.0);

        match color {
            Color::Rgb(r, g, b) => {
                assert!((r - 0.5).abs() < 0.01);
                assert!((g - 0.5).abs() < 0.01);
                assert!((b - 0.5).abs() < 0.01);
            }
            _ => panic!("Expected RGB color"),
        }
    }

    #[test]
    fn test_is_dark_color_with_black() {
        let data = sample_heatmap_data();
        let heatmap = HeatMap::new(data);

        assert!(heatmap.is_dark_color(&Color::rgb(0.0, 0.0, 0.0)));
    }

    #[test]
    fn test_is_dark_color_with_white() {
        let data = sample_heatmap_data();
        let heatmap = HeatMap::new(data);

        assert!(!heatmap.is_dark_color(&Color::rgb(1.0, 1.0, 1.0)));
    }

    #[test]
    fn test_is_dark_color_with_red() {
        let data = sample_heatmap_data();
        let heatmap = HeatMap::new(data);

        // Pure red has luminance = 0.299, which is < 0.5
        assert!(heatmap.is_dark_color(&Color::rgb(1.0, 0.0, 0.0)));
    }

    #[test]
    fn test_is_dark_color_with_gray() {
        let data = sample_heatmap_data();
        let heatmap = HeatMap::new(data);

        // Gray(0.3) should be dark
        assert!(heatmap.is_dark_color(&Color::Gray(0.3)));
        // Gray(0.7) should be light
        assert!(!heatmap.is_dark_color(&Color::Gray(0.7)));
    }

    #[test]
    fn test_is_dark_color_with_cmyk() {
        let data = sample_heatmap_data();
        let heatmap = HeatMap::new(data);

        // CMYK black (0, 0, 0, 1) should be dark
        assert!(heatmap.is_dark_color(&Color::Cmyk(0.0, 0.0, 0.0, 1.0)));
        // CMYK white-ish (0, 0, 0, 0) should be light
        assert!(!heatmap.is_dark_color(&Color::Cmyk(0.0, 0.0, 0.0, 0.0)));
    }

    #[test]
    fn test_heatmap_data_creation() {
        let data = HeatMapData {
            values: vec![vec![1.0, 2.0], vec![3.0, 4.0]],
            row_labels: vec!["A".to_string(), "B".to_string()],
            column_labels: vec!["X".to_string(), "Y".to_string()],
        };

        assert_eq!(data.values.len(), 2);
        assert_eq!(data.values[0].len(), 2);
        assert_eq!(data.row_labels[0], "A");
        assert_eq!(data.column_labels[1], "Y");
    }

    #[test]
    fn test_component_span() {
        let data = sample_heatmap_data();
        let mut heatmap = HeatMap::new(data);

        // Default span
        let span = heatmap.get_span();
        assert_eq!(span.columns, 6);

        // Set new span
        heatmap.set_span(ComponentSpan::new(12));
        assert_eq!(heatmap.get_span().columns, 12);
    }

    #[test]
    fn test_component_type() {
        let data = sample_heatmap_data();
        let heatmap = HeatMap::new(data);

        assert_eq!(heatmap.component_type(), "HeatMap");
    }

    #[test]
    fn test_complexity_score() {
        let data = sample_heatmap_data();
        let heatmap = HeatMap::new(data);

        assert_eq!(heatmap.complexity_score(), 75);
    }

    #[test]
    fn test_preferred_height() {
        let data = sample_heatmap_data();
        let heatmap = HeatMap::new(data);

        assert_eq!(heatmap.preferred_height(1000.0), 300.0);
    }

    #[test]
    fn test_interpolate_color_multi_color_scale() {
        let data = sample_heatmap_data();
        let color_scale = ColorScale {
            colors: vec![
                Color::rgb(0.0, 0.0, 1.0), // Blue
                Color::rgb(0.0, 1.0, 0.0), // Green
                Color::rgb(1.0, 0.0, 0.0), // Red
            ],
            min_value: None,
            max_value: None,
        };
        let heatmap = HeatMap::new(data).with_color_scale(color_scale);

        // At 0%, should be blue
        let color_start = heatmap.interpolate_color(0.0, 0.0, 100.0);
        match color_start {
            Color::Rgb(r, g, b) => {
                assert!(r < 0.1);
                assert!(g < 0.1);
                assert!(b > 0.9);
            }
            _ => panic!("Expected RGB"),
        }

        // At 50%, should be green
        let color_mid = heatmap.interpolate_color(50.0, 0.0, 100.0);
        match color_mid {
            Color::Rgb(r, g, b) => {
                assert!(r < 0.1);
                assert!(g > 0.9);
                assert!(b < 0.1);
            }
            _ => panic!("Expected RGB"),
        }

        // At 100%, should be red
        let color_end = heatmap.interpolate_color(100.0, 0.0, 100.0);
        match color_end {
            Color::Rgb(r, g, b) => {
                assert!(r > 0.9);
                assert!(g < 0.1);
                assert!(b < 0.1);
            }
            _ => panic!("Expected RGB"),
        }
    }

    #[test]
    fn test_get_value_range_empty_data() {
        let data = HeatMapData {
            values: vec![],
            row_labels: vec![],
            column_labels: vec![],
        };
        let heatmap = HeatMap::new(data);

        let (min, max) = heatmap.get_value_range();

        // With empty data, should return infinity values
        assert!(min.is_infinite());
        assert!(max.is_infinite());
    }

    #[test]
    fn test_get_value_range_negative_values() {
        let data = HeatMapData {
            values: vec![vec![-10.0, -5.0], vec![0.0, 5.0]],
            row_labels: vec!["A".to_string(), "B".to_string()],
            column_labels: vec!["X".to_string(), "Y".to_string()],
        };
        let heatmap = HeatMap::new(data);

        let (min, max) = heatmap.get_value_range();

        assert_eq!(min, -10.0);
        assert_eq!(max, 5.0);
    }

    #[test]
    fn test_interpolate_color_clamping() {
        let data = sample_heatmap_data();
        let heatmap = HeatMap::new(data);

        // Value below min should clamp
        let color_below = heatmap.interpolate_color(-100.0, 0.0, 100.0);
        let color_at_min = heatmap.interpolate_color(0.0, 0.0, 100.0);

        // Both should produce similar colors (clamped to min)
        match (color_below, color_at_min) {
            (Color::Rgb(r1, g1, b1), Color::Rgb(r2, g2, b2)) => {
                assert!((r1 - r2).abs() < 0.01);
                assert!((g1 - g2).abs() < 0.01);
                assert!((b1 - b2).abs() < 0.01);
            }
            _ => panic!("Expected RGB colors"),
        }

        // Value above max should clamp
        let color_above = heatmap.interpolate_color(200.0, 0.0, 100.0);
        let color_at_max = heatmap.interpolate_color(100.0, 0.0, 100.0);

        match (color_above, color_at_max) {
            (Color::Rgb(r1, g1, b1), Color::Rgb(r2, g2, b2)) => {
                assert!((r1 - r2).abs() < 0.01);
                assert!((g1 - g2).abs() < 0.01);
                assert!((b1 - b2).abs() < 0.01);
            }
            _ => panic!("Expected RGB colors"),
        }
    }
}
