//! Property-based tests for core primitive types
//!
//! Tests invariants and properties of fundamental PDF types using proptest
//! to automatically generate test cases and find edge cases.

use oxidize_pdf::graphics::Color;
use oxidize_pdf::objects::{Object, ObjectId};
use proptest::prelude::*;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

// Strategy for generating ObjectIds
prop_compose! {
    fn object_id_strategy()(
        number in 1u32..=999999u32,
        generation in 0u16..=65535u16
    ) -> ObjectId {
        ObjectId::new(number, generation)
    }
}

// Strategy for generating Colors
prop_compose! {
    fn color_strategy()(
        color_type in 0..3usize,
        r in any::<f64>(),
        g in any::<f64>(),
        b in any::<f64>(),
        c in any::<f64>(),
        m in any::<f64>(),
        y in any::<f64>(),
        k in any::<f64>(),
        gray in any::<f64>()
    ) -> Color {
        match color_type {
            0 => Color::rgb(r, g, b),
            1 => Color::gray(gray),
            _ => Color::cmyk(c, m, y, k),
        }
    }
}

// Strategy for generating Objects (simplified)
fn object_strategy() -> impl Strategy<Value = Object> {
    prop_oneof![
        Just(Object::Null),
        any::<bool>().prop_map(Object::Boolean),
        any::<i64>().prop_map(Object::Integer),
        any::<f64>()
            .prop_filter("not NaN", |f| !f.is_nan())
            .prop_map(Object::Real),
        "[a-zA-Z0-9 ]{0,100}".prop_map(Object::String),
        "[a-zA-Z][a-zA-Z0-9]{0,50}".prop_map(Object::Name),
        object_id_strategy().prop_map(Object::Reference),
    ]
}

proptest! {
    #[test]
    fn test_object_id_display_parse_roundtrip(id in object_id_strategy()) {
        // Test that ObjectId can be displayed and the format is correct
        let display = format!("{}", id);
        let expected = format!("{} {} R", id.number(), id.generation());
        prop_assert_eq!(display, expected);
    }

    #[test]
    fn test_object_id_equality_consistency(id1 in object_id_strategy(), id2 in object_id_strategy()) {
        // Test equality is consistent with component equality
        let equal = id1 == id2;
        let components_equal = id1.number() == id2.number() && id1.generation() == id2.generation();
        prop_assert_eq!(equal, components_equal);
    }

    #[test]
    fn test_object_id_hash_consistency(id in object_id_strategy()) {
        // Test that equal ObjectIds have equal hashes
        let id_copy = ObjectId::new(id.number(), id.generation());

        let mut hasher1 = DefaultHasher::new();
        id.hash(&mut hasher1);
        let hash1 = hasher1.finish();

        let mut hasher2 = DefaultHasher::new();
        id_copy.hash(&mut hasher2);
        let hash2 = hasher2.finish();

        prop_assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_color_rgb_clamping(r in any::<f64>(), g in any::<f64>(), b in any::<f64>()) {
        let color = Color::rgb(r, g, b);

        // Verify all components are clamped to [0.0, 1.0]
        match color {
            Color::Rgb(r_clamped, g_clamped, b_clamped) => {
                prop_assert!(r_clamped >= 0.0 && r_clamped <= 1.0);
                prop_assert!(g_clamped >= 0.0 && g_clamped <= 1.0);
                prop_assert!(b_clamped >= 0.0 && b_clamped <= 1.0);

                // Verify clamping logic
                prop_assert_eq!(r_clamped, r.clamp(0.0, 1.0));
                prop_assert_eq!(g_clamped, g.clamp(0.0, 1.0));
                prop_assert_eq!(b_clamped, b.clamp(0.0, 1.0));
            }
            _ => prop_assert!(false, "Expected RGB color"),
        }
    }

    #[test]
    fn test_color_gray_clamping(value in any::<f64>()) {
        let color = Color::gray(value);

        match color {
            Color::Gray(clamped) => {
                prop_assert!(clamped >= 0.0 && clamped <= 1.0);
                prop_assert_eq!(clamped, value.clamp(0.0, 1.0));
            }
            _ => prop_assert!(false, "Expected Gray color"),
        }
    }

    #[test]
    fn test_color_cmyk_clamping(c in any::<f64>(), m in any::<f64>(), y in any::<f64>(), k in any::<f64>()) {
        let color = Color::cmyk(c, m, y, k);

        match color {
            Color::Cmyk(c_clamped, m_clamped, y_clamped, k_clamped) => {
                prop_assert!(c_clamped >= 0.0 && c_clamped <= 1.0);
                prop_assert!(m_clamped >= 0.0 && m_clamped <= 1.0);
                prop_assert!(y_clamped >= 0.0 && y_clamped <= 1.0);
                prop_assert!(k_clamped >= 0.0 && k_clamped <= 1.0);

                prop_assert_eq!(c_clamped, c.clamp(0.0, 1.0));
                prop_assert_eq!(m_clamped, m.clamp(0.0, 1.0));
                prop_assert_eq!(y_clamped, y.clamp(0.0, 1.0));
                prop_assert_eq!(k_clamped, k.clamp(0.0, 1.0));
            }
            _ => prop_assert!(false, "Expected CMYK color"),
        }
    }

    #[test]
    fn test_color_equality_reflexive(color in color_strategy()) {
        // Test reflexivity: a == a
        prop_assert_eq!(color, color);
    }

    #[test]
    fn test_object_null_checking(obj in object_strategy()) {
        // Test that is_null() is consistent with Null variant
        let is_null = obj.is_null();
        let is_null_variant = matches!(obj, Object::Null);
        prop_assert_eq!(is_null, is_null_variant);
    }

    #[test]
    fn test_object_boolean_conversion(b in any::<bool>()) {
        let obj = Object::Boolean(b);
        prop_assert_eq!(obj.as_bool(), Some(b));

        // Test that non-boolean objects return None
        let null_obj = Object::Null;
        prop_assert_eq!(null_obj.as_bool(), None);
    }

    #[test]
    fn test_object_integer_conversion(i in any::<i64>()) {
        let obj = Object::Integer(i);
        prop_assert_eq!(obj.as_integer(), Some(i));

        // Test that non-integer objects return None
        let null_obj = Object::Null;
        prop_assert_eq!(null_obj.as_integer(), None);
    }

    #[test]
    fn test_object_reference_consistency(id in object_id_strategy()) {
        let obj = Object::Reference(id);

        // Verify we can extract the reference
        match obj {
            Object::Reference(extracted_id) => {
                prop_assert_eq!(extracted_id, id);
            }
            _ => prop_assert!(false, "Expected Reference object"),
        }
    }

    #[test]
    fn test_object_string_preservation(s in "[a-zA-Z0-9 !@#$%^&*()]{0,200}") {
        let obj = Object::String(s.clone());

        match obj {
            Object::String(extracted) => {
                prop_assert_eq!(extracted, s);
            }
            _ => prop_assert!(false, "Expected String object"),
        }
    }

    #[test]
    fn test_object_name_validity(name in "[a-zA-Z][a-zA-Z0-9._-]{0,100}") {
        let obj = Object::Name(name.clone());

        match obj {
            Object::Name(extracted) => {
                prop_assert_eq!(&extracted, &name);
                // Names should not be empty in valid PDFs
                prop_assert!(!extracted.is_empty());
            }
            _ => prop_assert!(false, "Expected Name object"),
        }
    }

    #[test]
    fn test_object_real_finite(f in any::<f64>().prop_filter("finite", |x| x.is_finite())) {
        let obj = Object::Real(f);

        match obj {
            Object::Real(extracted) => {
                prop_assert_eq!(extracted, f);
                prop_assert!(extracted.is_finite());
            }
            _ => prop_assert!(false, "Expected Real object"),
        }
    }
}

// Regression tests for specific edge cases found by property testing
#[cfg(test)]
mod regression_tests {
    use super::*;

    #[test]
    fn test_color_nan_handling() {
        // Ensure NaN values are handled gracefully
        let color = Color::rgb(f64::NAN, 0.5, f64::INFINITY);
        match color {
            Color::Rgb(r, g, b) => {
                assert!(r.is_nan()); // NaN remains NaN after clamp
                assert_eq!(g, 0.5);
                assert_eq!(b, 1.0); // Infinity should clamp to 1.0
            }
            _ => panic!("Expected RGB color"),
        }
    }

    #[test]
    fn test_object_id_zero_generation() {
        let id = ObjectId::new(42, 0);
        assert_eq!(format!("{}", id), "42 0 R");
    }

    #[test]
    fn test_object_id_max_values() {
        let id = ObjectId::new(u32::MAX, u16::MAX);
        assert_eq!(id.number(), u32::MAX);
        assert_eq!(id.generation(), u16::MAX);
    }
}
