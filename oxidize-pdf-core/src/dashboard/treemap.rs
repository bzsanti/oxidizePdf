//! TreeMap Visualization Component
//!
//! This module implements tree maps for hierarchical data visualization,
//! showing nested rectangles proportional to data values.

use super::{
    component::ComponentConfig, ComponentPosition, ComponentSpan, DashboardComponent,
    DashboardTheme,
};
use crate::error::PdfError;
use crate::graphics::Color;
use crate::page::Page;

/// TreeMap visualization component
#[derive(Debug, Clone)]
pub struct TreeMap {
    /// Component configuration
    config: ComponentConfig,
    /// Tree map data
    data: Vec<TreeMapNode>,
    /// Configuration options
    options: TreeMapOptions,
}

impl TreeMap {
    /// Create a new tree map
    pub fn new(data: Vec<TreeMapNode>) -> Self {
        Self {
            config: ComponentConfig::new(ComponentSpan::new(6)), // Half width by default
            data,
            options: TreeMapOptions::default(),
        }
    }

    /// Set tree map options
    pub fn with_options(mut self, options: TreeMapOptions) -> Self {
        self.options = options;
        self
    }

    /// Simple squarified treemap layout (recursive)
    fn layout_nodes(
        &self,
        nodes: &[TreeMapNode],
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        rects: &mut Vec<(TreeMapNode, f64, f64, f64, f64)>,
    ) {
        if nodes.is_empty() || width <= 0.0 || height <= 0.0 {
            return;
        }

        let total: f64 = nodes.iter().map(|n| n.value).sum();
        if total <= 0.0 {
            return;
        }

        let mut current_x = x;
        let mut current_y = y;
        let mut remaining_width = width;
        let mut remaining_height = height;

        for node in nodes {
            let ratio = node.value / total;
            let area = width * height * ratio;

            // Decide whether to split horizontally or vertically
            let (rect_width, rect_height) = if remaining_width > remaining_height {
                // Split horizontally
                let w = area / remaining_height;
                (w.min(remaining_width), remaining_height)
            } else {
                // Split vertically
                let h = area / remaining_width;
                (remaining_width, h.min(remaining_height))
            };

            // Add padding
            let padding = self.options.padding;
            let rect_x = current_x + padding;
            let rect_y = current_y + padding;
            let rect_w = (rect_width - 2.0 * padding).max(0.0);
            let rect_h = (rect_height - 2.0 * padding).max(0.0);

            rects.push((node.clone(), rect_x, rect_y, rect_w, rect_h));

            // Update position for next rectangle
            if remaining_width > remaining_height {
                current_x += rect_width;
                remaining_width -= rect_width;
            } else {
                current_y += rect_height;
                remaining_height -= rect_height;
            }
        }
    }
}

impl DashboardComponent for TreeMap {
    fn render(
        &self,
        page: &mut Page,
        position: ComponentPosition,
        theme: &DashboardTheme,
    ) -> Result<(), PdfError> {
        let title = self.options.title.as_deref().unwrap_or("TreeMap");

        let title_height = 30.0;
        let plot_x = position.x;
        let plot_y = position.y;
        let plot_width = position.width;
        let plot_height = position.height - title_height;

        // Render title
        page.text()
            .set_font(crate::Font::HelveticaBold, theme.typography.heading_size)
            .set_fill_color(theme.colors.text_primary)
            .at(position.x, position.y + position.height - 15.0)
            .write(title)?;

        // Calculate layout
        let mut rects = Vec::new();
        self.layout_nodes(
            &self.data,
            plot_x,
            plot_y,
            plot_width,
            plot_height,
            &mut rects,
        );

        // Default colors if not specified
        let default_colors = vec![
            Color::hex("#1f77b4"),
            Color::hex("#ff7f0e"),
            Color::hex("#2ca02c"),
            Color::hex("#d62728"),
            Color::hex("#9467bd"),
            Color::hex("#8c564b"),
            Color::hex("#e377c2"),
            Color::hex("#7f7f7f"),
            Color::hex("#bcbd22"),
            Color::hex("#17becf"),
        ];

        // Render rectangles
        for (idx, (node, x, y, w, h)) in rects.iter().enumerate() {
            let color = node
                .color
                .unwrap_or(default_colors[idx % default_colors.len()]);

            // Draw rectangle
            page.graphics()
                .set_fill_color(color)
                .rect(*x, *y, *w, *h)
                .fill();

            // Draw border
            page.graphics()
                .set_stroke_color(Color::white())
                .set_line_width(1.5)
                .rect(*x, *y, *w, *h)
                .stroke();

            // Draw label if enabled and rectangle is large enough
            if self.options.show_labels && *w > 40.0 && *h > 20.0 {
                // Determine text color based on background
                let text_color = if self.is_dark_color(&color) {
                    Color::white()
                } else {
                    Color::black()
                };

                // Draw name
                page.text()
                    .set_font(crate::Font::HelveticaBold, 9.0)
                    .set_fill_color(text_color)
                    .at(x + 5.0, y + h - 15.0)
                    .write(&node.name)?;

                // Draw value
                page.text()
                    .set_font(crate::Font::Helvetica, 8.0)
                    .set_fill_color(text_color)
                    .at(x + 5.0, y + h - 28.0)
                    .write(&format!("{:.0}", node.value))?;
            }
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
        250.0
    }
    fn component_type(&self) -> &'static str {
        "TreeMap"
    }
    fn complexity_score(&self) -> u8 {
        70
    }
}

impl TreeMap {
    /// Check if a color is dark (for text contrast)
    fn is_dark_color(&self, color: &Color) -> bool {
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
}

/// TreeMap node data
#[derive(Debug, Clone)]
pub struct TreeMapNode {
    pub name: String,
    pub value: f64,
    pub color: Option<Color>,
    pub children: Vec<TreeMapNode>,
}

/// TreeMap options
#[derive(Debug, Clone)]
pub struct TreeMapOptions {
    pub title: Option<String>,
    pub show_labels: bool,
    pub padding: f64,
}

impl Default for TreeMapOptions {
    fn default() -> Self {
        Self {
            title: None,
            show_labels: true,
            padding: 2.0,
        }
    }
}

/// Builder for TreeMap
pub struct TreeMapBuilder;

impl TreeMapBuilder {
    pub fn new() -> Self {
        Self
    }
    pub fn build(self) -> TreeMap {
        TreeMap::new(vec![])
    }
}
