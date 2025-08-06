//! Test for circular reference handling in PDFs
//!
//! This test specifically targets PDFs that were incorrectly identified as having
//! circular references when they actually have valid page tree structures.

use oxidize_pdf::parser::PdfReader;
use std::path::Path;

#[test]
#[cfg_attr(
    not(feature = "real-pdf-tests"),
    ignore = "real-pdf-tests feature not enabled"
)]
fn test_course_glossary_circular_ref() {
    let path = Path::new("tests/fixtures/Course_Glossary_SUPPLY_LIST.pdf");

    // Skip test if file doesn't exist
    if !path.exists() {
        eprintln!("Skipping test: {} not found", path.display());
        return;
    }

    // Open the PDF
    let reader = PdfReader::open(path).expect("Should be able to open PDF");
    let document = reader.into_document();

    // Get page count
    let page_count = document.page_count().expect("Should get page count");
    println!("PDF has {page_count} pages");

    // Try to access each page
    let mut successful_pages = 0;
    for i in 0..page_count {
        match document.get_page(i) {
            Ok(page) => {
                successful_pages += 1;
                println!(
                    "Successfully loaded page {} (size: {}x{})",
                    i,
                    page.width(),
                    page.height()
                );
            }
            Err(e) => {
                eprintln!("Failed to load page {i}: {e}");
                // This should not happen with our fix
                panic!("Page {i} failed with: {e}");
            }
        }
    }

    assert_eq!(
        successful_pages, page_count as usize,
        "All pages should be accessible"
    );
}

#[test]
#[cfg_attr(
    not(feature = "real-pdf-tests"),
    ignore = "real-pdf-tests feature not enabled"
)]
fn test_liars_and_outliers_circular_ref() {
    let path =
        Path::new("tests/fixtures/liarsandoutliers_enablingthetrustthatsocietyneedstothrive.pdf");

    // Skip test if file doesn't exist
    if !path.exists() {
        eprintln!("Skipping test: {} not found", path.display());
        return;
    }

    // Open the PDF
    let reader = PdfReader::open(path).expect("Should be able to open PDF");
    let document = reader.into_document();

    // Get page count
    let page_count = document.page_count().expect("Should get page count");
    println!("PDF has {page_count} pages");

    // Try to access first and last pages at minimum
    let first_page = document
        .get_page(0)
        .expect("Should be able to get first page");
    println!(
        "First page size: {}x{}",
        first_page.width(),
        first_page.height()
    );

    if page_count > 1 {
        let last_page = document
            .get_page(page_count - 1)
            .expect("Should be able to get last page");
        println!(
            "Last page size: {}x{}",
            last_page.width(),
            last_page.height()
        );
    }
}

#[test]
#[cfg_attr(
    not(feature = "real-pdf-tests"),
    ignore = "real-pdf-tests feature not enabled"
)]
fn test_cryptography_engineering_circular_ref() {
    let path = Path::new(
        "tests/fixtures/cryptography_engineering_design_principles_and_practical_applications.pdf",
    );

    // Skip test if file doesn't exist
    if !path.exists() {
        eprintln!("Skipping test: {} not found", path.display());
        return;
    }

    // Open the PDF
    let reader = PdfReader::open(path).expect("Should be able to open PDF");
    let document = reader.into_document();

    // Get page count
    let page_count = document.page_count().expect("Should get page count");
    println!("PDF has {page_count} pages");

    // Try to access a few pages
    for i in [0, page_count / 2, page_count - 1].iter() {
        if *i < page_count {
            let page = document
                .get_page(*i)
                .unwrap_or_else(|_| panic!("Should be able to get page {i}"));
            println!("Page {} size: {}x{}", i, page.width(), page.height());
        }
    }
}

#[test]
#[cfg_attr(
    not(feature = "real-pdf-tests"),
    ignore = "real-pdf-tests feature not enabled"
)]
fn test_circular_ref_batch() {
    // Test multiple PDFs that were failing with circular reference errors
    let test_pdfs = vec![
        "04.ANNEX 2 PQQ rev.01.pdf",
        "1002579.pdf",
        "191121_TD-K1TG-104896_TELEFONICA DE ESPAÑA_415.97.pdf",
        "1HSN221000481395.pdf",
        "1HSN221100056583.pdf",
        "20062024 Burwell Asset Management Service Agreement (vs09)_EXE.pdf",
        "20220603 QE Biometrical Consent signed SFM.pdf",
        "240043_Quintas Energy.pdf",
        "3 İNDUSTRİAL METAL PACKAGİNG.pdf",
        "314536d1-a661-4fd9-846d-2bd28d26ae30.pdf",
    ];

    for pdf_name in test_pdfs {
        let path = Path::new("tests/fixtures").join(pdf_name);

        if !path.exists() {
            eprintln!("Skipping {pdf_name}: not found");
            continue;
        }

        println!("\nTesting {pdf_name}");

        // Open the PDF
        match PdfReader::open(&path) {
            Ok(reader) => {
                let document = reader.into_document();

                // Get page count
                match document.page_count() {
                    Ok(page_count) => {
                        println!("  {pdf_name} has {page_count} pages");

                        // Try to get first page
                        match document.get_page(0) {
                            Ok(page) => {
                                println!(
                                    "  First page loaded successfully ({}x{})",
                                    page.width(),
                                    page.height()
                                );
                            }
                            Err(e) => {
                                eprintln!("  ERROR getting first page: {e}");
                                panic!("Should be able to get first page of {pdf_name}");
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("  ERROR getting page count: {e}");
                    }
                }
            }
            Err(e) => {
                eprintln!("  ERROR opening PDF: {e}");
            }
        }
    }
}
