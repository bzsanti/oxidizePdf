//! KPI Card Component
//!
//! This module implements KPI (Key Performance Indicator) cards for dashboards.
//! KPI cards display important metrics with values, trends, and optional sparklines
//! in a visually appealing card format.

use super::{
    component::ComponentConfig, ComponentPosition, ComponentSpan, DashboardComponent,
    DashboardTheme,
};
use crate::error::PdfError;
use crate::graphics::Color;
use crate::page::Page;
use crate::Font;

/// KPI Card component for displaying key metrics
#[derive(Debug, Clone)]
pub struct KpiCard {
    /// Component configuration
    config: ComponentConfig,
    /// KPI title/label
    title: String,
    /// Main value to display
    value: String,
    /// Value formatting options
    value_format: ValueFormat,
    /// Trend information (optional)
    trend: Option<TrendInfo>,
    /// Subtitle or additional context
    subtitle: Option<String>,
    /// Custom color theme for this card
    color_theme: Option<KpiColorTheme>,
    /// Sparkline data (optional)
    sparkline: Option<SparklineData>,
    /// Icon (optional)
    icon: Option<String>,
    /// Custom styling
    style: KpiCardStyle,
}

impl KpiCard {
    /// Create a new KPI card
    pub fn new<T: Into<String>, V: Into<String>>(title: T, value: V) -> Self {
        Self {
            config: ComponentConfig::new(ComponentSpan::new(12)), // Full width by default
            title: title.into(),
            value: value.into(),
            value_format: ValueFormat::default(),
            trend: None,
            subtitle: None,
            color_theme: None,
            sparkline: None,
            icon: None,
            style: KpiCardStyle::default(),
        }
    }

    /// Set the trend information
    pub fn with_trend(mut self, change: f64, direction: TrendDirection) -> Self {
        self.trend = Some(TrendInfo {
            change,
            direction,
            period: "vs previous".to_string(),
            is_good: matches!(direction, TrendDirection::Up),
        });
        self
    }

    /// Set trend with custom period
    pub fn with_trend_period<T: Into<String>>(
        mut self,
        change: f64,
        direction: TrendDirection,
        period: T,
    ) -> Self {
        self.trend = Some(TrendInfo {
            change,
            direction,
            period: period.into(),
            is_good: matches!(direction, TrendDirection::Up),
        });
        self
    }

    /// Set whether the trend is good or bad (for coloring)
    pub fn trend_is_good(mut self, is_good: bool) -> Self {
        if let Some(ref mut trend) = self.trend {
            trend.is_good = is_good;
        }
        self
    }

    /// Set a subtitle
    pub fn with_subtitle<T: Into<String>>(mut self, subtitle: T) -> Self {
        self.subtitle = Some(subtitle.into());
        self
    }

    /// Set value formatting
    pub fn with_format(mut self, format: ValueFormat) -> Self {
        self.value_format = format;
        self
    }

    /// Set currency formatting
    pub fn as_currency<T: Into<String>>(mut self, symbol: T) -> Self {
        self.value_format = ValueFormat::Currency {
            symbol: symbol.into(),
            decimal_places: 2,
        };
        self
    }

    /// Set percentage formatting
    pub fn as_percentage(mut self, decimal_places: u8) -> Self {
        self.value_format = ValueFormat::Percentage { decimal_places };
        self
    }

    /// Set number formatting
    pub fn as_number(mut self, decimal_places: u8, thousands_separator: bool) -> Self {
        self.value_format = ValueFormat::Number {
            decimal_places,
            thousands_separator,
        };
        self
    }

    /// Set custom color theme
    pub fn color(mut self, color: Color) -> Self {
        self.color_theme = Some(KpiColorTheme::from_primary(color));
        self
    }

    /// Set custom color theme
    pub fn with_colors(mut self, theme: KpiColorTheme) -> Self {
        self.color_theme = Some(theme);
        self
    }

    /// Add sparkline data
    pub fn with_sparkline(mut self, data: Vec<f64>) -> Self {
        self.sparkline = Some(SparklineData::new(data));
        self
    }

    /// Add sparkline with labels
    pub fn with_sparkline_labeled(mut self, data: Vec<f64>, labels: Vec<String>) -> Self {
        self.sparkline = Some(SparklineData::with_labels(data, labels));
        self
    }

    /// Set an icon
    pub fn with_icon<T: Into<String>>(mut self, icon: T) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Set card style
    pub fn with_style(mut self, style: KpiCardStyle) -> Self {
        self.style = style;
        self
    }
}

impl DashboardComponent for KpiCard {
    fn render(
        &self,
        page: &mut Page,
        position: ComponentPosition,
        theme: &DashboardTheme,
    ) -> Result<(), PdfError> {
        let card_area = position.with_padding(self.style.padding);

        // Determine colors to use
        let default_colors = KpiColorTheme::from_theme(theme);
        let colors = self.color_theme.as_ref().unwrap_or(&default_colors);

        // Draw background and border using single graphics context for proper z-order
        let graphics = page.graphics();

        // Background first (behind everything)
        graphics
            .set_fill_color(colors.background)
            .rect(position.x, position.y, position.width, position.height)
            .fill();

        // Border second (if enabled)
        if self.style.show_border {
            graphics
                .set_stroke_color(colors.border)
                .set_line_width(1.0)
                .rect(position.x, position.y, position.width, position.height)
                .stroke();
        }

        // Layout content areas
        let layout = self.calculate_layout(card_area);

        // Render icon if present
        if let Some(ref icon) = self.icon {
            self.render_icon(page, layout.icon_area, icon, colors)?;
        }

        // Render title
        self.render_title(page, layout.title_area, colors, theme)?;

        // Render value
        self.render_value(page, layout.value_area, colors, theme)?;

        // Render trend if present
        if let Some(ref trend) = self.trend {
            self.render_trend(page, layout.trend_area, trend, colors, theme)?;
        }

        // Render subtitle if present
        if let Some(ref subtitle) = self.subtitle {
            self.render_subtitle(page, layout.subtitle_area, subtitle, colors, theme)?;
        }

        // Render sparkline if present
        if let Some(ref sparkline) = self.sparkline {
            self.render_sparkline(page, layout.sparkline_area, sparkline, colors)?;
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
        // Base height for KPI cards
        let mut height = 120.0;

        // Add height for subtitle
        if self.subtitle.is_some() {
            height += 20.0;
        }

        // Add height for sparkline
        if self.sparkline.is_some() {
            height += 40.0;
        }

        // Add padding
        height += 2.0 * self.style.padding;

        height
    }

    fn minimum_width(&self) -> f64 {
        120.0 // Minimum width for readable KPI cards
    }

    fn estimated_render_time_ms(&self) -> u32 {
        let mut time = 15; // Base render time

        if self.sparkline.is_some() {
            time += 10; // Sparklines take extra time
        }

        if self.trend.is_some() {
            time += 3; // Trend indicators
        }

        time
    }

    fn estimated_memory_mb(&self) -> f64 {
        0.05 // Small memory footprint
    }

    fn complexity_score(&self) -> u8 {
        let mut score = 20; // Base complexity

        if self.sparkline.is_some() {
            score += 30; // Sparklines add complexity
        }

        if self.trend.is_some() {
            score += 10;
        }

        if self.icon.is_some() {
            score += 5;
        }

        score.min(100)
    }

    fn component_type(&self) -> &'static str {
        "KpiCard"
    }
}

/// Layout areas for KPI card components
#[derive(Debug, Clone)]
struct KpiCardLayout {
    icon_area: ComponentPosition,
    title_area: ComponentPosition,
    value_area: ComponentPosition,
    trend_area: ComponentPosition,
    subtitle_area: ComponentPosition,
    sparkline_area: ComponentPosition,
}

impl KpiCard {
    /// Calculate layout areas within the card
    fn calculate_layout(&self, card_area: ComponentPosition) -> KpiCardLayout {
        // Start from BOTTOM of card and work UPWARD (PDF coordinates)
        let bottom_y = card_area.y;
        let mut current_y = bottom_y;
        let line_height = 16.0;
        let padding = 8.0;

        // Icon area (top-right corner)
        let icon_size = 20.0;
        let icon_area = ComponentPosition::new(
            card_area.x + card_area.width - icon_size - padding,
            card_area.y + card_area.height - icon_size - padding, // Fixed to card top
            icon_size,
            icon_size,
        );

        // Sparkline area (bottom of card)
        current_y += padding;
        let sparkline_height = 20.0;
        let sparkline_area = ComponentPosition::new(
            card_area.x + padding,
            current_y,
            card_area.width - padding * 2.0,
            sparkline_height,
        );
        current_y += sparkline_height;

        // Subtitle area (above sparkline)
        current_y += padding / 2.0;
        let subtitle_area = ComponentPosition::new(
            card_area.x + padding,
            current_y,
            card_area.width - padding * 2.0,
            line_height,
        );
        current_y += line_height;

        // Value area (main content)
        current_y += padding / 2.0;
        let value_height = 24.0;
        let value_area = ComponentPosition::new(
            card_area.x + padding,
            current_y,
            card_area.width * 0.65,
            value_height,
        );

        // Trend area (to the right of value)
        let trend_area = ComponentPosition::new(
            card_area.x + card_area.width * 0.65,
            current_y,
            card_area.width * 0.35 - padding,
            value_height,
        );
        current_y += value_height;

        // Title area (top of content)
        current_y += padding / 2.0;
        let title_area = ComponentPosition::new(
            card_area.x + padding,
            current_y,
            card_area.width
                - padding * 2.0
                - (if self.icon.is_some() {
                    icon_size + padding
                } else {
                    0.0
                }),
            line_height,
        );

        KpiCardLayout {
            icon_area,
            title_area,
            value_area,
            trend_area,
            subtitle_area,
            sparkline_area,
        }
    }

    /// Draw card background
    fn draw_background(
        &self,
        page: &mut Page,
        position: ComponentPosition,
        colors: &KpiColorTheme,
        _theme: &DashboardTheme,
    ) -> Result<(), PdfError> {
        // Draw simple rectangle background
        page.graphics()
            .set_fill_color(colors.background)
            .rect(position.x, position.y, position.width, position.height)
            .fill();

        Ok(())
    }

    /// Draw card border
    fn draw_border(
        &self,
        page: &mut Page,
        position: ComponentPosition,
        colors: &KpiColorTheme,
        _theme: &DashboardTheme,
    ) -> Result<(), PdfError> {
        // Draw rectangle border
        page.graphics()
            .set_stroke_color(colors.border)
            .set_line_width(1.0)
            .rect(position.x, position.y, position.width, position.height)
            .stroke();

        Ok(())
    }

    /// Render the icon
    fn render_icon(
        &self,
        _page: &mut Page,
        _area: ComponentPosition,
        _icon: &str,
        _colors: &KpiColorTheme,
    ) -> Result<(), PdfError> {
        // For now, render icon as text (could be enhanced to support actual icons)
        // Placeholder: page.add_text replaced

        Ok(())
    }

    /// Render the title
    fn render_title(
        &self,
        page: &mut Page,
        area: ComponentPosition,
        colors: &KpiColorTheme,
        theme: &DashboardTheme,
    ) -> Result<(), PdfError> {
        page.text()
            .set_font(Font::Helvetica, theme.typography.body_size)
            .set_fill_color(colors.text_secondary)
            .at(area.x, area.y)
            .write(&self.title)?;

        Ok(())
    }

    /// Render the main value
    fn render_value(
        &self,
        page: &mut Page,
        area: ComponentPosition,
        colors: &KpiColorTheme,
        _theme: &DashboardTheme,
    ) -> Result<(), PdfError> {
        let formatted_value = self.format_value();
        page.text()
            .set_font(Font::HelveticaBold, 18.0)
            .set_fill_color(colors.text_primary)
            .at(area.x, area.y)
            .write(&formatted_value)?;

        Ok(())
    }

    /// Render trend information
    fn render_trend(
        &self,
        page: &mut Page,
        area: ComponentPosition,
        trend: &TrendInfo,
        colors: &KpiColorTheme,
        theme: &DashboardTheme,
    ) -> Result<(), PdfError> {
        let trend_text = format!(
            "{}{:.1}%",
            match trend.direction {
                TrendDirection::Up => "↑",
                TrendDirection::Down => "↓",
                TrendDirection::Flat => "→",
            },
            trend.change.abs()
        );

        let trend_color = if trend.is_good {
            colors.success
        } else {
            colors.danger
        };

        page.text()
            .set_font(Font::Helvetica, theme.typography.body_size)
            .set_fill_color(trend_color)
            .at(area.x, area.y)
            .write(&trend_text)?;

        Ok(())
    }

    /// Render subtitle
    fn render_subtitle(
        &self,
        page: &mut Page,
        area: ComponentPosition,
        subtitle: &str,
        colors: &KpiColorTheme,
        theme: &DashboardTheme,
    ) -> Result<(), PdfError> {
        page.text()
            .set_font(Font::Helvetica, theme.typography.caption_size)
            .set_fill_color(colors.text_muted)
            .at(area.x, area.y)
            .write(subtitle)?;

        Ok(())
    }

    /// Render sparkline
    fn render_sparkline(
        &self,
        page: &mut Page,
        area: ComponentPosition,
        sparkline: &SparklineData,
        colors: &KpiColorTheme,
    ) -> Result<(), PdfError> {
        if sparkline.data.is_empty() {
            return Ok(());
        }

        // Find min/max for scaling
        let min_val = sparkline.data.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        let max_val = sparkline
            .data
            .iter()
            .fold(f64::NEG_INFINITY, |a, &b| a.max(b));

        if (max_val - min_val).abs() < f64::EPSILON {
            return Ok(()); // No variation to show
        }

        // Setup graphics for drawing lines
        let graphics = page.graphics();
        graphics
            .set_stroke_color(colors.primary)
            .set_line_width(1.5);

        let step_x = area.width / (sparkline.data.len() - 1) as f64;

        // Draw sparkline as connected line segments
        let mut first_point = true;
        for (i, &value) in sparkline.data.iter().enumerate() {
            let x = area.x + i as f64 * step_x;
            let normalized = (value - min_val) / (max_val - min_val);
            let y = area.y + (1.0 - normalized) * area.height; // Invert y for proper display

            if first_point {
                graphics.move_to(x, y);
                first_point = false;
            } else {
                graphics.line_to(x, y);
            }
        }

        graphics.stroke();

        Ok(())
    }

    /// Format the value according to the specified format
    fn format_value(&self) -> String {
        match &self.value_format {
            ValueFormat::Raw => self.value.clone(),
            ValueFormat::Currency {
                symbol,
                decimal_places,
            } => {
                if let Ok(num) = self.value.parse::<f64>() {
                    format!(
                        "{symbol}{num:.prec$}",
                        symbol = symbol,
                        num = num,
                        prec = *decimal_places as usize
                    )
                } else {
                    self.value.clone()
                }
            }
            ValueFormat::Percentage { decimal_places } => {
                if let Ok(num) = self.value.parse::<f64>() {
                    format!("{:.1$}%", num, *decimal_places as usize)
                } else {
                    self.value.clone()
                }
            }
            ValueFormat::Number {
                decimal_places,
                thousands_separator,
            } => {
                if let Ok(num) = self.value.parse::<f64>() {
                    let formatted = format!("{:.1$}", num, *decimal_places as usize);
                    if *thousands_separator {
                        // Simple thousands separator implementation
                        // In a real implementation, you'd use proper number formatting
                        formatted
                    } else {
                        formatted
                    }
                } else {
                    self.value.clone()
                }
            }
        }
    }
}

/// Builder pattern for KPI cards
#[derive(Debug)]
pub struct KpiCardBuilder {
    card: KpiCard,
}

impl KpiCardBuilder {
    /// Create a new KPI card builder
    pub fn new<T: Into<String>, V: Into<String>>(title: T, value: V) -> Self {
        Self {
            card: KpiCard::new(title, value),
        }
    }

    /// Set trend
    pub fn trend(mut self, change: f64, direction: TrendDirection) -> Self {
        self.card = self.card.with_trend(change, direction);
        self
    }

    /// Set subtitle
    pub fn subtitle<T: Into<String>>(mut self, subtitle: T) -> Self {
        self.card = self.card.with_subtitle(subtitle);
        self
    }

    /// Set color
    pub fn color(mut self, color: Color) -> Self {
        self.card = self.card.color(color);
        self
    }

    /// Add sparkline
    pub fn sparkline(mut self, data: Vec<f64>) -> Self {
        self.card = self.card.with_sparkline(data);
        self
    }

    /// Build the KPI card
    pub fn build(self) -> KpiCard {
        self.card
    }
}

/// Trend direction for KPI cards
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrendDirection {
    Up,
    Down,
    Flat,
}

/// Trend information for KPI cards
#[derive(Debug, Clone)]
pub struct TrendInfo {
    /// Percentage change
    pub change: f64,
    /// Direction of change
    pub direction: TrendDirection,
    /// Time period for the change
    pub period: String,
    /// Whether this trend is considered positive
    pub is_good: bool,
}

/// Value formatting options
#[derive(Debug, Clone)]
pub enum ValueFormat {
    /// Display value as-is
    Raw,
    /// Format as currency
    Currency { symbol: String, decimal_places: u8 },
    /// Format as percentage
    Percentage { decimal_places: u8 },
    /// Format as number
    Number {
        decimal_places: u8,
        thousands_separator: bool,
    },
}

impl Default for ValueFormat {
    fn default() -> Self {
        Self::Raw
    }
}

/// Color theme for KPI cards
#[derive(Debug, Clone)]
pub struct KpiColorTheme {
    /// Primary color (for accents)
    pub primary: Color,
    /// Background color
    pub background: Color,
    /// Border color
    pub border: Color,
    /// Primary text color
    pub text_primary: Color,
    /// Secondary text color
    pub text_secondary: Color,
    /// Muted text color
    pub text_muted: Color,
    /// Accent color (for sparklines, etc.)
    pub accent: Color,
    /// Success color (for positive trends)
    pub success: Color,
    /// Danger color (for negative trends)
    pub danger: Color,
}

impl KpiColorTheme {
    /// Create theme from a primary color
    pub fn from_primary(primary: Color) -> Self {
        Self {
            primary,
            background: Color::white(),
            border: Color::hex("#e0e0e0"),
            text_primary: Color::hex("#212529"),
            text_secondary: Color::hex("#6c757d"),
            text_muted: Color::hex("#adb5bd"),
            accent: primary,
            success: Color::hex("#28a745"),
            danger: Color::hex("#dc3545"),
        }
    }

    /// Create theme from dashboard theme
    pub fn from_theme(theme: &DashboardTheme) -> Self {
        Self {
            primary: theme.colors.primary,
            background: theme.colors.surface,
            border: theme.colors.border,
            text_primary: theme.colors.text_primary,
            text_secondary: theme.colors.text_secondary,
            text_muted: theme.colors.text_muted,
            accent: theme.colors.accent,
            success: theme.colors.success,
            danger: theme.colors.danger,
        }
    }
}

/// Sparkline data for mini charts in KPI cards
#[derive(Debug, Clone)]
pub struct SparklineData {
    /// Data points
    pub data: Vec<f64>,
    /// Optional labels for data points
    pub labels: Option<Vec<String>>,
}

impl SparklineData {
    /// Create sparkline from data
    pub fn new(data: Vec<f64>) -> Self {
        Self { data, labels: None }
    }

    /// Create sparkline with labels
    pub fn with_labels(data: Vec<f64>, labels: Vec<String>) -> Self {
        Self {
            data,
            labels: Some(labels),
        }
    }
}

/// Styling options for KPI cards
#[derive(Debug, Clone)]
pub struct KpiCardStyle {
    /// Padding around content
    pub padding: f64,
    /// Whether to show border
    pub show_border: bool,
    /// Whether to show shadow
    pub show_shadow: bool,
    /// Corner radius for rounded corners
    pub corner_radius: f64,
}

impl Default for KpiCardStyle {
    fn default() -> Self {
        Self {
            padding: 12.0,
            show_border: true,
            show_shadow: false,
            corner_radius: 4.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kpi_card_creation() {
        let card = KpiCard::new("Revenue", "$1.2M");
        assert_eq!(card.title, "Revenue");
        assert_eq!(card.value, "$1.2M");
        assert_eq!(card.get_span().columns, 12); // Full width by default
    }

    #[test]
    fn test_kpi_card_with_trend() {
        let card = KpiCard::new("Sales", "1,247")
            .with_trend(12.5, TrendDirection::Up)
            .trend_is_good(true);

        let trend = card.trend.unwrap();
        assert_eq!(trend.change, 12.5);
        assert_eq!(trend.direction, TrendDirection::Up);
        assert!(trend.is_good);
    }

    #[test]
    fn test_kpi_card_formatting() {
        let card = KpiCard::new("Price", "1299.99").as_currency("$");

        let formatted = card.format_value();
        assert_eq!(formatted, "$1299.99");
    }

    #[test]
    fn test_kpi_card_builder() {
        let card = KpiCardBuilder::new("Conversion", "3.2")
            .subtitle("vs last month")
            .trend(0.3, TrendDirection::Up)
            .color(Color::blue())
            .sparkline(vec![3.1, 3.0, 3.2, 3.4, 3.2])
            .build();

        assert_eq!(card.title, "Conversion");
        assert!(card.subtitle.is_some());
        assert!(card.trend.is_some());
        assert!(card.sparkline.is_some());
    }

    #[test]
    fn test_sparkline_data() {
        let data = vec![10.0, 12.0, 8.0, 15.0, 11.0];
        let sparkline = SparklineData::new(data.clone());

        assert_eq!(sparkline.data, data);
        assert!(sparkline.labels.is_none());
    }

    #[test]
    fn test_kpi_color_theme() {
        let theme = KpiColorTheme::from_primary(Color::blue());
        assert_eq!(theme.primary, Color::blue());
        assert_eq!(theme.accent, Color::blue());
    }
}
