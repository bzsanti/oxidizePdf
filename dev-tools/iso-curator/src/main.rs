//! ISO Curator CLI - Tool for curating ISO 32000-1:2008 compliance matrix
//!
//! This tool filters 7,775 text fragments extracted from the ISO PDF
//! down to ~400 real, verifiable requirements.

use anyhow::Result;
use clap::{Parser, Subcommand};

mod commands;
mod curation;
mod matrix;
mod report;

use commands::{analyze, classify, consolidate, link, scan, stats};
use commands::report as cmd_report;

/// CLI tool for curating ISO 32000-1:2008 compliance matrix
#[derive(Parser)]
#[command(name = "iso-curator")]
#[command(author = "oxidize-pdf team")]
#[command(version = "0.1.0")]
#[command(about = "Curate ISO compliance matrix from fragments to real requirements")]
struct Cli {
    /// Path to the ISO compliance matrix TOML file
    #[arg(short, long, default_value = "../../ISO_COMPLIANCE_MATRIX.toml")]
    matrix: String,

    /// Verbosity level (-v, -vv, -vvv)
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Analyze the matrix and identify valid vs invalid requirements
    Analyze {
        /// Output detailed validation results for each fragment
        #[arg(short, long)]
        detailed: bool,

        /// Export results to JSON file
        #[arg(short, long)]
        output: Option<String>,

        /// Only show fragments matching this pattern
        #[arg(short, long)]
        filter: Option<String>,
    },

    /// Classify requirements by type (mandatory/recommended/optional) and priority
    Classify {
        /// Only classify fragments from this section (e.g., "7.3")
        #[arg(short, long)]
        section: Option<String>,

        /// Export classification to JSON
        #[arg(short, long)]
        output: Option<String>,
    },

    /// Consolidate related fragments into unified requirements
    Consolidate {
        /// Interactive mode - review each consolidation
        #[arg(short, long)]
        interactive: bool,

        /// Output path for curated matrix
        #[arg(short, long, default_value = "../../ISO_COMPLIANCE_MATRIX_CURATED.toml")]
        output: String,

        /// Dry run - show what would be consolidated without writing
        #[arg(short, long)]
        dry_run: bool,
    },

    /// Show statistics about the matrix
    Stats {
        /// Compare with curated matrix if available
        #[arg(short, long)]
        compare: Option<String>,
    },

    /// Scan codebase for ISO implementation references
    Scan {
        /// Path to source directory to scan
        #[arg(short, long, default_value = "../../oxidize-pdf-core/src")]
        source: String,

        /// Export results to JSON file
        #[arg(short, long)]
        output: Option<String>,

        /// Show verbose output with file details
        #[arg(short, long)]
        verbose: bool,
    },

    /// Link scan results to curated matrix requirements
    Link {
        /// Path to scan results JSON file
        #[arg(short = 'r', long, default_value = "scan_results.json")]
        scan_results: String,

        /// Path to curated matrix TOML file
        #[arg(short, long, default_value = "../../ISO_COMPLIANCE_MATRIX_CURATED.toml")]
        curated: String,

        /// Output path for updated matrix (defaults to curated path)
        #[arg(short, long)]
        output: Option<String>,

        /// Minimum confidence threshold for linking (0.0-1.0)
        #[arg(short = 'c', long, default_value = "0.6")]
        min_confidence: f64,
    },

    /// Generate compliance report from curated matrix
    Report {
        /// Path to curated matrix TOML file
        #[arg(short, long, default_value = "../../ISO_COMPLIANCE_MATRIX_CURATED.toml")]
        curated: String,

        /// Export report to JSON file
        #[arg(short, long)]
        output: Option<String>,

        /// Show detailed breakdown with unimplemented requirements
        #[arg(short, long)]
        detailed: bool,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Set up logging based on verbosity
    let log_level = match cli.verbose {
        0 => "warn",
        1 => "info",
        2 => "debug",
        _ => "trace",
    };
    std::env::set_var("RUST_LOG", log_level);

    // Load the matrix
    let matrix_path = std::path::Path::new(&cli.matrix);
    if !matrix_path.exists() {
        anyhow::bail!(
            "Matrix file not found: {}. Run from oxidize-pdf root directory.",
            cli.matrix
        );
    }

    match cli.command {
        Commands::Analyze {
            detailed,
            output,
            filter,
        } => {
            analyze::run(&cli.matrix, detailed, output, filter)?;
        }
        Commands::Classify { section, output } => {
            classify::run(&cli.matrix, section, output)?;
        }
        Commands::Consolidate {
            interactive,
            output,
            dry_run,
        } => {
            consolidate::run(&cli.matrix, interactive, &output, dry_run)?;
        }
        Commands::Stats { compare } => {
            stats::run(&cli.matrix, compare)?;
        }
        Commands::Scan {
            source,
            output,
            verbose,
        } => {
            scan::run(&source, output, verbose)?;
        }
        Commands::Link {
            scan_results,
            curated,
            output,
            min_confidence,
        } => {
            link::run(&scan_results, &curated, output, min_confidence)?;
        }
        Commands::Report {
            curated,
            output,
            detailed,
        } => {
            cmd_report::run(&curated, output, detailed)?;
        }
    }

    Ok(())
}
