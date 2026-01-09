//! Encryption performance benchmarks for oxidize-pdf
//!
//! Compares password validation throughput across:
//! - RC4 (R2/R3/R4): MD5-based, 50 iterations
//! - R5 (AES-256): SHA-256, simple hash
//! - R6 (AES-256): Algorithm 2.B with SHA-256/384/512 + AES iterations

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use oxidize_pdf::encryption::{Permissions, StandardSecurityHandler, UserPassword};

/// Benchmark R5 password validation (SHA-256 simple)
fn bench_r5_password_validation(c: &mut Criterion) {
    let handler = StandardSecurityHandler::aes_256_r5();
    let password = UserPassword("benchmark_password".to_string());

    // Pre-compute U entry
    let u_entry = handler.compute_r5_user_hash(&password).unwrap();

    c.bench_function("r5_validate_user_password", |b| {
        b.iter(|| {
            handler
                .validate_r5_user_password(black_box(&password), black_box(&u_entry))
                .unwrap()
        })
    });
}

/// Benchmark R5 hash computation (includes salt generation)
fn bench_r5_hash_computation(c: &mut Criterion) {
    let handler = StandardSecurityHandler::aes_256_r5();
    let password = UserPassword("benchmark_password".to_string());

    c.bench_function("r5_compute_user_hash", |b| {
        b.iter(|| handler.compute_r5_user_hash(black_box(&password)).unwrap())
    });
}

/// Benchmark R6 password validation (Algorithm 2.B - complex)
fn bench_r6_password_validation(c: &mut Criterion) {
    let handler = StandardSecurityHandler::aes_256_r6();
    let password = UserPassword("benchmark_password".to_string());

    // Pre-compute U entry
    let u_entry = handler.compute_r6_user_hash(&password).unwrap();

    c.bench_function("r6_validate_user_password", |b| {
        b.iter(|| {
            handler
                .validate_r6_user_password(black_box(&password), black_box(&u_entry))
                .unwrap()
        })
    });
}

/// Benchmark R6 hash computation (Algorithm 2.B with AES iterations)
fn bench_r6_hash_computation(c: &mut Criterion) {
    let handler = StandardSecurityHandler::aes_256_r6();
    let password = UserPassword("benchmark_password".to_string());

    c.bench_function("r6_compute_user_hash", |b| {
        b.iter(|| handler.compute_r6_user_hash(black_box(&password)).unwrap())
    });
}

/// Benchmark RC4 128-bit password validation
fn bench_rc4_password_validation(c: &mut Criterion) {
    let handler = StandardSecurityHandler::rc4_128bit();
    let password = UserPassword("benchmark_password".to_string());

    // For RC4, we need file_id, owner_hash and permissions
    let file_id = vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];
    let owner_hash = vec![0u8; 32]; // Placeholder owner hash
    let permissions = Permissions::all();

    // Pre-compute U entry using RC4 algorithm
    let u_entry = handler
        .compute_user_hash(&password, &owner_hash, permissions, Some(&file_id))
        .unwrap_or_else(|_| vec![0u8; 32]);

    c.bench_function("rc4_validate_user_password", |b| {
        b.iter(|| {
            handler
                .validate_user_password(
                    black_box(&password),
                    black_box(&u_entry),
                    black_box(&owner_hash),
                    black_box(permissions),
                    black_box(Some(&file_id as &[u8])),
                )
                .unwrap_or(false)
        })
    });
}

/// Compare all encryption methods side by side
fn bench_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("encryption_comparison");

    // R5
    let handler_r5 = StandardSecurityHandler::aes_256_r5();
    let password_r5 = UserPassword("benchmark".to_string());
    let u_r5 = handler_r5.compute_r5_user_hash(&password_r5).unwrap();

    group.bench_function("R5_AES256_SHA256", |b| {
        b.iter(|| {
            handler_r5
                .validate_r5_user_password(&password_r5, &u_r5)
                .unwrap()
        })
    });

    // R6
    let handler_r6 = StandardSecurityHandler::aes_256_r6();
    let password_r6 = UserPassword("benchmark".to_string());
    let u_r6 = handler_r6.compute_r6_user_hash(&password_r6).unwrap();

    group.bench_function("R6_AES256_Algorithm2B", |b| {
        b.iter(|| {
            handler_r6
                .validate_r6_user_password(&password_r6, &u_r6)
                .unwrap()
        })
    });

    // RC4 128-bit
    let handler_rc4 = StandardSecurityHandler::rc4_128bit();
    let password_rc4 = UserPassword("benchmark".to_string());
    let file_id = vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];
    let owner_hash_rc4 = vec![0u8; 32];
    let permissions_rc4 = Permissions::all();
    let u_rc4 = handler_rc4
        .compute_user_hash(
            &password_rc4,
            &owner_hash_rc4,
            permissions_rc4,
            Some(&file_id),
        )
        .unwrap_or_else(|_| vec![0u8; 32]);

    group.bench_function("RC4_128bit_MD5", |b| {
        b.iter(|| {
            handler_rc4
                .validate_user_password(
                    &password_rc4,
                    &u_rc4,
                    &owner_hash_rc4,
                    permissions_rc4,
                    Some(&file_id as &[u8]),
                )
                .unwrap_or(false)
        })
    });

    group.finish();
}

/// Benchmark with varying password lengths
fn bench_password_lengths(c: &mut Criterion) {
    let mut group = c.benchmark_group("password_length_impact");

    let handler_r6 = StandardSecurityHandler::aes_256_r6();

    // Max password length for R6 is 127 bytes
    for length in [8, 16, 32, 64, 127] {
        let password = UserPassword("x".repeat(length));

        group.throughput(Throughput::Bytes(length as u64));
        group.bench_with_input(BenchmarkId::new("R6", length), &password, |b, pwd| {
            let u = handler_r6.compute_r6_user_hash(pwd).unwrap();
            b.iter(|| handler_r6.validate_r6_user_password(pwd, &u).unwrap())
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_r5_password_validation,
    bench_r5_hash_computation,
    bench_r6_password_validation,
    bench_r6_hash_computation,
    bench_rc4_password_validation,
    bench_comparison,
    bench_password_lengths,
);

criterion_main!(benches);
