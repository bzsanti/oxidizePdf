//! Dashboard Theming System
//!
//! This module provides a comprehensive theming system for dashboards, including
//! color palettes, typography, spacing, and pre-defined themes for different
//! use cases (corporate, minimal, dark, colorful, etc.).

use crate::graphics::Color;

/// Font weight enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontWeight {
    Normal,
    Bold,
}

/// Main theme configuration for dashboards
#[derive(Debug, Clone)]
pub struct DashboardTheme {
    /// Color palette for the theme
    pub colors: ThemeColors,
    /// Typography settings
    pub typography: Typography,
    /// Spacing and layout settings
    pub spacing: ThemeSpacing,
    /// Border and shadow settings
    pub borders: ThemeBorders,
    /// Background settings
    pub backgrounds: ThemeBackgrounds,
}

impl DashboardTheme {
    /// Create a new custom theme
    pub fn new(colors: ThemeColors, typography: Typography) -> Self {
        Self {
            colors,
            typography,
            spacing: ThemeSpacing::default(),
            borders: ThemeBorders::default(),
            backgrounds: ThemeBackgrounds::default(),
        }
    }
    
    /// Corporate theme - professional, blue-based palette
    pub fn corporate() -> Self {
        let colors = ThemeColors {
            primary: Color::hex("#1f4788"),
            secondary: Color::hex("#4a90a4"),
            accent: Color::hex("#87ceeb"),
            success: Color::hex("#28a745"),
            warning: Color::hex("#ffc107"),
            danger: Color::hex("#dc3545"),
            info: Color::hex("#17a2b8"),
            light: Color::hex("#f8f9fa"),
            dark: Color::hex("#343a40"),
            muted: Color::hex("#6c757d"),
            background: Color::white(),
            surface: Color::hex("#f0f4f8"), // Light blue-gray instead of white
            text_primary: Color::hex("#212529"),
            text_secondary: Color::hex("#6c757d"),
            text_muted: Color::hex("#adb5bd"),
            border: Color::hex("#dee2e6"),
        };
        
        let typography = Typography {
            title_font: "Helvetica-Bold".to_string(),
            title_size: 24.0,
            title_color: colors.text_primary,
            heading_font: "Helvetica-Bold".to_string(),
            heading_size: 18.0,
            heading_color: colors.text_primary,
            body_font: "Helvetica".to_string(),
            body_size: 12.0,
            body_color: colors.text_primary,
            caption_font: "Helvetica".to_string(),
            caption_size: 10.0,
            caption_color: colors.text_secondary,
            line_height: 1.4,
        };
        
        Self {
            colors,
            typography,
            spacing: ThemeSpacing::corporate(),
            borders: ThemeBorders::corporate(),
            backgrounds: ThemeBackgrounds::corporate(),
        }
    }
    
    /// Minimal theme - clean, grayscale palette
    pub fn minimal() -> Self {
        let colors = ThemeColors {
            primary: Color::hex("#000000"),
            secondary: Color::hex("#666666"),
            accent: Color::hex("#999999"),
            success: Color::hex("#4caf50"),
            warning: Color::hex("#ff9800"),
            danger: Color::hex("#f44336"),
            info: Color::hex("#2196f3"),
            light: Color::hex("#fafafa"),
            dark: Color::hex("#212121"),
            muted: Color::hex("#757575"),
            background: Color::white(),
            surface: Color::hex("#ffffff"),
            text_primary: Color::hex("#212121"),
            text_secondary: Color::hex("#757575"),
            text_muted: Color::hex("#bdbdbd"),
            border: Color::hex("#e0e0e0"),
        };
        
        let typography = Typography::minimal();
        
        Self {
            colors,
            typography,
            spacing: ThemeSpacing::minimal(),
            borders: ThemeBorders::minimal(),
            backgrounds: ThemeBackgrounds::minimal(),
        }
    }
    
    /// Dark theme - dark background with light text
    pub fn dark() -> Self {
        let colors = ThemeColors {
            primary: Color::hex("#bb86fc"),
            secondary: Color::hex("#03dac6"),
            accent: Color::hex("#cf6679"),
            success: Color::hex("#4caf50"),
            warning: Color::hex("#ff9800"),
            danger: Color::hex("#f44336"),
            info: Color::hex("#2196f3"),
            light: Color::hex("#424242"),
            dark: Color::hex("#121212"),
            muted: Color::hex("#757575"),
            background: Color::hex("#121212"),
            surface: Color::hex("#1e1e1e"),
            text_primary: Color::hex("#ffffff"),
            text_secondary: Color::hex("#b3b3b3"),
            text_muted: Color::hex("#666666"),
            border: Color::hex("#333333"),
        };
        
        let typography = Typography::dark();
        
        Self {
            colors,
            typography,
            spacing: ThemeSpacing::default(),
            borders: ThemeBorders::dark(),
            backgrounds: ThemeBackgrounds::dark(),
        }
    }
    
    /// Colorful theme - vibrant, multi-color palette
    pub fn colorful() -> Self {
        let colors = ThemeColors {
            primary: Color::hex("#e91e63"),
            secondary: Color::hex("#9c27b0"),
            accent: Color::hex("#ff5722"),
            success: Color::hex("#4caf50"),
            warning: Color::hex("#ff9800"),
            danger: Color::hex("#f44336"),
            info: Color::hex("#2196f3"),
            light: Color::hex("#fce4ec"),
            dark: Color::hex("#880e4f"),
            muted: Color::hex("#ad1457"),
            background: Color::hex("#fafafa"),
            surface: Color::white(),
            text_primary: Color::hex("#212121"),
            text_secondary: Color::hex("#757575"),
            text_muted: Color::hex("#bdbdbd"),
            border: Color::hex("#e1bee7"),
        };
        
        let typography = Typography::colorful();
        
        Self {
            colors,
            typography,
            spacing: ThemeSpacing::colorful(),
            borders: ThemeBorders::colorful(),
            backgrounds: ThemeBackgrounds::colorful(),
        }
    }
    
    /// Set custom color palette
    pub fn set_color_palette(&mut self, colors: Vec<Color>) {
        if !colors.is_empty() {
            self.colors.primary = colors[0];
            if colors.len() > 1 {
                self.colors.secondary = colors[1];
            }
            if colors.len() > 2 {
                self.colors.accent = colors[2];
            }
        }
    }
    
    /// Set typography configuration
    pub fn set_typography(&mut self, typography: Typography) {
        self.typography = typography;
    }
    
    /// Get title text style
    pub fn title_style(&self) -> TextStyle {
        TextStyle {
            font_name: self.typography.title_font.clone(),
            font_size: self.typography.title_size,
            color: self.typography.title_color,
            weight: FontWeight::Bold,
            alignment: TextAlignment::Center,
        }
    }
    
    /// Get subtitle text style
    pub fn subtitle_style(&self) -> TextStyle {
        TextStyle {
            font_name: self.typography.heading_font.clone(),
            font_size: self.typography.heading_size,
            color: self.typography.heading_color,
            weight: FontWeight::Normal,
            alignment: TextAlignment::Center,
        }
    }
    
    /// Get footer text style
    pub fn footer_style(&self) -> TextStyle {
        TextStyle {
            font_name: self.typography.caption_font.clone(),
            font_size: self.typography.caption_size,
            color: self.typography.caption_color,
            weight: FontWeight::Normal,
            alignment: TextAlignment::Left,
        }
    }
    
    /// Get title height in points
    pub fn title_height(&self) -> f64 {
        self.typography.title_size * self.typography.line_height
    }
    
    /// Get footer height in points
    pub fn footer_height(&self) -> f64 {
        self.typography.caption_size * self.typography.line_height
    }
}

impl Default for DashboardTheme {
    fn default() -> Self {
        Self::corporate()
    }
}

/// Color palette for dashboard themes
#[derive(Debug, Clone)]
pub struct ThemeColors {
    /// Primary brand color
    pub primary: Color,
    /// Secondary brand color
    pub secondary: Color,
    /// Accent color for highlights
    pub accent: Color,
    /// Success state color (green)
    pub success: Color,
    /// Warning state color (orange)
    pub warning: Color,
    /// Danger/error state color (red)
    pub danger: Color,
    /// Information state color (blue)
    pub info: Color,
    /// Light neutral color
    pub light: Color,
    /// Dark neutral color
    pub dark: Color,
    /// Muted color for less important elements
    pub muted: Color,
    /// Main background color
    pub background: Color,
    /// Surface color (cards, panels)
    pub surface: Color,
    /// Primary text color
    pub text_primary: Color,
    /// Secondary text color
    pub text_secondary: Color,
    /// Muted text color
    pub text_muted: Color,
    /// Border color
    pub border: Color,
}

/// Typography configuration for dashboard themes
#[derive(Debug, Clone)]
pub struct Typography {
    /// Font for titles
    pub title_font: String,
    /// Title font size
    pub title_size: f64,
    /// Title text color
    pub title_color: Color,
    /// Font for headings
    pub heading_font: String,
    /// Heading font size
    pub heading_size: f64,
    /// Heading text color
    pub heading_color: Color,
    /// Font for body text
    pub body_font: String,
    /// Body font size
    pub body_size: f64,
    /// Body text color
    pub body_color: Color,
    /// Font for captions
    pub caption_font: String,
    /// Caption font size
    pub caption_size: f64,
    /// Caption text color
    pub caption_color: Color,
    /// Line height multiplier
    pub line_height: f64,
}

impl Typography {
    /// Professional typography setup
    pub fn professional() -> Self {
        Self {
            title_font: "Helvetica-Bold".to_string(),
            title_size: 24.0,
            title_color: Color::hex("#212529"),
            heading_font: "Helvetica-Bold".to_string(),
            heading_size: 18.0,
            heading_color: Color::hex("#212529"),
            body_font: "Helvetica".to_string(),
            body_size: 12.0,
            body_color: Color::hex("#212529"),
            caption_font: "Helvetica".to_string(),
            caption_size: 10.0,
            caption_color: Color::hex("#6c757d"),
            line_height: 1.4,
        }
    }
    
    /// Minimal typography setup
    pub fn minimal() -> Self {
        Self {
            title_font: "Helvetica-Light".to_string(),
            title_size: 28.0,
            title_color: Color::hex("#212121"),
            heading_font: "Helvetica".to_string(),
            heading_size: 16.0,
            heading_color: Color::hex("#212121"),
            body_font: "Helvetica".to_string(),
            body_size: 11.0,
            body_color: Color::hex("#212121"),
            caption_font: "Helvetica".to_string(),
            caption_size: 9.0,
            caption_color: Color::hex("#757575"),
            line_height: 1.5,
        }
    }
    
    /// Dark theme typography
    pub fn dark() -> Self {
        Self {
            title_font: "Helvetica-Bold".to_string(),
            title_size: 24.0,
            title_color: Color::white(),
            heading_font: "Helvetica-Bold".to_string(),
            heading_size: 18.0,
            heading_color: Color::white(),
            body_font: "Helvetica".to_string(),
            body_size: 12.0,
            body_color: Color::white(),
            caption_font: "Helvetica".to_string(),
            caption_size: 10.0,
            caption_color: Color::hex("#b3b3b3"),
            line_height: 1.4,
        }
    }
    
    /// Colorful theme typography
    pub fn colorful() -> Self {
        Self {
            title_font: "Helvetica-Bold".to_string(),
            title_size: 26.0,
            title_color: Color::hex("#880e4f"),
            heading_font: "Helvetica-Bold".to_string(),
            heading_size: 18.0,
            heading_color: Color::hex("#ad1457"),
            body_font: "Helvetica".to_string(),
            body_size: 12.0,
            body_color: Color::hex("#212121"),
            caption_font: "Helvetica".to_string(),
            caption_size: 10.0,
            caption_color: Color::hex("#757575"),
            line_height: 1.4,
        }
    }
}

impl Default for Typography {
    fn default() -> Self {
        Self::professional()
    }
}

/// Text style for rendering
#[derive(Debug, Clone)]
pub struct TextStyle {
    /// Font name
    pub font_name: String,
    /// Font size in points
    pub font_size: f64,
    /// Text color
    pub color: Color,
    /// Font weight
    pub weight: FontWeight,
    /// Text alignment
    pub alignment: TextAlignment,
}

/// Text alignment options
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextAlignment {
    Left,
    Center,
    Right,
}

/// Spacing configuration for themes
#[derive(Debug, Clone)]
pub struct ThemeSpacing {
    /// Extra small spacing (4pt)
    pub xs: f64,
    /// Small spacing (8pt)
    pub sm: f64,
    /// Medium spacing (16pt)
    pub md: f64,
    /// Large spacing (24pt)
    pub lg: f64,
    /// Extra large spacing (32pt)
    pub xl: f64,
}

impl ThemeSpacing {
    pub fn corporate() -> Self {
        Self {
            xs: 4.0,
            sm: 8.0,
            md: 16.0,
            lg: 24.0,
            xl: 32.0,
        }
    }
    
    pub fn minimal() -> Self {
        Self {
            xs: 2.0,
            sm: 6.0,
            md: 12.0,
            lg: 20.0,
            xl: 28.0,
        }
    }
    
    pub fn colorful() -> Self {
        Self {
            xs: 6.0,
            sm: 10.0,
            md: 18.0,
            lg: 28.0,
            xl: 36.0,
        }
    }
}

impl Default for ThemeSpacing {
    fn default() -> Self {
        Self::corporate()
    }
}

/// Border configuration for themes
#[derive(Debug, Clone)]
pub struct ThemeBorders {
    /// Thin border width
    pub thin: f64,
    /// Medium border width
    pub medium: f64,
    /// Thick border width
    pub thick: f64,
    /// Border radius for rounded corners
    pub radius: f64,
}

impl ThemeBorders {
    pub fn corporate() -> Self {
        Self {
            thin: 0.5,
            medium: 1.0,
            thick: 2.0,
            radius: 4.0,
        }
    }
    
    pub fn minimal() -> Self {
        Self {
            thin: 0.25,
            medium: 0.5,
            thick: 1.0,
            radius: 2.0,
        }
    }
    
    pub fn dark() -> Self {
        Self {
            thin: 0.5,
            medium: 1.0,
            thick: 2.0,
            radius: 6.0,
        }
    }
    
    pub fn colorful() -> Self {
        Self {
            thin: 1.0,
            medium: 2.0,
            thick: 3.0,
            radius: 8.0,
        }
    }
}

impl Default for ThemeBorders {
    fn default() -> Self {
        Self::corporate()
    }
}

/// Background configuration for themes
#[derive(Debug, Clone)]
pub struct ThemeBackgrounds {
    /// Primary background color
    pub primary: Color,
    /// Secondary background color
    pub secondary: Color,
    /// Card/panel background color
    pub surface: Color,
    /// Hover state background
    pub hover: Color,
}

impl ThemeBackgrounds {
    pub fn corporate() -> Self {
        Self {
            primary: Color::white(),
            secondary: Color::hex("#f8f9fa"),
            surface: Color::white(),
            hover: Color::hex("#f5f5f5"),
        }
    }
    
    pub fn minimal() -> Self {
        Self {
            primary: Color::white(),
            secondary: Color::hex("#fafafa"),
            surface: Color::white(),
            hover: Color::hex("#f0f0f0"),
        }
    }
    
    pub fn dark() -> Self {
        Self {
            primary: Color::hex("#121212"),
            secondary: Color::hex("#1e1e1e"),
            surface: Color::hex("#2d2d2d"),
            hover: Color::hex("#3d3d3d"),
        }
    }
    
    pub fn colorful() -> Self {
        Self {
            primary: Color::hex("#fafafa"),
            secondary: Color::hex("#fce4ec"),
            surface: Color::white(),
            hover: Color::hex("#f8bbd9"),
        }
    }
}

impl Default for ThemeBackgrounds {
    fn default() -> Self {
        Self::corporate()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_theme_creation() {
        let theme = DashboardTheme::corporate();
        assert_eq!(theme.typography.title_size, 24.0);
        assert_eq!(theme.colors.primary, Color::hex("#1f4788"));
    }
    
    #[test]
    fn test_theme_variants() {
        let corporate = DashboardTheme::corporate();
        let minimal = DashboardTheme::minimal();
        let dark = DashboardTheme::dark();
        let colorful = DashboardTheme::colorful();
        
        // Themes should have different color schemes
        assert_ne!(corporate.colors.primary, minimal.colors.primary);
        assert_ne!(minimal.colors.background, dark.colors.background);
        assert_ne!(dark.colors.accent, colorful.colors.accent);
    }
    
    #[test]
    fn test_typography_variants() {
        let professional = Typography::professional();
        let minimal = Typography::minimal();
        
        assert_eq!(professional.title_font, "Helvetica-Bold");
        assert_eq!(minimal.title_font, "Helvetica-Light");
        assert_ne!(professional.title_size, minimal.title_size);
    }
    
    #[test]
    fn test_theme_spacing() {
        let spacing = ThemeSpacing::corporate();
        assert_eq!(spacing.xs, 4.0);
        assert_eq!(spacing.sm, 8.0);
        assert_eq!(spacing.md, 16.0);
        assert_eq!(spacing.lg, 24.0);
        assert_eq!(spacing.xl, 32.0);
    }
    
    #[test]
    fn test_text_style_creation() {
        let theme = DashboardTheme::default();
        let title_style = theme.title_style();
        
        assert_eq!(title_style.alignment, TextAlignment::Center);
        assert_eq!(title_style.weight, FontWeight::Bold);
    }
}