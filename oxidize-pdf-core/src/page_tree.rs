//! Page Tree Implementation according to ISO 32000-1 Section 7.7.3
//!
//! The page tree is a hierarchical structure that organizes pages in a PDF document.
//! It supports inheritance of properties from parent nodes to child nodes, allowing
//! efficient sharing of common resources and attributes.

use crate::error::Result;
use crate::geometry::Rectangle;
use crate::objects::{Dictionary, Object};
use crate::page::Page;

/// Page tree node types
#[derive(Debug, Clone)]
pub enum PageTreeNode {
    /// Internal node containing other nodes
    Pages(PagesNode),
    /// Leaf node containing actual page content
    Page(PageNode),
}

/// Internal page tree node (Type = Pages)
#[derive(Debug, Clone)]
pub struct PagesNode {
    /// Child nodes (can be Pages or Page nodes)
    pub kids: Vec<PageTreeNode>,
    /// Number of leaf nodes (pages) under this node
    pub count: usize,
    /// Inheritable attributes
    pub attributes: InheritableAttributes,
    /// Parent node reference (None for root)
    pub parent: Option<Box<PageTreeNode>>,
}

/// Leaf page node (Type = Page)
#[derive(Debug, Clone)]
pub struct PageNode {
    /// Page content streams
    pub contents: Vec<Object>,
    /// Inheritable attributes (may override parent)
    pub attributes: InheritableAttributes,
    /// Parent node reference
    pub parent: Option<Box<PagesNode>>,
    /// Annotations on this page
    pub annotations: Vec<Object>,
    /// Page-specific metadata
    pub metadata: Option<Dictionary>,
}

/// Attributes that can be inherited through the page tree
#[derive(Debug, Clone, Default)]
pub struct InheritableAttributes {
    /// Resources dictionary (fonts, images, etc.)
    pub resources: Option<Dictionary>,
    /// Media box - defines the page size
    pub media_box: Option<Rectangle>,
    /// Crop box - visible region of the page
    pub crop_box: Option<Rectangle>,
    /// Rotation in degrees (0, 90, 180, 270)
    pub rotate: Option<i32>,
}

impl InheritableAttributes {
    /// Create new inheritable attributes
    pub fn new() -> Self {
        Self::default()
    }

    /// Merge with parent attributes (parent attributes are used if not overridden)
    pub fn merge_with_parent(&self, parent: &InheritableAttributes) -> InheritableAttributes {
        InheritableAttributes {
            resources: self.resources.clone().or_else(|| parent.resources.clone()),
            media_box: self.media_box.or(parent.media_box),
            crop_box: self.crop_box.or(parent.crop_box),
            rotate: self.rotate.or(parent.rotate),
        }
    }

    /// Convert to dictionary for PDF output
    pub fn to_dict(&self) -> Dictionary {
        let mut dict = Dictionary::new();

        if let Some(ref resources) = self.resources {
            dict.set("Resources", Object::Dictionary(resources.clone()));
        }

        if let Some(media_box) = self.media_box {
            dict.set(
                "MediaBox",
                Object::Array(vec![
                    Object::Real(media_box.lower_left.x),
                    Object::Real(media_box.lower_left.y),
                    Object::Real(media_box.upper_right.x),
                    Object::Real(media_box.upper_right.y),
                ]),
            );
        }

        if let Some(crop_box) = self.crop_box {
            dict.set(
                "CropBox",
                Object::Array(vec![
                    Object::Real(crop_box.lower_left.x),
                    Object::Real(crop_box.lower_left.y),
                    Object::Real(crop_box.upper_right.x),
                    Object::Real(crop_box.upper_right.y),
                ]),
            );
        }

        if let Some(rotate) = self.rotate {
            dict.set("Rotate", Object::Integer(rotate as i64));
        }

        dict
    }
}

/// Page tree structure for organizing pages hierarchically
pub struct PageTree {
    /// Root node of the tree
    root: PagesNode,
    /// Maximum number of children per internal node
    max_kids: usize,
}

impl PageTree {
    /// Create a new page tree
    pub fn new() -> Self {
        Self {
            root: PagesNode {
                kids: Vec::new(),
                count: 0,
                attributes: InheritableAttributes::new(),
                parent: None,
            },
            max_kids: 10, // Typical value for balanced trees
        }
    }

    /// Set the maximum number of children per internal node
    pub fn set_max_kids(&mut self, max: usize) {
        self.max_kids = max.max(2); // Minimum of 2 for a valid tree
    }

    /// Add a page to the tree
    pub fn add_page(&mut self, page: Page) -> Result<()> {
        use crate::geometry::Point;

        // Create media box from page dimensions
        let media_box = Rectangle::new(
            Point::new(0.0, 0.0),
            Point::new(page.width(), page.height()),
        );

        let page_node = PageNode {
            contents: vec![],
            attributes: InheritableAttributes {
                media_box: Some(media_box),
                crop_box: None,  // Not set by default
                rotate: Some(0), // Default rotation
                resources: None, // Will be set from page resources
            },
            parent: None,
            annotations: Vec::new(),
            metadata: None,
        };

        self.add_page_node(page_node);
        self.root.count += 1;
        Ok(())
    }

    /// Add a page node to the tree
    fn add_page_node(&mut self, page: PageNode) {
        // For simplicity, add directly to root
        // In a full implementation, this would balance the tree
        self.root.kids.push(PageTreeNode::Page(page));
    }

    /// Get the total number of pages
    pub fn page_count(&self) -> usize {
        self.root.count
    }

    /// Get a page by index (0-based)
    pub fn get_page(&self, index: usize) -> Option<PageNode> {
        self.get_page_from_node_clone(&self.root, index)
    }

    /// Recursively find a page by index (returns a clone)
    #[allow(clippy::only_used_in_recursion)]
    fn get_page_from_node_clone(&self, pages: &PagesNode, index: usize) -> Option<PageNode> {
        let mut current_index = index;
        for kid in &pages.kids {
            match kid {
                PageTreeNode::Page(page) => {
                    if current_index == 0 {
                        return Some(page.clone());
                    }
                    current_index -= 1;
                }
                PageTreeNode::Pages(pages) => {
                    let kid_count = pages.count;
                    if current_index < kid_count {
                        return self.get_page_from_node_clone(pages, current_index);
                    }
                    current_index -= kid_count;
                }
            }
        }
        None
    }

    /// Get the number of pages under a node
    #[allow(dead_code)]
    fn get_node_count(&self, node: &PageTreeNode) -> usize {
        match node {
            PageTreeNode::Page(_) => 1,
            PageTreeNode::Pages(pages) => pages.count,
        }
    }

    /// Convert the page tree to a PDF dictionary structure
    pub fn to_dict(&self) -> Dictionary {
        self.node_to_dict(&PageTreeNode::Pages(self.root.clone()))
    }

    /// Convert a node to a dictionary
    #[allow(clippy::only_used_in_recursion)]
    fn node_to_dict(&self, node: &PageTreeNode) -> Dictionary {
        let mut dict = Dictionary::new();

        match node {
            PageTreeNode::Page(page) => {
                dict.set("Type", Object::Name("Page".to_string()));

                // Add inheritable attributes
                let attrs = page.attributes.to_dict();
                for (key, value) in attrs.iter() {
                    dict.set(key, value.clone());
                }

                // Add contents
                if !page.contents.is_empty() {
                    if page.contents.len() == 1 {
                        dict.set("Contents", page.contents[0].clone());
                    } else {
                        dict.set("Contents", Object::Array(page.contents.clone()));
                    }
                }

                // Add annotations
                if !page.annotations.is_empty() {
                    dict.set("Annots", Object::Array(page.annotations.clone()));
                }
            }
            PageTreeNode::Pages(pages) => {
                dict.set("Type", Object::Name("Pages".to_string()));
                dict.set("Count", Object::Integer(pages.count as i64));

                // Add inheritable attributes
                let attrs = pages.attributes.to_dict();
                for (key, value) in attrs.iter() {
                    dict.set(key, value.clone());
                }

                // Add kids
                let kids: Vec<Object> = pages
                    .kids
                    .iter()
                    .map(|kid| Object::Dictionary(self.node_to_dict(kid)))
                    .collect();
                dict.set("Kids", Object::Array(kids));
            }
        }

        dict
    }

    /// Balance the tree to optimize performance
    pub fn balance(&mut self) {
        // This would implement tree balancing logic
        // For now, we keep a simple flat structure
    }

    /// Set inheritable attributes at the root level
    pub fn set_default_media_box(&mut self, rect: Rectangle) {
        self.root.attributes.media_box = Some(rect);
    }

    /// Set default resources for all pages
    pub fn set_default_resources(&mut self, resources: Dictionary) {
        self.root.attributes.resources = Some(resources);
    }

    /// Find all pages matching a predicate
    pub fn find_pages<F>(&self, predicate: F) -> Vec<usize>
    where
        F: Fn(&PageNode) -> bool,
    {
        let mut results = Vec::new();
        self.find_pages_in_node(&self.root, &predicate, 0, &mut results);
        results
    }

    #[allow(clippy::only_used_in_recursion)]
    fn find_pages_in_node<F>(
        &self,
        pages: &PagesNode,
        predicate: &F,
        base_index: usize,
        results: &mut Vec<usize>,
    ) -> usize
    where
        F: Fn(&PageNode) -> bool,
    {
        let mut current_index = base_index;
        let mut count = 0;

        for kid in &pages.kids {
            match kid {
                PageTreeNode::Page(page) => {
                    if predicate(page) {
                        results.push(current_index);
                    }
                    current_index += 1;
                    count += 1;
                }
                PageTreeNode::Pages(inner_pages) => {
                    let inner_count =
                        self.find_pages_in_node(inner_pages, predicate, current_index, results);
                    current_index += inner_count;
                    count += inner_count;
                }
            }
        }

        count
    }
}

impl Default for PageTree {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for constructing page trees with inherited attributes
pub struct PageTreeBuilder {
    tree: PageTree,
    default_attributes: InheritableAttributes,
}

impl Default for PageTreeBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl PageTreeBuilder {
    /// Create a new page tree builder
    pub fn new() -> Self {
        Self {
            tree: PageTree::new(),
            default_attributes: InheritableAttributes::new(),
        }
    }

    /// Set default media box for all pages
    pub fn with_media_box(mut self, rect: Rectangle) -> Self {
        self.default_attributes.media_box = Some(rect);
        self.tree.set_default_media_box(rect);
        self
    }

    /// Set default resources
    pub fn with_resources(mut self, resources: Dictionary) -> Self {
        self.default_attributes.resources = Some(resources.clone());
        self.tree.set_default_resources(resources);
        self
    }

    /// Set default rotation
    pub fn with_rotation(mut self, degrees: i32) -> Self {
        self.default_attributes.rotate = Some(degrees);
        self
    }

    /// Add a page to the tree
    pub fn add_page(mut self, page: Page) -> Self {
        self.tree.add_page(page).unwrap_or_else(|e| {
            eprintln!("Warning: Failed to add page: {}", e);
        });
        self
    }

    /// Build the final page tree
    pub fn build(mut self) -> PageTree {
        self.tree.balance();
        self.tree
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Point;

    #[test]
    fn test_inheritable_attributes_merge() {
        let parent = InheritableAttributes {
            resources: Some(Dictionary::new()),
            media_box: Some(Rectangle::new(
                Point::new(0.0, 0.0),
                Point::new(612.0, 792.0),
            )),
            crop_box: None,
            rotate: Some(0),
        };

        let child = InheritableAttributes {
            resources: None,
            media_box: None,
            crop_box: Some(Rectangle::new(
                Point::new(10.0, 10.0),
                Point::new(602.0, 782.0),
            )),
            rotate: Some(90),
        };

        let merged = child.merge_with_parent(&parent);
        assert!(merged.resources.is_some());
        assert!(merged.media_box.is_some());
        assert!(merged.crop_box.is_some());
        assert_eq!(merged.rotate, Some(90));
    }

    #[test]
    fn test_page_tree_creation() {
        let tree = PageTree::new();
        assert_eq!(tree.page_count(), 0);
    }

    #[test]
    fn test_page_tree_add_page() {
        let mut tree = PageTree::new();
        let page = Page::new(612.0, 792.0);
        tree.add_page(page).unwrap();
        assert_eq!(tree.page_count(), 1);
    }

    #[test]
    fn test_page_tree_builder() {
        let rect = Rectangle::new(Point::new(0.0, 0.0), Point::new(612.0, 792.0));
        let tree = PageTreeBuilder::new()
            .with_media_box(rect)
            .with_rotation(0)
            .add_page(Page::new(612.0, 792.0))
            .add_page(Page::new(612.0, 792.0))
            .build();

        assert_eq!(tree.page_count(), 2);
    }

    #[test]
    fn test_page_tree_to_dict() {
        let mut tree = PageTree::new();
        tree.add_page(Page::new(612.0, 792.0)).unwrap();

        let dict = tree.to_dict();
        assert_eq!(dict.get("Type"), Some(&Object::Name("Pages".to_string())));
        assert_eq!(dict.get("Count"), Some(&Object::Integer(1)));
        assert!(dict.get("Kids").is_some());
    }

    #[test]
    fn test_find_pages() {
        let mut tree = PageTree::new();

        // Add pages with different sizes
        tree.add_page(Page::new(612.0, 792.0)).unwrap(); // Letter
        tree.add_page(Page::new(595.0, 842.0)).unwrap(); // A4
        tree.add_page(Page::new(612.0, 792.0)).unwrap(); // Letter

        // Find all letter-sized pages
        let letter_pages = tree.find_pages(|page| {
            page.attributes
                .media_box
                .map(|mb| mb.upper_right.x == 612.0 && mb.upper_right.y == 792.0)
                .unwrap_or(false)
        });

        assert_eq!(letter_pages.len(), 2);
        assert_eq!(letter_pages[0], 0);
        assert_eq!(letter_pages[1], 2);
    }

    #[test]
    fn test_attributes_to_dict() {
        let attrs = InheritableAttributes {
            resources: Some(Dictionary::new()),
            media_box: Some(Rectangle::new(
                Point::new(0.0, 0.0),
                Point::new(612.0, 792.0),
            )),
            crop_box: None,
            rotate: Some(90),
        };

        let dict = attrs.to_dict();
        assert!(dict.get("Resources").is_some());
        assert!(dict.get("MediaBox").is_some());
        assert_eq!(dict.get("Rotate"), Some(&Object::Integer(90)));
    }

    #[test]
    fn test_max_kids_setting() {
        let mut tree = PageTree::new();
        tree.set_max_kids(5);
        assert_eq!(tree.max_kids, 5);

        tree.set_max_kids(1); // Should be clamped to minimum of 2
        assert_eq!(tree.max_kids, 2);
    }

    #[test]
    fn test_page_tree_get_page() {
        let mut tree = PageTree::new();

        // Add multiple pages
        tree.add_page(Page::new(612.0, 792.0)).unwrap();
        tree.add_page(Page::new(595.0, 842.0)).unwrap();
        tree.add_page(Page::new(420.0, 595.0)).unwrap();

        // Get pages by index
        let page0 = tree.get_page(0);
        assert!(page0.is_some());
        let page0 = page0.unwrap();
        assert_eq!(page0.attributes.media_box.unwrap().upper_right.x, 612.0);

        let page1 = tree.get_page(1);
        assert!(page1.is_some());
        let page1 = page1.unwrap();
        assert_eq!(page1.attributes.media_box.unwrap().upper_right.x, 595.0);

        let page2 = tree.get_page(2);
        assert!(page2.is_some());
        let page2 = page2.unwrap();
        assert_eq!(page2.attributes.media_box.unwrap().upper_right.x, 420.0);

        // Out of bounds
        assert!(tree.get_page(3).is_none());
        assert!(tree.get_page(100).is_none());
    }

    #[test]
    fn test_page_tree_empty() {
        let tree = PageTree::new();
        assert_eq!(tree.page_count(), 0);
        assert!(tree.get_page(0).is_none());

        let dict = tree.to_dict();
        assert_eq!(dict.get("Type"), Some(&Object::Name("Pages".to_string())));
        assert_eq!(dict.get("Count"), Some(&Object::Integer(0)));
    }

    #[test]
    fn test_page_tree_large_number_of_pages() {
        let mut tree = PageTree::new();

        // Add 100 pages
        for _ in 0..100 {
            tree.add_page(Page::new(612.0, 792.0)).unwrap();
        }

        assert_eq!(tree.page_count(), 100);

        // Check first, middle, and last pages
        assert!(tree.get_page(0).is_some());
        assert!(tree.get_page(49).is_some());
        assert!(tree.get_page(99).is_some());
        assert!(tree.get_page(100).is_none());
    }

    #[test]
    fn test_page_node_creation() {
        let page_node = PageNode {
            contents: vec![Object::Integer(1), Object::Integer(2)],
            attributes: InheritableAttributes::new(),
            parent: None,
            annotations: vec![Object::Name("Annot".to_string())],
            metadata: Some(Dictionary::new()),
        };

        assert_eq!(page_node.contents.len(), 2);
        assert_eq!(page_node.annotations.len(), 1);
        assert!(page_node.metadata.is_some());
    }

    #[test]
    fn test_pages_node_creation() {
        let pages_node = PagesNode {
            kids: vec![],
            count: 0,
            attributes: InheritableAttributes::new(),
            parent: None,
        };

        assert_eq!(pages_node.kids.len(), 0);
        assert_eq!(pages_node.count, 0);
    }

    #[test]
    fn test_inheritable_attributes_default() {
        let attrs = InheritableAttributes::default();
        assert!(attrs.resources.is_none());
        assert!(attrs.media_box.is_none());
        assert!(attrs.crop_box.is_none());
        assert!(attrs.rotate.is_none());
    }

    #[test]
    fn test_inheritable_attributes_complete() {
        let attrs = InheritableAttributes {
            resources: Some(Dictionary::new()),
            media_box: Some(Rectangle::new(
                Point::new(0.0, 0.0),
                Point::new(612.0, 792.0),
            )),
            crop_box: Some(Rectangle::new(
                Point::new(36.0, 36.0),
                Point::new(576.0, 756.0),
            )),
            rotate: Some(90),
        };

        let dict = attrs.to_dict();
        assert!(dict.get("Resources").is_some());
        assert!(dict.get("MediaBox").is_some());
        assert!(dict.get("CropBox").is_some());
        assert_eq!(dict.get("Rotate"), Some(&Object::Integer(90)));
    }

    #[test]
    fn test_merge_with_parent_override() {
        let parent = InheritableAttributes {
            resources: Some(Dictionary::new()),
            media_box: Some(Rectangle::new(
                Point::new(0.0, 0.0),
                Point::new(612.0, 792.0),
            )),
            crop_box: Some(Rectangle::new(
                Point::new(0.0, 0.0),
                Point::new(612.0, 792.0),
            )),
            rotate: Some(0),
        };

        let mut child_resources = Dictionary::new();
        child_resources.set("Font", Object::Name("F1".to_string()));

        let child = InheritableAttributes {
            resources: Some(child_resources.clone()),
            media_box: Some(Rectangle::new(
                Point::new(0.0, 0.0),
                Point::new(595.0, 842.0),
            )),
            crop_box: None,
            rotate: Some(180),
        };

        let merged = child.merge_with_parent(&parent);

        // Child values should override parent
        assert_eq!(
            merged.resources.unwrap().get("Font"),
            Some(&Object::Name("F1".to_string()))
        );
        assert_eq!(merged.media_box.unwrap().upper_right.x, 595.0);
        assert_eq!(merged.crop_box.unwrap().upper_right.x, 612.0); // From parent
        assert_eq!(merged.rotate, Some(180));
    }

    #[test]
    fn test_merge_with_parent_inherit_all() {
        let parent = InheritableAttributes {
            resources: Some(Dictionary::new()),
            media_box: Some(Rectangle::new(
                Point::new(0.0, 0.0),
                Point::new(612.0, 792.0),
            )),
            crop_box: Some(Rectangle::new(
                Point::new(36.0, 36.0),
                Point::new(576.0, 756.0),
            )),
            rotate: Some(90),
        };

        let child = InheritableAttributes::new();
        let merged = child.merge_with_parent(&parent);

        // All values should be inherited from parent
        assert!(merged.resources.is_some());
        assert_eq!(merged.media_box.unwrap().upper_right.x, 612.0);
        assert_eq!(merged.crop_box.unwrap().upper_right.x, 576.0);
        assert_eq!(merged.rotate, Some(90));
    }

    #[test]
    fn test_page_tree_builder_comprehensive() {
        let rect = Rectangle::new(Point::new(0.0, 0.0), Point::new(595.0, 842.0));
        let mut resources = Dictionary::new();
        resources.set("Font", Object::Name("F1".to_string()));

        let tree = PageTreeBuilder::new()
            .with_media_box(rect)
            .with_resources(resources.clone())
            .with_rotation(90)
            .add_page(Page::new(595.0, 842.0))
            .add_page(Page::new(595.0, 842.0))
            .add_page(Page::new(595.0, 842.0))
            .build();

        assert_eq!(tree.page_count(), 3);
        assert_eq!(tree.root.attributes.media_box.unwrap().upper_right.x, 595.0);
        // Note: rotate is not set at root level in current implementation
        assert!(tree.root.attributes.resources.is_some());
    }

    #[test]
    fn test_find_pages_complex_predicate() {
        let mut tree = PageTree::new();

        // Add pages with different sizes
        tree.add_page(Page::new(612.0, 792.0)).unwrap(); // Letter
        tree.add_page(Page::new(595.0, 842.0)).unwrap(); // A4
        tree.add_page(Page::new(420.0, 595.0)).unwrap(); // A5
        tree.add_page(Page::new(612.0, 792.0)).unwrap(); // Letter
        tree.add_page(Page::new(595.0, 842.0)).unwrap(); // A4

        // Find all A4 pages
        let a4_pages = tree.find_pages(|page| {
            page.attributes
                .media_box
                .map(|mb| mb.upper_right.x == 595.0 && mb.upper_right.y == 842.0)
                .unwrap_or(false)
        });

        assert_eq!(a4_pages.len(), 2);
        assert_eq!(a4_pages[0], 1);
        assert_eq!(a4_pages[1], 4);

        // Find all pages smaller than A4
        let small_pages = tree.find_pages(|page| {
            page.attributes
                .media_box
                .map(|mb| mb.upper_right.x < 595.0 || mb.upper_right.y < 842.0)
                .unwrap_or(false)
        });

        assert_eq!(small_pages.len(), 3); // 2 Letter + 1 A5
    }

    #[test]
    fn test_page_tree_node_to_dict_page() {
        let tree = PageTree::new();
        let page_node = PageNode {
            contents: vec![Object::Integer(1)],
            attributes: InheritableAttributes {
                media_box: Some(Rectangle::new(
                    Point::new(0.0, 0.0),
                    Point::new(612.0, 792.0),
                )),
                crop_box: None,
                rotate: Some(0),
                resources: None,
            },
            parent: None,
            annotations: vec![Object::Name("Annot1".to_string())],
            metadata: None,
        };

        let dict = tree.node_to_dict(&PageTreeNode::Page(page_node));
        assert_eq!(dict.get("Type"), Some(&Object::Name("Page".to_string())));
        assert_eq!(dict.get("Contents"), Some(&Object::Integer(1)));
        assert!(dict.get("Annots").is_some());
        assert!(dict.get("MediaBox").is_some());
    }

    #[test]
    fn test_page_tree_node_to_dict_pages() {
        let tree = PageTree::new();
        let pages_node = PagesNode {
            kids: vec![],
            count: 5,
            attributes: InheritableAttributes::new(),
            parent: None,
        };

        let dict = tree.node_to_dict(&PageTreeNode::Pages(pages_node));
        assert_eq!(dict.get("Type"), Some(&Object::Name("Pages".to_string())));
        assert_eq!(dict.get("Count"), Some(&Object::Integer(5)));
        assert!(dict.get("Kids").is_some());
    }

    #[test]
    fn test_rotation_values() {
        let attrs = InheritableAttributes {
            resources: None,
            media_box: None,
            crop_box: None,
            rotate: Some(270), // Valid rotation
        };

        let dict = attrs.to_dict();
        assert_eq!(dict.get("Rotate"), Some(&Object::Integer(270)));

        // Test all valid rotation values
        for rotation in &[0, 90, 180, 270] {
            let attrs = InheritableAttributes {
                resources: None,
                media_box: None,
                crop_box: None,
                rotate: Some(*rotation),
            };
            let dict = attrs.to_dict();
            assert_eq!(dict.get("Rotate"), Some(&Object::Integer(*rotation as i64)));
        }
    }

    #[test]
    fn test_balance_method() {
        let mut tree = PageTree::new();

        // Add many pages
        for _ in 0..50 {
            tree.add_page(Page::new(612.0, 792.0)).unwrap();
        }

        let count_before = tree.page_count();
        tree.balance();
        let count_after = tree.page_count();

        // Balance should not change page count
        assert_eq!(count_before, count_after);
        assert_eq!(count_after, 50);
    }

    #[test]
    fn test_set_default_methods() {
        let mut tree = PageTree::new();

        let rect = Rectangle::new(Point::new(0.0, 0.0), Point::new(420.0, 595.0));
        tree.set_default_media_box(rect);
        assert_eq!(tree.root.attributes.media_box.unwrap().upper_right.x, 420.0);

        let mut resources = Dictionary::new();
        resources.set(
            "ProcSet",
            Object::Array(vec![Object::Name("PDF".to_string())]),
        );
        tree.set_default_resources(resources.clone());
        assert!(tree.root.attributes.resources.is_some());
        assert_eq!(
            tree.root
                .attributes
                .resources
                .as_ref()
                .unwrap()
                .get("ProcSet"),
            Some(&Object::Array(vec![Object::Name("PDF".to_string())]))
        );
    }
}
