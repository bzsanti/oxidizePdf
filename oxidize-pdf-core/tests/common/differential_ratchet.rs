//! Ratchet policy shared by the two differential gates (fusion and order).
//!
//! Both gates compare our extraction against poppler over a corpus and refuse
//! to let a defect count grow. Ratcheting the ABSOLUTE count is unsound: the
//! count is a numerator whose denominator (how much text was actually compared)
//! is not fixed. Extracting less — a file that stops parsing, a page that comes
//! out empty — lowers the numerator and reads as an IMPROVEMENT while the
//! product got worse. Quantified on the committed baselines: losing 21 of 1037
//! files (2%) consumes the entire fusion-gate slack.
//!
//! So a run is judged on three axes, and a regression on any one fails:
//!
//! 1. **Rate**, not count: `numerator / denominator`, where the denominator is
//!    measured on POPPLER's side (candidate word pairs, aligned words) and is
//!    therefore independent of how good our extractor is that day.
//! 2. **File coverage**: the number of files actually compared may not fall
//!    below the baseline's. Losing files shrinks numerator and denominator
//!    together, so the rate alone cannot see it.
//! 3. **Content coverage**: alphabetic characters we emit over alphabetic
//!    characters poppler emits. Catches the file that still parses but now
//!    yields half a page — invisible to both of the above. Letters are the
//!    right unit here: it is unaffected by word fusion (gluing two words keeps
//!    every letter) and by ordering (a permutation keeps every letter), so it
//!    measures content retention and nothing else.
//!
//! The policy is a pure function so it can be unit-tested without a corpus:
//! the tests at the bottom of this file run on every PR, even though the gates
//! themselves are inert there.
#![allow(dead_code)]

/// One gate run, in the terms the ratchet judges.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GateSample {
    /// Files actually compared (excluded files are not counted here).
    pub compared: usize,
    /// The defect being counted: fusions, or transposed words.
    pub numerator: usize,
    /// Opportunities for that defect, measured on poppler's output:
    /// candidate word pairs (fusion gate) or aligned words (order gate).
    pub denominator: usize,
    /// Alphabetic characters in OUR text, summed over compared files.
    pub our_alpha_chars: u64,
    /// Alphabetic characters in POPPLER's text, summed over compared files.
    pub pop_alpha_chars: u64,
}

impl GateSample {
    /// Defects per opportunity. `0.0` for an empty denominator (nothing to be
    /// wrong about), which the coverage checks then catch separately.
    pub fn rate(&self) -> f64 {
        if self.denominator == 0 {
            0.0
        } else {
            self.numerator as f64 / self.denominator as f64
        }
    }

    /// Fraction of poppler's letters that our extraction also produced. Can
    /// exceed 1.0 when we emit text poppler drops; the gate only ever checks it
    /// from below.
    pub fn coverage(&self) -> f64 {
        if self.pop_alpha_chars == 0 {
            0.0
        } else {
            self.our_alpha_chars as f64 / self.pop_alpha_chars as f64
        }
    }
}

/// Relative slack on the rate, absorbing run-to-run nondeterminism (a file that
/// sometimes times out shifts both terms slightly). A real regression moves the
/// rate far more.
const RATE_SLACK_REL: f64 = 0.02;

/// Absolute floor on the rate slack, so a gate whose rate is near zero is not
/// held to an impossible tolerance.
const RATE_SLACK_ABS: f64 = 0.0005;

/// How much of the baseline's compared-file set a run may lose before it counts
/// as a coverage regression rather than noise.
const COMPARED_FLOOR_REL: f64 = 0.98;

/// Same tolerance applied to content coverage.
const COVERAGE_FLOOR_REL: f64 = 0.98;

/// Every way `current` is worse than `baseline`. Empty means the run passes.
///
/// Each message names the axis, both values, and what the failure means, so a
/// nightly failure is actionable without re-deriving the policy.
pub fn regressions(current: &GateSample, baseline: &GateSample, defect: &str) -> Vec<String> {
    let mut out = Vec::new();

    let allowed_rate = baseline.rate() * (1.0 + RATE_SLACK_REL) + RATE_SLACK_ABS;
    if current.rate() > allowed_rate {
        out.push(format!(
            "RATE regression: {defect} rate {:.6} ({}/{}) vs baseline {:.6} ({}/{}), \
             allowed up to {allowed_rate:.6}. The rate is what matters: the absolute count \
             can fall while the rate rises if we compare less text.",
            current.rate(),
            current.numerator,
            current.denominator,
            baseline.rate(),
            baseline.numerator,
            baseline.denominator,
        ));
    }

    let min_compared = (baseline.compared as f64 * COMPARED_FLOOR_REL).floor() as usize;
    if current.compared < min_compared {
        out.push(format!(
            "FILE COVERAGE regression: compared {} files vs baseline {} (floor {min_compared}). \
             Fewer files means a smaller numerator, which would otherwise read as an \
             improvement — files stopped being comparable (parse failures, timeouts).",
            current.compared, baseline.compared,
        ));
    }

    let min_coverage = baseline.coverage() * COVERAGE_FLOOR_REL;
    if current.coverage() < min_coverage {
        out.push(format!(
            "CONTENT COVERAGE regression: we emit {:.4} of poppler's letters ({} of {}) vs \
             baseline {:.4} ({} of {}), floor {min_coverage:.4}. Files still compare but yield \
             less text, which lowers the defect count without improving anything.",
            current.coverage(),
            current.our_alpha_chars,
            current.pop_alpha_chars,
            baseline.coverage(),
            baseline.our_alpha_chars,
            baseline.pop_alpha_chars,
        ));
    }

    out
}

/// Count alphabetic characters. The content-coverage unit; see the module docs
/// for why letters rather than words.
pub fn alpha_chars(s: &str) -> u64 {
    s.chars().filter(|c| c.is_alphabetic()).count() as u64
}

/// Read the baseline recorded for `key` from a gate's baseline file.
///
/// `None` means "no usable baseline", which the gates treat as first-run and
/// record. A baseline written before this policy existed carries only a count
/// (no denominator, no coverage) and is therefore unusable: judging a rate
/// against it would silently invent a denominator. Those are reported as
/// `None` so they are re-measured rather than half-honoured.
pub fn load_baseline(path: &std::path::Path, key: &str) -> Option<GateSample> {
    let file: serde_json::Map<String, serde_json::Value> = std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())?;
    let entry = file.get(key)?;
    let field = |name: &str| entry.get(name).and_then(|v| v.as_u64());
    Some(GateSample {
        compared: field("compared")? as usize,
        numerator: field("numerator")? as usize,
        denominator: field("denominator")? as usize,
        our_alpha_chars: field("our_alpha_chars")?,
        pop_alpha_chars: field("pop_alpha_chars")?,
    })
}

/// Write `sample` under `key`, preserving other keys in the file. Best-effort:
/// a read-only checkout must not fail the gate.
///
/// `rate` and `content_coverage` are written alongside the raw counts for the
/// human reading the diff; [`load_baseline`] recomputes both from the counts
/// and never reads them back, so an edited derived value cannot move the gate.
pub fn record_baseline(path: &std::path::Path, key: &str, sample: &GateSample) {
    let mut file: serde_json::Map<String, serde_json::Value> = std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    file.insert(
        key.to_string(),
        serde_json::json!({
            "compared": sample.compared,
            "numerator": sample.numerator,
            "denominator": sample.denominator,
            "our_alpha_chars": sample.our_alpha_chars,
            "pop_alpha_chars": sample.pop_alpha_chars,
            "rate": sample.rate(),
            "content_coverage": sample.coverage(),
        }),
    );
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    std::fs::write(path, serde_json::to_string_pretty(&file).unwrap()).ok();
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The baseline shape both gates ratchet against, in round numbers close to
    /// the committed ones (1037 files compared, ~750 fusions).
    fn baseline() -> GateSample {
        GateSample {
            compared: 1037,
            numerator: 752,
            denominator: 100_000,
            our_alpha_chars: 970_000,
            pop_alpha_chars: 1_000_000,
        }
    }

    /// The exact scenario that motivated the change (issue-448 review, I2):
    /// 2% of the corpus stops being comparable, so every total falls
    /// proportionally. The absolute count drops by 16 — an "IMPROVEMENT" under
    /// the old policy — while nothing about the extractor improved.
    #[test]
    fn losing_two_percent_of_the_files_is_not_an_improvement() {
        let b = baseline();
        let current = GateSample {
            compared: 1015,
            numerator: 736,
            denominator: 97_900,
            our_alpha_chars: 949_600,
            pop_alpha_chars: 979_000,
        };
        assert!(
            current.numerator < b.numerator,
            "precondition: the absolute count must LOOK better, or the test proves nothing"
        );
        let found = regressions(&current, &b, "fusion");
        assert!(
            found.iter().any(|m| m.contains("FILE COVERAGE")),
            "losing 22 of 1037 files must fail the file-coverage floor; got {found:?}"
        );
    }

    /// A file still parses and still compares, but now yields a fraction of its
    /// text. Counts fall, rate falls, file count is untouched: only content
    /// coverage can see this.
    #[test]
    fn emitting_less_text_from_the_same_files_is_a_regression() {
        let b = baseline();
        let current = GateSample {
            our_alpha_chars: 800_000,
            numerator: 600,
            ..b
        };
        let found = regressions(&current, &b, "fusion");
        assert!(
            found.iter().any(|m| m.contains("CONTENT COVERAGE")),
            "a 17% drop in emitted letters must fail the content floor; got {found:?}"
        );
    }

    /// Poppler fails on the biggest documents, so the denominator collapses
    /// faster than the numerator. The absolute count improves; the rate — the
    /// thing that actually describes the extractor — gets worse.
    #[test]
    fn a_falling_count_over_a_faster_falling_denominator_is_a_rate_regression() {
        let b = baseline();
        let current = GateSample {
            numerator: 700,
            denominator: 80_000,
            ..b
        };
        assert!(
            current.numerator < b.numerator,
            "precondition: count is down"
        );
        let found = regressions(&current, &b, "fusion");
        assert!(
            found.iter().any(|m| m.contains("RATE")),
            "rate 0.00875 vs baseline 0.00752 must fail; got {found:?}"
        );
    }

    /// The straightforward case the old gate did catch, which must keep failing.
    #[test]
    fn more_defects_over_the_same_corpus_is_a_rate_regression() {
        let b = baseline();
        let current = GateSample {
            numerator: 900,
            ..b
        };
        let found = regressions(&current, &b, "fusion");
        assert!(
            found.iter().any(|m| m.contains("RATE")),
            "900 vs 752 over an identical denominator must fail; got {found:?}"
        );
    }

    /// A real fix: same corpus, same text volume, far fewer defects.
    #[test]
    fn a_genuine_improvement_passes_every_axis() {
        let b = baseline();
        let current = GateSample {
            numerator: 300,
            ..b
        };
        assert_eq!(
            regressions(&current, &b, "fusion"),
            Vec::<String>::new(),
            "halving the defect rate at constant coverage must pass"
        );
    }

    /// Run-to-run noise (a couple of files timing out, a rate wobble under the
    /// slack) must not fail the nightly.
    #[test]
    fn noise_within_slack_passes() {
        let b = baseline();
        let current = GateSample {
            compared: 1030,
            numerator: 760,
            denominator: 99_500,
            our_alpha_chars: 965_000,
            pop_alpha_chars: 995_000,
        };
        assert_eq!(
            regressions(&current, &b, "fusion"),
            Vec::<String>::new(),
            "sub-1% wobble on every axis must pass; got {:?}",
            regressions(&current, &b, "fusion")
        );
    }

    /// A baseline survives the round trip through the file exactly, so the next
    /// run judges against what this run measured.
    #[test]
    fn a_recorded_baseline_reads_back_identically() {
        let dir = std::env::temp_dir().join("oxidize_ratchet_roundtrip");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("baseline.json");
        let _ = std::fs::remove_file(&path);

        let sample = baseline();
        record_baseline(&path, "t3-stress", &sample);
        assert_eq!(load_baseline(&path, "t3-stress"), Some(sample));
        assert_eq!(
            load_baseline(&path, "other-corpus"),
            None,
            "a key that was never recorded has no baseline"
        );
    }

    /// Recording one corpus must not erase another's baseline.
    #[test]
    fn recording_one_key_preserves_the_others() {
        let dir = std::env::temp_dir().join("oxidize_ratchet_multikey");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("baseline.json");
        let _ = std::fs::remove_file(&path);

        let first = baseline();
        let second = GateSample {
            compared: 12,
            ..baseline()
        };
        record_baseline(&path, "t3-stress", &first);
        record_baseline(&path, "t2-realworld", &second);
        assert_eq!(load_baseline(&path, "t3-stress"), Some(first));
        assert_eq!(load_baseline(&path, "t2-realworld"), Some(second));
    }

    /// Baselines written before this policy existed hold only a count. Judging
    /// a rate against them would mean inventing a denominator, so they must
    /// read as absent and be re-measured.
    #[test]
    fn a_pre_policy_baseline_is_not_usable() {
        let dir = std::env::temp_dir().join("oxidize_ratchet_legacy");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("baseline.json");
        std::fs::write(
            &path,
            r#"{"t3-stress": 752, "t2-realworld": {"transposed": 5, "common": 9}}"#,
        )
        .unwrap();
        assert_eq!(load_baseline(&path, "t3-stress"), None);
        assert_eq!(
            load_baseline(&path, "t2-realworld"),
            None,
            "the order gate's old two-field shape has no coverage data either"
        );
    }

    /// The order gate uses the same policy with `common` as denominator, so the
    /// baseline numbers there must judge the same way.
    #[test]
    fn the_policy_applies_unchanged_to_the_order_gate_numbers() {
        let b = GateSample {
            compared: 1037,
            numerator: 148_658,
            denominator: 570_129,
            our_alpha_chars: 970_000,
            pop_alpha_chars: 1_000_000,
        };
        let worse = GateSample {
            numerator: 160_000,
            ..b
        };
        assert!(
            regressions(&worse, &b, "transposed")
                .iter()
                .any(|m| m.contains("RATE")),
            "0.2807 vs 0.2607 transposed rate must fail"
        );
        let better = GateSample {
            numerator: 100_000,
            ..b
        };
        assert_eq!(regressions(&better, &b, "transposed"), Vec::<String>::new());
    }
}
