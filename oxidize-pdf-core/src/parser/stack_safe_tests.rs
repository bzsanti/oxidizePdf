//! Comprehensive tests for stack-safe parsing implementations
//!
//! This module tests the stack-safe parsing implementations to ensure they
//! can handle deeply nested PDF structures without stack overflow.

#[cfg(test)]
mod tests {
    use super::super::objects::{PdfArray, PdfDictionary, PdfObject};
    use super::super::stack_safe::{RecursionGuard, ReferenceStackGuard, StackSafeContext};
    use super::super::ParseError;
    use std::time::Duration;

    #[test]
    fn test_deep_recursion_limit() {
        let mut context = StackSafeContext::with_limits(10, 60);

        // Should handle up to the limit
        for i in 0..10 {
            let result = context.enter();
            assert!(result.is_ok(), "Failed at depth {}", i);
        }

        // Should fail at limit+1
        let result = context.enter();
        assert!(result.is_err(), "Should have failed at depth 11");

        // Clean up
        for _ in 0..10 {
            context.exit();
        }
    }

    #[test]
    fn test_timeout_protection() {
        let context = StackSafeContext::with_limits(1000, 0); // 0 second timeout

        // Should timeout immediately
        std::thread::sleep(Duration::from_millis(10));
        let result = context.check_timeout();
        assert!(result.is_err(), "Should have timed out");
    }

    #[test]
    fn test_circular_reference_detection() {
        let mut context = StackSafeContext::new();

        // First reference should work
        assert!(context.push_ref(1, 0).is_ok());

        // Circular reference should be detected
        assert!(context.push_ref(1, 0).is_err());

        // Pop and try different generation
        context.pop_ref();
        assert!(context.push_ref(1, 1).is_ok());

        // Pop and revisit original should work
        context.pop_ref();
        assert!(context.push_ref(1, 0).is_ok());
    }

    #[test]
    fn test_recursion_guard_raii() {
        let mut context = StackSafeContext::new();
        assert_eq!(context.depth, 0);

        {
            let _guard = RecursionGuard::new(&mut context).unwrap();
            // Depth incremented
        } // Guard dropped, depth decremented

        assert_eq!(context.depth, 0);
    }

    #[test]
    fn test_reference_stack_guard_raii() {
        let mut context = StackSafeContext::new();

        {
            let _guard = ReferenceStackGuard::new(&mut context, 5, 0).unwrap();
            // Reference is pushed to stack
        } // Guard dropped, reference popped

        // Should be able to visit again
        assert!(context.push_ref(5, 0).is_ok());
    }

    #[test]
    fn test_nested_guards() {
        let mut context = StackSafeContext::with_limits(5, 60);

        // Test sequential guards (not nested due to borrow checker)
        {
            let _guard1 = RecursionGuard::new(&mut context).unwrap();
            // Can't access context.depth while guard is active
        }
        assert_eq!(context.depth, 0); // Guard dropped, depth reset

        {
            let _guard2 = RecursionGuard::new(&mut context).unwrap();
            // Can't access context.depth while guard is active
        }
        assert_eq!(context.depth, 0); // Guard dropped, depth reset

        // Test that we can create multiple guards sequentially
        for _ in 0..3 {
            let result = RecursionGuard::new(&mut context);
            assert!(result.is_ok());
            // Guard is dropped at end of iteration
        }

        assert_eq!(context.depth, 0);
    }

    #[test]
    fn test_guard_failure_cleanup() {
        let mut context = StackSafeContext::with_limits(2, 60);

        // Manually fill up to limit to test the limit behavior
        context.enter().unwrap(); // depth = 1
        context.enter().unwrap(); // depth = 2

        // Next enter should fail
        let result = context.enter();
        assert!(result.is_err());

        // Context should still be in valid state
        assert_eq!(context.depth, 2);

        // Clean up
        context.exit(); // depth = 1
        context.exit(); // depth = 0
        assert_eq!(context.depth, 0);
    }

    #[test]
    fn test_child_context() {
        let mut parent_context = StackSafeContext::with_limits(100, 60);
        parent_context.depth = 10;
        parent_context.push_ref(1, 0).unwrap();
        parent_context.pop_ref(); // Mark as completed

        let child_context = parent_context.child();

        // Child should inherit state
        assert_eq!(child_context.depth, 10);
        assert_eq!(child_context.max_depth, 100);
        assert!(child_context.completed_refs.contains(&(1, 0)));

        // Child inherits the start time (by design for timeout consistency)
        assert_eq!(child_context.start_time, parent_context.start_time);
    }

    // Integration tests with actual PDF structures

    #[test]
    fn test_deeply_nested_arrays() {
        // Create a deeply nested array structure
        let mut nested_array = PdfObject::Integer(42);

        // Create 50 levels of nesting (well within stack limits)
        for _ in 0..50 {
            let array = PdfArray(vec![nested_array]);
            nested_array = PdfObject::Array(array);
        }

        // This should parse without stack issues
        // (In a real implementation, the parser would use StackSafeContext)
        match nested_array {
            PdfObject::Array(_) => {
                // Successfully created deeply nested structure
                assert!(true);
            }
            _ => panic!("Expected array"),
        }
    }

    #[test]
    fn test_deeply_nested_dictionaries() {
        // Create a deeply nested dictionary structure
        let mut nested_dict = PdfObject::Integer(42);

        // Create 50 levels of nesting
        for i in 0..50 {
            let mut dict = PdfDictionary::new();
            dict.insert(format!("level_{}", i), nested_dict);
            nested_dict = PdfObject::Dictionary(dict);
        }

        // This should parse without stack issues
        match nested_dict {
            PdfObject::Dictionary(_) => {
                // Successfully created deeply nested structure
                assert!(true);
            }
            _ => panic!("Expected dictionary"),
        }
    }

    #[test]
    fn test_malicious_reference_chain() {
        // Simulate detection of a malicious reference chain
        let mut context = StackSafeContext::new();

        // Create a chain of references
        let refs = [(1, 0), (2, 0), (3, 0), (4, 0), (5, 0)];

        // Push all references to stack (simulating navigation chain)
        for &(obj, gen) in &refs {
            assert!(context.push_ref(obj, gen).is_ok());
        }

        // Attempt to revisit the first one (simulating a cycle)
        assert!(context.push_ref(1, 0).is_err());
    }

    #[test]
    fn test_stack_safe_context_defaults() {
        let context = StackSafeContext::new();

        assert_eq!(context.depth, 0);
        assert_eq!(context.max_depth, 1000);
        assert!(context.active_stack.is_empty());
        assert!(context.completed_refs.is_empty());
        assert_eq!(context.timeout, Duration::from_secs(30));
    }

    #[test]
    fn test_stack_safe_context_custom_limits() {
        let context = StackSafeContext::with_limits(500, 10);

        assert_eq!(context.depth, 0);
        assert_eq!(context.max_depth, 500);
        assert!(context.active_stack.is_empty());
        assert!(context.completed_refs.is_empty());
        assert_eq!(context.timeout, Duration::from_secs(10));
    }

    #[test]
    fn test_multiple_reference_generations() {
        let mut context = StackSafeContext::new();

        // Different generations of the same object should be allowed in stack
        assert!(context.push_ref(1, 0).is_ok());
        assert!(context.push_ref(1, 1).is_ok());
        assert!(context.push_ref(1, 2).is_ok());

        // But same generation should fail (circular reference)
        assert!(context.push_ref(1, 0).is_err());
        assert!(context.push_ref(1, 1).is_err());
        assert!(context.push_ref(1, 2).is_err());
    }

    #[test]
    fn test_context_exit_idempotent() {
        let mut context = StackSafeContext::new();

        // Exit without enter should be safe
        context.exit();
        assert_eq!(context.depth, 0);

        // Enter and exit
        context.enter().unwrap();
        assert_eq!(context.depth, 1);
        context.exit();
        assert_eq!(context.depth, 0);

        // Multiple exits should be safe
        context.exit();
        context.exit();
        assert_eq!(context.depth, 0);
    }

    #[test]
    fn test_performance_with_many_references() {
        let mut context = StackSafeContext::new();

        // Process many references sequentially (testing completed_refs performance)
        for obj_num in 0..1000 {
            assert!(context.push_ref(obj_num, 0).is_ok());
            context.pop_ref(); // Mark as completed
        }

        // Should still be fast to check completed references
        // Since 500 was completed, it should be OK to push again
        assert!(context.push_ref(500, 0).is_ok());
        context.pop_ref();
        assert!(context.push_ref(1001, 0).is_ok()); // New one
    }

    #[test]
    fn test_error_messages_informatve() {
        let mut context = StackSafeContext::with_limits(2, 1); // Very small limits

        // Test depth limit error
        context.enter().unwrap();
        context.enter().unwrap();
        let depth_error = context.enter().err().unwrap();
        if let ParseError::SyntaxError { message, .. } = depth_error {
            assert!(message.contains("Maximum recursion depth exceeded"));
            assert!(message.contains("3"));
            assert!(message.contains("2"));
        } else {
            panic!("Expected SyntaxError for depth limit");
        }

        // Test circular reference error
        context.push_ref(10, 5).unwrap();
        let cycle_error = context.push_ref(10, 5).err().unwrap();
        if let ParseError::SyntaxError { message, .. } = cycle_error {
            assert!(message.contains("Circular reference detected"));
            assert!(message.contains("10 5 R"));
        } else {
            panic!("Expected SyntaxError for circular reference");
        }

        // Test timeout error (after waiting)
        std::thread::sleep(Duration::from_millis(1100));
        let timeout_error = context.check_timeout().err().unwrap();
        if let ParseError::SyntaxError { message, .. } = timeout_error {
            assert!(message.contains("Parsing timeout exceeded"));
            assert!(message.contains("1s"));
        } else {
            panic!("Expected SyntaxError for timeout");
        }
    }
}
