use oxidize_pdf::text::fonts::TrueTypeFont;
use std::{hash::{Hash, Hasher}, collections::hash_map::DefaultHasher};
fn main() {
    for path in std::fs::read_to_string(std::env::args().nth(1).unwrap()).unwrap().lines() {
        let bytes = std::fs::read(path).unwrap();
        let output = std::panic::catch_unwind(|| {
            let font = TrueTypeFont::parse(bytes).map_err(|e| e.to_string())?;
            let maps = font.parse_cmap().map_err(|e| e.to_string())?;
            let mut canonical: Vec<_> = maps.into_iter().map(|t| {
                let mut pairs: Vec<_> = t.mappings.into_iter().collect(); pairs.sort();
                (t.platform_id, t.encoding_id, t.format, pairs)
            }).collect(); canonical.sort();
            let mut hash = DefaultHasher::new(); canonical.hash(&mut hash);
            Ok::<_, String>((font.num_glyphs, canonical.len(), hash.finish()))
        });
        println!("{path}: {output:?}");
    }
}
