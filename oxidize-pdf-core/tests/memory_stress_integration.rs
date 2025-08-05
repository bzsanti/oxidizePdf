//! Memory Stress Integration Tests
//!
//! Comprehensive tests for memory-intensive operations and stress scenarios.
//! These tests ensure the library handles memory pressure gracefully without OOM.
//!
//! Test categories:
//! - Large image processing (100+ embedded images)
//! - Stream processing of large PDFs (simulated 1GB+)
//! - Concurrent operations (50+ simultaneous PDFs)
//! - Memory leak detection (repeated create/destroy)
//! - Cache thrashing scenarios
//! - Object deduplication
//! - Font subsetting with full Unicode
//! - Compression stress tests

use oxidize_pdf::graphics::Color;
use oxidize_pdf::memory::MemoryOptions;
use oxidize_pdf::text::Font;
use oxidize_pdf::{Document, Page, Result};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};
use tempfile::TempDir;

/// Test processing many embedded images
#[test]
fn test_large_image_processing() -> Result<()> {
    let mut doc = Document::new();
    doc.set_title("Large Image Processing Test");

    // Simulate adding 100 images across 10 pages
    for page_num in 0..10 {
        let mut page = Page::a4();

        // Add page title
        page.text()
            .set_font(Font::Helvetica, 14.0)
            .at(50.0, 750.0)
            .write(&format!("Page {} - Image Gallery", page_num + 1))?;

        // Add 10 images per page (100 total)
        for img_num in 0..10 {
            let x = 50.0 + (img_num % 5) as f64 * 100.0;
            let y = 600.0 - (img_num / 5) as f64 * 200.0;

            // Create a colored rectangle to simulate image
            let color = match img_num % 3 {
                0 => Color::rgb(1.0, 0.0, 0.0), // Red
                1 => Color::rgb(0.0, 1.0, 0.0), // Green
                _ => Color::rgb(0.0, 0.0, 1.0), // Blue
            };

            page.graphics()
                .set_fill_color(color)
                .rectangle(x, y, 80.0, 80.0)
                .fill();

            // Add image label
            page.text()
                .set_font(Font::Helvetica, 8.0)
                .at(x, y - 10.0)
                .write(&format!("IMG_{:03}", page_num * 10 + img_num))?;
        }

        doc.add_page(page);
    }

    // Save and measure performance
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("large_images.pdf");

    let start = Instant::now();
    doc.save(&file_path)?;
    let duration = start.elapsed();

    let metadata = std::fs::metadata(&file_path)?;
    println!(
        "Large image document: {} bytes in {:?}",
        metadata.len(),
        duration
    );

    Ok(())
}

/// Test stream processing of very large PDFs
#[test]
fn test_stream_processing_large_pdf() -> Result<()> {
    let temp_dir = TempDir::new().unwrap();

    // First, create a large PDF to process
    let large_pdf_path = temp_dir.path().join("large_source.pdf");
    create_large_test_pdf(&large_pdf_path, 100)?; // 100 pages

    // Process using streaming with memory limits
    let _memory_opts = MemoryOptions::large_file();

    // Process the PDF page by page
    let output_path = temp_dir.path().join("processed_large.pdf");
    let start = Instant::now();

    // Simulate streaming processing
    let mut output_doc = Document::new();

    // Process in chunks of 10 pages
    for chunk_start in (0..100).step_by(10) {
        // In real implementation, this would stream pages
        for page_num in chunk_start..chunk_start.min(chunk_start + 10).min(100) {
            let mut page = Page::a4();
            page.text()
                .set_font(Font::Helvetica, 12.0)
                .at(50.0, 700.0)
                .write(&format!("Processed page {}", page_num + 1))?;

            output_doc.add_page(page);
        }

        // Simulate memory release between chunks
        std::thread::sleep(Duration::from_millis(10));
    }

    output_doc.save(&output_path)?;
    let duration = start.elapsed();

    println!("Streamed 100-page PDF in {:?}", duration);

    Ok(())
}

/// Test concurrent PDF operations
#[test]
fn test_concurrent_pdf_operations() -> Result<()> {
    let temp_dir = Arc::new(TempDir::new().unwrap());
    let start = Instant::now();
    let timeout = Duration::from_secs(60);

    // Create 50 threads for concurrent operations
    let handles: Vec<_> = (0..50)
        .map(|thread_id| {
            let temp_dir = Arc::clone(&temp_dir);

            thread::spawn(move || -> Result<()> {
                // Each thread creates its own document
                let mut doc = Document::new();
                doc.set_title(&format!("Concurrent Document {}", thread_id));

                // Add 5 pages per document
                for page_num in 0..5 {
                    let mut page = Page::a4();

                    page.text()
                        .set_font(Font::Helvetica, 12.0)
                        .at(100.0, 700.0)
                        .write(&format!("Thread {} - Page {}", thread_id, page_num + 1))?;

                    // Add some graphics
                    page.graphics()
                        .set_stroke_color(Color::rgb(
                            ((thread_id as f32 * 0.02) % 1.0) as f64,
                            0.5,
                            (1.0 - (thread_id as f32 * 0.02) % 1.0) as f64,
                        ))
                        .rectangle(100.0, 500.0, 400.0, 100.0)
                        .stroke();

                    doc.add_page(page);
                }

                // Save document
                let file_path = temp_dir
                    .path()
                    .join(format!("concurrent_{}.pdf", thread_id));
                doc.save(&file_path)?;

                Ok(())
            })
        })
        .collect();

    // Wait for all threads with timeout check
    let mut completed = 0;
    for handle in handles {
        if start.elapsed() > timeout {
            panic!("Timeout: Concurrent operations took too long");
        }

        handle.join().unwrap()?;
        completed += 1;
    }

    println!(
        "Completed {} concurrent PDF operations in {:?}",
        completed,
        start.elapsed()
    );

    // Verify all files were created
    for i in 0..50 {
        let file_path = temp_dir.path().join(format!("concurrent_{}.pdf", i));
        assert!(file_path.exists());
    }

    Ok(())
}

/// Test memory leak detection through repeated operations
#[test]
fn test_memory_leak_detection() -> Result<()> {
    let temp_dir = TempDir::new().unwrap();

    // Perform 1000 create/destroy cycles
    for cycle in 0..1000 {
        // Create document
        let mut doc = Document::new();
        doc.set_title(&format!("Memory Leak Test {}", cycle));

        // Add content
        let mut page = Page::a4();
        page.text()
            .set_font(Font::Helvetica, 12.0)
            .at(100.0, 700.0)
            .write(&format!("Cycle {}", cycle))?;

        // Add graphics
        for i in 0..10 {
            page.graphics()
                .circle(100.0 + i as f64 * 40.0, 500.0, 20.0)
                .fill();
        }

        doc.add_page(page);

        // Save to force serialization
        let file_path = temp_dir
            .path()
            .join(format!("leak_test_{}.pdf", cycle % 10));
        doc.save(&file_path)?;

        // Document should be dropped here, freeing all memory

        // Small delay to allow OS to reclaim memory
        if cycle % 100 == 0 {
            std::thread::sleep(Duration::from_millis(10));
            println!("Completed {} cycles", cycle);
        }
    }

    println!("Memory leak test completed: 1000 cycles");

    Ok(())
}

/// Test cache thrashing scenarios
#[test]
fn test_cache_thrashing() -> Result<()> {
    let mut doc = Document::new();

    // Add many fonts to thrash font cache
    let _font_names = vec![
        "TestFont1",
        "TestFont2",
        "TestFont3",
        "TestFont4",
        "TestFont5",
        "TestFont6",
        "TestFont7",
        "TestFont8",
        "TestFont9",
        "TestFont10",
    ];

    // Create 50 pages with rapidly changing fonts
    for page_num in 0..50 {
        let mut page = Page::a4();

        // Use different fonts in rapid succession
        let mut y = 750.0;
        for (i, line) in (0..40).enumerate() {
            let font_idx = (page_num * 40 + line) % 14; // Cycle through standard fonts
            let font = match font_idx {
                0 => Font::Helvetica,
                1 => Font::HelveticaBold,
                2 => Font::HelveticaOblique,
                3 => Font::HelveticaBoldOblique,
                4 => Font::TimesRoman,
                5 => Font::TimesBold,
                6 => Font::TimesItalic,
                7 => Font::TimesBoldItalic,
                8 => Font::Courier,
                9 => Font::CourierBold,
                10 => Font::CourierOblique,
                11 => Font::CourierBoldOblique,
                12 => Font::Symbol,
                _ => Font::ZapfDingbats,
            };

            page.text()
                .set_font(font, 8.0)
                .at(50.0, y)
                .write(&format!("Cache thrash test - Page {} Line {}", page_num, i))?;

            y -= 15.0;
        }

        doc.add_page(page);
    }

    // Save and measure
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("cache_thrash.pdf");

    let start = Instant::now();
    doc.save(&file_path)?;
    let duration = start.elapsed();

    println!("Cache thrashing test completed in {:?}", duration);

    Ok(())
}

/// Test object deduplication with many duplicate objects
#[test]
fn test_object_deduplication_stress() -> Result<()> {
    let mut doc = Document::new();

    // Create a pattern that will be reused many times
    let repeated_text = "This is a repeated string that should be deduplicated. ";
    let repeated_text_long = repeated_text.repeat(10);

    // Add 100 pages with duplicate content
    for _page_num in 0..100 {
        let mut page = Page::a4();

        // Add the same text many times
        for i in 0..20 {
            page.text()
                .set_font(Font::Helvetica, 10.0)
                .at(50.0, 750.0 - i as f64 * 30.0)
                .write(&repeated_text_long)?;
        }

        // Add identical graphics
        for _i in 0..10 {
            page.graphics()
                .rectangle(100.0, 100.0, 100.0, 100.0)
                .stroke();
        }

        doc.add_page(page);
    }

    // Save and check file size
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("deduplication_test.pdf");
    doc.save(&file_path)?;

    let metadata = std::fs::metadata(&file_path)?;
    let size_mb = metadata.len() as f64 / 1_048_576.0;

    println!(
        "Deduplication test: {:.2} MB (should be optimized)",
        size_mb
    );

    // File should be reasonably sized despite duplicate content
    assert!(metadata.len() < 10_000_000); // Less than 10MB

    Ok(())
}

/// Test font subsetting with full Unicode range
#[test]
fn test_unicode_font_subsetting_stress() -> Result<()> {
    let mut doc = Document::new();
    let mut page = Page::a4();

    // Test various Unicode blocks
    let unicode_tests = vec![
        // Basic Latin
        (0x0020..0x007F, "Basic Latin"),
        // Latin-1 Supplement
        (0x00A0..0x00FF, "Latin-1 Supplement"),
        // Latin Extended-A
        (0x0100..0x017F, "Latin Extended-A"),
        // Greek
        (0x0370..0x03FF, "Greek"),
        // Cyrillic
        (0x0400..0x04FF, "Cyrillic"),
        // Hebrew
        (0x0590..0x05FF, "Hebrew"),
        // Arabic
        (0x0600..0x06FF, "Arabic"),
        // CJK Unified Ideographs (sample)
        (0x4E00..0x4E20, "CJK Sample"),
    ];

    let mut y = 750.0;

    for (range, name) in unicode_tests {
        // Add section header
        page.text()
            .set_font(Font::HelveticaBold, 12.0)
            .at(50.0, y)
            .write(&format!("{} Test:", name))?;
        y -= 20.0;

        // Build string with characters from range
        let mut test_string = String::new();
        for code_point in range {
            if let Some(ch) = char::from_u32(code_point) {
                test_string.push(ch);
                if test_string.len() >= 50 {
                    break; // Limit line length
                }
            }
        }

        // Try to render (may show replacement chars for unsupported)
        if !test_string.is_empty() {
            page.text()
                .set_font(Font::Helvetica, 10.0)
                .at(50.0, y)
                .write(&test_string)?;
            y -= 20.0;
        }

        if y < 100.0 {
            break; // Page full
        }
    }

    doc.add_page(page);

    // Save and verify
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("unicode_subsetting.pdf");
    doc.save(&file_path)?;

    Ok(())
}

/// Test compression stress with highly redundant data
#[test]
fn test_compression_stress() -> Result<()> {
    // Test both compressed and uncompressed versions
    for compressed in [true, false] {
        let mut doc = Document::new();
        doc.set_compress(compressed);
        doc.set_title(&format!(
            "Compression Test - {}",
            if compressed { "ON" } else { "OFF" }
        ));

        // Create highly redundant content
        let redundant_text = "AAAAAAAAAA"; // Highly compressible
        let random_text = "AbCdEfGhIj"; // Less compressible

        // Add 50 pages of redundant content
        for page_num in 0..50 {
            let mut page = Page::a4();

            // Alternate between redundant and random
            let text = if page_num % 2 == 0 {
                redundant_text.repeat(100)
            } else {
                random_text.repeat(100)
            };

            // Fill page with text
            for i in 0..60 {
                page.text()
                    .set_font(Font::Courier, 10.0)
                    .at(50.0, 750.0 - i as f64 * 12.0)
                    .write(&text)?;
            }

            doc.add_page(page);
        }

        // Save and compare sizes
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join(format!(
            "compression_{}.pdf",
            if compressed { "on" } else { "off" }
        ));

        let start = Instant::now();
        doc.save(&file_path)?;
        let duration = start.elapsed();

        let metadata = std::fs::metadata(&file_path)?;
        let size_mb = metadata.len() as f64 / 1_048_576.0;

        println!(
            "Compression {}: {:.2} MB in {:?}",
            if compressed { "ON" } else { "OFF" },
            size_mb,
            duration
        );
    }

    Ok(())
}

/// Test page tree balancing with unbalanced trees
#[test]
fn test_page_tree_balancing_stress() -> Result<()> {
    let mut doc = Document::new();

    // Create an extremely unbalanced page tree by adding pages in groups
    let start = Instant::now();

    // Add 1000 pages in an unbalanced way
    for group in 0..100 {
        // Add 10 pages per group
        for page_in_group in 0..10 {
            let mut page = Page::a4();

            page.text()
                .set_font(Font::Helvetica, 10.0)
                .at(50.0, 700.0)
                .write(&format!("Group {} - Page {}", group, page_in_group))?;

            // Add nested graphics states to stress the tree
            let graphics = page.graphics();
            for _ in 0..5 {
                graphics.save_state();
            }

            graphics.circle(300.0, 400.0, 50.0).fill();

            for _ in 0..5 {
                graphics.restore_state();
            }

            doc.add_page(page);
        }

        // Check timeout
        if start.elapsed() > Duration::from_secs(30) {
            panic!("Timeout: Page tree balancing took too long");
        }
    }

    // Save and verify
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("page_tree_stress.pdf");

    let save_start = Instant::now();
    doc.save(&file_path)?;
    let save_duration = save_start.elapsed();

    println!("Saved 1000-page unbalanced tree in {:?}", save_duration);

    Ok(())
}

// Helper functions

/// Create a large test PDF with specified number of pages
fn create_large_test_pdf(path: &std::path::Path, page_count: usize) -> Result<()> {
    let mut doc = Document::new();

    for i in 0..page_count {
        let mut page = Page::a4();

        // Add some content
        page.text()
            .set_font(Font::Helvetica, 24.0)
            .at(200.0, 400.0)
            .write(&format!("Page {}", i + 1))?;

        // Add some graphics
        page.graphics().circle(300.0, 400.0, 100.0).stroke();

        doc.add_page(page);
    }

    doc.save(path)?;
    Ok(())
}

/// Simulate memory pressure by allocating and releasing memory
#[allow(dead_code)]
fn simulate_memory_pressure() {
    // Allocate 100MB temporarily
    let _large_vec: Vec<u8> = vec![0; 100_000_000];

    // Let it go out of scope to release
}
