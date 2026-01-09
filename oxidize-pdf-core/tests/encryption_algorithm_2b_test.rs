/// Unit tests for Algorithm 2.B (R6 Key Derivation)
///
/// These tests validate the compute_hash_r6_algorithm_2b() function
/// according to ISO 32000-2:2020 §7.6.4.3.4.
///
/// Algorithm 2.B uses:
/// - AES-128-CBC encryption within the iteration loop
/// - Dynamic SHA-256/384/512 selection based on sum(E\[0..16\]) mod 3
/// - Variable iteration count (min 64, terminates when condition met)
/// - Password length limit of 127 bytes (DoS protection)
use oxidize_pdf::encryption::compute_hash_r6_algorithm_2b;

/// Test with known values from qpdf-generated R6 PDF
/// PDF: encrypted_aes256_r6_user.pdf (password: "user6")
#[test]
fn test_algorithm_2b_qpdf_compatibility() {
    // Values from the PDF fixture:
    // U[0:32] (expected hash): 300d98eb3816f45e79007d78d285fd18784e354b1279af3b4704f6bba1ac0270
    // U[32:40] (validation_salt): fd0f02fdee2fffe1
    let password = b"user6";
    let validation_salt = [0xfd, 0x0f, 0x02, 0xfd, 0xee, 0x2f, 0xff, 0xe1];
    let expected_hash = [
        0x30, 0x0d, 0x98, 0xeb, 0x38, 0x16, 0xf4, 0x5e, 0x79, 0x00, 0x7d, 0x78, 0xd2, 0x85, 0xfd,
        0x18, 0x78, 0x4e, 0x35, 0x4b, 0x12, 0x79, 0xaf, 0x3b, 0x47, 0x04, 0xf6, 0xbb, 0xa1, 0xac,
        0x02, 0x70,
    ];

    let computed_hash = compute_hash_r6_algorithm_2b(password, &validation_salt, &[]).unwrap();

    // Print for debugging
    println!("Expected:  {:02x?}", expected_hash);
    println!("Computed:  {:02x?}", computed_hash);

    assert_eq!(
        computed_hash, expected_hash,
        "Algorithm 2.B output should match qpdf R6 implementation"
    );
}

#[test]
fn test_algorithm_2b_basic_execution() {
    // Basic test: ensure function runs without error
    let password = b"test_password";
    let salt = b"12345678"; // 8 bytes

    let hash = compute_hash_r6_algorithm_2b(password, salt, &[]).unwrap();

    assert_eq!(hash.len(), 32, "Algorithm 2.B must return 32 bytes");
    assert!(hash.iter().any(|&b| b != 0), "Hash should not be all zeros");
}

#[test]
fn test_algorithm_2b_deterministic() {
    // Same input must produce same output
    let password = b"deterministic_test";
    let salt = b"saltsalt";

    let hash1 = compute_hash_r6_algorithm_2b(password, salt, &[]).unwrap();
    let hash2 = compute_hash_r6_algorithm_2b(password, salt, &[]).unwrap();

    assert_eq!(hash1, hash2, "Algorithm 2.B must be deterministic");
}

#[test]
fn test_algorithm_2b_different_passwords() {
    let salt = b"12345678";

    let hash1 = compute_hash_r6_algorithm_2b(b"password1", salt, &[]).unwrap();
    let hash2 = compute_hash_r6_algorithm_2b(b"password2", salt, &[]).unwrap();

    assert_ne!(
        hash1, hash2,
        "Different passwords must produce different hashes"
    );
}

#[test]
fn test_algorithm_2b_different_salts() {
    let password = b"test_password";

    let hash1 = compute_hash_r6_algorithm_2b(password, b"salt1234", &[]).unwrap();
    let hash2 = compute_hash_r6_algorithm_2b(password, b"abcd5678", &[]).unwrap();

    assert_ne!(
        hash1, hash2,
        "Different salts must produce different hashes"
    );
}

#[test]
fn test_algorithm_2b_empty_password() {
    // Empty password should work (R6 allows empty user passwords)
    let hash = compute_hash_r6_algorithm_2b(b"", b"12345678", &[]).unwrap();

    assert_eq!(hash.len(), 32);
}

#[test]
fn test_algorithm_2b_unicode_password() {
    // UTF-8 encoded password
    let unicode_pwd = "café🔒".as_bytes();
    let salt = b"12345678";

    let hash = compute_hash_r6_algorithm_2b(unicode_pwd, salt, &[]).unwrap();

    assert_eq!(hash.len(), 32);
}

#[test]
fn test_algorithm_2b_with_u_entry() {
    // Test with a mock U entry (48 bytes)
    let password = b"test";
    let salt = b"12345678";
    let u_entry = [0x42u8; 48];

    let hash_with_u = compute_hash_r6_algorithm_2b(password, salt, &u_entry).unwrap();
    let hash_without_u = compute_hash_r6_algorithm_2b(password, salt, &[]).unwrap();

    assert_eq!(hash_with_u.len(), 32);
    assert_ne!(
        hash_with_u, hash_without_u,
        "U entry must affect the hash output"
    );
}

#[test]
fn test_algorithm_2b_minimum_rounds() {
    // Algorithm must execute at least 64 rounds
    // We can't directly test this without instrumenting the code,
    // but we can verify it doesn't fail instantly
    let password = b"test";
    let salt = b"12345678";

    let start = std::time::Instant::now();
    let hash = compute_hash_r6_algorithm_2b(password, salt, &[]).unwrap();
    let elapsed = start.elapsed();

    assert_eq!(hash.len(), 32);
    // Should take at least a few milliseconds for 64+ AES rounds
    // (Being generous here since CI might be slow)
    assert!(
        elapsed.as_micros() > 100,
        "Algorithm 2.B should take measurable time, got {:?}",
        elapsed
    );
}

// ============================================================================
// Edge Case Tests (Security & Robustness)
// ============================================================================

#[test]
fn test_algorithm_2b_short_salt() {
    // Salt shorter than 8 bytes should still work (spec doesn't mandate exact length)
    let result = compute_hash_r6_algorithm_2b(b"test", b"short", &[]);
    assert!(result.is_ok(), "Short salt should not cause error");
    assert_eq!(result.unwrap().len(), 32);
}

#[test]
fn test_algorithm_2b_password_at_max_length() {
    // Password exactly at 127 bytes should work
    let max_pwd = vec![b'a'; 127];
    let result = compute_hash_r6_algorithm_2b(&max_pwd, b"12345678", &[]);
    assert!(result.is_ok(), "127-byte password should work");
    assert_eq!(result.unwrap().len(), 32);
}

#[test]
fn test_algorithm_2b_password_exceeds_max_length() {
    // Password > 127 bytes should be rejected (DoS protection)
    let too_long_pwd = vec![b'a'; 128];
    let result = compute_hash_r6_algorithm_2b(&too_long_pwd, b"12345678", &[]);
    assert!(result.is_err(), "Password > 127 bytes should be rejected");

    let err = result.unwrap_err();
    let err_msg = format!("{}", err);
    assert!(
        err_msg.contains("too long") || err_msg.contains("128"),
        "Error should mention password length: {}",
        err_msg
    );
}

#[test]
fn test_algorithm_2b_partial_u_entry() {
    // U entry shorter than 48 bytes should work (uses what's available)
    let short_u = [0x42u8; 32]; // Only 32 bytes instead of 48
    let result = compute_hash_r6_algorithm_2b(b"test", b"12345678", &short_u);
    assert!(result.is_ok(), "Partial U entry should work");
    assert_eq!(result.unwrap().len(), 32);
}

#[test]
fn test_algorithm_2b_hash_selector_math_equivalence() {
    // Verify that sum(bytes) mod 3 == BigInteger(bytes) mod 3
    // This is because 256 mod 3 = 1, so 256^k mod 3 = 1 for all k

    // Example: [0x01, 0x00] as big-endian = 256, as sum = 1
    // 256 mod 3 = 1, sum mod 3 = 1 ✓
    let bytes1 = [0x01u8, 0x00];
    let sum1: u64 = bytes1.iter().map(|&b| b as u64).sum();
    let bigint1: u64 = 0x0100; // 256
    assert_eq!(sum1 % 3, bigint1 % 3, "Equivalence for [0x01, 0x00]");

    // Example: [0xFF, 0xFF] = 65535
    // sum = 255 + 255 = 510, 510 mod 3 = 0
    // bigint = 65535, 65535 mod 3 = 0 ✓
    let bytes2 = [0xFFu8, 0xFF];
    let sum2: u64 = bytes2.iter().map(|&b| b as u64).sum();
    let bigint2: u64 = 0xFFFF;
    assert_eq!(sum2 % 3, bigint2 % 3, "Equivalence for [0xFF, 0xFF]");

    // Example: 16 random bytes
    let bytes3 = [
        0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77,
        0x88,
    ];
    let sum3: u64 = bytes3.iter().map(|&b| b as u64).sum();
    // Calculate BigInteger value mod 3 using the mathematical property
    let mut bigint_mod3: u64 = 0;
    for &b in &bytes3 {
        bigint_mod3 = (bigint_mod3 * 256 + b as u64) % 3;
    }
    assert_eq!(sum3 % 3, bigint_mod3, "Equivalence for 16 random bytes");
}
