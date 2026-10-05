#[path = "../tests/common/text_contracts.rs"] mod contract;
#[path = "../tests/common/pdf_assembler.rs"] mod assembler;
use contract::{pdf,cmap};
fn type3_pdf(matrix: f64, unicode: bool) -> Vec<u8> {
    let definition = format!(
        "<< /Type /Font /Subtype /Type3 /FontBBox [0 0 500 700] \
         /FontMatrix [{matrix} 0 0 {matrix} 0 0] \
         /CharProcs << /A 6 0 R /B 7 0 R >> \
         /Encoding << /Type /Encoding /Differences [65 /A /B] >> \
         /FirstChar 65 /LastChar 66 /Widths [500 500] /Resources << >> {} >>",
        if unicode { "/ToUnicode 8 0 R" } else { "" }
    );
    // d1 declares the same advance/bbox as the font. Each glyph paints a box.
    let program = b"500 0 0 0 500 700 d1 0 0 400 600 re f";
    let mut objects = vec![
        assembler::stream_obj("", program),
        assembler::stream_obj("", program),
    ];
    if unicode {
        objects.push(cmap("<41> <00660069>\n<42> <D83DDE00>", 2, "<00> <FF>"));
    }
    // Rendering-mode boundary keeps A and B in separate fragments without moving B.
    pdf(
        &definition,
        b"BT /F1 10 Tf 100 700 Td (A) Tj 1 Tr (B) Tj ET",
        objects,
    )
}

fn main() {
 for (name, matrix, unicode) in [("type3-normal",0.001,false),("type3-scaled",0.002,false),("type3-unicode",0.001,true)] {
 std::fs::write(format!("/tmp/issue666-step2-{name}.pdf"),type3_pdf(matrix,unicode)).unwrap();
 }
}
