#[path="issue666_step6_support/vertical.rs"] mod vertical;
fn main() {
 let two=b"BT /F1 10 Tf 100 700 Td <0011> Tj 1 Tr <0013> Tj ET";
 for (name,metrics,embedded,content) in [
 ("default","",false,two.as_slice()),("embedded","",true,two.as_slice()),
 ("dw2","/DW2 [900 -1300]",false,two.as_slice()),
 ("array","/DW2 [900 -1300] /W2 [17 [-1200 272 880]]",false,two.as_slice()),
 ("range","/DW2 [900 -1300] /W2 [17 19 -1200 272 880]",false,two.as_slice()),
 ("fallback","/DW2 [900 -1300] /W2 [17 [-1200 272 880]]",false,b"BT /F1 10 Tf 100 700 Td <0011> Tj 1 Tr <0013> Tj 0 Tr <0011> Tj ET".as_slice()),
 ("tj-positive","",false,b"BT /F1 10 Tf 100 700 Td [<0011> 300] TJ 1 Tr <0013> Tj ET".as_slice()),
 ("tj-negative","",false,b"BT /F1 10 Tf 100 700 Td [<0011> -300] TJ 1 Tr <0013> Tj ET".as_slice()),
 ("scale-50","",false,b"BT /F1 10 Tf 50 Tz 100 700 Td <0011> Tj 1 Tr <0013> Tj ET".as_slice()),
 ("scale-200","",false,b"BT /F1 10 Tf 200 Tz 100 700 Td <0011> Tj 1 Tr <0013> Tj ET".as_slice()),
 ("char-spacing","",false,b"BT /F1 10 Tf 2 Tc 100 700 Td <0011> Tj 1 Tr <0013> Tj ET".as_slice())
 ] {std::fs::write(format!("/tmp/issue666-step6-{name}.pdf"),vertical::vertical_pdf(metrics,embedded,content)).unwrap();}
}