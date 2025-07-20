//! Fixture Support for PDF Integration Tests
//!
//! This module provides utilities for working with optional local PDF fixtures.
//! The fixtures are NOT part of the repository and should be available only
//! for local development and testing.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

/// Check if PDF fixtures are available for testing
pub fn fixtures_available() -> bool {
    let fixtures_path = Path::new("../tests/fixtures");

    // Skip if running in CI environment
    if env::var("CI").is_ok() {
        return false;
    }

    // Skip if explicitly disabled
    if env::var("OXIDIZE_PDF_FIXTURES").unwrap_or_default() == "off" {
        return false;
    }

    // Check if directory exists and contains PDFs
    if !fixtures_path.exists() {
        return false;
    }

    // Quick check for PDF files
    if let Ok(entries) = fs::read_dir(fixtures_path) {
        for entry in entries {
            if let Ok(entry) = entry {
                if let Some(ext) = entry.path().extension() {
                    if ext == "pdf" {
                        return true;
                    }
                }
            }
        }
    }

    false
}

/// Get all available PDF fixtures
pub fn get_fixture_pdfs() -> Vec<PathBuf> {
    if !fixtures_available() {
        return Vec::new();
    }

    let fixtures_path = Path::new("../tests/fixtures");
    let mut pdfs = Vec::new();

    if let Ok(entries) = fs::read_dir(fixtures_path) {
        for entry in entries {
            if let Ok(entry) = entry {
                let path = entry.path();
                if let Some(ext) = path.extension() {
                    if ext == "pdf" {
                        pdfs.push(path);
                    }
                }
            }
        }
    }

    // Sort for consistent ordering
    pdfs.sort();
    pdfs
}

/// Get a sample of fixture PDFs for quick testing
pub fn get_fixture_sample(size: usize) -> Vec<PathBuf> {
    let all_pdfs = get_fixture_pdfs();

    if all_pdfs.len() <= size {
        return all_pdfs;
    }

    // Simple deterministic sampling by taking every nth file
    let step = all_pdfs.len() / size;
    let mut sample = Vec::new();

    for i in (0..all_pdfs.len()).step_by(step.max(1)) {
        sample.push(all_pdfs[i].clone());
        if sample.len() >= size {
            break;
        }
    }

    sample
}

/// Log fixture availability status
pub fn log_fixture_status() {
    if fixtures_available() {
        let count = get_fixture_pdfs().len();
        println!("📁 Found {} PDF fixtures for testing", count);
    } else if env::var("CI").is_ok() {
        println!("🤖 Running in CI: Using synthetic PDFs only");
    } else if env::var("OXIDIZE_PDF_FIXTURES").unwrap_or_default() == "off" {
        println!("🚫 PDF fixtures disabled via OXIDIZE_PDF_FIXTURES=off");
    } else {
        println!("📂 No PDF fixtures found at ../tests/fixtures/ - using synthetic PDFs only");
    }
}

/// Fixture statistics for reporting
#[derive(Debug)]
pub struct FixtureStats {
    pub total_pdfs: usize,
    pub total_size_bytes: u64,
    pub smallest_pdf: Option<(PathBuf, u64)>,
    pub largest_pdf: Option<(PathBuf, u64)>,
}

impl FixtureStats {
    pub fn collect() -> Self {
        let pdfs = get_fixture_pdfs();
        let mut stats = FixtureStats {
            total_pdfs: pdfs.len(),
            total_size_bytes: 0,
            smallest_pdf: None,
            largest_pdf: None,
        };

        for pdf in &pdfs {
            if let Ok(metadata) = fs::metadata(pdf) {
                let size = metadata.len();
                stats.total_size_bytes += size;

                // Track smallest
                if stats.smallest_pdf.is_none() || size < stats.smallest_pdf.as_ref().unwrap().1 {
                    stats.smallest_pdf = Some((pdf.clone(), size));
                }

                // Track largest
                if stats.largest_pdf.is_none() || size > stats.largest_pdf.as_ref().unwrap().1 {
                    stats.largest_pdf = Some((pdf.clone(), size));
                }
            }
        }

        stats
    }

    pub fn print_summary(&self) {
        println!("📊 Fixture Statistics:");
        println!("   Total PDFs: {}", self.total_pdfs);
        println!(
            "   Total size: {:.2} MB",
            self.total_size_bytes as f64 / 1_048_576.0
        );

        if let Some((path, size)) = &self.smallest_pdf {
            println!(
                "   Smallest: {} ({:.1} KB)",
                path.file_name().unwrap().to_string_lossy(),
                *size as f64 / 1024.0
            );
        }

        if let Some((path, size)) = &self.largest_pdf {
            println!(
                "   Largest: {} ({:.1} MB)",
                path.file_name().unwrap().to_string_lossy(),
                *size as f64 / 1_048_576.0
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fixture_detection() {
        // This test should always pass regardless of fixture availability
        let available = fixtures_available();
        println!("Fixtures available: {}", available);

        if available {
            let pdfs = get_fixture_pdfs();
            assert!(!pdfs.is_empty(), "If fixtures available, should find PDFs");
            println!("Found {} fixture PDFs", pdfs.len());
        }
    }

    #[test]
    fn test_fixture_sampling() {
        let sample = get_fixture_sample(5);
        if fixtures_available() {
            assert!(sample.len() <= 5, "Sample should not exceed requested size");
            assert!(
                sample.len() <= get_fixture_pdfs().len(),
                "Sample should not exceed total"
            );
        } else {
            assert!(sample.is_empty(), "No fixtures should mean empty sample");
        }
    }

    #[test]
    fn test_fixture_stats() {
        let stats = FixtureStats::collect();

        if fixtures_available() {
            assert!(
                stats.total_pdfs > 0,
                "Should have PDFs if fixtures available"
            );
            stats.print_summary();
        } else {
            assert_eq!(stats.total_pdfs, 0, "No fixtures should mean zero PDFs");
        }
    }
}
