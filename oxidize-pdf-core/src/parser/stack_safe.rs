//! Stack-safe parsing utilities
//!
//! This module provides utilities for parsing deeply nested PDF structures
//! without risking stack overflow. It implements recursion limits and
//! iterative alternatives to recursive algorithms.

use super::{ParseError, ParseResult};
use std::collections::HashSet;

/// Maximum recursion depth for PDF parsing operations
pub const MAX_RECURSION_DEPTH: usize = 1000;

/// Timeout for long-running parsing operations (in seconds)
pub const PARSING_TIMEOUT_SECS: u64 = 120; // Aumentado para documentos complejos

/// Stack-safe parsing context
#[derive(Debug)]
pub struct StackSafeContext {
    /// Current recursion depth
    pub depth: usize,
    /// Maximum allowed depth
    pub max_depth: usize,
    /// Pila de referencias activas (para detectar ciclos reales)
    pub active_stack: Vec<(u32, u16)>,
    /// Referencias completamente procesadas (no son ciclos)
    pub completed_refs: HashSet<(u32, u16)>,
    /// Start time for timeout tracking
    pub start_time: std::time::Instant,
    /// Timeout duration
    pub timeout: std::time::Duration,
}

impl Default for StackSafeContext {
    fn default() -> Self {
        Self::new()
    }
}

impl StackSafeContext {
    /// Create a new stack-safe context
    pub fn new() -> Self {
        Self {
            depth: 0,
            max_depth: MAX_RECURSION_DEPTH,
            active_stack: Vec::new(),
            completed_refs: HashSet::new(),
            start_time: std::time::Instant::now(),
            timeout: std::time::Duration::from_secs(PARSING_TIMEOUT_SECS),
        }
    }

    /// Create a new context with custom limits
    pub fn with_limits(max_depth: usize, timeout_secs: u64) -> Self {
        Self {
            depth: 0,
            max_depth,
            active_stack: Vec::new(),
            completed_refs: HashSet::new(),
            start_time: std::time::Instant::now(),
            timeout: std::time::Duration::from_secs(timeout_secs),
        }
    }

    /// Enter a new recursion level
    pub fn enter(&mut self) -> ParseResult<()> {
        if self.depth + 1 > self.max_depth {
            return Err(ParseError::SyntaxError {
                position: 0,
                message: format!(
                    "Maximum recursion depth exceeded: {} (limit: {})",
                    self.depth + 1,
                    self.max_depth
                ),
            });
        }
        self.depth += 1;
        self.check_timeout()?;
        Ok(())
    }

    /// Exit a recursion level
    pub fn exit(&mut self) {
        if self.depth > 0 {
            self.depth -= 1;
        }
    }

    /// Push a reference onto the active stack (for cycle detection)
    pub fn push_ref(&mut self, obj_num: u32, gen_num: u16) -> ParseResult<()> {
        let ref_key = (obj_num, gen_num);

        // Check if it's already in the active stack (real circular reference)
        if self.active_stack.contains(&ref_key) {
            return Err(ParseError::SyntaxError {
                position: 0,
                message: format!("Circular reference detected: {obj_num} {gen_num} R"),
            });
        }

        // It's OK if it was already processed completely
        self.active_stack.push(ref_key);
        Ok(())
    }

    /// Pop a reference from the active stack and mark as completed
    pub fn pop_ref(&mut self) {
        if let Some(ref_key) = self.active_stack.pop() {
            self.completed_refs.insert(ref_key);
        }
    }

    /// Check if parsing has timed out
    pub fn check_timeout(&self) -> ParseResult<()> {
        if self.start_time.elapsed() > self.timeout {
            return Err(ParseError::SyntaxError {
                position: 0,
                message: format!("Parsing timeout exceeded: {}s", self.timeout.as_secs()),
            });
        }
        Ok(())
    }

    /// Create a child context for nested operations
    pub fn child(&self) -> Self {
        Self {
            depth: self.depth,
            max_depth: self.max_depth,
            active_stack: self.active_stack.clone(),
            completed_refs: self.completed_refs.clone(),
            start_time: self.start_time,
            timeout: self.timeout,
        }
    }
}

/// RAII guard for recursion depth tracking
pub struct RecursionGuard<'a> {
    context: &'a mut StackSafeContext,
}

impl<'a> RecursionGuard<'a> {
    /// Create a new recursion guard
    pub fn new(context: &'a mut StackSafeContext) -> ParseResult<Self> {
        context.enter()?;
        Ok(Self { context })
    }
}

impl<'a> Drop for RecursionGuard<'a> {
    fn drop(&mut self) {
        self.context.exit();
    }
}

/// RAII guard for reference stack tracking
pub struct ReferenceStackGuard<'a> {
    context: &'a mut StackSafeContext,
}

impl<'a> ReferenceStackGuard<'a> {
    /// Create a new reference stack guard
    pub fn new(context: &'a mut StackSafeContext, obj_num: u32, gen_num: u16) -> ParseResult<Self> {
        context.push_ref(obj_num, gen_num)?;
        Ok(Self { context })
    }
}

impl<'a> Drop for ReferenceStackGuard<'a> {
    fn drop(&mut self) {
        self.context.pop_ref();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== StackSafeContext Tests ====================

    #[test]
    fn test_stack_safe_context_new() {
        let context = StackSafeContext::new();
        assert_eq!(context.depth, 0);
        assert_eq!(context.max_depth, MAX_RECURSION_DEPTH);
        assert!(context.active_stack.is_empty());
        assert!(context.completed_refs.is_empty());
    }

    #[test]
    fn test_stack_safe_context_default() {
        let context = StackSafeContext::default();
        assert_eq!(context.depth, 0);
        assert_eq!(context.max_depth, MAX_RECURSION_DEPTH);
    }

    #[test]
    fn test_stack_safe_context_with_limits() {
        let context = StackSafeContext::with_limits(50, 30);
        assert_eq!(context.depth, 0);
        assert_eq!(context.max_depth, 50);
        assert_eq!(context.timeout.as_secs(), 30);
    }

    #[test]
    fn test_recursion_limits() {
        let mut context = StackSafeContext::with_limits(3, 60);

        // Should work within limits
        assert!(context.enter().is_ok());
        assert_eq!(context.depth, 1);

        assert!(context.enter().is_ok());
        assert_eq!(context.depth, 2);

        assert!(context.enter().is_ok());
        assert_eq!(context.depth, 3);

        // Should fail when exceeding limit
        assert!(context.enter().is_err());

        // Test exit
        context.exit();
        assert_eq!(context.depth, 2);
    }

    #[test]
    fn test_enter_increments_depth() {
        let mut context = StackSafeContext::new();
        assert_eq!(context.depth, 0);

        context.enter().unwrap();
        assert_eq!(context.depth, 1);

        context.enter().unwrap();
        assert_eq!(context.depth, 2);
    }

    #[test]
    fn test_exit_decrements_depth() {
        let mut context = StackSafeContext::new();
        context.enter().unwrap();
        context.enter().unwrap();
        assert_eq!(context.depth, 2);

        context.exit();
        assert_eq!(context.depth, 1);

        context.exit();
        assert_eq!(context.depth, 0);
    }

    #[test]
    fn test_exit_at_zero_does_not_underflow() {
        let mut context = StackSafeContext::new();
        assert_eq!(context.depth, 0);

        context.exit(); // Should not underflow
        assert_eq!(context.depth, 0);

        context.exit(); // Multiple exits at zero
        assert_eq!(context.depth, 0);
    }

    #[test]
    fn test_cycle_detection() {
        let mut context = StackSafeContext::new();

        // First push should work
        assert!(context.push_ref(1, 0).is_ok());

        // Second push of same ref should fail (circular)
        assert!(context.push_ref(1, 0).is_err());

        // Different ref should work
        assert!(context.push_ref(2, 0).is_ok());

        // Pop refs
        context.pop_ref(); // pops 2,0
        context.pop_ref(); // pops 1,0

        // Now we can push 1,0 again
        assert!(context.push_ref(1, 0).is_ok());
    }

    #[test]
    fn test_push_ref_adds_to_active_stack() {
        let mut context = StackSafeContext::new();
        assert!(context.active_stack.is_empty());

        context.push_ref(10, 5).unwrap();
        assert_eq!(context.active_stack.len(), 1);
        assert!(context.active_stack.contains(&(10, 5)));
    }

    #[test]
    fn test_pop_ref_marks_as_completed() {
        let mut context = StackSafeContext::new();
        context.push_ref(7, 3).unwrap();
        assert!(context.completed_refs.is_empty());

        context.pop_ref();
        assert!(context.active_stack.is_empty());
        assert!(context.completed_refs.contains(&(7, 3)));
    }

    #[test]
    fn test_pop_ref_on_empty_stack() {
        let mut context = StackSafeContext::new();
        assert!(context.active_stack.is_empty());

        // Should not panic
        context.pop_ref();
        assert!(context.active_stack.is_empty());
    }

    #[test]
    fn test_multiple_refs_stack_order() {
        let mut context = StackSafeContext::new();
        context.push_ref(1, 0).unwrap();
        context.push_ref(2, 0).unwrap();
        context.push_ref(3, 0).unwrap();

        assert_eq!(context.active_stack.len(), 3);

        context.pop_ref(); // pops 3,0
        assert!(context.completed_refs.contains(&(3, 0)));
        assert!(!context.completed_refs.contains(&(2, 0)));
        assert!(!context.completed_refs.contains(&(1, 0)));

        context.pop_ref(); // pops 2,0
        assert!(context.completed_refs.contains(&(2, 0)));

        context.pop_ref(); // pops 1,0
        assert!(context.completed_refs.contains(&(1, 0)));
    }

    #[test]
    fn test_check_timeout_within_limit() {
        let context = StackSafeContext::with_limits(100, 60);
        // Should be well within timeout
        assert!(context.check_timeout().is_ok());
    }

    #[test]
    fn test_child_context() {
        let mut context = StackSafeContext::with_limits(50, 30);
        context.enter().unwrap();
        context.enter().unwrap();
        context.push_ref(5, 0).unwrap();
        context.pop_ref(); // completes 5,0

        let child = context.child();
        assert_eq!(child.depth, context.depth);
        assert_eq!(child.max_depth, context.max_depth);
        assert!(child.completed_refs.contains(&(5, 0)));
    }

    #[test]
    fn test_child_context_is_independent() {
        let mut context = StackSafeContext::new();
        context.enter().unwrap();

        let child = context.child();
        assert_eq!(child.depth, 1);

        // Original context still has depth 1
        context.exit();
        assert_eq!(context.depth, 0);
        // Child should still be at 1 (cloned state)
    }

    #[test]
    fn test_different_generation_numbers() {
        let mut context = StackSafeContext::new();
        // Same object number but different generations
        context.push_ref(1, 0).unwrap();
        context.push_ref(1, 1).unwrap(); // Different gen
        context.push_ref(1, 2).unwrap(); // Different gen

        assert_eq!(context.active_stack.len(), 3);
    }

    // ==================== RecursionGuard Tests ====================

    #[test]
    fn test_recursion_guard() {
        let mut context = StackSafeContext::new();
        assert_eq!(context.depth, 0);

        {
            let _guard = RecursionGuard::new(&mut context).unwrap();
            // Can't access context.depth while guard is active due to borrow checker
        }

        // Should auto-exit when guard drops
        assert_eq!(context.depth, 0);
    }

    #[test]
    fn test_recursion_guard_nesting() {
        let mut context = StackSafeContext::with_limits(10, 60);

        {
            let _guard1 = RecursionGuard::new(&mut context).unwrap();
            // depth is 1 but can't access directly
        }
        assert_eq!(context.depth, 0);

        // Manually test multiple enters/exits
        context.enter().unwrap();
        context.enter().unwrap();
        assert_eq!(context.depth, 2);
    }

    #[test]
    fn test_recursion_guard_fails_at_limit() {
        let mut context = StackSafeContext::with_limits(1, 60);
        context.enter().unwrap(); // Already at limit

        let result = RecursionGuard::new(&mut context);
        assert!(result.is_err());
    }

    // ==================== ReferenceStackGuard Tests ====================

    #[test]
    fn test_reference_stack_guard() {
        let mut context = StackSafeContext::new();

        {
            let _guard = ReferenceStackGuard::new(&mut context, 1, 0).unwrap();
            // Reference is in active stack while guard is active
            // Note: Can't check stack length here due to borrow checker constraints
        }

        // Should auto-pop when guard drops
        assert_eq!(context.active_stack.len(), 0);
        assert!(context.completed_refs.contains(&(1, 0)));

        // Can visit again after guard is dropped
        assert!(context.push_ref(1, 0).is_ok());
    }

    #[test]
    fn test_reference_stack_guard_circular_detection() {
        let mut context = StackSafeContext::new();
        context.push_ref(5, 0).unwrap();

        // Should fail - circular reference
        let result = ReferenceStackGuard::new(&mut context, 5, 0);
        assert!(result.is_err());
    }

    // ==================== Constants Tests ====================

    #[test]
    fn test_constants() {
        assert_eq!(MAX_RECURSION_DEPTH, 1000);
        assert_eq!(PARSING_TIMEOUT_SECS, 120);
    }

    // ==================== Debug Tests ====================

    #[test]
    fn test_stack_safe_context_debug() {
        let context = StackSafeContext::new();
        let debug_str = format!("{:?}", context);
        assert!(debug_str.contains("StackSafeContext"));
        assert!(debug_str.contains("depth: 0"));
    }

    // ==================== Integration Tests ====================

    #[test]
    fn test_deep_recursion_simulation() {
        let mut context = StackSafeContext::with_limits(100, 60);

        // Simulate deep recursion
        for i in 0..100 {
            assert!(context.enter().is_ok(), "Failed at depth {}", i);
        }
        assert_eq!(context.depth, 100);

        // Should fail at 101
        assert!(context.enter().is_err());

        // Unwind
        for _ in 0..100 {
            context.exit();
        }
        assert_eq!(context.depth, 0);
    }

    #[test]
    fn test_complex_reference_scenario() {
        let mut context = StackSafeContext::new();

        // Simulate traversing a PDF object tree
        context.push_ref(1, 0).unwrap(); // Enter root
        context.push_ref(2, 0).unwrap(); // Enter child
        context.push_ref(3, 0).unwrap(); // Enter grandchild

        // Can't reference grandchild again (circular)
        assert!(context.push_ref(3, 0).is_err());

        // But can reference a different object
        context.push_ref(4, 0).unwrap();

        // Pop all
        context.pop_ref();
        context.pop_ref();
        context.pop_ref();
        context.pop_ref();

        // All should be completed
        assert!(context.completed_refs.contains(&(1, 0)));
        assert!(context.completed_refs.contains(&(2, 0)));
        assert!(context.completed_refs.contains(&(3, 0)));
        assert!(context.completed_refs.contains(&(4, 0)));

        // Can now revisit any
        context.push_ref(3, 0).unwrap();
    }
}
