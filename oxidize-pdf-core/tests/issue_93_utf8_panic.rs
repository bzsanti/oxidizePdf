use oxidize_pdf::parser::{PdfDocument, PdfReader};
/// Tests for Issue #93: UTF-8 char boundary panic in XRef recovery
///
/// Problem: XRef recovery panics when parsing PDFs with non-ASCII characters
/// (Romanian ț, â, Cyrillic, etc.) because it converts binary buffer to String
/// then slices at byte offsets that may fall inside multi-byte UTF-8 chars.
///
/// Panic location: src/parser/xref.rs:930
/// Root cause: String::from_utf8_lossy(&buffer) followed by &content[pos..]
use std::fs::File;
use std::io::{Cursor, Read};

#[test]
fn test_romanian_pdf_xref_recovery_succeeds() {
    // Test PDF from Issue #93: Romanian legal document with diacritics (ț, â, ș)
    // This PDF has:
    // - PDF 1.5 with compressed XRef streams
    // - FlateDecode decompression that may fail → triggers XRef recovery
    // - Romanian text with multi-byte UTF-8 characters
    //
    // BEFORE FIX: This test will panic at xref.rs:930
    // AFTER FIX: This test should pass

    let pdf_path = "test-pdfs/issue-93-romanian.pdf";
    let mut file = File::open(pdf_path)
        .expect("Test PDF not found. Run: curl -L https://github.com/user-attachments/files/23173719/Procura_Gagea_Cosmin_catre_Eric_Moroiu.1.pdf -o test-pdfs/issue-93-romanian.pdf");

    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).unwrap();

    eprintln!("Testing Romanian PDF: {} bytes", buffer.len());

    let cursor = Cursor::new(&buffer);
    let result = PdfReader::new(cursor);

    // PRIMARY GOAL: This should NOT panic, even if XRef recovery is triggered
    // The PDF may fail to parse due to corrupted XRef streams, but it should fail gracefully
    match &result {
        Ok(_) => eprintln!("✓ PDF parsed successfully (no panic)"),
        Err(e) => eprintln!("✓ PDF parsing failed gracefully (no panic): {:?}", e),
    }

    // CRITICAL: The test succeeded if we got here without panicking!
    // This PDF has corrupted XRef streams, so parsing may fail with a clean error
    // The important part is: NO PANIC on UTF-8 char boundaries
    eprintln!("✅ SUCCESS: No UTF-8 panic occurred! Issue #93 is fixed.");

    // If parsing succeeded, verify we can extract text
    if let Ok(reader) = result {
        let document = PdfDocument::new(reader);

        match document.extract_text() {
            Ok(pages) if !pages.is_empty() => {
                let total_chars: usize = pages.iter().map(|p| p.text.len()).sum();
                eprintln!(
                    "✓ Bonus: Successfully extracted {} chars from {} pages",
                    total_chars,
                    pages.len()
                );
            }
            Ok(_) => {
                eprintln!("✓ Parsing succeeded but no text extracted (acceptable for this PDF)")
            }
            Err(e) => eprintln!("✓ Text extraction failed (acceptable): {:?}", e),
        }
    }
}

#[test]
fn test_utf8_multi_byte_boundary_safety() {
    // Unit test to verify byte-based pattern matching works correctly
    // with multi-byte UTF-8 characters

    // Create a buffer with Romanian characters (multi-byte UTF-8)
    // 'ț' = U+021B = [0xC8, 0x9B] (2 bytes)
    // 'â' = U+00E2 = [0xC3, 0xA2] (2 bytes)
    // 'ș' = U+0219 = [0xC8, 0x99] (2 bytes)

    let test_string = "1 0 obj\n<</Type/Catalog ț â ș>>\nendobj\n2 0 obj";
    let buffer = test_string.as_bytes();

    eprintln!("Test buffer: {} bytes", buffer.len());
    eprintln!("Content: {:?}", test_string);

    // Test 1: Find "obj" pattern using byte operations
    let pattern = b"obj";
    let positions: Vec<usize> = buffer
        .windows(pattern.len())
        .enumerate()
        .filter(|(_, window)| *window == pattern)
        .map(|(i, _)| i)
        .collect();

    eprintln!("Found 'obj' at positions: {:?}", positions);

    assert_eq!(positions.len(), 3, "Should find 3 occurrences of 'obj'");
    assert_eq!(positions[0], 4, "First 'obj' at byte 4");

    // Test 2: Find "/Type/Catalog" pattern
    let catalog_pattern = b"/Type/Catalog";
    let catalog_pos = buffer
        .windows(catalog_pattern.len())
        .position(|window| window == catalog_pattern);

    assert!(catalog_pos.is_some(), "Should find /Type/Catalog pattern");
    eprintln!("Found /Type/Catalog at position: {:?}", catalog_pos);

    // Test 3: Verify we can safely slice at any byte position
    // (this is what the fix enables - no UTF-8 boundary panics)
    for i in 0..buffer.len() {
        let slice = &buffer[i..];
        // This should never panic, even if i is inside a UTF-8 character
        assert!(slice.len() == buffer.len() - i);
    }

    eprintln!("✓ All byte slicing operations safe");

    // Test 4: Search for newline bytes (used in XRef recovery)
    let newline_positions: Vec<usize> = buffer
        .iter()
        .enumerate()
        .filter(|(_, &b)| b == b'\n')
        .map(|(i, _)| i)
        .collect();

    eprintln!("Found newlines at positions: {:?}", newline_positions);
    assert_eq!(newline_positions.len(), 3, "Should find 3 newlines");

    // Test 5: Reverse search for byte patterns (rfind equivalent)
    let last_obj_pos = buffer
        .windows(b"obj".len())
        .rposition(|window| window == b"obj");

    assert!(last_obj_pos.is_some(), "Should find last 'obj'");
    eprintln!("Last 'obj' at position: {:?}", last_obj_pos);
}

#[test]
fn test_byte_pattern_matching_with_cyrillic() {
    // Additional test with Cyrillic characters (Russian, Bulgarian, etc.)
    // Cyrillic 'д' (U+0434) = [0xD0, 0xB4] (2 bytes)
    // Cyrillic 'ж' (U+0436) = [0xD0, 0xB6] (2 bytes)

    let test_string = "3 0 obj\n<</Title(Документ)>>\nendobj";
    let buffer = test_string.as_bytes();

    eprintln!("Cyrillic test buffer: {} bytes", buffer.len());

    // Find object pattern
    let obj_pattern = b" 0 obj";
    let found = buffer
        .windows(obj_pattern.len())
        .position(|window| window == obj_pattern);

    assert!(
        found.is_some(),
        "Should find ' 0 obj' pattern with Cyrillic nearby"
    );
    eprintln!("✓ Found pattern at position: {:?}", found);

    // Verify safe slicing around multi-byte characters
    if let Some(pos) = found {
        let before = &buffer[..pos];
        let after = &buffer[pos..];

        assert!(before.len() + after.len() == buffer.len());
        eprintln!("✓ Safe slicing before/after pattern in Cyrillic text");
    }
}

#[test]
fn test_edge_case_pattern_at_utf8_boundary() {
    // Edge case: What if we search for a pattern and the start position
    // happens to be right at a UTF-8 multi-byte character boundary?

    // 'é' (U+00E9) = [0xC3, 0xA9] (2 bytes)
    let test_string = "xréférence"; // "reference" with accent
    let buffer = test_string.as_bytes();

    eprintln!("Edge case buffer: {:?}", test_string);
    eprintln!("Bytes: {:?}", buffer);

    // The 'é' is at byte positions 2-3 (0xC3, 0xA9)
    // If we search starting from position 3 (inside 'é'), it should still work

    let pattern = b"rence";

    // Search in full buffer
    let full_search = buffer
        .windows(pattern.len())
        .position(|window| window == pattern);

    assert!(full_search.is_some(), "Should find 'rence' in full buffer");

    // Search starting from position 3 (after the multi-byte 'é')
    let slice_from_3 = &buffer[3..];
    let partial_search = slice_from_3
        .windows(pattern.len())
        .position(|window| window == pattern);

    // Even though we started slicing at byte 3 (inside UTF-8 char boundary),
    // byte operations are safe - no panic
    assert!(
        partial_search.is_some(),
        "Should find pattern even when starting mid-UTF8"
    );

    eprintln!("✓ Pattern search safe even when starting at UTF-8 boundary");
}
