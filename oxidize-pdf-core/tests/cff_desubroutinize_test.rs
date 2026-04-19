//! Tests for CFF charstring desubroutinization.

use oxidize_pdf::text::fonts::cff::charstring::{cff_subr_bias, desubroutinize};
use oxidize_pdf::text::fonts::cff::index::{build_cff_index, parse_cff_index};

// =========================================================================
// Bias calculation (these should pass immediately — already implemented)
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
    // Uses 5-byte encoding (prefix 29): [29, i32 BE bytes]
    let charstring: Vec<u8> = vec![
        29, 0, 0, 0, 100, // operand 100
        29, 0, 0, 0, 200, // operand 200
        21,  // rmoveto
        14,  // endchar
    ];

    let empty_index_data = build_cff_index(&[]);
    let global_subrs = parse_cff_index(&empty_index_data, 0).unwrap();
    let local_subrs = parse_cff_index(&empty_index_data, 0).unwrap();

    let result = desubroutinize(&charstring, &global_subrs, &local_subrs).unwrap();

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
        29, 0, 0, 0, 50, // operand 50
        29, 0, 0, 0, 50, // operand 50
        21, // rmoveto
        11, // return
    ];

    // Charstring: push biased index for subr 0 (bias=107 → -107), callsubr, endchar.
    // CFF Type 2 1-byte integer encoding: value = b0 - 139, so -107 → b0 = 32.
    let charstring: Vec<u8> = vec![
        32, // operand -107 (1-byte: 32 - 139 = -107)
        10, // callsubr
        14, // endchar
    ];

    let local_subrs_data = build_cff_index(&[&subr_body]);
    let local_subrs = parse_cff_index(&local_subrs_data, 0).unwrap();
    let empty_index_data = build_cff_index(&[]);
    let global_subrs = parse_cff_index(&empty_index_data, 0).unwrap();

    let result = desubroutinize(&charstring, &global_subrs, &local_subrs).unwrap();

    // Expected: subroutine body inlined (without return), then endchar
    let expected: Vec<u8> = vec![
        29, 0, 0, 0, 50, // operand 50
        29, 0, 0, 0, 50, // operand 50
        21, // rmoveto
        14, // endchar
    ];
    assert_eq!(result, expected);
}

// =========================================================================
// Desubroutinization: global subr call
// =========================================================================

#[test]
fn test_desubroutinize_global_subr_call() {
    // Global subroutine 0: rlineto(10, 20) + return
    let gsubr_body: Vec<u8> = vec![
        29, 0, 0, 0, 10, // operand 10
        29, 0, 0, 0, 20, // operand 20
        5,  // rlineto
        11, // return
    ];

    // NOTE: CFF Type 2 charstring encoding differs from CFF DICT encoding:
    //   - In DICT: byte 29 = 5-byte integer prefix
    //   - In Type 2 charstrings: byte 29 = callgsubr operator
    // Bytes 32-246 encode 1-byte integers (value = b0 - 139).
    // For subr index -107 (biased) we use byte 32.
    let charstring: Vec<u8> = vec![
        32, // operand -107 (biased index for gsubr 0)
        29, // callgsubr
        14, // endchar
    ];

    let global_subrs_data = build_cff_index(&[&gsubr_body]);
    let global_subrs = parse_cff_index(&global_subrs_data, 0).unwrap();
    let empty_index_data = build_cff_index(&[]);
    let local_subrs = parse_cff_index(&empty_index_data, 0).unwrap();

    let result = desubroutinize(&charstring, &global_subrs, &local_subrs).unwrap();

    let expected: Vec<u8> = vec![
        29, 0, 0, 0, 10, // operand 10
        29, 0, 0, 0, 20, // operand 20
        5,  // rlineto
        14, // endchar
    ];
    assert_eq!(result, expected);
}

// =========================================================================
// Desubroutinization: nested calls (subr calls another subr)
// =========================================================================

#[test]
fn test_desubroutinize_nested_local_subrs() {
    // Subr 1: rlineto(5, 5) + return
    let subr1_body: Vec<u8> = vec![
        29, 0, 0, 0, 5, // operand 5
        29, 0, 0, 0, 5,  // operand 5
        5,  // rlineto
        11, // return
    ];

    // Subr 0: calls subr 1 + return
    // 2 subrs → bias = 107. Biased index for subr 1 = 1 - 107 = -106 → 1-byte 33.
    let subr0_body: Vec<u8> = vec![
        33, // operand -106 (biased index for subr 1)
        10, // callsubr
        11, // return
    ];

    // Charstring: call subr 0 + endchar
    let charstring: Vec<u8> = vec![
        32, // operand -107 (biased index for subr 0)
        10, // callsubr
        14, // endchar
    ];

    let local_subrs_data = build_cff_index(&[&subr0_body, &subr1_body]);
    let local_subrs = parse_cff_index(&local_subrs_data, 0).unwrap();
    let empty_index_data = build_cff_index(&[]);
    let global_subrs = parse_cff_index(&empty_index_data, 0).unwrap();

    let result = desubroutinize(&charstring, &global_subrs, &local_subrs).unwrap();

    // Expected: subr0 → subr1 inlined, fully flat
    let expected: Vec<u8> = vec![
        29, 0, 0, 0, 5, // operand 5
        29, 0, 0, 0, 5,  // operand 5
        5,  // rlineto
        14, // endchar
    ];
    assert_eq!(result, expected);
}

// =========================================================================
// Desubroutinization: max depth exceeded
// =========================================================================

#[test]
fn test_desubroutinize_max_depth_exceeded() {
    // Create a chain of 65 subrs, each calling the next (exceeds CFF 10-level spec limit,
    // also exceeds the commonly used 64-level safety bound).
    let mut subr_bodies: Vec<Vec<u8>> = Vec::new();

    // 65 total subrs → bias = 107. Biased index for subr i+1 = (i + 1) - 107.
    for i in 0i32..64 {
        let next_biased = (i + 1) - 107;
        let mut body = Vec::new();
        let b0 = (next_biased + 139) as u8;
        body.push(b0);
        body.push(10); // callsubr
        body.push(11); // return
        subr_bodies.push(body);
    }

    // Subr 64: just return
    subr_bodies.push(vec![11]);

    let subr_refs: Vec<&[u8]> = subr_bodies.iter().map(|v| v.as_slice()).collect();
    let local_subrs_data = build_cff_index(&subr_refs);
    let local_subrs = parse_cff_index(&local_subrs_data, 0).unwrap();
    let empty_index_data = build_cff_index(&[]);
    let global_subrs = parse_cff_index(&empty_index_data, 0).unwrap();

    // Charstring: call subr 0 + endchar
    let charstring: Vec<u8> = vec![
        32, // operand -107 (biased index for subr 0)
        10, // callsubr
        14, // endchar
    ];

    let result = desubroutinize(&charstring, &global_subrs, &local_subrs);
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

    let result = desubroutinize(&[], &global_subrs, &local_subrs).unwrap();
    assert!(result.is_empty());
}

#[test]
fn test_desubroutinize_endchar_only() {
    let empty_index_data = build_cff_index(&[]);
    let global_subrs = parse_cff_index(&empty_index_data, 0).unwrap();
    let local_subrs = parse_cff_index(&empty_index_data, 0).unwrap();

    let charstring = vec![14u8]; // endchar
    let result = desubroutinize(&charstring, &global_subrs, &local_subrs).unwrap();
    assert_eq!(result, vec![14u8]);
}

// =========================================================================
// Desubroutinization: mixed local + global calls
// =========================================================================

#[test]
fn test_desubroutinize_mixed_local_and_global() {
    // Local subr 0: rmoveto(1, 2) + return
    let local_body: Vec<u8> = vec![
        29, 0, 0, 0, 1, // operand 1
        29, 0, 0, 0, 2,  // operand 2
        21, // rmoveto
        11, // return
    ];

    // Global subr 0: rlineto(3, 4) + return
    let global_body: Vec<u8> = vec![
        29, 0, 0, 0, 3, // operand 3
        29, 0, 0, 0, 4,  // operand 4
        5,  // rlineto
        11, // return
    ];

    // Charstring: call local 0, call global 0, endchar
    let charstring: Vec<u8> = vec![
        32, // operand -107 (biased index for local subr 0)
        10, // callsubr
        32, // operand -107 (biased index for global subr 0)
        29, // callgsubr
        14, // endchar
    ];

    let local_data = build_cff_index(&[&local_body]);
    let local_subrs = parse_cff_index(&local_data, 0).unwrap();
    let global_data = build_cff_index(&[&global_body]);
    let global_subrs = parse_cff_index(&global_data, 0).unwrap();

    let result = desubroutinize(&charstring, &global_subrs, &local_subrs).unwrap();

    let expected: Vec<u8> = vec![
        29, 0, 0, 0, 1, // operand 1 (from local subr)
        29, 0, 0, 0, 2,  // operand 2
        21, // rmoveto
        29, 0, 0, 0, 3, // operand 3 (from global subr)
        29, 0, 0, 0, 4,  // operand 4
        5,  // rlineto
        14, // endchar
    ];
    assert_eq!(result, expected);
}

// =========================================================================
// Desubroutinization: operands before call are preserved except the index
// =========================================================================

#[test]
fn test_desubroutinize_preserves_preceding_operands() {
    // Local subr 0: just rmoveto + return (consumes 2 args from caller's stack)
    let local_body: Vec<u8> = vec![
        21, // rmoveto (uses the 2 operands pushed before callsubr)
        11, // return
    ];

    // Charstring: push 100, 200, then call subr 0 (index is separate arg), then endchar.
    // The desubroutinizer must remove only the subr index operand,
    // keeping 100 and 200 on the output.
    let charstring: Vec<u8> = vec![
        29, 0, 0, 0, 100, // operand 100
        29, 0, 0, 0, 200, // operand 200
        32,  // operand -107 (subr index)
        10,  // callsubr
        14,  // endchar
    ];

    let local_data = build_cff_index(&[&local_body]);
    let local_subrs = parse_cff_index(&local_data, 0).unwrap();
    let empty = build_cff_index(&[]);
    let global_subrs = parse_cff_index(&empty, 0).unwrap();

    let result = desubroutinize(&charstring, &global_subrs, &local_subrs).unwrap();

    // Expected: 100, 200 preserved, then rmoveto (from subr body), then endchar
    let expected: Vec<u8> = vec![
        29, 0, 0, 0, 100, // operand 100 (preserved)
        29, 0, 0, 0, 200, // operand 200 (preserved)
        21,  // rmoveto (from subr body)
        14,  // endchar
    ];
    assert_eq!(result, expected);
}
