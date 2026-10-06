//! Synthetic #677 variant with distinct visible text and ActualText.
use std::collections::BTreeMap;

pub fn replace(objects: &mut BTreeMap<u32, Vec<u8>>, id: u32, from: &str, to: &str) {
    let old = String::from_utf8(objects[&id].clone()).unwrap();
    assert!(old.contains(from), "fixture must exercise {from}");
    objects.insert(id, old.replace(from, to).into_bytes());
}

pub fn missing_pages(objects: &mut BTreeMap<u32, Vec<u8>>) {
    for n in 0..12 {
        replace(objects, 10 + n, &format!("/StructParents {n}"), "");
        let content = format!("/P << /MCID 0 /ActualText (accessible page {}) >> BDC BT /F1 12 Tf 20 200 Td (visible page {}) Tj ET EMC\n", n + 1, n + 1);
        objects.insert(
            30 + n,
            format!(
                "<< /Length {} >>\nstream\n{content}endstream",
                content.len()
            )
            .into_bytes(),
        );
    }
}
