//! Reproduces the snippet in Issue #165 to verify end-to-end size impact of
//! the post v3.0 + CIDToGIDMap FlateDecode fixes.
//!
//! Runs two variants:
//! - OTF (CFF) with SourceHanSansSC-Regular.otf, same text as the user's case.
//! - TTF (glyf) with Roboto-Regular.ttf as a proxy (we don't ship a CJK TTF).
//!
//! Usage:
//!   cargo run --example issue_165_repro --features compression
//!
//! This is a **diagnostic script**, not a test — it prints sizes but makes no
//! assertions. Size regressions are guarded by the integration tests in
//! `tests/font_subset_post_and_cidtogidmap_test.rs`.

use oxidize_pdf::{Document, Font, Page};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ----- OTF / CFF path -----
    let otf_path = "../test-pdfs/SourceHanSansSC-Regular.otf";
    if let Ok(font_data) = std::fs::read(otf_path) {
        let original_len = font_data.len();
        let mut doc = Document::new();
        doc.add_font_from_bytes("SourceHanSans", font_data)?;
        let mut page = Page::a4();
        page.text()
            .set_font(Font::Custom("SourceHanSans".to_string()), 10.5)
            .at(30.0, 535.0)
            .write("Rust 擁有完整的技術文件、友善的編譯器與清晰的錯誤訊息，還整合了一流的工具 — 包含套件管理工具、")?;
        page.text()
            .set_font(Font::Custom("SourceHanSans".to_string()), 10.5)
            .at(30.0, 515.0)
            .write(
                "建構工具、支援多種編輯器的自動補齊、型別檢測、自動格式化程式碼，以及更多等等。",
            )?;
        doc.add_page(page);
        let bytes = doc.to_bytes()?;
        std::fs::write("/tmp/issue165_repro_otf.pdf", &bytes)?;
        println!(
            "OTF/CFF: original={} KB, PDF={} KB (v2.5.3 was 62 KB, krilla 59 KB)",
            original_len / 1024,
            bytes.len() / 1024
        );
    } else {
        println!("SKIPPED OTF: {} not found", otf_path);
    }

    // ----- TTF / glyf path: user's exact TTF font if available -----
    // SourceHanSansTC-Regular.ttf is not shipped in the repo (16+ MB). The
    // fixture is kept at /tmp/issue165_v253/SourceHanSansTC-Regular.ttf for
    // local reproduction of the user's case. If absent, fall back to Roboto
    // as a Latin-TTF smoke test.
    let cjk_ttf_path = "/tmp/issue165_v253/SourceHanSansTC-Regular.ttf";
    let user_text_line1 = "Rust 擁有完整的技術文件、友善的編譯器與清晰的錯誤訊息，還整合了一流的工具 — 包含套件管理工具、";
    let user_text_line2 =
        "建構工具、支援多種編輯器的自動補齊、型別檢測、自動格式化程式碼，以及更多等等。";

    if let Ok(font_data) = std::fs::read(cjk_ttf_path) {
        let original_len = font_data.len();
        let mut doc = Document::new();
        doc.add_font_from_bytes("SourceHanSansTC", font_data)?;
        let mut page = Page::a4();
        page.text()
            .set_font(Font::Custom("SourceHanSansTC".to_string()), 10.5)
            .at(30.0, 535.0)
            .write(user_text_line1)?;
        page.text()
            .set_font(Font::Custom("SourceHanSansTC".to_string()), 10.5)
            .at(30.0, 515.0)
            .write(user_text_line2)?;
        doc.add_page(page);
        let bytes = doc.to_bytes()?;
        std::fs::write("/tmp/issue165_repro_cjk_ttf.pdf", &bytes)?;
        println!(
            "TTF/glyf CJK: original={} KB, PDF={} KB (user reported 307 KB on v2.5.3, krilla 48 KB)",
            original_len / 1024,
            bytes.len() / 1024
        );
    } else {
        println!("SKIPPED CJK TTF: {} not found", cjk_ttf_path);
    }

    let ttf_path = "../test-pdfs/Roboto-Regular.ttf";
    if let Ok(font_data) = std::fs::read(ttf_path) {
        let original_len = font_data.len();
        let mut doc = Document::new();
        doc.add_font_from_bytes("Roboto", font_data)?;
        let mut page = Page::a4();
        page.text()
            .set_font(Font::Custom("Roboto".to_string()), 10.5)
            .at(30.0, 535.0)
            .write("Rust has complete documentation, a friendly compiler and clear error messages, plus top-tier tools.")?;
        page.text()
            .set_font(Font::Custom("Roboto".to_string()), 10.5)
            .at(30.0, 515.0)
            .write("Build tools, multi-editor autocomplete, type detection, automatic formatting, and much more.")?;
        doc.add_page(page);
        let bytes = doc.to_bytes()?;
        std::fs::write("/tmp/issue165_repro_ttf.pdf", &bytes)?;
        println!(
            "TTF/glyf Latin: original={} KB, PDF={} KB (Roboto proxy)",
            original_len / 1024,
            bytes.len() / 1024
        );
    } else {
        println!("SKIPPED Latin TTF: {} not found", ttf_path);
    }

    Ok(())
}
