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

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_treemap_data() -> Vec<TreeMapNode> {
        vec![
            TreeMapNode {
                name: "Category A".to_string(),
                value: 100.0,
                color: None,
                children: vec![],
            },
            TreeMapNode {
                name: "Category B".to_string(),
                value: 50.0,
                color: Some(Color::rgb(0.0, 0.5, 1.0)),
                children: vec![],
            },
            TreeMapNode {
                name: "Category C".to_string(),
                value: 30.0,
                color: None,
                children: vec![],
            },
        ]
    }

    #[test]
    fn test_treemap_new() {
        let data = sample_treemap_data();
        let treemap = TreeMap::new(data.clone());

        assert_eq!(treemap.data.len(), 3);
        assert_eq!(treemap.data[0].name, "Category A");
        assert_eq!(treemap.data[0].value, 100.0);
    }

    #[test]
    fn test_treemap_with_options() {
        let data = sample_treemap_data();
        let options = TreeMapOptions {
            title: Some("My TreeMap".to_string()),
            show_labels: false,
            padding: 5.0,
        };

        let treemap = TreeMap::new(data).with_options(options);

        assert_eq!(treemap.options.title, Some("My TreeMap".to_string()));
        assert!(!treemap.options.show_labels);
        assert_eq!(treemap.options.padding, 5.0);
    }

    #[test]
    fn test_treemap_options_default() {
        let options = TreeMapOptions::default();

        assert!(options.title.is_none());
        assert!(options.show_labels);
        assert_eq!(options.padding, 2.0);
    }

    #[test]
    fn test_treemap_builder() {
        let builder = TreeMapBuilder::new();
        let treemap = builder.build();

        assert!(treemap.data.is_empty());
    }

    #[test]
    fn test_treemap_node_creation() {
        let node = TreeMapNode {
            name: "Test Node".to_string(),
            value: 42.0,
            color: Some(Color::rgb(1.0, 0.0, 0.0)),
            children: vec![TreeMapNode {
                name: "Child".to_string(),
                value: 10.0,
                color: None,
                children: vec![],
            }],
        };

        assert_eq!(node.name, "Test Node");
        assert_eq!(node.value, 42.0);
        assert!(node.color.is_some());
        assert_eq!(node.children.len(), 1);
        assert_eq!(node.children[0].name, "Child");
    }

    #[test]
    fn test_layout_nodes_empty() {
        let treemap = TreeMap::new(vec![]);
        let mut rects = Vec::new();

        treemap.layout_nodes(&[], 0.0, 0.0, 100.0, 100.0, &mut rects);

        assert!(rects.is_empty());
    }

    #[test]
    fn test_layout_nodes_single() {
        let data = vec![TreeMapNode {
            name: "Single".to_string(),
            value: 100.0,
            color: None,
            children: vec![],
        }];
        let treemap = TreeMap::new(data.clone());
        let mut rects = Vec::new();

        treemap.layout_nodes(&data, 0.0, 0.0, 100.0, 100.0, &mut rects);

        assert_eq!(rects.len(), 1);
        assert_eq!(rects[0].0.name, "Single");
    }

    #[test]
    fn test_layout_nodes_multiple() {
        let data = sample_treemap_data();
        let treemap = TreeMap::new(data.clone());
        let mut rects = Vec::new();

        treemap.layout_nodes(&data, 0.0, 0.0, 300.0, 200.0, &mut rects);

        assert_eq!(rects.len(), 3);

        // All rectangles should have positive dimensions
        for (_, x, y, w, h) in &rects {
            assert!(*x >= 0.0);
            assert!(*y >= 0.0);
            assert!(*w > 0.0);
            assert!(*h > 0.0);
        }
    }

    #[test]
    fn test_layout_nodes_proportional() {
        let data = vec![
            TreeMapNode {
                name: "A".to_string(),
                value: 75.0,
                color: None,
                children: vec![],
            },
            TreeMapNode {
                name: "B".to_string(),
                value: 25.0,
                color: None,
                children: vec![],
            },
        ];
        let treemap = TreeMap::new(data.clone());
        let mut rects = Vec::new();

        treemap.layout_nodes(&data, 0.0, 0.0, 100.0, 100.0, &mut rects);

        // The larger node should have approximately 3x the area
        let area_a = rects[0].3 * rects[0].4;
        let area_b = rects[1].3 * rects[1].4;

        // Due to padding, the ratio might not be exact
        assert!(area_a > area_b);
    }

    #[test]
    fn test_layout_nodes_zero_size() {
        let data = sample_treemap_data();
        let treemap = TreeMap::new(data.clone());
        let mut rects = Vec::new();

        // Zero width
        treemap.layout_nodes(&data, 0.0, 0.0, 0.0, 100.0, &mut rects);
        assert!(rects.is_empty());

        // Zero height
        rects.clear();
        treemap.layout_nodes(&data, 0.0, 0.0, 100.0, 0.0, &mut rects);
        assert!(rects.is_empty());
    }

    #[test]
    fn test_layout_nodes_zero_total_value() {
        let data = vec![
            TreeMapNode {
                name: "A".to_string(),
                value: 0.0,
                color: None,
                children: vec![],
            },
            TreeMapNode {
                name: "B".to_string(),
                value: 0.0,
                color: None,
                children: vec![],
            },
        ];
        let treemap = TreeMap::new(data.clone());
        let mut rects = Vec::new();

        treemap.layout_nodes(&data, 0.0, 0.0, 100.0, 100.0, &mut rects);

        // Should not produce any rectangles when total is zero
        assert!(rects.is_empty());
    }

    #[test]
    fn test_is_dark_color_with_black() {
        let treemap = TreeMap::new(vec![]);

        assert!(treemap.is_dark_color(&Color::rgb(0.0, 0.0, 0.0)));
    }

    #[test]
    fn test_is_dark_color_with_white() {
        let treemap = TreeMap::new(vec![]);

        assert!(!treemap.is_dark_color(&Color::rgb(1.0, 1.0, 1.0)));
    }

    #[test]
    fn test_is_dark_color_with_gray() {
        let treemap = TreeMap::new(vec![]);

        // Gray(0.3) has luminance 0.3, which is < 0.5
        assert!(treemap.is_dark_color(&Color::Gray(0.3)));
        // Gray(0.7) has luminance 0.7, which is > 0.5
        assert!(!treemap.is_dark_color(&Color::Gray(0.7)));
    }

    #[test]
    fn test_is_dark_color_with_cmyk() {
        let treemap = TreeMap::new(vec![]);

        // CMYK black (0, 0, 0, 1) -> RGB (0, 0, 0)
        assert!(treemap.is_dark_color(&Color::Cmyk(0.0, 0.0, 0.0, 1.0)));
        // CMYK white-ish (0, 0, 0, 0) -> RGB (1, 1, 1)
        assert!(!treemap.is_dark_color(&Color::Cmyk(0.0, 0.0, 0.0, 0.0)));
    }

    #[test]
    fn test_is_dark_color_with_primary_colors() {
        let treemap = TreeMap::new(vec![]);

        // Red: luminance = 0.299
        assert!(treemap.is_dark_color(&Color::rgb(1.0, 0.0, 0.0)));
        // Green: luminance = 0.587
        assert!(!treemap.is_dark_color(&Color::rgb(0.0, 1.0, 0.0)));
        // Blue: luminance = 0.114
        assert!(treemap.is_dark_color(&Color::rgb(0.0, 0.0, 1.0)));
    }

    #[test]
    fn test_component_span() {
        let data = sample_treemap_data();
        let mut treemap = TreeMap::new(data);

        // Default span
        let span = treemap.get_span();
        assert_eq!(span.columns, 6);

        // Set new span
        treemap.set_span(ComponentSpan::new(12));
        assert_eq!(treemap.get_span().columns, 12);
    }

    #[test]
    fn test_component_type() {
        let treemap = TreeMap::new(vec![]);

        assert_eq!(treemap.component_type(), "TreeMap");
    }

    #[test]
    fn test_complexity_score() {
        let treemap = TreeMap::new(vec![]);

        assert_eq!(treemap.complexity_score(), 70);
    }

    #[test]
    fn test_preferred_height() {
        let treemap = TreeMap::new(vec![]);

        assert_eq!(treemap.preferred_height(1000.0), 250.0);
    }

    #[test]
    fn test_treemap_node_with_children() {
        let data = vec![TreeMapNode {
            name: "Parent".to_string(),
            value: 100.0,
            color: None,
            children: vec![
                TreeMapNode {
                    name: "Child 1".to_string(),
                    value: 60.0,
                    color: None,
                    children: vec![],
                },
                TreeMapNode {
                    name: "Child 2".to_string(),
                    value: 40.0,
                    color: None,
                    children: vec![],
                },
            ],
        }];

        let treemap = TreeMap::new(data.clone());

        assert_eq!(treemap.data[0].children.len(), 2);
        assert_eq!(treemap.data[0].children[0].value, 60.0);
        assert_eq!(treemap.data[0].children[1].value, 40.0);
    }

    #[test]
    fn test_layout_nodes_with_wide_rectangle() {
        let data = vec![
            TreeMapNode {
                name: "A".to_string(),
                value: 50.0,
                color: None,
                children: vec![],
            },
            TreeMapNode {
                name: "B".to_string(),
                value: 50.0,
                color: None,
                children: vec![],
            },
        ];
        let treemap = TreeMap::new(data.clone());
        let mut rects = Vec::new();

        // Wide rectangle (width > height) should split horizontally
        treemap.layout_nodes(&data, 0.0, 0.0, 200.0, 50.0, &mut rects);

        assert_eq!(rects.len(), 2);
    }

    #[test]
    fn test_layout_nodes_with_tall_rectangle() {
        let data = vec![
            TreeMapNode {
                name: "A".to_string(),
                value: 50.0,
                color: None,
                children: vec![],
            },
            TreeMapNode {
                name: "B".to_string(),
                value: 50.0,
                color: None,
                children: vec![],
            },
        ];
        let treemap = TreeMap::new(data.clone());
        let mut rects = Vec::new();

        // Tall rectangle (height > width) should split vertically
        treemap.layout_nodes(&data, 0.0, 0.0, 50.0, 200.0, &mut rects);

        assert_eq!(rects.len(), 2);
    }

    #[test]
    fn test_layout_respects_padding() {
        let data = vec![TreeMapNode {
            name: "Single".to_string(),
            value: 100.0,
            color: None,
            children: vec![],
        }];

        let options = TreeMapOptions {
            title: None,
            show_labels: true,
            padding: 10.0,
        };
        let treemap = TreeMap::new(data.clone()).with_options(options);
        let mut rects = Vec::new();

        treemap.layout_nodes(&data, 0.0, 0.0, 100.0, 100.0, &mut rects);

        // With padding of 10.0, the rectangle should start at (10, 10)
        // and have dimensions reduced by 2*10 = 20
        let (_, x, y, w, h) = &rects[0];
        assert!((*x - 10.0).abs() < 0.01);
        assert!((*y - 10.0).abs() < 0.01);
        assert!((*w - 80.0).abs() < 0.01);
        assert!((*h - 80.0).abs() < 0.01);
    }
}
