//! Transparency Groups implementation for ISO 32000-1:2008 Section 11.4
//!
//! Transparency groups allow applying transparency effects to a collection
//! of objects as a single unit rather than individually.

use super::state::BlendMode;
use crate::objects::{Dictionary, Object};

/// Transparency group attributes according to ISO 32000-1:2008 Section 11.4.5
#[derive(Debug, Clone)]
pub struct TransparencyGroup {
    /// Whether the group is isolated from its backdrop
    /// When true, the group is composited against a fully transparent backdrop
    /// When false, the group inherits the backdrop from its parent
    pub isolated: bool,

    /// Whether the group is a knockout group
    /// When true, objects within the group knock out (replace) earlier objects
    /// When false, objects composite normally with each other
    pub knockout: bool,

    /// The blend mode to apply to the entire group
    pub blend_mode: BlendMode,

    /// The opacity to apply to the entire group (0.0 = transparent, 1.0 = opaque)
    pub opacity: f32,

    /// Optional color space for the group
    pub color_space: Option<String>,
}

impl Default for TransparencyGroup {
    fn default() -> Self {
        Self {
            isolated: false,
            knockout: false,
            blend_mode: BlendMode::Normal,
            opacity: 1.0,
            color_space: None,
        }
    }
}

impl TransparencyGroup {
    /// Create a new transparency group with default settings
    pub fn new() -> Self {
        Self::default()
    }

    /// Create an isolated transparency group
    pub fn isolated() -> Self {
        Self {
            isolated: true,
            ..Default::default()
        }
    }

    /// Create a knockout transparency group
    pub fn knockout() -> Self {
        Self {
            knockout: true,
            ..Default::default()
        }
    }

    /// Set whether the group is isolated
    pub fn with_isolated(mut self, isolated: bool) -> Self {
        self.isolated = isolated;
        self
    }

    /// Set whether the group is a knockout group
    pub fn with_knockout(mut self, knockout: bool) -> Self {
        self.knockout = knockout;
        self
    }

    /// Set the blend mode for the group
    pub fn with_blend_mode(mut self, blend_mode: BlendMode) -> Self {
        self.blend_mode = blend_mode;
        self
    }

    /// Set the opacity for the group
    pub fn with_opacity(mut self, opacity: f32) -> Self {
        self.opacity = opacity.clamp(0.0, 1.0);
        self
    }

    /// Set the color space for the group
    pub fn with_color_space(mut self, color_space: impl Into<String>) -> Self {
        self.color_space = Some(color_space.into());
        self
    }

    /// Convert to PDF dictionary representation
    pub fn to_dict(&self) -> Dictionary {
        let mut dict = Dictionary::new();

        // Required entries
        dict.set("Type", Object::Name("Group".into()));
        dict.set("S", Object::Name("Transparency".into()));

        // Optional entries
        if self.isolated {
            dict.set("I", Object::Boolean(true));
        }

        if self.knockout {
            dict.set("K", Object::Boolean(true));
        }

        if let Some(ref cs) = self.color_space {
            dict.set("CS", Object::Name(cs.clone()));
        }

        dict
    }
}

/// Stack entry for managing nested transparency groups
#[derive(Debug, Clone)]
pub(crate) struct TransparencyGroupState {
    /// The transparency group configuration
    pub group: TransparencyGroup,

    /// Saved graphics state before entering the group
    pub saved_state: Vec<u8>,

    /// Content stream for the group
    #[allow(dead_code)]
    pub content: Vec<u8>,
}

impl TransparencyGroupState {
    /// Create a new transparency group state
    pub fn new(group: TransparencyGroup) -> Self {
        Self {
            group,
            saved_state: Vec::new(),
            content: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transparency_group_creation() {
        let group = TransparencyGroup::new();
        assert!(!group.isolated);
        assert!(!group.knockout);
        assert_eq!(group.opacity, 1.0);
        assert!(matches!(group.blend_mode, BlendMode::Normal));
    }

    #[test]
    fn test_isolated_group() {
        let group = TransparencyGroup::isolated();
        assert!(group.isolated);
        assert!(!group.knockout);
    }

    #[test]
    fn test_knockout_group() {
        let group = TransparencyGroup::knockout();
        assert!(!group.isolated);
        assert!(group.knockout);
    }

    #[test]
    fn test_group_builder() {
        let group = TransparencyGroup::new()
            .with_isolated(true)
            .with_knockout(true)
            .with_blend_mode(BlendMode::Multiply)
            .with_opacity(0.5)
            .with_color_space("DeviceRGB");

        assert!(group.isolated);
        assert!(group.knockout);
        assert_eq!(group.opacity, 0.5);
        assert!(matches!(group.blend_mode, BlendMode::Multiply));
        assert_eq!(group.color_space, Some("DeviceRGB".to_string()));
    }

    #[test]
    fn test_opacity_clamping() {
        let group1 = TransparencyGroup::new().with_opacity(1.5);
        assert_eq!(group1.opacity, 1.0);

        let group2 = TransparencyGroup::new().with_opacity(-0.5);
        assert_eq!(group2.opacity, 0.0);
    }

    #[test]
    fn test_to_dict() {
        let group = TransparencyGroup::new()
            .with_isolated(true)
            .with_knockout(true)
            .with_color_space("DeviceCMYK");

        let dict = group.to_dict();

        // Check required entries
        assert_eq!(dict.get("Type"), Some(&Object::Name("Group".into())));
        assert_eq!(dict.get("S"), Some(&Object::Name("Transparency".into())));

        // Check optional entries
        assert_eq!(dict.get("I"), Some(&Object::Boolean(true)));
        assert_eq!(dict.get("K"), Some(&Object::Boolean(true)));
        assert_eq!(dict.get("CS"), Some(&Object::Name("DeviceCMYK".into())));
    }

    #[test]
    fn test_default_dict() {
        let group = TransparencyGroup::new();
        let dict = group.to_dict();

        // Should only have required entries for default group
        assert_eq!(dict.get("Type"), Some(&Object::Name("Group".into())));
        assert_eq!(dict.get("S"), Some(&Object::Name("Transparency".into())));
        assert!(dict.get("I").is_none());
        assert!(dict.get("K").is_none());
        assert!(dict.get("CS").is_none());
    }
}
