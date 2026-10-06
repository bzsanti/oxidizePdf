//! Bounded, per-page flat text export for the independent content differential.
//! Usage: text_content_probe path.pdf
//! Errors are records, never successful empty text. The caller bounds runtime.
use oxidize_pdf::parser::{ParseOptions, PdfReader};
use oxidize_pdf::text::{ExtractionOptions, TextExtractor};
use serde_json::json;
use std::io::{self, Write};

const BYTE_BUDGET: usize = 32 * 1024 * 1024;

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args_os().nth(1).ok_or("expected PDF path")?;
    if std::fs::metadata(&path)?.len() > 128 * 1024 * 1024 {
        return Err("PDF exceeds 128 MiB input limit".into());
    }
    let reader = PdfReader::new_with_options(
        std::io::Cursor::new(std::fs::read(path)?),
        ParseOptions::lenient(),
    )?;
    let doc = reader.into_document();
    let pages = doc.page_count()?;
    if pages > 10_000 {
        return Err("PDF exceeds 10000 page limit".into());
    }
    let mut stdout = io::BufWriter::new(io::stdout().lock());
    writeln!(stdout, "{}", json!({"kind":"document", "pages":pages}))?;
    let mut extractor = TextExtractor::with_options(ExtractionOptions {
        max_extracted_bytes: Some(BYTE_BUDGET),
        ..Default::default()
    });
    let mut bytes = 0usize;
    for page in 0..pages {
        let record = match extractor.extract_from_page(&doc, page) {
            Ok(result) => {
                bytes += result.text.len();
                if result.truncated || bytes > BYTE_BUDGET {
                    return Err("extracted text reaches 32 MiB budget; no complete sample".into());
                }
                json!({"kind":"page", "page":page, "text":result.text})
            }
            Err(error) => json!({"kind":"page", "page":page, "error":error.to_string()}),
        };
        serde_json::to_writer(&mut stdout, &record)?;
        writeln!(stdout)?;
    }
    stdout.flush()?;
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
