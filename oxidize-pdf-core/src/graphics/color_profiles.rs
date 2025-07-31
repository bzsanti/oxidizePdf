//! ICC Color Profile support for PDF graphics according to ISO 32000-1 Section 8.6.5.5
//!
//! This module provides basic support for ICC-based color spaces including:
//! - ICC profile embedding
//! - Basic ICC color space definitions
//! - RGB, CMYK, and Lab ICC profiles
//! - Color space dictionaries

use crate::error::{PdfError, Result};
use crate::objects::{Dictionary, Object};
use std::collections::HashMap;

/// ICC color profile data
#[derive(Debug, Clone)]
pub struct IccProfile {
    /// Profile name for referencing
    pub name: String,
    /// Raw ICC profile data
    pub data: Vec<u8>,
    /// Number of color components
    pub components: u8,
    /// Color space type (RGB, CMYK, Lab, etc.)
    pub color_space: IccColorSpace,
    /// Range array for color components
    pub range: Option<Vec<f64>>,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

/// ICC color space types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum IccColorSpace {
    /// RGB color space (3 components)
    Rgb,
    /// CMYK color space (4 components)
    Cmyk,
    /// Lab color space (3 components)
    Lab,
    /// Gray color space (1 component)
    Gray,
    /// Generic multi-component color space
    Generic(u8),
}

impl IccColorSpace {
    /// Get the number of components for this color space
    pub fn component_count(&self) -> u8 {
        match self {
            IccColorSpace::Gray => 1,
            IccColorSpace::Rgb | IccColorSpace::Lab => 3,
            IccColorSpace::Cmyk => 4,
            IccColorSpace::Generic(n) => *n,
        }
    }

    /// Get the PDF name for this color space type
    pub fn pdf_name(&self) -> &'static str {
        match self {
            IccColorSpace::Gray => "DeviceGray",
            IccColorSpace::Rgb => "DeviceRGB",
            IccColorSpace::Cmyk => "DeviceCMYK",
            IccColorSpace::Lab => "Lab",
            IccColorSpace::Generic(_) => "ICCBased",
        }
    }

    /// Get default range for color components
    pub fn default_range(&self) -> Vec<f64> {
        match self {
            IccColorSpace::Gray => vec![0.0, 1.0],
            IccColorSpace::Rgb => vec![0.0, 1.0, 0.0, 1.0, 0.0, 1.0],
            IccColorSpace::Cmyk => vec![0.0, 1.0, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0],
            IccColorSpace::Lab => vec![0.0, 100.0, -128.0, 127.0, -128.0, 127.0],
            IccColorSpace::Generic(n) => {
                let mut range = Vec::new();
                for _ in 0..*n {
                    range.extend_from_slice(&[0.0, 1.0]);
                }
                range
            }
        }
    }
}

/// Standard ICC profiles commonly used in PDF
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StandardIccProfile {
    /// sRGB color space
    SRgb,
    /// Adobe RGB (1998)
    AdobeRgb,
    /// ProPhoto RGB
    ProPhotoRgb,
    /// U.S. Web Coated (SWOP) v2
    UswcSwopV2,
    /// Coated FOGRA39 (ISO 12647-2:2004)
    CoatedFogra39,
    /// Uncoated FOGRA29 (ISO 12647-2:2004)  
    UncoatedFogra29,
    /// Generic Gray Gamma 2.2
    GrayGamma22,
}

impl StandardIccProfile {
    /// Get the color space for this standard profile
    pub fn color_space(&self) -> IccColorSpace {
        match self {
            StandardIccProfile::SRgb
            | StandardIccProfile::AdobeRgb
            | StandardIccProfile::ProPhotoRgb => IccColorSpace::Rgb,
            StandardIccProfile::UswcSwopV2
            | StandardIccProfile::CoatedFogra39
            | StandardIccProfile::UncoatedFogra29 => IccColorSpace::Cmyk,
            StandardIccProfile::GrayGamma22 => IccColorSpace::Gray,
        }
    }

    /// Get the standard name for this profile
    pub fn profile_name(&self) -> &'static str {
        match self {
            StandardIccProfile::SRgb => "sRGB IEC61966-2.1",
            StandardIccProfile::AdobeRgb => "Adobe RGB (1998)",
            StandardIccProfile::ProPhotoRgb => "ProPhoto RGB",
            StandardIccProfile::UswcSwopV2 => "U.S. Web Coated (SWOP) v2",
            StandardIccProfile::CoatedFogra39 => "Coated FOGRA39 (ISO 12647-2:2004)",
            StandardIccProfile::UncoatedFogra29 => "Uncoated FOGRA29 (ISO 12647-2:2004)",
            StandardIccProfile::GrayGamma22 => "Generic Gray Gamma 2.2",
        }
    }

    /// Get a minimal ICC profile data for this standard profile
    /// Note: In a real implementation, these would be actual ICC profile files
    pub fn minimal_profile_data(&self) -> Vec<u8> {
        // This is a placeholder - real ICC profiles are binary data
        // In production, you would embed actual ICC profile files
        let profile_name = self.profile_name();
        let mut data = Vec::new();

        // ICC profile header (simplified)
        data.extend_from_slice(b"ADSP"); // Profile CMM type
        data.extend_from_slice(&[0; 124]); // Placeholder header

        // Add profile description
        data.extend_from_slice(profile_name.as_bytes());

        // Pad to minimum size
        while data.len() < 128 {
            data.push(0);
        }

        data
    }
}

impl IccProfile {
    /// Create a new ICC profile
    pub fn new(name: String, data: Vec<u8>, color_space: IccColorSpace) -> Self {
        let components = color_space.component_count();

        Self {
            name,
            data,
            components,
            color_space,
            range: Some(color_space.default_range()),
            metadata: HashMap::new(),
        }
    }

    /// Create ICC profile from standard profile
    pub fn from_standard(profile: StandardIccProfile) -> Self {
        let color_space = profile.color_space();
        let data = profile.minimal_profile_data();

        Self::new(profile.profile_name().to_string(), data, color_space)
    }

    /// Set custom range for color components
    pub fn with_range(mut self, range: Vec<f64>) -> Self {
        // Validate range array length
        let expected_len = (self.components as usize) * 2;
        if range.len() == expected_len {
            self.range = Some(range);
        }
        self
    }

    /// Add metadata to the profile
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }

    /// Generate ICC-based color space array for PDF
    pub fn to_pdf_color_space_array(&self) -> Result<Vec<Object>> {
        let mut array = Vec::new();

        // Color space name
        array.push(Object::Name("ICCBased".to_string()));

        // ICC stream dictionary (simplified - would reference actual stream)
        let mut icc_dict = Dictionary::new();
        icc_dict.set("N", Object::Integer(self.components as i64));

        // Alternate color space
        icc_dict.set(
            "Alternate",
            Object::Name(self.color_space.pdf_name().to_string()),
        );

        // Range array
        if let Some(ref range) = self.range {
            let range_objects: Vec<Object> = range.iter().map(|&x| Object::Real(x)).collect();
            icc_dict.set("Range", Object::Array(range_objects));
        }

        // Add metadata if present
        if !self.metadata.is_empty() {
            // In a real implementation, this would create a metadata dictionary
            // For now, we'll add a simple description
            if let Some(desc) = self.metadata.get("Description") {
                icc_dict.set("Description", Object::String(desc.clone()));
            }
        }

        array.push(Object::Dictionary(icc_dict));

        Ok(array)
    }

    /// Validate ICC profile data
    pub fn validate(&self) -> Result<()> {
        // Basic validation
        if self.data.is_empty() {
            return Err(PdfError::InvalidStructure(
                "ICC profile data cannot be empty".to_string(),
            ));
        }

        if self.data.len() < 128 {
            return Err(PdfError::InvalidStructure(
                "ICC profile data too small (minimum 128 bytes)".to_string(),
            ));
        }

        if self.components == 0 || self.components > 15 {
            return Err(PdfError::InvalidStructure(
                "Invalid number of color components".to_string(),
            ));
        }

        // Validate range array if present
        if let Some(ref range) = self.range {
            let expected_len = (self.components as usize) * 2;
            if range.len() != expected_len {
                return Err(PdfError::InvalidStructure(format!(
                    "Range array length {} does not match expected {} for {} components",
                    range.len(),
                    expected_len,
                    self.components
                )));
            }

            // Check that min <= max for each component
            for i in 0..self.components as usize {
                let min = range[i * 2];
                let max = range[i * 2 + 1];
                if min > max {
                    return Err(PdfError::InvalidStructure(format!(
                        "Invalid range for component {}: min {} > max {}",
                        i, min, max
                    )));
                }
            }
        }

        Ok(())
    }

    /// Get profile size in bytes
    pub fn size(&self) -> usize {
        self.data.len()
    }

    /// Check if profile is for RGB color space
    pub fn is_rgb(&self) -> bool {
        matches!(self.color_space, IccColorSpace::Rgb)
    }

    /// Check if profile is for CMYK color space
    pub fn is_cmyk(&self) -> bool {
        matches!(self.color_space, IccColorSpace::Cmyk)
    }

    /// Check if profile is for grayscale
    pub fn is_gray(&self) -> bool {
        matches!(self.color_space, IccColorSpace::Gray)
    }
}

/// ICC profile manager for handling multiple color profiles
#[derive(Debug, Clone)]
pub struct IccProfileManager {
    /// Stored profiles
    profiles: HashMap<String, IccProfile>,
    /// Next profile ID
    next_id: usize,
}

impl Default for IccProfileManager {
    fn default() -> Self {
        Self::new()
    }
}

impl IccProfileManager {
    /// Create a new ICC profile manager
    pub fn new() -> Self {
        Self {
            profiles: HashMap::new(),
            next_id: 1,
        }
    }

    /// Add an ICC profile
    pub fn add_profile(&mut self, mut profile: IccProfile) -> Result<String> {
        // Validate profile before adding
        profile.validate()?;

        // Generate unique name if not provided or already exists
        if profile.name.is_empty() || self.profiles.contains_key(&profile.name) {
            profile.name = format!("ICC{}", self.next_id);
            self.next_id += 1;
        }

        let name = profile.name.clone();
        self.profiles.insert(name.clone(), profile);
        Ok(name)
    }

    /// Add a standard ICC profile
    pub fn add_standard_profile(&mut self, standard_profile: StandardIccProfile) -> Result<String> {
        let profile = IccProfile::from_standard(standard_profile);
        self.add_profile(profile)
    }

    /// Get an ICC profile by name
    pub fn get_profile(&self, name: &str) -> Option<&IccProfile> {
        self.profiles.get(name)
    }

    /// Get all profiles
    pub fn profiles(&self) -> &HashMap<String, IccProfile> {
        &self.profiles
    }

    /// Remove a profile
    pub fn remove_profile(&mut self, name: &str) -> Option<IccProfile> {
        self.profiles.remove(name)
    }

    /// Clear all profiles
    pub fn clear(&mut self) {
        self.profiles.clear();
        self.next_id = 1;
    }

    /// Count of registered profiles
    pub fn count(&self) -> usize {
        self.profiles.len()
    }

    /// Generate ICC profile resource dictionary for PDF
    pub fn to_resource_dictionary(&self) -> Result<String> {
        if self.profiles.is_empty() {
            return Ok(String::new());
        }

        let mut dict = String::from("/ColorSpace <<");

        for (name, _profile) in &self.profiles {
            // In a real implementation, this would reference the color space object
            dict.push_str(&format!(" /{} {} 0 R", name, self.next_id));
        }

        dict.push_str(" >>");
        Ok(dict)
    }

    /// Get profiles by color space type
    pub fn get_profiles_by_type(&self, color_space: IccColorSpace) -> Vec<&IccProfile> {
        self.profiles
            .values()
            .filter(|profile| profile.color_space == color_space)
            .collect()
    }

    /// Get RGB profiles
    pub fn get_rgb_profiles(&self) -> Vec<&IccProfile> {
        self.get_profiles_by_type(IccColorSpace::Rgb)
    }

    /// Get CMYK profiles
    pub fn get_cmyk_profiles(&self) -> Vec<&IccProfile> {
        self.get_profiles_by_type(IccColorSpace::Cmyk)
    }

    /// Get grayscale profiles
    pub fn get_gray_profiles(&self) -> Vec<&IccProfile> {
        self.get_profiles_by_type(IccColorSpace::Gray)
    }

    /// Create a default sRGB profile
    pub fn create_default_srgb(&mut self) -> Result<String> {
        self.add_standard_profile(StandardIccProfile::SRgb)
    }

    /// Create a default CMYK profile
    pub fn create_default_cmyk(&mut self) -> Result<String> {
        self.add_standard_profile(StandardIccProfile::CoatedFogra39)
    }

    /// Create a default grayscale profile
    pub fn create_default_gray(&mut self) -> Result<String> {
        self.add_standard_profile(StandardIccProfile::GrayGamma22)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_icc_color_space_component_count() {
        assert_eq!(IccColorSpace::Gray.component_count(), 1);
        assert_eq!(IccColorSpace::Rgb.component_count(), 3);
        assert_eq!(IccColorSpace::Cmyk.component_count(), 4);
        assert_eq!(IccColorSpace::Lab.component_count(), 3);
        assert_eq!(IccColorSpace::Generic(5).component_count(), 5);
    }

    #[test]
    fn test_icc_color_space_pdf_names() {
        assert_eq!(IccColorSpace::Gray.pdf_name(), "DeviceGray");
        assert_eq!(IccColorSpace::Rgb.pdf_name(), "DeviceRGB");
        assert_eq!(IccColorSpace::Cmyk.pdf_name(), "DeviceCMYK");
        assert_eq!(IccColorSpace::Lab.pdf_name(), "Lab");
        assert_eq!(IccColorSpace::Generic(3).pdf_name(), "ICCBased");
    }

    #[test]
    fn test_icc_color_space_default_range() {
        let gray_range = IccColorSpace::Gray.default_range();
        assert_eq!(gray_range, vec![0.0, 1.0]);

        let rgb_range = IccColorSpace::Rgb.default_range();
        assert_eq!(rgb_range, vec![0.0, 1.0, 0.0, 1.0, 0.0, 1.0]);

        let cmyk_range = IccColorSpace::Cmyk.default_range();
        assert_eq!(cmyk_range, vec![0.0, 1.0, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0]);
    }

    #[test]
    fn test_standard_icc_profile_properties() {
        assert_eq!(StandardIccProfile::SRgb.color_space(), IccColorSpace::Rgb);
        assert_eq!(
            StandardIccProfile::UswcSwopV2.color_space(),
            IccColorSpace::Cmyk
        );
        assert_eq!(
            StandardIccProfile::GrayGamma22.color_space(),
            IccColorSpace::Gray
        );

        assert_eq!(StandardIccProfile::SRgb.profile_name(), "sRGB IEC61966-2.1");
        assert_eq!(
            StandardIccProfile::AdobeRgb.profile_name(),
            "Adobe RGB (1998)"
        );
    }

    #[test]
    fn test_standard_icc_profile_minimal_data() {
        let profile_data = StandardIccProfile::SRgb.minimal_profile_data();
        assert!(!profile_data.is_empty());
        assert!(profile_data.len() >= 128);
    }

    #[test]
    fn test_icc_profile_creation() {
        let data = vec![0u8; 200];
        let profile = IccProfile::new("TestProfile".to_string(), data.clone(), IccColorSpace::Rgb);

        assert_eq!(profile.name, "TestProfile");
        assert_eq!(profile.data, data);
        assert_eq!(profile.components, 3);
        assert_eq!(profile.color_space, IccColorSpace::Rgb);
        assert!(profile.range.is_some());
    }

    #[test]
    fn test_icc_profile_from_standard() {
        let profile = IccProfile::from_standard(StandardIccProfile::SRgb);
        assert_eq!(profile.name, "sRGB IEC61966-2.1");
        assert_eq!(profile.color_space, IccColorSpace::Rgb);
        assert_eq!(profile.components, 3);
        assert!(!profile.data.is_empty());
    }

    #[test]
    fn test_icc_profile_with_range() {
        let data = vec![0u8; 200];
        let custom_range = vec![0.0, 255.0, 0.0, 255.0, 0.0, 255.0];
        let profile = IccProfile::new("TestProfile".to_string(), data, IccColorSpace::Rgb)
            .with_range(custom_range.clone());

        assert_eq!(profile.range, Some(custom_range));
    }

    #[test]
    fn test_icc_profile_with_metadata() {
        let data = vec![0u8; 200];
        let profile = IccProfile::new("TestProfile".to_string(), data, IccColorSpace::Rgb)
            .with_metadata("Description".to_string(), "Test RGB Profile".to_string());

        assert_eq!(
            profile.metadata.get("Description"),
            Some(&"Test RGB Profile".to_string())
        );
    }

    #[test]
    fn test_icc_profile_validation_valid() {
        let data = vec![0u8; 200];
        let profile = IccProfile::new("TestProfile".to_string(), data, IccColorSpace::Rgb);

        assert!(profile.validate().is_ok());
    }

    #[test]
    fn test_icc_profile_validation_empty_data() {
        let profile = IccProfile::new("TestProfile".to_string(), Vec::new(), IccColorSpace::Rgb);

        assert!(profile.validate().is_err());
    }

    #[test]
    fn test_icc_profile_validation_too_small() {
        let data = vec![0u8; 50]; // Too small
        let profile = IccProfile::new("TestProfile".to_string(), data, IccColorSpace::Rgb);

        assert!(profile.validate().is_err());
    }

    #[test]
    fn test_icc_profile_validation_invalid_range() {
        let data = vec![0u8; 200];
        let invalid_range = vec![1.0, 0.0]; // min > max
        let profile = IccProfile::new("TestProfile".to_string(), data, IccColorSpace::Gray)
            .with_range(invalid_range);

        assert!(profile.validate().is_err());
    }

    #[test]
    fn test_icc_profile_color_space_checks() {
        let rgb_profile = IccProfile::from_standard(StandardIccProfile::SRgb);
        assert!(rgb_profile.is_rgb());
        assert!(!rgb_profile.is_cmyk());
        assert!(!rgb_profile.is_gray());

        let cmyk_profile = IccProfile::from_standard(StandardIccProfile::CoatedFogra39);
        assert!(!cmyk_profile.is_rgb());
        assert!(cmyk_profile.is_cmyk());
        assert!(!cmyk_profile.is_gray());

        let gray_profile = IccProfile::from_standard(StandardIccProfile::GrayGamma22);
        assert!(!gray_profile.is_rgb());
        assert!(!gray_profile.is_cmyk());
        assert!(gray_profile.is_gray());
    }

    #[test]
    fn test_icc_profile_to_pdf_color_space_array() {
        let profile = IccProfile::from_standard(StandardIccProfile::SRgb);
        let array = profile.to_pdf_color_space_array().unwrap();

        assert_eq!(array.len(), 2);

        if let Object::Name(name) = &array[0] {
            assert_eq!(name, "ICCBased");
        } else {
            panic!("First element should be ICCBased name");
        }

        if let Object::Dictionary(dict) = &array[1] {
            assert!(dict.contains_key("N"));
            assert!(dict.contains_key("Alternate"));
        } else {
            panic!("Second element should be dictionary");
        }
    }

    #[test]
    fn test_icc_profile_manager_creation() {
        let manager = IccProfileManager::new();
        assert_eq!(manager.count(), 0);
        assert!(manager.profiles().is_empty());
    }

    #[test]
    fn test_icc_profile_manager_add_profile() {
        let mut manager = IccProfileManager::new();
        let profile = IccProfile::from_standard(StandardIccProfile::SRgb);

        let name = manager.add_profile(profile).unwrap();
        assert_eq!(name, "sRGB IEC61966-2.1");
        assert_eq!(manager.count(), 1);

        let retrieved = manager.get_profile(&name).unwrap();
        assert_eq!(retrieved.name, "sRGB IEC61966-2.1");
    }

    #[test]
    fn test_icc_profile_manager_add_standard() {
        let mut manager = IccProfileManager::new();

        let name = manager
            .add_standard_profile(StandardIccProfile::SRgb)
            .unwrap();
        assert_eq!(name, "sRGB IEC61966-2.1");
        assert_eq!(manager.count(), 1);
    }

    #[test]
    fn test_icc_profile_manager_auto_naming() {
        let mut manager = IccProfileManager::new();

        let data = vec![0u8; 200];
        let profile = IccProfile::new(
            String::new(), // Empty name
            data,
            IccColorSpace::Rgb,
        );

        let name = manager.add_profile(profile).unwrap();
        assert_eq!(name, "ICC1");

        let data2 = vec![0u8; 200];
        let profile2 = IccProfile::new(String::new(), data2, IccColorSpace::Cmyk);

        let name2 = manager.add_profile(profile2).unwrap();
        assert_eq!(name2, "ICC2");
    }

    #[test]
    fn test_icc_profile_manager_get_by_type() {
        let mut manager = IccProfileManager::new();

        manager
            .add_standard_profile(StandardIccProfile::SRgb)
            .unwrap();
        manager
            .add_standard_profile(StandardIccProfile::AdobeRgb)
            .unwrap();
        manager
            .add_standard_profile(StandardIccProfile::CoatedFogra39)
            .unwrap();
        manager
            .add_standard_profile(StandardIccProfile::GrayGamma22)
            .unwrap();

        let rgb_profiles = manager.get_rgb_profiles();
        assert_eq!(rgb_profiles.len(), 2);

        let cmyk_profiles = manager.get_cmyk_profiles();
        assert_eq!(cmyk_profiles.len(), 1);

        let gray_profiles = manager.get_gray_profiles();
        assert_eq!(gray_profiles.len(), 1);
    }

    #[test]
    fn test_icc_profile_manager_defaults() {
        let mut manager = IccProfileManager::new();

        let srgb_name = manager.create_default_srgb().unwrap();
        let cmyk_name = manager.create_default_cmyk().unwrap();
        let gray_name = manager.create_default_gray().unwrap();

        assert_eq!(manager.count(), 3);
        assert!(manager.get_profile(&srgb_name).unwrap().is_rgb());
        assert!(manager.get_profile(&cmyk_name).unwrap().is_cmyk());
        assert!(manager.get_profile(&gray_name).unwrap().is_gray());
    }

    #[test]
    fn test_icc_profile_manager_clear() {
        let mut manager = IccProfileManager::new();

        manager
            .add_standard_profile(StandardIccProfile::SRgb)
            .unwrap();
        manager
            .add_standard_profile(StandardIccProfile::CoatedFogra39)
            .unwrap();
        assert_eq!(manager.count(), 2);

        manager.clear();
        assert_eq!(manager.count(), 0);
        assert!(manager.profiles().is_empty());
    }

    #[test]
    fn test_icc_profile_manager_remove() {
        let mut manager = IccProfileManager::new();

        let name = manager
            .add_standard_profile(StandardIccProfile::SRgb)
            .unwrap();
        assert_eq!(manager.count(), 1);

        let removed = manager.remove_profile(&name);
        assert!(removed.is_some());
        assert_eq!(manager.count(), 0);

        // Try to remove again
        let not_found = manager.remove_profile(&name);
        assert!(not_found.is_none());
    }
}
