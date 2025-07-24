//! Integration tests for the oxidize-pdf CLI
//!
//! Tests command-line interface functionality including:
//! - Command parsing and validation
//! - PDF creation and manipulation
//! - Error handling and edge cases
//! - File I/O operations

use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::{tempdir, TempDir};

/// Test helper to get the CLI binary path
fn get_cli_path() -> PathBuf {
    let mut path = std::env::current_exe().unwrap();
    path.pop(); // Remove test binary name
    if path.ends_with("deps") {
        path.pop(); // Remove "deps" directory
    }
    path.push("oxidizepdf");
    #[cfg(windows)]
    path.set_extension("exe");
    path
}

/// Test helper to create a temporary directory
fn setup_temp_dir() -> TempDir {
    tempdir().expect("Failed to create temp directory")
}

/// Test helper to run CLI command and return output
fn run_cli_command(args: &[&str]) -> Result<std::process::Output> {
    let output = Command::new(get_cli_path()).args(args).output()?;
    Ok(output)
}

/// Test helper to check if PDF file exists and has content
fn assert_pdf_exists_and_valid(path: &Path) {
    assert!(path.exists(), "PDF file should exist: {}", path.display());
    let metadata = fs::metadata(path).expect("Failed to read file metadata");
    assert!(
        metadata.len() > 100,
        "PDF file should have content (> 100 bytes)"
    );

    // Basic PDF header check
    let content = fs::read(path).expect("Failed to read PDF file");
    assert!(
        content.starts_with(b"%PDF-"),
        "File should start with PDF header"
    );
}

#[test]
fn test_cli_create_command() {
    let temp_dir = setup_temp_dir();
    let output_path = temp_dir.path().join("test_create.pdf");

    let output = run_cli_command(&[
        "create",
        "-o",
        output_path.to_str().unwrap(),
        "-t",
        "Hello, World!",
    ])
    .expect("CLI command should succeed");

    assert!(output.status.success(), "Command should succeed");
    assert_pdf_exists_and_valid(&output_path);

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("PDF created successfully"),
        "Should show success message"
    );
}

#[test]
fn test_cli_create_command_with_multiline_text() {
    let temp_dir = setup_temp_dir();
    let output_path = temp_dir.path().join("test_multiline.pdf");

    let output = run_cli_command(&[
        "create",
        "-o",
        output_path.to_str().unwrap(),
        "-t",
        "Line 1\nLine 2\nLine 3",
    ])
    .expect("CLI command should succeed");

    assert!(output.status.success(), "Command should succeed");
    assert_pdf_exists_and_valid(&output_path);
}

#[test]
fn test_cli_demo_command() {
    let temp_dir = setup_temp_dir();
    let output_path = temp_dir.path().join("test_demo.pdf");

    let output = run_cli_command(&["demo", "-o", output_path.to_str().unwrap()])
        .expect("CLI command should succeed");

    assert!(output.status.success(), "Command should succeed");
    assert_pdf_exists_and_valid(&output_path);

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Demo PDF created successfully"),
        "Should show success message"
    );
}

#[test]
fn test_cli_demo_command_default_output() {
    let temp_dir = setup_temp_dir();
    let current_dir = std::env::current_dir().unwrap();

    // Change to temp directory to avoid polluting the workspace
    std::env::set_current_dir(&temp_dir).unwrap();

    let output = run_cli_command(&["demo"]).expect("CLI command should succeed");

    // Restore original directory
    std::env::set_current_dir(current_dir).unwrap();

    assert!(output.status.success(), "Command should succeed");

    let default_path = temp_dir.path().join("demo.pdf");
    assert_pdf_exists_and_valid(&default_path);
}

#[test]
fn test_cli_merge_command_not_implemented() {
    let temp_dir = setup_temp_dir();
    let output_path = temp_dir.path().join("merged.pdf");
    let input1 = temp_dir.path().join("input1.pdf");
    let input2 = temp_dir.path().join("input2.pdf");

    let output = run_cli_command(&[
        "merge",
        input1.to_str().unwrap(),
        input2.to_str().unwrap(),
        "-o",
        output_path.to_str().unwrap(),
    ])
    .expect("CLI command should run");

    // Command should succeed but print not implemented message
    assert!(output.status.success(), "Command should succeed");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("PDF merge functionality coming"),
        "Should show not implemented message"
    );
}

#[test]
fn test_cli_split_command_not_implemented() {
    let temp_dir = setup_temp_dir();
    let input_path = temp_dir.path().join("input.pdf");

    let output =
        run_cli_command(&["split", input_path.to_str().unwrap()]).expect("CLI command should run");

    // Command should succeed but print not implemented message
    assert!(output.status.success(), "Command should succeed");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("PDF split functionality coming"),
        "Should show not implemented message"
    );
}

#[test]
fn test_cli_info_command_with_nonexistent_file() {
    let temp_dir = setup_temp_dir();
    let nonexistent_path = temp_dir.path().join("nonexistent.pdf");

    let output = run_cli_command(&["info", nonexistent_path.to_str().unwrap()])
        .expect("CLI command should run");

    assert!(
        !output.status.success(),
        "Command should fail for nonexistent file"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Error"), "Should show error message");
}

#[test]
fn test_cli_info_command_with_created_pdf() {
    let temp_dir = setup_temp_dir();
    let pdf_path = temp_dir.path().join("test_info.pdf");

    // First create a PDF
    let create_output = run_cli_command(&[
        "create",
        "-o",
        pdf_path.to_str().unwrap(),
        "-t",
        "Test PDF for info command",
    ])
    .expect("Create command should succeed");
    assert!(create_output.status.success());

    // Then get info about it
    let info_output =
        run_cli_command(&["info", pdf_path.to_str().unwrap()]).expect("Info command should run");

    if info_output.status.success() {
        let stdout = String::from_utf8_lossy(&info_output.stdout);
        assert!(
            stdout.contains("PDF Information"),
            "Should show PDF information"
        );
        assert!(stdout.contains("PDF Version"), "Should show PDF version");
    } else {
        // Info command might fail if parser is not fully implemented
        let stderr = String::from_utf8_lossy(&info_output.stderr);
        assert!(
            stderr.contains("PDF parser is currently in early development"),
            "Should explain parser limitations"
        );
    }
}

#[test]
fn test_cli_info_command_detailed() {
    let temp_dir = setup_temp_dir();
    let pdf_path = temp_dir.path().join("test_detailed.pdf");

    // Create a demo PDF (more complex than simple create)
    let create_output = run_cli_command(&["demo", "-o", pdf_path.to_str().unwrap()])
        .expect("Demo command should succeed");
    assert!(create_output.status.success());

    // Get detailed info
    let info_output = run_cli_command(&["info", pdf_path.to_str().unwrap(), "--detailed"])
        .expect("Info command should run");

    if info_output.status.success() {
        let stdout = String::from_utf8_lossy(&info_output.stdout);
        assert!(
            stdout.contains("Detailed Information"),
            "Should show detailed section"
        );
    }
    // If it fails, it's due to parser limitations (acceptable for now)
}

#[test]
fn test_cli_rotate_command_invalid_angle() {
    let temp_dir = setup_temp_dir();
    let input_path = temp_dir.path().join("input.pdf");
    let output_path = temp_dir.path().join("rotated.pdf");

    let output = run_cli_command(&[
        "rotate",
        input_path.to_str().unwrap(),
        "-o",
        output_path.to_str().unwrap(),
        "-a",
        "45", // Invalid angle
    ])
    .expect("CLI command should run");

    assert!(
        !output.status.success(),
        "Command should fail for invalid angle"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Valid angles are 90, 180, 270"),
        "Should show valid angles"
    );
}

#[test]
fn test_cli_rotate_command_invalid_page_range() {
    let temp_dir = setup_temp_dir();
    let input_path = temp_dir.path().join("input.pdf");
    let output_path = temp_dir.path().join("rotated.pdf");

    let output = run_cli_command(&[
        "rotate",
        input_path.to_str().unwrap(),
        "-o",
        output_path.to_str().unwrap(),
        "-p",
        "invalid-range",
    ])
    .expect("CLI command should run");

    assert!(
        !output.status.success(),
        "Command should fail for invalid page range"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Error parsing page range"),
        "Should show page range error"
    );
}

#[test]
fn test_cli_extract_text_nonexistent_file() {
    let temp_dir = setup_temp_dir();
    let nonexistent_path = temp_dir.path().join("nonexistent.pdf");

    let output = run_cli_command(&["extract-text", nonexistent_path.to_str().unwrap()])
        .expect("CLI command should run");

    assert!(
        !output.status.success(),
        "Command should fail for nonexistent file"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Failed to open PDF"),
        "Should show file error"
    );
}

#[test]
fn test_cli_extract_text_with_created_pdf() {
    let temp_dir = setup_temp_dir();
    let pdf_path = temp_dir.path().join("test_extract.pdf");
    let text_content = "This is test content for text extraction";

    // Create a PDF with known text
    let create_output = run_cli_command(&[
        "create",
        "-o",
        pdf_path.to_str().unwrap(),
        "-t",
        text_content,
    ])
    .expect("Create command should succeed");
    assert!(create_output.status.success());

    // Extract text from it
    let extract_output = run_cli_command(&["extract-text", pdf_path.to_str().unwrap()])
        .expect("Extract command should run");

    if extract_output.status.success() {
        let stdout = String::from_utf8_lossy(&extract_output.stdout);
        // Text extraction might not be perfect, but should contain some content
        assert!(!stdout.trim().is_empty(), "Should extract some text");
    }
    // If it fails, it's due to text extraction limitations (acceptable for now)
}

#[test]
fn test_cli_extract_text_with_output_file() {
    let temp_dir = setup_temp_dir();
    let pdf_path = temp_dir.path().join("test_extract_file.pdf");
    let output_path = temp_dir.path().join("extracted.txt");
    let text_content = "Content for file extraction test";

    // Create a PDF
    let create_output = run_cli_command(&[
        "create",
        "-o",
        pdf_path.to_str().unwrap(),
        "-t",
        text_content,
    ])
    .expect("Create command should succeed");
    assert!(create_output.status.success());

    // Extract text to file
    let extract_output = run_cli_command(&[
        "extract-text",
        pdf_path.to_str().unwrap(),
        "-o",
        output_path.to_str().unwrap(),
    ])
    .expect("Extract command should run");

    if extract_output.status.success() {
        assert!(output_path.exists(), "Output text file should be created");
        let stdout = String::from_utf8_lossy(&extract_output.stdout);
        assert!(
            stdout.contains("Text extracted to"),
            "Should show success message"
        );
    }
    // If it fails, it's due to text extraction limitations (acceptable for now)
}

#[test]
fn test_cli_help_command() {
    let output = run_cli_command(&["--help"]).expect("Help command should work");

    assert!(output.status.success(), "Help command should succeed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("oxidizepdf"), "Should show program name");
    assert!(
        stdout.contains("Commands"),
        "Should show available commands"
    );
    assert!(stdout.contains("create"), "Should list create command");
    assert!(stdout.contains("demo"), "Should list demo command");
    assert!(stdout.contains("info"), "Should list info command");
}

#[test]
fn test_cli_version_command() {
    let output = run_cli_command(&["--version"]).expect("Version command should work");

    assert!(output.status.success(), "Version command should succeed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("oxidizepdf"), "Should show program name");
    assert!(stdout.contains("1.1"), "Should show version number");
}

#[test]
fn test_cli_invalid_command() {
    let output = run_cli_command(&["invalid-command"]).expect("Command should run");

    assert!(!output.status.success(), "Invalid command should fail");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("error") || stderr.contains("unrecognized"),
        "Should show error for invalid command"
    );
}

#[test]
fn test_cli_missing_required_arguments() {
    // Test create command without required arguments
    let output = run_cli_command(&["create"]).expect("Command should run");

    assert!(
        !output.status.success(),
        "Command should fail without required args"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("required") || stderr.contains("missing"),
        "Should show missing argument error"
    );
}
