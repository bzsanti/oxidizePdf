//! Soft Mask support for PDF graphics according to ISO 32000-1 Section 11.6
//!
//! Soft masks allow specifying transparency using luminosity or alpha values
//! from another graphics object, enabling complex transparency effects.

use crate::error::Result;
use crate::objects::{Dictionary, Object};
use std::fmt;

/// Soft mask type according to ISO 32000-1
#[derive(Debug, Clone, PartialEq)]
pub enum SoftMaskType {
    /// Alpha channel soft mask
    Alpha,
    /// Luminosity soft mask
    Luminosity,
}

impl SoftMaskType {
    /// Get PDF name for this soft mask type
    pub fn pdf_name(&self) -> &'static str {
        match self {
            SoftMaskType::Alpha => "Alpha",
            SoftMaskType::Luminosity => "Luminosity",
        }
    }
}

/// Transfer function for soft mask
#[derive(Debug, Clone)]
pub enum TransferFunction {
    /// Identity function (no transformation)
    Identity,
    /// Custom function name
    Custom(String),
    /// Function array for complex transformations
    FunctionArray(Vec<f64>),
}

impl TransferFunction {
    /// Convert to PDF representation
    pub fn to_pdf_object(&self) -> Object {
        match self {
            TransferFunction::Identity => Object::Name("Identity".to_string()),
            TransferFunction::Custom(name) => Object::Name(name.clone()),
            TransferFunction::FunctionArray(values) => {
                Object::Array(values.iter().map(|&v| Object::Real(v)).collect())
            }
        }
    }
}

/// Soft mask configuration
#[derive(Debug, Clone)]
pub struct SoftMask {
    /// Type of soft mask (Alpha or Luminosity)
    pub mask_type: SoftMaskType,

    /// Reference to the transparency group XObject
    pub group_ref: Option<String>,

    /// Background color to use with the soft mask
    pub background_color: Option<Vec<f64>>,

    /// Transfer function for the soft mask
    pub transfer_function: Option<TransferFunction>,

    /// Bounding box for the soft mask effect
    pub bbox: Option<[f64; 4]>,
}

impl Default for SoftMask {
    fn default() -> Self {
        Self::none()
    }
}

impl SoftMask {
    /// Create a soft mask that disables masking (None)
    pub fn none() -> Self {
        Self {
            mask_type: SoftMaskType::Alpha,
            group_ref: None,
            background_color: None,
            transfer_function: None,
            bbox: None,
        }
    }

    /// Create an alpha soft mask
    pub fn alpha(group_ref: String) -> Self {
        Self {
            mask_type: SoftMaskType::Alpha,
            group_ref: Some(group_ref),
            background_color: None,
            transfer_function: None,
            bbox: None,
        }
    }

    /// Create a luminosity soft mask
    pub fn luminosity(group_ref: String) -> Self {
        Self {
            mask_type: SoftMaskType::Luminosity,
            group_ref: Some(group_ref),
            background_color: None,
            transfer_function: None,
            bbox: None,
        }
    }

    /// Set background color for the soft mask
    pub fn with_background_color(mut self, color: Vec<f64>) -> Self {
        self.background_color = Some(color);
        self
    }

    /// Set transfer function for the soft mask
    pub fn with_transfer_function(mut self, func: TransferFunction) -> Self {
        self.transfer_function = Some(func);
        self
    }

    /// Set bounding box for the soft mask
    pub fn with_bbox(mut self, bbox: [f64; 4]) -> Self {
        self.bbox = Some(bbox);
        self
    }

    /// Check if this is a "None" soft mask (disables masking)
    pub fn is_none(&self) -> bool {
        self.group_ref.is_none()
    }

    /// Convert to PDF dictionary representation
    pub fn to_pdf_dictionary(&self) -> Result<Dictionary> {
        // If no group reference, return "None" soft mask
        if self.group_ref.is_none() {
            let mut dict = Dictionary::new();
            dict.set("Type", Object::Name("Mask".to_string()));
            dict.set("S", Object::Name("None".to_string()));
            return Ok(dict);
        }

        let mut dict = Dictionary::new();
        dict.set("Type", Object::Name("Mask".to_string()));
        dict.set("S", Object::Name(self.mask_type.pdf_name().to_string()));

        // Reference to transparency group (would be an indirect reference in real PDF)
        if let Some(ref group_ref) = self.group_ref {
            dict.set("G", Object::Name(group_ref.clone()));
        }

        // Background color array
        if let Some(ref bc) = self.background_color {
            let color_array: Vec<Object> = bc.iter().map(|&c| Object::Real(c)).collect();
            dict.set("BC", Object::Array(color_array));
        }

        // Transfer function
        if let Some(ref tr) = self.transfer_function {
            dict.set("TR", tr.to_pdf_object());
        }

        // Bounding box
        if let Some(bbox) = self.bbox {
            let bbox_array = vec![
                Object::Real(bbox[0]),
                Object::Real(bbox[1]),
                Object::Real(bbox[2]),
                Object::Real(bbox[3]),
            ];
            dict.set("BBox", Object::Array(bbox_array));
        }

        Ok(dict)
    }

    /// Convert to PDF string representation for inline use
    pub fn to_pdf_string(&self) -> String {
        if self.is_none() {
            "/None".to_string()
        } else {
            // In real implementation, this would reference an indirect object
            format!("/SM{}", self.group_ref.as_ref().unwrap_or(&"1".to_string()))
        }
    }
}

impl fmt::Display for SoftMask {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_none() {
            write!(f, "SoftMask::None")
        } else {
            write!(f, "SoftMask::{:?}", self.mask_type)
        }
    }
}

/// Soft mask state for graphics context
#[derive(Debug, Clone)]
pub struct SoftMaskState {
    /// Current soft mask
    pub mask: SoftMask,

    /// Stack of saved soft masks for nested operations
    pub saved_masks: Vec<SoftMask>,
}

impl Default for SoftMaskState {
    fn default() -> Self {
        Self::new()
    }
}

impl SoftMaskState {
    /// Create new soft mask state
    pub fn new() -> Self {
        Self {
            mask: SoftMask::none(),
            saved_masks: Vec::new(),
        }
    }

    /// Set current soft mask
    pub fn set_mask(&mut self, mask: SoftMask) {
        self.mask = mask;
    }

    /// Push current mask to stack and set new mask
    pub fn push_mask(&mut self, mask: SoftMask) {
        self.saved_masks.push(self.mask.clone());
        self.mask = mask;
    }

    /// Pop mask from stack and restore it
    pub fn pop_mask(&mut self) -> Option<SoftMask> {
        if let Some(mask) = self.saved_masks.pop() {
            let current = self.mask.clone();
            self.mask = mask;
            Some(current)
        } else {
            None
        }
    }

    /// Clear all masks (reset to None)
    pub fn clear(&mut self) {
        self.mask = SoftMask::none();
        self.saved_masks.clear();
    }

    /// Check if any soft mask is active
    pub fn is_active(&self) -> bool {
        !self.mask.is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_soft_mask_none() {
        let mask = SoftMask::none();
        assert!(mask.is_none());
        assert_eq!(mask.to_pdf_string(), "/None");
    }

    #[test]
    fn test_soft_mask_alpha() {
        let mask = SoftMask::alpha("Group1".to_string());
        assert!(!mask.is_none());
        assert_eq!(mask.mask_type, SoftMaskType::Alpha);
        assert_eq!(mask.group_ref, Some("Group1".to_string()));
    }

    #[test]
    fn test_soft_mask_luminosity() {
        let mask = SoftMask::luminosity("Group2".to_string());
        assert!(!mask.is_none());
        assert_eq!(mask.mask_type, SoftMaskType::Luminosity);
        assert_eq!(mask.group_ref, Some("Group2".to_string()));
    }

    #[test]
    fn test_soft_mask_with_background() {
        let mask = SoftMask::alpha("Group1".to_string()).with_background_color(vec![1.0, 1.0, 1.0]);

        assert_eq!(mask.background_color, Some(vec![1.0, 1.0, 1.0]));
    }

    #[test]
    fn test_soft_mask_with_transfer_function() {
        let mask = SoftMask::alpha("Group1".to_string())
            .with_transfer_function(TransferFunction::Identity);

        assert!(mask.transfer_function.is_some());
    }

    #[test]
    fn test_soft_mask_with_bbox() {
        let mask = SoftMask::alpha("Group1".to_string()).with_bbox([0.0, 0.0, 100.0, 100.0]);

        assert_eq!(mask.bbox, Some([0.0, 0.0, 100.0, 100.0]));
    }

    #[test]
    fn test_soft_mask_to_dictionary() {
        let mask = SoftMask::luminosity("Group1".to_string())
            .with_background_color(vec![0.5, 0.5, 0.5])
            .with_transfer_function(TransferFunction::Identity)
            .with_bbox([0.0, 0.0, 200.0, 200.0]);

        let dict = mask.to_pdf_dictionary().unwrap();

        assert_eq!(dict.get("Type"), Some(&Object::Name("Mask".to_string())));
        assert_eq!(dict.get("S"), Some(&Object::Name("Luminosity".to_string())));
        assert!(dict.contains_key("G"));
        assert!(dict.contains_key("BC"));
        assert!(dict.contains_key("TR"));
        assert!(dict.contains_key("BBox"));
    }

    #[test]
    fn test_soft_mask_none_dictionary() {
        let mask = SoftMask::none();
        let dict = mask.to_pdf_dictionary().unwrap();

        assert_eq!(dict.get("Type"), Some(&Object::Name("Mask".to_string())));
        assert_eq!(dict.get("S"), Some(&Object::Name("None".to_string())));
    }

    #[test]
    fn test_soft_mask_state() {
        let mut state = SoftMaskState::new();
        assert!(state.mask.is_none());

        let mask1 = SoftMask::alpha("Group1".to_string());
        state.set_mask(mask1.clone());
        assert!(!state.mask.is_none());

        let mask2 = SoftMask::luminosity("Group2".to_string());
        state.push_mask(mask2);
        assert_eq!(state.saved_masks.len(), 1);

        let popped = state.pop_mask();
        assert!(popped.is_some());
        assert_eq!(state.mask.group_ref, Some("Group1".to_string()));
    }

    #[test]
    fn test_transfer_function_to_pdf() {
        let identity = TransferFunction::Identity;
        assert_eq!(
            identity.to_pdf_object(),
            Object::Name("Identity".to_string())
        );

        let custom = TransferFunction::Custom("Custom1".to_string());
        assert_eq!(custom.to_pdf_object(), Object::Name("Custom1".to_string()));

        let array = TransferFunction::FunctionArray(vec![0.0, 0.5, 1.0]);
        if let Object::Array(arr) = array.to_pdf_object() {
            assert_eq!(arr.len(), 3);
        } else {
            panic!("Expected array");
        }
    }
}
