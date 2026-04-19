//! CFF charstring desubroutinization.
//!
//! Inlines all subroutine calls (callsubr/callgsubr) to produce
//! self-contained charstrings with no external references.

use super::index::CffIndex;
use crate::parser::{ParseError, ParseResult};

/// Calculate subroutine bias per CFF spec.
/// - count < 1240:  bias = 107
/// - count < 33900: bias = 1131
/// - else:          bias = 32768
pub fn cff_subr_bias(count: usize) -> i32 {
    if count < 1240 {
        107
    } else if count < 33900 {
        1131
    } else {
        32768
    }
}

/// Desubroutinize a charstring by recursively inlining all subroutine calls.
///
/// Returns a new charstring with all `callsubr` (10), `callgsubr` (29), and
/// `return` (11) operators removed, and subroutine bodies inlined in place.
///
/// `global_subrs` and `local_subrs` are the parsed CFF INDEX structures for
/// global and local (per-FD) subroutines respectively.
pub fn desubroutinize(
    _charstring: &[u8],
    _global_subrs: &CffIndex,
    _local_subrs: &CffIndex,
) -> ParseResult<Vec<u8>> {
    Err(ParseError::SyntaxError {
        position: 0,
        message: "desubroutinize not yet implemented".to_string(),
    })
}
