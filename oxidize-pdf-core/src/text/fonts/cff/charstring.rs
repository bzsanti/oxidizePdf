//! CFF Type 2 charstring desubroutinization.
//!
//! Inlines every `callsubr` (opcode 10) and `callgsubr` (opcode 29) call so
//! the resulting charstring is self-contained and can be embedded without
//! requiring the source font's Local/Global Subr INDEX.
//!
//! Byte-level dispatch follows the CFF Type 2 encoding (distinct from CFF
//! DICT): byte 29 is `callgsubr`, not an integer prefix.

use super::index::CffIndex;
use crate::parser::{ParseError, ParseResult};

/// Recursion depth guard for subroutine inlining. Matches the bound used by
/// `typst/subsetter` — the Type 2 spec only requires 10, but 64 is the
/// common safety margin seen in real-world CFF.
const MAX_SUBR_DEPTH: u8 = 64;

/// Calculate subroutine bias per CFF Type 2 spec.
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

/// Desubroutinize a charstring.
///
/// Inlines all `callsubr` and `callgsubr` calls, drops `return` operators,
/// and preserves every other byte verbatim. The output is valid CFF Type 2
/// that references neither the global nor the local Subr INDEX.
pub fn desubroutinize(
    charstring: &[u8],
    global_subrs: &CffIndex,
    global_subrs_data: &[u8],
    local_subrs: &CffIndex,
    local_subrs_data: &[u8],
) -> ParseResult<Vec<u8>> {
    let mut output = Vec::with_capacity(charstring.len());
    let mut state = DesubState::default();
    // The bool return from desubroutinize_inner tells us whether endchar was
    // seen; at the top level we don't care — the output is returned either way.
    desubroutinize_inner(
        charstring,
        global_subrs,
        global_subrs_data,
        local_subrs,
        local_subrs_data,
        0,
        &mut state,
        &mut output,
    )?;
    Ok(output)
}

#[derive(Default)]
struct DesubState {
    /// Byte positions in `output` where each pushed operand starts.
    /// Lets us pop the last operand (subr index) when we hit callsubr/callgsubr.
    operand_positions: Vec<usize>,
    /// Accumulated stem-hint count (each stem = 2 operands).
    hint_count: usize,
    /// Cached byte width for hintmask/cntrmask data (= ceil(hint_count / 8)).
    hint_mask_bytes: usize,
    /// Whether we've already computed `hint_mask_bytes` for this run.
    seen_hint_mask: bool,
}

/// Returns `Ok(true)` when an `endchar` was emitted during this call (possibly
/// via a nested subroutine). Callers use that signal to stop processing their
/// own remaining bytes, since `endchar` terminates the entire charstring per
/// Type 2 spec §4.3. `Ok(false)` means the body ended by `return` or ran off
/// the end without an endchar — the caller should resume normal processing.
#[allow(clippy::too_many_arguments)]
fn desubroutinize_inner(
    charstring: &[u8],
    global_subrs: &CffIndex,
    global_subrs_data: &[u8],
    local_subrs: &CffIndex,
    local_subrs_data: &[u8],
    depth: u8,
    state: &mut DesubState,
    output: &mut Vec<u8>,
) -> ParseResult<bool> {
    if depth > MAX_SUBR_DEPTH {
        return Err(ParseError::SyntaxError {
            position: 0,
            message: format!(
                "CFF subroutine recursion depth exceeded (max {}, got {})",
                MAX_SUBR_DEPTH, depth
            ),
        });
    }

    let mut offset = 0;
    while offset < charstring.len() {
        let b0 = charstring[offset];

        match b0 {
            // ---------------- Subroutine calls ----------------
            10 => {
                // callsubr: pop index operand, inline local subr body
                let (biased_index, operand_start) =
                    pop_last_operand(output, &mut state.operand_positions)?;
                output.truncate(operand_start);
                let actual_index = biased_index + cff_subr_bias(local_subrs.count());
                let subr_data = subr_item(local_subrs, local_subrs_data, actual_index, offset)?;
                let saw_endchar = desubroutinize_inner(
                    subr_data,
                    global_subrs,
                    global_subrs_data,
                    local_subrs,
                    local_subrs_data,
                    depth + 1,
                    state,
                    output,
                )?;
                if saw_endchar {
                    return Ok(true);
                }
                offset += 1;
            }
            29 => {
                // callgsubr: pop index operand, inline global subr body
                let (biased_index, operand_start) =
                    pop_last_operand(output, &mut state.operand_positions)?;
                output.truncate(operand_start);
                let actual_index = biased_index + cff_subr_bias(global_subrs.count());
                let subr_data = subr_item(global_subrs, global_subrs_data, actual_index, offset)?;
                let saw_endchar = desubroutinize_inner(
                    subr_data,
                    global_subrs,
                    global_subrs_data,
                    local_subrs,
                    local_subrs_data,
                    depth + 1,
                    state,
                    output,
                )?;
                if saw_endchar {
                    return Ok(true);
                }
                offset += 1;
            }
            11 => {
                // return: end of this (sub)routine — drop the operator
                return Ok(false);
            }
            14 => {
                // endchar: emit and stop the entire charstring (spec §4.3)
                output.push(14);
                state.operand_positions.clear();
                return Ok(true);
            }

            // ---------------- Stem operators ----------------
            1 | 3 | 18 | 23 => {
                // hstem(1), vstem(3), hstemhm(18), vstemhm(23):
                // each stem = 2 operands on the stack.
                state.hint_count += state.operand_positions.len() / 2;
                output.push(b0);
                offset += 1;
                state.operand_positions.clear();
            }

            // ---------------- Hint masks ----------------
            19 | 20 => {
                // hintmask(19) / cntrmask(20): on first mask, any operands
                // still on the stack are an implicit vstemhm (per spec).
                if !state.seen_hint_mask {
                    state.hint_count += state.operand_positions.len() / 2;
                    state.hint_mask_bytes = state.hint_count.div_ceil(8);
                    state.seen_hint_mask = true;
                }
                output.push(b0);
                offset += 1;
                let mask_end = offset + state.hint_mask_bytes;
                if mask_end > charstring.len() {
                    return Err(ParseError::SyntaxError {
                        position: offset,
                        message: "Truncated hintmask/cntrmask data".to_string(),
                    });
                }
                output.extend_from_slice(&charstring[offset..mask_end]);
                offset = mask_end;
                state.operand_positions.clear();
            }

            // ---------------- 2-byte escape operator ----------------
            12 => {
                if offset + 1 >= charstring.len() {
                    return Err(ParseError::SyntaxError {
                        position: offset,
                        message: "Truncated 2-byte operator".to_string(),
                    });
                }
                output.push(12);
                output.push(charstring[offset + 1]);
                offset += 2;
                state.operand_positions.clear();
            }

            // ---------------- Remaining 1-byte operators ----------------
            // Everything in 0..=31 not handled above: stack-consuming operators
            // like rmoveto(21), hmoveto(22), rlineto(5), curve operators, etc.
            0 | 2 | 4..=9 | 13 | 15..=17 | 21 | 22 | 24..=27 | 30 | 31 => {
                output.push(b0);
                offset += 1;
                state.operand_positions.clear();
            }

            // ---------------- Operands ----------------
            28 => {
                // 3-byte integer (signed i16 big-endian)
                if offset + 2 >= charstring.len() {
                    return Err(ParseError::SyntaxError {
                        position: offset,
                        message: "Truncated 3-byte integer".to_string(),
                    });
                }
                state.operand_positions.push(output.len());
                output.extend_from_slice(&charstring[offset..offset + 3]);
                offset += 3;
            }
            32..=246 => {
                // 1-byte integer
                state.operand_positions.push(output.len());
                output.push(b0);
                offset += 1;
            }
            247..=254 => {
                // 2-byte integer (positive 247..=250, negative 251..=254)
                if offset + 1 >= charstring.len() {
                    return Err(ParseError::SyntaxError {
                        position: offset,
                        message: "Truncated 2-byte integer".to_string(),
                    });
                }
                state.operand_positions.push(output.len());
                output.extend_from_slice(&charstring[offset..offset + 2]);
                offset += 2;
            }
            255 => {
                // 5-byte 16.16 fixed-point (Type 2 charstring only)
                if offset + 4 >= charstring.len() {
                    return Err(ParseError::SyntaxError {
                        position: offset,
                        message: "Truncated 5-byte fixed-point".to_string(),
                    });
                }
                state.operand_positions.push(output.len());
                output.extend_from_slice(&charstring[offset..offset + 5]);
                offset += 5;
            }
        }
    }
    Ok(false)
}

fn subr_item<'a>(
    index: &CffIndex,
    data: &'a [u8],
    subr_index: i32,
    position: usize,
) -> ParseResult<&'a [u8]> {
    if subr_index < 0 {
        return Err(ParseError::SyntaxError {
            position,
            message: format!("Negative subr index {} after bias", subr_index),
        });
    }
    index
        .get_item(subr_index as usize, data)
        .ok_or_else(|| ParseError::SyntaxError {
            position,
            message: format!("Subr index {} out of range", subr_index),
        })
}

fn pop_last_operand(
    output: &[u8],
    operand_positions: &mut Vec<usize>,
) -> ParseResult<(i32, usize)> {
    let start = operand_positions
        .pop()
        .ok_or_else(|| ParseError::SyntaxError {
            position: 0,
            message: "callsubr/callgsubr with empty operand stack".to_string(),
        })?;
    let value = decode_type2_number(&output[start..])?;
    Ok((value, start))
}

fn decode_type2_number(bytes: &[u8]) -> ParseResult<i32> {
    if bytes.is_empty() {
        return Err(ParseError::SyntaxError {
            position: 0,
            message: "Empty number encoding".to_string(),
        });
    }
    let b0 = bytes[0];
    match b0 {
        32..=246 => Ok(b0 as i32 - 139),
        247..=250 => {
            if bytes.len() < 2 {
                return Err(ParseError::SyntaxError {
                    position: 0,
                    message: "Truncated 2-byte positive number".to_string(),
                });
            }
            Ok((b0 as i32 - 247) * 256 + bytes[1] as i32 + 108)
        }
        251..=254 => {
            if bytes.len() < 2 {
                return Err(ParseError::SyntaxError {
                    position: 0,
                    message: "Truncated 2-byte negative number".to_string(),
                });
            }
            Ok(-(b0 as i32 - 251) * 256 - bytes[1] as i32 - 108)
        }
        28 => {
            if bytes.len() < 3 {
                return Err(ParseError::SyntaxError {
                    position: 0,
                    message: "Truncated 3-byte integer".to_string(),
                });
            }
            Ok(i16::from_be_bytes([bytes[1], bytes[2]]) as i32)
        }
        255 => {
            if bytes.len() < 5 {
                return Err(ParseError::SyntaxError {
                    position: 0,
                    message: "Truncated 5-byte fixed-point number".to_string(),
                });
            }
            // 16.16 fixed-point: integer part is the upper 16 bits.
            // Subr indices are always integers in practice.
            let fixed = i32::from_be_bytes([bytes[1], bytes[2], bytes[3], bytes[4]]);
            Ok(fixed >> 16)
        }
        _ => Err(ParseError::SyntaxError {
            position: 0,
            message: format!("Byte {} is not a valid Type 2 number encoding", b0),
        }),
    }
}
