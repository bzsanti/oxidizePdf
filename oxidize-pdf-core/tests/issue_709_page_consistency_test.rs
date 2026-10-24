//! Page access must use the same recovered page sequence as enumeration.
use oxidize_pdf::parser::{ParseOptions, PdfDocument, PdfReader};
use std::io::Cursor;
use std::time::{Duration, Instant};

#[path = "common/pdf_assembler.rs"]
mod pdf_assembler;

const ORIGINAL: &[u8] = include_bytes!("fixtures/poppler-85140-0.pdf");

fn fixture(kids: &str, count: u32) -> Vec<u8> {
    pdf_assembler::assemble_pdf(&[
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        format!("<< /Type /Pages /Kids [{kids}] /Count {count} >>").into_bytes(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 125 275] >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 350 450] >>".to_vec(),
        b"null".to_vec(),
    ])
}

fn document(bytes: &[u8], options: ParseOptions) -> PdfDocument<Cursor<Vec<u8>>> {
    PdfDocument::new(PdfReader::new_with_options(Cursor::new(bytes.to_vec()), options).unwrap())
}

#[test]
fn original_overflowing_generation_has_no_accessible_or_fabricated_page() {
    for options in [ParseOptions::default(), ParseOptions::tolerant()] {
        for access_first in [false, true] {
            let doc = document(ORIGINAL, options.clone());
            let start = Instant::now();
            if access_first {
                assert!(doc.get_page(0).is_err());
            }
            assert_eq!(doc.page_count().unwrap(), 0);
            for index in [0, 1, u32::MAX] {
                assert!(doc.get_page(index).is_err(), "invented page at {index}");
            }
            assert!(doc.extract_text().unwrap().is_empty());
            assert!(start.elapsed() < Duration::from_secs(2));
        }
    }
    assert!(PdfReader::new_with_options(Cursor::new(ORIGINAL), ParseOptions::strict()).is_err());
}

#[test]
fn empty_or_broken_tree_does_not_expose_unreachable_page_objects() {
    for (kids, declared_count) in [
        ("", 0),
        ("", 213_804_087),
        ("5 0 R", 0),
        ("5 0 R", 213_804_087),
        ("2 0 R", 0),
        ("2 0 R", 213_804_087),
    ] {
        for options in [
            ParseOptions::default(),
            ParseOptions::tolerant(),
            ParseOptions::strict(),
        ] {
            for access_first in [false, true] {
                let doc = document(&fixture(kids, declared_count), options.clone());
                let start = Instant::now();
                if access_first {
                    assert!(doc.get_page(0).is_err(), "unreachable page: Kids=[{kids}]");
                }
                assert_eq!(doc.page_count().unwrap(), 0);
                for index in [0, 1, u32::MAX] {
                    assert!(doc.get_page(index).is_err(), "Kids=[{kids}], index={index}");
                }
                assert!(doc.extract_text().unwrap().is_empty());
                assert!(start.elapsed() < Duration::from_secs(2));
            }
        }
    }
}

#[test]
fn reachable_pages_retain_order_geometry_and_count_independent_of_declared_count() {
    for declared_count in [0, 2, 213_804_087] {
        for options in [
            ParseOptions::default(),
            ParseOptions::tolerant(),
            ParseOptions::strict(),
        ] {
            let doc = document(&fixture("4 0 R 3 0 R", declared_count), options);
            let first = doc.get_page(0).unwrap();
            assert_eq!(first.obj_ref, (4, 0));
            assert_eq!((first.width(), first.height()), (350.0, 450.0));
            assert_eq!(doc.page_count().unwrap(), 2);
            let second = doc.get_page(1).unwrap();
            assert_eq!(second.obj_ref, (3, 0));
            assert_eq!((second.width(), second.height()), (125.0, 275.0));
            assert_eq!(doc.extract_text().unwrap().len(), 2);
            assert!(doc.get_page(2).is_err());
            assert!(doc.get_page(u32::MAX).is_err());
        }
    }
}

#[test]
fn repaired_generation_control_recovers_the_real_geometry_without_trusting_count() {
    let repaired = String::from_utf8(ORIGINAL.to_vec())
        .unwrap()
        .replace("3 18446744073709551616 obj", "3 0 obj")
        .replace("4 233245 obj", "4 0 obj");
    for options in [ParseOptions::default(), ParseOptions::tolerant()] {
        let doc = document(repaired.as_bytes(), options);
        assert_eq!(doc.page_count().unwrap(), 1);
        let page = doc.get_page(0).unwrap();
        assert_eq!(page.obj_ref, (3, 0));
        assert_eq!((page.width(), page.height()), (595.0, 2_147_483_647.0));
        assert!(doc.get_page(1).is_err());
    }
}
