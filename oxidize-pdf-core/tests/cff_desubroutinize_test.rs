//! Tests for CFF charstring desubroutinization.
//!
//! Operand encoding cheat-sheet (CFF Type 2 charstring format):
//!
//! - Bytes 32..=246: 1-byte integer, value = b0 - 139 (range -107..=107).
//!   140=1, 141=2, 142=3, 143=4, 144=5, 149=10, 159=20,
//!   189=50, 239=100, 32=-107, 33=-106.
//! - Bytes 247..=250: 2-byte positive, value = (b0-247)*256 + b1 + 108.
//!   [247, 92] = 200.
//! - Byte 28: 3-byte integer (i16 big-endian).
//! - Byte 255: 5-byte 16.16 fixed-point.
//!
//! Byte 29 is `callgsubr`, NOT an integer prefix (that's CFF DICT encoding).

use oxidize_pdf::text::fonts::cff::charstring::{cff_subr_bias, desubroutinize};
use oxidize_pdf::text::fonts::cff::index::{build_cff_index, parse_cff_index};

// =========================================================================
// Bias calculation
// =========================================================================

#[test]
fn test_bias_small() {
    assert_eq!(cff_subr_bias(0), 107);
    assert_eq!(cff_subr_bias(1), 107);
    assert_eq!(cff_subr_bias(1239), 107);
}

#[test]
fn test_bias_medium() {
    assert_eq!(cff_subr_bias(1240), 1131);
    assert_eq!(cff_subr_bias(33899), 1131);
}

#[test]
fn test_bias_large() {
    assert_eq!(cff_subr_bias(33900), 32768);
    assert_eq!(cff_subr_bias(65535), 32768);
}

// =========================================================================
// Desubroutinization: no subroutine calls
// =========================================================================

#[test]
fn test_desubroutinize_no_calls() {
    // Charstring: rmoveto(100, 200) + endchar
    let charstring: Vec<u8> = vec![
        239, // operand 100 (1-byte)
        247, 92, // operand 200 (2-byte)
        21, // rmoveto
        14, // endchar
    ];

    let empty_index_data = build_cff_index(&[]);
    let global_subrs = parse_cff_index(&empty_index_data, 0).unwrap();
    let local_subrs = parse_cff_index(&empty_index_data, 0).unwrap();

    let result = desubroutinize(
        &charstring,
        &global_subrs,
        &empty_index_data,
        &local_subrs,
        &empty_index_data,
    )
    .unwrap();

    // No calls to inline — output should be identical to input
    assert_eq!(result, charstring);
}

// =========================================================================
// Desubroutinization: local subr call
// =========================================================================

#[test]
fn test_desubroutinize_local_subr_call() {
    // Local subroutine 0: rmoveto(50, 50) + return
    let subr_body: Vec<u8> = vec![
        189, // operand 50
        189, // operand 50
        21,  // rmoveto
        11,  // return
    ];

    // Charstring: push biased index for subr 0 (-107), callsubr, endchar
    let charstring: Vec<u8> = vec![
        32, // operand -107 (biased index for subr 0, bias=107)
        10, // callsubr
        14, // endchar
    ];

    let local_subrs_data = build_cff_index(&[&subr_body]);
    let local_subrs = parse_cff_index(&local_subrs_data, 0).unwrap();
    let empty_index_data = build_cff_index(&[]);
    let global_subrs = parse_cff_index(&empty_index_data, 0).unwrap();

    let result = desubroutinize(
        &charstring,
        &global_subrs,
        &empty_index_data,
        &local_subrs,
        &local_subrs_data,
    )
    .unwrap();

    // Expected: subroutine body inlined (without return), then endchar
    let expected: Vec<u8> = vec![189, 189, 21, 14];
    assert_eq!(result, expected);
}

// =========================================================================
// Desubroutinization: global subr call
// =========================================================================

#[test]
fn test_desubroutinize_global_subr_call() {
    // Global subroutine 0: rlineto(10, 20) + return
    let gsubr_body: Vec<u8> = vec![
        149, // operand 10
        159, // operand 20
        5,   // rlineto
        11,  // return
    ];

    let charstring: Vec<u8> = vec![
        32, // operand -107 (biased index for gsubr 0)
        29, // callgsubr
        14, // endchar
    ];

    let global_subrs_data = build_cff_index(&[&gsubr_body]);
    let global_subrs = parse_cff_index(&global_subrs_data, 0).unwrap();
    let empty_index_data = build_cff_index(&[]);
    let local_subrs = parse_cff_index(&empty_index_data, 0).unwrap();

    let result = desubroutinize(
        &charstring,
        &global_subrs,
        &global_subrs_data,
        &local_subrs,
        &empty_index_data,
    )
    .unwrap();

    let expected: Vec<u8> = vec![149, 159, 5, 14];
    assert_eq!(result, expected);
}

// =========================================================================
// Desubroutinization: nested calls (subr calls another subr)
// =========================================================================

#[test]
fn test_desubroutinize_nested_local_subrs() {
    // Subr 1: rlineto(5, 5) + return
    let subr1_body: Vec<u8> = vec![144, 144, 5, 11];

    // Subr 0: calls subr 1 + return
    // 2 subrs → bias = 107. Biased index for subr 1 = 1 - 107 = -106 → byte 33.
    let subr0_body: Vec<u8> = vec![
        33, // operand -106 (biased index for subr 1)
        10, // callsubr
        11, // return
    ];

    // Charstring: call subr 0 + endchar. Biased index for subr 0 = -107 → byte 32.
    let charstring: Vec<u8> = vec![32, 10, 14];

    let local_subrs_data = build_cff_index(&[&subr0_body, &subr1_body]);
    let local_subrs = parse_cff_index(&local_subrs_data, 0).unwrap();
    let empty_index_data = build_cff_index(&[]);
    let global_subrs = parse_cff_index(&empty_index_data, 0).unwrap();

    let result = desubroutinize(
        &charstring,
        &global_subrs,
        &empty_index_data,
        &local_subrs,
        &local_subrs_data,
    )
    .unwrap();

    // Expected: subr0 → subr1 inlined, fully flat
    let expected: Vec<u8> = vec![144, 144, 5, 14];
    assert_eq!(result, expected);
}

// =========================================================================
// Desubroutinization: max depth boundary
// =========================================================================
//
// The depth guard uses MAX_SUBR_DEPTH = 64. Counting the top-level
// charstring as depth 0, subr N at depth N+1: a chain of 64 subrs
// (indices 0..=63) reaches a frame at depth 64 — which must error.
// A chain of 63 subrs reaches depth 63 — which must succeed.

/// Build a chain of N subrs where subr i calls subr i+1 and the last
/// one just returns. Returns (index_data, index) ready for desubroutinize.
fn build_subr_chain(n: usize) -> (Vec<u8>, Vec<Vec<u8>>) {
    let mut bodies: Vec<Vec<u8>> = Vec::new();
    // N subrs with bias = 107 (N < 1240). Biased index for subr i+1 = (i+1) - 107.
    for i in 0i32..(n as i32 - 1) {
        let biased = (i + 1) - 107;
        let b0 = (biased + 139) as u8;
        bodies.push(vec![b0, 10, 11]); // operand + callsubr + return
    }
    // Last subr: just return
    bodies.push(vec![11]);
    let index_data = {
        let refs: Vec<&[u8]> = bodies.iter().map(|v| v.as_slice()).collect();
        build_cff_index(&refs)
    };
    (index_data, bodies)
}

#[test]
fn test_desubroutinize_max_depth_exceeded() {
    // Chain of 64 subrs → deepest frame at depth 64 → must error under
    // the MAX_SUBR_DEPTH = 64 invariant (depth >= MAX triggers the guard).
    let (local_subrs_data, _bodies) = build_subr_chain(64);
    let local_subrs = parse_cff_index(&local_subrs_data, 0).unwrap();
    let empty_index_data = build_cff_index(&[]);
    let global_subrs = parse_cff_index(&empty_index_data, 0).unwrap();

    // Charstring: call subr 0 + endchar
    let charstring: Vec<u8> = vec![32, 10, 14];

    let result = desubroutinize(
        &charstring,
        &global_subrs,
        &empty_index_data,
        &local_subrs,
        &local_subrs_data,
    );
    assert!(
        result.is_err(),
        "A chain of 64 subrs reaches depth 64, must exceed MAX_SUBR_DEPTH"
    );
}

#[test]
fn test_desubroutinize_max_depth_allowed() {
    // Chain of 63 subrs → deepest frame at depth 63 → must succeed
    // under the MAX_SUBR_DEPTH = 64 invariant.
    let (local_subrs_data, _bodies) = build_subr_chain(63);
    let local_subrs = parse_cff_index(&local_subrs_data, 0).unwrap();
    let empty_index_data = build_cff_index(&[]);
    let global_subrs = parse_cff_index(&empty_index_data, 0).unwrap();

    let charstring: Vec<u8> = vec![32, 10, 14];

    let result = desubroutinize(
        &charstring,
        &global_subrs,
        &empty_index_data,
        &local_subrs,
        &local_subrs_data,
    );
    assert!(
        result.is_ok(),
        "A chain of 63 subrs reaches depth 63, must stay below MAX_SUBR_DEPTH"
    );
    // All calls fold into endchar; output is just the outer endchar.
    assert_eq!(result.unwrap(), vec![14]);
}

// =========================================================================
// Desubroutinization: empty / trivial charstrings
// =========================================================================

#[test]
fn test_desubroutinize_empty_charstring() {
    let empty_index_data = build_cff_index(&[]);
    let global_subrs = parse_cff_index(&empty_index_data, 0).unwrap();
    let local_subrs = parse_cff_index(&empty_index_data, 0).unwrap();

    let result = desubroutinize(
        &[],
        &global_subrs,
        &empty_index_data,
        &local_subrs,
        &empty_index_data,
    )
    .unwrap();
    assert!(result.is_empty());
}

#[test]
fn test_desubroutinize_endchar_only() {
    let empty_index_data = build_cff_index(&[]);
    let global_subrs = parse_cff_index(&empty_index_data, 0).unwrap();
    let local_subrs = parse_cff_index(&empty_index_data, 0).unwrap();

    let charstring = vec![14u8]; // endchar
    let result = desubroutinize(
        &charstring,
        &global_subrs,
        &empty_index_data,
        &local_subrs,
        &empty_index_data,
    )
    .unwrap();
    assert_eq!(result, vec![14u8]);
}

// =========================================================================
// Desubroutinization: mixed local + global calls
// =========================================================================

#[test]
fn test_desubroutinize_mixed_local_and_global() {
    // Local subr 0: rmoveto(1, 2) + return
    let local_body: Vec<u8> = vec![140, 141, 21, 11];

    // Global subr 0: rlineto(3, 4) + return
    let global_body: Vec<u8> = vec![142, 143, 5, 11];

    // Charstring: call local 0, call global 0, endchar
    let charstring: Vec<u8> = vec![
        32, 10, // call local subr 0
        32, 29, // call global subr 0
        14, // endchar
    ];

    let local_data = build_cff_index(&[&local_body]);
    let local_subrs = parse_cff_index(&local_data, 0).unwrap();
    let global_data = build_cff_index(&[&global_body]);
    let global_subrs = parse_cff_index(&global_data, 0).unwrap();

    let result = desubroutinize(
        &charstring,
        &global_subrs,
        &global_data,
        &local_subrs,
        &local_data,
    )
    .unwrap();

    let expected: Vec<u8> = vec![
        140, 141, 21, // rmoveto(1, 2) from local subr
        142, 143, 5,  // rlineto(3, 4) from global subr
        14, // endchar
    ];
    assert_eq!(result, expected);
}

// =========================================================================
// Desubroutinization: operands before call are preserved except the index
// =========================================================================

#[test]
fn test_desubroutinize_preserves_preceding_operands() {
    // Local subr 0: just rmoveto + return (uses the caller's 2 pushed operands)
    let local_body: Vec<u8> = vec![21, 11];

    // Charstring: push 100, 200, then subr index, callsubr, endchar.
    // The desubroutinizer must remove only the subr-index operand,
    // keeping 100 and 200 in the output for the inlined rmoveto.
    let charstring: Vec<u8> = vec![
        239, // operand 100
        247, 92, // operand 200
        32, // operand -107 (subr index)
        10, // callsubr
        14, // endchar
    ];

    let local_data = build_cff_index(&[&local_body]);
    let local_subrs = parse_cff_index(&local_data, 0).unwrap();
    let empty = build_cff_index(&[]);
    let global_subrs = parse_cff_index(&empty, 0).unwrap();

    let result = desubroutinize(
        &charstring,
        &global_subrs,
        &empty,
        &local_subrs,
        &local_data,
    )
    .unwrap();

    let expected: Vec<u8> = vec![
        239, // operand 100 (preserved)
        247, 92, // operand 200 (preserved)
        21, // rmoveto (from subr body)
        14, // endchar
    ];
    assert_eq!(result, expected);
}

// =========================================================================
// Desubroutinization: endchar inside a subroutine terminates the caller
// (Type 2 spec §4.3: endchar must be the last command in any charstring)
// =========================================================================

#[test]
fn test_desubroutinize_endchar_in_subr_terminates_caller() {
    // Local subr 0: rmoveto(1, 2) + endchar (no `return`). Per Type 2 spec,
    // endchar terminates the whole charstring, including callers that
    // inlined this subr — any bytes after the callsubr operator must be
    // discarded.
    let local_body: Vec<u8> = vec![
        140, // operand 1
        141, // operand 2
        21,  // rmoveto
        14,  // endchar
    ];

    // Charstring: call subr 0, then some spurious trailing bytes that
    // must NOT appear in the output because the inlined endchar stops
    // processing.
    let charstring: Vec<u8> = vec![
        32,  // operand -107 (biased subr index 0)
        10,  // callsubr
        239, // spurious operand 100 — must be dropped
        21,  // spurious rmoveto — must be dropped
        14,  // outer endchar — also dropped (endchar already emitted)
    ];

    let local_data = build_cff_index(&[&local_body]);
    let local_subrs = parse_cff_index(&local_data, 0).unwrap();
    let empty = build_cff_index(&[]);
    let global_subrs = parse_cff_index(&empty, 0).unwrap();

    let result = desubroutinize(
        &charstring,
        &global_subrs,
        &empty,
        &local_subrs,
        &local_data,
    )
    .unwrap();

    // Expected: only the subr body (rmoveto + endchar), nothing after.
    let expected: Vec<u8> = vec![140, 141, 21, 14];
    assert_eq!(
        result, expected,
        "endchar inside subr must terminate the caller's processing"
    );
}

// =========================================================================
// Adversarial cases — Cycle 9 of the quality-review fixes
// =========================================================================

/// `callgsubr` against an empty Global Subr INDEX must return an error —
/// there is no subr 0 (or any subr) to inline.
#[test]
fn test_desubroutinize_callgsubr_empty_global_index() {
    // Push biased index 0 (byte 139), then callgsubr. With an empty global
    // INDEX (count=0) the bias is 107, so actual_index = 0 + 107 = 107 —
    // out of range of an empty INDEX, which `subr_item` rejects.
    let charstring: Vec<u8> = vec![
        139, // operand 0
        29,  // callgsubr
        14,  // endchar (never reached)
    ];

    let empty = build_cff_index(&[]);
    let global_subrs = parse_cff_index(&empty, 0).unwrap();
    let local_subrs = parse_cff_index(&empty, 0).unwrap();

    let result = desubroutinize(&charstring, &global_subrs, &empty, &local_subrs, &empty);
    assert!(
        result.is_err(),
        "callgsubr into an empty global INDEX must be an error"
    );
}

/// `hintmask` (op 19) with no prior stem operators and no operands on the
/// stack is a degenerate but valid case: `hint_count = 0` so
/// `hint_mask_bytes = 0` and no mask bytes are consumed.
#[test]
fn test_desubroutinize_hintmask_before_any_stem() {
    let charstring: Vec<u8> = vec![
        19, // hintmask (zero hints → zero mask bytes)
        14, // endchar
    ];

    let empty = build_cff_index(&[]);
    let global_subrs = parse_cff_index(&empty, 0).unwrap();
    let local_subrs = parse_cff_index(&empty, 0).unwrap();

    let result = desubroutinize(&charstring, &global_subrs, &empty, &local_subrs, &empty).unwrap();

    // Output equals input: the `19` byte is emitted, no mask bytes follow
    // it, then `14` terminates.
    assert_eq!(result, vec![19, 14]);
}

/// A 5-byte fixed-point operand (byte 255) used as a subroutine index goes
/// through `decode_type2_number`'s 16.16 path. Biased index -107 → byte
/// sequence [255, 0xFF, 0x95, 0x00, 0x00] (i32 = -107 << 16). After bias
/// addition the actual index is 0, which resolves to subr 0.
#[test]
fn test_desubroutinize_5byte_fixed_point_as_subr_index() {
    // Subr 0: just endchar. After inlining the outer endchar is never
    // reached because the inner endchar propagates up (Cycle 2 fix).
    let subr_body: Vec<u8> = vec![14];

    let charstring: Vec<u8> = vec![
        255, 0xFF, 0x95, 0x00, 0x00, // 16.16 fixed-point value -107 (-107 << 16)
        10,   // callsubr
        14,   // endchar (unreachable due to endchar propagation from subr)
    ];

    let local_subrs_data = build_cff_index(&[&subr_body]);
    let local_subrs = parse_cff_index(&local_subrs_data, 0).unwrap();
    let empty = build_cff_index(&[]);
    let global_subrs = parse_cff_index(&empty, 0).unwrap();

    let result = desubroutinize(
        &charstring,
        &global_subrs,
        &empty,
        &local_subrs,
        &local_subrs_data,
    )
    .unwrap();

    // Only the subr's endchar is emitted — the outer endchar is unreachable.
    assert_eq!(result, vec![14]);
}

/// `cntrmask` (op 20) following `hintmask` (op 19) reuses the cached
/// `hint_mask_bytes` without recounting stems (`seen_hint_mask` is true
/// after the first mask). Both masks must emit their byte + mask bytes.
#[test]
fn test_desubroutinize_cntrmask_after_hintmask() {
    // hstem consumes 2 operands → hint_count = 1 → hint_mask_bytes = 1.
    let charstring: Vec<u8> = vec![
        140,  // operand 1 (stem edge)
        141,  // operand 2 (stem width)
        1,    // hstem
        19,   // hintmask
        0x80, // 1-byte mask
        20,   // cntrmask
        0x40, // 1-byte mask
        14,   // endchar
    ];

    let empty = build_cff_index(&[]);
    let global_subrs = parse_cff_index(&empty, 0).unwrap();
    let local_subrs = parse_cff_index(&empty, 0).unwrap();

    let result = desubroutinize(&charstring, &global_subrs, &empty, &local_subrs, &empty).unwrap();

    // Output is byte-identical: operands + hstem + hintmask + mask byte +
    // cntrmask + mask byte + endchar. The second mask reuses the cached
    // byte width rather than recounting stems.
    assert_eq!(result, vec![140, 141, 1, 19, 0x80, 20, 0x40, 14]);
}
