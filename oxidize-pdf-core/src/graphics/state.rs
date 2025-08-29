//! Extended Graphics State Dictionary support according to ISO 32000-1 Section 8.4
//!
//! This module provides comprehensive support for PDF Extended Graphics State (ExtGState)
//! dictionary parameters as specified in ISO 32000-1:2008.

use super::soft_mask::SoftMask;
use crate::error::{PdfError, Result};
use crate::graphics::{LineCap, LineJoin};
use crate::text::Font;
use std::collections::HashMap;
use std::fmt::Write;

/// Rendering intent values according to ISO 32000-1
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RenderingIntent {
    /// Absolute colorimetric
    AbsoluteColorimetric,
    /// Relative colorimetric
    RelativeColorimetric,
    /// Saturation
    Saturation,
    /// Perceptual
    Perceptual,
}

impl RenderingIntent {
    /// Get the PDF name for this rendering intent
    pub fn pdf_name(&self) -> &'static str {
        match self {
            RenderingIntent::AbsoluteColorimetric => "AbsoluteColorimetric",
            RenderingIntent::RelativeColorimetric => "RelativeColorimetric",
            RenderingIntent::Saturation => "Saturation",
            RenderingIntent::Perceptual => "Perceptual",
        }
    }
}

/// Blend mode values for transparency
#[derive(Debug, Clone, PartialEq)]
pub enum BlendMode {
    /// Normal blend mode (default)
    Normal,
    /// Multiply blend mode
    Multiply,
    /// Screen blend mode
    Screen,
    /// Overlay blend mode
    Overlay,
    /// SoftLight blend mode
    SoftLight,
    /// HardLight blend mode
    HardLight,
    /// ColorDodge blend mode
    ColorDodge,
    /// ColorBurn blend mode
    ColorBurn,
    /// Darken blend mode
    Darken,
    /// Lighten blend mode
    Lighten,
    /// Difference blend mode
    Difference,
    /// Exclusion blend mode
    Exclusion,
    /// Hue blend mode (PDF 1.4)
    Hue,
    /// Saturation blend mode (PDF 1.4)
    Saturation,
    /// Color blend mode (PDF 1.4)
    Color,
    /// Luminosity blend mode (PDF 1.4)
    Luminosity,
}

impl BlendMode {
    /// Get the PDF name for this blend mode
    pub fn pdf_name(&self) -> &'static str {
        match self {
            BlendMode::Normal => "Normal",
            BlendMode::Multiply => "Multiply",
            BlendMode::Screen => "Screen",
            BlendMode::Overlay => "Overlay",
            BlendMode::SoftLight => "SoftLight",
            BlendMode::HardLight => "HardLight",
            BlendMode::ColorDodge => "ColorDodge",
            BlendMode::ColorBurn => "ColorBurn",
            BlendMode::Darken => "Darken",
            BlendMode::Lighten => "Lighten",
            BlendMode::Difference => "Difference",
            BlendMode::Exclusion => "Exclusion",
            BlendMode::Hue => "Hue",
            BlendMode::Saturation => "Saturation",
            BlendMode::Color => "Color",
            BlendMode::Luminosity => "Luminosity",
        }
    }
}

/// Line dash pattern specification
#[derive(Debug, Clone, PartialEq)]
pub struct LineDashPattern {
    /// Array of dash and gap lengths
    pub array: Vec<f64>,
    /// Phase offset
    pub phase: f64,
}

impl LineDashPattern {
    /// Create a new line dash pattern
    pub fn new(array: Vec<f64>, phase: f64) -> Self {
        Self { array, phase }
    }

    /// Create a solid line (no dashes)
    pub fn solid() -> Self {
        Self {
            array: Vec::new(),
            phase: 0.0,
        }
    }

    /// Create a simple dashed line
    pub fn dashed(dash_length: f64, gap_length: f64) -> Self {
        Self {
            array: vec![dash_length, gap_length],
            phase: 0.0,
        }
    }

    /// Create a dotted line
    pub fn dotted(dot_size: f64, gap_size: f64) -> Self {
        Self {
            array: vec![dot_size, gap_size],
            phase: 0.0,
        }
    }

    /// Generate PDF representation of the line dash pattern
    pub fn to_pdf_string(&self) -> String {
        if self.array.is_empty() {
            "[] 0".to_string()
        } else {
            let array_str = self
                .array
                .iter()
                .map(|&x| format!("{x:.2}"))
                .collect::<Vec<_>>()
                .join(" ");
            format!("[{array_str}] {:.2}", self.phase)
        }
    }
}

/// Font specification for ExtGState
#[derive(Debug, Clone, PartialEq)]
pub struct ExtGStateFont {
    /// Font
    pub font: Font,
    /// Font size
    pub size: f64,
}

impl ExtGStateFont {
    /// Create a new ExtGState font specification
    pub fn new(font: Font, size: f64) -> Self {
        Self { font, size }
    }
}

/// Transfer function specification according to ISO 32000-1
#[derive(Debug, Clone, PartialEq)]
#[allow(clippy::large_enum_variant)]
pub enum TransferFunction {
    /// Identity transfer function (no transformation)
    Identity,
    /// Single transfer function for all components
    Single(TransferFunctionData),
    /// Separate transfer functions for each color component (C, M, Y, K or R, G, B)
    Separate {
        /// Function for first component (Cyan or Red)
        c_or_r: TransferFunctionData,
        /// Function for second component (Magenta or Green)
        m_or_g: TransferFunctionData,
        /// Function for third component (Yellow or Blue)
        y_or_b: TransferFunctionData,
        /// Function for fourth component (Black) - optional for RGB
        k: Option<TransferFunctionData>,
    },
}

/// Data for a single transfer function
#[derive(Debug, Clone, PartialEq)]
pub struct TransferFunctionData {
    /// Function type (0, 2, 3, or 4)
    pub function_type: u32,
    /// Domain of the function
    pub domain: Vec<f64>,
    /// Range of the function
    pub range: Vec<f64>,
    /// Function-specific parameters
    pub params: TransferFunctionParams,
}

/// Parameters for different transfer function types
#[derive(Debug, Clone, PartialEq)]
pub enum TransferFunctionParams {
    /// Type 0: Sampled function
    Sampled {
        /// Sample values
        samples: Vec<f64>,
        /// Number of samples in each dimension
        size: Vec<u32>,
        /// Bits per sample
        bits_per_sample: u32,
    },
    /// Type 2: Exponential interpolation
    Exponential {
        /// C0 values
        c0: Vec<f64>,
        /// C1 values
        c1: Vec<f64>,
        /// Exponent
        n: f64,
    },
    /// Type 3: Stitching function
    Stitching {
        /// Functions to stitch together
        functions: Vec<TransferFunctionData>,
        /// Bounds for stitching
        bounds: Vec<f64>,
        /// Encode values
        encode: Vec<f64>,
    },
    /// Type 4: PostScript calculator function
    PostScript {
        /// PostScript code
        code: String,
    },
}

impl TransferFunction {
    /// Create an identity transfer function
    pub fn identity() -> Self {
        TransferFunction::Identity
    }

    /// Create a gamma correction transfer function
    pub fn gamma(gamma_value: f64) -> Self {
        TransferFunction::Single(TransferFunctionData {
            function_type: 2,
            domain: vec![0.0, 1.0],
            range: vec![0.0, 1.0],
            params: TransferFunctionParams::Exponential {
                c0: vec![0.0],
                c1: vec![1.0],
                n: gamma_value,
            },
        })
    }

    /// Create a linear transfer function with slope and intercept
    pub fn linear(slope: f64, intercept: f64) -> Self {
        TransferFunction::Single(TransferFunctionData {
            function_type: 2,
            domain: vec![0.0, 1.0],
            range: vec![0.0, 1.0],
            params: TransferFunctionParams::Exponential {
                c0: vec![intercept],
                c1: vec![slope + intercept],
                n: 1.0,
            },
        })
    }

    /// Convert transfer function to PDF representation
    pub fn to_pdf_string(&self) -> String {
        match self {
            TransferFunction::Identity => "/Identity".to_string(),
            TransferFunction::Single(data) => data.to_pdf_string(),
            TransferFunction::Separate {
                c_or_r,
                m_or_g,
                y_or_b,
                k,
            } => {
                let mut result = String::from("[");
                result.push_str(&c_or_r.to_pdf_string());
                result.push(' ');
                result.push_str(&m_or_g.to_pdf_string());
                result.push(' ');
                result.push_str(&y_or_b.to_pdf_string());
                if let Some(k_func) = k {
                    result.push(' ');
                    result.push_str(&k_func.to_pdf_string());
                }
                result.push(']');
                result
            }
        }
    }
}

impl TransferFunctionData {
    /// Convert transfer function data to PDF representation
    pub fn to_pdf_string(&self) -> String {
        let mut dict = String::from("<<");

        // Function type
        dict.push_str(&format!(" /FunctionType {}", self.function_type));

        // Domain
        dict.push_str(" /Domain [");
        for (i, val) in self.domain.iter().enumerate() {
            if i > 0 {
                dict.push(' ');
            }
            dict.push_str(&format!("{:.3}", val));
        }
        dict.push(']');

        // Range
        dict.push_str(" /Range [");
        for (i, val) in self.range.iter().enumerate() {
            if i > 0 {
                dict.push(' ');
            }
            dict.push_str(&format!("{:.3}", val));
        }
        dict.push(']');

        // Function-specific parameters
        match &self.params {
            TransferFunctionParams::Exponential { c0, c1, n } => {
                // Type 2: Exponential interpolation function
                dict.push_str(" /C0 [");
                for (i, val) in c0.iter().enumerate() {
                    if i > 0 {
                        dict.push(' ');
                    }
                    dict.push_str(&format!("{:.3}", val));
                }
                dict.push_str("] /C1 [");
                for (i, val) in c1.iter().enumerate() {
                    if i > 0 {
                        dict.push(' ');
                    }
                    dict.push_str(&format!("{:.3}", val));
                }
                dict.push_str(&format!("] /N {:.3}", n));
            }
            TransferFunctionParams::Sampled {
                size,
                bits_per_sample,
                samples,
                ..
            } => {
                // Type 0: Sampled function
                dict.push_str(" /Size [");
                for (i, val) in size.iter().enumerate() {
                    if i > 0 {
                        dict.push(' ');
                    }
                    dict.push_str(&format!("{}", val));
                }
                dict.push_str(&format!("] /BitsPerSample {}", bits_per_sample));
                // Samples would be encoded as a stream
                dict.push_str(" /Length ");
                dict.push_str(&format!("{}", samples.len()));
            }
            TransferFunctionParams::Stitching {
                bounds,
                encode,
                functions,
            } => {
                // Type 3: Stitching function
                dict.push_str(" /Bounds [");
                for (i, val) in bounds.iter().enumerate() {
                    if i > 0 {
                        dict.push(' ');
                    }
                    dict.push_str(&format!("{:.3}", val));
                }
                dict.push_str("] /Encode [");
                for (i, val) in encode.iter().enumerate() {
                    if i > 0 {
                        dict.push(' ');
                    }
                    dict.push_str(&format!("{:.3}", val));
                }
                dict.push_str("] /Functions [");
                for (i, func) in functions.iter().enumerate() {
                    if i > 0 {
                        dict.push(' ');
                    }
                    dict.push_str(&func.to_pdf_string());
                }
                dict.push(']');
            }
            TransferFunctionParams::PostScript { code } => {
                // Type 4: PostScript calculator function
                dict.push_str(&format!(
                    " /Length {} stream\n{}\nendstream",
                    code.len(),
                    code
                ));
            }
        }

        dict.push_str(" >>");
        dict
    }
}

/// Halftone specification according to ISO 32000-1
#[derive(Debug, Clone, PartialEq)]
pub enum Halftone {
    /// Default halftone
    Default,
    /// Type 1: Simple halftone
    Type1 {
        /// Halftone frequency
        frequency: f64,
        /// Halftone angle in degrees
        angle: f64,
        /// Spot function name
        spot_function: SpotFunction,
    },
    /// Type 5: Halftone with multiple colorants
    Type5 {
        /// Halftone for each colorant
        colorants: HashMap<String, HalftoneColorant>,
        /// Default halftone
        default: Box<Halftone>,
    },
    /// Type 6: Threshold array
    Type6 {
        /// Width of threshold array
        width: u32,
        /// Height of threshold array
        height: u32,
        /// Threshold values
        thresholds: Vec<u8>,
    },
    /// Type 10: Stochastic (FM) screening
    Type10 {
        /// Halftone frequency
        frequency: f64,
    },
    /// Type 16: Multiple threshold arrays
    Type16 {
        /// Width of threshold arrays
        width: u32,
        /// Height of threshold arrays  
        height: u32,
        /// Multiple threshold arrays
        thresholds: Vec<Vec<u8>>,
    },
}

/// Spot function for halftone screening
#[derive(Debug, Clone, PartialEq)]
pub enum SpotFunction {
    /// Simple dot
    SimpleDot,
    /// Inverted simple dot
    InvertedSimpleDot,
    /// Round dot
    Round,
    /// Inverted round dot
    InvertedRound,
    /// Ellipse
    Ellipse,
    /// Square
    Square,
    /// Cross
    Cross,
    /// Diamond
    Diamond,
    /// Line
    Line,
    /// Custom spot function
    Custom(String),
}

impl SpotFunction {
    /// Get the PDF name for this spot function
    pub fn pdf_name(&self) -> String {
        match self {
            SpotFunction::SimpleDot => "SimpleDot".to_string(),
            SpotFunction::InvertedSimpleDot => "InvertedSimpleDot".to_string(),
            SpotFunction::Round => "Round".to_string(),
            SpotFunction::InvertedRound => "InvertedRound".to_string(),
            SpotFunction::Ellipse => "Ellipse".to_string(),
            SpotFunction::Square => "Square".to_string(),
            SpotFunction::Cross => "Cross".to_string(),
            SpotFunction::Diamond => "Diamond".to_string(),
            SpotFunction::Line => "Line".to_string(),
            SpotFunction::Custom(name) => name.clone(),
        }
    }
}

/// Halftone specification for a single colorant
#[derive(Debug, Clone, PartialEq)]
pub struct HalftoneColorant {
    /// Halftone frequency
    pub frequency: f64,
    /// Halftone angle in degrees
    pub angle: f64,
    /// Spot function
    pub spot_function: SpotFunction,
}

/// Extended Graphics State Dictionary according to ISO 32000-1 Section 8.4
#[derive(Debug, Clone)]
pub struct ExtGState {
    // Line parameters
    /// Line width (LW)
    pub line_width: Option<f64>,
    /// Line cap style (LC)
    pub line_cap: Option<LineCap>,
    /// Line join style (LJ)
    pub line_join: Option<LineJoin>,
    /// Miter limit (ML)
    pub miter_limit: Option<f64>,
    /// Line dash pattern (D)
    pub dash_pattern: Option<LineDashPattern>,

    // Rendering intent
    /// Rendering intent (RI)
    pub rendering_intent: Option<RenderingIntent>,

    // Overprint control
    /// Overprint for stroking operations (OP)
    pub overprint_stroke: Option<bool>,
    /// Overprint for non-stroking operations (op)
    pub overprint_fill: Option<bool>,
    /// Overprint mode (OPM)
    pub overprint_mode: Option<u8>,

    // Font
    /// Font and size (Font)
    pub font: Option<ExtGStateFont>,

    // Color functions (simplified for basic implementation)
    /// Black generation function (BG)
    pub black_generation: Option<TransferFunction>,
    /// Black generation function alternative (BG2)
    pub black_generation_2: Option<TransferFunction>,
    /// Undercolor removal function (UCR)
    pub undercolor_removal: Option<TransferFunction>,
    /// Undercolor removal function alternative (UCR2)
    pub undercolor_removal_2: Option<TransferFunction>,
    /// Transfer function (TR)
    pub transfer_function: Option<TransferFunction>,
    /// Transfer function alternative (TR2)
    pub transfer_function_2: Option<TransferFunction>,

    // Halftone
    /// Halftone dictionary (HT)
    pub halftone: Option<Halftone>,

    // Flatness and smoothness
    /// Flatness tolerance (FL)
    pub flatness: Option<f64>,
    /// Smoothness tolerance (SM)
    pub smoothness: Option<f64>,

    // Additional parameters
    /// Automatic stroke adjustment (SA)
    pub stroke_adjustment: Option<bool>,

    // Transparency parameters (PDF 1.4+)
    /// Blend mode (BM)
    pub blend_mode: Option<BlendMode>,
    /// Soft mask (SMask)
    pub soft_mask: Option<SoftMask>,
    /// Alpha constant for stroking (CA)
    pub alpha_stroke: Option<f64>,
    /// Alpha constant for non-stroking (ca)
    pub alpha_fill: Option<f64>,
    /// Alpha source flag (AIS)
    pub alpha_is_shape: Option<bool>,
    /// Text knockout flag (TK)
    pub text_knockout: Option<bool>,

    // PDF 2.0 additions
    /// Black point compensation (UseBlackPtComp)
    pub use_black_point_compensation: Option<bool>,
}

impl Default for ExtGState {
    fn default() -> Self {
        Self::new()
    }
}

impl ExtGState {
    /// Create a new empty ExtGState dictionary
    pub fn new() -> Self {
        Self {
            line_width: None,
            line_cap: None,
            line_join: None,
            miter_limit: None,
            dash_pattern: None,
            rendering_intent: None,
            overprint_stroke: None,
            overprint_fill: None,
            overprint_mode: None,
            font: None,
            black_generation: None,
            black_generation_2: None,
            undercolor_removal: None,
            undercolor_removal_2: None,
            transfer_function: None,
            transfer_function_2: None,
            halftone: None,
            flatness: None,
            smoothness: None,
            stroke_adjustment: None,
            blend_mode: None,
            soft_mask: None,
            alpha_stroke: None,
            alpha_fill: None,
            alpha_is_shape: None,
            text_knockout: None,
            use_black_point_compensation: None,
        }
    }

    // Line parameter setters
    /// Set line width
    pub fn with_line_width(mut self, width: f64) -> Self {
        self.line_width = Some(width.max(0.0));
        self
    }

    /// Set line cap style
    pub fn with_line_cap(mut self, cap: LineCap) -> Self {
        self.line_cap = Some(cap);
        self
    }

    /// Set line join style
    pub fn with_line_join(mut self, join: LineJoin) -> Self {
        self.line_join = Some(join);
        self
    }

    /// Set miter limit
    pub fn with_miter_limit(mut self, limit: f64) -> Self {
        self.miter_limit = Some(limit.max(1.0));
        self
    }

    /// Set line dash pattern
    pub fn with_dash_pattern(mut self, pattern: LineDashPattern) -> Self {
        self.dash_pattern = Some(pattern);
        self
    }

    // Rendering intent setter
    /// Set rendering intent
    pub fn with_rendering_intent(mut self, intent: RenderingIntent) -> Self {
        self.rendering_intent = Some(intent);
        self
    }

    // Overprint setters
    /// Set overprint for stroking operations
    pub fn with_overprint_stroke(mut self, overprint: bool) -> Self {
        self.overprint_stroke = Some(overprint);
        self
    }

    /// Set overprint for non-stroking operations
    pub fn with_overprint_fill(mut self, overprint: bool) -> Self {
        self.overprint_fill = Some(overprint);
        self
    }

    /// Set overprint mode
    pub fn with_overprint_mode(mut self, mode: u8) -> Self {
        self.overprint_mode = Some(mode);
        self
    }

    // Font setter
    /// Set font and size
    pub fn with_font(mut self, font: Font, size: f64) -> Self {
        self.font = Some(ExtGStateFont::new(font, size.max(0.0)));
        self
    }

    // Flatness and smoothness setters
    /// Set flatness tolerance
    pub fn with_flatness(mut self, flatness: f64) -> Self {
        self.flatness = Some(flatness.clamp(0.0, 100.0));
        self
    }

    /// Set smoothness tolerance
    pub fn with_smoothness(mut self, smoothness: f64) -> Self {
        self.smoothness = Some(smoothness.clamp(0.0, 1.0));
        self
    }

    /// Set automatic stroke adjustment
    pub fn with_stroke_adjustment(mut self, adjustment: bool) -> Self {
        self.stroke_adjustment = Some(adjustment);
        self
    }

    // Transparency setters
    /// Set blend mode
    pub fn with_blend_mode(mut self, mode: BlendMode) -> Self {
        self.blend_mode = Some(mode);
        self
    }

    /// Set alpha constant for stroking operations
    pub fn with_alpha_stroke(mut self, alpha: f64) -> Self {
        self.alpha_stroke = Some(alpha.clamp(0.0, 1.0));
        self
    }

    /// Set alpha constant for non-stroking operations
    pub fn with_alpha_fill(mut self, alpha: f64) -> Self {
        self.alpha_fill = Some(alpha.clamp(0.0, 1.0));
        self
    }

    /// Set alpha constant for both stroking and non-stroking operations
    pub fn with_alpha(mut self, alpha: f64) -> Self {
        let clamped = alpha.clamp(0.0, 1.0);
        self.alpha_stroke = Some(clamped);
        self.alpha_fill = Some(clamped);
        self
    }

    /// Set alpha source flag
    pub fn with_alpha_is_shape(mut self, is_shape: bool) -> Self {
        self.alpha_is_shape = Some(is_shape);
        self
    }

    /// Set text knockout flag
    pub fn with_text_knockout(mut self, knockout: bool) -> Self {
        self.text_knockout = Some(knockout);
        self
    }

    /// Set soft mask for transparency
    pub fn set_soft_mask(&mut self, mask: SoftMask) {
        self.soft_mask = Some(mask);
    }

    /// Set soft mask with a named XObject
    pub fn set_soft_mask_name(&mut self, name: String) {
        self.soft_mask = Some(SoftMask::luminosity(name));
    }

    /// Remove soft mask (set to None)
    pub fn set_soft_mask_none(&mut self) {
        self.soft_mask = Some(SoftMask::none());
    }

    /// Set black point compensation (PDF 2.0)
    pub fn with_black_point_compensation(mut self, use_compensation: bool) -> Self {
        self.use_black_point_compensation = Some(use_compensation);
        self
    }

    // Transfer function setters
    /// Set transfer function for output device gamma correction
    pub fn with_transfer_function(mut self, func: TransferFunction) -> Self {
        self.transfer_function = Some(func);
        self
    }

    /// Set gamma correction transfer function
    pub fn with_gamma_correction(mut self, gamma: f64) -> Self {
        self.transfer_function = Some(TransferFunction::gamma(gamma));
        self
    }

    /// Set linear transfer function with slope and intercept
    pub fn with_linear_transfer(mut self, slope: f64, intercept: f64) -> Self {
        self.transfer_function = Some(TransferFunction::linear(slope, intercept));
        self
    }

    /// Set alternative transfer function (TR2)
    pub fn with_transfer_function_2(mut self, func: TransferFunction) -> Self {
        self.transfer_function_2 = Some(func);
        self
    }

    /// Set black generation function
    pub fn with_black_generation(mut self, func: TransferFunction) -> Self {
        self.black_generation = Some(func);
        self
    }

    /// Set undercolor removal function
    pub fn with_undercolor_removal(mut self, func: TransferFunction) -> Self {
        self.undercolor_removal = Some(func);
        self
    }

    /// Check if any transparency parameters are set
    pub fn uses_transparency(&self) -> bool {
        self.alpha_stroke.is_some_and(|a| a < 1.0)
            || self.alpha_fill.is_some_and(|a| a < 1.0)
            || self.blend_mode.is_some()
            || self.soft_mask.is_some()
    }

    /// Generate PDF dictionary representation
    pub fn to_pdf_dictionary(&self) -> Result<String> {
        let mut dict = String::from("<< /Type /ExtGState");

        // Line parameters
        if let Some(width) = self.line_width {
            write!(&mut dict, " /LW {width:.3}").map_err(|_| {
                PdfError::InvalidStructure("Failed to write line width".to_string())
            })?;
        }

        if let Some(cap) = self.line_cap {
            write!(&mut dict, " /LC {}", cap as u8)
                .map_err(|_| PdfError::InvalidStructure("Failed to write line cap".to_string()))?;
        }

        if let Some(join) = self.line_join {
            write!(&mut dict, " /LJ {}", join as u8)
                .map_err(|_| PdfError::InvalidStructure("Failed to write line join".to_string()))?;
        }

        if let Some(limit) = self.miter_limit {
            write!(&mut dict, " /ML {limit:.3}").map_err(|_| {
                PdfError::InvalidStructure("Failed to write miter limit".to_string())
            })?;
        }

        if let Some(ref pattern) = self.dash_pattern {
            write!(&mut dict, " /D {}", pattern.to_pdf_string()).map_err(|_| {
                PdfError::InvalidStructure("Failed to write dash pattern".to_string())
            })?;
        }

        // Rendering intent
        if let Some(intent) = self.rendering_intent {
            write!(&mut dict, " /RI /{}", intent.pdf_name()).map_err(|_| {
                PdfError::InvalidStructure("Failed to write rendering intent".to_string())
            })?;
        }

        // Overprint control
        if let Some(op) = self.overprint_stroke {
            write!(&mut dict, " /OP {op}").map_err(|_| {
                PdfError::InvalidStructure("Failed to write overprint stroke".to_string())
            })?;
        }

        if let Some(op) = self.overprint_fill {
            write!(&mut dict, " /op {op}").map_err(|_| {
                PdfError::InvalidStructure("Failed to write overprint fill".to_string())
            })?;
        }

        if let Some(mode) = self.overprint_mode {
            write!(&mut dict, " /OPM {mode}").map_err(|_| {
                PdfError::InvalidStructure("Failed to write overprint mode".to_string())
            })?;
        }

        // Font
        if let Some(ref font) = self.font {
            write!(
                &mut dict,
                " /Font [/{} {:.3}]",
                font.font.pdf_name(),
                font.size
            )
            .map_err(|_| PdfError::InvalidStructure("Failed to write font".to_string()))?;
        }

        // Flatness and smoothness
        if let Some(flatness) = self.flatness {
            write!(&mut dict, " /FL {flatness:.3}")
                .map_err(|_| PdfError::InvalidStructure("Failed to write flatness".to_string()))?;
        }

        if let Some(smoothness) = self.smoothness {
            write!(&mut dict, " /SM {smoothness:.3}").map_err(|_| {
                PdfError::InvalidStructure("Failed to write smoothness".to_string())
            })?;
        }

        // Stroke adjustment
        if let Some(sa) = self.stroke_adjustment {
            write!(&mut dict, " /SA {sa}").map_err(|_| {
                PdfError::InvalidStructure("Failed to write stroke adjustment".to_string())
            })?;
        }

        // Transparency parameters
        if let Some(ref mode) = self.blend_mode {
            write!(&mut dict, " /BM /{}", mode.pdf_name()).map_err(|_| {
                PdfError::InvalidStructure("Failed to write blend mode".to_string())
            })?;
        }

        if let Some(ref mask) = self.soft_mask {
            if mask.is_none() {
                write!(&mut dict, " /SMask /None").map_err(|_| {
                    PdfError::InvalidStructure("Failed to write soft mask".to_string())
                })?;
            } else {
                // In a full implementation, this would write the soft mask dictionary
                // For now, we write a reference
                write!(&mut dict, " /SMask {}", mask.to_pdf_string()).map_err(|_| {
                    PdfError::InvalidStructure("Failed to write soft mask".to_string())
                })?;
            }
        }

        if let Some(alpha) = self.alpha_stroke {
            write!(&mut dict, " /CA {alpha:.3}").map_err(|_| {
                PdfError::InvalidStructure("Failed to write stroke alpha".to_string())
            })?;
        }

        if let Some(alpha) = self.alpha_fill {
            write!(&mut dict, " /ca {alpha:.3}").map_err(|_| {
                PdfError::InvalidStructure("Failed to write fill alpha".to_string())
            })?;
        }

        if let Some(ais) = self.alpha_is_shape {
            write!(&mut dict, " /AIS {ais}").map_err(|_| {
                PdfError::InvalidStructure("Failed to write alpha is shape".to_string())
            })?;
        }

        if let Some(tk) = self.text_knockout {
            write!(&mut dict, " /TK {tk}").map_err(|_| {
                PdfError::InvalidStructure("Failed to write text knockout".to_string())
            })?;
        }

        // Transfer functions
        if let Some(ref tf) = self.transfer_function {
            write!(&mut dict, " /TR {}", tf.to_pdf_string()).map_err(|_| {
                PdfError::InvalidStructure("Failed to write transfer function".to_string())
            })?;
        }

        if let Some(ref tf) = self.transfer_function_2 {
            write!(&mut dict, " /TR2 {}", tf.to_pdf_string()).map_err(|_| {
                PdfError::InvalidStructure("Failed to write transfer function 2".to_string())
            })?;
        }

        if let Some(ref bg) = self.black_generation {
            write!(&mut dict, " /BG {}", bg.to_pdf_string()).map_err(|_| {
                PdfError::InvalidStructure("Failed to write black generation".to_string())
            })?;
        }

        if let Some(ref bg) = self.black_generation_2 {
            write!(&mut dict, " /BG2 {}", bg.to_pdf_string()).map_err(|_| {
                PdfError::InvalidStructure("Failed to write black generation 2".to_string())
            })?;
        }

        if let Some(ref ucr) = self.undercolor_removal {
            write!(&mut dict, " /UCR {}", ucr.to_pdf_string()).map_err(|_| {
                PdfError::InvalidStructure("Failed to write undercolor removal".to_string())
            })?;
        }

        if let Some(ref ucr) = self.undercolor_removal_2 {
            write!(&mut dict, " /UCR2 {}", ucr.to_pdf_string()).map_err(|_| {
                PdfError::InvalidStructure("Failed to write undercolor removal 2".to_string())
            })?;
        }

        // PDF 2.0 parameters
        if let Some(use_comp) = self.use_black_point_compensation {
            write!(&mut dict, " /UseBlackPtComp {use_comp}").map_err(|_| {
                PdfError::InvalidStructure("Failed to write black point compensation".to_string())
            })?;
        }

        dict.push_str(" >>");
        Ok(dict)
    }

    /// Check if the ExtGState is empty (no parameters set)
    pub fn is_empty(&self) -> bool {
        self.line_width.is_none()
            && self.line_cap.is_none()
            && self.line_join.is_none()
            && self.miter_limit.is_none()
            && self.dash_pattern.is_none()
            && self.rendering_intent.is_none()
            && self.overprint_stroke.is_none()
            && self.overprint_fill.is_none()
            && self.overprint_mode.is_none()
            && self.font.is_none()
            && self.flatness.is_none()
            && self.smoothness.is_none()
            && self.stroke_adjustment.is_none()
            && self.blend_mode.is_none()
            && self.soft_mask.is_none()
            && self.alpha_stroke.is_none()
            && self.alpha_fill.is_none()
            && self.alpha_is_shape.is_none()
            && self.text_knockout.is_none()
            && self.transfer_function.is_none()
            && self.transfer_function_2.is_none()
            && self.black_generation.is_none()
            && self.black_generation_2.is_none()
            && self.undercolor_removal.is_none()
            && self.undercolor_removal_2.is_none()
            && self.use_black_point_compensation.is_none()
    }

    /// Convert to Dictionary object for PDF writer
    pub fn to_dict(&self) -> crate::objects::Dictionary {
        use crate::objects::{Dictionary, Object};

        let mut dict = Dictionary::new();
        dict.set("Type", Object::Name("ExtGState".to_string()));

        // Line parameters
        if let Some(width) = self.line_width {
            dict.set("LW", Object::Real(width));
        }

        if let Some(cap) = self.line_cap {
            dict.set("LC", Object::Integer(cap as i64));
        }

        if let Some(join) = self.line_join {
            dict.set("LJ", Object::Integer(join as i64));
        }

        if let Some(limit) = self.miter_limit {
            dict.set("ML", Object::Real(limit));
        }

        // Transparency parameters
        if let Some(mode) = &self.blend_mode {
            dict.set("BM", Object::Name(mode.pdf_name().to_string()));
        }

        if let Some(alpha) = self.alpha_stroke {
            dict.set("CA", Object::Real(alpha));
        }

        if let Some(alpha) = self.alpha_fill {
            dict.set("ca", Object::Real(alpha));
        }

        if let Some(ais) = self.alpha_is_shape {
            dict.set("AIS", Object::Boolean(ais));
        }

        if let Some(tk) = self.text_knockout {
            dict.set("TK", Object::Boolean(tk));
        }

        // Other parameters
        if let Some(intent) = &self.rendering_intent {
            dict.set("RI", Object::Name(intent.pdf_name().to_string()));
        }

        if let Some(op) = self.overprint_stroke {
            dict.set("OP", Object::Boolean(op));
        }

        if let Some(op) = self.overprint_fill {
            dict.set("op", Object::Boolean(op));
        }

        if let Some(mode) = self.overprint_mode {
            dict.set("OPM", Object::Integer(mode as i64));
        }

        if let Some(flatness) = self.flatness {
            dict.set("FL", Object::Real(flatness));
        }

        if let Some(smoothness) = self.smoothness {
            dict.set("SM", Object::Real(smoothness));
        }

        if let Some(sa) = self.stroke_adjustment {
            dict.set("SA", Object::Boolean(sa));
        }

        dict
    }
}

/// ExtGState manager for handling multiple graphics states
#[derive(Debug, Clone)]
pub struct ExtGStateManager {
    states: HashMap<String, ExtGState>,
    next_id: usize,
}

impl Default for ExtGStateManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ExtGStateManager {
    /// Create a new ExtGState manager
    pub fn new() -> Self {
        Self {
            states: HashMap::new(),
            next_id: 1,
        }
    }

    /// Add an ExtGState and return its name
    pub fn add_state(&mut self, state: ExtGState) -> Result<String> {
        if state.is_empty() {
            return Err(PdfError::InvalidStructure(
                "ExtGState cannot be empty".to_string(),
            ));
        }

        let name = format!("GS{}", self.next_id);
        self.states.insert(name.clone(), state);
        self.next_id += 1;
        Ok(name)
    }

    /// Get an ExtGState by name
    pub fn get_state(&self, name: &str) -> Option<&ExtGState> {
        self.states.get(name)
    }

    /// Get all states
    pub fn states(&self) -> &HashMap<String, ExtGState> {
        &self.states
    }

    /// Generate ExtGState resource dictionary
    pub fn to_resource_dictionary(&self) -> Result<String> {
        if self.states.is_empty() {
            return Ok(String::new());
        }

        let mut dict = String::from("/ExtGState <<");

        for (name, state) in &self.states {
            let state_dict = state.to_pdf_dictionary()?;
            write!(&mut dict, " /{name} {state_dict}").map_err(|_| {
                PdfError::InvalidStructure("Failed to write ExtGState resource".to_string())
            })?;
        }

        dict.push_str(" >>");
        Ok(dict)
    }

    /// Clear all states
    pub fn clear(&mut self) {
        self.states.clear();
        self.next_id = 1;
    }

    /// Count of registered states
    pub fn count(&self) -> usize {
        self.states.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rendering_intent_pdf_names() {
        assert_eq!(
            RenderingIntent::AbsoluteColorimetric.pdf_name(),
            "AbsoluteColorimetric"
        );
        assert_eq!(
            RenderingIntent::RelativeColorimetric.pdf_name(),
            "RelativeColorimetric"
        );
        assert_eq!(RenderingIntent::Saturation.pdf_name(), "Saturation");
        assert_eq!(RenderingIntent::Perceptual.pdf_name(), "Perceptual");
    }

    #[test]
    fn test_blend_mode_pdf_names() {
        assert_eq!(BlendMode::Normal.pdf_name(), "Normal");
        assert_eq!(BlendMode::Multiply.pdf_name(), "Multiply");
        assert_eq!(BlendMode::Screen.pdf_name(), "Screen");
        assert_eq!(BlendMode::Overlay.pdf_name(), "Overlay");
    }

    #[test]
    fn test_line_dash_pattern_creation() {
        let solid = LineDashPattern::solid();
        assert!(solid.array.is_empty());
        assert_eq!(solid.phase, 0.0);

        let dashed = LineDashPattern::dashed(5.0, 3.0);
        assert_eq!(dashed.array, vec![5.0, 3.0]);
        assert_eq!(dashed.phase, 0.0);

        let dotted = LineDashPattern::dotted(1.0, 2.0);
        assert_eq!(dotted.array, vec![1.0, 2.0]);
    }

    #[test]
    fn test_line_dash_pattern_pdf_string() {
        let solid = LineDashPattern::solid();
        assert_eq!(solid.to_pdf_string(), "[] 0");

        let dashed = LineDashPattern::dashed(5.0, 3.0);
        assert_eq!(dashed.to_pdf_string(), "[5.00 3.00] 0.00");

        let custom = LineDashPattern::new(vec![10.0, 5.0, 2.0, 5.0], 2.5);
        assert_eq!(custom.to_pdf_string(), "[10.00 5.00 2.00 5.00] 2.50");
    }

    #[test]
    fn test_extgstate_font() {
        let font = ExtGStateFont::new(Font::Helvetica, 12.0);
        assert_eq!(font.font, Font::Helvetica);
        assert_eq!(font.size, 12.0);
    }

    #[test]
    fn test_extgstate_creation() {
        let state = ExtGState::new();
        assert!(state.is_empty());
        assert!(!state.uses_transparency());
    }

    #[test]
    fn test_extgstate_line_parameters() {
        let state = ExtGState::new()
            .with_line_width(2.5)
            .with_line_cap(LineCap::Round)
            .with_line_join(LineJoin::Bevel)
            .with_miter_limit(4.0);

        assert_eq!(state.line_width, Some(2.5));
        assert_eq!(state.line_cap, Some(LineCap::Round));
        assert_eq!(state.line_join, Some(LineJoin::Bevel));
        assert_eq!(state.miter_limit, Some(4.0));
        assert!(!state.is_empty());
    }

    #[test]
    fn test_extgstate_transparency() {
        let state = ExtGState::new()
            .with_alpha_stroke(0.8)
            .with_alpha_fill(0.6)
            .with_blend_mode(BlendMode::Multiply);

        assert_eq!(state.alpha_stroke, Some(0.8));
        assert_eq!(state.alpha_fill, Some(0.6));
        assert_eq!(state.blend_mode, Some(BlendMode::Multiply));
        assert!(state.uses_transparency());
    }

    #[test]
    fn test_extgstate_alpha_clamping() {
        let state = ExtGState::new()
            .with_alpha_stroke(1.5) // Should clamp to 1.0
            .with_alpha_fill(-0.1); // Should clamp to 0.0

        assert_eq!(state.alpha_stroke, Some(1.0));
        assert_eq!(state.alpha_fill, Some(0.0));
    }

    #[test]
    fn test_extgstate_combined_alpha() {
        let state = ExtGState::new().with_alpha(0.5);

        assert_eq!(state.alpha_stroke, Some(0.5));
        assert_eq!(state.alpha_fill, Some(0.5));
    }

    #[test]
    fn test_extgstate_rendering_intent() {
        let state = ExtGState::new().with_rendering_intent(RenderingIntent::Perceptual);

        assert_eq!(state.rendering_intent, Some(RenderingIntent::Perceptual));
    }

    #[test]
    fn test_extgstate_overprint() {
        let state = ExtGState::new()
            .with_overprint_stroke(true)
            .with_overprint_fill(false)
            .with_overprint_mode(1);

        assert_eq!(state.overprint_stroke, Some(true));
        assert_eq!(state.overprint_fill, Some(false));
        assert_eq!(state.overprint_mode, Some(1));
    }

    #[test]
    fn test_extgstate_font_setting() {
        let state = ExtGState::new().with_font(Font::HelveticaBold, 14.0);

        assert!(state.font.is_some());
        let font = state.font.unwrap();
        assert_eq!(font.font, Font::HelveticaBold);
        assert_eq!(font.size, 14.0);
    }

    #[test]
    fn test_extgstate_tolerance_parameters() {
        let state = ExtGState::new()
            .with_flatness(1.5)
            .with_smoothness(0.8)
            .with_stroke_adjustment(true);

        assert_eq!(state.flatness, Some(1.5));
        assert_eq!(state.smoothness, Some(0.8));
        assert_eq!(state.stroke_adjustment, Some(true));
    }

    #[test]
    fn test_extgstate_pdf_dictionary_generation() {
        let state = ExtGState::new()
            .with_line_width(2.0)
            .with_line_cap(LineCap::Round)
            .with_alpha(0.5)
            .with_blend_mode(BlendMode::Multiply);

        let dict = state.to_pdf_dictionary().unwrap();
        assert!(dict.contains("/Type /ExtGState"));
        assert!(dict.contains("/LW 2.000"));
        assert!(dict.contains("/LC 1"));
        assert!(dict.contains("/CA 0.500"));
        assert!(dict.contains("/ca 0.500"));
        assert!(dict.contains("/BM /Multiply"));
    }

    #[test]
    fn test_extgstate_manager_creation() {
        let manager = ExtGStateManager::new();
        assert_eq!(manager.count(), 0);
        assert!(manager.states().is_empty());
    }

    #[test]
    fn test_extgstate_manager_add_state() {
        let mut manager = ExtGStateManager::new();
        let state = ExtGState::new().with_line_width(2.0);

        let name = manager.add_state(state).unwrap();
        assert_eq!(name, "GS1");
        assert_eq!(manager.count(), 1);

        let retrieved = manager.get_state(&name).unwrap();
        assert_eq!(retrieved.line_width, Some(2.0));
    }

    #[test]
    fn test_extgstate_manager_empty_state_rejection() {
        let mut manager = ExtGStateManager::new();
        let empty_state = ExtGState::new();

        let result = manager.add_state(empty_state);
        assert!(result.is_err());
        assert_eq!(manager.count(), 0);
    }

    #[test]
    fn test_extgstate_manager_multiple_states() {
        let mut manager = ExtGStateManager::new();

        let state1 = ExtGState::new().with_line_width(1.0);
        let state2 = ExtGState::new().with_alpha(0.5);

        let name1 = manager.add_state(state1).unwrap();
        let name2 = manager.add_state(state2).unwrap();

        assert_eq!(name1, "GS1");
        assert_eq!(name2, "GS2");
        assert_eq!(manager.count(), 2);
    }

    #[test]
    fn test_extgstate_manager_resource_dictionary() {
        let mut manager = ExtGStateManager::new();

        let state = ExtGState::new().with_line_width(2.0);
        manager.add_state(state).unwrap();

        let dict = manager.to_resource_dictionary().unwrap();
        assert!(dict.contains("/ExtGState"));
        assert!(dict.contains("/GS1"));
        assert!(dict.contains("/LW 2.000"));
    }

    #[test]
    fn test_extgstate_manager_clear() {
        let mut manager = ExtGStateManager::new();

        let state = ExtGState::new().with_line_width(1.0);
        manager.add_state(state).unwrap();
        assert_eq!(manager.count(), 1);

        manager.clear();
        assert_eq!(manager.count(), 0);
        assert!(manager.states().is_empty());
    }

    #[test]
    fn test_extgstate_value_validation() {
        // Test line width validation (non-negative)
        let state = ExtGState::new().with_line_width(-1.0);
        assert_eq!(state.line_width, Some(0.0));

        // Test miter limit validation (>= 1.0)
        let state = ExtGState::new().with_miter_limit(0.5);
        assert_eq!(state.miter_limit, Some(1.0));

        // Test flatness validation (0-100)
        let state = ExtGState::new().with_flatness(150.0);
        assert_eq!(state.flatness, Some(100.0));

        // Test smoothness validation (0-1)
        let state = ExtGState::new().with_smoothness(1.5);
        assert_eq!(state.smoothness, Some(1.0));

        // Test font size validation (non-negative)
        let state = ExtGState::new().with_font(Font::Helvetica, -5.0);
        assert_eq!(state.font.unwrap().size, 0.0);
    }

    #[test]
    fn test_line_dash_patterns() {
        let state = ExtGState::new().with_dash_pattern(LineDashPattern::dashed(10.0, 5.0));

        let dict = state.to_pdf_dictionary().unwrap();
        assert!(dict.contains("/D [10.00 5.00] 0.00"));
    }

    #[test]
    fn test_complex_extgstate() {
        let dash_pattern = LineDashPattern::new(vec![3.0, 2.0, 1.0, 2.0], 1.0);

        let state = ExtGState::new()
            .with_line_width(1.5)
            .with_line_cap(LineCap::Square)
            .with_line_join(LineJoin::Round)
            .with_miter_limit(10.0)
            .with_dash_pattern(dash_pattern)
            .with_rendering_intent(RenderingIntent::Saturation)
            .with_overprint_stroke(true)
            .with_overprint_fill(false)
            .with_font(Font::TimesBold, 18.0)
            .with_flatness(0.5)
            .with_smoothness(0.1)
            .with_stroke_adjustment(false)
            .with_blend_mode(BlendMode::SoftLight)
            .with_alpha_stroke(0.8)
            .with_alpha_fill(0.6)
            .with_alpha_is_shape(true)
            .with_text_knockout(false);

        assert!(!state.is_empty());
        assert!(state.uses_transparency());

        let dict = state.to_pdf_dictionary().unwrap();
        assert!(dict.contains("/Type /ExtGState"));
        assert!(dict.contains("/LW 1.500"));
        assert!(dict.contains("/LC 2"));
        assert!(dict.contains("/LJ 1"));
        assert!(dict.contains("/ML 10.000"));
        assert!(dict.contains("/D [3.00 2.00 1.00 2.00] 1.00"));
        assert!(dict.contains("/RI /Saturation"));
        assert!(dict.contains("/OP true"));
        assert!(dict.contains("/op false"));
        assert!(dict.contains("/Font [/Times-Bold 18.000]"));
        assert!(dict.contains("/FL 0.500"));
        assert!(dict.contains("/SM 0.100"));
        assert!(dict.contains("/SA false"));
        assert!(dict.contains("/BM /SoftLight"));
        assert!(dict.contains("/CA 0.800"));
        assert!(dict.contains("/ca 0.600"));
        assert!(dict.contains("/AIS true"));
        assert!(dict.contains("/TK false"));
    }

    #[test]
    fn test_transfer_function_identity() {
        let tf = TransferFunction::identity();
        assert_eq!(tf.to_pdf_string(), "/Identity");
    }

    #[test]
    fn test_transfer_function_gamma() {
        let tf = TransferFunction::gamma(2.2);
        let pdf = tf.to_pdf_string();
        assert!(pdf.contains("/FunctionType 2"));
        assert!(pdf.contains("/N 2.200"));
        assert!(pdf.contains("/Domain [0.000 1.000]"));
        assert!(pdf.contains("/Range [0.000 1.000]"));
        assert!(pdf.contains("/C0 [0.000]"));
        assert!(pdf.contains("/C1 [1.000]"));
    }

    #[test]
    fn test_transfer_function_linear() {
        let tf = TransferFunction::linear(0.8, 0.1);
        let pdf = tf.to_pdf_string();
        assert!(pdf.contains("/FunctionType 2"));
        assert!(pdf.contains("/N 1.000"));
        assert!(pdf.contains("/C0 [0.100]")); // intercept
        assert!(pdf.contains("/C1 [0.900]")); // slope + intercept
    }

    #[test]
    fn test_extgstate_with_transfer_functions() {
        let state = ExtGState::new()
            .with_gamma_correction(1.8)
            .with_transfer_function_2(TransferFunction::identity())
            .with_black_generation(TransferFunction::linear(1.0, 0.0))
            .with_undercolor_removal(TransferFunction::gamma(2.2));

        assert!(!state.is_empty());

        let dict = state.to_pdf_dictionary().unwrap();
        assert!(dict.contains("/TR"));
        assert!(dict.contains("/TR2 /Identity"));
        assert!(dict.contains("/BG"));
        assert!(dict.contains("/UCR"));
        assert!(dict.contains("/N 1.800")); // gamma value for TR
        assert!(dict.contains("/N 2.200")); // gamma value for UCR
    }

    #[test]
    fn test_transfer_function_separate() {
        let c_func = TransferFunctionData {
            function_type: 2,
            domain: vec![0.0, 1.0],
            range: vec![0.0, 1.0],
            params: TransferFunctionParams::Exponential {
                c0: vec![0.0],
                c1: vec![1.0],
                n: 1.5,
            },
        };

        let m_func = c_func.clone();
        let y_func = c_func.clone();
        let k_func = Some(TransferFunctionData {
            function_type: 2,
            domain: vec![0.0, 1.0],
            range: vec![0.0, 1.0],
            params: TransferFunctionParams::Exponential {
                c0: vec![0.1],
                c1: vec![0.9],
                n: 2.0,
            },
        });

        let tf = TransferFunction::Separate {
            c_or_r: c_func,
            m_or_g: m_func,
            y_or_b: y_func,
            k: k_func,
        };

        let pdf = tf.to_pdf_string();
        assert!(pdf.starts_with('['));
        assert!(pdf.ends_with(']'));
        assert!(pdf.contains("/FunctionType 2"));
        // Should have 4 functions for CMYK
        assert_eq!(pdf.matches("/FunctionType 2").count(), 4);
    }
}
