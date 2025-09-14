use std::path::Path;
use std::fs::File;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 TESTING OCR WITH REAL O&M PDFs");
    println!("=================================");
    
    let pdfs = [
        "/Users/santifdezmunoz/Downloads/ocr/FIS2 160930 O&M Agreement ESS.pdf",
        "/Users/santifdezmunoz/Downloads/ocr/MADRIDEJOS_O&M CONTRACT_2013.pdf",
    ];

    for pdf_path in &pdfs {
        let path = Path::new(pdf_path);
        if !path.exists() {
            println!("❌ File not found: {}", pdf_path);
            continue;
        }

        println!("\n📄 Testing: {}", path.file_name().unwrap().to_string_lossy());
        println!("   📁 Path: {}", pdf_path);
        
        match File::open(path) {
            Ok(file) => {
                let size = file.metadata()?.len();
                println!("   📊 Size: {:.1}MB", size as f64 / 1_048_576.0);
                println!("   ✅ File accessible");
            }
            Err(e) => {
                println!("   ❌ Cannot access file: {}", e);
            }
        }
    }

    println!("\n📋 Files validated - ready for OCR testing");
    Ok(())
}