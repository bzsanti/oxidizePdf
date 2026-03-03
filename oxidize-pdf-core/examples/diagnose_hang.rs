//! Diagnostic tool to pinpoint where extract_text() hangs.
//! Usage: cargo run --release --example diagnose_hang -- <path.pdf>

use oxidize_pdf::parser::content::ContentParser;
use oxidize_pdf::parser::{PdfDocument, PdfReader};
use std::env;
use std::time::Instant;

fn main() {
    let args: Vec<String> = env::args().collect();
    let path = args.get(1).expect("Usage: diagnose_hang <path.pdf>");

    eprintln!("[1] Opening PDF...");
    let reader = PdfReader::open(path).expect("Failed to open PDF");
    let doc = PdfDocument::new(reader);
    let page_count = doc.page_count().unwrap_or(0);
    eprintln!("[1] OK - {page_count} pages");

    // Only diagnose first page (enough for 5KB PDF)
    let max_pages = page_count.min(3);

    for page_idx in 0..max_pages {
        eprintln!("\n=== PAGE {page_idx} ===");

        eprintln!("[2] Getting page dict...");
        let t = Instant::now();
        let page = match doc.get_page(page_idx) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("[2] FAILED: {e}");
                continue;
            }
        };
        eprintln!("[2] OK - {}ms", t.elapsed().as_millis());

        // Check resources
        eprintln!("[3] Checking resources...");
        if let Some(resources) = page.get_resources() {
            let has_font = resources.get("Font").is_some();
            let has_xobj = resources.get("XObject").is_some();
            let has_cs = resources.get("ColorSpace").is_some();
            eprintln!("[3] Font={has_font}, XObject={has_xobj}, ColorSpace={has_cs}");
        } else {
            eprintln!("[3] No resources dict found");
        }

        // Get content streams
        eprintln!("[4] Getting content streams...");
        let t = Instant::now();
        let streams = match page.content_streams_with_document(&doc) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("[4] FAILED: {e}");
                continue;
            }
        };
        eprintln!(
            "[4] OK - {} streams, sizes: {:?}, {}ms",
            streams.len(),
            streams.iter().map(|s| s.len()).collect::<Vec<_>>(),
            t.elapsed().as_millis()
        );

        // Try parsing each content stream
        for (stream_idx, stream_data) in streams.iter().enumerate() {
            eprintln!(
                "[5] Parsing content stream {stream_idx} ({} bytes)...",
                stream_data.len()
            );

            // Show first 200 bytes of decompressed content
            let preview = String::from_utf8_lossy(&stream_data[..stream_data.len().min(200)]);
            eprintln!("[5] Preview: {:?}", &preview[..preview.len().min(100)]);

            let t = Instant::now();
            match ContentParser::parse_content(stream_data) {
                Ok(ops) => {
                    eprintln!(
                        "[5] OK - {} operations, {}ms",
                        ops.len(),
                        t.elapsed().as_millis()
                    );
                    // Show first few operations
                    for (i, op) in ops.iter().take(10).enumerate() {
                        eprintln!("[5]   op[{i}]: {op:?}");
                    }
                    if ops.len() > 10 {
                        eprintln!("[5]   ... and {} more", ops.len() - 10);
                    }
                }
                Err(e) => {
                    eprintln!("[5] FAILED: {e} ({}ms)", t.elapsed().as_millis());
                }
            }
        }

        // Now try the full extract_text_from_page
        eprintln!("[6] Full extract_text_from_page...");
        let t = Instant::now();
        match doc.extract_text_from_page(page_idx) {
            Ok(extracted) => {
                eprintln!(
                    "[6] OK - {} chars, {} fragments, {}ms",
                    extracted.text.len(),
                    extracted.fragments.len(),
                    t.elapsed().as_millis()
                );
            }
            Err(e) => {
                eprintln!("[6] FAILED: {e} ({}ms)", t.elapsed().as_millis());
            }
        }
    }

    eprintln!("\n[DONE]");
}
