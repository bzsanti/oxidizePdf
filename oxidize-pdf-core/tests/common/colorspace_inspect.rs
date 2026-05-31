//! Read-back helper for `/Resources/ColorSpace` round-trip tests.
//!
//! Several test crates write a `Document`, re-parse it, and inspect the first
//! page's colour-space resource map. This navigator (Kids → page → Resources →
//! ColorSpace, resolving indirect references at each hop) is shared here so the
//! walk lives in one place rather than being copied per test file.
//!
//! `dead_code` is suppressed at the module level: each `tests/*.rs` file is its
//! own crate, so crates that don't call this flag it as unused.
#![allow(dead_code)]

use oxidize_pdf::parser::objects::{PdfDictionary, PdfObject};
use oxidize_pdf::parser::PdfReader;

/// Walk the first page's `/Resources/ColorSpace` dict, resolving through
/// indirect references if the writer chose to emit any hop that way.
pub fn resolve_page0_colorspace<R: std::io::Read + std::io::Seek>(
    reader: &mut PdfReader<R>,
) -> PdfDictionary {
    let pages = reader.pages().expect("/Pages").clone();
    let kids = pages
        .get("Kids")
        .and_then(|o| o.as_array())
        .expect("/Pages/Kids");
    let (page_n, page_g) = kids.0[0].as_reference().expect("/Pages/Kids[0] ref");
    let page_obj = reader.get_object(page_n, page_g).expect("page").clone();
    let page_dict = page_obj.as_dict().expect("page dict").clone();
    let resources = match page_dict.get("Resources").expect("/Resources") {
        PdfObject::Dictionary(d) => d.clone(),
        PdfObject::Reference(n, g) => reader
            .get_object(*n, *g)
            .expect("resolve /Resources")
            .clone()
            .as_dict()
            .expect("/Resources is dict")
            .clone(),
        other => panic!("/Resources: unexpected {:?}", other),
    };
    match resources.get("ColorSpace").expect("/Resources/ColorSpace") {
        PdfObject::Dictionary(d) => d.clone(),
        PdfObject::Reference(n, g) => reader
            .get_object(*n, *g)
            .expect("resolve /ColorSpace")
            .clone()
            .as_dict()
            .expect("/ColorSpace dict")
            .clone(),
        other => panic!("/ColorSpace: unexpected {:?}", other),
    }
}
