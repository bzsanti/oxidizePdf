/// Demo: CFF Local Subr Subsetting — visual verification
///
/// Generates a PDF with CJK text using a CID-keyed CFF font (Source Han Sans SC).
/// The font is 16MB; after Local Subr subsetting, the output PDF should be <200KB.
///
/// Usage:
///   cargo run --example cff_subr_subsetting_demo
///
/// Opens: /tmp/cff_subr_subsetting_demo.pdf
use oxidize_pdf::{Document, Font, Page};

fn main() {
    let font_path = "../test-pdfs/SourceHanSansSC-Regular.otf";
    let font_data = match std::fs::read(font_path) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Cannot read font at {font_path}: {e}");
            std::process::exit(1);
        }
    };
    let original_size = font_data.len();

    let mut doc = Document::new();
    doc.add_font_from_bytes("SourceHanSC", font_data)
        .expect("Font loading failed");

    let font = Font::Custom("SourceHanSC".to_string());

    let mut page = Page::a4();

    // Title
    page.text()
        .set_font(font.clone(), 18.0)
        .at(30.0, 780.0)
        .write("CFF Local Subr Subsetting — Demo")
        .expect("title");

    // Separator
    page.text()
        .set_font(font.clone(), 10.0)
        .at(30.0, 755.0)
        .write("────────────────────────────────────────────────────")
        .expect("separator");

    // Chinese (Simplified)
    page.text()
        .set_font(font.clone(), 12.0)
        .at(30.0, 720.0)
        .write("简体中文：Rust 拥有完整的技术文件、友善的编译器与清晰的错误讯息。")
        .expect("zh-CN");

    // Chinese (Traditional) — from issue #165
    page.text()
        .set_font(font.clone(), 12.0)
        .at(30.0, 695.0)
        .write("繁體中文：Rust 擁有完整的技術文件、友善的編譯器與清晰的錯誤訊息，")
        .expect("zh-TW line 1");

    page.text()
        .set_font(font.clone(), 12.0)
        .at(30.0, 675.0)
        .write("還整合了一流的工具 — 包含套件管理工具、建構工具、")
        .expect("zh-TW line 2");

    page.text()
        .set_font(font.clone(), 12.0)
        .at(30.0, 655.0)
        .write("支援多種編輯器的自動補齊、型別檢測、自動格式化程式碼，以及更多等等。")
        .expect("zh-TW line 3");

    // Mixed CJK + Latin
    page.text()
        .set_font(font.clone(), 12.0)
        .at(30.0, 620.0)
        .write("混合文本：Hello 你好世界 — oxidize-pdf v2.5.0")
        .expect("mixed");

    // Numbers and punctuation
    page.text()
        .set_font(font.clone(), 12.0)
        .at(30.0, 595.0)
        .write("数字标点：2026年4月13日 — 测试通过！（100%）")
        .expect("numbers");

    // Short text (few unique glyphs)
    page.text()
        .set_font(font.clone(), 14.0)
        .at(30.0, 555.0)
        .write("你好世界")
        .expect("hello world");

    doc.add_page(page);

    let pdf_bytes = doc.to_bytes().expect("PDF generation failed");
    let output_path = "/tmp/cff_subr_subsetting_demo.pdf";
    std::fs::write(output_path, &pdf_bytes).expect("Failed to write PDF");

    let pdf_kb = pdf_bytes.len() as f64 / 1024.0;
    let original_mb = original_size as f64 / (1024.0 * 1024.0);
    let reduction = (1.0 - pdf_bytes.len() as f64 / original_size as f64) * 100.0;

    println!("=== CFF Local Subr Subsetting Demo ===");
    println!("Font original:  {original_mb:.1} MB ({original_size} bytes)");
    println!("PDF output:     {pdf_kb:.1} KB ({} bytes)", pdf_bytes.len());
    println!("Size reduction: {reduction:.1}%");
    println!();
    println!("Output: {output_path}");
    println!("Open it in a PDF viewer to verify CJK text renders correctly.");
}
