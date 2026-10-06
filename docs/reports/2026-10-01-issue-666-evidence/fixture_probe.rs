#[path = "../tests/common/text_contracts.rs"]
mod contract;
use contract::{pdf, font, cmap, corrupt_descendant_pdf};
fn main() {
    let dir=std::path::PathBuf::from(std::env::args_os().nth(1).expect("output directory"));
    std::fs::create_dir_all(&dir).unwrap();
    for encoding in ["StandardEncoding", "WinAnsiEncoding", "MacRomanEncoding"] {
        let data=pdf(&font("Helvetica", &format!("/{encoding}"),None,""),b"BT /F1 12 Tf 100 700 Td <41DB> Tj ET",vec![]);
        std::fs::write(dir.join(format!("{encoding}.pdf")),data).unwrap();
    }
    let data=pdf(&font("Helvetica","<< /BaseEncoding /MacRomanEncoding /Differences [65 /Euro] >>",None,"/ToUnicode 6 0 R"),b"BT /F1 12 Tf 100 700 Td <41> Tj ET",vec![cmap("<41> <D83DDE00>",1,"<00> <FF>")]);
    std::fs::write(dir.join("precedence.pdf"),data).unwrap();
    let data=pdf(&font("Helvetica","/WinAnsiEncoding",Some(500.0),""),b"BT /F1 10 Tf 50 Tz 100 700 Td [(A)-300] TJ 1 Tr (B) Tj ET",vec![]);
    std::fs::write(dir.join("geometry.pdf"),data).unwrap();
    std::fs::write(dir.join("damaged-descendant.pdf"),corrupt_descendant_pdf(b"BT /F1 10 Tf 100 700 Td [<0001>-1000<0001>-1000<0003>-1000<0002>] TJ ET")).unwrap();
}
