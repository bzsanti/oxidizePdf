use oxidize_pdf::pipeline::PartitionConfig;

#[test]
fn prefer_ruling_tables_defaults_on() {
    assert!(PartitionConfig::default().prefer_ruling_tables);
}

use oxidize_pdf::pipeline::Partitioner;
use oxidize_pdf::text::TextFragment;

#[test]
fn with_graphics_none_matches_legacy_partition() {
    let frags: Vec<TextFragment> = vec![];
    let p = Partitioner::new(PartitionConfig::default());
    let legacy = p.partition_fragments(&frags, 1, 800.0);
    let with_graphics = p.partition_fragments_with_graphics(&frags, None, 1, 800.0);
    assert_eq!(legacy.len(), with_graphics.len());
}
