//! Page tree structure according to ISO 32000-1 Section 7.7.3

#![allow(clippy::large_enum_variant, clippy::collapsible_match)]

use crate::error::{PdfError, Result};
use crate::geometry::Rectangle;
use crate::objects::{Array, Dictionary, Object, ObjectId};
use crate::page::Page;
use std::collections::HashMap;

/// Page tree node type
#[derive(Clone)]
pub enum PageTreeNode {
    /// Pages node (intermediate node)
    Pages {
        /// Child nodes
        kids: Vec<ObjectId>,
        /// Total page count in subtree
        count: u32,
        /// Media box inherited by children
        media_box: Option<Rectangle>,
        /// Resources inherited by children
        resources: Option<Dictionary>,
        /// Parent node reference
        parent: Option<ObjectId>,
    },
    /// Page node (leaf node)
    Page {
        /// Page content
        page: Page,
        /// Parent node reference
        parent: ObjectId,
    },
}

/// Page tree structure
pub struct PageTree {
    /// Root node ID
    root: ObjectId,
    /// All nodes in the tree
    nodes: HashMap<ObjectId, PageTreeNode>,
    /// Page to node mapping
    page_map: HashMap<u32, ObjectId>,
}

impl PageTree {
    /// Create new page tree
    pub fn new(root_id: ObjectId) -> Self {
        let mut nodes = HashMap::new();
        nodes.insert(
            root_id,
            PageTreeNode::Pages {
                kids: Vec::new(),
                count: 0,
                media_box: None,
                resources: None,
                parent: None,
            },
        );

        Self {
            root: root_id,
            nodes,
            page_map: HashMap::new(),
        }
    }

    /// Get root node ID
    pub fn root(&self) -> ObjectId {
        self.root
    }

    /// Get total page count
    pub fn page_count(&self) -> u32 {
        match self.nodes.get(&self.root) {
            Some(PageTreeNode::Pages { count, .. }) => *count,
            _ => 0,
        }
    }

    /// Get page by index
    pub fn get_page(&self, index: u32) -> Option<&Page> {
        let node_id = self.page_map.get(&index)?;
        match self.nodes.get(node_id)? {
            PageTreeNode::Page { page, .. } => Some(page),
            _ => None,
        }
    }

    /// Add a page
    pub fn add_page(&mut self, page: Page, page_id: ObjectId, parent_id: ObjectId) -> Result<()> {
        // Add page node
        self.nodes.insert(
            page_id,
            PageTreeNode::Page {
                page,
                parent: parent_id,
            },
        );

        // Update parent
        self.add_kid_to_parent(parent_id, page_id)?;

        // Increment count for immediate parent and all ancestors
        if let Some(PageTreeNode::Pages { count, .. }) = self.nodes.get_mut(&parent_id) {
            *count += 1;
        }
        self.update_ancestor_counts(parent_id)?;

        // Update page map
        let page_index = self.page_count() - 1;
        self.page_map.insert(page_index, page_id);

        Ok(())
    }

    /// Add intermediate node
    pub fn add_pages_node(&mut self, node_id: ObjectId, parent_id: Option<ObjectId>) -> Result<()> {
        self.nodes.insert(
            node_id,
            PageTreeNode::Pages {
                kids: Vec::new(),
                count: 0,
                media_box: None,
                resources: None,
                parent: parent_id,
            },
        );

        if let Some(parent) = parent_id {
            self.add_kid_to_parent(parent, node_id)?;
        }

        Ok(())
    }

    /// Add kid to parent node
    fn add_kid_to_parent(&mut self, parent_id: ObjectId, kid_id: ObjectId) -> Result<()> {
        match self.nodes.get_mut(&parent_id) {
            Some(PageTreeNode::Pages { kids, .. }) => {
                kids.push(kid_id);
                // Don't increment count here - let update_ancestor_counts handle it if it's a page
                Ok(())
            }
            _ => Err(PdfError::InvalidStructure(
                "Parent is not a Pages node".to_string(),
            )),
        }
    }

    /// Update ancestor page counts
    fn update_ancestor_counts(&mut self, node_id: ObjectId) -> Result<()> {
        if let Some(PageTreeNode::Pages { parent, .. }) = self.nodes.get(&node_id) {
            if let Some(parent_id) = parent {
                let parent_id = *parent_id;
                if let Some(PageTreeNode::Pages { count, .. }) = self.nodes.get_mut(&parent_id) {
                    *count += 1;
                    self.update_ancestor_counts(parent_id)?;
                }
            }
        }
        Ok(())
    }

    /// Convert node to dictionary
    pub fn node_to_dict(&self, node_id: ObjectId) -> Result<Dictionary> {
        let node = self
            .nodes
            .get(&node_id)
            .ok_or_else(|| PdfError::InvalidStructure("Node not found".to_string()))?;

        match node {
            PageTreeNode::Pages {
                kids,
                count,
                media_box,
                resources,
                parent,
            } => {
                let mut dict = Dictionary::new();
                dict.set("Type", Object::Name("Pages".to_string()));

                // Kids array
                let kids_array: Array = kids.iter().map(|id| Object::Reference(*id)).collect();
                dict.set("Kids", Object::Array(kids_array.into()));

                dict.set("Count", Object::Integer(*count as i64));

                if let Some(parent_id) = parent {
                    dict.set("Parent", Object::Reference(*parent_id));
                }

                if let Some(mb) = media_box {
                    let mb_array = Array::from(vec![
                        Object::Real(mb.lower_left.x),
                        Object::Real(mb.lower_left.y),
                        Object::Real(mb.upper_right.x),
                        Object::Real(mb.upper_right.y),
                    ]);
                    dict.set("MediaBox", Object::Array(mb_array.into()));
                }

                if let Some(res) = resources {
                    dict.set("Resources", Object::Dictionary(res.clone()));
                }

                Ok(dict)
            }
            PageTreeNode::Page { page, parent } => {
                // Convert page to dictionary
                let mut dict = page.to_dict();
                dict.set("Type", Object::Name("Page".to_string()));
                dict.set("Parent", Object::Reference(*parent));
                Ok(dict)
            }
        }
    }
}

/// Page tree builder
pub struct PageTreeBuilder {
    /// Current tree
    tree: PageTree,
    /// Next object ID
    next_id: u32,
}

impl PageTreeBuilder {
    /// Create new builder
    pub fn new(start_id: u32) -> Self {
        let root_id = ObjectId::new(start_id, 0);
        Self {
            tree: PageTree::new(root_id),
            next_id: start_id + 1,
        }
    }

    /// Add pages in balanced tree structure
    pub fn add_pages(&mut self, pages: Vec<Page>) -> Result<()> {
        // For simplicity, add all pages under root
        // In production, create intermediate nodes for better performance
        for page in pages {
            let page_id = ObjectId::new(self.next_id, 0);
            self.next_id += 1;
            self.tree.add_page(page, page_id, self.tree.root)?;
        }
        Ok(())
    }

    /// Build the tree
    pub fn build(self) -> PageTree {
        self.tree
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_page_tree_new() {
        let root_id = ObjectId::new(1, 0);
        let tree = PageTree::new(root_id);
        assert_eq!(tree.root(), root_id);
        assert_eq!(tree.page_count(), 0);
    }

    #[test]
    fn test_add_page() {
        let root_id = ObjectId::new(1, 0);
        let mut tree = PageTree::new(root_id);

        let page = Page::a4();
        let page_id = ObjectId::new(2, 0);

        tree.add_page(page, page_id, root_id).unwrap();
        assert_eq!(tree.page_count(), 1);
        assert!(tree.get_page(0).is_some());
    }

    #[test]
    fn test_page_tree_builder() {
        let mut builder = PageTreeBuilder::new(1);
        let pages = vec![Page::a4(), Page::letter(), Page::a4()];

        builder.add_pages(pages).unwrap();
        let tree = builder.build();

        assert_eq!(tree.page_count(), 3);
        assert!(tree.get_page(0).is_some());
        assert!(tree.get_page(1).is_some());
        assert!(tree.get_page(2).is_some());
        assert!(tree.get_page(3).is_none());
    }

    #[test]
    fn test_node_to_dict() {
        let root_id = ObjectId::new(1, 0);
        let tree = PageTree::new(root_id);

        let dict = tree.node_to_dict(root_id).unwrap();
        assert_eq!(dict.get("Type"), Some(&Object::Name("Pages".to_string())));
        assert!(dict.get("Kids").is_some());
        assert_eq!(dict.get("Count"), Some(&Object::Integer(0)));
    }

    #[test]
    fn test_page_tree_node_clone() {
        use crate::geometry::Point;

        let node = PageTreeNode::Pages {
            kids: vec![ObjectId::new(2, 0), ObjectId::new(3, 0)],
            count: 2,
            media_box: Some(Rectangle::new(
                Point::new(0.0, 0.0),
                Point::new(612.0, 792.0),
            )),
            resources: Some(Dictionary::new()),
            parent: Some(ObjectId::new(1, 0)),
        };

        let cloned = node.clone();
        match cloned {
            PageTreeNode::Pages { kids, count, .. } => {
                assert_eq!(kids.len(), 2);
                assert_eq!(count, 2);
            }
            _ => panic!("Wrong node type"),
        }
    }

    #[test]
    fn test_page_tree_node_page_variant() {
        let page = Page::a4();
        let parent_id = ObjectId::new(1, 0);
        let node = PageTreeNode::Page {
            page: page.clone(),
            parent: parent_id,
        };

        match &node {
            PageTreeNode::Page { parent, .. } => {
                assert_eq!(*parent, parent_id);
            }
            _ => panic!("Wrong node type"),
        }
    }

    #[test]
    fn test_page_tree_add_pages_node() {
        let root_id = ObjectId::new(1, 0);
        let mut tree = PageTree::new(root_id);

        let child_id = ObjectId::new(2, 0);
        tree.add_pages_node(child_id, Some(root_id)).unwrap();

        // Check that the child was added
        assert!(tree.nodes.contains_key(&child_id));

        // Check parent has the child
        match &tree.nodes[&root_id] {
            PageTreeNode::Pages { kids, .. } => {
                assert_eq!(kids.len(), 1);
                assert_eq!(kids[0], child_id);
            }
            _ => panic!("Root should be Pages node"),
        }
    }

    #[test]
    fn test_page_tree_get_page_invalid_index() {
        let root_id = ObjectId::new(1, 0);
        let tree = PageTree::new(root_id);

        assert!(tree.get_page(0).is_none());
        assert!(tree.get_page(100).is_none());
    }

    #[test]
    fn test_page_tree_multiple_pages() {
        let root_id = ObjectId::new(1, 0);
        let mut tree = PageTree::new(root_id);

        // Add multiple pages
        for i in 0..5 {
            let page = Page::a4();
            let page_id = ObjectId::new(i + 2, 0);
            tree.add_page(page, page_id, root_id).unwrap();
        }

        assert_eq!(tree.page_count(), 5);

        // Check all pages are accessible
        for i in 0..5 {
            assert!(tree.get_page(i).is_some());
        }
    }

    #[test]
    fn test_page_tree_nested_structure() {
        let root_id = ObjectId::new(1, 0);
        let mut tree = PageTree::new(root_id);

        // Add intermediate node
        let intermediate_id = ObjectId::new(2, 0);
        tree.add_pages_node(intermediate_id, Some(root_id)).unwrap();

        // Add pages under intermediate node
        let page1_id = ObjectId::new(3, 0);
        let page2_id = ObjectId::new(4, 0);

        tree.add_page(Page::a4(), page1_id, intermediate_id)
            .unwrap();
        tree.add_page(Page::letter(), page2_id, intermediate_id)
            .unwrap();

        // Root should have count 2
        assert_eq!(tree.page_count(), 2);
    }

    #[test]
    fn test_page_tree_add_kid_to_invalid_parent() {
        let root_id = ObjectId::new(1, 0);
        let mut tree = PageTree::new(root_id);

        let page_id = ObjectId::new(2, 0);
        tree.add_page(Page::a4(), page_id, root_id).unwrap();

        // Try to add a kid to a page node (not a Pages node)
        let result = tree.add_kid_to_parent(page_id, ObjectId::new(3, 0));
        assert!(result.is_err());
    }

    #[test]
    fn test_page_tree_node_to_dict_page_node() {
        let root_id = ObjectId::new(1, 0);
        let mut tree = PageTree::new(root_id);

        let page = Page::a4();
        let page_id = ObjectId::new(2, 0);
        tree.add_page(page, page_id, root_id).unwrap();

        let dict = tree.node_to_dict(page_id).unwrap();
        assert_eq!(dict.get("Type"), Some(&Object::Name("Page".to_string())));
        assert_eq!(dict.get("Parent"), Some(&Object::Reference(root_id)));
    }

    #[test]
    fn test_page_tree_node_to_dict_with_media_box() {
        use crate::geometry::Point;

        let root_id = ObjectId::new(1, 0);
        let mut tree = PageTree::new(root_id);

        // Add media box to root
        if let Some(PageTreeNode::Pages { media_box, .. }) = tree.nodes.get_mut(&root_id) {
            *media_box = Some(Rectangle::new(
                Point::new(0.0, 0.0),
                Point::new(612.0, 792.0),
            ));
        }

        let dict = tree.node_to_dict(root_id).unwrap();
        assert!(dict.get("MediaBox").is_some());

        match dict.get("MediaBox") {
            Some(Object::Array(arr)) => {
                assert_eq!(arr.len(), 4);
            }
            _ => panic!("MediaBox should be an array"),
        }
    }

    #[test]
    fn test_page_tree_node_to_dict_with_resources() {
        let root_id = ObjectId::new(1, 0);
        let mut tree = PageTree::new(root_id);

        // Add resources to root
        let mut res = Dictionary::new();
        res.set("Font", Object::Dictionary(Dictionary::new()));

        if let Some(PageTreeNode::Pages { resources, .. }) = tree.nodes.get_mut(&root_id) {
            *resources = Some(res);
        }

        let dict = tree.node_to_dict(root_id).unwrap();
        assert!(dict.get("Resources").is_some());
    }

    #[test]
    fn test_page_tree_node_to_dict_invalid_node() {
        let root_id = ObjectId::new(1, 0);
        let tree = PageTree::new(root_id);

        let invalid_id = ObjectId::new(999, 0);
        let result = tree.node_to_dict(invalid_id);
        assert!(result.is_err());
    }

    #[test]
    fn test_page_tree_builder_empty() {
        let builder = PageTreeBuilder::new(10);
        let tree = builder.build();

        assert_eq!(tree.root(), ObjectId::new(10, 0));
        assert_eq!(tree.page_count(), 0);
    }

    #[test]
    fn test_page_tree_builder_with_custom_start_id() {
        let mut builder = PageTreeBuilder::new(100);
        builder.add_pages(vec![Page::a4()]).unwrap();

        let tree = builder.build();
        assert_eq!(tree.root(), ObjectId::new(100, 0));

        // The page should have ID 101
        match tree.nodes.get(&ObjectId::new(101, 0)) {
            Some(PageTreeNode::Page { .. }) => (),
            _ => panic!("Page not found with expected ID"),
        }
    }

    #[test]
    fn test_page_tree_update_ancestor_counts() {
        let root_id = ObjectId::new(1, 0);
        let mut tree = PageTree::new(root_id);

        // Create a nested structure
        let level1_id = ObjectId::new(2, 0);
        let level2_id = ObjectId::new(3, 0);

        tree.add_pages_node(level1_id, Some(root_id)).unwrap();
        tree.add_pages_node(level2_id, Some(level1_id)).unwrap();

        // Add a page at the deepest level
        let page_id = ObjectId::new(4, 0);
        tree.add_page(Page::a4(), page_id, level2_id).unwrap();

        // Check counts at all levels
        assert_eq!(tree.page_count(), 1); // Root count

        match &tree.nodes[&level1_id] {
            PageTreeNode::Pages { count, .. } => assert_eq!(*count, 1),
            _ => panic!("Wrong node type"),
        }

        match &tree.nodes[&level2_id] {
            PageTreeNode::Pages { count, .. } => assert_eq!(*count, 1),
            _ => panic!("Wrong node type"),
        }
    }

    #[test]
    fn test_page_tree_node_to_dict_with_parent() {
        let root_id = ObjectId::new(1, 0);
        let mut tree = PageTree::new(root_id);

        let child_id = ObjectId::new(2, 0);
        tree.add_pages_node(child_id, Some(root_id)).unwrap();

        let dict = tree.node_to_dict(child_id).unwrap();
        assert_eq!(dict.get("Parent"), Some(&Object::Reference(root_id)));
    }

    #[test]
    fn test_page_tree_node_to_dict_root_no_parent() {
        let root_id = ObjectId::new(1, 0);
        let tree = PageTree::new(root_id);

        let dict = tree.node_to_dict(root_id).unwrap();
        assert!(dict.get("Parent").is_none());
    }

    #[test]
    fn test_page_tree_page_map() {
        let root_id = ObjectId::new(1, 0);
        let mut tree = PageTree::new(root_id);

        let page1_id = ObjectId::new(2, 0);
        let page2_id = ObjectId::new(3, 0);

        tree.add_page(Page::a4(), page1_id, root_id).unwrap();
        tree.add_page(Page::letter(), page2_id, root_id).unwrap();

        // Check page map
        assert_eq!(tree.page_map.get(&0), Some(&page1_id));
        assert_eq!(tree.page_map.get(&1), Some(&page2_id));
    }

    #[test]
    fn test_page_tree_different_page_sizes() {
        let mut builder = PageTreeBuilder::new(1);
        let pages = vec![Page::a4(), Page::letter(), Page::new(200.0, 300.0)];

        builder.add_pages(pages).unwrap();
        let tree = builder.build();

        assert_eq!(tree.page_count(), 3);

        // Verify different sizes
        let page0 = tree.get_page(0).unwrap();
        let page1 = tree.get_page(1).unwrap();
        let page2 = tree.get_page(2).unwrap();

        // Verify we got the pages (we can't access private fields)
        // A4, Letter, and custom sizes were added
        assert_eq!(tree.page_count(), 3);

        // Since we can't access the private width/height fields,
        // we just verify the pages exist
        assert!(std::ptr::eq(page0, tree.get_page(0).unwrap()));
        assert!(std::ptr::eq(page1, tree.get_page(1).unwrap()));
        assert!(std::ptr::eq(page2, tree.get_page(2).unwrap()));
    }

    #[test]
    fn test_page_tree_add_page_to_non_existent_parent() {
        let root_id = ObjectId::new(1, 0);
        let mut tree = PageTree::new(root_id);

        let page = Page::a4();
        let page_id = ObjectId::new(2, 0);
        let invalid_parent = ObjectId::new(999, 0);

        let result = tree.add_page(page, page_id, invalid_parent);
        assert!(result.is_err());
    }

    #[test]
    fn test_page_tree_empty_kids_array() {
        let root_id = ObjectId::new(1, 0);
        let tree = PageTree::new(root_id);

        let dict = tree.node_to_dict(root_id).unwrap();
        match dict.get("Kids") {
            Some(Object::Array(arr)) => {
                assert_eq!(arr.len(), 0);
            }
            _ => panic!("Kids should be an empty array"),
        }
    }

    #[test]
    fn test_page_tree_builder_large_number_of_pages() {
        let mut builder = PageTreeBuilder::new(1);
        let pages: Vec<Page> = (0..100).map(|_| Page::a4()).collect();

        builder.add_pages(pages).unwrap();
        let tree = builder.build();

        assert_eq!(tree.page_count(), 100);

        // Spot check some pages
        assert!(tree.get_page(0).is_some());
        assert!(tree.get_page(50).is_some());
        assert!(tree.get_page(99).is_some());
        assert!(tree.get_page(100).is_none());
    }

    #[test]
    fn test_page_tree_add_pages_node_without_parent() {
        let root_id = ObjectId::new(1, 0);
        let mut tree = PageTree::new(root_id);

        let orphan_id = ObjectId::new(2, 0);
        tree.add_pages_node(orphan_id, None).unwrap();

        // Orphan node should exist but not be connected
        assert!(tree.nodes.contains_key(&orphan_id));

        match &tree.nodes[&root_id] {
            PageTreeNode::Pages { kids, .. } => {
                assert_eq!(kids.len(), 0); // Root should have no kids
            }
            _ => panic!("Wrong node type"),
        }
    }
}
