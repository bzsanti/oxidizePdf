#[path="issue666_step7_support/font.rs"] mod font;
fn main() {
 for (name,remap,vertical,content) in [
 ("identity",false,false,"BT /F1 10 Tf 100 700 Td <0011> Tj 1 Tr <001D> Tj ET"),
 ("remapped",true,false,"BT /F1 10 Tf 100 700 Td <41> Tj 1 Tr <42> Tj ET"),
 ("equivalent",true,false,"BT /F1 10 Tf 100 700 Td <42> Tj 1 Tr <41> Tj ET"),
 ("vertical",false,true,"BT /F1 10 Tf 100 700 Td <0011> Tj 1 Tr <001D> Tj ET")
 ] {std::fs::write(format!("/tmp/issue666-step7-{name}.pdf"),font::cid_cff_pdf(remap,vertical,content.as_bytes())).unwrap();}
}