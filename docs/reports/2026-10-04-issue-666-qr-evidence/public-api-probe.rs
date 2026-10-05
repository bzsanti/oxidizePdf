use oxidize_pdf::text::cmap::CMap;
fn main() { let _ = CMap { name: Some("Example".into()), ..Default::default() }; }
