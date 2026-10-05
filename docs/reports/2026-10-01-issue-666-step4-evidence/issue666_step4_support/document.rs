#[path="../../tests/common/pdf_assembler.rs"] mod assembler;
use assembler::assemble_pdf;
fn hex(bytes:&[u8])->String {bytes.iter().map(|b|format!("{b:02X}")).collect()}
pub fn navigation_form_pdf(bytes: &[u8]) -> Vec<u8> {
    assemble_pdf(&[
        b"<< /Type /Catalog /Pages 2 0 R /Outlines 4 0 R /AcroForm 6 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << >> >>".to_vec(),
        b"<< /Type /Outlines /First 5 0 R /Last 5 0 R /Count 1 >>".to_vec(),
        format!("<< /Title <{}> /Parent 4 0 R /Dest [3 0 R /Fit] >>",hex(bytes)).into_bytes(),
        b"<< /Fields [7 0 R] /DA (/Helv 12 Tf 0 g) /DR << /Font << /Helv << /Type /Font /Subtype /Type1 /BaseFont /Helvetica >> >> >> >>".to_vec(),
        format!("<< /FT /Tx /T (contract) /V <{}> >>",hex(bytes)).into_bytes(),
    ])
}
