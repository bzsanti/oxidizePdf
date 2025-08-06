//! PDF destinations according to ISO 32000-1 Section 12.3.2

use crate::geometry::Rectangle;
use crate::objects::{Array, Object, ObjectId};

/// PDF destination types
#[derive(Debug, Clone, PartialEq)]
pub enum DestinationType {
    /// Display page with coordinates (left, top) at upper-left corner
    XYZ {
        left: Option<f64>,
        top: Option<f64>,
        zoom: Option<f64>,
    },
    /// Fit entire page in window
    Fit,
    /// Fit width of page in window
    FitH { top: Option<f64> },
    /// Fit height of page in window
    FitV { left: Option<f64> },
    /// Fit rectangle in window
    FitR { rect: Rectangle },
    /// Fit page bounding box in window
    FitB,
    /// Fit width of bounding box
    FitBH { top: Option<f64> },
    /// Fit height of bounding box
    FitBV { left: Option<f64> },
}

/// Page destination reference
#[derive(Debug, Clone)]
pub enum PageDestination {
    /// Page number (0-based)
    PageNumber(u32),
    /// Page object reference
    PageRef(ObjectId),
}

/// PDF destination
#[derive(Debug, Clone)]
pub struct Destination {
    /// Target page
    pub page: PageDestination,
    /// Destination type
    pub dest_type: DestinationType,
}

impl Destination {
    /// Create XYZ destination
    pub fn xyz(
        page: PageDestination,
        left: Option<f64>,
        top: Option<f64>,
        zoom: Option<f64>,
    ) -> Self {
        Self {
            page,
            dest_type: DestinationType::XYZ { left, top, zoom },
        }
    }

    /// Create Fit destination
    pub fn fit(page: PageDestination) -> Self {
        Self {
            page,
            dest_type: DestinationType::Fit,
        }
    }

    /// Create FitH destination
    pub fn fit_h(page: PageDestination, top: Option<f64>) -> Self {
        Self {
            page,
            dest_type: DestinationType::FitH { top },
        }
    }

    /// Create FitV destination
    pub fn fit_v(page: PageDestination, left: Option<f64>) -> Self {
        Self {
            page,
            dest_type: DestinationType::FitV { left },
        }
    }

    /// Create FitR destination
    pub fn fit_r(page: PageDestination, rect: Rectangle) -> Self {
        Self {
            page,
            dest_type: DestinationType::FitR { rect },
        }
    }

    /// Create FitB destination
    pub fn fit_b(page: PageDestination) -> Self {
        Self {
            page,
            dest_type: DestinationType::FitB,
        }
    }

    /// Create FitBH destination
    pub fn fit_bh(page: PageDestination, top: Option<f64>) -> Self {
        Self {
            page,
            dest_type: DestinationType::FitBH { top },
        }
    }

    /// Create FitBV destination
    pub fn fit_bv(page: PageDestination, left: Option<f64>) -> Self {
        Self {
            page,
            dest_type: DestinationType::FitBV { left },
        }
    }

    /// Create destination from array
    pub fn from_array(arr: &Array) -> Result<Self, crate::error::PdfError> {
        use crate::error::PdfError;

        if arr.len() < 2 {
            return Err(PdfError::InvalidStructure(
                "Destination array too short".into(),
            ));
        }

        // Get page reference
        let page = match arr.get(0) {
            Some(Object::Integer(num)) => PageDestination::PageNumber(*num as u32),
            Some(Object::Reference(id)) => PageDestination::PageRef(*id),
            _ => {
                return Err(PdfError::InvalidStructure(
                    "Invalid page reference in destination".into(),
                ))
            }
        };

        // Get destination type
        let type_name = match arr.get(1) {
            Some(Object::Name(name)) => name,
            _ => {
                return Err(PdfError::InvalidStructure(
                    "Invalid destination type".into(),
                ))
            }
        };

        let dest_type = match type_name.as_str() {
            "XYZ" => {
                if arr.len() < 5 {
                    return Err(PdfError::InvalidStructure(
                        "XYZ destination missing parameters".into(),
                    ));
                }
                let left = match arr.get(2) {
                    Some(Object::Real(v)) => Some(*v),
                    Some(Object::Integer(v)) => Some(*v as f64),
                    Some(Object::Null) => None,
                    _ => {
                        return Err(PdfError::InvalidStructure(
                            "Invalid XYZ left parameter".into(),
                        ))
                    }
                };
                let top = match arr.get(3) {
                    Some(Object::Real(v)) => Some(*v),
                    Some(Object::Integer(v)) => Some(*v as f64),
                    Some(Object::Null) => None,
                    _ => {
                        return Err(PdfError::InvalidStructure(
                            "Invalid XYZ top parameter".into(),
                        ))
                    }
                };
                let zoom = match arr.get(4) {
                    Some(Object::Real(v)) => Some(*v),
                    Some(Object::Integer(v)) => Some(*v as f64),
                    Some(Object::Null) => None,
                    _ => {
                        return Err(PdfError::InvalidStructure(
                            "Invalid XYZ zoom parameter".into(),
                        ))
                    }
                };
                DestinationType::XYZ { left, top, zoom }
            }
            "Fit" => DestinationType::Fit,
            "FitH" => {
                if arr.len() < 3 {
                    return Err(PdfError::InvalidStructure(
                        "FitH destination missing parameter".into(),
                    ));
                }
                let top = match arr.get(2) {
                    Some(Object::Real(v)) => Some(*v),
                    Some(Object::Integer(v)) => Some(*v as f64),
                    Some(Object::Null) => None,
                    _ => {
                        return Err(PdfError::InvalidStructure(
                            "Invalid FitH top parameter".into(),
                        ))
                    }
                };
                DestinationType::FitH { top }
            }
            "FitV" => {
                if arr.len() < 3 {
                    return Err(PdfError::InvalidStructure(
                        "FitV destination missing parameter".into(),
                    ));
                }
                let left = match arr.get(2) {
                    Some(Object::Real(v)) => Some(*v),
                    Some(Object::Integer(v)) => Some(*v as f64),
                    Some(Object::Null) => None,
                    _ => {
                        return Err(PdfError::InvalidStructure(
                            "Invalid FitV left parameter".into(),
                        ))
                    }
                };
                DestinationType::FitV { left }
            }
            "FitR" => {
                if arr.len() < 6 {
                    return Err(PdfError::InvalidStructure(
                        "FitR destination missing parameters".into(),
                    ));
                }
                let left = match arr.get(2) {
                    Some(Object::Real(v)) => *v,
                    Some(Object::Integer(v)) => *v as f64,
                    _ => {
                        return Err(PdfError::InvalidStructure(
                            "Invalid FitR left parameter".into(),
                        ))
                    }
                };
                let bottom = match arr.get(3) {
                    Some(Object::Real(v)) => *v,
                    Some(Object::Integer(v)) => *v as f64,
                    _ => {
                        return Err(PdfError::InvalidStructure(
                            "Invalid FitR bottom parameter".into(),
                        ))
                    }
                };
                let right = match arr.get(4) {
                    Some(Object::Real(v)) => *v,
                    Some(Object::Integer(v)) => *v as f64,
                    _ => {
                        return Err(PdfError::InvalidStructure(
                            "Invalid FitR right parameter".into(),
                        ))
                    }
                };
                let top = match arr.get(5) {
                    Some(Object::Real(v)) => *v,
                    Some(Object::Integer(v)) => *v as f64,
                    _ => {
                        return Err(PdfError::InvalidStructure(
                            "Invalid FitR top parameter".into(),
                        ))
                    }
                };
                let rect = Rectangle::new(
                    crate::geometry::Point::new(left, bottom),
                    crate::geometry::Point::new(right, top),
                );
                DestinationType::FitR { rect }
            }
            "FitB" => DestinationType::FitB,
            "FitBH" => {
                if arr.len() < 3 {
                    return Err(PdfError::InvalidStructure(
                        "FitBH destination missing parameter".into(),
                    ));
                }
                let top = match arr.get(2) {
                    Some(Object::Real(v)) => Some(*v),
                    Some(Object::Integer(v)) => Some(*v as f64),
                    Some(Object::Null) => None,
                    _ => {
                        return Err(PdfError::InvalidStructure(
                            "Invalid FitBH top parameter".into(),
                        ))
                    }
                };
                DestinationType::FitBH { top }
            }
            "FitBV" => {
                if arr.len() < 3 {
                    return Err(PdfError::InvalidStructure(
                        "FitBV destination missing parameter".into(),
                    ));
                }
                let left = match arr.get(2) {
                    Some(Object::Real(v)) => Some(*v),
                    Some(Object::Integer(v)) => Some(*v as f64),
                    Some(Object::Null) => None,
                    _ => {
                        return Err(PdfError::InvalidStructure(
                            "Invalid FitBV left parameter".into(),
                        ))
                    }
                };
                DestinationType::FitBV { left }
            }
            _ => {
                return Err(PdfError::InvalidStructure(format!(
                    "Unknown destination type: {type_name}"
                )))
            }
        };

        Ok(Self { page, dest_type })
    }

    /// Convert to PDF array
    pub fn to_array(&self) -> Array {
        let mut arr = Array::new();

        // Add page reference
        match &self.page {
            PageDestination::PageNumber(num) => {
                arr.push(Object::Integer(*num as i64));
            }
            PageDestination::PageRef(id) => {
                arr.push(Object::Reference(*id));
            }
        }

        // Add destination type
        match &self.dest_type {
            DestinationType::XYZ { left, top, zoom } => {
                arr.push(Object::Name("XYZ".to_string()));
                arr.push(left.map(Object::Real).unwrap_or(Object::Null));
                arr.push(top.map(Object::Real).unwrap_or(Object::Null));
                arr.push(zoom.map(Object::Real).unwrap_or(Object::Null));
            }
            DestinationType::Fit => {
                arr.push(Object::Name("Fit".to_string()));
            }
            DestinationType::FitH { top } => {
                arr.push(Object::Name("FitH".to_string()));
                arr.push(top.map(Object::Real).unwrap_or(Object::Null));
            }
            DestinationType::FitV { left } => {
                arr.push(Object::Name("FitV".to_string()));
                arr.push(left.map(Object::Real).unwrap_or(Object::Null));
            }
            DestinationType::FitR { rect } => {
                arr.push(Object::Name("FitR".to_string()));
                arr.push(Object::Real(rect.lower_left.x));
                arr.push(Object::Real(rect.lower_left.y));
                arr.push(Object::Real(rect.upper_right.x));
                arr.push(Object::Real(rect.upper_right.y));
            }
            DestinationType::FitB => {
                arr.push(Object::Name("FitB".to_string()));
            }
            DestinationType::FitBH { top } => {
                arr.push(Object::Name("FitBH".to_string()));
                arr.push(top.map(Object::Real).unwrap_or(Object::Null));
            }
            DestinationType::FitBV { left } => {
                arr.push(Object::Name("FitBV".to_string()));
                arr.push(left.map(Object::Real).unwrap_or(Object::Null));
            }
        }

        arr
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Point;

    #[test]
    fn test_destination_type_debug_clone_partial_eq() {
        let dest_type = DestinationType::XYZ {
            left: Some(10.0),
            top: Some(20.0),
            zoom: None,
        };
        let debug_str = format!("{dest_type:?}");
        assert!(debug_str.contains("XYZ"));

        let cloned = dest_type.clone();
        assert_eq!(dest_type, cloned);

        let different = DestinationType::Fit;
        assert_ne!(dest_type, different);
    }

    #[test]
    fn test_page_destination_variants() {
        let page_num = PageDestination::PageNumber(42);
        let page_ref = PageDestination::PageRef(ObjectId::new(5, 0));

        match page_num {
            PageDestination::PageNumber(n) => assert_eq!(n, 42),
            _ => panic!("Wrong variant"),
        }

        match page_ref {
            PageDestination::PageRef(id) => {
                assert_eq!(id.number(), 5);
                assert_eq!(id.generation(), 0);
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn test_page_destination_debug_clone() {
        let page_dest = PageDestination::PageNumber(10);
        let debug_str = format!("{page_dest:?}");
        assert!(debug_str.contains("PageNumber"));
        assert!(debug_str.contains("10"));

        let cloned = page_dest.clone();
        match cloned {
            PageDestination::PageNumber(n) => assert_eq!(n, 10),
            _ => panic!("Clone failed"),
        }
    }

    #[test]
    fn test_destination_debug_clone() {
        let dest = Destination::fit(PageDestination::PageNumber(3));
        let debug_str = format!("{dest:?}");
        assert!(debug_str.contains("Destination"));
        assert!(debug_str.contains("Fit"));

        let cloned = dest.clone();
        match cloned.page {
            PageDestination::PageNumber(n) => assert_eq!(n, 3),
            _ => panic!("Clone failed"),
        }
    }

    #[test]
    fn test_xyz_destination() {
        let dest = Destination::xyz(
            PageDestination::PageNumber(0),
            Some(100.0),
            Some(200.0),
            Some(1.5),
        );

        match dest.dest_type {
            DestinationType::XYZ { left, top, zoom } => {
                assert_eq!(left, Some(100.0));
                assert_eq!(top, Some(200.0));
                assert_eq!(zoom, Some(1.5));
            }
            _ => panic!("Wrong destination type"),
        }

        let arr = dest.to_array();
        assert_eq!(arr.len(), 5);
        assert_eq!(arr.get(0), Some(&Object::Integer(0)));
        assert_eq!(arr.get(1), Some(&Object::Name("XYZ".to_string())));
        assert_eq!(arr.get(2), Some(&Object::Real(100.0)));
        assert_eq!(arr.get(3), Some(&Object::Real(200.0)));
        assert_eq!(arr.get(4), Some(&Object::Real(1.5)));
    }

    #[test]
    fn test_fit_destination() {
        let dest = Destination::fit(PageDestination::PageNumber(5));

        match dest.dest_type {
            DestinationType::Fit => (),
            _ => panic!("Wrong destination type"),
        }

        let arr = dest.to_array();
        assert_eq!(arr.len(), 2);
        assert_eq!(arr.get(0), Some(&Object::Integer(5)));
        assert_eq!(arr.get(1), Some(&Object::Name("Fit".to_string())));
    }

    #[test]
    fn test_fit_h_destination() {
        let dest = Destination::fit_h(PageDestination::PageNumber(1), Some(150.0));
        match dest.dest_type {
            DestinationType::FitH { top } => assert_eq!(top, Some(150.0)),
            _ => panic!("Wrong destination type"),
        }

        let arr = dest.to_array();
        assert_eq!(arr.len(), 3);
        assert_eq!(arr.get(1), Some(&Object::Name("FitH".to_string())));
        assert_eq!(arr.get(2), Some(&Object::Real(150.0)));

        let dest_none = Destination::fit_h(PageDestination::PageNumber(1), None);
        match dest_none.dest_type {
            DestinationType::FitH { top } => assert!(top.is_none()),
            _ => panic!("Wrong destination type"),
        }
    }

    #[test]
    fn test_fit_v_destination() {
        let dest = Destination::fit_v(PageDestination::PageNumber(2), Some(75.0));
        match dest.dest_type {
            DestinationType::FitV { left } => assert_eq!(left, Some(75.0)),
            _ => panic!("Wrong destination type"),
        }

        let arr = dest.to_array();
        assert_eq!(arr.len(), 3);
        assert_eq!(arr.get(1), Some(&Object::Name("FitV".to_string())));
        assert_eq!(arr.get(2), Some(&Object::Real(75.0)));
    }

    #[test]
    fn test_fit_r_destination() {
        let rect = Rectangle::new(Point::new(50.0, 50.0), Point::new(150.0, 150.0));
        let dest = Destination::fit_r(PageDestination::PageNumber(2), rect);

        match dest.dest_type {
            DestinationType::FitR { rect: r } => {
                assert_eq!(r.lower_left.x, 50.0);
                assert_eq!(r.lower_left.y, 50.0);
                assert_eq!(r.upper_right.x, 150.0);
                assert_eq!(r.upper_right.y, 150.0);
            }
            _ => panic!("Wrong destination type"),
        }

        let arr = dest.to_array();
        assert_eq!(arr.len(), 6);
        assert_eq!(arr.get(1), Some(&Object::Name("FitR".to_string())));
        assert_eq!(arr.get(2), Some(&Object::Real(50.0)));
        assert_eq!(arr.get(3), Some(&Object::Real(50.0)));
        assert_eq!(arr.get(4), Some(&Object::Real(150.0)));
        assert_eq!(arr.get(5), Some(&Object::Real(150.0)));
    }

    #[test]
    fn test_fit_b_destinations() {
        let dest_b = Destination::fit_b(PageDestination::PageNumber(3));
        match dest_b.dest_type {
            DestinationType::FitB => (),
            _ => panic!("Wrong destination type"),
        }

        let arr = dest_b.to_array();
        assert_eq!(arr.len(), 2);
        assert_eq!(arr.get(1), Some(&Object::Name("FitB".to_string())));

        let dest_bh = Destination::fit_bh(PageDestination::PageNumber(4), Some(100.0));
        match dest_bh.dest_type {
            DestinationType::FitBH { top } => assert_eq!(top, Some(100.0)),
            _ => panic!("Wrong destination type"),
        }

        let dest_bv = Destination::fit_bv(PageDestination::PageNumber(5), Some(50.0));
        match dest_bv.dest_type {
            DestinationType::FitBV { left } => assert_eq!(left, Some(50.0)),
            _ => panic!("Wrong destination type"),
        }
    }

    #[test]
    fn test_destination_to_array_page_ref() {
        let page_ref = ObjectId::new(10, 0);
        let dest = Destination::fit(PageDestination::PageRef(page_ref));
        let array = dest.to_array();

        assert_eq!(array.len(), 2);
        assert_eq!(array.get(0), Some(&Object::Reference(page_ref)));
        assert_eq!(array.get(1), Some(&Object::Name("Fit".to_string())));
    }

    #[test]
    fn test_destination_to_array_xyz_with_nulls() {
        let dest = Destination::xyz(PageDestination::PageNumber(0), None, Some(100.0), None);
        let array = dest.to_array();

        assert_eq!(array.len(), 5);
        assert_eq!(array.get(0), Some(&Object::Integer(0)));
        assert_eq!(array.get(1), Some(&Object::Name("XYZ".to_string())));
        assert_eq!(array.get(2), Some(&Object::Null));
        assert_eq!(array.get(3), Some(&Object::Real(100.0)));
        assert_eq!(array.get(4), Some(&Object::Null));
    }

    #[test]
    fn test_destination_to_array_all_types() {
        // Test all destination types
        let destinations = vec![
            Destination::fit(PageDestination::PageNumber(0)),
            Destination::fit_h(PageDestination::PageNumber(1), Some(50.0)),
            Destination::fit_v(PageDestination::PageNumber(2), Some(75.0)),
            Destination::fit_r(
                PageDestination::PageNumber(3),
                Rectangle::new(Point::new(10.0, 20.0), Point::new(100.0, 200.0)),
            ),
            Destination::fit_b(PageDestination::PageNumber(4)),
            Destination::fit_bh(PageDestination::PageNumber(5), Some(150.0)),
            Destination::fit_bv(PageDestination::PageNumber(6), Some(125.0)),
        ];

        let expected_names = ["Fit", "FitH", "FitV", "FitR", "FitB", "FitBH", "FitBV"];
        let expected_lengths = [2, 3, 3, 6, 2, 3, 3];

        for (dest, (expected_name, expected_len)) in destinations
            .iter()
            .zip(expected_names.iter().zip(expected_lengths.iter()))
        {
            let array = dest.to_array();
            assert_eq!(array.len(), *expected_len);
            assert_eq!(array.get(1), Some(&Object::Name(expected_name.to_string())));
        }
    }

    #[test]
    fn test_destination_type_all_variants() {
        let variants = vec![
            DestinationType::XYZ {
                left: Some(1.0),
                top: Some(2.0),
                zoom: Some(3.0),
            },
            DestinationType::Fit,
            DestinationType::FitH { top: Some(4.0) },
            DestinationType::FitV { left: Some(5.0) },
            DestinationType::FitR {
                rect: Rectangle::new(Point::new(0.0, 0.0), Point::new(10.0, 10.0)),
            },
            DestinationType::FitB,
            DestinationType::FitBH { top: Some(6.0) },
            DestinationType::FitBV { left: Some(7.0) },
        ];

        for variant in variants {
            let _ = format!("{variant:?}"); // Test Debug
            let _ = variant.clone(); // Test Clone
        }
    }

    #[test]
    fn test_destination_edge_cases() {
        // Test with page 0
        let dest = Destination::fit(PageDestination::PageNumber(0));
        let array = dest.to_array();
        assert_eq!(array.get(0), Some(&Object::Integer(0)));

        // Test with very high page number
        let dest = Destination::fit(PageDestination::PageNumber(999999));
        let array = dest.to_array();
        assert_eq!(array.get(0), Some(&Object::Integer(999999)));

        // Test with negative coordinates (should work)
        let dest = Destination::xyz(
            PageDestination::PageNumber(0),
            Some(-100.0),
            Some(-200.0),
            Some(0.5),
        );
        let array = dest.to_array();
        assert_eq!(array.get(2), Some(&Object::Real(-100.0)));
        assert_eq!(array.get(3), Some(&Object::Real(-200.0)));
    }

    #[test]
    fn test_destination_from_array() {
        // Test basic from_array parsing
        let mut array = Array::new();
        array.push(Object::Integer(5));
        array.push(Object::Name("XYZ".to_string()));
        array.push(Object::Real(100.0));
        array.push(Object::Real(200.0));
        array.push(Object::Real(1.5));

        let dest = Destination::from_array(&array).expect("Should parse destination");
        match dest.page {
            PageDestination::PageNumber(n) => assert_eq!(n, 5),
            _ => panic!("Wrong page type"),
        }
        match dest.dest_type {
            DestinationType::XYZ { left, top, zoom } => {
                assert_eq!(left, Some(100.0));
                assert_eq!(top, Some(200.0));
                assert_eq!(zoom, Some(1.5));
            }
            _ => panic!("Wrong destination type"),
        }
    }

    #[test]
    fn test_destination_from_array_with_nulls() {
        let mut array = Array::new();
        array.push(Object::Integer(0));
        array.push(Object::Name("XYZ".to_string()));
        array.push(Object::Null);
        array.push(Object::Real(100.0));
        array.push(Object::Null);

        let dest = Destination::from_array(&array).expect("Should parse destination");
        match dest.dest_type {
            DestinationType::XYZ { left, top, zoom } => {
                assert!(left.is_none());
                assert_eq!(top, Some(100.0));
                assert!(zoom.is_none());
            }
            _ => panic!("Wrong destination type"),
        }
    }

    #[test]
    fn test_destination_from_array_with_page_ref() {
        let page_ref = ObjectId::new(10, 0);
        let mut array = Array::new();
        array.push(Object::Reference(page_ref));
        array.push(Object::Name("Fit".to_string()));

        let dest = Destination::from_array(&array).expect("Should parse destination");
        match dest.page {
            PageDestination::PageRef(id) => assert_eq!(id, page_ref),
            _ => panic!("Wrong page type"),
        }
    }

    #[test]
    fn test_destination_from_array_all_types() {
        // Test FitH
        let mut array = Array::new();
        array.push(Object::Integer(1));
        array.push(Object::Name("FitH".to_string()));
        array.push(Object::Real(100.0));
        let dest = Destination::from_array(&array).expect("Should parse FitH");
        assert!(matches!(
            dest.dest_type,
            DestinationType::FitH { top: Some(100.0) }
        ));

        // Test FitV
        let mut array = Array::new();
        array.push(Object::Integer(2));
        array.push(Object::Name("FitV".to_string()));
        array.push(Object::Real(50.0));
        let dest = Destination::from_array(&array).expect("Should parse FitV");
        assert!(matches!(
            dest.dest_type,
            DestinationType::FitV { left: Some(50.0) }
        ));

        // Test FitR
        let mut array = Array::new();
        array.push(Object::Integer(3));
        array.push(Object::Name("FitR".to_string()));
        array.push(Object::Real(10.0));
        array.push(Object::Real(20.0));
        array.push(Object::Real(100.0));
        array.push(Object::Real(200.0));
        let dest = Destination::from_array(&array).expect("Should parse FitR");
        match dest.dest_type {
            DestinationType::FitR { rect } => {
                assert_eq!(rect.lower_left.x, 10.0);
                assert_eq!(rect.lower_left.y, 20.0);
                assert_eq!(rect.upper_right.x, 100.0);
                assert_eq!(rect.upper_right.y, 200.0);
            }
            _ => panic!("Wrong type"),
        }

        // Test FitB
        let mut array = Array::new();
        array.push(Object::Integer(4));
        array.push(Object::Name("FitB".to_string()));
        let dest = Destination::from_array(&array).expect("Should parse FitB");
        assert!(matches!(dest.dest_type, DestinationType::FitB));
    }

    #[test]
    fn test_destination_from_array_errors() {
        // Empty array
        let empty = Array::new();
        assert!(Destination::from_array(&empty).is_err());

        // Missing type
        let mut array = Array::new();
        array.push(Object::Integer(0));
        assert!(Destination::from_array(&array).is_err());

        // Invalid type
        let mut array = Array::new();
        array.push(Object::Integer(0));
        array.push(Object::Name("InvalidType".to_string()));
        assert!(Destination::from_array(&array).is_err());

        // Missing XYZ parameters
        let mut array = Array::new();
        array.push(Object::Integer(0));
        array.push(Object::Name("XYZ".to_string()));
        // Missing left, top, zoom
        assert!(Destination::from_array(&array).is_err());

        // Invalid page reference
        let mut array = Array::new();
        array.push(Object::String("invalid".to_string()));
        array.push(Object::Name("Fit".to_string()));
        assert!(Destination::from_array(&array).is_err());
    }

    #[test]
    fn test_destination_from_array_integer_coordinates() {
        // Test that integer coordinates are properly converted to floats
        let mut array = Array::new();
        array.push(Object::Integer(0));
        array.push(Object::Name("XYZ".to_string()));
        array.push(Object::Integer(100));
        array.push(Object::Integer(200));
        array.push(Object::Integer(2));

        let dest = Destination::from_array(&array).expect("Should parse with integer coords");
        match dest.dest_type {
            DestinationType::XYZ { left, top, zoom } => {
                assert_eq!(left, Some(100.0));
                assert_eq!(top, Some(200.0));
                assert_eq!(zoom, Some(2.0));
            }
            _ => panic!("Wrong type"),
        }
    }

    #[test]
    fn test_destination_roundtrip() {
        // Test that to_array and from_array are inverses
        let original = Destination::xyz(
            PageDestination::PageNumber(42),
            Some(123.45),
            Some(678.90),
            Some(1.25),
        );

        let array = original.to_array();
        let parsed = Destination::from_array(&array).expect("Should parse");

        match parsed.page {
            PageDestination::PageNumber(n) => assert_eq!(n, 42),
            _ => panic!("Wrong page"),
        }

        match parsed.dest_type {
            DestinationType::XYZ { left, top, zoom } => {
                assert_eq!(left, Some(123.45));
                assert_eq!(top, Some(678.90));
                assert_eq!(zoom, Some(1.25));
            }
            _ => panic!("Wrong type"),
        }
    }
}
