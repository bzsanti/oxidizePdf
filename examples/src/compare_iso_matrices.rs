//! Compare ISO Compliance Matrices
//!
//! This tool compares the existing ISO compliance matrix with the newly generated
//! comprehensive matrix to identify gaps, improvements, and priorities.
//!
//! Usage:
//!   cargo run --example compare_iso_matrices
//!
//! Output:
//!   - Gap analysis report
//!   - Priority recommendations
//!   - Implementation roadmap

use std::collections::{HashMap, HashSet};
use std::fs;

#[derive(Debug, Clone)]
struct MatrixComparison {
    existing_requirements: usize,
    comprehensive_requirements: usize,
    _matching_requirements: usize,
    new_requirements: Vec<String>,
    _missing_implementations: Vec<String>,
    priority_gaps: Vec<PriorityGap>,
    implementation_status: ImplementationStatus,
}

#[derive(Debug, Clone)]
struct PriorityGap {
    requirement_id: String,
    name: String,
    priority: String,
    current_status: String,
    impact: String,
}

#[derive(Debug, Clone)]
struct ImplementationStatus {
    critical_missing: usize,
    important_missing: usize,
    nice2have_missing: usize,
    critical_partial: usize,
    important_partial: usize,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Comparing ISO Compliance Matrices");
    println!("====================================");

    // Load existing matrix
    let existing_matrix = match fs::read_to_string("ISO_COMPLIANCE_MATRIX.toml") {
        Ok(content) => {
            println!("✓ Loaded existing matrix: {} KB", content.len() / 1024);
            Some(content)
        }
        Err(_) => {
            println!("⚠️  Existing matrix not found, creating baseline comparison");
            None
        }
    };

    // Load comprehensive matrix
    let comprehensive_matrix = fs::read_to_string("ISO_COMPLIANCE_MATRIX_COMPLETE.toml")?;
    println!(
        "✓ Loaded comprehensive matrix: {} KB",
        comprehensive_matrix.len() / 1024
    );

    // Parse and compare matrices
    println!("\n📊 Analyzing matrices...");
    let comparison = if let Some(existing) = existing_matrix {
        compare_matrices(&existing, &comprehensive_matrix)?
    } else {
        create_baseline_comparison(&comprehensive_matrix)?
    };

    println!("✓ Matrix comparison completed");
    println!(
        "  - Existing requirements: {}",
        comparison.existing_requirements
    );
    println!(
        "  - Comprehensive requirements: {}",
        comparison.comprehensive_requirements
    );
    println!(
        "  - New requirements identified: {}",
        comparison.new_requirements.len()
    );

    // Generate comparison report
    println!("\n📝 Generating comparison report...");
    let report = generate_comparison_report(&comparison)?;

    let results_dir = "examples/results";
    fs::create_dir_all(results_dir)?;

    let report_path = format!("{}/iso_matrix_comparison_detailed.md", results_dir);
    fs::write(&report_path, &report)?;

    println!("✓ Comparison report generated: {}", report_path);

    // Generate priority roadmap
    println!("\n📋 Generating implementation roadmap...");
    let roadmap = generate_implementation_roadmap(&comparison)?;

    let roadmap_path = format!("{}/iso_implementation_roadmap.md", results_dir);
    fs::write(&roadmap_path, &roadmap)?;

    println!("✓ Implementation roadmap generated: {}", roadmap_path);

    // Generate executive summary
    println!("\n📊 Generating executive summary...");
    let summary = generate_executive_summary(&comparison)?;

    let summary_path = format!("{}/iso_compliance_executive_summary.md", results_dir);
    fs::write(&summary_path, &summary)?;

    println!("✓ Executive summary generated: {}", summary_path);

    // Show key insights
    println!("\n🎯 Key Insights:");
    println!(
        "  - Implementation completeness: {:.1}%",
        calculate_implementation_percentage(&comparison)
    );
    println!(
        "  - Critical gaps: {}",
        comparison.implementation_status.critical_missing
    );
    println!(
        "  - Important gaps: {}",
        comparison.implementation_status.important_missing
    );
    println!("  - Priority actions: {}", comparison.priority_gaps.len());

    println!("\n🎉 Matrix comparison completed!");
    println!("   - Detailed comparison: {}", report_path);
    println!("   - Implementation roadmap: {}", roadmap_path);
    println!("   - Executive summary: {}", summary_path);

    Ok(())
}

fn compare_matrices(
    existing: &str,
    comprehensive: &str,
) -> Result<MatrixComparison, Box<dyn std::error::Error>> {
    // Parse existing matrix to extract requirements
    let existing_reqs = parse_matrix_requirements(existing)?;
    let comprehensive_reqs = parse_matrix_requirements(comprehensive)?;

    println!("  - Existing requirements parsed: {}", existing_reqs.len());
    println!(
        "  - Comprehensive requirements parsed: {}",
        comprehensive_reqs.len()
    );

    // Find matching, new, and missing requirements
    let existing_ids: HashSet<String> = existing_reqs.keys().cloned().collect();
    let comprehensive_ids: HashSet<String> = comprehensive_reqs.keys().cloned().collect();

    let matching: HashSet<_> = existing_ids.intersection(&comprehensive_ids).collect();
    let new_requirements: Vec<String> = comprehensive_ids
        .difference(&existing_ids)
        .cloned()
        .collect();

    // Analyze priority gaps
    let mut priority_gaps = Vec::new();
    let mut implementation_status = ImplementationStatus {
        critical_missing: 0,
        important_missing: 0,
        nice2have_missing: 0,
        critical_partial: 0,
        important_partial: 0,
    };

    for (req_id, req_info) in &comprehensive_reqs {
        if let Some(priority_str) = req_info.get("priority") {
            if let Some(status_str) = req_info.get("current_status") {
                match (priority_str.as_str(), status_str.as_str()) {
                    ("Critical", "Not Implemented") => implementation_status.critical_missing += 1,
                    ("Important", "Not Implemented") => {
                        implementation_status.important_missing += 1
                    }
                    ("Nice2Have", "Not Implemented") => {
                        implementation_status.nice2have_missing += 1
                    }
                    ("Critical", "Partially Implemented") => {
                        implementation_status.critical_partial += 1
                    }
                    ("Important", "Partially Implemented") => {
                        implementation_status.important_partial += 1
                    }
                    _ => {}
                }

                if status_str != "Implemented" {
                    let impact = determine_impact(priority_str, status_str);
                    priority_gaps.push(PriorityGap {
                        requirement_id: req_id.clone(),
                        name: req_info
                            .get("name")
                            .unwrap_or(&"Unknown".to_string())
                            .clone(),
                        priority: priority_str.clone(),
                        current_status: status_str.clone(),
                        impact,
                    });
                }
            }
        }
    }

    Ok(MatrixComparison {
        existing_requirements: existing_reqs.len(),
        comprehensive_requirements: comprehensive_reqs.len(),
        _matching_requirements: matching.len(),
        new_requirements,
        _missing_implementations: Vec::new(), // TODO: Implement
        priority_gaps,
        implementation_status,
    })
}

fn create_baseline_comparison(
    comprehensive: &str,
) -> Result<MatrixComparison, Box<dyn std::error::Error>> {
    let comprehensive_reqs = parse_matrix_requirements(comprehensive)?;

    let mut priority_gaps = Vec::new();
    let mut implementation_status = ImplementationStatus {
        critical_missing: 0,
        important_missing: 0,
        nice2have_missing: 0,
        critical_partial: 0,
        important_partial: 0,
    };

    for (req_id, req_info) in &comprehensive_reqs {
        if let Some(priority_str) = req_info.get("priority") {
            if let Some(status_str) = req_info.get("current_status") {
                match (priority_str.as_str(), status_str.as_str()) {
                    ("Critical", "Not Implemented") => implementation_status.critical_missing += 1,
                    ("Important", "Not Implemented") => {
                        implementation_status.important_missing += 1
                    }
                    ("Nice2Have", "Not Implemented") => {
                        implementation_status.nice2have_missing += 1
                    }
                    ("Critical", "Partially Implemented") => {
                        implementation_status.critical_partial += 1
                    }
                    ("Important", "Partially Implemented") => {
                        implementation_status.important_partial += 1
                    }
                    _ => {}
                }

                if status_str != "Implemented" {
                    let impact = determine_impact(priority_str, status_str);
                    priority_gaps.push(PriorityGap {
                        requirement_id: req_id.clone(),
                        name: req_info
                            .get("name")
                            .unwrap_or(&"Unknown".to_string())
                            .clone(),
                        priority: priority_str.clone(),
                        current_status: status_str.clone(),
                        impact,
                    });
                }
            }
        }
    }

    Ok(MatrixComparison {
        existing_requirements: 0,
        comprehensive_requirements: comprehensive_reqs.len(),
        _matching_requirements: 0,
        new_requirements: comprehensive_reqs.keys().cloned().collect(),
        _missing_implementations: Vec::new(),
        priority_gaps,
        implementation_status,
    })
}

fn parse_matrix_requirements(
    matrix_content: &str,
) -> Result<HashMap<String, HashMap<String, String>>, Box<dyn std::error::Error>> {
    let mut requirements = HashMap::new();

    // Simple TOML parsing for requirements
    // In a real implementation, would use proper TOML parsing
    let lines: Vec<&str> = matrix_content.lines().collect();
    let mut current_req: Option<HashMap<String, String>> = None;
    let mut current_id: Option<String> = None;

    for line in lines {
        let line = line.trim();

        if line.starts_with("[[") && line.contains(".requirements]]") {
            // Save previous requirement
            if let (Some(req), Some(id)) = (current_req.take(), current_id.take()) {
                requirements.insert(id, req);
            }

            current_req = Some(HashMap::new());
            current_id = None;
        } else if line.starts_with("id = ") {
            let id = line.replace("id = ", "").trim_matches('"').to_string();
            current_id = Some(id);
        } else if let Some(ref mut req) = current_req {
            if line.contains(" = ") {
                let parts: Vec<&str> = line.splitn(2, " = ").collect();
                if parts.len() == 2 {
                    let key = parts[0].trim().to_string();
                    let value = parts[1].trim_matches('"').to_string();
                    req.insert(key, value);
                }
            }
        }
    }

    // Save last requirement
    if let (Some(req), Some(id)) = (current_req, current_id) {
        requirements.insert(id, req);
    }

    Ok(requirements)
}

fn determine_impact(priority: &str, status: &str) -> String {
    match (priority, status) {
        ("Critical", "Not Implemented") => "High Impact - Core functionality missing".to_string(),
        ("Critical", "Partially Implemented") => {
            "Medium Impact - Core functionality incomplete".to_string()
        }
        ("Important", "Not Implemented") => "Medium Impact - Important feature missing".to_string(),
        ("Important", "Partially Implemented") => {
            "Low Impact - Feature needs refinement".to_string()
        }
        ("Nice2Have", "Not Implemented") => "Low Impact - Optional feature".to_string(),
        _ => "Unknown Impact".to_string(),
    }
}

fn calculate_implementation_percentage(comparison: &MatrixComparison) -> f64 {
    if comparison.comprehensive_requirements == 0 {
        return 0.0;
    }

    let implemented = comparison.comprehensive_requirements - comparison.priority_gaps.len();
    (implemented as f64 / comparison.comprehensive_requirements as f64) * 100.0
}

fn generate_comparison_report(
    comparison: &MatrixComparison,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut report = String::new();

    report.push_str(&format!(r#"# ISO 32000-1:2008 Matrix Comparison Report

**Generated**: {}  
**Analysis Type**: Comprehensive Matrix Comparison  

## Overview

This report compares the current oxidize-pdf ISO compliance status against the complete ISO 32000-1:2008 requirements matrix.

## Quantitative Analysis

### Matrix Size Comparison
- **Current Matrix**: {} requirements
- **Comprehensive Matrix**: {} requirements  
- **Coverage Increase**: {}x more comprehensive
- **New Requirements Identified**: {}

### Implementation Status Distribution
- **Critical Missing**: {} requirements
- **Important Missing**: {} requirements  
- **Nice-to-Have Missing**: {} requirements
- **Critical Partial**: {} requirements
- **Important Partial**: {} requirements

## Priority Gap Analysis

### Critical Gaps (High Impact)
"#,
        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
        comparison.existing_requirements,
        comparison.comprehensive_requirements,
        comparison.comprehensive_requirements as f64 / comparison.existing_requirements.max(1) as f64,
        comparison.new_requirements.len(),
        comparison.implementation_status.critical_missing,
        comparison.implementation_status.important_missing,
        comparison.implementation_status.nice2have_missing,
        comparison.implementation_status.critical_partial,
        comparison.implementation_status.important_partial
    ));

    // Add critical gaps
    for gap in &comparison.priority_gaps {
        if gap.priority == "Critical" && gap.current_status == "Not Implemented" {
            report.push_str(&format!(
                "- **{}**: {} ({})\n",
                gap.requirement_id, gap.name, gap.impact
            ));
        }
    }

    report.push_str("\n### Important Gaps (Medium Impact)\n");

    // Add important gaps
    for gap in &comparison.priority_gaps {
        if gap.priority == "Important" && gap.current_status == "Not Implemented" {
            report.push_str(&format!(
                "- **{}**: {} ({})\n",
                gap.requirement_id, gap.name, gap.impact
            ));
        }
    }

    report.push_str("\n## Recommendations\n\n");
    report.push_str("### Immediate Actions (Next Sprint)\n");
    report.push_str(
        "1. **Complete Critical Missing Requirements**: Focus on core PDF functionality gaps\n",
    );
    report.push_str("2. **Verify Critical Partial Implementation**: Ensure existing implementations are complete\n");
    report.push_str("3. **Create Verification Tests**: Implement level 4 compliance tests\n\n");

    report.push_str("### Medium-term Goals (Next Quarter)\n");
    report
        .push_str("1. **Address Important Missing Requirements**: Implement common PDF features\n");
    report
        .push_str("2. **Improve Documentation**: Document implementation status and limitations\n");
    report.push_str("3. **External Validation**: Integrate qpdf/veraPDF validation pipeline\n\n");

    report.push_str("### Long-term Goals (Next Year)\n");
    report.push_str("1. **Complete Optional Features**: Implement nice-to-have requirements based on user needs\n");
    report.push_str("2. **Advanced Features**: Multimedia, advanced graphics, accessibility\n");
    report.push_str("3. **Performance Optimization**: Optimize existing implementations\n");

    Ok(report)
}

fn generate_implementation_roadmap(
    comparison: &MatrixComparison,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut roadmap = String::new();

    roadmap.push_str(&format!(r#"# ISO 32000-1:2008 Implementation Roadmap

**Generated**: {}  
**Target**: Complete ISO 32000-1:2008 Compliance  

## Executive Summary

oxidize-pdf currently implements {:.1}% of ISO 32000-1:2008 requirements. This roadmap prioritizes the remaining {} requirements based on impact and complexity.

## Phase 1: Critical Foundation (Weeks 1-4)
**Goal**: Complete all critical missing requirements

### High Priority Tasks
"#,
        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
        calculate_implementation_percentage(comparison),
        comparison.priority_gaps.len()
    ));

    let mut phase1_tasks = 0;
    for gap in &comparison.priority_gaps {
        if gap.priority == "Critical" && gap.current_status == "Not Implemented" && phase1_tasks < 5
        {
            roadmap.push_str(&format!("- [ ] **{}**: {}\n", gap.requirement_id, gap.name));
            roadmap.push_str(&format!("  - Impact: {}\n", gap.impact));
            roadmap.push_str(&format!("  - Status: {}\n\n", gap.current_status));
            phase1_tasks += 1;
        }
    }

    roadmap.push_str("## Phase 2: Important Features (Weeks 5-12)\n");
    roadmap.push_str("**Goal**: Complete important missing requirements\n\n");

    let mut phase2_tasks = 0;
    for gap in &comparison.priority_gaps {
        if gap.priority == "Important"
            && gap.current_status == "Not Implemented"
            && phase2_tasks < 8
        {
            roadmap.push_str(&format!("- [ ] **{}**: {}\n", gap.requirement_id, gap.name));
            phase2_tasks += 1;
        }
    }

    roadmap.push_str("\n## Phase 3: Enhancement & Polish (Weeks 13-26)\n");
    roadmap.push_str("**Goal**: Complete partial implementations and add optional features\n\n");

    for gap in &comparison.priority_gaps {
        if gap.current_status == "Partially Implemented" {
            roadmap.push_str(&format!(
                "- [ ] **{}**: Complete {}\n",
                gap.requirement_id, gap.name
            ));
        }
    }

    roadmap.push_str("\n## Success Metrics\n\n");
    roadmap.push_str(&format!(
        "- [ ] **Phase 1**: 0 critical gaps (currently {})\n",
        comparison.implementation_status.critical_missing
    ));
    roadmap.push_str(&format!(
        "- [ ] **Phase 2**: 0 important gaps (currently {})\n",
        comparison.implementation_status.important_missing
    ));
    roadmap.push_str("- [ ] **Phase 3**: 95%+ implementation completeness\n");
    roadmap.push_str("- [ ] **Overall**: External validation with qpdf and veraPDF\n");

    Ok(roadmap)
}

fn generate_executive_summary(
    comparison: &MatrixComparison,
) -> Result<String, Box<dyn std::error::Error>> {
    let completion_pct = calculate_implementation_percentage(comparison);
    let critical_gaps = comparison.implementation_status.critical_missing;
    let important_gaps = comparison.implementation_status.important_missing;

    let mut summary = String::new();

    summary.push_str(&format!(
        r#"# ISO 32000-1:2008 Compliance - Executive Summary

**Date**: {}  
**Library**: oxidize-pdf v1.1.9  
**Standard**: ISO 32000-1:2008 (PDF 1.7)  

## Current Status

🎯 **Overall Compliance**: {:.1}%  
📊 **Requirements Analyzed**: {}  
⚠️  **Critical Gaps**: {}  
🔧 **Important Gaps**: {}  

## Key Findings

### Strengths
- **Core PDF Generation**: Basic document structure, objects, and serialization ✅
- **Text Rendering**: Font support and text positioning implemented ✅  
- **Graphics**: Path construction and basic color spaces working ✅
- **Images**: Image XObject support functional ✅

### Critical Gaps
"#,
        chrono::Utc::now().format("%Y-%m-%d"),
        completion_pct,
        comparison.comprehensive_requirements,
        critical_gaps,
        important_gaps
    ));

    if critical_gaps > 0 {
        summary.push_str("- **Security**: Encryption support missing\n");
        summary.push_str("- **Advanced Text**: Text scaling and advanced positioning\n");
        summary.push_str("- **Forms**: Form XObject support incomplete\n");
    } else {
        summary.push_str("- No critical gaps identified ✅\n");
    }

    summary.push_str("\n### Business Impact\n\n");

    if completion_pct >= 80.0 {
        summary.push_str("🟢 **HIGH COMPLIANCE**: Ready for most commercial use cases\n");
    } else if completion_pct >= 60.0 {
        summary.push_str("🟡 **MODERATE COMPLIANCE**: Suitable for basic PDF workflows\n");
    } else {
        summary.push_str("🔴 **LIMITED COMPLIANCE**: Significant gaps in core functionality\n");
    }

    summary.push_str("\n## Recommendations\n\n");
    summary.push_str("### Immediate (Next 4 weeks)\n");
    summary
        .push_str("1. **Address Critical Gaps**: Focus development on critical missing features\n");
    summary.push_str("2. **Implement Verification**: Set up automated compliance testing\n");
    summary.push_str("3. **Document Limitations**: Clearly document current limitations\n\n");

    summary.push_str("### Strategic (Next 6 months)\n");
    summary.push_str("1. **Complete Core Features**: Achieve 90%+ compliance in core areas\n");
    summary.push_str("2. **External Validation**: Integrate industry-standard validation tools\n");
    summary.push_str("3. **Performance Optimization**: Optimize existing implementations\n\n");

    summary.push_str(&"## Risk Assessment\n\n".to_string());

    if critical_gaps > 3 {
        summary
            .push_str("🔴 **HIGH RISK**: Multiple critical gaps may impact core functionality\n");
    } else if critical_gaps > 0 {
        summary.push_str("🟡 **MEDIUM RISK**: Some critical gaps require attention\n");
    } else {
        summary.push_str("🟢 **LOW RISK**: All critical requirements implemented\n");
    }

    summary.push_str("\n---\n");
    summary.push_str("*This analysis is based on ISO 32000-1:2008 standard requirements and current oxidize-pdf implementation status.*\n");

    Ok(summary)
}

