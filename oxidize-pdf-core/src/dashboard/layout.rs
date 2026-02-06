//! Dashboard Layout System - 12-Column Grid with Responsive Design
//!
//! This module implements a CSS Grid-like system for positioning dashboard components
//! in a 12-column responsive grid. It handles component positioning, row management,
//! spacing, and responsive breakpoints.

use super::{ComponentPosition, ComponentSpan, DashboardComponent, DashboardConfig};
use crate::error::PdfError;
use std::collections::HashMap;

/// Main layout manager for dashboard components
#[derive(Debug, Clone)]
pub struct DashboardLayout {
    /// Layout configuration
    config: DashboardConfig,
    /// Grid system
    grid: GridSystem,
    /// Component positions cache
    position_cache: HashMap<String, ComponentPosition>,
}

impl DashboardLayout {
    /// Create a new dashboard layout
    pub fn new(config: DashboardConfig) -> Self {
        let grid = GridSystem::new(12, config.column_gutter, config.row_gutter);

        Self {
            config,
            grid,
            position_cache: HashMap::new(),
        }
    }

    /// Calculate the content area based on page bounds and configuration
    pub fn calculate_content_area(
        &self,
        page_bounds: (f64, f64, f64, f64),
    ) -> (f64, f64, f64, f64) {
        let (page_x, page_y, page_width, page_height) = page_bounds;
        let (margin_top, margin_right, margin_bottom, margin_left) = self.config.margins;

        // Calculate basic content area
        let mut content_x = page_x + margin_left;
        let content_y = page_y + margin_top;
        let mut content_width = page_width - margin_left - margin_right;
        let content_height = page_height
            - margin_top
            - margin_bottom
            - self.config.header_height
            - self.config.footer_height;

        // Apply maximum content width if specified
        if self.config.max_content_width > 0.0 && content_width > self.config.max_content_width {
            content_width = self.config.max_content_width;

            // Center content if enabled
            if self.config.center_content {
                content_x = page_x + (page_width - content_width) / 2.0;
            }
        }

        (content_x, content_y, content_width, content_height)
    }

    /// Calculate positions for all components in the dashboard
    pub fn calculate_positions(
        &self,
        components: &[Box<dyn DashboardComponent>],
        content_area: (f64, f64, f64, f64),
    ) -> Result<Vec<ComponentPosition>, PdfError> {
        let (content_x, content_y, content_width, content_height) = content_area;

        // Adjust content area to account for header
        let layout_y = content_y + content_height - self.config.header_height;
        let layout_height = content_height - self.config.header_height;

        // Use grid system to calculate positions
        self.grid.layout_components(
            components,
            content_x,
            layout_y,
            content_width,
            layout_height,
            self.config.default_component_height,
        )
    }

    /// Get layout statistics
    pub fn get_stats(&self, components: &[Box<dyn DashboardComponent>]) -> LayoutStats {
        let total_components = components.len();
        let rows_used = self.estimate_rows_needed(components);
        let column_utilization = self.calculate_column_utilization(components);

        LayoutStats {
            total_components,
            rows_used,
            column_utilization,
            has_overflow: column_utilization > 1.0,
        }
    }

    /// Estimate number of rows needed for components
    fn estimate_rows_needed(&self, components: &[Box<dyn DashboardComponent>]) -> usize {
        let mut current_row_span = 0;
        let mut rows = 0;

        for component in components {
            let span = component.get_span().columns;

            if current_row_span + span > 12 {
                rows += 1;
                current_row_span = span;
            } else {
                current_row_span += span;
                if current_row_span == 12 {
                    rows += 1;
                    current_row_span = 0;
                }
            }
        }

        if current_row_span > 0 {
            rows += 1;
        }

        rows.max(1)
    }

    /// Calculate average column utilization
    fn calculate_column_utilization(&self, components: &[Box<dyn DashboardComponent>]) -> f64 {
        if components.is_empty() {
            return 0.0;
        }

        let total_span: u32 = components.iter().map(|c| c.get_span().columns as u32).sum();

        let estimated_rows = self.estimate_rows_needed(components) as u32;
        let available_columns = estimated_rows * 12;

        if available_columns > 0 {
            total_span as f64 / available_columns as f64
        } else {
            1.0
        }
    }
}

/// Grid system for component layout
#[derive(Debug, Clone)]
pub struct GridSystem {
    /// Number of columns in the grid
    columns: u8,
    /// Gutter between columns
    column_gutter: f64,
    /// Gutter between rows  
    row_gutter: f64,
}

impl GridSystem {
    /// Create a new grid system
    pub fn new(columns: u8, column_gutter: f64, row_gutter: f64) -> Self {
        Self {
            columns,
            column_gutter,
            row_gutter,
        }
    }

    /// Layout components in the grid
    pub fn layout_components(
        &self,
        components: &[Box<dyn DashboardComponent>],
        start_x: f64,
        start_y: f64,
        total_width: f64,
        total_height: f64,
        default_height: f64,
    ) -> Result<Vec<ComponentPosition>, PdfError> {
        let mut positions = Vec::new();

        // Start from the top and work downward (PDF coordinates)
        let mut current_y = start_y;
        let mut row_start = 0;

        // Calculate column width accounting for gutters
        let total_gutter_width = (self.columns as f64 - 1.0) * self.column_gutter;
        let available_width = total_width - total_gutter_width;
        let column_width = available_width / self.columns as f64;

        // Reduce default height to fit more components
        let adjusted_height = (default_height * 0.6).max(120.0); // Minimum 120 points

        while row_start < components.len() {
            // Find components for current row
            let row_end = self.find_row_end(components, row_start);
            let row_components = &components[row_start..row_end];

            // Calculate row height - use consistent height for KPI cards
            let row_height = adjusted_height;

            // Check if we have enough space for this row
            if current_y - row_height < start_y - total_height {
                tracing::warn!(
                    "Dashboard components exceed available height, stopping at row {}",
                    positions.len() / row_components.len()
                );
                break;
            }

            // Position components in this row
            let mut current_x = start_x;

            for component in row_components {
                let span = component.get_span();
                let component_width = column_width * span.columns as f64
                    + self.column_gutter * (span.columns as f64 - 1.0);

                // Position component at current_y - row_height (bottom of component)
                positions.push(ComponentPosition::new(
                    current_x,
                    current_y - row_height,
                    component_width,
                    row_height,
                ));

                current_x += component_width + self.column_gutter;
            }

            // Move to next row with proper spacing
            current_y -= row_height + self.row_gutter;
            row_start = row_end;
        }

        Ok(positions)
    }

    /// Find the end index for the current row
    fn find_row_end(&self, components: &[Box<dyn DashboardComponent>], start: usize) -> usize {
        let mut current_span = 0;
        let mut end = start;

        for (i, component) in components[start..].iter().enumerate() {
            let span = component.get_span().columns;

            if current_span + span > self.columns {
                break;
            }

            current_span += span;
            end = start + i + 1;

            if current_span == self.columns {
                break;
            }
        }

        end.max(start + 1) // Ensure at least one component per row
    }

    /// Calculate the height needed for a row of components
    fn calculate_row_height(
        &self,
        components: &[Box<dyn DashboardComponent>],
        column_width: f64,
        default_height: f64,
    ) -> f64 {
        components
            .iter()
            .map(|component| {
                let span = component.get_span();
                let available_width = column_width * span.columns as f64;
                component.preferred_height(available_width)
            })
            .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .unwrap_or(default_height)
    }
}

/// Layout manager for advanced positioning and responsive behavior
#[derive(Debug, Clone)]
pub struct LayoutManager {
    /// Current layout state
    state: LayoutState,
    /// Responsive breakpoints
    breakpoints: HashMap<String, f64>,
}

impl LayoutManager {
    /// Create a new layout manager
    pub fn new() -> Self {
        Self {
            state: LayoutState::default(),
            breakpoints: HashMap::new(),
        }
    }

    /// Add a responsive breakpoint
    pub fn add_breakpoint<T: Into<String>>(&mut self, name: T, width: f64) {
        self.breakpoints.insert(name.into(), width);
    }

    /// Get current breakpoint based on available width
    pub fn get_current_breakpoint(&self, width: f64) -> String {
        let mut best_match = "default".to_string();
        let mut best_width = 0.0;

        for (name, breakpoint_width) in &self.breakpoints {
            if width >= *breakpoint_width && *breakpoint_width > best_width {
                best_match = name.clone();
                best_width = *breakpoint_width;
            }
        }

        best_match
    }

    /// Optimize layout for the given constraints
    pub fn optimize_layout(
        &self,
        components: &mut [Box<dyn DashboardComponent>],
        available_width: f64,
    ) -> Result<(), PdfError> {
        let breakpoint = self.get_current_breakpoint(available_width);

        // Apply responsive adjustments based on breakpoint
        match breakpoint.as_str() {
            "small" => self.apply_mobile_layout(components)?,
            "medium" => self.apply_tablet_layout(components)?,
            _ => {} // Use default layout
        }

        Ok(())
    }

    /// Apply mobile-friendly layout adjustments
    fn apply_mobile_layout(
        &self,
        components: &mut [Box<dyn DashboardComponent>],
    ) -> Result<(), PdfError> {
        for component in components.iter_mut() {
            // Force components to full width on mobile
            component.set_span(ComponentSpan::new(12));
        }
        Ok(())
    }

    /// Apply tablet-friendly layout adjustments
    fn apply_tablet_layout(
        &self,
        components: &mut [Box<dyn DashboardComponent>],
    ) -> Result<(), PdfError> {
        for component in components.iter_mut() {
            let current_span = component.get_span().columns;

            // Adjust spans for tablet layout
            let new_span = match current_span {
                1..=3 => 6,   // Quarter -> Half width
                4..=6 => 6,   // Keep half width
                7..=12 => 12, // Keep full width
                _ => current_span,
            };

            component.set_span(ComponentSpan::new(new_span));
        }
        Ok(())
    }
}

impl Default for LayoutManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Current state of the layout system
#[derive(Debug, Clone)]
pub struct LayoutState {
    /// Current row being processed
    pub current_row: usize,
    /// Current column position in row
    pub current_column: u8,
    /// Total rows used
    pub total_rows: usize,
}

impl Default for LayoutState {
    fn default() -> Self {
        Self {
            current_row: 0,
            current_column: 0,
            total_rows: 0,
        }
    }
}

/// Grid position for component placement
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GridPosition {
    /// Row number (0-based)
    pub row: usize,
    /// Column start (0-based)
    pub column_start: u8,
    /// Column span (1-12)
    pub column_span: u8,
    /// Row span (default 1)
    pub row_span: u8,
}

impl GridPosition {
    /// Create a new grid position
    pub fn new(row: usize, column_start: u8, column_span: u8) -> Self {
        Self {
            row,
            column_start,
            column_span,
            row_span: 1,
        }
    }

    /// Create a position with row span
    pub fn with_row_span(mut self, row_span: u8) -> Self {
        self.row_span = row_span;
        self
    }

    /// Get the ending column (exclusive)
    pub fn column_end(&self) -> u8 {
        self.column_start + self.column_span
    }

    /// Check if this position overlaps with another
    pub fn overlaps(&self, other: &GridPosition) -> bool {
        self.row < other.row + other.row_span as usize
            && other.row < self.row + self.row_span as usize
            && self.column_start < other.column_end()
            && other.column_start < self.column_end()
    }
}

/// Layout statistics for monitoring and debugging
#[derive(Debug, Clone)]
pub struct LayoutStats {
    /// Total number of components
    pub total_components: usize,
    /// Number of rows used
    pub rows_used: usize,
    /// Column utilization (0.0-1.0, >1.0 indicates overflow)
    pub column_utilization: f64,
    /// Whether there's content overflow
    pub has_overflow: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grid_system() {
        let grid = GridSystem::new(12, 15.0, 20.0);
        assert_eq!(grid.columns, 12);
        assert_eq!(grid.column_gutter, 15.0);
        assert_eq!(grid.row_gutter, 20.0);
    }

    #[test]
    fn test_grid_position() {
        let pos1 = GridPosition::new(0, 0, 6);
        let pos2 = GridPosition::new(0, 6, 6);
        let pos3 = GridPosition::new(0, 3, 6);

        assert!(!pos1.overlaps(&pos2));
        assert!(pos1.overlaps(&pos3));
        assert_eq!(pos1.column_end(), 6);
    }

    #[test]
    fn test_layout_manager_breakpoints() {
        let mut manager = LayoutManager::new();
        manager.add_breakpoint("small", 400.0);
        manager.add_breakpoint("medium", 768.0);
        manager.add_breakpoint("large", 1024.0);

        assert_eq!(manager.get_current_breakpoint(300.0), "default");
        assert_eq!(manager.get_current_breakpoint(500.0), "small");
        assert_eq!(manager.get_current_breakpoint(800.0), "medium");
        assert_eq!(manager.get_current_breakpoint(1200.0), "large");
    }

    #[test]
    fn test_dashboard_layout_content_area() {
        let config = DashboardConfig::default();
        let layout = DashboardLayout::new(config);

        let page_bounds = (0.0, 0.0, 800.0, 600.0);
        let content_area = layout.calculate_content_area(page_bounds);

        // Should account for margins, header, and footer
        assert_eq!(content_area.0, 30.0); // Left margin
        assert!(content_area.2 < 800.0); // Width reduced by margins
        assert!(content_area.3 < 600.0); // Height reduced by margins + header + footer
    }

    #[test]
    fn test_grid_position_with_row_span() {
        let pos = GridPosition::new(0, 0, 6).with_row_span(2);
        assert_eq!(pos.row, 0);
        assert_eq!(pos.column_start, 0);
        assert_eq!(pos.column_span, 6);
        assert_eq!(pos.row_span, 2);
    }

    #[test]
    fn test_grid_position_column_end() {
        let pos = GridPosition::new(0, 3, 5);
        assert_eq!(pos.column_end(), 8);
    }

    #[test]
    fn test_grid_position_overlaps_same_row() {
        let pos1 = GridPosition::new(0, 0, 4);
        let pos2 = GridPosition::new(0, 2, 4);
        assert!(pos1.overlaps(&pos2));
        assert!(pos2.overlaps(&pos1));
    }

    #[test]
    fn test_grid_position_overlaps_with_row_span() {
        let pos1 = GridPosition::new(0, 0, 6).with_row_span(2);
        let pos2 = GridPosition::new(1, 0, 6);
        assert!(pos1.overlaps(&pos2));
    }

    #[test]
    fn test_grid_position_no_overlap_different_rows() {
        let pos1 = GridPosition::new(0, 0, 6);
        let pos2 = GridPosition::new(2, 0, 6);
        assert!(!pos1.overlaps(&pos2));
    }

    #[test]
    fn test_grid_position_equality() {
        let pos1 = GridPosition::new(1, 2, 3);
        let pos2 = GridPosition::new(1, 2, 3);
        let pos3 = GridPosition::new(1, 2, 4);
        assert_eq!(pos1, pos2);
        assert_ne!(pos1, pos3);
    }

    #[test]
    fn test_layout_state_default() {
        let state = LayoutState::default();
        assert_eq!(state.current_row, 0);
        assert_eq!(state.current_column, 0);
        assert_eq!(state.total_rows, 0);
    }

    #[test]
    fn test_layout_state_clone() {
        let state = LayoutState {
            current_row: 2,
            current_column: 5,
            total_rows: 3,
        };
        let cloned = state.clone();
        assert_eq!(state.current_row, cloned.current_row);
        assert_eq!(state.current_column, cloned.current_column);
        assert_eq!(state.total_rows, cloned.total_rows);
    }

    #[test]
    fn test_layout_manager_default() {
        let manager = LayoutManager::default();
        assert_eq!(manager.get_current_breakpoint(500.0), "default");
    }

    #[test]
    fn test_layout_manager_no_breakpoints() {
        let manager = LayoutManager::new();
        assert_eq!(manager.get_current_breakpoint(0.0), "default");
        assert_eq!(manager.get_current_breakpoint(10000.0), "default");
    }

    #[test]
    fn test_layout_stats_debug() {
        let stats = LayoutStats {
            total_components: 5,
            rows_used: 2,
            column_utilization: 0.8,
            has_overflow: false,
        };
        let debug_str = format!("{:?}", stats);
        assert!(debug_str.contains("LayoutStats"));
        assert!(debug_str.contains("5"));
    }

    #[test]
    fn test_layout_stats_clone() {
        let stats = LayoutStats {
            total_components: 3,
            rows_used: 1,
            column_utilization: 0.5,
            has_overflow: true,
        };
        let cloned = stats.clone();
        assert_eq!(stats.total_components, cloned.total_components);
        assert_eq!(stats.rows_used, cloned.rows_used);
        assert_eq!(stats.column_utilization, cloned.column_utilization);
        assert_eq!(stats.has_overflow, cloned.has_overflow);
    }

    #[test]
    fn test_grid_system_clone() {
        let grid = GridSystem::new(12, 10.0, 15.0);
        let cloned = grid.clone();
        assert_eq!(grid.columns, cloned.columns);
        assert_eq!(grid.column_gutter, cloned.column_gutter);
        assert_eq!(grid.row_gutter, cloned.row_gutter);
    }

    #[test]
    fn test_grid_system_debug() {
        let grid = GridSystem::new(8, 5.0, 10.0);
        let debug_str = format!("{:?}", grid);
        assert!(debug_str.contains("GridSystem"));
        assert!(debug_str.contains("8"));
    }

    #[test]
    fn test_layout_manager_clone() {
        let mut manager = LayoutManager::new();
        manager.add_breakpoint("test", 500.0);
        let cloned = manager.clone();
        assert_eq!(
            manager.get_current_breakpoint(600.0),
            cloned.get_current_breakpoint(600.0)
        );
    }

    #[test]
    fn test_dashboard_layout_clone() {
        let config = DashboardConfig::default();
        let layout = DashboardLayout::new(config);
        let cloned = layout.clone();
        let page_bounds = (0.0, 0.0, 800.0, 600.0);
        assert_eq!(
            layout.calculate_content_area(page_bounds),
            cloned.calculate_content_area(page_bounds)
        );
    }
}
