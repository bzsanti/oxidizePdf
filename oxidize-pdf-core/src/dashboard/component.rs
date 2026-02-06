//! Dashboard Component System
//!
//! This module defines the trait and types for dashboard components. All dashboard
//! elements (KPI cards, charts, tables, etc.) implement the DashboardComponent trait
//! to ensure consistent rendering and layout behavior.

use super::theme::DashboardTheme;
use crate::error::PdfError;
use crate::graphics::Point;
use crate::page::Page;

/// Trait that all dashboard components must implement
pub trait DashboardComponent: std::fmt::Debug + DashboardComponentClone {
    /// Render the component to a PDF page at the specified position
    fn render(
        &self,
        page: &mut Page,
        position: ComponentPosition,
        theme: &DashboardTheme,
    ) -> Result<(), PdfError>;

    /// Get the column span for this component (1-12)
    fn get_span(&self) -> ComponentSpan;

    /// Set the column span for this component
    fn set_span(&mut self, span: ComponentSpan);

    /// Get the preferred height for this component in points
    fn preferred_height(&self, available_width: f64) -> f64;

    /// Get the minimum width required for this component
    fn minimum_width(&self) -> f64 {
        50.0 // Default minimum width
    }

    /// Estimate rendering time in milliseconds
    fn estimated_render_time_ms(&self) -> u32 {
        10 // Default estimate
    }

    /// Estimate memory usage in MB
    fn estimated_memory_mb(&self) -> f64 {
        0.1 // Default estimate
    }

    /// Get complexity score (0-100)
    fn complexity_score(&self) -> u8 {
        25 // Default complexity
    }

    /// Get component type name for debugging
    fn component_type(&self) -> &'static str;

    /// Validate component configuration
    fn validate(&self) -> Result<(), PdfError> {
        // Default validation - components can override
        if self.get_span().columns < 1 || self.get_span().columns > 12 {
            return Err(PdfError::InvalidOperation(format!(
                "Invalid span: {}. Must be 1-12",
                self.get_span().columns
            )));
        }
        Ok(())
    }
}

/// Helper trait for cloning dashboard components
pub trait DashboardComponentClone {
    fn clone_box(&self) -> Box<dyn DashboardComponent>;
}

impl<T> DashboardComponentClone for T
where
    T: 'static + DashboardComponent + Clone,
{
    fn clone_box(&self) -> Box<dyn DashboardComponent> {
        Box::new(self.clone())
    }
}

impl Clone for Box<dyn DashboardComponent> {
    fn clone(&self) -> Box<dyn DashboardComponent> {
        self.clone_box()
    }
}

/// Position and dimensions for a component within the dashboard grid
#[derive(Debug, Clone, Copy)]
pub struct ComponentPosition {
    /// X coordinate in points
    pub x: f64,
    /// Y coordinate in points
    pub y: f64,
    /// Width in points
    pub width: f64,
    /// Height in points
    pub height: f64,
}

impl ComponentPosition {
    /// Create a new component position
    pub fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Get the center point of this position
    pub fn center(&self) -> Point {
        Point::new(self.x + self.width / 2.0, self.y + self.height / 2.0)
    }

    /// Get the top-left corner
    pub fn top_left(&self) -> Point {
        Point::new(self.x, self.y + self.height)
    }

    /// Get the bottom-right corner
    pub fn bottom_right(&self) -> Point {
        Point::new(self.x + self.width, self.y)
    }

    /// Create a position with padding applied
    pub fn with_padding(&self, padding: f64) -> Self {
        Self {
            x: self.x + padding,
            y: self.y + padding,
            width: self.width - 2.0 * padding,
            height: self.height - 2.0 * padding,
        }
    }

    /// Check if this position contains a point
    pub fn contains(&self, point: Point) -> bool {
        point.x >= self.x
            && point.x <= self.x + self.width
            && point.y >= self.y
            && point.y <= self.y + self.height
    }

    /// Get aspect ratio (width/height)
    pub fn aspect_ratio(&self) -> f64 {
        if self.height > 0.0 {
            self.width / self.height
        } else {
            1.0
        }
    }
}

/// Column span configuration for grid layout
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ComponentSpan {
    /// Number of columns to span (1-12)
    pub columns: u8,
    /// Optional row span for multi-row components
    pub rows: Option<u8>,
}

impl ComponentSpan {
    /// Create a new component span
    pub fn new(columns: u8) -> Self {
        Self {
            columns: columns.clamp(1, 12),
            rows: None,
        }
    }

    /// Create a span with both column and row specification
    pub fn with_rows(columns: u8, rows: u8) -> Self {
        Self {
            columns: columns.clamp(1, 12),
            rows: Some(rows.max(1)),
        }
    }

    /// Get column span as a fraction (0.0-1.0)
    pub fn as_fraction(&self) -> f64 {
        self.columns as f64 / 12.0
    }

    /// Check if this is a full-width component
    pub fn is_full_width(&self) -> bool {
        self.columns == 12
    }

    /// Check if this is a half-width component
    pub fn is_half_width(&self) -> bool {
        self.columns == 6
    }

    /// Check if this is a quarter-width component
    pub fn is_quarter_width(&self) -> bool {
        self.columns == 3
    }
}

impl From<u8> for ComponentSpan {
    fn from(columns: u8) -> Self {
        Self::new(columns)
    }
}

/// Component alignment options within its allocated space
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComponentAlignment {
    /// Align to the left/top of the space
    Start,
    /// Center within the space
    Center,
    /// Align to the right/bottom of the space
    End,
    /// Stretch to fill the entire space
    Stretch,
}

impl Default for ComponentAlignment {
    fn default() -> Self {
        Self::Stretch
    }
}

/// Component margin configuration
#[derive(Debug, Clone, Copy)]
pub struct ComponentMargin {
    /// Top margin in points
    pub top: f64,
    /// Right margin in points
    pub right: f64,
    /// Bottom margin in points
    pub bottom: f64,
    /// Left margin in points
    pub left: f64,
}

impl ComponentMargin {
    /// Create uniform margin
    pub fn uniform(margin: f64) -> Self {
        Self {
            top: margin,
            right: margin,
            bottom: margin,
            left: margin,
        }
    }

    /// Create symmetric margin (vertical, horizontal)
    pub fn symmetric(vertical: f64, horizontal: f64) -> Self {
        Self {
            top: vertical,
            right: horizontal,
            bottom: vertical,
            left: horizontal,
        }
    }

    /// Create individual margins
    pub fn new(top: f64, right: f64, bottom: f64, left: f64) -> Self {
        Self {
            top,
            right,
            bottom,
            left,
        }
    }

    /// Get total horizontal margin
    pub fn horizontal(&self) -> f64 {
        self.left + self.right
    }

    /// Get total vertical margin
    pub fn vertical(&self) -> f64 {
        self.top + self.bottom
    }
}

impl Default for ComponentMargin {
    fn default() -> Self {
        Self::uniform(8.0) // 8pt default margin
    }
}

/// Base component configuration shared by all dashboard components
#[derive(Debug, Clone)]
pub struct ComponentConfig {
    /// Column span in the grid
    pub span: ComponentSpan,
    /// Component alignment
    pub alignment: ComponentAlignment,
    /// Component margins
    pub margin: ComponentMargin,
    /// Optional custom ID for the component
    pub id: Option<String>,
    /// Whether the component is visible
    pub visible: bool,
    /// Custom CSS-like classes for advanced styling
    pub classes: Vec<String>,
}

impl ComponentConfig {
    /// Create a new component config with default values
    pub fn new(span: ComponentSpan) -> Self {
        Self {
            span,
            alignment: ComponentAlignment::default(),
            margin: ComponentMargin::default(),
            id: None,
            visible: true,
            classes: Vec::new(),
        }
    }

    /// Set component alignment
    pub fn with_alignment(mut self, alignment: ComponentAlignment) -> Self {
        self.alignment = alignment;
        self
    }

    /// Set component margin
    pub fn with_margin(mut self, margin: ComponentMargin) -> Self {
        self.margin = margin;
        self
    }

    /// Set component ID
    pub fn with_id(mut self, id: String) -> Self {
        self.id = Some(id);
        self
    }

    /// Add CSS-like class
    pub fn with_class(mut self, class: String) -> Self {
        self.classes.push(class);
        self
    }

    /// Set visibility
    pub fn with_visibility(mut self, visible: bool) -> Self {
        self.visible = visible;
        self
    }
}

impl Default for ComponentConfig {
    fn default() -> Self {
        Self::new(ComponentSpan::new(12)) // Full width by default
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_component_span() {
        let span = ComponentSpan::new(6);
        assert_eq!(span.columns, 6);
        assert_eq!(span.as_fraction(), 0.5);
        assert!(span.is_half_width());
        assert!(!span.is_full_width());
    }

    #[test]
    fn test_component_span_bounds() {
        let span_too_large = ComponentSpan::new(15);
        assert_eq!(span_too_large.columns, 12);

        let span_too_small = ComponentSpan::new(0);
        assert_eq!(span_too_small.columns, 1);
    }

    #[test]
    fn test_component_position() {
        let pos = ComponentPosition::new(100.0, 200.0, 300.0, 400.0);
        let center = pos.center();

        assert_eq!(center.x, 250.0);
        assert_eq!(center.y, 400.0);
        assert_eq!(pos.aspect_ratio(), 0.75);
    }

    #[test]
    fn test_component_margin() {
        let margin = ComponentMargin::uniform(10.0);
        assert_eq!(margin.horizontal(), 20.0);
        assert_eq!(margin.vertical(), 20.0);

        let asymmetric = ComponentMargin::symmetric(5.0, 8.0);
        assert_eq!(asymmetric.vertical(), 10.0);
        assert_eq!(asymmetric.horizontal(), 16.0);
    }

    #[test]
    fn test_component_config() {
        let config = ComponentConfig::new(ComponentSpan::new(6))
            .with_id("test-component".to_string())
            .with_alignment(ComponentAlignment::Center)
            .with_class("highlight".to_string());

        assert_eq!(config.span.columns, 6);
        assert_eq!(config.id, Some("test-component".to_string()));
        assert_eq!(config.alignment, ComponentAlignment::Center);
        assert!(config.classes.contains(&"highlight".to_string()));
    }

    #[test]
    fn test_component_position_top_left() {
        let pos = ComponentPosition::new(100.0, 200.0, 300.0, 400.0);
        let top_left = pos.top_left();
        assert_eq!(top_left.x, 100.0);
        assert_eq!(top_left.y, 600.0); // y + height
    }

    #[test]
    fn test_component_position_bottom_right() {
        let pos = ComponentPosition::new(100.0, 200.0, 300.0, 400.0);
        let bottom_right = pos.bottom_right();
        assert_eq!(bottom_right.x, 400.0); // x + width
        assert_eq!(bottom_right.y, 200.0);
    }

    #[test]
    fn test_component_position_with_padding() {
        let pos = ComponentPosition::new(100.0, 200.0, 300.0, 400.0);
        let padded = pos.with_padding(10.0);

        assert_eq!(padded.x, 110.0);
        assert_eq!(padded.y, 210.0);
        assert_eq!(padded.width, 280.0); // 300 - 2*10
        assert_eq!(padded.height, 380.0); // 400 - 2*10
    }

    #[test]
    fn test_component_position_contains() {
        let pos = ComponentPosition::new(100.0, 200.0, 300.0, 400.0);

        // Point inside
        assert!(pos.contains(Point::new(200.0, 300.0)));

        // Point on edges
        assert!(pos.contains(Point::new(100.0, 200.0))); // bottom-left
        assert!(pos.contains(Point::new(400.0, 600.0))); // top-right

        // Point outside
        assert!(!pos.contains(Point::new(50.0, 300.0))); // left
        assert!(!pos.contains(Point::new(500.0, 300.0))); // right
        assert!(!pos.contains(Point::new(200.0, 100.0))); // below
        assert!(!pos.contains(Point::new(200.0, 700.0))); // above
    }

    #[test]
    fn test_component_position_aspect_ratio_zero_height() {
        let pos = ComponentPosition::new(100.0, 200.0, 300.0, 0.0);
        assert_eq!(pos.aspect_ratio(), 1.0); // Default when height is 0
    }

    #[test]
    fn test_component_span_with_rows() {
        let span = ComponentSpan::with_rows(6, 2);
        assert_eq!(span.columns, 6);
        assert_eq!(span.rows, Some(2));

        // Test clamping
        let span_clamped = ComponentSpan::with_rows(15, 0);
        assert_eq!(span_clamped.columns, 12);
        assert_eq!(span_clamped.rows, Some(1));
    }

    #[test]
    fn test_component_span_is_quarter_width() {
        let span = ComponentSpan::new(3);
        assert!(span.is_quarter_width());
        assert!(!span.is_half_width());
        assert!(!span.is_full_width());
    }

    #[test]
    fn test_component_span_from_u8() {
        let span: ComponentSpan = 4u8.into();
        assert_eq!(span.columns, 4);
        assert!(span.rows.is_none());
    }

    #[test]
    fn test_component_alignment_debug() {
        let alignments = vec![
            ComponentAlignment::Start,
            ComponentAlignment::Center,
            ComponentAlignment::End,
            ComponentAlignment::Stretch,
        ];

        for alignment in alignments {
            let debug_str = format!("{:?}", alignment);
            assert!(!debug_str.is_empty());
        }
    }

    #[test]
    fn test_component_alignment_default() {
        let default = ComponentAlignment::default();
        assert_eq!(default, ComponentAlignment::Stretch);
    }

    #[test]
    fn test_component_margin_new() {
        let margin = ComponentMargin::new(1.0, 2.0, 3.0, 4.0);
        assert_eq!(margin.top, 1.0);
        assert_eq!(margin.right, 2.0);
        assert_eq!(margin.bottom, 3.0);
        assert_eq!(margin.left, 4.0);
    }

    #[test]
    fn test_component_margin_default() {
        let default = ComponentMargin::default();
        assert_eq!(default.top, 8.0);
        assert_eq!(default.right, 8.0);
        assert_eq!(default.bottom, 8.0);
        assert_eq!(default.left, 8.0);
    }

    #[test]
    fn test_component_config_default() {
        let default = ComponentConfig::default();
        assert_eq!(default.span.columns, 12);
        assert_eq!(default.alignment, ComponentAlignment::Stretch);
        assert!(default.visible);
        assert!(default.classes.is_empty());
        assert!(default.id.is_none());
    }

    #[test]
    fn test_component_config_with_margin() {
        let config =
            ComponentConfig::new(ComponentSpan::new(6)).with_margin(ComponentMargin::uniform(16.0));

        assert_eq!(config.margin.top, 16.0);
        assert_eq!(config.margin.horizontal(), 32.0);
    }

    #[test]
    fn test_component_config_with_visibility() {
        let config = ComponentConfig::new(ComponentSpan::new(6)).with_visibility(false);

        assert!(!config.visible);
    }

    #[test]
    fn test_component_config_clone() {
        let config = ComponentConfig::new(ComponentSpan::new(6))
            .with_id("test".to_string())
            .with_class("class1".to_string());

        let cloned = config.clone();
        assert_eq!(config.span, cloned.span);
        assert_eq!(config.id, cloned.id);
        assert_eq!(config.classes.len(), cloned.classes.len());
    }

    #[test]
    fn test_component_position_clone_copy() {
        let pos = ComponentPosition::new(10.0, 20.0, 30.0, 40.0);
        let cloned = pos.clone();
        let copied = pos;

        assert_eq!(pos.x, cloned.x);
        assert_eq!(pos.y, copied.y);
    }

    #[test]
    fn test_component_span_equality() {
        let span1 = ComponentSpan::new(6);
        let span2 = ComponentSpan::new(6);
        let span3 = ComponentSpan::new(8);

        assert_eq!(span1, span2);
        assert_ne!(span1, span3);
    }

    #[test]
    fn test_component_margin_clone_copy() {
        let margin = ComponentMargin::new(1.0, 2.0, 3.0, 4.0);
        let cloned = margin.clone();
        let copied = margin;

        assert_eq!(margin.top, cloned.top);
        assert_eq!(margin.left, copied.left);
    }
}
