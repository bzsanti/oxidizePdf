//! Every differential gate must actually be RUN by the nightly corpus job.
//!
//! The differential gates skip themselves when poppler or the corpus is absent,
//! which makes them inert on PRs by design — and invisible when nobody wires
//! them into CI. `differential_order_test.rs` shipped with a committed baseline
//! that nothing ever ratcheted against: the file existed, the baseline existed,
//! and no workflow step ran either. A gate that never runs is worse than no
//! gate, because the committed baseline reads as a guarantee.
//!
//! This test closes the class rather than the instance: it discovers the gates
//! from the filesystem, so the next `differential_*_test.rs` added without a
//! workflow step fails here instead of quietly never running.

use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR is <repo>/oxidize-pdf-core.
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate dir must have a parent")
        .to_path_buf()
}

/// Names of the differential gate test binaries present in `tests/`.
fn differential_gate_names() -> Vec<String> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
    let mut names: Vec<String> = std::fs::read_dir(&dir)
        .expect("tests/ must be readable")
        .filter_map(|e| e.ok())
        .filter_map(|e| e.file_name().into_string().ok())
        .filter_map(|f| f.strip_suffix(".rs").map(str::to_string))
        .filter(|n| n.starts_with("differential_") && n.ends_with("_test"))
        .collect();
    names.sort();
    names
}

#[test]
fn every_differential_gate_has_a_step_in_the_corpus_workflow() {
    let workflow_path = repo_root().join(".github/workflows/corpus-tests.yml");
    let workflow = std::fs::read_to_string(&workflow_path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", workflow_path.display()));

    let gates = differential_gate_names();
    assert!(
        !gates.is_empty(),
        "no differential_*_test.rs found — this test would pass vacuously"
    );

    let missing: Vec<&String> = gates
        .iter()
        .filter(|name| !workflow.contains(&format!("--test {name}")))
        .collect();

    assert!(
        missing.is_empty(),
        "these differential gates are never executed by {}: {:?}\n\
         Each one skips silently without poppler and the corpus, so an unwired gate reports \
         nothing while its committed baseline suggests it is being enforced. Add a \
         `cargo test --test <name>` step to the nightly corpus job.",
        workflow_path.display(),
        missing
    );
}

/// The gates need `pdftotext`; without it every one of them takes its skip path
/// and the job goes green having measured nothing.
#[test]
fn the_corpus_workflow_installs_poppler_for_the_gates() {
    let workflow_path = repo_root().join(".github/workflows/corpus-tests.yml");
    let workflow = std::fs::read_to_string(&workflow_path).expect("workflow must be readable");
    assert!(
        workflow.contains("poppler-utils"),
        "the corpus workflow must install poppler-utils, or every differential gate skips \
         itself and the job is green without measuring anything"
    );
}
