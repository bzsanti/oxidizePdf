//! Debug example to test tesseract compilation

#[cfg(feature = "ocr-tesseract")]
fn main() {
    println!("Testing rusty-tesseract dependency...");

    // Try to use rusty-tesseract
    use rusty_tesseract::{Args, Image};

    println!("Creating test args...");
    let args = Args::default();
    println!("Args created: {:?}", args);

    println!("rusty-tesseract dependency works!");
}

#[cfg(not(feature = "ocr-tesseract"))]
fn main() {
    println!("OCR feature not enabled. Use --features ocr-tesseract");
}
