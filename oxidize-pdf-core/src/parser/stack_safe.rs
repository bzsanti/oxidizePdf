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
pub const PARSING_TIMEOUT_SECS: u64 = 30;

/// Stack-safe parsing context
#[derive(Debug)]
pub struct StackSafeContext {
    /// Current recursion depth
    pub depth: usize,
    /// Maximum allowed depth
    pub max_depth: usize,
    /// Set of visited object references to detect cycles
    pub visited_refs: HashSet<(u32, u16)>,
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
            visited_refs: HashSet::new(),
            start_time: std::time::Instant::now(),
            timeout: std::time::Duration::from_secs(PARSING_TIMEOUT_SECS),
        }
    }

    /// Create a new context with custom limits
    pub fn with_limits(max_depth: usize, timeout_secs: u64) -> Self {
        Self {
            depth: 0,
            max_depth,
            visited_refs: HashSet::new(),
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

    /// Check if we've visited an object reference (for cycle detection)
    pub fn visit_ref(&mut self, obj_num: u32, gen_num: u16) -> ParseResult<()> {
        let ref_key = (obj_num, gen_num);
        if self.visited_refs.contains(&ref_key) {
            return Err(ParseError::SyntaxError {
                position: 0,
                message: format!("Circular reference detected: {} {} R", obj_num, gen_num),
            });
        }
        self.visited_refs.insert(ref_key);
        Ok(())
    }

    /// Mark a reference as no longer being processed
    pub fn unvisit_ref(&mut self, obj_num: u32, gen_num: u16) {
        self.visited_refs.remove(&(obj_num, gen_num));
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
            visited_refs: self.visited_refs.clone(),
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

/// RAII guard for reference tracking
pub struct ReferenceGuard<'a> {
    context: &'a mut StackSafeContext,
    obj_num: u32,
    gen_num: u16,
}

impl<'a> ReferenceGuard<'a> {
    /// Create a new reference guard
    pub fn new(context: &'a mut StackSafeContext, obj_num: u32, gen_num: u16) -> ParseResult<Self> {
        context.visit_ref(obj_num, gen_num)?;
        Ok(Self {
            context,
            obj_num,
            gen_num,
        })
    }
}

impl<'a> Drop for ReferenceGuard<'a> {
    fn drop(&mut self) {
        self.context.unvisit_ref(self.obj_num, self.gen_num);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_cycle_detection() {
        let mut context = StackSafeContext::new();

        // First visit should work
        assert!(context.visit_ref(1, 0).is_ok());

        // Second visit to same ref should fail
        assert!(context.visit_ref(1, 0).is_err());

        // Different ref should work
        assert!(context.visit_ref(2, 0).is_ok());

        // Unvisit and try again
        context.unvisit_ref(1, 0);
        assert!(context.visit_ref(1, 0).is_ok());
    }

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
    fn test_reference_guard() {
        let mut context = StackSafeContext::new();

        {
            let _guard = ReferenceGuard::new(&mut context, 1, 0).unwrap();
            // Can't access context while guard is active due to borrow checker
        }

        // Should auto-unvisit when guard drops
        assert!(!context.visited_refs.contains(&(1, 0)));

        // Can visit again after guard is dropped
        assert!(context.visit_ref(1, 0).is_ok());
    }
}
