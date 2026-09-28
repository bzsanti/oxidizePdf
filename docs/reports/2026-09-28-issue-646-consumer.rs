use oxidize_pdf::{Document, Page};
use oxidize_pdf::signatures::{create_signature_slot, draw_signature_slot, read_signature_slot, list_signature_slots, remove_signature_slot, SignatureRect};
fn main() {
    let mut doc = Document::new();
    doc.add_page(Page::a4());
    let base = doc.to_bytes().unwrap();
    let rect = SignatureRect {left: 30.0, bottom: 40.0, right: 330.0, top: 140.0};
    let prepared = create_signature_slot(&base, "alice", 0, rect, "Alice|handwriting").unwrap();
    let drawn = draw_signature_slot(&prepared, "alice", &[vec![[0.0, 0.2], [0.4, 0.8], [0.8, 0.1], [1.0, 0.5]]], true, Some("Alice (participant)")).unwrap();
    let slot = read_signature_slot(&drawn, "alice").unwrap();
    assert!(slot.completed && !slot.digitally_signed);
    assert!(drawn.starts_with(&base));
    assert_eq!(list_signature_slots(&drawn).unwrap().len(), 1);
    let removed = remove_signature_slot(&prepared, "alice").unwrap();
    assert!(list_signature_slots(&removed).unwrap().is_empty());
    std::fs::write("/tmp/signature-preparation-consumer/prepared.pdf", prepared).unwrap();
    std::fs::write("/tmp/signature-preparation-consumer/drawn.pdf", drawn).unwrap();
    std::fs::write("/tmp/signature-preparation-consumer/removed.pdf", removed).unwrap();
    println!("Public compression-only consumer passed");
}
