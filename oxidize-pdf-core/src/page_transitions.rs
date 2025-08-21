//! Page transitions for presentations in PDF documents
//!
//! Page transitions control the visual effect when transitioning between pages
//! in presentation mode (full screen). These are defined in ISO 32000-1:2008.

use crate::objects::{Dictionary, Object};

/// Page transition styles defined in ISO 32000-1
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TransitionStyle {
    /// Split transition
    Split,
    /// Blinds transition  
    Blinds,
    /// Box transition
    Box,
    /// Wipe transition
    Wipe,
    /// Dissolve transition
    Dissolve,
    /// Glitter transition
    Glitter,
    /// Replace transition (default)
    Replace,
    /// Fly transition  
    Fly,
    /// Push transition
    Push,
    /// Cover transition
    Cover,
    /// Uncover transition
    Uncover,
    /// Fade transition
    Fade,
}

impl TransitionStyle {
    /// Convert to PDF name
    pub fn to_pdf_name(&self) -> &'static str {
        match self {
            TransitionStyle::Split => "Split",
            TransitionStyle::Blinds => "Blinds",
            TransitionStyle::Box => "Box",
            TransitionStyle::Wipe => "Wipe",
            TransitionStyle::Dissolve => "Dissolve",
            TransitionStyle::Glitter => "Glitter",
            TransitionStyle::Replace => "Replace",
            TransitionStyle::Fly => "Fly",
            TransitionStyle::Push => "Push",
            TransitionStyle::Cover => "Cover",
            TransitionStyle::Uncover => "Uncover",
            TransitionStyle::Fade => "Fade",
        }
    }
}

/// Transition dimension for applicable styles
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TransitionDimension {
    /// Horizontal direction
    Horizontal,
    /// Vertical direction
    Vertical,
}

impl TransitionDimension {
    /// Convert to PDF name
    pub fn to_pdf_name(&self) -> &'static str {
        match self {
            TransitionDimension::Horizontal => "H",
            TransitionDimension::Vertical => "V",
        }
    }
}

/// Motion direction for applicable transition styles
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TransitionMotion {
    /// Inward motion
    Inward,
    /// Outward motion
    Outward,
}

impl TransitionMotion {
    /// Convert to PDF name
    pub fn to_pdf_name(&self) -> &'static str {
        match self {
            TransitionMotion::Inward => "I",
            TransitionMotion::Outward => "O",
        }
    }
}

/// Direction angle for glitter and fly transitions
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TransitionDirection {
    /// Left to right (0 degrees)
    LeftToRight,
    /// Bottom to top (90 degrees)  
    BottomToTop,
    /// Right to left (180 degrees)
    RightToLeft,
    /// Top to bottom (270 degrees)
    TopToBottom,
    /// Top-left to bottom-right (315 degrees)
    TopLeftToBottomRight,
    /// Custom angle in degrees (0-360)
    Custom(u16),
}

impl TransitionDirection {
    /// Convert to PDF angle value
    pub fn to_pdf_angle(&self) -> u16 {
        match self {
            TransitionDirection::LeftToRight => 0,
            TransitionDirection::BottomToTop => 90,
            TransitionDirection::RightToLeft => 180,
            TransitionDirection::TopToBottom => 270,
            TransitionDirection::TopLeftToBottomRight => 315,
            TransitionDirection::Custom(angle) => *angle % 360,
        }
    }
}

/// Page transition definition
#[derive(Debug, Clone)]
pub struct PageTransition {
    /// Transition style
    pub style: TransitionStyle,
    /// Duration in seconds
    pub duration: Option<f32>,
    /// Dimension (for applicable styles)
    pub dimension: Option<TransitionDimension>,
    /// Motion direction (for applicable styles)
    pub motion: Option<TransitionMotion>,
    /// Direction angle (for glitter and fly)
    pub direction: Option<TransitionDirection>,
    /// Scale factor (for fly transitions)
    pub scale: Option<f32>,
    /// Rectangular area (for fly transitions)
    pub area: Option<[f32; 4]>, // [x, y, width, height]
}

impl PageTransition {
    /// Create a new page transition with the specified style
    pub fn new(style: TransitionStyle) -> Self {
        PageTransition {
            style,
            duration: None,
            dimension: None,
            motion: None,
            direction: None,
            scale: None,
            area: None,
        }
    }

    /// Set transition duration in seconds
    pub fn with_duration(mut self, duration: f32) -> Self {
        self.duration = Some(duration.max(0.0));
        self
    }

    /// Set transition dimension (for Split, Blinds styles)
    pub fn with_dimension(mut self, dimension: TransitionDimension) -> Self {
        self.dimension = Some(dimension);
        self
    }

    /// Set transition motion (for Split, Box styles)
    pub fn with_motion(mut self, motion: TransitionMotion) -> Self {
        self.motion = Some(motion);
        self
    }

    /// Set transition direction (for Wipe, Glitter, Fly styles)
    pub fn with_direction(mut self, direction: TransitionDirection) -> Self {
        self.direction = Some(direction);
        self
    }

    /// Set scale factor (for Fly style)
    pub fn with_scale(mut self, scale: f32) -> Self {
        self.scale = Some(scale.clamp(0.01, 100.0));
        self
    }

    /// Set rectangular area (for Fly style)
    pub fn with_area(mut self, x: f32, y: f32, width: f32, height: f32) -> Self {
        self.area = Some([x, y, width, height]);
        self
    }

    /// Convert to PDF dictionary
    pub fn to_dict(&self) -> Dictionary {
        let mut dict = Dictionary::new();
        dict.set("Type", Object::Name("Trans".to_string()));
        dict.set("S", Object::Name(self.style.to_pdf_name().to_string()));

        if let Some(duration) = self.duration {
            dict.set("D", Object::Real(duration as f64));
        }

        if let Some(dimension) = self.dimension {
            dict.set("Dm", Object::Name(dimension.to_pdf_name().to_string()));
        }

        if let Some(motion) = self.motion {
            dict.set("M", Object::Name(motion.to_pdf_name().to_string()));
        }

        if let Some(direction) = self.direction {
            dict.set("Di", Object::Integer(direction.to_pdf_angle() as i64));
        }

        if let Some(scale) = self.scale {
            dict.set("SS", Object::Real(scale as f64));
        }

        if let Some(area) = self.area {
            let area_array = vec![
                Object::Real(area[0] as f64),
                Object::Real(area[1] as f64),
                Object::Real(area[2] as f64),
                Object::Real(area[3] as f64),
            ];
            dict.set("B", Object::Array(area_array));
        }

        dict
    }

    // Convenience constructors for common transitions

    /// Split transition (horizontal or vertical)
    pub fn split(dimension: TransitionDimension, motion: TransitionMotion) -> Self {
        PageTransition::new(TransitionStyle::Split)
            .with_dimension(dimension)
            .with_motion(motion)
    }

    /// Blinds transition (horizontal or vertical)
    pub fn blinds(dimension: TransitionDimension) -> Self {
        PageTransition::new(TransitionStyle::Blinds).with_dimension(dimension)
    }

    /// Box transition (inward or outward)
    pub fn box_transition(motion: TransitionMotion) -> Self {
        PageTransition::new(TransitionStyle::Box).with_motion(motion)
    }

    /// Wipe transition with direction
    pub fn wipe(direction: TransitionDirection) -> Self {
        PageTransition::new(TransitionStyle::Wipe).with_direction(direction)
    }

    /// Dissolve transition
    pub fn dissolve() -> Self {
        PageTransition::new(TransitionStyle::Dissolve)
    }

    /// Glitter transition with direction
    pub fn glitter(direction: TransitionDirection) -> Self {
        PageTransition::new(TransitionStyle::Glitter).with_direction(direction)
    }

    /// Replace transition (no effect)
    pub fn replace() -> Self {
        PageTransition::new(TransitionStyle::Replace)
    }

    /// Fly transition with direction and optional scale
    pub fn fly(direction: TransitionDirection) -> Self {
        PageTransition::new(TransitionStyle::Fly).with_direction(direction)
    }

    /// Push transition with direction
    pub fn push(direction: TransitionDirection) -> Self {
        PageTransition::new(TransitionStyle::Push).with_direction(direction)
    }

    /// Cover transition with direction
    pub fn cover(direction: TransitionDirection) -> Self {
        PageTransition::new(TransitionStyle::Cover).with_direction(direction)
    }

    /// Uncover transition with direction
    pub fn uncover(direction: TransitionDirection) -> Self {
        PageTransition::new(TransitionStyle::Uncover).with_direction(direction)
    }

    /// Fade transition
    pub fn fade() -> Self {
        PageTransition::new(TransitionStyle::Fade)
    }
}

impl Default for PageTransition {
    fn default() -> Self {
        PageTransition::replace()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transition_style_names() {
        assert_eq!(TransitionStyle::Split.to_pdf_name(), "Split");
        assert_eq!(TransitionStyle::Blinds.to_pdf_name(), "Blinds");
        assert_eq!(TransitionStyle::Box.to_pdf_name(), "Box");
        assert_eq!(TransitionStyle::Wipe.to_pdf_name(), "Wipe");
        assert_eq!(TransitionStyle::Dissolve.to_pdf_name(), "Dissolve");
        assert_eq!(TransitionStyle::Glitter.to_pdf_name(), "Glitter");
        assert_eq!(TransitionStyle::Replace.to_pdf_name(), "Replace");
        assert_eq!(TransitionStyle::Fly.to_pdf_name(), "Fly");
        assert_eq!(TransitionStyle::Push.to_pdf_name(), "Push");
        assert_eq!(TransitionStyle::Cover.to_pdf_name(), "Cover");
        assert_eq!(TransitionStyle::Uncover.to_pdf_name(), "Uncover");
        assert_eq!(TransitionStyle::Fade.to_pdf_name(), "Fade");
    }

    #[test]
    fn test_transition_dimension_names() {
        assert_eq!(TransitionDimension::Horizontal.to_pdf_name(), "H");
        assert_eq!(TransitionDimension::Vertical.to_pdf_name(), "V");
    }

    #[test]
    fn test_transition_motion_names() {
        assert_eq!(TransitionMotion::Inward.to_pdf_name(), "I");
        assert_eq!(TransitionMotion::Outward.to_pdf_name(), "O");
    }

    #[test]
    fn test_transition_direction_angles() {
        assert_eq!(TransitionDirection::LeftToRight.to_pdf_angle(), 0);
        assert_eq!(TransitionDirection::BottomToTop.to_pdf_angle(), 90);
        assert_eq!(TransitionDirection::RightToLeft.to_pdf_angle(), 180);
        assert_eq!(TransitionDirection::TopToBottom.to_pdf_angle(), 270);
        assert_eq!(
            TransitionDirection::TopLeftToBottomRight.to_pdf_angle(),
            315
        );
        assert_eq!(TransitionDirection::Custom(45).to_pdf_angle(), 45);
        assert_eq!(TransitionDirection::Custom(450).to_pdf_angle(), 90); // 450 % 360 = 90
    }

    #[test]
    fn test_basic_transition() {
        let transition = PageTransition::new(TransitionStyle::Dissolve);
        let dict = transition.to_dict();

        assert_eq!(dict.get("Type"), Some(&Object::Name("Trans".to_string())));
        assert_eq!(dict.get("S"), Some(&Object::Name("Dissolve".to_string())));
    }

    #[test]
    fn test_transition_with_duration() {
        let transition = PageTransition::dissolve().with_duration(2.5);
        let dict = transition.to_dict();

        assert_eq!(dict.get("D"), Some(&Object::Real(2.5)));
    }

    #[test]
    fn test_split_transition() {
        let transition =
            PageTransition::split(TransitionDimension::Horizontal, TransitionMotion::Inward);
        let dict = transition.to_dict();

        assert_eq!(dict.get("S"), Some(&Object::Name("Split".to_string())));
        assert_eq!(dict.get("Dm"), Some(&Object::Name("H".to_string())));
        assert_eq!(dict.get("M"), Some(&Object::Name("I".to_string())));
    }

    #[test]
    fn test_wipe_transition_with_direction() {
        let transition = PageTransition::wipe(TransitionDirection::LeftToRight);
        let dict = transition.to_dict();

        assert_eq!(dict.get("S"), Some(&Object::Name("Wipe".to_string())));
        assert_eq!(dict.get("Di"), Some(&Object::Integer(0)));
    }

    #[test]
    fn test_fly_transition_with_scale() {
        let transition = PageTransition::fly(TransitionDirection::BottomToTop)
            .with_scale(1.5)
            .with_area(100.0, 100.0, 200.0, 200.0);
        let dict = transition.to_dict();

        assert_eq!(dict.get("S"), Some(&Object::Name("Fly".to_string())));
        assert_eq!(dict.get("Di"), Some(&Object::Integer(90)));
        assert_eq!(dict.get("SS"), Some(&Object::Real(1.5)));

        if let Some(Object::Array(area)) = dict.get("B") {
            assert_eq!(area.len(), 4);
        } else {
            panic!("Expected area array");
        }
    }

    #[test]
    fn test_convenience_constructors() {
        // Test all convenience constructors
        assert!(matches!(
            PageTransition::split(TransitionDimension::Horizontal, TransitionMotion::Inward).style,
            TransitionStyle::Split
        ));
        assert!(matches!(
            PageTransition::blinds(TransitionDimension::Vertical).style,
            TransitionStyle::Blinds
        ));
        assert!(matches!(
            PageTransition::box_transition(TransitionMotion::Outward).style,
            TransitionStyle::Box
        ));
        assert!(matches!(
            PageTransition::wipe(TransitionDirection::LeftToRight).style,
            TransitionStyle::Wipe
        ));
        assert!(matches!(
            PageTransition::dissolve().style,
            TransitionStyle::Dissolve
        ));
        assert!(matches!(
            PageTransition::glitter(TransitionDirection::TopToBottom).style,
            TransitionStyle::Glitter
        ));
        assert!(matches!(
            PageTransition::replace().style,
            TransitionStyle::Replace
        ));
        assert!(matches!(
            PageTransition::fly(TransitionDirection::RightToLeft).style,
            TransitionStyle::Fly
        ));
        assert!(matches!(
            PageTransition::push(TransitionDirection::BottomToTop).style,
            TransitionStyle::Push
        ));
        assert!(matches!(
            PageTransition::cover(TransitionDirection::TopToBottom).style,
            TransitionStyle::Cover
        ));
        assert!(matches!(
            PageTransition::uncover(TransitionDirection::LeftToRight).style,
            TransitionStyle::Uncover
        ));
        assert!(matches!(
            PageTransition::fade().style,
            TransitionStyle::Fade
        ));
    }

    #[test]
    fn test_default_transition() {
        let transition = PageTransition::default();
        assert!(matches!(transition.style, TransitionStyle::Replace));
    }

    #[test]
    fn test_duration_bounds() {
        let transition = PageTransition::dissolve().with_duration(-1.0);
        assert_eq!(transition.duration, Some(0.0));
    }

    #[test]
    fn test_scale_bounds() {
        let transition = PageTransition::fly(TransitionDirection::LeftToRight).with_scale(0.001); // Too small
        assert_eq!(transition.scale, Some(0.01));

        let transition = PageTransition::fly(TransitionDirection::LeftToRight).with_scale(200.0); // Too large
        assert_eq!(transition.scale, Some(100.0));
    }
}
