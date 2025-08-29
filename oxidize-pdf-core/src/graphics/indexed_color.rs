//! Indexed color space support according to ISO 32000-1 Section 8.6.6.3
//!
//! This module provides comprehensive support for indexed color spaces which allow
//! efficient encoding of images with limited color palettes. Indexed color spaces
//! map index values to colors in a base color space, reducing file size for images
//! with few colors.

use crate::error::{PdfError, Result};
use crate::graphics::color::Color;
use crate::graphics::color_profiles::IccColorSpace;
use crate::objects::{Dictionary, Object};
use std::collections::HashMap;

/// Indexed color space representation
#[derive(Debug, Clone)]
pub struct IndexedColorSpace {
    /// Base color space (DeviceRGB, DeviceCMYK, DeviceGray, or ICCBased)
    pub base_space: BaseColorSpace,
    /// Maximum valid index value (0 to hival, max 255)
    pub hival: u8,
    /// Color lookup table
    pub lookup_table: ColorLookupTable,
    /// Optional name for the indexed color space
    pub name: Option<String>,
}

/// Base color space for indexed colors
#[derive(Debug, Clone, PartialEq)]
pub enum BaseColorSpace {
    /// Device RGB
    DeviceRGB,
    /// Device CMYK
    DeviceCMYK,
    /// Device Gray
    DeviceGray,
    /// ICC-based color space
    ICCBased(IccColorSpace),
    /// Separation color space
    Separation(String),
    /// Lab color space
    Lab,
}

impl BaseColorSpace {
    /// Get the number of components for this color space
    pub fn component_count(&self) -> usize {
        match self {
            BaseColorSpace::DeviceGray => 1,
            BaseColorSpace::DeviceRGB | BaseColorSpace::Lab => 3,
            BaseColorSpace::DeviceCMYK => 4,
            BaseColorSpace::ICCBased(icc) => icc.component_count() as usize,
            BaseColorSpace::Separation(_) => 1,
        }
    }

    /// Get the PDF name for this color space
    pub fn pdf_name(&self) -> String {
        match self {
            BaseColorSpace::DeviceGray => "DeviceGray".to_string(),
            BaseColorSpace::DeviceRGB => "DeviceRGB".to_string(),
            BaseColorSpace::DeviceCMYK => "DeviceCMYK".to_string(),
            BaseColorSpace::ICCBased(_) => "ICCBased".to_string(),
            BaseColorSpace::Separation(name) => format!("Separation({})", name),
            BaseColorSpace::Lab => "Lab".to_string(),
        }
    }

    /// Convert to PDF object representation
    pub fn to_pdf_object(&self) -> Object {
        match self {
            BaseColorSpace::DeviceGray => Object::Name("DeviceGray".to_string()),
            BaseColorSpace::DeviceRGB => Object::Name("DeviceRGB".to_string()),
            BaseColorSpace::DeviceCMYK => Object::Name("DeviceCMYK".to_string()),
            BaseColorSpace::Lab => Object::Name("Lab".to_string()),
            BaseColorSpace::ICCBased(_) => {
                // In real implementation, this would reference the ICC profile
                Object::Array(vec![
                    Object::Name("ICCBased".to_string()),
                    Object::Dictionary(Dictionary::new()),
                ])
            }
            BaseColorSpace::Separation(name) => Object::Array(vec![
                Object::Name("Separation".to_string()),
                Object::Name(name.clone()),
            ]),
        }
    }
}

/// Color lookup table for indexed color space
#[derive(Debug, Clone)]
pub struct ColorLookupTable {
    /// Raw color data (packed according to base color space)
    data: Vec<u8>,
    /// Number of components per color
    components_per_color: usize,
    /// Number of colors in the table
    color_count: usize,
}

impl ColorLookupTable {
    /// Create a new color lookup table
    pub fn new(data: Vec<u8>, components_per_color: usize) -> Result<Self> {
        if components_per_color == 0 {
            return Err(PdfError::InvalidStructure(
                "Components per color must be greater than 0".to_string(),
            ));
        }

        if data.len() % components_per_color != 0 {
            return Err(PdfError::InvalidStructure(format!(
                "Color data length {} is not a multiple of components per color {}",
                data.len(),
                components_per_color
            )));
        }

        let color_count = data.len() / components_per_color;
        if color_count > 256 {
            return Err(PdfError::InvalidStructure(format!(
                "Color count {} exceeds maximum of 256",
                color_count
            )));
        }

        Ok(Self {
            data,
            components_per_color,
            color_count,
        })
    }

    /// Create from a list of colors
    pub fn from_colors(colors: &[Color]) -> Result<Self> {
        if colors.is_empty() {
            return Err(PdfError::InvalidStructure(
                "Color list cannot be empty".to_string(),
            ));
        }

        if colors.len() > 256 {
            return Err(PdfError::InvalidStructure(format!(
                "Color count {} exceeds maximum of 256",
                colors.len()
            )));
        }

        // Determine base color space from first color
        let (components_per_color, data) = match &colors[0] {
            Color::Gray(_) => {
                let mut data = Vec::with_capacity(colors.len());
                for color in colors {
                    if let Color::Gray(g) = color {
                        data.push((g * 255.0) as u8);
                    } else {
                        return Err(PdfError::InvalidStructure(
                            "All colors must be of the same type".to_string(),
                        ));
                    }
                }
                (1, data)
            }
            Color::Rgb(_, _, _) => {
                let mut data = Vec::with_capacity(colors.len() * 3);
                for color in colors {
                    if let Color::Rgb(r, g, b) = color {
                        data.push((r * 255.0) as u8);
                        data.push((g * 255.0) as u8);
                        data.push((b * 255.0) as u8);
                    } else {
                        return Err(PdfError::InvalidStructure(
                            "All colors must be of the same type".to_string(),
                        ));
                    }
                }
                (3, data)
            }
            Color::Cmyk(_, _, _, _) => {
                let mut data = Vec::with_capacity(colors.len() * 4);
                for color in colors {
                    if let Color::Cmyk(c, m, y, k) = color {
                        data.push((c * 255.0) as u8);
                        data.push((m * 255.0) as u8);
                        data.push((y * 255.0) as u8);
                        data.push((k * 255.0) as u8);
                    } else {
                        return Err(PdfError::InvalidStructure(
                            "All colors must be of the same type".to_string(),
                        ));
                    }
                }
                (4, data)
            }
        };

        Ok(Self {
            data,
            components_per_color,
            color_count: colors.len(),
        })
    }

    /// Get color at index
    pub fn get_color(&self, index: u8) -> Option<Vec<f64>> {
        let idx = index as usize;
        if idx >= self.color_count {
            return None;
        }

        let start = idx * self.components_per_color;
        let end = start + self.components_per_color;

        let components: Vec<f64> = self.data[start..end]
            .iter()
            .map(|&b| b as f64 / 255.0)
            .collect();

        Some(components)
    }

    /// Get raw color data at index (as bytes)
    pub fn get_raw_color(&self, index: u8) -> Option<&[u8]> {
        let idx = index as usize;
        if idx >= self.color_count {
            return None;
        }

        let start = idx * self.components_per_color;
        let end = start + self.components_per_color;
        Some(&self.data[start..end])
    }

    /// Get the number of colors in the table
    pub fn color_count(&self) -> usize {
        self.color_count
    }

    /// Get components per color
    pub fn components_per_color(&self) -> usize {
        self.components_per_color
    }

    /// Get raw data
    pub fn raw_data(&self) -> &[u8] {
        &self.data
    }
}

impl IndexedColorSpace {
    /// Create a new indexed color space
    pub fn new(base_space: BaseColorSpace, lookup_table: ColorLookupTable) -> Result<Self> {
        // Validate that lookup table matches base space
        let expected_components = base_space.component_count();
        if lookup_table.components_per_color != expected_components {
            return Err(PdfError::InvalidStructure(format!(
                "Lookup table has {} components per color but base space {} requires {}",
                lookup_table.components_per_color,
                base_space.pdf_name(),
                expected_components
            )));
        }

        let hival = (lookup_table.color_count() - 1) as u8;

        Ok(Self {
            base_space,
            hival,
            lookup_table,
            name: None,
        })
    }

    /// Create an indexed color space from a palette
    pub fn from_palette(colors: &[Color]) -> Result<Self> {
        let lookup_table = ColorLookupTable::from_colors(colors)?;

        let base_space = match &colors[0] {
            Color::Gray(_) => BaseColorSpace::DeviceGray,
            Color::Rgb(_, _, _) => BaseColorSpace::DeviceRGB,
            Color::Cmyk(_, _, _, _) => BaseColorSpace::DeviceCMYK,
        };

        Self::new(base_space, lookup_table)
    }

    /// Create a web-safe color palette (216 colors)
    pub fn web_safe_palette() -> Result<Self> {
        let mut colors = Vec::with_capacity(216);

        for r in 0..6 {
            for g in 0..6 {
                for b in 0..6 {
                    colors.push(Color::rgb(r as f64 * 0.2, g as f64 * 0.2, b as f64 * 0.2));
                }
            }
        }

        Self::from_palette(&colors)
    }

    /// Create a grayscale palette
    pub fn grayscale_palette(levels: u8) -> Result<Self> {
        if levels == 0 {
            return Err(PdfError::InvalidStructure(
                "Grayscale levels must be between 1 and 255".to_string(),
            ));
        }

        let mut colors = Vec::with_capacity(levels as usize);
        for i in 0..levels {
            let gray = i as f64 / (levels - 1) as f64;
            colors.push(Color::gray(gray));
        }

        Self::from_palette(&colors)
    }

    /// Set the name for this indexed color space
    pub fn with_name(mut self, name: String) -> Self {
        self.name = Some(name);
        self
    }

    /// Get color at index
    pub fn get_color(&self, index: u8) -> Option<Color> {
        let components = self.lookup_table.get_color(index)?;

        match self.base_space {
            BaseColorSpace::DeviceGray => Some(Color::gray(components[0])),
            BaseColorSpace::DeviceRGB | BaseColorSpace::Lab => {
                Some(Color::rgb(components[0], components[1], components[2]))
            }
            BaseColorSpace::DeviceCMYK => Some(Color::cmyk(
                components[0],
                components[1],
                components[2],
                components[3],
            )),
            _ => None,
        }
    }

    /// Find closest color index for a given color
    pub fn find_closest_index(&self, target: &Color) -> u8 {
        let mut best_index = 0;
        let mut best_distance = f64::MAX;

        for i in 0..=self.hival {
            if let Some(color) = self.get_color(i) {
                let distance = self.color_distance(target, &color);
                if distance < best_distance {
                    best_distance = distance;
                    best_index = i;
                }
            }
        }

        best_index
    }

    /// Calculate color distance (Euclidean)
    fn color_distance(&self, c1: &Color, c2: &Color) -> f64 {
        match (c1, c2) {
            (Color::Gray(g1), Color::Gray(g2)) => (g1 - g2).abs(),
            (Color::Rgb(r1, g1, b1), Color::Rgb(r2, g2, b2)) => {
                let dr = r1 - r2;
                let dg = g1 - g2;
                let db = b1 - b2;
                (dr * dr + dg * dg + db * db).sqrt()
            }
            (Color::Cmyk(c1, m1, y1, k1), Color::Cmyk(c2, m2, y2, k2)) => {
                let dc = c1 - c2;
                let dm = m1 - m2;
                let dy = y1 - y2;
                let dk = k1 - k2;
                (dc * dc + dm * dm + dy * dy + dk * dk).sqrt()
            }
            _ => f64::MAX,
        }
    }

    /// Convert to PDF color space array
    pub fn to_pdf_array(&self) -> Result<Vec<Object>> {
        let array = vec![
            // Color space name
            Object::Name("Indexed".to_string()),
            // Base color space
            self.base_space.to_pdf_object(),
            // Maximum index value
            Object::Integer(self.hival as i64),
            // Lookup table as string (raw bytes)
            Object::String(String::from_utf8_lossy(self.lookup_table.raw_data()).to_string()),
        ];

        Ok(array)
    }

    /// Get the maximum valid index
    pub fn max_index(&self) -> u8 {
        self.hival
    }

    /// Get the number of colors
    pub fn color_count(&self) -> usize {
        (self.hival as usize) + 1
    }

    /// Validate the indexed color space
    pub fn validate(&self) -> Result<()> {
        if self.hival as usize >= self.lookup_table.color_count() {
            return Err(PdfError::InvalidStructure(format!(
                "hival {} exceeds lookup table size {}",
                self.hival,
                self.lookup_table.color_count()
            )));
        }

        Ok(())
    }
}

/// Indexed color space manager
#[derive(Debug, Clone, Default)]
pub struct IndexedColorManager {
    /// Registered indexed color spaces
    spaces: HashMap<String, IndexedColorSpace>,
    /// Color to index cache for performance
    cache: HashMap<String, HashMap<String, u8>>,
}

impl IndexedColorManager {
    /// Create a new indexed color manager
    pub fn new() -> Self {
        Self::default()
    }

    /// Add an indexed color space
    pub fn add_space(&mut self, name: String, space: IndexedColorSpace) -> Result<()> {
        space.validate()?;
        self.spaces.insert(name.clone(), space);
        self.cache.insert(name, HashMap::new());
        Ok(())
    }

    /// Get an indexed color space
    pub fn get_space(&self, name: &str) -> Option<&IndexedColorSpace> {
        self.spaces.get(name)
    }

    /// Get or create index for a color in a space
    pub fn get_color_index(&mut self, space_name: &str, color: &Color) -> Option<u8> {
        let space = self.spaces.get(space_name)?;

        // Check cache first
        let color_key = format!("{:?}", color);
        if let Some(cache) = self.cache.get(space_name) {
            if let Some(&index) = cache.get(&color_key) {
                return Some(index);
            }
        }

        // Find closest color
        let index = space.find_closest_index(color);

        // Update cache
        if let Some(cache) = self.cache.get_mut(space_name) {
            cache.insert(color_key, index);
        }

        Some(index)
    }

    /// Create standard palettes
    pub fn create_web_safe(&mut self) -> Result<String> {
        let name = "WebSafe".to_string();
        let space = IndexedColorSpace::web_safe_palette()?;
        self.add_space(name.clone(), space)?;
        Ok(name)
    }

    /// Create grayscale palette
    pub fn create_grayscale(&mut self, levels: u8) -> Result<String> {
        let name = format!("Gray{}", levels);
        let space = IndexedColorSpace::grayscale_palette(levels)?;
        self.add_space(name.clone(), space)?;
        Ok(name)
    }

    /// Get all space names
    pub fn space_names(&self) -> Vec<String> {
        self.spaces.keys().cloned().collect()
    }

    /// Clear all spaces
    pub fn clear(&mut self) {
        self.spaces.clear();
        self.cache.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base_color_space_components() {
        assert_eq!(BaseColorSpace::DeviceGray.component_count(), 1);
        assert_eq!(BaseColorSpace::DeviceRGB.component_count(), 3);
        assert_eq!(BaseColorSpace::DeviceCMYK.component_count(), 4);
        assert_eq!(BaseColorSpace::Lab.component_count(), 3);
        assert_eq!(
            BaseColorSpace::Separation("Spot".to_string()).component_count(),
            1
        );
    }

    #[test]
    fn test_color_lookup_table_creation() {
        let data = vec![255, 0, 0, 0, 255, 0, 0, 0, 255]; // RGB: red, green, blue
        let table = ColorLookupTable::new(data, 3).unwrap();

        assert_eq!(table.color_count(), 3);
        assert_eq!(table.components_per_color(), 3);
    }

    #[test]
    fn test_color_lookup_table_from_colors() {
        let colors = vec![
            Color::rgb(1.0, 0.0, 0.0),
            Color::rgb(0.0, 1.0, 0.0),
            Color::rgb(0.0, 0.0, 1.0),
        ];

        let table = ColorLookupTable::from_colors(&colors).unwrap();
        assert_eq!(table.color_count(), 3);
        assert_eq!(table.components_per_color(), 3);

        // Check first color (red)
        let red = table.get_color(0).unwrap();
        assert!((red[0] - 1.0).abs() < 0.01);
        assert!((red[1] - 0.0).abs() < 0.01);
        assert!((red[2] - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_indexed_color_space_creation() {
        let colors = vec![
            Color::rgb(1.0, 0.0, 0.0),
            Color::rgb(0.0, 1.0, 0.0),
            Color::rgb(0.0, 0.0, 1.0),
        ];

        let space = IndexedColorSpace::from_palette(&colors).unwrap();
        assert_eq!(space.hival, 2);
        assert_eq!(space.color_count(), 3);
    }

    #[test]
    fn test_indexed_color_space_get_color() {
        let colors = vec![
            Color::rgb(1.0, 0.0, 0.0),
            Color::rgb(0.0, 1.0, 0.0),
            Color::rgb(0.0, 0.0, 1.0),
        ];

        let space = IndexedColorSpace::from_palette(&colors).unwrap();

        let red = space.get_color(0).unwrap();
        assert_eq!(red, Color::rgb(1.0, 0.0, 0.0));

        let green = space.get_color(1).unwrap();
        assert_eq!(green, Color::rgb(0.0, 1.0, 0.0));

        let blue = space.get_color(2).unwrap();
        assert_eq!(blue, Color::rgb(0.0, 0.0, 1.0));

        assert!(space.get_color(3).is_none());
    }

    #[test]
    fn test_web_safe_palette() {
        let space = IndexedColorSpace::web_safe_palette().unwrap();
        assert_eq!(space.color_count(), 216);
        assert_eq!(space.hival, 215);
    }

    #[test]
    fn test_grayscale_palette() {
        let space = IndexedColorSpace::grayscale_palette(16).unwrap();
        assert_eq!(space.color_count(), 16);
        assert_eq!(space.hival, 15);

        // Check first and last colors
        let black = space.get_color(0).unwrap();
        assert_eq!(black, Color::gray(0.0));

        let white = space.get_color(15).unwrap();
        assert!(matches!(white, Color::Gray(g) if (g - 1.0).abs() < 0.01));
    }

    #[test]
    fn test_find_closest_index() {
        let colors = vec![
            Color::rgb(1.0, 0.0, 0.0), // Red
            Color::rgb(0.0, 1.0, 0.0), // Green
            Color::rgb(0.0, 0.0, 1.0), // Blue
        ];

        let space = IndexedColorSpace::from_palette(&colors).unwrap();

        // Exact matches
        assert_eq!(space.find_closest_index(&Color::rgb(1.0, 0.0, 0.0)), 0);
        assert_eq!(space.find_closest_index(&Color::rgb(0.0, 1.0, 0.0)), 1);
        assert_eq!(space.find_closest_index(&Color::rgb(0.0, 0.0, 1.0)), 2);

        // Close to red
        assert_eq!(space.find_closest_index(&Color::rgb(0.8, 0.2, 0.1)), 0);

        // Close to green
        assert_eq!(space.find_closest_index(&Color::rgb(0.1, 0.8, 0.2)), 1);
    }

    #[test]
    fn test_indexed_color_manager() {
        let mut manager = IndexedColorManager::new();

        let colors = vec![
            Color::rgb(1.0, 0.0, 0.0),
            Color::rgb(0.0, 1.0, 0.0),
            Color::rgb(0.0, 0.0, 1.0),
        ];

        let space = IndexedColorSpace::from_palette(&colors).unwrap();
        manager.add_space("TestPalette".to_string(), space).unwrap();

        assert!(manager.get_space("TestPalette").is_some());

        let index = manager.get_color_index("TestPalette", &Color::rgb(1.0, 0.0, 0.0));
        assert_eq!(index, Some(0));
    }

    #[test]
    fn test_manager_standard_palettes() {
        let mut manager = IndexedColorManager::new();

        let web_name = manager.create_web_safe().unwrap();
        assert_eq!(web_name, "WebSafe");
        assert!(manager.get_space(&web_name).is_some());

        let gray_name = manager.create_grayscale(255).unwrap();
        assert_eq!(gray_name, "Gray255");
        assert!(manager.get_space(&gray_name).is_some());
    }

    #[test]
    fn test_invalid_lookup_table() {
        // Data length not multiple of components
        let result = ColorLookupTable::new(vec![255, 0], 3);
        assert!(result.is_err());

        // Zero components
        let result = ColorLookupTable::new(vec![255, 0, 0], 0);
        assert!(result.is_err());
    }

    #[test]
    fn test_mismatched_color_types() {
        let colors = vec![
            Color::rgb(1.0, 0.0, 0.0),
            Color::gray(0.5), // Different type
        ];

        let result = ColorLookupTable::from_colors(&colors);
        assert!(result.is_err());
    }

    #[test]
    fn test_too_many_colors() {
        let mut colors = Vec::new();
        for i in 0..257 {
            colors.push(Color::gray(i as f64 / 256.0));
        }

        let result = ColorLookupTable::from_colors(&colors);
        assert!(result.is_err());
    }

    #[test]
    fn test_cmyk_indexed_space() {
        let colors = vec![
            Color::cmyk(1.0, 0.0, 0.0, 0.0), // Cyan
            Color::cmyk(0.0, 1.0, 0.0, 0.0), // Magenta
            Color::cmyk(0.0, 0.0, 1.0, 0.0), // Yellow
            Color::cmyk(0.0, 0.0, 0.0, 1.0), // Black
        ];

        let space = IndexedColorSpace::from_palette(&colors).unwrap();
        assert_eq!(space.base_space, BaseColorSpace::DeviceCMYK);
        assert_eq!(space.color_count(), 4);

        let cyan = space.get_color(0).unwrap();
        assert_eq!(cyan, Color::cmyk(1.0, 0.0, 0.0, 0.0));
    }
}
