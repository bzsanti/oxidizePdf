//! Generate ISO 32000-1:2008 Compliance Report
//!
//! This example demonstrates basic compliance reporting using oxidize-pdf's verification system.
//!
//! Usage:
//!   cargo run --example generate_compliance_report
//!
//! Output:
//!   - Shows current compliance metrics and system status

use oxidize_pdf::verification::iso_matrix::{
    load_default_matrix, load_default_verification_status,
};
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 ISO 32000-1:2008 Compliance Report Generator");
    println!("================================================");

    // Step 1: Try to load ISO compliance matrix (skip if fails)
    println!("\n📋 Loading ISO compliance matrix...");
    let matrix_result = load_default_matrix();
    match &matrix_result {
        Ok(matrix) => {
            println!("✅ Successfully loaded ISO compliance matrix");
            println!("  - Version: {}", matrix.metadata.version);
            println!("  - Total features: {}", matrix.metadata.total_features);
            println!("  - Sections: {}", matrix.sections.len());
        }
        Err(e) => {
            println!("⚠️  Matrix has issues: {}", e);
            println!("   Continuing with verification status only...");
        }
    }

    // Step 2: Try to load verification status
    println!("\n📊 Loading verification status...");
    let status_result = load_default_verification_status();
    match &status_result {
        Ok(status) => {
            println!("✅ Successfully loaded verification status");
            println!("  - Last updated: {}", status.metadata.last_updated);
            println!(
                "  - Total requirements: {}",
                status.metadata.total_requirements
            );
            println!("  - Status entries: {}", status.status.len());

            // Calculate basic statistics
            if !status.status.is_empty() {
                let levels: Vec<u8> = status.status.values().map(|s| s.level).collect();
                let level_0 = levels.iter().filter(|&&l| l == 0).count();
                let level_1 = levels.iter().filter(|&&l| l == 1).count();
                let level_2 = levels.iter().filter(|&&l| l == 2).count();
                let level_3 = levels.iter().filter(|&&l| l == 3).count();
                let level_4 = levels.iter().filter(|&&l| l == 4).count();
                let avg_level: f64 =
                    levels.iter().map(|&l| l as f64).sum::<f64>() / levels.len() as f64;

                println!("\n📈 Current Implementation Status:");
                println!("  - Level 0 (Not implemented): {}", level_0);
                println!("  - Level 1 (Code exists): {}", level_1);
                println!("  - Level 2 (Generates PDF): {}", level_2);
                println!("  - Level 3 (Content verified): {}", level_3);
                println!("  - Level 4 (ISO compliant): {}", level_4);
                println!("  - Average level: {:.2}", avg_level);
                println!("  - Real compliance: {:.1}%", (avg_level / 4.0) * 100.0);
            }
        }
        Err(e) => {
            println!("❌ Failed to load verification status: {}", e);
            println!("   This is likely due to TOML parsing issues");
        }
    }

    // Step 3: System health check
    println!("\n🏥 System Health Check:");

    // Check if matrix file exists
    let matrix_paths = [
        "../ISO_COMPLIANCE_MATRIX.toml",
        "../../ISO_COMPLIANCE_MATRIX.toml",
        "ISO_COMPLIANCE_MATRIX.toml",
    ];
    let mut matrix_found = false;
    for path in &matrix_paths {
        if Path::new(path).exists() {
            println!("✅ Matrix file found: {}", path);
            matrix_found = true;
            break;
        }
    }
    if !matrix_found {
        println!("❌ Matrix file not found in expected locations");
    }

    // Check if status file exists
    let status_paths = [
        "../ISO_VERIFICATION_STATUS.toml",
        "../../ISO_VERIFICATION_STATUS.toml",
        "ISO_VERIFICATION_STATUS.toml",
    ];
    let mut status_found = false;
    for path in &status_paths {
        if Path::new(path).exists() {
            println!("✅ Status file found: {}", path);
            status_found = true;
            break;
        }
    }
    if !status_found {
        println!("❌ Status file not found in expected locations");
    }

    // Summary
    println!("\n🎯 Summary:");
    if matrix_result.is_ok() && status_result.is_ok() {
        println!("✅ ISO verification system is functional");
        println!("   The compliance tracking infrastructure is working");
        println!("   Ready for implementing and testing ISO requirements");
    } else {
        println!("⚠️  ISO verification system has issues");
        println!("   Some components failed to load properly");
        println!("   Check file locations and TOML formatting");
    }

    println!("\n🚀 Next Steps:");
    println!("  1. Review individual ISO requirements in the matrix");
    println!("  2. Implement core PDF features with proper verification");
    println!("  3. Run verification tests to improve compliance levels");
    println!("  4. Focus on Level 3-4 implementation for critical requirements");

    Ok(())
}
