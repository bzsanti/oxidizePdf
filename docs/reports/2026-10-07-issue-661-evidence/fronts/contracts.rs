use oxidize_pdf::{Document, Font, Page};
use oxidize_pdf::writer::WriterConfig;
use std::path::Path;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    let fixtures = Path::new(&args[1]);
    let output = Path::new(&args[2]);
    std::fs::create_dir_all(output)?;
    for (case, font_name, fixture, texts) in [
        ("custom", "Roboto", "Roboto-Regular.ttf", ["Alpha", "Omega"]),
        ("collision", "Helvetica", "Roboto-Regular.ttf", ["Alpha", "Omega"]),
        ("cjk", "CJK", "SourceHanSansSC-Regular.otf", ["中文", "测试"]),
    ] {
        let mut doc = Document::new();
        doc.add_font_from_bytes(font_name, std::fs::read(fixtures.join(fixture))?)?;
        for text in texts {
            let mut p = Page::a4();
            p.text().set_font(Font::Custom(font_name.into()), 12.0).at(50.0,700.0).write(text)?;
            doc.add_page(p);
        }
        for (mode, config) in [("legacy", WriterConfig::legacy()), ("modern", WriterConfig::modern())] {
            std::fs::write(output.join(format!("{case}-{mode}.pdf")),doc.to_bytes_with_config(config)?)?;
        }
    }
    let names = ["Helvetica", "Helvetica-Bold", "Helvetica-Oblique", "Helvetica-BoldOblique", "Times-Roman", "Times-Bold", "Times-Italic", "Times-BoldItalic", "Courier", "Courier-Bold", "Courier-Oblique", "Courier-BoldOblique"];
    let mut doc = Document::new();
    for _ in 0..2 {
        let mut p=Page::a4();
        for (i,name) in names.iter().enumerate() {
            p.graphics().add_command(&format!("BT /{name} 12 Tf 50 {} Td (Sample{i}) Tj ET",750-i*20));
        }
        doc.add_page(p);
    }
    for (mode, config) in [("legacy", WriterConfig::legacy()), ("modern", WriterConfig::modern())] {
        std::fs::write(output.join(format!("raw-{mode}.pdf")),doc.to_bytes_with_config(config)?)?;
    }
    Ok(())
}
