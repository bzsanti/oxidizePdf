//! Form XObjects for reusable graphics content
//!
//! Implements ISO 32000-1 Section 8.10 (Form XObjects)
//! Form XObjects are self-contained descriptions of graphics objects that can be
//! painted multiple times on different pages or at different locations.

use crate::error::Result;
use crate::geometry::Rectangle;
use crate::objects::{Dictionary, Object, ObjectId, Stream};
use std::collections::HashMap;

/// Form XObject - reusable graphics content
#[derive(Debug, Clone)]
pub struct FormXObject {
    /// Bounding box of the form
    pub bbox: Rectangle,
    /// Optional transformation matrix
    pub matrix: Option<[f64; 6]>,
    /// Resources used by the form
    pub resources: Dictionary,
    /// Graphics operations content
    pub content: Vec<u8>,
    /// Optional group attributes for transparency
    pub group: Option<TransparencyGroup>,
    /// Optional reference to external XObject
    pub reference: Option<ObjectId>,
    /// Metadata
    pub metadata: Option<Dictionary>,
}

/// Transparency group attributes
#[derive(Debug, Clone)]
pub struct TransparencyGroup {
    /// Color space for group
    pub color_space: String,
    /// Whether group is isolated
    pub isolated: bool,
    /// Whether group is knockout
    pub knockout: bool,
}

impl Default for TransparencyGroup {
    fn default() -> Self {
        Self {
            color_space: "DeviceRGB".to_string(),
            isolated: false,
            knockout: false,
        }
    }
}

impl FormXObject {
    /// Create a new form XObject
    pub fn new(bbox: Rectangle) -> Self {
        Self {
            bbox,
            matrix: None,
            resources: Dictionary::new(),
            content: Vec::new(),
            group: None,
            reference: None,
            metadata: None,
        }
    }

    /// Set transformation matrix
    pub fn with_matrix(mut self, matrix: [f64; 6]) -> Self {
        self.matrix = Some(matrix);
        self
    }

    /// Set resources
    pub fn with_resources(mut self, resources: Dictionary) -> Self {
        self.resources = resources;
        self
    }

    /// Set content stream
    pub fn with_content(mut self, content: Vec<u8>) -> Self {
        self.content = content;
        self
    }

    /// Add transparency group
    pub fn with_transparency_group(mut self, group: TransparencyGroup) -> Self {
        self.group = Some(group);
        self
    }

    /// Set metadata
    pub fn with_metadata(mut self, metadata: Dictionary) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Create a form XObject from graphics operations
    pub fn from_graphics_ops(bbox: Rectangle, ops: &str) -> Self {
        Self {
            bbox,
            matrix: None,
            resources: Dictionary::new(),
            content: ops.as_bytes().to_vec(),
            group: None,
            reference: None,
            metadata: None,
        }
    }

    /// Convert to PDF stream object
    pub fn to_stream(&self) -> Result<Stream> {
        let mut dict = Dictionary::new();

        // Required entries
        dict.set("Type", Object::Name("XObject".to_string()));
        dict.set("Subtype", Object::Name("Form".to_string()));

        // BBox is required
        dict.set(
            "BBox",
            Object::Array(vec![
                Object::Real(self.bbox.lower_left.x),
                Object::Real(self.bbox.lower_left.y),
                Object::Real(self.bbox.upper_right.x),
                Object::Real(self.bbox.upper_right.y),
            ]),
        );

        // Optional matrix
        if let Some(matrix) = &self.matrix {
            dict.set(
                "Matrix",
                Object::Array(matrix.iter().map(|&v| Object::Real(v)).collect()),
            );
        }

        // Resources
        dict.set("Resources", Object::Dictionary(self.resources.clone()));

        // Transparency group if present
        if let Some(group) = &self.group {
            let mut group_dict = Dictionary::new();
            group_dict.set("Type", Object::Name("Group".to_string()));
            group_dict.set("S", Object::Name("Transparency".to_string()));
            group_dict.set("CS", Object::Name(group.color_space.clone()));

            if group.isolated {
                group_dict.set("I", Object::Boolean(true));
            }
            if group.knockout {
                group_dict.set("K", Object::Boolean(true));
            }

            dict.set("Group", Object::Dictionary(group_dict));
        }

        // Optional metadata
        if let Some(metadata) = &self.metadata {
            dict.set("Metadata", Object::Dictionary(metadata.clone()));
        }

        Ok(Stream::with_dictionary(dict, self.content.clone()))
    }

    /// Get the bounding box
    pub fn get_bbox(&self) -> &Rectangle {
        &self.bbox
    }

    /// Check if form has transparency
    pub fn has_transparency(&self) -> bool {
        self.group.is_some()
    }
}

/// Builder for creating form XObjects with graphics operations
pub struct FormXObjectBuilder {
    bbox: Rectangle,
    matrix: Option<[f64; 6]>,
    resources: Dictionary,
    operations: Vec<String>,
    group: Option<TransparencyGroup>,
}

impl FormXObjectBuilder {
    /// Create a new builder
    pub fn new(bbox: Rectangle) -> Self {
        Self {
            bbox,
            matrix: None,
            resources: Dictionary::new(),
            operations: Vec::new(),
            group: None,
        }
    }

    /// Set transformation matrix
    pub fn matrix(mut self, matrix: [f64; 6]) -> Self {
        self.matrix = Some(matrix);
        self
    }

    /// Add a graphics operation
    pub fn add_operation(mut self, op: impl Into<String>) -> Self {
        self.operations.push(op.into());
        self
    }

    /// Draw a rectangle
    pub fn rectangle(mut self, x: f64, y: f64, width: f64, height: f64) -> Self {
        self.operations
            .push(format!("{} {} {} {} re", x, y, width, height));
        self
    }

    /// Move to point
    pub fn move_to(mut self, x: f64, y: f64) -> Self {
        self.operations.push(format!("{} {} m", x, y));
        self
    }

    /// Line to point
    pub fn line_to(mut self, x: f64, y: f64) -> Self {
        self.operations.push(format!("{} {} l", x, y));
        self
    }

    /// Set fill color (RGB)
    pub fn fill_color(mut self, r: f64, g: f64, b: f64) -> Self {
        self.operations.push(format!("{} {} {} rg", r, g, b));
        self
    }

    /// Set stroke color (RGB)
    pub fn stroke_color(mut self, r: f64, g: f64, b: f64) -> Self {
        self.operations.push(format!("{} {} {} RG", r, g, b));
        self
    }

    /// Fill path
    pub fn fill(mut self) -> Self {
        self.operations.push("f".to_string());
        self
    }

    /// Stroke path
    pub fn stroke(mut self) -> Self {
        self.operations.push("S".to_string());
        self
    }

    /// Fill and stroke path
    pub fn fill_stroke(mut self) -> Self {
        self.operations.push("B".to_string());
        self
    }

    /// Save graphics state
    pub fn save_state(mut self) -> Self {
        self.operations.push("q".to_string());
        self
    }

    /// Restore graphics state
    pub fn restore_state(mut self) -> Self {
        self.operations.push("Q".to_string());
        self
    }

    /// Add transparency group
    pub fn transparency_group(mut self, isolated: bool, knockout: bool) -> Self {
        self.group = Some(TransparencyGroup {
            color_space: "DeviceRGB".to_string(),
            isolated,
            knockout,
        });
        self
    }

    /// Build the form XObject
    pub fn build(self) -> FormXObject {
        let content = self.operations.join("\n").into_bytes();

        FormXObject {
            bbox: self.bbox,
            matrix: self.matrix,
            resources: self.resources,
            content,
            group: self.group,
            reference: None,
            metadata: None,
        }
    }
}

/// Template form XObject for common shapes
pub struct FormTemplates;

impl FormTemplates {
    /// Create a checkmark form
    pub fn checkmark(size: f64) -> FormXObject {
        let bbox = Rectangle::from_position_and_size(0.0, 0.0, size, size);

        FormXObjectBuilder::new(bbox)
            .stroke_color(0.0, 0.5, 0.0)
            .move_to(size * 0.2, size * 0.5)
            .line_to(size * 0.4, size * 0.3)
            .line_to(size * 0.8, size * 0.7)
            .stroke()
            .build()
    }

    /// Create a cross/X form
    pub fn cross(size: f64) -> FormXObject {
        let bbox = Rectangle::from_position_and_size(0.0, 0.0, size, size);

        FormXObjectBuilder::new(bbox)
            .stroke_color(0.8, 0.0, 0.0)
            .move_to(size * 0.2, size * 0.2)
            .line_to(size * 0.8, size * 0.8)
            .move_to(size * 0.2, size * 0.8)
            .line_to(size * 0.8, size * 0.2)
            .stroke()
            .build()
    }

    /// Create a circle form
    pub fn circle(radius: f64, filled: bool) -> FormXObject {
        let size = radius * 2.0;
        let bbox = Rectangle::from_position_and_size(0.0, 0.0, size, size);

        // Approximate circle with Bézier curves
        let k = 0.5522847498; // Magic constant for circle approximation
        let cp = radius * k; // Control point offset

        let mut builder = FormXObjectBuilder::new(bbox);

        if filled {
            builder = builder.fill_color(0.0, 0.0, 1.0);
        } else {
            builder = builder.stroke_color(0.0, 0.0, 1.0);
        }

        // Move to right point
        builder = builder
            .move_to(size, radius)
            .add_operation(format!(
                "{} {} {} {} {} {} c", // Top right quadrant
                size,
                radius + cp,
                radius + cp,
                size,
                radius,
                size
            ))
            .add_operation(format!(
                "{} {} {} {} {} {} c", // Top left quadrant
                radius - cp,
                size,
                0.0,
                radius + cp,
                0.0,
                radius
            ))
            .add_operation(format!(
                "{} {} {} {} {} {} c", // Bottom left quadrant
                0.0,
                radius - cp,
                radius - cp,
                0.0,
                radius,
                0.0
            ))
            .add_operation(format!(
                "{} {} {} {} {} {} c", // Bottom right quadrant
                radius + cp,
                0.0,
                size,
                radius - cp,
                size,
                radius
            ));

        if filled {
            builder.fill()
        } else {
            builder.stroke()
        }
        .build()
    }

    /// Create a star form
    pub fn star(size: f64, points: usize) -> FormXObject {
        let bbox = Rectangle::from_position_and_size(0.0, 0.0, size, size);
        let center = size / 2.0;
        let outer_radius = size / 2.0 * 0.9;
        let inner_radius = outer_radius * 0.4;

        let mut builder = FormXObjectBuilder::new(bbox).fill_color(1.0, 0.8, 0.0);

        let angle_step = std::f64::consts::PI * 2.0 / (points * 2) as f64;

        for i in 0..(points * 2) {
            let angle = i as f64 * angle_step - std::f64::consts::PI / 2.0;
            let radius = if i % 2 == 0 {
                outer_radius
            } else {
                inner_radius
            };
            let x = center + radius * angle.cos();
            let y = center + radius * angle.sin();

            if i == 0 {
                builder = builder.move_to(x, y);
            } else {
                builder = builder.line_to(x, y);
            }
        }

        builder.add_operation("h".to_string()).fill().build()
    }

    /// Create a logo placeholder form
    pub fn logo_placeholder(width: f64, height: f64) -> FormXObject {
        let bbox = Rectangle::from_position_and_size(0.0, 0.0, width, height);

        FormXObjectBuilder::new(bbox)
            .save_state()
            // Border
            .stroke_color(0.5, 0.5, 0.5)
            .rectangle(1.0, 1.0, width - 2.0, height - 2.0)
            .stroke()
            // Diagonal lines
            .move_to(0.0, 0.0)
            .line_to(width, height)
            .move_to(0.0, height)
            .line_to(width, 0.0)
            .stroke()
            .restore_state()
            .build()
    }
}

/// Manager for form XObjects in a document
#[derive(Debug, Clone)]
pub struct FormXObjectManager {
    forms: HashMap<String, FormXObject>,
    next_id: usize,
}

impl Default for FormXObjectManager {
    fn default() -> Self {
        Self {
            forms: HashMap::new(),
            next_id: 1,
        }
    }
}

impl FormXObjectManager {
    /// Create a new manager
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a form XObject
    pub fn add_form(&mut self, name: Option<String>, form: FormXObject) -> String {
        let name = name.unwrap_or_else(|| {
            let id = format!("Fm{}", self.next_id);
            self.next_id += 1;
            id
        });

        self.forms.insert(name.clone(), form);
        name
    }

    /// Get a form XObject
    pub fn get_form(&self, name: &str) -> Option<&FormXObject> {
        self.forms.get(name)
    }

    /// Get all forms
    pub fn get_all_forms(&self) -> &HashMap<String, FormXObject> {
        &self.forms
    }

    /// Remove a form
    pub fn remove_form(&mut self, name: &str) -> Option<FormXObject> {
        self.forms.remove(name)
    }

    /// Clear all forms
    pub fn clear(&mut self) {
        self.forms.clear();
        self.next_id = 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Point;

    #[test]
    fn test_form_xobject_creation() {
        let bbox = Rectangle::new(Point::new(0.0, 0.0), Point::new(100.0, 100.0));
        let form = FormXObject::new(bbox.clone());

        assert_eq!(form.bbox, bbox);
        assert!(form.matrix.is_none());
        assert!(form.content.is_empty());
    }

    #[test]
    fn test_form_xobject_with_matrix() {
        let bbox = Rectangle::new(Point::new(0.0, 0.0), Point::new(50.0, 50.0));
        let matrix = [2.0, 0.0, 0.0, 2.0, 10.0, 10.0]; // Scale by 2, translate by (10, 10)

        let form = FormXObject::new(bbox).with_matrix(matrix);

        assert_eq!(form.matrix, Some(matrix));
    }

    #[test]
    fn test_form_xobject_from_graphics_ops() {
        let bbox = Rectangle::new(Point::new(0.0, 0.0), Point::new(100.0, 100.0));
        let ops = "0 0 100 100 re\nf";

        let form = FormXObject::from_graphics_ops(bbox.clone(), ops);

        assert_eq!(form.bbox, bbox);
        assert_eq!(form.content, ops.as_bytes());
    }

    #[test]
    fn test_form_xobject_to_stream() {
        let bbox = Rectangle::new(Point::new(0.0, 0.0), Point::new(200.0, 100.0));
        let form = FormXObject::new(bbox).with_content(b"q\n1 0 0 1 0 0 cm\nQ".to_vec());

        let stream = form.to_stream();
        assert!(stream.is_ok());

        let stream = stream.unwrap();
        let dict = stream.dictionary();

        assert_eq!(dict.get("Type"), Some(&Object::Name("XObject".to_string())));
        assert_eq!(dict.get("Subtype"), Some(&Object::Name("Form".to_string())));
        assert!(dict.get("BBox").is_some());
    }

    #[test]
    fn test_transparency_group() {
        let bbox = Rectangle::new(Point::new(0.0, 0.0), Point::new(100.0, 100.0));
        let group = TransparencyGroup {
            color_space: "DeviceCMYK".to_string(),
            isolated: true,
            knockout: false,
        };

        let form = FormXObject::new(bbox).with_transparency_group(group);

        assert!(form.has_transparency());
        assert_eq!(form.group.as_ref().unwrap().color_space, "DeviceCMYK");
        assert!(form.group.as_ref().unwrap().isolated);
    }

    #[test]
    fn test_form_builder_basic() {
        let bbox = Rectangle::new(Point::new(0.0, 0.0), Point::new(100.0, 100.0));

        let form = FormXObjectBuilder::new(bbox)
            .fill_color(1.0, 0.0, 0.0)
            .rectangle(10.0, 10.0, 80.0, 80.0)
            .fill()
            .build();

        let content = String::from_utf8(form.content.clone()).unwrap();
        assert!(content.contains("1 0 0 rg"));
        assert!(content.contains("10 10 80 80 re"));
        assert!(content.contains("f"));
    }

    #[test]
    fn test_form_builder_complex() {
        let bbox = Rectangle::new(Point::new(0.0, 0.0), Point::new(200.0, 200.0));

        let form = FormXObjectBuilder::new(bbox)
            .save_state()
            .stroke_color(0.0, 0.0, 1.0)
            .move_to(50.0, 50.0)
            .line_to(150.0, 150.0)
            .stroke()
            .restore_state()
            .transparency_group(true, false)
            .build();

        let content = String::from_utf8(form.content.clone()).unwrap();
        assert!(content.contains("q"));
        assert!(content.contains("Q"));
        assert!(content.contains("0 0 1 RG"));
        assert!(form.has_transparency());
    }

    #[test]
    fn test_form_templates_checkmark() {
        let form = FormTemplates::checkmark(20.0);

        assert_eq!(form.bbox.width(), 20.0);
        assert_eq!(form.bbox.height(), 20.0);

        let content = String::from_utf8(form.content.clone()).unwrap();
        assert!(content.contains("0 0.5 0 RG")); // Green color
    }

    #[test]
    fn test_form_templates_cross() {
        let form = FormTemplates::cross(30.0);

        assert_eq!(form.bbox.width(), 30.0);

        let content = String::from_utf8(form.content.clone()).unwrap();
        assert!(content.contains("0.8 0 0 RG")); // Red color
    }

    #[test]
    fn test_form_templates_circle() {
        let filled_circle = FormTemplates::circle(25.0, true);
        let stroked_circle = FormTemplates::circle(25.0, false);

        assert_eq!(filled_circle.bbox.width(), 50.0);
        assert_eq!(stroked_circle.bbox.width(), 50.0);

        let filled_content = String::from_utf8(filled_circle.content.clone()).unwrap();
        let stroked_content = String::from_utf8(stroked_circle.content.clone()).unwrap();

        assert!(filled_content.contains("f")); // Fill
        assert!(stroked_content.contains("S")); // Stroke
    }

    #[test]
    fn test_form_templates_star() {
        let star = FormTemplates::star(100.0, 5);

        assert_eq!(star.bbox.width(), 100.0);

        let content = String::from_utf8(star.content.clone()).unwrap();
        assert!(content.contains("1 0.8 0 rg")); // Gold color
    }

    #[test]
    fn test_form_xobject_manager() {
        let mut manager = FormXObjectManager::new();

        let bbox = Rectangle::new(Point::new(0.0, 0.0), Point::new(50.0, 50.0));
        let form1 = FormXObject::new(bbox.clone());
        let form2 = FormXObject::new(bbox);

        let name1 = manager.add_form(Some("custom".to_string()), form1);
        let name2 = manager.add_form(None, form2);

        assert_eq!(name1, "custom");
        assert!(name2.starts_with("Fm"));

        assert!(manager.get_form("custom").is_some());
        assert!(manager.get_form(&name2).is_some());
        assert!(manager.get_form("nonexistent").is_none());
    }

    #[test]
    fn test_form_xobject_manager_operations() {
        let mut manager = FormXObjectManager::new();

        let bbox = Rectangle::new(Point::new(0.0, 0.0), Point::new(100.0, 100.0));
        let form = FormXObject::new(bbox);

        manager.add_form(Some("test".to_string()), form.clone());
        assert_eq!(manager.get_all_forms().len(), 1);

        let removed = manager.remove_form("test");
        assert!(removed.is_some());
        assert_eq!(manager.get_all_forms().len(), 0);

        manager.add_form(None, form);
        manager.clear();
        assert_eq!(manager.get_all_forms().len(), 0);
    }
}
