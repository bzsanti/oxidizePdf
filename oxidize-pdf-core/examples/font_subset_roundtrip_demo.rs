//! Generate the three PDFs from the font-subset round-trip tests and save
//! them to /tmp so they can be opened in a viewer for visual inspection.
//!
//! Run with:
//!   cargo run --example font_subset_roundtrip_demo -p oxidize-pdf

use oxidize_pdf::{Document, Font, Page};
use std::path::Path;

const SOURCE_SANS_PATH: &str = "../test-pdfs/SourceSans3-Regular.otf";
const ROBOTO_PATH: &str = "../test-pdfs/Roboto-Regular.ttf";
const SOURCE_HAN_PATH: &str = "../test-pdfs/SourceHanSansSC-Regular.otf";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = Path::new("/tmp");

    // 1) Non-CID CFF (SourceSans3) with accented Latin.
    if let Ok(data) = std::fs::read(SOURCE_SANS_PATH) {
        let mut doc = Document::new();
        doc.add_font_from_bytes("SourceSans3", data)?;
        let mut page = Page::a4();
        page.text()
            .set_font(Font::Custom("SourceSans3".into()), 18.0)
            .at(60.0, 720.0)
            .write("Non-CID CFF roundtrip")?;
        page.text()
            .set_font(Font::Custom("SourceSans3".into()), 14.0)
            .at(60.0, 680.0)
            .write("café résumé naïve")?;
        page.text()
            .set_font(Font::Custom("SourceSans3".into()), 12.0)
            .at(60.0, 640.0)
            .write("The quick brown fox jumps over the lazy dog.")?;
        doc.add_page(page);
        let bytes = doc.to_bytes()?;
        let path = out_dir.join("font_subset_demo_sourcesans.pdf");
        std::fs::write(&path, &bytes)?;
        println!("wrote {} ({} bytes)", path.display(), bytes.len());
    } else {
        eprintln!("SKIPPED SourceSans3: {} not found", SOURCE_SANS_PATH);
    }

    // 2) TTF (Roboto).
    if let Ok(data) = std::fs::read(ROBOTO_PATH) {
        let mut doc = Document::new();
        doc.add_font_from_bytes("Roboto", data)?;
        let mut page = Page::a4();
        page.text()
            .set_font(Font::Custom("Roboto".into()), 18.0)
            .at(60.0, 720.0)
            .write("TTF roundtrip")?;
        page.text()
            .set_font(Font::Custom("Roboto".into()), 14.0)
            .at(60.0, 680.0)
            .write("The quick brown fox jumps over the lazy dog.")?;
        page.text()
            .set_font(Font::Custom("Roboto".into()), 12.0)
            .at(60.0, 640.0)
            .write("Subset has no cmap/OS/2/name; PDF resolves via CIDToGIDMap.")?;
        doc.add_page(page);
        let bytes = doc.to_bytes()?;
        let path = out_dir.join("font_subset_demo_roboto.pdf");
        std::fs::write(&path, &bytes)?;
        println!("wrote {} ({} bytes)", path.display(), bytes.len());
    } else {
        eprintln!("SKIPPED Roboto: {} not found", ROBOTO_PATH);
    }

    // 3) CID-keyed CFF (SourceHanSansSC, 16 MB, ~65K glyphs) with CJK.
    //    This is the headline case from Issue #165.
    if let Ok(data) = std::fs::read(SOURCE_HAN_PATH) {
        let mut doc = Document::new();
        doc.add_font_from_bytes("SourceHanSC", data)?;
        let mut page = Page::a4();
        page.text()
            .set_font(Font::Custom("SourceHanSC".into()), 18.0)
            .at(60.0, 760.0)
            .write("CID CFF roundtrip (Issue #165)")?;
        page.text()
            .set_font(Font::Custom("SourceHanSC".into()), 14.0)
            .at(60.0, 720.0)
            .write("你好世界")?;
        page.text()
            .set_font(Font::Custom("SourceHanSC".into()), 12.0)
            .at(60.0, 690.0)
            .write("Rust 擁有完整的技術文件、友善的編譯器與清晰的錯誤訊息")?;
        page.text()
            .set_font(Font::Custom("SourceHanSC".into()), 12.0)
            .at(60.0, 665.0)
            .write("日本語テキスト / 한글 텍스트")?;
        doc.add_page(page);
        let bytes = doc.to_bytes()?;
        let path = out_dir.join("font_subset_demo_cjk.pdf");
        std::fs::write(&path, &bytes)?;
        println!("wrote {} ({} bytes)", path.display(), bytes.len());
    } else {
        eprintln!("SKIPPED SourceHanSansSC: {} not found", SOURCE_HAN_PATH);
    }

    // 4) Mixed CFF + TTF on one page.
    let cff = std::fs::read(SOURCE_SANS_PATH).ok();
    let ttf = std::fs::read(ROBOTO_PATH).ok();
    if let (Some(cff), Some(ttf)) = (cff, ttf) {
        let mut doc = Document::new();
        doc.add_font_from_bytes("SourceSans3", cff)?;
        doc.add_font_from_bytes("Roboto", ttf)?;
        let mut page = Page::a4();
        page.text()
            .set_font(Font::Custom("SourceSans3".into()), 18.0)
            .at(60.0, 720.0)
            .write("Mixed CFF + TTF on one page")?;
        page.text()
            .set_font(Font::Custom("SourceSans3".into()), 14.0)
            .at(60.0, 680.0)
            .write("SourceSans3 (CFF): café résumé naïve")?;
        page.text()
            .set_font(Font::Custom("Roboto".into()), 14.0)
            .at(60.0, 650.0)
            .write("Roboto (TTF): The quick brown fox")?;
        doc.add_page(page);
        let bytes = doc.to_bytes()?;
        let path = out_dir.join("font_subset_demo_mixed.pdf");
        std::fs::write(&path, &bytes)?;
        println!("wrote {} ({} bytes)", path.display(), bytes.len());
    }

    Ok(())
}
