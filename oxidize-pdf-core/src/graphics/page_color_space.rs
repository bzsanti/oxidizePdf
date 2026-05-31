//! Typed wrapper for page-level colour-space resource registration
//! (ISO 32000-1 §8.6, Table 62).
//!
//! `Page::add_color_space` originally took a raw
//! [`crate::objects::Object`], which leaked an internal serialization
//! type across the public API and made the signature SemVer-fragile.
//! This module introduces a small enum that models the two wire-format
//! shapes a colour-space resource entry is allowed to take:
//!
//!   * A single `/Name` alias for a device space (ISO 32000-1 §8.6.4,
//!     e.g. `/DeviceRGB`, `/DeviceCMYK`, `/Pattern`).
//!   * A parameterised array `[/<family> <<params>>]` for calibrated
//!     spaces (§8.6.5 `CalGray`, `CalRGB`, `Lab`, `ICCBased`).
//!
//! Indexed, Separation, and `DeviceN` spaces are intentionally out of
//! scope for the v2.5.6 wrapper — those require longer tuple shapes
//! (`[/Indexed base hival lookup]`, `[/Separation name alt tintFn]`,
//! `[/DeviceN names alt tintFn attributes]`) that are better served by
//! dedicated constructors added in a future SemVer-compatible superset
//! (the enum is `#[non_exhaustive]` to preserve that option).

use super::calibrated_color::{CalGrayColorSpace, CalRgbColorSpace};
use super::color_profiles::IccProfile;
use super::lab_color::LabColorSpace;
use crate::objects::{Dictionary, Object};

/// A colour space eligible for registration on a [`crate::Page`] under
/// `/Resources/ColorSpace/<name>`.
///
/// See the module-level docs for the ISO 32000-1 clauses this models.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum PageColorSpace {
    /// A named device-space alias — emitted as a single `/Name` at the
    /// resource slot (ISO 32000-1 §8.6.4). Use when the caller wants
    /// to reference a device space via a numeric or symbolic alias
    /// (e.g. `/CS1 /DeviceRGB`).
    DeviceAlias(DeviceColorSpace),
    /// A calibrated colour space — emitted as `[/<family> <<params>>]`
    /// (ISO 32000-1 §8.6.5). The parameter dictionary is written
    /// verbatim; callers are responsible for its content.
    Parameterised {
        /// Which calibrated family this entry represents.
        family: ParameterisedFamily,
        /// Parameter dictionary — e.g. `WhitePoint`, `Gamma`, `Matrix`
        /// for CalRGB; `N`, `Alternate`, `Metadata` for ICCBased.
        params: Dictionary,
    },
    /// An ICC-profile-backed colour space emitted as a conformant indirect
    /// **stream**: `[/ICCBased <ref>]` where `<ref>` resolves to a stream
    /// `<< /N n /Alternate … /Range … >> stream <profile bytes> endstream`
    /// (ISO 32000-1 §8.6.5.5). Unlike [`Self::Parameterised`] with
    /// [`ParameterisedFamily::IccBased`] — which can only express an inline
    /// dict and therefore drops the profile bytes — this variant carries the
    /// raw profile so the writer can embed it. The stream object id is
    /// allocated by the writer at emit time (a stream cannot be inlined into a
    /// resource dict), so the conversion goes through
    /// [`Self::icc_stream_parts`], not [`Self::to_object`].
    IccStream {
        /// Number of colour components (`/N`: 1, 3, or 4).
        n: u8,
        /// Device colour space to fall back to (`/Alternate`).
        alternate: DeviceColorSpace,
        /// Raw ICC profile bytes, written verbatim into the stream.
        profile_data: Vec<u8>,
        /// Optional `/Range` for the components.
        range: Option<Vec<f64>>,
    },
}

/// The four device colour spaces addressable through
/// [`PageColorSpace::DeviceAlias`] (ISO 32000-1 §8.6.4 device spaces
/// + §8.7.3.1 Pattern colour space).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum DeviceColorSpace {
    /// Single-channel grayscale (`/DeviceGray`).
    Gray,
    /// Three-channel RGB (`/DeviceRGB`).
    Rgb,
    /// Four-channel CMYK (`/DeviceCMYK`).
    Cmyk,
    /// Pattern colour space (`/Pattern`).
    Pattern,
}

/// The calibrated colour-space families addressable through
/// [`PageColorSpace::Parameterised`] (ISO 32000-1 §8.6.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ParameterisedFamily {
    /// `CalGray` — CIE-based single-component (§8.6.5.1).
    CalGray,
    /// `CalRGB` — CIE-based three-component (§8.6.5.2).
    CalRgb,
    /// `Lab` — CIE 1976 L*a*b* (§8.6.5.4).
    Lab,
    /// `ICCBased` — ICC-profile-backed (§8.6.5.5).
    IccBased,
}

impl DeviceColorSpace {
    /// Returns the ISO 32000-1 §8.6 PDF name for this device space,
    /// without the leading `/`.
    pub const fn pdf_name(self) -> &'static str {
        match self {
            DeviceColorSpace::Gray => "DeviceGray",
            DeviceColorSpace::Rgb => "DeviceRGB",
            DeviceColorSpace::Cmyk => "DeviceCMYK",
            DeviceColorSpace::Pattern => "Pattern",
        }
    }
}

impl ParameterisedFamily {
    /// Returns the ISO 32000-1 §8.6.5 family name for this calibrated
    /// colour space (the first element of the emitted array).
    pub const fn pdf_name(self) -> &'static str {
        match self {
            ParameterisedFamily::CalGray => "CalGray",
            ParameterisedFamily::CalRgb => "CalRGB",
            ParameterisedFamily::Lab => "Lab",
            ParameterisedFamily::IccBased => "ICCBased",
        }
    }
}

impl PageColorSpace {
    /// Convert to the concrete [`Object`] shape the writer emits at
    /// `/Resources/ColorSpace/<name>`.
    ///
    /// Device aliases become `Object::Name`; parameterised entries
    /// become `Object::Array([Name, Dictionary])`. This keeps the
    /// conversion in one place so wire-format decisions (e.g. whether
    /// a future family needs a stream instead of a dict) live with the
    /// enum they describe, not scattered across the writer.
    pub(crate) fn to_object(&self) -> Object {
        match self {
            PageColorSpace::DeviceAlias(device) => Object::Name(device.pdf_name().to_string()),
            PageColorSpace::Parameterised { family, params } => Object::Array(vec![
                Object::Name(family.pdf_name().to_string()),
                Object::Dictionary(params.clone()),
            ]),
            // Inline fallback only. The conformant emission for ICC profiles is
            // a stream (see `icc_stream_parts`); the writer routes `IccStream`
            // there and never calls `to_object` for it. This arm keeps the
            // method total and yields the non-stream `[/ICCBased <<dict>>]`
            // shape (profile bytes omitted) should a caller invoke it directly.
            PageColorSpace::IccStream {
                n,
                alternate,
                range,
                ..
            } => {
                let mut dict = Dictionary::new();
                dict.set("N", Object::Integer(*n as i64));
                dict.set("Alternate", Object::Name(alternate.pdf_name().to_string()));
                if let Some(r) = range {
                    dict.set(
                        "Range",
                        Object::Array(r.iter().map(|&x| Object::Real(x)).collect()),
                    );
                }
                Object::Array(vec![
                    Object::Name("ICCBased".to_string()),
                    Object::Dictionary(dict),
                ])
            }
        }
    }

    /// If this is an [`Self::IccStream`], return the ICC stream dictionary
    /// (`/N`, `/Alternate`, optional `/Range`) and the raw profile bytes for
    /// the writer to emit as an indirect stream object. Returns `None` for the
    /// inline (name / parameterised-dict) variants.
    pub(crate) fn icc_stream_parts(&self) -> Option<(Dictionary, Vec<u8>)> {
        match self {
            PageColorSpace::IccStream {
                n,
                alternate,
                profile_data,
                range,
            } => {
                let mut dict = Dictionary::new();
                dict.set("N", Object::Integer(*n as i64));
                dict.set("Alternate", Object::Name(alternate.pdf_name().to_string()));
                if let Some(r) = range {
                    dict.set(
                        "Range",
                        Object::Array(r.iter().map(|&x| Object::Real(x)).collect()),
                    );
                }
                Some((dict, profile_data.clone()))
            }
            _ => None,
        }
    }
}

impl From<&CalGrayColorSpace> for PageColorSpace {
    /// Bridge a typed [`CalGrayColorSpace`] into a registrable colour space,
    /// reusing the struct's own [`CalGrayColorSpace::params_dictionary`]
    /// (ISO 32000-1 §8.6.5.1).
    fn from(cs: &CalGrayColorSpace) -> Self {
        PageColorSpace::Parameterised {
            family: ParameterisedFamily::CalGray,
            params: cs.params_dictionary(),
        }
    }
}

impl From<&CalRgbColorSpace> for PageColorSpace {
    /// Bridge a typed [`CalRgbColorSpace`] into a registrable colour space
    /// (ISO 32000-1 §8.6.5.2).
    fn from(cs: &CalRgbColorSpace) -> Self {
        PageColorSpace::Parameterised {
            family: ParameterisedFamily::CalRgb,
            params: cs.params_dictionary(),
        }
    }
}

impl From<&LabColorSpace> for PageColorSpace {
    /// Bridge a typed [`LabColorSpace`] into a registrable colour space
    /// (ISO 32000-1 §8.6.5.4).
    fn from(cs: &LabColorSpace) -> Self {
        PageColorSpace::Parameterised {
            family: ParameterisedFamily::Lab,
            params: cs.params_dictionary(),
        }
    }
}

impl From<&IccProfile> for PageColorSpace {
    /// Bridge an [`IccProfile`] into a stream-backed ICC colour space
    /// ([`PageColorSpace::IccStream`]), carrying the profile bytes so the
    /// writer emits a conformant `/ICCBased` stream (ISO 32000-1 §8.6.5.5).
    /// `/Alternate` is the device space matching the component count.
    fn from(profile: &IccProfile) -> Self {
        let alternate = match profile.components {
            1 => DeviceColorSpace::Gray,
            4 => DeviceColorSpace::Cmyk,
            _ => DeviceColorSpace::Rgb,
        };
        PageColorSpace::IccStream {
            n: profile.components,
            alternate,
            profile_data: profile.data.clone(),
            range: profile.range.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn device_alias_to_object_is_name() {
        let obj = PageColorSpace::DeviceAlias(DeviceColorSpace::Cmyk).to_object();
        match obj {
            Object::Name(n) => assert_eq!(n, "DeviceCMYK"),
            other => panic!("expected Name(DeviceCMYK), got {other:?}"),
        }
    }

    #[test]
    fn parameterised_to_object_is_two_element_array() {
        let mut params = Dictionary::new();
        params.set("Gamma", Object::Real(2.2));
        let obj = PageColorSpace::Parameterised {
            family: ParameterisedFamily::CalGray,
            params,
        }
        .to_object();
        match obj {
            Object::Array(a) => {
                assert_eq!(a.len(), 2);
                assert!(matches!(&a[0], Object::Name(n) if n == "CalGray"));
                assert!(matches!(&a[1], Object::Dictionary(_)));
            }
            other => panic!("expected two-element array, got {other:?}"),
        }
    }

    #[test]
    fn device_pdf_name_covers_all_variants() {
        assert_eq!(DeviceColorSpace::Gray.pdf_name(), "DeviceGray");
        assert_eq!(DeviceColorSpace::Rgb.pdf_name(), "DeviceRGB");
        assert_eq!(DeviceColorSpace::Cmyk.pdf_name(), "DeviceCMYK");
        assert_eq!(DeviceColorSpace::Pattern.pdf_name(), "Pattern");
    }

    #[test]
    fn parameterised_pdf_name_covers_all_variants() {
        assert_eq!(ParameterisedFamily::CalGray.pdf_name(), "CalGray");
        assert_eq!(ParameterisedFamily::CalRgb.pdf_name(), "CalRGB");
        assert_eq!(ParameterisedFamily::Lab.pdf_name(), "Lab");
        assert_eq!(ParameterisedFamily::IccBased.pdf_name(), "ICCBased");
    }
}
