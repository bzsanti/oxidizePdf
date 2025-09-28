use oxidize_pdf::{Document, Font, Page};
use std::error::Error;
use std::path::Path;

fn main() -> Result<(), Box<dyn Error>> {
    println!("🧪 Testing CJK Font Support with Preview Compatibility");

    let font_path = "test-pdfs/SourceHanSansSC-Regular.otf";

    println!("📝 Creating new PDF document");
    let mut doc = Document::new();
    doc.set_title("CJK Preview Compatibility Test");

    // Test 1: Using embedded custom font
    if Path::new(font_path).exists() {
        println!("🔤 Loading custom CJK font: {}", font_path);
        if let Ok(font_data) = std::fs::read(font_path) {
            println!("✅ Font file loaded: {} bytes", font_data.len());

            if doc.add_font_from_bytes("SourceHanSC", font_data).is_ok() {
                println!("✅ Custom font added to document");

                let mut page = Page::a4();

                // Add title
                page.text()
                    .set_font(Font::Helvetica, 16.0)
                    .at(50.0, 750.0)
                    .write("CJK Font Compatibility Test for Preview.app")?;

                // Test embedded font
                page.text()
                    .set_font(Font::Custom("SourceHanSC".to_string()), 14.0)
                    .at(50.0, 700.0)
                    .write("Embedded SourceHanSC: 你好，世界！这是中文测试。")?;

                println!("✅ CJK text with embedded font written successfully");
                doc.add_page(page);
            }
        }
    }

    // Test 2: Create a page that references system fonts by name for Preview compatibility
    let mut page2 = Page::a4();

    page2
        .text()
        .set_font(Font::Helvetica, 16.0)
        .at(50.0, 750.0)
        .write("System Font Fallback Test")?;

    page2
        .text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 720.0)
        .write("If you can see the Chinese characters below correctly in Preview,")?;

    page2
        .text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 705.0)
        .write("then the system fonts are working properly:")?;

    // Add instructions for the user
    page2
        .text()
        .set_font(Font::Helvetica, 10.0)
        .at(50.0, 680.0)
        .write("Note: Preview.app should now use Noto Sans CJK SC for the Chinese text")?;

    page2
        .text()
        .set_font(Font::Helvetica, 10.0)
        .at(50.0, 665.0)
        .write("If characters appear as boxes [], you may need to restart Preview")?;

    // Test with basic characters that should work with system fallback
    page2
        .text()
        .set_font(Font::Helvetica, 14.0)
        .at(50.0, 630.0)
        .write("你好，世界！")?;

    page2
        .text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 605.0)
        .write("Japanese: こんにちは世界")?;

    page2
        .text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 585.0)
        .write("Korean: 안녕하세요 세계")?;

    doc.add_page(page2);

    // Save the PDF
    let output_path = "examples/results/test_cjk_preview_compatibility.pdf";
    match doc.save(output_path) {
        Ok(_) => {
            println!("✅ PDF saved to: {}", output_path);
            println!("📖 Open in Preview.app to test compatibility");
            println!("🔍 If you see Chinese characters correctly, the system fonts are working!");
        }
        Err(e) => println!("❌ Failed to save PDF: {}", e),
    }

    Ok(())
}
