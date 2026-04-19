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
// Desubroutinization: max depth exceeded
// =========================================================================

#[test]
fn test_desubroutinize_max_depth_exceeded() {
    // Chain of 65 subrs, each calling the next. Exceeds the 64-level safety bound.
    let mut subr_bodies: Vec<Vec<u8>> = Vec::new();

    // 65 subrs → bias = 107. Biased index for subr i+1 = (i + 1) - 107.
    for i in 0i32..64 {
        let next_biased = (i + 1) - 107;
        let b0 = (next_biased + 139) as u8;
        subr_bodies.push(vec![
            b0, 10, // operand + callsubr
            11, // return
        ]);
    }

    // Subr 64: just return
    subr_bodies.push(vec![11]);

    let subr_refs: Vec<&[u8]> = subr_bodies.iter().map(|v| v.as_slice()).collect();
    let local_subrs_data = build_cff_index(&subr_refs);
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
        "Should fail when recursion depth exceeds limit"
    );
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
