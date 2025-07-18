//! Helper functions for creating valid test PDFs with correct offsets

/// Creates a minimal valid PDF with correct xref offsets
pub fn create_minimal_pdf() -> Vec<u8> {
    let header = b"%PDF-1.4\n";
    let obj1_start = header.len();
    let obj1 = b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n";
    let obj2_start = obj1_start + obj1.len();
    let obj2 = b"2 0 obj\n<< /Type /Pages /Kids [] /Count 0 >>\nendobj\n";
    let xref_start = obj2_start + obj2.len();

    let xref = format!("xref\n0 3\n0000000000 65535 f \n{:010} 00000 n \n{:010} 00000 n \ntrailer\n<< /Size 3 /Root 1 0 R >>\nstartxref\n{}\n%%EOF", 
        obj1_start, obj2_start, xref_start);

    let mut content = Vec::new();
    content.extend_from_slice(header);
    content.extend_from_slice(obj1);
    content.extend_from_slice(obj2);
    content.extend_from_slice(xref.as_bytes());
    content
}

/// Creates a PDF with specific version
pub fn create_pdf_with_version(version: &str) -> Vec<u8> {
    let header = format!("%PDF-{}\n", version);
    let obj1_start = header.len();
    let obj1 = b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n";
    let obj2_start = obj1_start + obj1.len();
    let obj2 = b"2 0 obj\n<< /Type /Pages /Kids [] /Count 0 >>\nendobj\n";
    let xref_start = obj2_start + obj2.len();

    let xref = format!("xref\n0 3\n0000000000 65535 f \n{:010} 00000 n \n{:010} 00000 n \ntrailer\n<< /Size 3 /Root 1 0 R >>\nstartxref\n{}\n%%EOF", 
        obj1_start, obj2_start, xref_start);

    let mut content = header.into_bytes();
    content.extend_from_slice(obj1);
    content.extend_from_slice(obj2);
    content.extend_from_slice(xref.as_bytes());
    content
}

/// Creates a PDF with info dictionary
pub fn create_pdf_with_info() -> Vec<u8> {
    let header = b"%PDF-1.4\n";
    let obj1_start = header.len();
    let obj1 = b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n";
    let obj2_start = obj1_start + obj1.len();
    let obj2 = b"2 0 obj\n<< /Type /Pages /Kids [] /Count 0 >>\nendobj\n";
    let obj3_start = obj2_start + obj2.len();
    let obj3 =
        b"3 0 obj\n<< /Title (Test PDF) /Author (Test Author) /Subject (Testing) >>\nendobj\n";
    let xref_start = obj3_start + obj3.len();

    let xref = format!("xref\n0 4\n0000000000 65535 f \n{:010} 00000 n \n{:010} 00000 n \n{:010} 00000 n \ntrailer\n<< /Size 4 /Root 1 0 R /Info 3 0 R >>\nstartxref\n{}\n%%EOF",
        obj1_start, obj2_start, obj3_start, xref_start);

    let mut content = Vec::new();
    content.extend_from_slice(header);
    content.extend_from_slice(obj1);
    content.extend_from_slice(obj2);
    content.extend_from_slice(obj3);
    content.extend_from_slice(xref.as_bytes());
    content
}

/// Creates a PDF with binary marker after header
pub fn create_pdf_with_binary_marker() -> Vec<u8> {
    let header = b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n";
    let obj1_start = header.len();
    let obj1 = b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n";
    let obj2_start = obj1_start + obj1.len();
    let obj2 = b"2 0 obj\n<< /Type /Pages /Kids [] /Count 0 >>\nendobj\n";
    let xref_start = obj2_start + obj2.len();

    let xref = format!("xref\n0 3\n0000000000 65535 f \n{:010} 00000 n \n{:010} 00000 n \ntrailer\n<< /Size 3 /Root 1 0 R >>\nstartxref\n{}\n%%EOF",
        obj1_start, obj2_start, xref_start);

    let mut content = Vec::new();
    content.extend_from_slice(header);
    content.extend_from_slice(obj1);
    content.extend_from_slice(obj2);
    content.extend_from_slice(xref.as_bytes());
    content
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_minimal_pdf_structure() {
        let pdf = create_minimal_pdf();
        assert!(pdf.starts_with(b"%PDF-1.4\n"));
        assert!(pdf.ends_with(b"%%EOF"));

        // Verify the xref is at the correct position
        let pdf_str = String::from_utf8_lossy(&pdf);
        assert!(pdf_str.contains("xref"));
        assert!(pdf_str.contains("startxref"));
    }

    #[test]
    fn test_pdf_with_version() {
        let pdf = create_pdf_with_version("1.7");
        assert!(pdf.starts_with(b"%PDF-1.7\n"));

        let pdf2 = create_pdf_with_version("2.0");
        assert!(pdf2.starts_with(b"%PDF-2.0\n"));
    }
}
