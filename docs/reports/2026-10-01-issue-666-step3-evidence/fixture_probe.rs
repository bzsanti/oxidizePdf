#[path="issue666_step3_support/embedded.rs"] mod embedded;
#[path="issue666_step3_support/document.rs"] mod document;
fn main() {
 for (name,bytes) in [("embedded",embedded::embedded_pdf(false)),("embedded-unicode",embedded::embedded_pdf(true)),("metadata-pdfdoc",document::metadata_pdf(&[0x80,0xA0])),("metadata-utf16",document::metadata_pdf(&[0xFE,0xFF,0xD8,0x3D,0xDE,0x00]))] {
 std::fs::write(format!("/tmp/issue666-step3-{name}.pdf"),bytes).unwrap();
 }
}