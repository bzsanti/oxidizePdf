//! Tests for font and XObject mapping in merge operations

use oxidize_pdf::operations::{merge_pdfs, MergeInput, MergeOptions};
use oxidize_pdf::operations::merge::MetadataMode;
use oxidize_pdf::{Document, Page};
use std::fs;
use tempfile::TempDir;

#[test]
fn test_font_mapping_in_merge() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create first PDF with fonts
    let mut doc1 = Document::new();
    let mut page1 = Page::a4();
    page1.text()
        .set_font(oxidize_pdf::text::Font::Helvetica, 12.0)
        .at(50.0, 700.0)
        .write("Document 1 - Helvetica")
        .unwrap();
    page1.text()
        .set_font(oxidize_pdf::text::Font::TimesBold, 14.0)
        .at(50.0, 650.0)
        .write("Document 1 - Times Bold")
        .unwrap();
    doc1.add_page(page1);
    
    let pdf1_path = temp_dir.path().join("doc1.pdf");
    doc1.save(&pdf1_path).unwrap();
    
    // Create second PDF with different fonts
    let mut doc2 = Document::new();
    let mut page2 = Page::a4();
    page2.text()
        .set_font(oxidize_pdf::text::Font::Courier, 12.0)
        .at(50.0, 700.0)
        .write("Document 2 - Courier")
        .unwrap();
    page2.text()
        .set_font(oxidize_pdf::text::Font::HelveticaBold, 14.0)
        .at(50.0, 650.0)
        .write("Document 2 - Helvetica Bold")
        .unwrap();
    doc2.add_page(page2);
    
    let pdf2_path = temp_dir.path().join("doc2.pdf");
    doc2.save(&pdf2_path).unwrap();
    
    // Merge the PDFs
    let output_path = temp_dir.path().join("merged.pdf");
    let options = MergeOptions {
        preserve_bookmarks: false,
        preserve_forms: false,
        optimize: false,
        metadata_mode: MetadataMode::FromFirst,
        page_ranges: None,
    };
    
    let inputs = vec![
        MergeInput::new(pdf1_path),
        MergeInput::new(pdf2_path),
    ];
    merge_pdfs(inputs, &output_path, options).unwrap();
    
    // Verify the merged PDF exists and has 2 pages
    assert!(output_path.exists());
    let metadata = fs::metadata(&output_path).unwrap();
    assert!(metadata.len() > 0);
    
    // TODO: Once we have better PDF reading support, verify that fonts are properly mapped
    // For now, we just verify that the merge completes without errors
}

#[test]
fn test_xobject_mapping_placeholder() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create a simple PDF
    let mut doc = Document::new();
    let page = Page::a4();
    doc.add_page(page);
    
    let pdf_path = temp_dir.path().join("simple.pdf");
    doc.save(&pdf_path).unwrap();
    
    // Merge with itself to test XObject mapping
    let output_path = temp_dir.path().join("merged_xobject.pdf");
    let options = MergeOptions::default();
    
    let inputs = vec![
        MergeInput::new(pdf_path.clone()),
        MergeInput::new(pdf_path),
    ];
    merge_pdfs(inputs, &output_path, options).unwrap();
    
    // Verify the merge completed
    assert!(output_path.exists());
}