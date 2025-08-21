//! Tests for Array object functionality

use super::*;
use crate::objects::Object;

#[test]
fn test_array_new() {
    let array = Array::new();
    assert_eq!(array.len(), 0, "New array should be empty");
    assert!(array.is_empty(), "New array should be empty");
}

#[test]
fn test_array_with_capacity() {
    let array = Array::with_capacity(10);
    assert_eq!(array.len(), 0, "Array with capacity should start empty");
    assert!(array.is_empty(), "Array with capacity should be empty");
}

#[test]
fn test_array_push_and_len() {
    let mut array = Array::new();

    array.push(Object::Integer(42));
    assert_eq!(array.len(), 1, "Array should have one element after push");
    assert!(!array.is_empty(), "Array should not be empty after push");

    array.push(Object::Boolean(true));
    assert_eq!(
        array.len(),
        2,
        "Array should have two elements after second push"
    );
}

#[test]
fn test_array_pop() {
    let mut array = Array::new();

    // Test pop on empty array
    assert!(
        array.pop().is_none(),
        "Pop on empty array should return None"
    );

    // Add elements and test pop
    array.push(Object::Integer(42));
    array.push(Object::Boolean(true));

    let popped = array.pop();
    assert!(popped.is_some(), "Should pop an element");
    assert_eq!(array.len(), 1, "Array should have one element after pop");

    let popped = array.pop();
    assert!(popped.is_some(), "Should pop remaining element");
    assert_eq!(
        array.len(),
        0,
        "Array should be empty after popping all elements"
    );
    assert!(
        array.is_empty(),
        "Array should be empty after popping all elements"
    );
}

#[test]
fn test_array_insert() {
    let mut array = Array::new();

    // Insert into empty array
    array.insert(0, Object::Integer(42));
    assert_eq!(array.len(), 1, "Array should have one element after insert");

    // Insert at beginning
    array.insert(0, Object::Boolean(true));
    assert_eq!(array.len(), 2, "Array should have two elements");

    // Insert at end
    array.insert(2, Object::Integer(100));
    assert_eq!(array.len(), 3, "Array should have three elements");

    // Insert in middle
    array.insert(1, Object::Integer(50));
    assert_eq!(array.len(), 4, "Array should have four elements");
}

#[test]
fn test_array_remove() {
    let mut array = Array::new();
    array.push(Object::Integer(10));
    array.push(Object::Integer(20));
    array.push(Object::Integer(30));

    // Remove middle element
    let _removed = array.remove(1);
    assert_eq!(
        array.len(),
        2,
        "Array should have two elements after remove"
    );

    // Remove first element
    let _removed = array.remove(0);
    assert_eq!(array.len(), 1, "Array should have one element after remove");
}

#[test]
#[should_panic]
fn test_array_remove_out_of_bounds() {
    let mut array = Array::new();
    array.push(Object::Integer(42));
    array.remove(1); // Should panic - index out of bounds
}

#[test]
fn test_array_get() {
    let mut array = Array::new();

    // Test get on empty array
    assert!(
        array.get(0).is_none(),
        "Get on empty array should return None"
    );

    // Add elements and test get
    array.push(Object::Integer(42));
    array.push(Object::Boolean(true));

    assert!(array.get(0).is_some(), "Should get first element");
    assert!(array.get(1).is_some(), "Should get second element");
    assert!(
        array.get(2).is_none(),
        "Get out of bounds should return None"
    );

    // Verify the types
    if let Some(Object::Integer(val)) = array.get(0) {
        assert_eq!(*val, 42, "First element should be integer 42");
    }
    if let Some(Object::Boolean(val)) = array.get(1) {
        assert!(*val, "Second element should be boolean true");
    }
}

#[test]
fn test_array_get_mut() {
    let mut array = Array::new();

    // Test get_mut on empty array
    assert!(
        array.get_mut(0).is_none(),
        "Get_mut on empty array should return None"
    );

    // Add elements and test get_mut
    array.push(Object::Integer(42));
    array.push(Object::Boolean(true));

    // Modify element via get_mut
    if let Some(element) = array.get_mut(0) {
        *element = Object::Integer(100);
    }

    assert!(array.get(0).is_some(), "Element should exist");
    assert!(array.get(1).is_some(), "Other elements should remain");
    assert!(
        array.get_mut(2).is_none(),
        "Get_mut out of bounds should return None"
    );

    // Verify the modification
    if let Some(Object::Integer(val)) = array.get(0) {
        assert_eq!(*val, 100, "Element should be modified to 100");
    }
}

#[test]
fn test_array_clear() {
    let mut array = Array::new();
    array.push(Object::Integer(42));
    array.push(Object::Boolean(true));
    array.push(Object::Integer(100));

    assert_eq!(
        array.len(),
        3,
        "Array should have three elements before clear"
    );

    array.clear();

    assert_eq!(array.len(), 0, "Array should be empty after clear");
    assert!(array.is_empty(), "Array should be empty after clear");
    assert!(
        array.get(0).is_none(),
        "No elements should be accessible after clear"
    );
}

#[test]
fn test_array_iter() {
    let mut array = Array::new();
    array.push(Object::Integer(10));
    array.push(Object::Integer(20));
    array.push(Object::Integer(30));

    let collected: Vec<&Object> = array.iter().collect();
    assert_eq!(collected.len(), 3, "Iterator should yield all elements");

    // Test iterator on empty array
    let empty_array = Array::new();
    let empty_collected: Vec<&Object> = empty_array.iter().collect();
    assert_eq!(
        empty_collected.len(),
        0,
        "Empty array iterator should yield no elements"
    );
}

#[test]
fn test_array_iter_mut() {
    let mut array = Array::new();
    array.push(Object::Integer(10));
    array.push(Object::Integer(20));
    array.push(Object::Integer(30));

    // Count elements via iter_mut
    let mut count = 0;
    for _element in array.iter_mut() {
        count += 1;
    }

    assert_eq!(count, 3, "iter_mut should iterate over all elements");
}

#[test]
fn test_array_default() {
    let array: Array = Default::default();
    assert_eq!(array.len(), 0, "Default array should be empty");
    assert!(array.is_empty(), "Default array should be empty");
}

#[test]
fn test_array_from_vec() {
    let vec = vec![
        Object::Integer(10),
        Object::Boolean(true),
        Object::Integer(20),
    ];

    let array = Array::from(vec);
    assert_eq!(array.len(), 3, "Array from vec should have same length");
}

#[test]
fn test_array_into_vec() {
    let mut array = Array::new();
    array.push(Object::Integer(10));
    array.push(Object::Boolean(true));
    array.push(Object::Integer(20));

    let vec: Vec<Object> = array.into();
    assert_eq!(vec.len(), 3, "Vec from array should have same length");
}

#[test]
fn test_array_from_iterator() {
    let objects = vec![Object::Integer(1), Object::Integer(2), Object::Integer(3)];

    let array: Array = objects.into_iter().collect();
    assert_eq!(
        array.len(),
        3,
        "Array from iterator should have correct length"
    );

    // Test empty iterator
    let empty_objects: Vec<Object> = vec![];
    let empty_array: Array = empty_objects.into_iter().collect();
    assert_eq!(
        empty_array.len(),
        0,
        "Array from empty iterator should be empty"
    );
    assert!(
        empty_array.is_empty(),
        "Array from empty iterator should be empty"
    );
}

#[test]
fn test_array_mixed_types() {
    let mut array = Array::new();
    array.push(Object::Integer(42));
    array.push(Object::Boolean(true));
    array.push(Object::Null);

    assert_eq!(array.len(), 3, "Array should handle mixed types");

    // Verify types
    if let Some(Object::Integer(_)) = array.get(0) {
        // Integer type confirmed
    } else {
        panic!("First element should be integer");
    }

    if let Some(Object::Boolean(_)) = array.get(1) {
        // Boolean type confirmed
    } else {
        panic!("Second element should be boolean");
    }

    if let Some(Object::Null) = array.get(2) {
        // Null type confirmed
    } else {
        panic!("Third element should be null");
    }
}

#[test]
fn test_array_large_operations() {
    let mut array = Array::new();

    // Add many elements
    for i in 0..100 {
        array.push(Object::Integer(i));
    }

    assert_eq!(
        array.len(),
        100,
        "Array should handle large number of elements"
    );
    assert!(array.get(0).is_some(), "First element should exist");
    assert!(array.get(99).is_some(), "Last element should exist");
    assert!(
        array.get(100).is_none(),
        "Out of bounds access should return None"
    );

    // Remove many elements
    for _ in 0..50 {
        array.pop();
    }

    assert_eq!(
        array.len(),
        50,
        "Array should have correct length after removals"
    );
    assert!(array.get(49).is_some(), "Last element should exist");
    assert!(
        array.get(50).is_none(),
        "Beyond last element should return None"
    );
}

#[test]
fn test_array_clone() {
    let mut original = Array::new();
    original.push(Object::Integer(42));
    original.push(Object::Boolean(true));

    let cloned = original.clone();

    assert_eq!(
        cloned.len(),
        original.len(),
        "Cloned array should have same length"
    );

    // Verify they are independent
    original.push(Object::Integer(100));
    assert_eq!(original.len(), 3, "Original should have 3 elements");
    assert_eq!(cloned.len(), 2, "Clone should still have 2 elements");
}

#[test]
fn test_array_debug() {
    let mut array = Array::new();
    array.push(Object::Integer(42));
    array.push(Object::Boolean(true));

    let debug_str = format!("{array:?}");
    assert!(
        debug_str.contains("Array"),
        "Debug output should contain Array"
    );
    // Note: We don't test exact format as it may change, just that it doesn't panic
}
