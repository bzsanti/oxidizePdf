#[path="issue666_step4_support/font.rs"] mod font;
#[path="issue666_step4_support/document.rs"] mod document;
fn main() {
 for subset in [false,true] { for unicode in [false,true] {
 std::fs::write(format!("/tmp/issue666-step4-ttf-{subset}-{unicode}.pdf"),font::truetype_pdf(subset,unicode,b"BT /F1 12 Tf 100 700 Td <41E942> Tj ET")).unwrap();
 }}
 std::fs::write("/tmp/issue666-step4-navigation.pdf",document::navigation_form_pdf(&[0x80,0xA0])).unwrap();
}