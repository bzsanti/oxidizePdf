//! Minimal probe: parse a single PDF and extract text.
//! Used to identify which PDFs cause hangs/OOM.
//! Usage: cargo run --release --example corpus_probe -- <path.pdf> [--parse-only]

use oxidize_pdf::parser::{PdfDocument, PdfReader};
use std::env;
use std::time::Instant;

fn main() {
    let args: Vec<String> = env::args().collect();
    let path = args
        .get(1)
        .expect("Usage: corpus_probe <path.pdf> [--parse-only]");
    let parse_only = args.iter().any(|a| a == "--parse-only");

    eprintln!("STEP: opening {path}");
    let start = Instant::now();
    let reader = match PdfReader::open(path) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("PARSE_ERROR: {e}");
            std::process::exit(1);
        }
    };
    let parse_ms = start.elapsed().as_millis();
    eprintln!("STEP: parsed in {parse_ms}ms");

    let doc = PdfDocument::new(reader);
    let pages = doc.page_count().unwrap_or(0);
    eprintln!("STEP: page_count={pages}");

    if parse_only {
        println!("OK pages={pages} parse={parse_ms}ms (parse-only)");
        return;
    }

    eprintln!("STEP: extracting text...");
    let ext_start = Instant::now();
    let text = doc.extract_text();
    let ext_ms = ext_start.elapsed().as_millis();

    let text_len: usize = text
        .as_ref()
        .map(|pages| pages.iter().map(|p| p.text.len()).sum())
        .unwrap_or(0);
    eprintln!("STEP: done text_len={text_len}");

    println!("OK pages={pages} parse={parse_ms}ms extract={ext_ms}ms text_len={text_len}",);
}
