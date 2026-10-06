#[path="issue666_step5_support/cid.rs"] mod cid;
#[path="issue666_step5_support/expert.rs"] mod expert;
fn main() {
 for (name,identity,range,swapped,content) in [
 ("cidchar",false,false,false,"BT /F1 12 Tf 100 700 Td <018001C00001E0000001> Tj ET"),
 ("cidrange",false,true,false,"BT /F1 12 Tf 100 700 Td <0102C00001E0000001> Tj ET"),
 ("identity",true,false,false,"BT /F1 12 Tf 100 700 Td <001100130017001D> Tj ET"),
 ("swapped",false,false,true,"BT /F1 12 Tf 100 700 Td <018001C00001E0000001> Tj ET")
 ] {std::fs::write(format!("/tmp/issue666-step5-{name}.pdf"),cid::cid_pdf(identity,range,swapped,content.as_bytes())).unwrap();}
 std::fs::write("/tmp/issue666-step5-expert.pdf",expert::expert_pdf(b"BT /F1 12 Tf 100 700 Td <2C2E57> Tj ET",None)).unwrap();
}