//! Document outline (bookmarks) according to ISO 32000-1 Section 12.3.3

use crate::graphics::Color;
use crate::objects::{Array, Dictionary, Object, ObjectId};
use crate::structure::destination::Destination;
use std::collections::VecDeque;

/// Outline item flags
#[derive(Debug, Clone, Copy, Default)]
pub struct OutlineFlags {
    /// Italic text
    pub italic: bool,
    /// Bold text
    pub bold: bool,
}

impl OutlineFlags {
    /// Convert to integer flags
    #[allow(clippy::wrong_self_convention)]
    pub fn to_int(&self) -> i64 {
        let mut flags = 0;
        if self.italic {
            flags |= 1;
        }
        if self.bold {
            flags |= 2;
        }
        flags
    }
}

/// Outline item (bookmark)
#[derive(Debug, Clone)]
pub struct OutlineItem {
    /// Item title
    pub title: String,
    /// Destination
    pub destination: Option<Destination>,
    /// Child items
    pub children: Vec<OutlineItem>,
    /// Text color
    pub color: Option<Color>,
    /// Text style flags
    pub flags: OutlineFlags,
    /// Whether item is open by default
    pub open: bool,
}

impl OutlineItem {
    /// Create new outline item
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            destination: None,
            children: Vec::new(),
            color: None,
            flags: OutlineFlags::default(),
            open: true,
        }
    }

    /// Set destination
    pub fn with_destination(mut self, dest: Destination) -> Self {
        self.destination = Some(dest);
        self
    }

    /// Add child item
    pub fn add_child(&mut self, child: OutlineItem) {
        self.children.push(child);
    }

    /// Set color
    pub fn with_color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// Set bold
    pub fn bold(mut self) -> Self {
        self.flags.bold = true;
        self
    }

    /// Set italic
    pub fn italic(mut self) -> Self {
        self.flags.italic = true;
        self
    }

    /// Set closed by default
    pub fn closed(mut self) -> Self {
        self.open = false;
        self
    }

    /// Count total items in subtree
    pub fn count_all(&self) -> i64 {
        let mut count = 1; // Self
        for child in &self.children {
            count += child.count_all();
        }
        count
    }

    /// Count visible items (respecting open/closed state)
    pub fn count_visible(&self) -> i64 {
        let mut count = 1; // Self
        if self.open {
            for child in &self.children {
                count += child.count_visible();
            }
        }
        count
    }
}

/// Outline tree structure
pub struct OutlineTree {
    /// Root items
    pub items: Vec<OutlineItem>,
}

impl Default for OutlineTree {
    fn default() -> Self {
        Self::new()
    }
}

impl OutlineTree {
    /// Create new outline tree
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    /// Add root item
    pub fn add_item(&mut self, item: OutlineItem) {
        self.items.push(item);
    }

    /// Get total item count
    pub fn total_count(&self) -> i64 {
        self.items.iter().map(|item| item.count_all()).sum()
    }

    /// Get visible item count
    pub fn visible_count(&self) -> i64 {
        self.items.iter().map(|item| item.count_visible()).sum()
    }
}

/// Outline builder for creating outline hierarchy
pub struct OutlineBuilder {
    /// Current outline tree
    tree: OutlineTree,
    /// Stack for building hierarchy
    stack: VecDeque<OutlineItem>,
}

impl Default for OutlineBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl OutlineBuilder {
    /// Create new builder
    pub fn new() -> Self {
        Self {
            tree: OutlineTree::new(),
            stack: VecDeque::new(),
        }
    }

    /// Add item at current level
    pub fn add_item(&mut self, item: OutlineItem) {
        if let Some(parent) = self.stack.back_mut() {
            parent.add_child(item);
        } else {
            self.tree.add_item(item);
        }
    }

    /// Push item and make it current parent
    pub fn push_item(&mut self, item: OutlineItem) {
        self.stack.push_back(item);
    }

    /// Pop current parent and add to tree
    pub fn pop_item(&mut self) {
        if let Some(item) = self.stack.pop_back() {
            if let Some(parent) = self.stack.back_mut() {
                parent.add_child(item);
            } else {
                self.tree.add_item(item);
            }
        }
    }

    /// Build the outline tree
    pub fn build(mut self) -> OutlineTree {
        // Pop any remaining items
        while !self.stack.is_empty() {
            self.pop_item();
        }
        self.tree
    }
}

/// Convert outline item to dictionary (for PDF generation)
pub fn outline_item_to_dict(
    item: &OutlineItem,
    parent_ref: ObjectId,
    first_ref: Option<ObjectId>,
    last_ref: Option<ObjectId>,
    prev_ref: Option<ObjectId>,
    next_ref: Option<ObjectId>,
) -> Dictionary {
    let mut dict = Dictionary::new();

    // Title
    dict.set("Title", Object::String(item.title.clone()));

    // Parent
    dict.set("Parent", Object::Reference(parent_ref));

    // Siblings
    if let Some(prev) = prev_ref {
        dict.set("Prev", Object::Reference(prev));
    }
    if let Some(next) = next_ref {
        dict.set("Next", Object::Reference(next));
    }

    // Children
    if !item.children.is_empty() {
        if let Some(first) = first_ref {
            dict.set("First", Object::Reference(first));
        }
        if let Some(last) = last_ref {
            dict.set("Last", Object::Reference(last));
        }

        // Count (negative if closed)
        let count = if item.open {
            item.count_visible() - 1 // Exclude self
        } else {
            item.count_all() - 1 // For closed items, count all children
        };
        dict.set(
            "Count",
            Object::Integer(if item.open { count } else { -count }),
        );
    }

    // Destination
    if let Some(dest) = &item.destination {
        dict.set("Dest", Object::Array(dest.to_array().into()));
    }

    // Color
    if let Some(color) = &item.color {
        let color_array = match color {
            Color::Rgb(r, g, b) => {
                Array::from(vec![Object::Real(*r), Object::Real(*g), Object::Real(*b)])
            }
            Color::Gray(g) => {
                Array::from(vec![Object::Real(*g), Object::Real(*g), Object::Real(*g)])
            }
            Color::Cmyk(c, m, y, k) => {
                // Convert CMYK to RGB approximation for outline color
                let r = (1.0 - c) * (1.0 - k);
                let g = (1.0 - m) * (1.0 - k);
                let b = (1.0 - y) * (1.0 - k);
                Array::from(vec![Object::Real(r), Object::Real(g), Object::Real(b)])
            }
        };
        dict.set("C", Object::Array(color_array.into()));
    }

    // Flags
    let flags = item.flags.to_int();
    if flags != 0 {
        dict.set("F", Object::Integer(flags));
    }

    dict
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::structure::destination::PageDestination;

    #[test]
    fn test_outline_item_new() {
        let item = OutlineItem::new("Chapter 1");
        assert_eq!(item.title, "Chapter 1");
        assert!(item.destination.is_none());
        assert!(item.children.is_empty());
        assert!(item.color.is_none());
        assert!(!item.flags.bold);
        assert!(!item.flags.italic);
        assert!(item.open);
    }

    #[test]
    fn test_outline_item_builder() {
        let dest = Destination::fit(PageDestination::PageNumber(0));
        let item = OutlineItem::new("Bold Chapter")
            .with_destination(dest)
            .with_color(Color::rgb(1.0, 0.0, 0.0))
            .bold()
            .closed();

        assert!(item.destination.is_some());
        assert!(item.color.is_some());
        assert!(item.flags.bold);
        assert!(!item.open);
    }

    #[test]
    fn test_outline_hierarchy() {
        let mut chapter1 = OutlineItem::new("Chapter 1");
        chapter1.add_child(OutlineItem::new("Section 1.1"));
        chapter1.add_child(OutlineItem::new("Section 1.2"));

        assert_eq!(chapter1.children.len(), 2);
        assert_eq!(chapter1.count_all(), 3); // Chapter + 2 sections
    }

    #[test]
    fn test_outline_count() {
        let mut root = OutlineItem::new("Book");

        let mut ch1 = OutlineItem::new("Chapter 1");
        ch1.add_child(OutlineItem::new("Section 1.1"));
        ch1.add_child(OutlineItem::new("Section 1.2"));

        let mut ch2 = OutlineItem::new("Chapter 2").closed();
        ch2.add_child(OutlineItem::new("Section 2.1"));

        root.add_child(ch1);
        root.add_child(ch2);

        assert_eq!(root.count_all(), 6); // Book + 2 chapters + 3 sections
        assert_eq!(root.count_visible(), 5); // Ch2's child hidden
    }

    #[test]
    fn test_outline_builder() {
        let mut builder = OutlineBuilder::new();

        // Add root items
        builder.add_item(OutlineItem::new("Preface"));

        // Add chapter with sections
        builder.push_item(OutlineItem::new("Chapter 1"));
        builder.add_item(OutlineItem::new("Section 1.1"));
        builder.add_item(OutlineItem::new("Section 1.2"));
        builder.pop_item();

        builder.add_item(OutlineItem::new("Chapter 2"));

        let tree = builder.build();
        assert_eq!(tree.items.len(), 3); // Preface, Ch1, Ch2
        assert_eq!(tree.total_count(), 5); // All items
    }

    #[test]
    fn test_outline_flags() {
        let flags = OutlineFlags {
            italic: true,
            bold: true,
        };
        assert_eq!(flags.to_int(), 3);

        let flags2 = OutlineFlags {
            italic: true,
            bold: false,
        };
        assert_eq!(flags2.to_int(), 1);

        let flags3 = OutlineFlags::default();
        assert_eq!(flags3.to_int(), 0);
    }

    #[test]
    fn test_outline_flags_debug_clone_default() {
        let flags = OutlineFlags {
            italic: true,
            bold: false,
        };
        let debug_str = format!("{flags:?}");
        assert!(debug_str.contains("OutlineFlags"));
        assert!(debug_str.contains("italic: true"));
        assert!(debug_str.contains("bold: false"));

        let cloned = flags;
        assert_eq!(cloned.italic, flags.italic);
        assert_eq!(cloned.bold, flags.bold);

        let default_flags = OutlineFlags::default();
        assert!(!default_flags.italic);
        assert!(!default_flags.bold);
    }

    #[test]
    fn test_outline_item_italic() {
        let item = OutlineItem::new("Italic Text").italic();
        assert!(item.flags.italic);
        assert!(!item.flags.bold);
    }

    #[test]
    fn test_outline_item_bold_italic() {
        let item = OutlineItem::new("Bold Italic").bold().italic();
        assert!(item.flags.italic);
        assert!(item.flags.bold);
        assert_eq!(item.flags.to_int(), 3);
    }

    #[test]
    fn test_outline_item_with_complex_destination() {
        use crate::geometry::{Point, Rectangle};

        let dest = Destination::fit_r(
            PageDestination::PageNumber(5),
            Rectangle::new(Point::new(100.0, 200.0), Point::new(300.0, 400.0)),
        );
        let item = OutlineItem::new("Complex Destination").with_destination(dest);

        assert!(item.destination.is_some());
        match &item.destination {
            Some(d) => match &d.page {
                PageDestination::PageNumber(n) => assert_eq!(*n, 5),
                _ => panic!("Wrong destination type"),
            },
            None => panic!("Destination should be set"),
        }
    }

    #[test]
    fn test_outline_item_with_different_colors() {
        let rgb_item = OutlineItem::new("RGB Color").with_color(Color::rgb(0.5, 0.7, 1.0));
        assert!(rgb_item.color.is_some());

        let gray_item = OutlineItem::new("Gray Color").with_color(Color::gray(0.5));
        assert!(gray_item.color.is_some());

        let cmyk_item = OutlineItem::new("CMYK Color").with_color(Color::cmyk(0.1, 0.2, 0.3, 0.4));
        assert!(cmyk_item.color.is_some());
    }

    #[test]
    fn test_outline_item_debug_clone() {
        let item = OutlineItem::new("Test Item")
            .bold()
            .with_color(Color::rgb(1.0, 0.0, 0.0));

        let debug_str = format!("{item:?}");
        assert!(debug_str.contains("OutlineItem"));
        assert!(debug_str.contains("Test Item"));

        let cloned = item.clone();
        assert_eq!(cloned.title, item.title);
        assert_eq!(cloned.flags.bold, item.flags.bold);
        assert_eq!(cloned.open, item.open);
    }

    #[test]
    fn test_outline_tree_default() {
        let tree = OutlineTree::default();
        assert!(tree.items.is_empty());
        assert_eq!(tree.total_count(), 0);
        assert_eq!(tree.visible_count(), 0);
    }

    #[test]
    fn test_outline_tree_add_multiple_items() {
        let mut tree = OutlineTree::new();

        tree.add_item(OutlineItem::new("First"));
        tree.add_item(OutlineItem::new("Second"));
        tree.add_item(OutlineItem::new("Third"));

        assert_eq!(tree.items.len(), 3);
        assert_eq!(tree.total_count(), 3);
        assert_eq!(tree.visible_count(), 3);
    }

    #[test]
    fn test_outline_tree_with_closed_items() {
        let mut tree = OutlineTree::new();

        let mut chapter = OutlineItem::new("Chapter").closed();
        chapter.add_child(OutlineItem::new("Hidden Section 1"));
        chapter.add_child(OutlineItem::new("Hidden Section 2"));

        tree.add_item(chapter);
        tree.add_item(OutlineItem::new("Visible Item"));

        assert_eq!(tree.total_count(), 4); // All items
        assert_eq!(tree.visible_count(), 2); // Only chapter and visible item
    }

    #[test]
    fn test_outline_builder_default() {
        let builder = OutlineBuilder::default();
        let tree = builder.build();
        assert!(tree.items.is_empty());
    }

    #[test]
    fn test_outline_builder_nested_structure() {
        let mut builder = OutlineBuilder::new();

        // Build a complex nested structure
        builder.push_item(OutlineItem::new("Part I"));
        builder.push_item(OutlineItem::new("Chapter 1"));
        builder.add_item(OutlineItem::new("Section 1.1"));
        builder.add_item(OutlineItem::new("Section 1.2"));
        builder.pop_item(); // Pop Chapter 1
        builder.push_item(OutlineItem::new("Chapter 2"));
        builder.add_item(OutlineItem::new("Section 2.1"));
        builder.pop_item(); // Pop Chapter 2
        builder.pop_item(); // Pop Part I

        builder.add_item(OutlineItem::new("Part II"));

        let tree = builder.build();
        assert_eq!(tree.items.len(), 2); // Part I and Part II
        assert_eq!(tree.total_count(), 7); // All items
    }

    #[test]
    fn test_outline_builder_auto_pop() {
        let mut builder = OutlineBuilder::new();

        // Push items without popping - should auto-pop on build
        builder.push_item(OutlineItem::new("Root"));
        builder.push_item(OutlineItem::new("Child"));
        builder.add_item(OutlineItem::new("Grandchild"));

        let tree = builder.build();
        assert_eq!(tree.items.len(), 1); // Only root
        assert_eq!(tree.total_count(), 3); // Root + Child + Grandchild
    }

    #[test]
    fn test_outline_item_count_deep_hierarchy() {
        let mut root = OutlineItem::new("Root");

        let mut level1 = OutlineItem::new("Level 1");
        let mut level2 = OutlineItem::new("Level 2");
        let mut level3 = OutlineItem::new("Level 3");
        level3.add_child(OutlineItem::new("Level 4"));
        level2.add_child(level3);
        level1.add_child(level2);
        root.add_child(level1);

        assert_eq!(root.count_all(), 5); // All 5 levels
        assert_eq!(root.count_visible(), 5); // All visible

        // Close level2 - should hide level 3 and 4
        root.children[0].children[0].open = false;
        assert_eq!(root.count_visible(), 3); // Root, Level1, Level2 (closed)
    }

    #[test]
    fn test_outline_item_to_dict_basic() {
        let item = OutlineItem::new("Test Title");
        let parent_ref = ObjectId::new(1, 0);

        let dict = outline_item_to_dict(&item, parent_ref, None, None, None, None);

        assert_eq!(
            dict.get("Title"),
            Some(&Object::String("Test Title".to_string()))
        );
        assert_eq!(dict.get("Parent"), Some(&Object::Reference(parent_ref)));
        assert!(dict.get("Prev").is_none());
        assert!(dict.get("Next").is_none());
        assert!(dict.get("First").is_none());
        assert!(dict.get("Last").is_none());
    }

    #[test]
    fn test_outline_item_to_dict_with_siblings() {
        let item = OutlineItem::new("Middle Child");
        let parent_ref = ObjectId::new(1, 0);
        let prev_ref = Some(ObjectId::new(2, 0));
        let next_ref = Some(ObjectId::new(3, 0));

        let dict = outline_item_to_dict(&item, parent_ref, None, None, prev_ref, next_ref);

        assert_eq!(
            dict.get("Prev"),
            Some(&Object::Reference(ObjectId::new(2, 0)))
        );
        assert_eq!(
            dict.get("Next"),
            Some(&Object::Reference(ObjectId::new(3, 0)))
        );
    }

    #[test]
    fn test_outline_item_to_dict_with_children() {
        let mut item = OutlineItem::new("Parent");
        item.add_child(OutlineItem::new("Child 1"));
        item.add_child(OutlineItem::new("Child 2"));

        let parent_ref = ObjectId::new(1, 0);
        let first_ref = Some(ObjectId::new(10, 0));
        let last_ref = Some(ObjectId::new(11, 0));

        let dict = outline_item_to_dict(&item, parent_ref, first_ref, last_ref, None, None);

        assert_eq!(
            dict.get("First"),
            Some(&Object::Reference(ObjectId::new(10, 0)))
        );
        assert_eq!(
            dict.get("Last"),
            Some(&Object::Reference(ObjectId::new(11, 0)))
        );
        assert_eq!(dict.get("Count"), Some(&Object::Integer(2))); // 2 visible children
    }

    #[test]
    fn test_outline_item_to_dict_closed_with_children() {
        let mut item = OutlineItem::new("Closed Parent").closed();
        item.add_child(OutlineItem::new("Hidden 1"));
        item.add_child(OutlineItem::new("Hidden 2"));
        item.add_child(OutlineItem::new("Hidden 3"));

        let dict = outline_item_to_dict(
            &item,
            ObjectId::new(1, 0),
            Some(ObjectId::new(10, 0)),
            Some(ObjectId::new(12, 0)),
            None,
            None,
        );

        // Count should be negative for closed items
        assert_eq!(dict.get("Count"), Some(&Object::Integer(-3)));
    }

    #[test]
    fn test_outline_item_to_dict_with_destination() {
        let dest = Destination::xyz(
            PageDestination::PageNumber(5),
            Some(100.0),
            Some(200.0),
            Some(1.5),
        );
        let item = OutlineItem::new("With Destination").with_destination(dest);

        let dict = outline_item_to_dict(&item, ObjectId::new(1, 0), None, None, None, None);

        assert!(dict.get("Dest").is_some());
        match dict.get("Dest") {
            Some(Object::Array(arr)) => {
                // Should be the destination array
                assert!(!arr.is_empty());
            }
            _ => panic!("Dest should be an array"),
        }
    }

    #[test]
    fn test_outline_item_to_dict_with_color_rgb() {
        let item = OutlineItem::new("Red Item").with_color(Color::rgb(1.0, 0.0, 0.0));

        let dict = outline_item_to_dict(&item, ObjectId::new(1, 0), None, None, None, None);

        match dict.get("C") {
            Some(Object::Array(arr)) => {
                assert_eq!(arr.len(), 3);
                assert_eq!(arr.first(), Some(&Object::Real(1.0)));
                assert_eq!(arr.get(1), Some(&Object::Real(0.0)));
                assert_eq!(arr.get(2), Some(&Object::Real(0.0)));
            }
            _ => panic!("C should be an array"),
        }
    }

    #[test]
    fn test_outline_item_to_dict_with_color_gray() {
        let item = OutlineItem::new("Gray Item").with_color(Color::gray(0.5));

        let dict = outline_item_to_dict(&item, ObjectId::new(1, 0), None, None, None, None);

        match dict.get("C") {
            Some(Object::Array(arr)) => {
                assert_eq!(arr.len(), 3);
                // Gray color should be converted to RGB with equal components
                assert_eq!(arr.first(), Some(&Object::Real(0.5)));
                assert_eq!(arr.get(1), Some(&Object::Real(0.5)));
                assert_eq!(arr.get(2), Some(&Object::Real(0.5)));
            }
            _ => panic!("C should be an array"),
        }
    }

    #[test]
    fn test_outline_item_to_dict_with_color_cmyk() {
        let item = OutlineItem::new("CMYK Item").with_color(Color::cmyk(0.0, 1.0, 1.0, 0.0));

        let dict = outline_item_to_dict(&item, ObjectId::new(1, 0), None, None, None, None);

        match dict.get("C") {
            Some(Object::Array(arr)) => {
                assert_eq!(arr.len(), 3);
                // CMYK (0,1,1,0) should convert to RGB (1,0,0) - red
                assert_eq!(arr.first(), Some(&Object::Real(1.0)));
                assert_eq!(arr.get(1), Some(&Object::Real(0.0)));
                assert_eq!(arr.get(2), Some(&Object::Real(0.0)));
            }
            _ => panic!("C should be an array"),
        }
    }

    #[test]
    fn test_outline_item_to_dict_with_flags() {
        let item = OutlineItem::new("Styled Item").bold().italic();

        let dict = outline_item_to_dict(&item, ObjectId::new(1, 0), None, None, None, None);

        assert_eq!(dict.get("F"), Some(&Object::Integer(3))); // Both bold and italic
    }

    #[test]
    fn test_outline_item_to_dict_no_flags() {
        let item = OutlineItem::new("Plain Item");

        let dict = outline_item_to_dict(&item, ObjectId::new(1, 0), None, None, None, None);

        // F field should not be present when flags are 0
        assert!(dict.get("F").is_none());
    }

    #[test]
    fn test_outline_tree_empty_counts() {
        let tree = OutlineTree::new();
        assert_eq!(tree.total_count(), 0);
        assert_eq!(tree.visible_count(), 0);
    }

    #[test]
    fn test_outline_builder_empty_pop() {
        let mut builder = OutlineBuilder::new();
        // Popping from empty stack should not panic
        builder.pop_item();
        let tree = builder.build();
        assert!(tree.items.is_empty());
    }

    #[test]
    fn test_outline_complex_visibility() {
        let mut root = OutlineItem::new("Book");

        let mut part1 = OutlineItem::new("Part 1"); // open
        let mut ch1 = OutlineItem::new("Chapter 1").closed();
        ch1.add_child(OutlineItem::new("Section 1.1"));
        ch1.add_child(OutlineItem::new("Section 1.2"));
        part1.add_child(ch1);

        let mut ch2 = OutlineItem::new("Chapter 2"); // open
        ch2.add_child(OutlineItem::new("Section 2.1"));
        part1.add_child(ch2);

        root.add_child(part1);

        // Structure:
        // Book (open)
        //   Part 1 (open)
        //     Chapter 1 (closed)
        //       Section 1.1 (hidden)
        //       Section 1.2 (hidden)
        //     Chapter 2 (open)
        //       Section 2.1 (visible)

        assert_eq!(root.count_all(), 7); // All items
        assert_eq!(root.count_visible(), 5); // Hidden: Section 1.1, 1.2
    }
}
