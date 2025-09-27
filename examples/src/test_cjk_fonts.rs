use oxidize_pdf::{Document, Font, Page};
use std::error::Error;
use std::path::Path;

fn main() -> Result<(), Box<dyn Error>> {
    println!("🧪 Testing Custom CJK Font Support");

    let font_path = "test-pdfs/SourceHanSansSC-Regular.otf";

    if !Path::new(font_path).exists() {
        println!("❌ Font file not found: {}", font_path);
        println!("💡 Download it from: https://github.com/adobe-fonts/source-han-sans/raw/release/OTF/SimplifiedChinese/SourceHanSansSC-Regular.otf");
        return Ok(());
    }

    println!("📝 Creating new PDF document");
    let mut doc = Document::new();
    doc.set_title("Test CJK Fonts");

    println!("🔤 Loading custom CJK font: {}", font_path);
    match std::fs::read(font_path) {
        Ok(font_data) => {
            println!("✅ Font file loaded: {} bytes", font_data.len());

            match doc.add_font_from_bytes("HanSansSC", font_data) {
                Ok(_) => {
                    println!("✅ Font added to document successfully");

                    let mut page = Page::a4();

                    println!(
                        "💥 Attempting to write CJK text with custom font - this should panic!"
                    );

                    // This will either work (if fixed) or panic (current bug)
                    match page
                        .text()
                        .set_font(Font::Custom("HanSansSC".to_string()), 12.0)
                        .at(50.0, 700.0)
                        .write("你好，世界！")
                    {
                        Ok(_) => {
                            println!(
                                "✅ CJK text written successfully! (This is the fixed behavior)"
                            );

                            // Save the PDF
                            doc.add_page(page);
                            let output_path = "examples/results/test_cjk_fonts.pdf";
                            match doc.save(output_path) {
                                Ok(_) => println!("✅ PDF saved to: {}", output_path),
                                Err(e) => println!("❌ Failed to save PDF: {}", e),
                            }
                        }
                        Err(e) => {
                            println!("❌ Failed to write CJK text: {}", e);
                        }
                    }
                }
                Err(e) => {
                    println!("❌ Failed to add font to document: {}", e);
                }
            }
        }
        Err(e) => {
            println!("❌ Failed to read font file: {}", e);
        }
    }

    Ok(())
}
