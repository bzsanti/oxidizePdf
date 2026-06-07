use oxidize_pdf::pipeline::PartitionConfig;

#[test]
fn prefer_ruling_tables_defaults_on() {
    assert!(PartitionConfig::default().prefer_ruling_tables);
}
