//! Cell styling system for advanced tables

use crate::graphics::Color;
use crate::text::Font;
use crate::CoordinateSystem;

/// Border styles for table cells
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum BorderStyle {
    /// No border
    None,
    /// Solid line border
    #[default]
    Solid,
    /// Dashed line border
    Dashed,
    /// Dotted line border  
    Dotted,
    /// Double line border
    Double,
}

/// Cell alignment options
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum CellAlignment {
    /// Left-aligned content
    #[default]
    Left,
    /// Center-aligned content
    Center,
    /// Right-aligned content
    Right,
    /// Justified content (for multi-line text)
    Justify,
}

/// Padding configuration for cells
#[derive(Debug, Clone, Copy)]
pub struct Padding {
    /// Top padding
    pub top: f64,
    /// Right padding
    pub right: f64,
    /// Bottom padding
    pub bottom: f64,
    /// Left padding
    pub left: f64,
}

impl Padding {
    /// Create padding with individual values (top, right, bottom, left)
    pub fn new(top: f64, right: f64, bottom: f64, left: f64) -> Self {
        Self {
            top,
            right,
            bottom,
            left,
        }
    }

    /// Create uniform padding on all sides
    pub fn uniform(padding: f64) -> Self {
        Self {
            top: padding,
            right: padding,
            bottom: padding,
            left: padding,
        }
    }

    /// Create padding with horizontal and vertical values
    pub fn symmetric(horizontal: f64, vertical: f64) -> Self {
        Self {
            top: vertical,
            right: horizontal,
            bottom: vertical,
            left: horizontal,
        }
    }

    /// Create padding with individual values for each side
    pub fn individual(top: f64, right: f64, bottom: f64, left: f64) -> Self {
        Self {
            top,
            right,
            bottom,
            left,
        }
    }

    /// Get total horizontal padding (left + right)
    pub fn horizontal_total(&self) -> f64 {
        self.left + self.right
    }

    /// Get total vertical padding (top + bottom)
    pub fn vertical_total(&self) -> f64 {
        self.top + self.bottom
    }

    pub fn pad_vertically(&self, coordinate_system: &CoordinateSystem, y: f64) -> f64 {
        let mut padded = y;
        match coordinate_system {
            CoordinateSystem::PdfStandard | CoordinateSystem::Custom(_) => {
                padded -= self.top;
                padded += self.bottom;
            }
            CoordinateSystem::ScreenSpace => {
                padded += self.top;
                padded -= self.bottom;
            }
        }

        padded
    }

    pub fn pad_horizontally(&self, x: f64) -> f64 {
        let mut padded = x;
        padded -= self.right;
        padded += self.left;

        padded
    }
}

impl Default for Padding {
    fn default() -> Self {
        Self::uniform(4.0)
    }
}

/// Comprehensive cell styling configuration
#[derive(Debug, Clone)]
pub struct CellStyle {
    /// Background color of the cell
    pub background_color: Option<Color>,
    /// Text color
    pub text_color: Option<Color>,
    /// Font to use for text
    pub font: Option<Font>,
    /// Font size
    pub font_size: Option<f64>,
    /// Cell padding
    pub padding: Padding,
    /// Text alignment within the cell
    pub alignment: CellAlignment,
    /// Border style configuration
    pub border: BorderConfiguration,
    /// Simple border style (for backward compatibility)
    pub border_style: BorderStyle,
    /// Whether text should wrap within the cell
    pub text_wrap: bool,
    /// Minimum cell height
    pub min_height: Option<f64>,
    /// Maximum cell height (text will be clipped if exceeded)
    pub max_height: Option<f64>,
}

/// Border configuration for cells
#[derive(Debug, Clone)]
pub struct BorderConfiguration {
    /// Top border
    pub top: BorderEdge,
    /// Right border
    pub right: BorderEdge,
    /// Bottom border
    pub bottom: BorderEdge,
    /// Left border
    pub left: BorderEdge,
}

/// Individual border edge configuration
#[derive(Debug, Clone)]
pub struct BorderEdge {
    /// Border style
    pub style: BorderStyle,
    /// Border width
    pub width: f64,
    /// Border color
    pub color: Color,
}

impl BorderEdge {
    /// Create a new border edge
    pub fn new(style: BorderStyle, width: f64, color: Color) -> Self {
        Self {
            style,
            width,
            color,
        }
    }

    /// Create a solid black border edge
    pub fn solid(width: f64) -> Self {
        Self::new(BorderStyle::Solid, width, Color::black())
    }

    /// Create a dashed border edge
    pub fn dashed(width: f64, color: Color) -> Self {
        Self::new(BorderStyle::Dashed, width, color)
    }

    /// Create a dotted border edge
    pub fn dotted(width: f64, color: Color) -> Self {
        Self::new(BorderStyle::Dotted, width, color)
    }

    /// No border
    pub fn none() -> Self {
        Self::new(BorderStyle::None, 0.0, Color::black())
    }
}

impl Default for BorderEdge {
    fn default() -> Self {
        Self::solid(1.0)
    }
}

impl BorderConfiguration {
    /// Create a new border configuration
    pub fn new() -> Self {
        Self {
            top: BorderEdge::default(),
            right: BorderEdge::default(),
            bottom: BorderEdge::default(),
            left: BorderEdge::default(),
        }
    }

    /// Create uniform border on all sides
    pub fn uniform(edge: BorderEdge) -> Self {
        Self {
            top: edge.clone(),
            right: edge.clone(),
            bottom: edge.clone(),
            left: edge,
        }
    }

    /// Create border with only specific edges
    pub fn edges(top: bool, right: bool, bottom: bool, left: bool) -> Self {
        let solid_edge = BorderEdge::solid(1.0);
        let no_edge = BorderEdge::none();

        Self {
            top: if top {
                solid_edge.clone()
            } else {
                no_edge.clone()
            },
            right: if right {
                solid_edge.clone()
            } else {
                no_edge.clone()
            },
            bottom: if bottom {
                solid_edge.clone()
            } else {
                no_edge.clone()
            },
            left: if left { solid_edge } else { no_edge },
        }
    }

    /// No borders
    pub fn none() -> Self {
        let no_edge = BorderEdge::none();
        Self {
            top: no_edge.clone(),
            right: no_edge.clone(),
            bottom: no_edge.clone(),
            left: no_edge,
        }
    }
}

impl Default for BorderConfiguration {
    fn default() -> Self {
        Self::new()
    }
}

impl CellStyle {
    /// Create a new default cell style
    pub fn new() -> Self {
        Self {
            background_color: None,
            text_color: Some(Color::black()),
            font: Some(Font::Helvetica),
            font_size: Some(12.0),
            padding: Padding::default(),
            alignment: CellAlignment::Left,
            border: BorderConfiguration::default(),
            border_style: BorderStyle::Solid,
            text_wrap: true,
            min_height: None,
            max_height: None,
        }
    }

    /// Set background color
    pub fn background_color(mut self, color: Color) -> Self {
        self.background_color = Some(color);
        self
    }

    /// Set text color
    pub fn text_color(mut self, color: Color) -> Self {
        self.text_color = Some(color);
        self
    }

    /// Set font
    pub fn font(mut self, font: Font) -> Self {
        self.font = Some(font);
        self
    }

    /// Set font size
    pub fn font_size(mut self, size: f64) -> Self {
        self.font_size = Some(size);
        self
    }

    /// Set padding
    pub fn padding(mut self, padding: Padding) -> Self {
        self.padding = padding;
        self
    }

    /// Set alignment
    pub fn alignment(mut self, alignment: CellAlignment) -> Self {
        self.alignment = alignment;
        self
    }

    /// Set border configuration
    pub fn border_config(mut self, border: BorderConfiguration) -> Self {
        self.border = border;
        self
    }

    /// Set simple border (style, width, color) - used by tests
    pub fn border(mut self, style: BorderStyle, width: f64, color: Color) -> Self {
        self.border_style = style;
        self.border = BorderConfiguration::uniform(BorderEdge::new(style, width, color));
        self
    }

    /// Enable or disable text wrapping
    pub fn text_wrap(mut self, wrap: bool) -> Self {
        self.text_wrap = wrap;
        self
    }

    /// Set minimum cell height
    pub fn min_height(mut self, height: f64) -> Self {
        self.min_height = Some(height);
        self
    }

    /// Set maximum cell height
    pub fn max_height(mut self, height: f64) -> Self {
        self.max_height = Some(height);
        self
    }

    /// Create a header style (bold, centered, with background)
    pub fn header() -> Self {
        Self::new()
            .font(Font::HelveticaBold)
            .font_size(14.0)
            .alignment(CellAlignment::Center)
            .background_color(Color::rgb(0.9, 0.9, 0.9))
            .padding(Padding::uniform(8.0))
    }

    /// Create a data cell style (left-aligned, normal font)
    pub fn data() -> Self {
        Self::new()
            .font(Font::Helvetica)
            .font_size(12.0)
            .alignment(CellAlignment::Left)
            .padding(Padding::uniform(6.0))
    }

    /// Create a numeric cell style (right-aligned, monospace)
    pub fn numeric() -> Self {
        Self::new()
            .font(Font::Courier)
            .font_size(11.0)
            .alignment(CellAlignment::Right)
            .padding(Padding::uniform(6.0))
    }

    /// Create an alternating row style (with light background)
    pub fn alternating() -> Self {
        Self::data().background_color(Color::rgb(0.97, 0.97, 0.97))
    }
}

impl Default for CellStyle {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pad_vertically() {
        let cs = CoordinateSystem::PdfStandard;
        let padding = Padding::new(6.0, 0.0, 2.0, 2.0);

        assert_eq!(padding.pad_vertically(&cs, 100.0), 96.0);
        assert_eq!(padding.pad_vertically(&cs, 50.0), 46.0);

        let cs = CoordinateSystem::ScreenSpace;
        let padding = Padding::new(6.0, 0.0, 2.0, 2.0);

        assert_eq!(padding.pad_vertically(&cs, 100.0), 104.0);
        assert_eq!(padding.pad_vertically(&cs, 50.0), 54.0);
    }

    #[test]
    fn test_pad_horizontally() {
        let padding = Padding::new(6.0, 12.0, 2.0, 2.0);

        assert_eq!(padding.pad_horizontally(100.0), 90.0);
        assert_eq!(padding.pad_horizontally(50.0), 40.0);
    }
}
