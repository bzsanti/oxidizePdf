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
}

impl DashboardComponent for HeatMap {
    fn render(
        &self,
        _page: &mut Page,
        _position: ComponentPosition,
        _theme: &DashboardTheme,
    ) -> Result<(), PdfError> {
        // Implementation placeholder - full implementation would require complex matrix rendering
        let _title = self.options.title.as_deref().unwrap_or("HeatMap");

        // Render title
        // Placeholder: page.add_text replaced

        // Draw placeholder rectangle
        // Placeholder: page graphics operation
        // Placeholder: page graphics operation
        // Placeholder: page graphics operation
        // Placeholder: page graphics operation
        // Placeholder: page graphics operation

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
