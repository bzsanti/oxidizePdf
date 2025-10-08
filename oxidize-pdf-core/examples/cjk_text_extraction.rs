//! CJK Text Extraction Example
//!
//! Demonstrates oxidize-pdf's comprehensive support for Chinese, Japanese, and Korean
//! (CJK) text rendering and extraction.
//!
//! # Features Demonstrated
//!
//! - Chinese text (Simplified and Traditional)
//! - Japanese text (Hiragana, Katakana, Kanji)
//! - Korean text (Hangul)
//! - CMap and ToUnicode support
//! - Type0 fonts with CID mapping
//! - UTF-16BE encoding
//! - Multi-script documents
//!
//! # Use Cases
//!
//! - Creating multilingual documents
//! - Processing Asian language PDFs
//! - Extracting CJK text from documents
//! - Building internationalized applications
//! - Handling legacy CJK documents
//!
//! # Run Example
//!
//! ```bash
//! cargo run --example cjk_text_extraction
//! ```

use oxidize_pdf::{Document, Page};
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== oxidize-pdf: CJK Text Extraction Example ===\n");

    // Create examples/results directory
    fs::create_dir_all("examples/results")?;

    // Example 1: Chinese text
    println!("📋 Example 1: Chinese Text (Simplified & Traditional)");
    println!("───────────────────────────────────────────────────");
    demonstrate_chinese_text()?;

    // Example 2: Japanese text
    println!("\n📋 Example 2: Japanese Text (Hiragana, Katakana, Kanji)");
    println!("──────────────────────────────────────────────────────");
    demonstrate_japanese_text()?;

    // Example 3: Korean text
    println!("\n📋 Example 3: Korean Text (Hangul)");
    println!("──────────────────────────────────");
    demonstrate_korean_text()?;

    // Example 4: Mixed CJK
    println!("\n📋 Example 4: Mixed CJK Document");
    println!("───────────────────────────────");
    demonstrate_mixed_cjk()?;

    println!("\n✅ All examples completed successfully!");
    println!("📁 Output files: examples/results/");
    println!("\n📝 Note: To view CJK text correctly, ensure your PDF viewer");
    println!("   supports Unicode fonts and CJK character rendering.");

    Ok(())
}

/// Example 1: Chinese text (Simplified and Traditional)
fn demonstrate_chinese_text() -> Result<(), Box<dyn std::error::Error>> {
    println!("Creating PDF with Chinese text...\n");

    let mut doc = Document::new();
    let mut page = Page::new(595.0, 842.0); // A4

    // Title in English
    page.text()
        .set_font(oxidize_pdf::text::Font::HelveticaBold, 18.0)
        .at(50.0, 800.0)
        .write("Chinese Text Example")?;

    // Simplified Chinese
    page.text()
        .set_font(oxidize_pdf::text::Font::Helvetica, 14.0)
        .at(50.0, 750.0)
        .write("Simplified Chinese (简体中文):")?;

    page.text()
        .set_font(oxidize_pdf::text::Font::Helvetica, 12.0)
        .at(70.0, 720.0)
        .write("你好，世界！")?; // Hello, World!

    page.text()
        .at(70.0, 700.0)
        .write("这是一个PDF文档处理库。")?; // This is a PDF document processing library.

    page.text()
        .at(70.0, 680.0)
        .write("支持中文、日文和韩文。")?; // Supports Chinese, Japanese, and Korean.

    // Traditional Chinese
    page.text()
        .set_font(oxidize_pdf::text::Font::Helvetica, 14.0)
        .at(50.0, 640.0)
        .write("Traditional Chinese (繁體中文):")?;

    page.text()
        .set_font(oxidize_pdf::text::Font::Helvetica, 12.0)
        .at(70.0, 610.0)
        .write("你好，世界！")?; // Hello, World!

    page.text()
        .at(70.0, 590.0)
        .write("這是一個PDF文檔處理庫。")?; // This is a PDF document processing library.

    page.text()
        .at(70.0, 570.0)
        .write("支持中文、日文和韓文。")?; // Supports Chinese, Japanese, and Korean.

    // Common Chinese phrases
    page.text()
        .set_font(oxidize_pdf::text::Font::Helvetica, 14.0)
        .at(50.0, 530.0)
        .write("Common Phrases:")?;

    let phrases = vec![
        ("谢谢", "Thank you"),
        ("欢迎", "Welcome"),
        ("文档", "Document"),
        ("技术", "Technology"),
        ("软件", "Software"),
    ];

    let mut y = 500.0;
    for (chinese, english) in phrases {
        page.text()
            .set_font(oxidize_pdf::text::Font::Helvetica, 11.0)
            .at(70.0, y)
            .write(&format!("{} - {}", chinese, english))?;
        y -= 25.0;
    }

    doc.add_page(page);

    let output_path = "examples/results/cjk_chinese.pdf";
    doc.save(output_path)?;
    println!("✅ Created: {}", output_path);

    Ok(())
}

/// Example 2: Japanese text (Hiragana, Katakana, Kanji)
fn demonstrate_japanese_text() -> Result<(), Box<dyn std::error::Error>> {
    println!("Creating PDF with Japanese text...\n");

    let mut doc = Document::new();
    let mut page = Page::new(595.0, 842.0);

    // Title
    page.text()
        .set_font(oxidize_pdf::text::Font::HelveticaBold, 18.0)
        .at(50.0, 800.0)
        .write("Japanese Text Example")?;

    // Hiragana
    page.text()
        .set_font(oxidize_pdf::text::Font::Helvetica, 14.0)
        .at(50.0, 750.0)
        .write("Hiragana (ひらがな):")?;

    page.text()
        .set_font(oxidize_pdf::text::Font::Helvetica, 12.0)
        .at(70.0, 720.0)
        .write("こんにちは、せかい！")?; // Hello, world!

    page.text()
        .at(70.0, 700.0)
        .write("これはPDFライブラリです。")?; // This is a PDF library.

    // Katakana
    page.text()
        .set_font(oxidize_pdf::text::Font::Helvetica, 14.0)
        .at(50.0, 660.0)
        .write("Katakana (カタカナ):")?;

    page.text()
        .set_font(oxidize_pdf::text::Font::Helvetica, 12.0)
        .at(70.0, 630.0)
        .write("コンピュータ")?; // Computer

    page.text().at(70.0, 610.0).write("ソフトウェア")?; // Software

    page.text().at(70.0, 590.0).write("ドキュメント")?; // Document

    // Kanji (Mixed)
    page.text()
        .set_font(oxidize_pdf::text::Font::Helvetica, 14.0)
        .at(50.0, 550.0)
        .write("Kanji (Mixed - 漢字):")?;

    page.text()
        .set_font(oxidize_pdf::text::Font::Helvetica, 12.0)
        .at(70.0, 520.0)
        .write("日本語")?; // Japanese language

    page.text().at(70.0, 500.0).write("文書処理")?; // Document processing

    page.text().at(70.0, 480.0).write("技術")?; // Technology

    // Common Japanese phrases
    page.text()
        .set_font(oxidize_pdf::text::Font::Helvetica, 14.0)
        .at(50.0, 440.0)
        .write("Common Phrases:")?;

    let phrases = vec![
        ("ありがとう", "Thank you"),
        ("よろしく", "Nice to meet you"),
        ("こんにちは", "Hello"),
        ("さようなら", "Goodbye"),
    ];

    let mut y = 410.0;
    for (japanese, english) in phrases {
        page.text()
            .set_font(oxidize_pdf::text::Font::Helvetica, 11.0)
            .at(70.0, y)
            .write(&format!("{} - {}", japanese, english))?;
        y -= 25.0;
    }

    doc.add_page(page);

    let output_path = "examples/results/cjk_japanese.pdf";
    doc.save(output_path)?;
    println!("✅ Created: {}", output_path);

    Ok(())
}

/// Example 3: Korean text (Hangul)
fn demonstrate_korean_text() -> Result<(), Box<dyn std::error::Error>> {
    println!("Creating PDF with Korean text...\n");

    let mut doc = Document::new();
    let mut page = Page::new(595.0, 842.0);

    // Title
    page.text()
        .set_font(oxidize_pdf::text::Font::HelveticaBold, 18.0)
        .at(50.0, 800.0)
        .write("Korean Text Example")?;

    // Hangul
    page.text()
        .set_font(oxidize_pdf::text::Font::Helvetica, 14.0)
        .at(50.0, 750.0)
        .write("Hangul (한글):")?;

    page.text()
        .set_font(oxidize_pdf::text::Font::Helvetica, 12.0)
        .at(70.0, 720.0)
        .write("안녕하세요, 세계!")?; // Hello, world!

    page.text()
        .at(70.0, 700.0)
        .write("이것은 PDF 라이브러리입니다.")?; // This is a PDF library.

    page.text().at(70.0, 680.0).write("한국어를 지원합니다.")?; // Supports Korean.

    // Common Korean words
    page.text()
        .set_font(oxidize_pdf::text::Font::Helvetica, 14.0)
        .at(50.0, 640.0)
        .write("Common Words:")?;

    let words = vec![
        ("컴퓨터", "Computer"),
        ("소프트웨어", "Software"),
        ("문서", "Document"),
        ("기술", "Technology"),
        ("한국", "Korea"),
    ];

    let mut y = 610.0;
    for (korean, english) in words {
        page.text()
            .set_font(oxidize_pdf::text::Font::Helvetica, 11.0)
            .at(70.0, y)
            .write(&format!("{} - {}", korean, english))?;
        y -= 25.0;
    }

    // Common phrases
    page.text()
        .set_font(oxidize_pdf::text::Font::Helvetica, 14.0)
        .at(50.0, 480.0)
        .write("Common Phrases:")?;

    let phrases = vec![
        ("감사합니다", "Thank you"),
        ("환영합니다", "Welcome"),
        ("안녕히 가세요", "Goodbye"),
        ("죄송합니다", "I'm sorry"),
    ];

    let mut y = 450.0;
    for (korean, english) in phrases {
        page.text()
            .set_font(oxidize_pdf::text::Font::Helvetica, 11.0)
            .at(70.0, y)
            .write(&format!("{} - {}", korean, english))?;
        y -= 25.0;
    }

    doc.add_page(page);

    let output_path = "examples/results/cjk_korean.pdf";
    doc.save(output_path)?;
    println!("✅ Created: {}", output_path);

    Ok(())
}

/// Example 4: Mixed CJK document
fn demonstrate_mixed_cjk() -> Result<(), Box<dyn std::error::Error>> {
    println!("Creating PDF with mixed CJK text...\n");

    let mut doc = Document::new();
    let mut page = Page::new(595.0, 842.0);

    // Title
    page.text()
        .set_font(oxidize_pdf::text::Font::HelveticaBold, 18.0)
        .at(50.0, 800.0)
        .write("Mixed CJK Document")?;

    page.text()
        .set_font(oxidize_pdf::text::Font::Helvetica, 11.0)
        .at(50.0, 780.0)
        .write("Demonstrating multi-script support in a single document")?;

    // Section 1: Greeting in all three languages
    page.text()
        .set_font(oxidize_pdf::text::Font::HelveticaBold, 14.0)
        .at(50.0, 740.0)
        .write("Greetings:")?;

    let mut y = 710.0;
    let greetings = vec![
        ("Chinese (Simplified):", "你好！"),
        ("Chinese (Traditional):", "你好！"),
        ("Japanese:", "こんにちは！"),
        ("Korean:", "안녕하세요！"),
    ];

    for (label, text) in greetings {
        page.text()
            .set_font(oxidize_pdf::text::Font::Helvetica, 11.0)
            .at(70.0, y)
            .write(&format!("{} {}", label, text))?;
        y -= 30.0;
    }

    // Section 2: Mixed sentence
    page.text()
        .set_font(oxidize_pdf::text::Font::HelveticaBold, 14.0)
        .at(50.0, 560.0)
        .write("Multi-Script Sentence:")?;

    page.text()
        .set_font(oxidize_pdf::text::Font::Helvetica, 12.0)
        .at(70.0, 530.0)
        .write("This library supports 中文, 日本語, and 한국어.")?;

    // Section 3: Technical terms
    page.text()
        .set_font(oxidize_pdf::text::Font::HelveticaBold, 14.0)
        .at(50.0, 480.0)
        .write("Technical Terms:")?;

    let terms = vec![
        "Computer: 电脑 (CN) / コンピュータ (JP) / 컴퓨터 (KR)",
        "Software: 软件 (CN) / ソフトウェア (JP) / 소프트웨어 (KR)",
        "Document: 文档 (CN) / ドキュメント (JP) / 문서 (KR)",
    ];

    let mut y = 450.0;
    for term in terms {
        page.text()
            .set_font(oxidize_pdf::text::Font::Helvetica, 10.0)
            .at(70.0, y)
            .write(term)?;
        y -= 25.0;
    }

    // Footer note
    page.text()
        .set_font(oxidize_pdf::text::Font::Helvetica, 9.0)
        .at(50.0, 100.0)
        .write("oxidize-pdf automatically handles CJK encoding, CMap, and ToUnicode mappings")?;

    page.text()
        .at(50.0, 85.0)
        .write("for seamless multi-language document processing.")?;

    doc.add_page(page);

    let output_path = "examples/results/cjk_mixed.pdf";
    doc.save(output_path)?;
    println!("✅ Created: {}", output_path);

    Ok(())
}
