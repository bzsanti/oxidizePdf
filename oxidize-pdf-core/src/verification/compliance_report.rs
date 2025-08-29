//! ISO Compliance Reporting
//!
//! This module generates comprehensive reports about ISO 32000-1:2008 compliance
//! status, including verification results, statistics, and actionable recommendations.

#![allow(deprecated)]

use crate::verification::iso_matrix::{ComplianceStats, IsoMatrix};
use crate::verification::VerificationLevel;
use crate::{Color, Document, Font, Page, Result as PdfResult};
use chrono::{DateTime, Utc};
use std::collections::HashMap;

/// Complete compliance report
#[derive(Debug, Clone)]
pub struct ComplianceReport {
    pub generated_at: DateTime<Utc>,
    pub matrix_version: String,
    pub overall_stats: ComplianceStats,
    pub section_reports: Vec<SectionReport>,
    pub verification_results: HashMap<String, String>, // Simplified for now
    pub external_validation_summary: ExternalValidationSummary,
    pub recommendations: Vec<Recommendation>,
    pub detailed_findings: Vec<DetailedFinding>,
}

/// Report for a specific ISO section
#[derive(Debug, Clone)]
pub struct SectionReport {
    pub section_id: String,
    pub section_name: String,
    pub iso_reference: String,
    pub total_requirements: u32,
    pub implemented_count: u32,
    pub average_level: f64,
    pub compliance_percentage: f64,
    pub priority_requirements: Vec<String>, // IDs of high-priority missing features
}

/// Summary of external validation results
#[derive(Debug, Clone)]
pub struct ExternalValidationSummary {
    pub total_pdfs_tested: u32,
    pub qpdf_success_rate: f64,
    pub verapdf_success_rate: f64,
    pub adobe_preflight_success_rate: f64,
    pub common_failures: Vec<String>,
    pub tools_available: Vec<String>,
}

/// Actionable recommendation for improvement
#[derive(Debug, Clone)]
pub struct Recommendation {
    pub priority: RecommendationPriority,
    pub title: String,
    pub description: String,
    pub affected_requirements: Vec<String>,
    pub estimated_effort: EffortEstimate,
    pub impact: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum RecommendationPriority {
    Critical,
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone)]
pub enum EffortEstimate {
    Small,      // 1-2 days
    Medium,     // 1 week
    Large,      // 2-4 weeks
    ExtraLarge, // 1+ months
}

/// Detailed finding from verification
#[derive(Debug, Clone)]
pub struct DetailedFinding {
    pub requirement_id: String,
    pub finding_type: FindingType,
    pub severity: FindingSeverity,
    pub description: String,
    pub evidence: Vec<String>,
    pub suggested_fix: Option<String>,
}

#[derive(Debug, Clone)]
pub enum FindingType {
    NotImplemented,
    PartialImplementation,
    StructuralIssue,
    ContentMismatch,
    ExternalValidationFailure,
    TestingGap,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum FindingSeverity {
    Critical,
    Major,
    Minor,
    Info,
}

/// Generate comprehensive compliance report
pub fn generate_compliance_report(matrix: &IsoMatrix) -> ComplianceReport {
    let generated_at = Utc::now();
    let overall_stats = matrix.calculate_compliance_stats();

    let section_reports = generate_section_reports(matrix);
    let recommendations = generate_recommendations(matrix, &overall_stats);
    let detailed_findings = generate_detailed_findings(matrix);

    ComplianceReport {
        generated_at,
        matrix_version: matrix.metadata.version.clone(),
        overall_stats,
        section_reports,
        verification_results: HashMap::new(), // Would be populated with actual test results
        external_validation_summary: ExternalValidationSummary {
            total_pdfs_tested: 0,
            qpdf_success_rate: 0.0,
            verapdf_success_rate: 0.0,
            adobe_preflight_success_rate: 0.0,
            common_failures: Vec::new(),
            tools_available: matrix.validation_tools.external_validators.clone(),
        },
        recommendations,
        detailed_findings,
    }
}

/// Generate section-specific reports
fn generate_section_reports(matrix: &IsoMatrix) -> Vec<SectionReport> {
    let mut reports = Vec::new();

    for (section_id, section) in &matrix.sections {
        let requirements = matrix
            .get_section_requirements(section_id)
            .unwrap_or_default();
        let implemented_count = requirements
            .iter()
            .filter(|req| req.level as u8 > 0)
            .count() as u32;

        let total_percentage: f64 = requirements
            .iter()
            .map(|req| req.level.as_percentage())
            .sum();

        let average_level = if !requirements.is_empty() {
            total_percentage / requirements.len() as f64
        } else {
            0.0
        };

        // Identify priority requirements (high impact, not implemented)
        let priority_requirements: Vec<String> = requirements
            .iter()
            .filter(|req| req.level == VerificationLevel::NotImplemented)
            .filter(|req| is_high_priority_requirement(&req.id))
            .map(|req| req.id.clone())
            .collect();

        reports.push(SectionReport {
            section_id: section_id.clone(),
            section_name: section.name.clone(),
            iso_reference: section.iso_section.clone(),
            total_requirements: requirements.len() as u32,
            implemented_count,
            average_level,
            compliance_percentage: average_level,
            priority_requirements,
        });
    }

    reports
}

/// Generate actionable recommendations
fn generate_recommendations(matrix: &IsoMatrix, stats: &ComplianceStats) -> Vec<Recommendation> {
    let mut recommendations = Vec::new();

    // Critical: If too many level 0 (not implemented)
    if stats.level_0_count > stats.total_requirements / 2 {
        recommendations.push(Recommendation {
            priority: RecommendationPriority::Critical,
            title: "Implement Core PDF Features".to_string(),
            description: format!(
                "Over 50% of ISO requirements ({}/{}) are not implemented. Focus on basic document structure and graphics features first.",
                stats.level_0_count, stats.total_requirements
            ),
            affected_requirements: matrix.get_unimplemented_requirements()
                .into_iter()
                .take(10)
                .map(|req| req.id)
                .collect(),
            estimated_effort: EffortEstimate::ExtraLarge,
            impact: "Essential for basic PDF compatibility".to_string(),
        });
    }

    // High: Upgrade level 1 (code exists) to level 2 (generates PDF)
    if stats.level_1_count > 10 {
        recommendations.push(Recommendation {
            priority: RecommendationPriority::High,
            title: "Verify API Implementations Generate Valid PDFs".to_string(),
            description: format!(
                "{} features have APIs but don't generate proper PDF output. Add verification tests.",
                stats.level_1_count
            ),
            affected_requirements: matrix.get_partially_implemented_requirements()
                .into_iter()
                .filter(|req| req.level == VerificationLevel::CodeExists)
                .map(|req| req.id)
                .collect(),
            estimated_effort: EffortEstimate::Large,
            impact: "Ensures APIs actually work in practice".to_string(),
        });
    }

    // Medium: Add external validation for level 2/3 features
    if stats.level_2_count + stats.level_3_count > 5 {
        recommendations.push(Recommendation {
            priority: RecommendationPriority::Medium,
            title: "Add External Validation Tests".to_string(),
            description: "Upgrade content-verified features to full ISO compliance with qpdf/veraPDF validation.".to_string(),
            affected_requirements: matrix.get_partially_implemented_requirements()
                .into_iter()
                .filter(|req| matches!(req.level, VerificationLevel::GeneratesPdf | VerificationLevel::ContentVerified))
                .map(|req| req.id)
                .collect(),
            estimated_effort: EffortEstimate::Medium,
            impact: "Validates against industry-standard PDF tools".to_string(),
        });
    }

    // Section-specific recommendations
    for (section_id, section) in &matrix.sections {
        if section.summary.compliance_percentage < 25.0 {
            recommendations.push(Recommendation {
                priority: RecommendationPriority::High,
                title: format!("Prioritize {} Implementation", section.name),
                description: format!(
                    "Section {} has {:.1}% compliance. This is a core PDF area that needs attention.",
                    section.iso_section, section.summary.compliance_percentage
                ),
                affected_requirements: matrix.get_section_requirements(section_id)
                    .unwrap_or_default()
                    .into_iter()
                    .filter(|req| req.level == VerificationLevel::NotImplemented)
                    .map(|req| req.id)
                    .collect(),
                estimated_effort: EffortEstimate::Large,
                impact: format!("Essential for {} functionality", section.name),
            });
        }
    }

    recommendations.sort_by_key(|r| r.priority.clone());
    recommendations
}

/// Generate detailed findings from verification
fn generate_detailed_findings(matrix: &IsoMatrix) -> Vec<DetailedFinding> {
    let mut findings = Vec::new();

    for requirement in matrix.get_all_requirements() {
        match requirement.level {
            VerificationLevel::NotImplemented => {
                findings.push(DetailedFinding {
                    requirement_id: requirement.id.clone(),
                    finding_type: FindingType::NotImplemented,
                    severity: if is_critical_requirement(&requirement.id) {
                        FindingSeverity::Critical
                    } else {
                        FindingSeverity::Major
                    },
                    description: format!(
                        "Requirement {} ({}) is not implemented",
                        requirement.id, requirement.name
                    ),
                    evidence: vec![format!(
                        "Implementation field: {}",
                        requirement.implementation.unwrap_or("None".to_string())
                    )],
                    suggested_fix: Some(format!(
                        "Implement {} according to ISO reference {}",
                        requirement.name, requirement.iso_reference
                    )),
                });
            }
            VerificationLevel::CodeExists => {
                findings.push(DetailedFinding {
                    requirement_id: requirement.id.clone(),
                    finding_type: FindingType::PartialImplementation,
                    severity: FindingSeverity::Major,
                    description: format!(
                        "Requirement {} has API but doesn't generate valid PDF output",
                        requirement.id
                    ),
                    evidence: vec![format!(
                        "Implementation exists: {}",
                        requirement.implementation.unwrap_or("Unknown".to_string())
                    )],
                    suggested_fix: Some("Add integration tests that verify PDF output".to_string()),
                });
            }
            VerificationLevel::GeneratesPdf => {
                findings.push(DetailedFinding {
                    requirement_id: requirement.id.clone(),
                    finding_type: FindingType::TestingGap,
                    severity: FindingSeverity::Minor,
                    description: format!(
                        "Requirement {} generates PDF but content not verified",
                        requirement.id
                    ),
                    evidence: vec![
                        "PDF generation works but content structure not validated".to_string()
                    ],
                    suggested_fix: Some("Add content parsing verification tests".to_string()),
                });
            }
            _ => {} // Level 3 and 4 are good
        }
    }

    findings.sort_by_key(|f| f.severity.clone());
    findings
}

/// Check if a requirement is high priority for implementation
fn is_high_priority_requirement(requirement_id: &str) -> bool {
    // Core document structure and basic graphics are high priority
    let high_priority_prefixes = ["7.5", "8.4", "8.6", "9.7"];
    high_priority_prefixes
        .iter()
        .any(|prefix| requirement_id.starts_with(prefix))
}

/// Check if a requirement is critical for basic PDF functionality
fn is_critical_requirement(requirement_id: &str) -> bool {
    // Document catalog, page tree, and basic color spaces are critical
    let critical_requirements = [
        "7.5.2.1", // Document catalog
        "7.5.3.1", // Page tree root
        "8.6.3.1", // DeviceRGB
        "9.7.1.1", // Standard 14 fonts
    ];
    critical_requirements.contains(&requirement_id)
}

/// Format compliance report as markdown
pub fn format_report_markdown(report: &ComplianceReport) -> String {
    let mut output = String::new();

    output.push_str("# ISO 32000-1:2008 Compliance Report\n\n");
    output.push_str(&format!(
        "**Generated:** {}\n",
        report.generated_at.format("%Y-%m-%d %H:%M:%S UTC")
    ));
    output.push_str(&format!(
        "**Matrix Version:** {}\n\n",
        report.matrix_version
    ));

    // Overall Statistics
    output.push_str("## Overall Compliance\n\n");
    output.push_str(&format!(
        "- **Total Requirements:** {}\n",
        report.overall_stats.total_requirements
    ));
    output.push_str(&format!(
        "- **Implemented:** {}\n",
        report.overall_stats.implemented_requirements
    ));
    output.push_str(&format!(
        "- **Average Compliance:** {:.1}%\n\n",
        report.overall_stats.average_compliance_percentage
    ));

    // Level Distribution
    output.push_str("### Implementation Level Distribution\n\n");
    output.push_str(&format!(
        "- Level 0 (Not Implemented): {}\n",
        report.overall_stats.level_0_count
    ));
    output.push_str(&format!(
        "- Level 1 (Code Exists): {}\n",
        report.overall_stats.level_1_count
    ));
    output.push_str(&format!(
        "- Level 2 (Generates PDF): {}\n",
        report.overall_stats.level_2_count
    ));
    output.push_str(&format!(
        "- Level 3 (Content Verified): {}\n",
        report.overall_stats.level_3_count
    ));
    output.push_str(&format!(
        "- Level 4 (ISO Compliant): {}\n\n",
        report.overall_stats.level_4_count
    ));

    // Section Reports
    output.push_str("## Section Compliance\n\n");
    for section in &report.section_reports {
        output.push_str(&format!(
            "### {} ({})\n",
            section.section_name, section.iso_reference
        ));
        output.push_str(&format!(
            "- Compliance: {:.1}%\n",
            section.compliance_percentage
        ));
        output.push_str(&format!(
            "- Implemented: {}/{}\n",
            section.implemented_count, section.total_requirements
        ));

        if !section.priority_requirements.is_empty() {
            output.push_str(&format!(
                "- Priority Missing: {}\n",
                section.priority_requirements.join(", ")
            ));
        }
        output.push('\n');
    }

    // Recommendations
    output.push_str("## Recommendations\n\n");
    for (i, rec) in report.recommendations.iter().enumerate() {
        output.push_str(&format!(
            "### {}. {} ({:?} Priority)\n",
            i + 1,
            rec.title,
            rec.priority
        ));
        output.push_str(&format!("{}\n\n", rec.description));
        output.push_str(&format!("**Impact:** {}\n", rec.impact));
        output.push_str(&format!("**Effort:** {:?}\n", rec.estimated_effort));

        if !rec.affected_requirements.is_empty() {
            output.push_str(&format!(
                "**Affects:** {}\n",
                rec.affected_requirements.join(", ")
            ));
        }
        output.push('\n');
    }

    // Critical Findings
    let critical_findings: Vec<_> = report
        .detailed_findings
        .iter()
        .filter(|f| f.severity == FindingSeverity::Critical)
        .collect();

    if !critical_findings.is_empty() {
        output.push_str("## Critical Issues\n\n");
        for finding in critical_findings {
            output.push_str(&format!(
                "- **{}:** {}\n",
                finding.requirement_id, finding.description
            ));
        }
        output.push('\n');
    }

    output.push_str("---\n*Report generated by oxidize-pdf verification system*\n");

    output
}

/// Generate a professional PDF report of ISO compliance status
pub fn generate_pdf_report(report: &ComplianceReport) -> PdfResult<Vec<u8>> {
    let mut doc = Document::new();
    doc.set_title("ISO 32000-1:2008 Compliance Report");
    doc.set_author("oxidize-pdf verification system");
    doc.set_subject("ISO compliance analysis and recommendations");
    doc.set_creator("oxidize-pdf");

    // Page 1: Cover page
    generate_cover_page(&mut doc, report)?;

    // Page 2: Executive summary
    generate_summary_page(&mut doc, report)?;

    // Pages 3-N: Section details
    generate_section_pages(&mut doc, report)?;

    // Final pages: Recommendations and findings
    generate_recommendations_page(&mut doc, report)?;
    generate_findings_page(&mut doc, report)?;

    doc.to_bytes()
}

/// Generate cover page with title and key metrics
fn generate_cover_page(doc: &mut Document, report: &ComplianceReport) -> PdfResult<()> {
    let mut page = Page::a4();

    // Title
    page.text()
        .set_font(Font::Helvetica, 24.0)
        .at(50.0, 750.0)
        .write("ISO 32000-1:2008 Compliance Report")?;

    // Subtitle
    page.text()
        .set_font(Font::Helvetica, 16.0)
        .at(50.0, 710.0)
        .write("oxidize-pdf Library Verification Analysis")?;

    // Date
    page.text()
        .set_font(Font::TimesRoman, 12.0)
        .at(50.0, 670.0)
        .write(&format!(
            "Generated: {}",
            report.generated_at.format("%Y-%m-%d %H:%M:%S UTC")
        ))?;

    // Matrix version
    page.text()
        .set_font(Font::TimesRoman, 12.0)
        .at(50.0, 650.0)
        .write(&format!("Matrix Version: {}", report.matrix_version))?;

    // Key metrics box
    let box_y = 550.0;
    let box_height = 150.0;

    // Draw background box
    page.graphics()
        .set_fill_color(Color::rgb(0.95, 0.95, 0.95))
        .rectangle(50.0, box_y, 500.0, box_height)
        .fill();

    // Key metrics title
    page.text()
        .set_font(Font::Helvetica, 16.0)
        .at(70.0, box_y + 120.0)
        .write("Executive Summary")?;

    // Total requirements
    page.text()
        .set_font(Font::TimesRoman, 12.0)
        .at(70.0, box_y + 90.0)
        .write(&format!(
            "• Total ISO Requirements: {}",
            report.overall_stats.total_requirements
        ))?;

    // Implemented requirements
    page.text()
        .set_font(Font::TimesRoman, 12.0)
        .at(70.0, box_y + 70.0)
        .write(&format!(
            "• Implemented Features: {}",
            report.overall_stats.implemented_requirements
        ))?;

    // Average compliance
    page.text()
        .set_font(Font::TimesRoman, 12.0)
        .at(70.0, box_y + 50.0)
        .write(&format!(
            "• Average Compliance: {:.1}%",
            report.overall_stats.average_compliance_percentage
        ))?;

    // Implementation rate
    let impl_rate = (report.overall_stats.implemented_requirements as f64
        / report.overall_stats.total_requirements as f64)
        * 100.0;
    page.text()
        .set_font(Font::TimesRoman, 12.0)
        .at(70.0, box_y + 30.0)
        .write(&format!("• Implementation Rate: {:.1}%", impl_rate))?;

    // Footer
    page.text()
        .set_font(Font::Courier, 10.0)
        .at(50.0, 50.0)
        .write("This report provides a comprehensive analysis of ISO 32000-1:2008 compliance")?;

    page.text()
        .set_font(Font::Courier, 10.0)
        .at(50.0, 35.0)
        .write("using real verification methodology with 5 levels of implementation depth.")?;

    doc.add_page(page);
    Ok(())
}

/// Generate summary page with level distribution
fn generate_summary_page(doc: &mut Document, report: &ComplianceReport) -> PdfResult<()> {
    let mut page = Page::a4();

    // Title
    page.text()
        .set_font(Font::Helvetica, 18.0)
        .at(50.0, 750.0)
        .write("Implementation Level Distribution")?;

    let start_y = 700.0;
    let bar_height = 20.0;
    let bar_spacing = 35.0;

    // Level definitions and counts
    let levels = vec![
        (
            "Level 4 (ISO Compliant)",
            report.overall_stats.level_4_count,
            Color::rgb(0.0, 0.5, 0.0),
        ),
        (
            "Level 3 (Content Verified)",
            report.overall_stats.level_3_count,
            Color::rgb(0.56, 0.93, 0.56),
        ),
        (
            "Level 2 (Generates PDF)",
            report.overall_stats.level_2_count,
            Color::rgb(1.0, 1.0, 0.0),
        ),
        (
            "Level 1 (Code Exists)",
            report.overall_stats.level_1_count,
            Color::rgb(1.0, 0.65, 0.0),
        ),
        (
            "Level 0 (Not Implemented)",
            report.overall_stats.level_0_count,
            Color::rgb(1.0, 0.0, 0.0),
        ),
    ];

    for (i, (level_name, count, color)) in levels.iter().enumerate() {
        let y = start_y - (i as f64 * bar_spacing);

        // Draw level label
        page.text()
            .set_font(Font::TimesRoman, 12.0)
            .at(50.0, y + 5.0)
            .write(&format!("{}: {}", level_name, count))?;

        // Calculate bar width (max 300px)
        let max_width = 300.0;
        let width = if report.overall_stats.total_requirements > 0 {
            (*count as f64 / report.overall_stats.total_requirements as f64) * max_width
        } else {
            0.0
        };

        // Draw progress bar background
        page.graphics()
            .set_fill_color(Color::rgb(0.9, 0.9, 0.9))
            .rectangle(200.0, y - 5.0, max_width, bar_height)
            .fill();

        // Draw progress bar
        if width > 0.0 {
            page.graphics()
                .set_fill_color(*color)
                .rectangle(200.0, y - 5.0, width, bar_height)
                .fill();
        }

        // Draw percentage
        let percentage = if report.overall_stats.total_requirements > 0 {
            (*count as f64 / report.overall_stats.total_requirements as f64) * 100.0
        } else {
            0.0
        };

        page.text()
            .set_font(Font::Courier, 10.0)
            .at(520.0, y + 5.0)
            .write(&format!("{:.1}%", percentage))?;
    }

    // Verification methodology explanation
    let explanation_y = 450.0;
    page.text()
        .set_font(Font::Helvetica, 14.0)
        .at(50.0, explanation_y)
        .write("Verification Methodology")?;

    let method_text = [
        "This report uses a 5-level verification system:",
        "",
        "• Level 0: Feature not implemented (0% compliance)",
        "• Level 1: API exists, basic functionality (25% compliance)",
        "• Level 2: Generates valid PDF output (50% compliance)",
        "• Level 3: Content verified with parser (75% compliance)",
        "• Level 4: Full ISO compliance with external validation (100% compliance)",
        "",
        "Each requirement is individually assessed using real PDF generation,",
        "content parsing, and external tool validation where applicable.",
    ];

    for (i, text) in method_text.iter().enumerate() {
        page.text()
            .set_font(Font::TimesRoman, 11.0)
            .at(50.0, explanation_y - 30.0 - (i as f64 * 15.0))
            .write(text)?;
    }

    doc.add_page(page);
    Ok(())
}

/// Generate detailed pages for each ISO section
fn generate_section_pages(doc: &mut Document, report: &ComplianceReport) -> PdfResult<()> {
    for section in &report.section_reports {
        let mut page = Page::a4();

        // Section title
        page.text()
            .set_font(Font::Helvetica, 18.0)
            .at(50.0, 750.0)
            .write(&format!(
                "Section {}: {}",
                section.iso_reference, section.section_name
            ))?;

        // Section compliance bar
        let bar_y = 700.0;
        let bar_width = 400.0;
        let bar_height = 25.0;

        // Background
        page.graphics()
            .set_fill_color(Color::rgb(0.9, 0.9, 0.9))
            .rectangle(50.0, bar_y, bar_width, bar_height)
            .fill();

        // Progress fill
        let progress_width = (section.compliance_percentage / 100.0) * bar_width;
        let progress_color = if section.compliance_percentage >= 75.0 {
            Color::rgb(0.0, 0.7, 0.0)
        } else if section.compliance_percentage >= 50.0 {
            Color::rgb(0.8, 0.8, 0.0)
        } else if section.compliance_percentage >= 25.0 {
            Color::rgb(1.0, 0.6, 0.0)
        } else {
            Color::rgb(1.0, 0.2, 0.2)
        };

        page.graphics()
            .set_fill_color(progress_color)
            .rectangle(50.0, bar_y, progress_width, bar_height)
            .fill();

        // Percentage text
        page.text()
            .set_font(Font::Helvetica, 12.0)
            .at(470.0, bar_y + 8.0)
            .write(&format!("{:.1}%", section.compliance_percentage))?;

        // Section statistics
        let stats_y = 650.0;
        page.text()
            .set_font(Font::TimesRoman, 12.0)
            .at(50.0, stats_y)
            .write(&format!(
                "Total Requirements: {}",
                section.total_requirements
            ))?;

        page.text()
            .set_font(Font::TimesRoman, 12.0)
            .at(50.0, stats_y - 20.0)
            .write(&format!("Implemented: {}", section.implemented_count))?;

        page.text()
            .set_font(Font::TimesRoman, 12.0)
            .at(50.0, stats_y - 40.0)
            .write(&format!("Average Level: {:.1}", section.average_level))?;

        // Priority missing features
        if !section.priority_requirements.is_empty() {
            page.text()
                .set_font(Font::Helvetica, 14.0)
                .at(50.0, stats_y - 80.0)
                .write("Priority Missing Features:")?;

            for (i, req_id) in section.priority_requirements.iter().take(10).enumerate() {
                page.text()
                    .set_font(Font::Courier, 10.0)
                    .at(70.0, stats_y - 110.0 - (i as f64 * 15.0))
                    .write(&format!("• {}", req_id))?;
            }
        }

        doc.add_page(page);
    }

    Ok(())
}

/// Generate recommendations page
fn generate_recommendations_page(doc: &mut Document, report: &ComplianceReport) -> PdfResult<()> {
    let mut page = Page::a4();

    // Title
    page.text()
        .set_font(Font::Helvetica, 18.0)
        .at(50.0, 750.0)
        .write("Recommendations for Improvement")?;

    let mut y = 700.0;
    for (i, recommendation) in report.recommendations.iter().take(8).enumerate() {
        // Priority icon
        let priority_color = match recommendation.priority {
            RecommendationPriority::Critical => Color::rgb(1.0, 0.0, 0.0),
            RecommendationPriority::High => Color::rgb(1.0, 0.5, 0.0),
            RecommendationPriority::Medium => Color::rgb(1.0, 1.0, 0.0),
            RecommendationPriority::Low => Color::rgb(0.5, 0.5, 0.5),
        };

        // Priority indicator
        page.graphics()
            .set_fill_color(priority_color)
            .circle(60.0, y - 5.0, 5.0)
            .fill();

        // Recommendation title
        page.text()
            .set_font(Font::Helvetica, 12.0)
            .at(80.0, y)
            .write(&format!(
                "{}. {} ({:?})",
                i + 1,
                recommendation.title,
                recommendation.priority
            ))?;

        // Description
        let description_words: Vec<&str> = recommendation.description.split_whitespace().collect();
        let mut line = String::new();
        let mut line_y = y - 20.0;

        for word in description_words {
            if line.len() + word.len() > 70 {
                page.text()
                    .set_font(Font::TimesRoman, 10.0)
                    .at(80.0, line_y)
                    .write(&line)?;
                line = word.to_string();
                line_y -= 15.0;
            } else {
                if !line.is_empty() {
                    line.push(' ');
                }
                line.push_str(word);
            }
        }

        if !line.is_empty() {
            page.text()
                .set_font(Font::TimesRoman, 10.0)
                .at(80.0, line_y)
                .write(&line)?;
        }

        y -= 80.0;

        if y < 100.0 {
            break;
        }
    }

    doc.add_page(page);
    Ok(())
}

/// Generate detailed findings page
fn generate_findings_page(doc: &mut Document, report: &ComplianceReport) -> PdfResult<()> {
    let mut page = Page::a4();

    // Title
    page.text()
        .set_font(Font::Helvetica, 18.0)
        .at(50.0, 750.0)
        .write("Detailed Findings")?;

    // Critical findings
    let critical_findings: Vec<_> = report
        .detailed_findings
        .iter()
        .filter(|f| f.severity == FindingSeverity::Critical)
        .take(10)
        .collect();

    if !critical_findings.is_empty() {
        page.text()
            .set_font(Font::Helvetica, 14.0)
            .at(50.0, 700.0)
            .write("Critical Issues:")?;

        let mut y = 670.0;
        for finding in critical_findings {
            page.text()
                .set_font(Font::TimesRoman, 11.0)
                .at(70.0, y)
                .write(&format!(
                    "• {}: {}",
                    finding.requirement_id, finding.description
                ))?;
            y -= 20.0;
        }
    }

    // Summary statistics
    page.text()
        .set_font(Font::Helvetica, 14.0)
        .at(50.0, 400.0)
        .write("Summary Statistics:")?;

    page.text()
        .set_font(Font::TimesRoman, 11.0)
        .at(70.0, 370.0)
        .write(&format!(
            "• Total findings: {}",
            report.detailed_findings.len()
        ))?;

    let critical_count = report
        .detailed_findings
        .iter()
        .filter(|f| f.severity == FindingSeverity::Critical)
        .count();
    page.text()
        .set_font(Font::TimesRoman, 11.0)
        .at(70.0, 350.0)
        .write(&format!("• Critical issues: {}", critical_count))?;

    let major_count = report
        .detailed_findings
        .iter()
        .filter(|f| f.severity == FindingSeverity::Major)
        .count();
    page.text()
        .set_font(Font::TimesRoman, 11.0)
        .at(70.0, 330.0)
        .write(&format!("• Major issues: {}", major_count))?;

    // Footer
    page.text()
        .set_font(Font::Courier, 9.0)
        .at(50.0, 50.0)
        .write("Generated by oxidize-pdf verification system")?;

    page.text()
        .set_font(Font::Courier, 9.0)
        .at(50.0, 35.0)
        .write(&format!(
            "Report timestamp: {}",
            report.generated_at.format("%Y-%m-%d %H:%M:%S UTC")
        ))?;

    doc.add_page(page);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::verification::iso_matrix::IsoMatrix;

    fn create_test_matrix() -> IsoMatrix {
        let toml_content = r#"
[metadata]
version = "2025-08-21"
total_features = 4
specification = "ISO 32000-1:2008"
methodology = "docs/ISO_TESTING_METHODOLOGY.md"

[section_7_5]
name = "Document Structure"
iso_section = "7.5"
total_requirements = 2

[section_7_5.summary]
implemented = 1
average_level = 1.5
compliance_percentage = 37.5

[[section_7_5.requirements]]
id = "7.5.2.1"
name = "Catalog Type Entry"
description = "Document catalog must have /Type /Catalog"
iso_reference = "7.5.2, Table 3.25"
requirement_type = "mandatory"
page = 45
original_text = "The document catalog dictionary's Type entry shall have the value Catalog."

[[section_7_5.requirements]]
id = "7.5.2.2"
name = "Catalog Version Entry"
description = "Optional /Version entry in catalog"
iso_reference = "7.5.2, Table 3.25"
requirement_type = "optional"
page = 45
original_text = "The Version entry may be present to override the PDF version from the header."

[overall_summary]
total_sections = 1
total_requirements = 2
total_implemented = 1
average_level = 1.5
real_compliance_percentage = 37.5
level_0_count = 1
level_1_count = 0
level_2_count = 0
level_3_count = 1
level_4_count = 0

[validation_tools]
external_validators = ["qpdf"]
internal_parser = true
reference_pdfs = false
automated_testing = false
"#;
        toml::from_str(toml_content).unwrap()
    }

    #[test]
    fn test_generate_compliance_report() {
        let matrix = create_test_matrix();
        let report = generate_compliance_report(&matrix);

        assert_eq!(report.overall_stats.total_requirements, 2);
        assert_eq!(report.overall_stats.implemented_requirements, 0); // New system: all start at level 0
        assert_eq!(report.section_reports.len(), 1);
        assert!(!report.recommendations.is_empty());
        assert!(!report.detailed_findings.is_empty());
    }

    #[test]
    fn test_format_report_markdown() {
        let matrix = create_test_matrix();
        let report = generate_compliance_report(&matrix);
        let markdown = format_report_markdown(&report);

        assert!(markdown.contains("# ISO 32000-1:2008 Compliance Report"));
        assert!(markdown.contains("Total Requirements"));
        assert!(markdown.contains("Document Structure"));
        assert!(markdown.contains("Recommendations"));
    }

    #[test]
    fn test_is_high_priority_requirement() {
        assert!(is_high_priority_requirement("7.5.2.1"));
        assert!(is_high_priority_requirement("8.4.1.1"));
        assert!(!is_high_priority_requirement("13.2.1.1"));
    }

    #[test]
    fn test_is_critical_requirement() {
        assert!(is_critical_requirement("7.5.2.1"));
        assert!(is_critical_requirement("8.6.3.1"));
        assert!(!is_critical_requirement("7.5.2.2"));
    }
}
