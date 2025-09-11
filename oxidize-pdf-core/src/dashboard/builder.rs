//! Dashboard Builder - Fluent API for Dashboard Construction
//!
//! This module provides a builder pattern for creating dashboards with a fluent API.
//! The builder handles component arrangement, layout configuration, theming, and
//! validation to ensure professional dashboard creation.

use super::{
    ComponentSpan, Dashboard, DashboardComponent, DashboardLayout, DashboardMetadata,
    DashboardTheme, KpiCard, Typography,
};
use crate::error::PdfError;
use crate::graphics::Color;
use std::collections::HashMap;

/// Builder for creating dashboards with a fluent API
#[derive(Debug)]
pub struct DashboardBuilder {
    /// Dashboard title
    title: Option<String>,
    /// Dashboard subtitle
    subtitle: Option<String>,
    /// Dashboard theme
    theme: DashboardTheme,
    /// Dashboard layout configuration
    layout_config: DashboardConfig,
    /// Components to add to the dashboard
    components: Vec<Box<dyn DashboardComponent>>,
    /// Metadata
    metadata: DashboardMetadata,
    /// Current row being built
    current_row: Vec<Box<dyn DashboardComponent>>,
}

impl DashboardBuilder {
    /// Create a new dashboard builder
    pub fn new() -> Self {
        Self {
            title: None,
            subtitle: None,
            theme: DashboardTheme::default(),
            layout_config: DashboardConfig::default(),
            components: Vec::new(),
            metadata: DashboardMetadata::default(),
            current_row: Vec::new(),
        }
    }

    /// Set the dashboard title
    pub fn title<T: Into<String>>(mut self, title: T) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set the dashboard subtitle
    pub fn subtitle<T: Into<String>>(mut self, subtitle: T) -> Self {
        self.subtitle = Some(subtitle.into());
        self
    }

    /// Set the dashboard theme
    pub fn theme(mut self, theme: DashboardTheme) -> Self {
        self.theme = theme;
        self
    }

    /// Use a pre-defined theme by name
    pub fn theme_by_name(mut self, theme_name: &str) -> Self {
        self.theme = match theme_name.to_lowercase().as_str() {
            "corporate" => DashboardTheme::corporate(),
            "minimal" => DashboardTheme::minimal(),
            "dark" => DashboardTheme::dark(),
            "colorful" => DashboardTheme::colorful(),
            _ => DashboardTheme::default(),
        };
        self
    }

    /// Set custom color palette
    pub fn color_palette(mut self, colors: Vec<Color>) -> Self {
        self.theme.set_color_palette(colors);
        self
    }

    /// Set typography configuration
    pub fn typography(mut self, typography: Typography) -> Self {
        self.theme.set_typography(typography);
        self
    }

    /// Set layout configuration
    pub fn layout_config(mut self, config: DashboardConfig) -> Self {
        self.layout_config = config;
        self
    }

    /// Add a single component to the dashboard
    pub fn add_component(mut self, component: Box<dyn DashboardComponent>) -> Self {
        self.finish_current_row();
        self.components.push(component);
        self
    }

    /// Add multiple components as a row
    pub fn add_row(mut self, components: Vec<Box<dyn DashboardComponent>>) -> Self {
        self.finish_current_row();

        // Validate row span doesn't exceed 12 columns
        let total_span: u8 = components.iter().map(|c| c.get_span().columns).sum();

        if total_span > 12 {
            tracing::warn!(
                "Row components span {} columns, exceeding maximum of 12",
                total_span
            );
        }

        self.components.extend(components);
        self
    }

    /// Start building a row of components
    pub fn start_row(mut self) -> Self {
        self.finish_current_row();
        self
    }

    /// Add a component to the current row
    pub fn add_to_row(mut self, component: Box<dyn DashboardComponent>) -> Self {
        self.current_row.push(component);
        self
    }

    /// Finish the current row and add it to the dashboard
    pub fn finish_row(mut self) -> Self {
        self.finish_current_row();
        self
    }

    /// Add a row of KPI cards (convenience method)
    /// Automatically splits KPIs into multiple rows to ensure adequate width for text rendering
    pub fn add_kpi_row(mut self, kpi_cards: Vec<KpiCard>) -> Self {
        let total_cards = kpi_cards.len();

        if total_cards <= 2 {
            // 1-2 KPIs: Use full width available
            let span_per_card = (12 / total_cards.max(1)) as u8;
            let components: Vec<Box<dyn DashboardComponent>> = kpi_cards
                .into_iter()
                .map(|mut card| {
                    card.set_span(ComponentSpan::new(span_per_card));
                    Box::new(card) as Box<dyn DashboardComponent>
                })
                .collect();
            self.add_row(components)
        } else {
            // 3+ KPIs: Split into multiple rows with max 2 KPIs per row (span=6 each)
            self.finish_current_row();

            for chunk in kpi_cards.chunks(2) {
                let span_per_card = (12 / chunk.len().max(1)) as u8;
                let row_components: Vec<Box<dyn DashboardComponent>> = chunk
                    .iter()
                    .cloned()
                    .map(|mut card| {
                        card.set_span(ComponentSpan::new(span_per_card));
                        Box::new(card) as Box<dyn DashboardComponent>
                    })
                    .collect();
                self.components.extend(row_components);
            }

            self
        }
    }

    /// Set dashboard author
    pub fn author<T: Into<String>>(mut self, author: T) -> Self {
        self.metadata.author = Some(author.into());
        self
    }

    /// Add data source
    pub fn data_source<T: Into<String>>(mut self, source: T) -> Self {
        self.metadata.data_sources.push(source.into());
        self
    }

    /// Add multiple data sources
    pub fn data_sources<T: Into<String>>(mut self, sources: Vec<T>) -> Self {
        let sources: Vec<String> = sources.into_iter().map(|s| s.into()).collect();
        self.metadata.data_sources.extend(sources);
        self
    }

    /// Add a tag
    pub fn tag<T: Into<String>>(mut self, tag: T) -> Self {
        self.metadata.tags.push(tag.into());
        self
    }

    /// Add multiple tags
    pub fn tags<T: Into<String>>(mut self, tags: Vec<T>) -> Self {
        let tags: Vec<String> = tags.into_iter().map(|t| t.into()).collect();
        self.metadata.tags.extend(tags);
        self
    }

    /// Set dashboard version
    pub fn version<T: Into<String>>(mut self, version: T) -> Self {
        self.metadata.version = version.into();
        self
    }

    /// Build the dashboard
    pub fn build(mut self) -> Result<Dashboard, PdfError> {
        self.finish_current_row();

        // Validate required fields
        if self.title.is_none() {
            return Err(PdfError::InvalidOperation(
                "Dashboard title is required".to_string(),
            ));
        }

        // Validate components
        for component in &self.components {
            component.validate()?;
        }

        // Create layout
        let layout = DashboardLayout::new(self.layout_config);

        Ok(Dashboard {
            title: self.title.unwrap(),
            subtitle: self.subtitle,
            layout,
            theme: self.theme,
            components: self.components,
            metadata: self.metadata,
        })
    }

    /// Finish the current row if it has components
    fn finish_current_row(&mut self) {
        if !self.current_row.is_empty() {
            let row_components = std::mem::take(&mut self.current_row);
            self.components.extend(row_components);
        }
    }
}

impl Default for DashboardBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Configuration for dashboard layout and spacing
#[derive(Debug, Clone)]
pub struct DashboardConfig {
    /// Page margins in points (top, right, bottom, left)
    pub margins: (f64, f64, f64, f64),
    /// Gutter between columns in points
    pub column_gutter: f64,
    /// Gutter between rows in points
    pub row_gutter: f64,
    /// Header height in points
    pub header_height: f64,
    /// Footer height in points
    pub footer_height: f64,
    /// Maximum content width in points (0 = use full page)
    pub max_content_width: f64,
    /// Whether to center content horizontally
    pub center_content: bool,
    /// Default component height in points
    pub default_component_height: f64,
    /// Responsive breakpoints for different page sizes
    pub breakpoints: HashMap<String, f64>,
}

impl DashboardConfig {
    /// Create a new dashboard configuration
    pub fn new() -> Self {
        Self::default()
    }

    /// Set page margins
    pub fn with_margins(mut self, top: f64, right: f64, bottom: f64, left: f64) -> Self {
        self.margins = (top, right, bottom, left);
        self
    }

    /// Set uniform margins
    pub fn with_uniform_margins(mut self, margin: f64) -> Self {
        self.margins = (margin, margin, margin, margin);
        self
    }

    /// Set column gutter
    pub fn with_column_gutter(mut self, gutter: f64) -> Self {
        self.column_gutter = gutter;
        self
    }

    /// Set row gutter
    pub fn with_row_gutter(mut self, gutter: f64) -> Self {
        self.row_gutter = gutter;
        self
    }

    /// Set maximum content width
    pub fn with_max_content_width(mut self, width: f64) -> Self {
        self.max_content_width = width;
        self
    }

    /// Enable content centering
    pub fn with_centered_content(mut self, center: bool) -> Self {
        self.center_content = center;
        self
    }

    /// Set default component height
    pub fn with_default_component_height(mut self, height: f64) -> Self {
        self.default_component_height = height;
        self
    }

    /// Add a responsive breakpoint
    pub fn with_breakpoint<T: Into<String>>(mut self, name: T, width: f64) -> Self {
        self.breakpoints.insert(name.into(), width);
        self
    }
}

impl Default for DashboardConfig {
    fn default() -> Self {
        let mut breakpoints = HashMap::new();
        breakpoints.insert("small".to_string(), 400.0);
        breakpoints.insert("medium".to_string(), 600.0);
        breakpoints.insert("large".to_string(), 800.0);
        breakpoints.insert("xlarge".to_string(), 1000.0);

        Self {
            margins: (30.0, 30.0, 30.0, 30.0), // Reduced margins
            column_gutter: 12.0,               // Reduced column spacing
            row_gutter: 30.0,                  // Increased row spacing
            header_height: 60.0,               // Reduced header height
            footer_height: 25.0,               // Reduced footer height
            max_content_width: 0.0,            // Use full page width
            center_content: false,
            default_component_height: 120.0, // Reduced default height
            breakpoints,
        }
    }
}

impl Default for DashboardMetadata {
    fn default() -> Self {
        Self {
            created_at: chrono::Utc::now(),
            version: "1.0.0".to_string(),
            data_sources: Vec::new(),
            author: None,
            tags: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graphics::Color;

    #[test]
    fn test_dashboard_builder_basic() {
        let dashboard = DashboardBuilder::new()
            .title("Test Dashboard")
            .subtitle("Unit Test")
            .author("Test Author")
            .build()
            .unwrap();

        assert_eq!(dashboard.title, "Test Dashboard");
        assert_eq!(dashboard.subtitle, Some("Unit Test".to_string()));
        assert_eq!(dashboard.metadata.author, Some("Test Author".to_string()));
    }

    #[test]
    fn test_dashboard_builder_validation() {
        let result = DashboardBuilder::new().subtitle("Missing title").build();

        assert!(result.is_err());

        if let Err(PdfError::InvalidOperation(msg)) = result {
            assert!(msg.contains("title is required"));
        }
    }

    #[test]
    fn test_dashboard_config() {
        let config = DashboardConfig::new()
            .with_uniform_margins(40.0)
            .with_column_gutter(12.0)
            .with_max_content_width(800.0)
            .with_breakpoint("custom", 500.0);

        assert_eq!(config.margins, (40.0, 40.0, 40.0, 40.0));
        assert_eq!(config.column_gutter, 12.0);
        assert_eq!(config.max_content_width, 800.0);
        assert_eq!(config.breakpoints.get("custom"), Some(&500.0));
    }

    #[test]
    fn test_dashboard_builder_theming() {
        let dashboard = DashboardBuilder::new()
            .title("Themed Dashboard")
            .theme_by_name("corporate")
            .color_palette(vec![Color::blue(), Color::green()])
            .build()
            .unwrap();

        assert_eq!(dashboard.title, "Themed Dashboard");
    }

    #[test]
    fn test_dashboard_builder_metadata() {
        let dashboard = DashboardBuilder::new()
            .title("Data Dashboard")
            .data_sources(vec!["Sales DB", "Analytics API"])
            .tags(vec!["sales", "q4", "executive"])
            .version("2.1.0")
            .build()
            .unwrap();

        assert_eq!(dashboard.metadata.data_sources.len(), 2);
        assert_eq!(dashboard.metadata.tags.len(), 3);
        assert_eq!(dashboard.metadata.version, "2.1.0");
    }
}
