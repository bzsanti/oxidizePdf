//! Simple test for the new architecture without file I/O

use oxidize_pdf::parser::objects::PdfDictionary;
use oxidize_pdf::parser::page_tree::{PageTree, ParsedPage};
use oxidize_pdf::parser::ResourceManager;

#[test]
fn test_resource_manager() {
    let manager = ResourceManager::new();

    // Test caching
    let obj_ref = (1, 0);
    let obj = oxidize_pdf::parser::PdfObject::Integer(42);

    // Initially empty
    assert!(manager.get_cached(obj_ref).is_none());

    // Cache an object
    manager.cache_object(obj_ref, obj.clone());

    // Should be cached now
    assert!(manager.get_cached(obj_ref).is_some());
    assert_eq!(manager.get_cached(obj_ref).unwrap(), obj);

    // Clear cache
    manager.clear_cache();
    assert!(manager.get_cached(obj_ref).is_none());
}

#[test]
fn test_page_tree_caching() {
    let mut page_tree = PageTree::new(5);

    // Initially no pages cached
    assert!(page_tree.get_cached_page(0).is_none());

    // Create a test page
    let page = ParsedPage {
        obj_ref: (1, 0),
        dict: PdfDictionary::new(),
        inherited_resources: None,
        media_box: [0.0, 0.0, 612.0, 792.0],
        crop_box: None,
        rotation: 0,
        annotations: None,
    };

    // Cache it
    page_tree.cache_page(0, page.clone());

    // Should be cached now
    assert!(page_tree.get_cached_page(0).is_some());
    assert_eq!(page_tree.get_cached_page(0).unwrap().obj_ref, (1, 0));

    // Test page dimensions
    let cached_page = page_tree.get_cached_page(0).unwrap();
    assert_eq!(cached_page.width(), 612.0);
    assert_eq!(cached_page.height(), 792.0);
}

#[test]
fn test_parsed_page_dimensions() {
    // Test normal orientation
    let page = ParsedPage {
        obj_ref: (1, 0),
        dict: PdfDictionary::new(),
        inherited_resources: None,
        media_box: [0.0, 0.0, 612.0, 792.0],
        crop_box: None,
        rotation: 0,
        annotations: None,
    };

    assert_eq!(page.width(), 612.0);
    assert_eq!(page.height(), 792.0);

    // Test 90 degree rotation - dimensions should swap
    let rotated_page = ParsedPage {
        obj_ref: (1, 0),
        dict: PdfDictionary::new(),
        inherited_resources: None,
        media_box: [0.0, 0.0, 612.0, 792.0],
        crop_box: None,
        rotation: 90,
        annotations: None,
    };

    assert_eq!(rotated_page.width(), 792.0);
    assert_eq!(rotated_page.height(), 612.0);

    // Test 270 degree rotation - dimensions should also swap
    let rotated_page_270 = ParsedPage {
        obj_ref: (1, 0),
        dict: PdfDictionary::new(),
        inherited_resources: None,
        media_box: [0.0, 0.0, 612.0, 792.0],
        crop_box: None,
        rotation: 270,
        annotations: None,
    };

    assert_eq!(rotated_page_270.width(), 792.0);
    assert_eq!(rotated_page_270.height(), 612.0);

    // Test 180 degree rotation - dimensions stay the same
    let rotated_page_180 = ParsedPage {
        obj_ref: (1, 0),
        dict: PdfDictionary::new(),
        inherited_resources: None,
        media_box: [0.0, 0.0, 612.0, 792.0],
        crop_box: None,
        rotation: 180,
        annotations: None,
    };

    assert_eq!(rotated_page_180.width(), 612.0);
    assert_eq!(rotated_page_180.height(), 792.0);
}
