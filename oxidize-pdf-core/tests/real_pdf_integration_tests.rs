//! Real PDF Integration Tests
//!
//! These tests use actual PDF files when available for comprehensive testing.
//! They are designed to work with or without local PDF fixtures.

mod fixtures_support;

use fixtures_support::*;
use std::fs;

/// Test PDF fixture detection and basic functionality
#[test]
#[cfg_attr(
    not(feature = "real-pdf-tests"),
    ignore = "real-pdf-tests feature not enabled"
)]
fn test_pdf_parsing_with_fixtures() {
    log_fixture_status();

    let mut total_tested = 0;
    let mut successful_tests = 0;

    // Test with real PDFs if available
    if fixtures_available() {
        println!("📁 Testing with real PDF fixtures...");
        let fixture_sample = get_fixture_sample(10); // Sample for speed

        for pdf_path in fixture_sample {
            total_tested += 1;

            match fs::read(&pdf_path) {
                Ok(pdf_bytes) => {
                    // Basic validation - check PDF header
                    if pdf_bytes.len() >= 8 && pdf_bytes.starts_with(b"%PDF-") {
                        successful_tests += 1;
                        println!(
                            "  ✅ {} is valid PDF format",
                            pdf_path.file_name().unwrap().to_string_lossy()
                        );
                    } else {
                        println!(
                            "  ❌ {} has invalid PDF header",
                            pdf_path.file_name().unwrap().to_string_lossy()
                        );
                    }
                }
                Err(e) => {
                    println!(
                        "  🚫 Could not read {}: {}",
                        pdf_path.file_name().unwrap().to_string_lossy(),
                        e
                    );
                }
            }
        }

        println!(
            "📊 Results: {}/{} PDFs passed basic validation ({:.1}% success rate)",
            successful_tests,
            total_tested,
            successful_tests as f64 / total_tested as f64 * 100.0
        );

        // Should have reasonable success rate with real PDFs
        let success_rate = successful_tests as f64 / total_tested as f64;
        assert!(
            success_rate >= 0.8,
            "PDF validation success rate too low: {:.1}%",
            success_rate * 100.0
        );
    } else {
        println!("📂 No PDF fixtures available - test will be skipped");
        println!("   To enable: create symbolic link tests/fixtures -> /path/to/pdfs");
    }
}

/// Test fixture statistics and reporting
#[test]
#[cfg_attr(
    not(feature = "real-pdf-tests"),
    ignore = "real-pdf-tests feature not enabled"
)]
fn test_fixture_statistics() {
    log_fixture_status();

    if fixtures_available() {
        let stats = FixtureStats::collect();
        stats.print_summary();

        // Basic sanity checks
        assert!(
            stats.total_pdfs > 0,
            "Should have PDFs if fixtures available"
        );
        assert!(
            stats.total_size_bytes > 0,
            "Should have total size if PDFs exist"
        );

        println!("✅ Fixture statistics collected successfully");
    } else {
        println!("📂 No fixtures available for statistics test");
    }
}

/// Test PDF content analysis (when fixtures available)
#[test]
#[cfg_attr(
    not(feature = "real-pdf-tests"),
    ignore = "real-pdf-tests feature not enabled"
)]
fn test_pdf_content_analysis() {
    if !fixtures_available() {
        println!("⏭️ Skipping content analysis - no fixtures available");
        return;
    }

    println!("🔍 Analyzing PDF content with real fixtures...");
    let fixtures = get_fixture_sample(20);

    let mut analyzed = 0;
    let mut with_text = 0;
    let mut with_images = 0;
    let mut multipage = 0;

    for pdf_path in fixtures {
        if let Ok(pdf_bytes) = fs::read(&pdf_path) {
            analyzed += 1;

            let pdf_str = String::from_utf8_lossy(&pdf_bytes);

            // Check for text content indicators
            if pdf_str.contains("/Font") || pdf_str.contains(" Tj") || pdf_str.contains(" TJ") {
                with_text += 1;
            }

            // Check for image indicators
            if pdf_str.contains("/Image") || pdf_str.contains("/DCTDecode") {
                with_images += 1;
            }

            // Check for multiple pages
            let page_count = pdf_str.matches("/Type /Page ").count();
            if page_count > 1 {
                multipage += 1;
            }

            println!(
                "  📄 {}: {} pages, text: {}, images: {}",
                pdf_path.file_name().unwrap().to_string_lossy(),
                page_count.max(1),
                if pdf_str.contains("/Font") {
                    "yes"
                } else {
                    "no"
                },
                if pdf_str.contains("/Image") {
                    "yes"
                } else {
                    "no"
                }
            );
        }
    }

    println!("📊 Content Analysis Results:");
    println!("   Analyzed: {} PDFs", analyzed);
    println!(
        "   With text: {} ({:.1}%)",
        with_text,
        with_text as f64 / analyzed as f64 * 100.0
    );
    println!(
        "   With images: {} ({:.1}%)",
        with_images,
        with_images as f64 / analyzed as f64 * 100.0
    );
    println!(
        "   Multi-page: {} ({:.1}%)",
        multipage,
        multipage as f64 / analyzed as f64 * 100.0
    );

    assert!(analyzed > 0, "Should have analyzed some PDFs");
}

/// Performance test with real PDFs (when available)
#[test]
#[cfg_attr(
    not(feature = "real-pdf-tests"),
    ignore = "real-pdf-tests feature not enabled"
)]
fn test_pdf_performance_benchmark() {
    if !fixtures_available() {
        println!("⏭️ Skipping performance benchmark - no fixtures available");
        return;
    }

    println!("🏃 Running PDF I/O performance benchmark...");
    let stats = FixtureStats::collect();
    stats.print_summary();

    let fixtures = get_fixture_pdfs();
    let test_count = fixtures.len().min(50); // Limit for reasonable test time

    let start_time = std::time::Instant::now();
    let mut total_bytes = 0u64;
    let mut successful_reads = 0;

    for pdf_path in fixtures.iter().take(test_count) {
        match fs::read(pdf_path) {
            Ok(pdf_bytes) => {
                total_bytes += pdf_bytes.len() as u64;
                successful_reads += 1;

                // Simple validation
                if !pdf_bytes.starts_with(b"%PDF-") {
                    println!(
                        "  ⚠️ {} has invalid header",
                        pdf_path.file_name().unwrap().to_string_lossy()
                    );
                }
            }
            Err(e) => {
                println!(
                    "  ❌ Failed to read {}: {}",
                    pdf_path.file_name().unwrap().to_string_lossy(),
                    e
                );
            }
        }
    }

    let elapsed = start_time.elapsed();

    println!("⏱️ Performance Results:");
    println!("   Processed: {} PDFs", successful_reads);
    println!("   Total size: {:.2} MB", total_bytes as f64 / 1_048_576.0);
    println!("   Total time: {:.2}s", elapsed.as_secs_f64());
    println!(
        "   Throughput: {:.2} MB/s",
        (total_bytes as f64 / 1_048_576.0) / elapsed.as_secs_f64()
    );
    println!(
        "   Avg time per PDF: {:.2}ms",
        elapsed.as_millis() as f64 / successful_reads as f64
    );

    // Performance assertions
    assert!(
        successful_reads > 0,
        "Should have successfully read some PDFs"
    );
    assert!(
        elapsed.as_secs() < 30,
        "Benchmark should complete within 30 seconds"
    );
}

/// Test environment detection
#[test]
fn test_environment_detection() {
    log_fixture_status();

    // Test CI detection
    let in_ci = std::env::var("CI").is_ok();
    if in_ci {
        assert!(
            !fixtures_available(),
            "Fixtures should not be available in CI"
        );
        println!("🤖 Correctly detected CI environment");
    }

    // Test fixture disable
    std::env::set_var("OXIDIZE_PDF_FIXTURES", "off");
    assert!(
        !fixtures_available(),
        "Fixtures should be disabled when OXIDIZE_PDF_FIXTURES=off"
    );

    // Clean up
    std::env::remove_var("OXIDIZE_PDF_FIXTURES");

    println!("✅ Environment detection working correctly");
}
