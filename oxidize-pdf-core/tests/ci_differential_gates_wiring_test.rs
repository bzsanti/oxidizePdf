//! Every differential gate must actually be RUN, and its result must count.
//!
//! The differential gates skip themselves when poppler or the corpus is absent,
//! which makes them inert on PRs by design — and invisible when nobody wires
//! them into CI. `differential_order_test.rs` shipped with a committed baseline
//! that nothing ever ratcheted against: the file existed, the baseline existed,
//! and no workflow step ran either. A gate that never runs is worse than no
//! gate, because the committed baseline reads as a guarantee.
//!
//! Three ways that failure hides, all checked here:
//!   1. no step invokes the gate at all;
//!   2. a step invokes it but the job cannot fail on it (`continue-on-error`),
//!      or the step is commented out;
//!   3. the step runs and the gate silently skips, because poppler is missing
//!      or the best-effort corpus download produced nothing. The gates turn
//!      that into a failure when `OXIDIZE_DIFF_REQUIRE_CORPUS=1`, so the
//!      nightly must set it.
//!
//! This closes the class rather than the instance: the gates are discovered
//! from the filesystem, so the next `differential_*_test.rs` added without a
//! workflow step fails here instead of quietly never running.

use std::path::{Path, PathBuf};

/// The job that is supposed to run the gates.
const GATE_JOB: &str = "tier-nightly";

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR is <repo>/oxidize-pdf-core.
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate dir must have a parent")
        .to_path_buf()
}

fn workflow_path() -> PathBuf {
    repo_root().join(".github/workflows/corpus-tests.yml")
}

fn workflow_text() -> String {
    let path = workflow_path();
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()))
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

/// The lines of one job, by name. Jobs are the 2-space-indented keys under
/// `jobs:`; the block ends at the next key at that indent.
fn job_lines(workflow: &str, job: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut inside = false;
    for line in workflow.lines() {
        let is_job_key = line.starts_with("  ")
            && !line.starts_with("   ")
            && line.trim_end().ends_with(':')
            && !line.trim_start().starts_with('#');
        if is_job_key {
            inside = line.trim() == format!("{job}:");
            continue;
        }
        if inside {
            out.push(line.to_string());
        }
    }
    out
}

/// Split a job's lines into steps (each starting at a `- ` list item).
fn steps(job: &[String]) -> Vec<Vec<String>> {
    let mut out: Vec<Vec<String>> = Vec::new();
    for line in job {
        if line.trim_start().starts_with("- ") && !line.trim_start().starts_with("#") {
            out.push(vec![line.clone()]);
        } else if let Some(last) = out.last_mut() {
            last.push(line.clone());
        }
    }
    out
}

/// Lines of a step with YAML comments dropped, so a commented-out command
/// cannot satisfy a search.
fn effective(step: &[String]) -> String {
    step.iter()
        .filter(|l| !l.trim_start().starts_with('#'))
        .cloned()
        .collect::<Vec<_>>()
        .join("\n")
}

/// The step in `GATE_JOB` that runs `cargo test --test <gate>`, if any.
fn step_running(workflow: &str, gate: &str) -> Option<String> {
    let job = job_lines(workflow, GATE_JOB);
    steps(&job)
        .iter()
        .map(|s| effective(s))
        .find(|s| s.contains(&format!("--test {gate}")))
}

#[test]
fn every_differential_gate_has_a_step_in_the_nightly_job() {
    let workflow = workflow_text();
    let gates = differential_gate_names();
    assert!(
        !gates.is_empty(),
        "no differential_*_test.rs found — this test would pass vacuously"
    );

    let missing: Vec<&String> = gates
        .iter()
        .filter(|g| step_running(&workflow, g).is_none())
        .collect();

    assert!(
        missing.is_empty(),
        "these differential gates are never executed by the `{GATE_JOB}` job of {}: {missing:?}\n\
         Each one skips silently without poppler and the corpus, so an unwired gate reports \
         nothing while its committed baseline suggests it is being enforced. Add a \
         `cargo test --test <name>` step to that job.",
        workflow_path().display(),
    );
}

/// A step that cannot fail the job is decoration.
#[test]
fn no_gate_step_is_allowed_to_swallow_its_failure() {
    let workflow = workflow_text();
    for gate in differential_gate_names() {
        let step = step_running(&workflow, &gate)
            .unwrap_or_else(|| panic!("{gate} has no step; see the wiring test"));
        assert!(
            !step.contains("continue-on-error: true"),
            "the step running {gate} sets continue-on-error, so a regression it detects \
             cannot fail the job"
        );
    }
}

/// Without poppler, or with an empty corpus, the gates skip and the job goes
/// green having measured nothing. The nightly must demand a real measurement.
#[test]
fn the_nightly_requires_the_gates_to_actually_measure() {
    let workflow = workflow_text();
    for gate in differential_gate_names() {
        let step = step_running(&workflow, &gate)
            .unwrap_or_else(|| panic!("{gate} has no step; see the wiring test"));
        assert!(
            step.contains("OXIDIZE_DIFF_REQUIRE_CORPUS"),
            "the step running {gate} does not set OXIDIZE_DIFF_REQUIRE_CORPUS, so a failed \
             corpus download or a missing poppler turns the gate into a silent skip and the \
             job still passes"
        );
    }
}

/// The gates need `pdftotext`; without it every one of them takes its skip path.
#[test]
fn the_corpus_workflow_installs_poppler_for_the_gates() {
    assert!(
        workflow_text().contains("poppler-utils"),
        "the corpus workflow must install poppler-utils, or every differential gate skips \
         itself and the job is green without measuring anything"
    );
}
