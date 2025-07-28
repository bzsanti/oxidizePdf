use oxidize_pdf::parser::{PdfDocument, PdfReader};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing issue #20 - Invalid element in dash array");
    
    // Open and parse the PDF
    let reader = PdfReader::open("test_issue_20.pdf")?;
    let document = PdfDocument::new(reader);
    
    // Get document information
    println!("Pages: {}", document.page_count()?);
    println!("Version: {}", document.version()?);
    
    // Try to extract text
    match document.extract_text() {
        Ok(text_pages) => {
            println!("Text extraction successful!");
            for (i, page_text) in text_pages.iter().enumerate() {
                println!("Page {} has {} characters", i + 1, page_text.text.len());
            }
        }
        Err(e) => {
            println!("Error extracting text: {:?}", e);
        }
    }
    
    Ok(())
}