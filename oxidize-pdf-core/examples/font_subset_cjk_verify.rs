//! Character-by-character verification of the CJK roundtrip: generates a
//! PDF using SourceHanSansSC, extracts the text back via the parser, and
//! reports any character that went in but did not come out.
//!
//! Run with:
//!   cargo run --example font_subset_cjk_verify -p oxidize-pdf

use oxidize_pdf::parser::{PdfDocument, PdfReader};
use oxidize_pdf::{Document, Font, Page};
use std::collections::BTreeSet;
use std::io::Cursor;

const SOURCE_HAN_PATH: &str = "../test-pdfs/SourceHanSansSC-Regular.otf";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let font_data = match std::fs::read(SOURCE_HAN_PATH) {
        Ok(d) => d,
        Err(_) => {
            eprintln!("SKIPPED: {} not found", SOURCE_HAN_PATH);
            return Ok(());
        }
    };

    // Issue #165 full user text plus Japanese + Korean greetings.
    let lines = [
        "你好世界",
        "Rust 擁有完整的技術文件、友善的編譯器與清晰的錯誤訊息，還整合了一流的工具",
        "建構工具、支援多種編輯器的自動補齊、型別檢測、自動格式化程式碼",
        "日本語テキスト",
        "한글 텍스트",
    ];

    let mut doc = Document::new();
    doc.add_font_from_bytes("SourceHanSC", font_data)?;
    let mut page = Page::a4();
    let mut y = 760.0;
    for line in &lines {
        page.text()
            .set_font(Font::Custom("SourceHanSC".into()), 12.0)
            .at(60.0, y)
            .write(line)?;
        y -= 25.0;
    }
    doc.add_page(page);
    let pdf_bytes = doc.to_bytes()?;

    // Save for visual inspection
    let out_path = "/tmp/font_subset_cjk_verify.pdf";
    std::fs::write(out_path, &pdf_bytes)?;
    println!("Generated {} ({} bytes)", out_path, pdf_bytes.len());

    // Parse back and extract
    let reader = PdfReader::new(Cursor::new(&pdf_bytes))?;
    let parsed = PdfDocument::new(reader);
    let extracted = parsed.extract_text_from_page(0)?;

    // Compare: every non-whitespace char in the input must appear in the output.
    let input_chars: BTreeSet<char> = lines
        .iter()
        .flat_map(|s| s.chars())
        .filter(|c| !c.is_whitespace())
        .collect();
    let output_chars: BTreeSet<char> = extracted
        .text
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();

    let missing: Vec<char> = input_chars.difference(&output_chars).copied().collect();
    let added: Vec<char> = output_chars.difference(&input_chars).copied().collect();

    println!("\nInput unique chars : {}", input_chars.len());
    println!("Output unique chars: {}", output_chars.len());
    println!("Missing from output: {} chars", missing.len());
    println!("Extra in output   : {} chars", added.len());

    if !missing.is_empty() {
        println!("\n❌ MISSING characters (went in, did not come out):");
        for ch in &missing {
            println!("   U+{:04X}  '{}'", *ch as u32, ch);
        }
    }
    if !added.is_empty() {
        println!("\n⚠️  EXTRA characters (came out, were not in input — whitespace artifacts?):");
        for ch in &added {
            println!("   U+{:04X}  '{}'", *ch as u32, ch);
        }
    }

    if missing.is_empty() {
        println!("\n✅ Every input character round-tripped successfully.");
        println!("\n--- INPUT (lines separated by ¶) ---");
        for line in &lines {
            println!("¶ {}", line);
        }
        println!("\n--- EXTRACTED (raw) ---");
        println!("{}", extracted.text);
    } else {
        std::process::exit(1);
    }

    Ok(())
}
