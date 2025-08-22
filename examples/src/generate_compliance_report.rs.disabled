//! Generate ISO 32000-1:2008 Compliance Report
//!
//! This example demonstrates the complete workflow for generating
//! a visual PDF compliance report using oxidize-pdf's verification system.
//!
//! Usage:
//!   cargo run --example generate_compliance_report
//!
//! Output:
//!   - Creates examples/results/iso_compliance_report.pdf
//!   - Shows real compliance metrics and detailed analysis

use oxidize_pdf::verification::{
    compliance_report::{generate_compliance_report, generate_pdf_report},
    iso_matrix::{
        load_default_matrix, IsoMatrix, IsoRequirementData, IsoSection, MatrixMetadata,
        OverallSummary, SectionSummary, ValidationTools,
    },
};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 ISO 32000-1:2008 Compliance Report Generator");
    println!("================================================");

    // Ensure results directory exists
    let results_dir = Path::new("examples/results");
    if !results_dir.exists() {
        fs::create_dir_all(results_dir)?;
        println!("✓ Created results directory: examples/results/");
    }

    // Step 1: Load ISO compliance matrix
    println!("\n📋 Loading ISO compliance matrix...");
    let matrix = match load_default_matrix() {
        Ok(matrix) => {
            println!("✓ Successfully loaded ISO compliance matrix");
            println!("  - Version: {}", matrix.metadata.version);
            println!("  - Total features: {}", matrix.metadata.total_features);
            println!("  - Specification: {}", matrix.metadata.specification);
            matrix
        }
        Err(e) => {
            println!("⚠️  Could not load ISO compliance matrix: {}", e);
            println!("   Creating a demo matrix for report generation...");

            // Create a demo matrix for testing
            create_demo_matrix()
        }
    };

    // Step 2: Generate compliance analysis
    println!("\n📊 Generating compliance analysis...");
    let report = generate_compliance_report(&matrix);

    println!("✓ Compliance analysis completed");
    println!(
        "  - Overall compliance: {:.1}%",
        report.overall_stats.average_compliance_percentage
    );
    println!("  - Sections analyzed: {}", report.section_reports.len());
    println!(
        "  - Total requirements: {}",
        report.overall_stats.total_requirements
    );
    println!(
        "  - Implemented features: {}",
        report.overall_stats.implemented_requirements
    );

    // Show level distribution
    println!("\n📈 Implementation Level Distribution:");
    println!(
        "  - Level 0 (Not implemented): {}",
        report.overall_stats.level_0_count
    );
    println!(
        "  - Level 1 (Code exists): {}",
        report.overall_stats.level_1_count
    );
    println!(
        "  - Level 2 (Generates PDF): {}",
        report.overall_stats.level_2_count
    );
    println!(
        "  - Level 3 (Content verified): {}",
        report.overall_stats.level_3_count
    );
    println!(
        "  - Level 4 (ISO compliant): {}",
        report.overall_stats.level_4_count
    );

    // Step 3: Generate PDF report
    println!("\n📄 Generating PDF compliance report...");
    let pdf_bytes = generate_pdf_report(&report)?;

    println!("✓ PDF report generated successfully");
    println!("  - Report size: {} bytes", pdf_bytes.len());
    println!("  - Pages: Multiple (cover, summary, sections, recommendations)");

    // Step 4: Save PDF report
    let output_path = results_dir.join("iso_compliance_report.pdf");
    fs::write(&output_path, &pdf_bytes)?;

    println!("✓ PDF report saved to: {}", output_path.display());

    // Step 5: Summary and recommendations
    println!("\n🎯 Summary:");
    println!(
        "  Real ISO 32000-1:2008 compliance: {:.1}%",
        report.overall_stats.average_compliance_percentage
    );
    println!(
        "  Priority recommendations: {}",
        report.recommendations.len()
    );
    println!("  Critical findings: {}", report.detailed_findings.len());

    if !report.recommendations.is_empty() {
        println!("\n💡 Top Recommendations:");
        for (i, rec) in report.recommendations.iter().take(3).enumerate() {
            println!("  {}. {} (Priority: {:?})", i + 1, rec.title, rec.priority);
        }
    }

    if !report.detailed_findings.is_empty() {
        println!("\n⚠️  Critical Findings:");
        for (i, finding) in report.detailed_findings.iter().take(3).enumerate() {
            println!(
                "  {}. {}: {:?}",
                i + 1,
                finding.requirement_id,
                finding.severity
            );
        }
    }

    println!("\n🎉 Compliance report generation completed!");
    println!(
        "   Open {} to view the detailed PDF report",
        output_path.display()
    );

    Ok(())
}

/// Create a demo compliance matrix for testing when TOML file is not available
fn create_demo_matrix() -> IsoMatrix {
    let mut sections = HashMap::new();

    // Add a few demo sections with realistic data
    sections.insert(
        "section_7_5".to_string(),
        IsoSection {
            name: "Document Structure".to_string(),
            iso_section: "7.5".to_string(),
            total_requirements: 43,
            summary: SectionSummary {
                implemented: 18,
                average_level: 2.1,
                compliance_percentage: 41.9,
            },
            requirements: vec![
                IsoRequirementData {
                    id: "7.5.2.1".to_string(),
                    name: "Catalog Type Entry".to_string(),
                    description: "Document catalog must have /Type /Catalog".to_string(),
                    iso_reference: "7.5.2, Table 3.25".to_string(),
                    implementation: "src/document.rs:156-160".to_string(),
                    test_file: "tests/iso_verification/section_7/test_catalog.rs".to_string(),
                    level: 3,
                    verified: true,
                    external_validation: Some("qpdf".to_string()),
                    notes: "Implemented and verified with content parsing".to_string(),
                },
                IsoRequirementData {
                    id: "7.5.2.2".to_string(),
                    name: "Catalog Version Entry".to_string(),
                    description: "Optional /Version entry in catalog".to_string(),
                    iso_reference: "7.5.2, Table 3.25".to_string(),
                    implementation: "None".to_string(),
                    test_file: "None".to_string(),
                    level: 0,
                    verified: false,
                    external_validation: None,
                    notes: "Not implemented - always uses PDF header version".to_string(),
                },
            ],
        },
    );

    sections.insert(
        "section_8_6".to_string(),
        IsoSection {
            name: "Color Spaces".to_string(),
            iso_section: "8.6".to_string(),
            total_requirements: 24,
            summary: SectionSummary {
                implemented: 15,
                average_level: 2.4,
                compliance_percentage: 60.0,
            },
            requirements: vec![IsoRequirementData {
                id: "8.6.3.1".to_string(),
                name: "DeviceRGB Color Space".to_string(),
                description: "Support for RGB color specification".to_string(),
                iso_reference: "8.6.3".to_string(),
                implementation: "src/graphics/color.rs:89-112".to_string(),
                test_file: "tests/iso_verification/section_8/test_color_spaces.rs".to_string(),
                level: 4,
                verified: true,
                external_validation: Some("qpdf,verapdf".to_string()),
                notes: "Fully compliant, verified with external tools".to_string(),
            }],
        },
    );

    IsoMatrix {
        metadata: MatrixMetadata {
            version: "2025-08-21-demo".to_string(),
            total_features: 100,
            specification: "ISO 32000-1:2008".to_string(),
            methodology: "demo_methodology".to_string(),
        },
        sections,
        overall_summary: OverallSummary {
            total_sections: 2,
            total_requirements: 100,
            total_implemented: 33,
            average_level: 1.67,
            real_compliance_percentage: 33.0,
            level_0_count: 67,
            level_1_count: 15,
            level_2_count: 10,
            level_3_count: 6,
            level_4_count: 2,
        },
        validation_tools: ValidationTools {
            external_validators: vec!["qpdf".to_string(), "verapdf".to_string()],
            internal_parser: true,
            reference_pdfs: false,
            automated_testing: false,
        },
    }
}
