// Included by several test binaries (the nightly order gate and the #448
// probe); each uses a different subset of the API.
#![allow(dead_code)]

//! Reading-ORDER metric shared by the differential order gate and the #448
//! reading-order probe.
//!
//! Method: align our word sequence with poppler's keeping only tokens present
//! in both with the same multiplicity (k-th occurrence to k-th occurrence), so
//! our filtered sequence is a PERMUTATION of poppler's. The longest increasing
//! subsequence of that permutation is the largest set of words we emit in
//! poppler's relative order; everything else is transposed.
//!
//! It lives here rather than inside the gate because the probe (#448 §5.7)
//! measures the SAME quantity on an experimental ordering. Re-deriving the
//! metric by eye would give a second oracle that does not agree with the gate,
//! and the two numbers would not be comparable — which is the whole point of
//! running the probe before committing to the wiring.

use std::collections::HashMap;

/// Words worth aligning: alphabetic, at least 4 characters. Shorter tokens
/// repeat too often to align reliably and would add ordering noise.
pub fn words(s: &str) -> Vec<String> {
    s.split(|c: char| !c.is_alphabetic())
        .filter(|t| t.chars().count() >= 4)
        .map(|t| t.to_lowercase())
        .collect()
}

/// Longest increasing subsequence length (patience sorting, O(n log n)).
pub fn lis(seq: &[usize]) -> usize {
    let mut tails: Vec<usize> = Vec::new();
    for &v in seq {
        match tails.binary_search(&v) {
            Ok(_) => {}
            Err(pos) if pos == tails.len() => tails.push(v),
            Err(pos) => tails[pos] = v,
        }
    }
    tails.len()
}

/// Alignment of one document pair against poppler's word sequence.
#[derive(Debug, Clone, Copy, Default)]
pub struct OrderMetrics {
    /// Poppler's alignable words. The DENOMINATOR: measured entirely on
    /// poppler's side, so it does not move with the quality of our extraction.
    pub pop_words: usize,
    /// Of those, how many we also emit (matched by multiplicity).
    pub common: usize,
    /// Of the common ones, the largest subset we emit in poppler's relative
    /// order (longest increasing subsequence).
    pub in_order: usize,
}

impl OrderMetrics {
    /// Poppler words we did not put where poppler put them: transposed
    /// (`common - in_order`) plus never emitted (`pop_words - common`).
    ///
    /// Ratcheting `transposed / common` instead would be unsound in the
    /// direction that matters: dropping the words we order worst shrinks both
    /// terms and *improves* the score. Missing text is a reading-order failure
    /// too — the reader does not get the words in poppler's order because the
    /// reader does not get them at all.
    pub fn misplaced(&self) -> usize {
        self.pop_words - self.in_order
    }

    /// Transposed only, over the aligned set: the diagnostic the redesign is
    /// steering, reported next to the ratcheted number but not ratcheted.
    pub fn transposed(&self) -> usize {
        self.common - self.in_order
    }
}

/// Alignment metrics for one document pair.
pub fn order_metrics(our_txt: &str, pop_txt: &str) -> OrderMetrics {
    let pw = words(pop_txt);
    let ow = words(our_txt);
    order_metrics_words(&ow, &pw)
}

/// Same as [`order_metrics`] but on already-tokenised sequences, so a caller
/// comparing several orderings of the same document tokenises poppler once.
pub fn order_metrics_words(our_words: &[String], pop_words: &[String]) -> OrderMetrics {
    let mut pos_of: HashMap<&str, Vec<usize>> = HashMap::new();
    for (i, w) in pop_words.iter().enumerate() {
        pos_of.entry(w.as_str()).or_default().push(i);
    }
    let mut next: HashMap<&str, usize> = HashMap::new();
    let mut mapped: Vec<usize> = Vec::new();
    for w in our_words {
        if let Some(list) = pos_of.get(w.as_str()) {
            let k = next.entry(w.as_str()).or_insert(0);
            if *k < list.len() {
                mapped.push(list[*k]);
                *k += 1;
            }
        }
    }
    OrderMetrics {
        pop_words: pop_words.len(),
        common: mapped.len(),
        in_order: lis(&mapped),
    }
}

#[cfg(test)]
mod order_metric_tests {
    use super::*;

    /// Ten words, every one at least four letters so none is filtered out by
    /// `words()` — otherwise the fixture would silently shrink the denominator
    /// and the assertions below would encode the filter, not the metric.
    const POP: &str = "alpha beta gamma delta epsilon zeta sigma theta iota kappa";

    /// A faithful extraction leaves nothing out of order.
    #[test]
    fn identical_text_has_no_misplaced_words() {
        let m = order_metrics(POP, POP);
        assert_eq!(m.pop_words, 10);
        assert_eq!(m.misplaced(), 0);
    }

    /// Reordering the words is what the gate exists to see.
    #[test]
    fn permuted_text_is_misplaced() {
        let permuted = "kappa iota theta sigma zeta epsilon delta gamma beta alpha";
        let m = order_metrics(permuted, POP);
        assert_eq!(m.common, 10, "every word is still present");
        assert!(
            m.misplaced() >= 8,
            "a full reversal leaves at most one word in increasing order; got {m:?}"
        );
    }

    /// The reason the denominator is poppler's word count and not the size of
    /// the intersection: text we simply fail to emit must COUNT AGAINST us. With
    /// `common` as denominator, dropping words the extractor was mangling reads
    /// as a perfect score.
    #[test]
    fn dropping_words_counts_against_us_instead_of_flattering_the_rate() {
        let ours = "alpha beta gamma delta";
        let m = order_metrics(ours, POP);
        assert_eq!(m.common, 4, "only four of poppler's words survive");
        assert_eq!(
            m.in_order, 4,
            "and those four are in poppler's relative order"
        );
        assert_eq!(
            m.misplaced(),
            6,
            "the six words we never emitted are misplaced, not excused"
        );
        assert_eq!(m.pop_words, 10, "the denominator stays poppler-side");
    }

    /// Words we invent are not credited: the metric only ever walks poppler's
    /// sequence.
    #[test]
    fn extra_words_of_ours_do_not_change_the_denominator() {
        let ours = format!("{POP} lambda mu nu");
        let m = order_metrics(&ours, POP);
        assert_eq!(m.pop_words, 10);
        assert_eq!(m.misplaced(), 0);
    }

    /// The pre-tokenised entry point must agree with the string one, or the
    /// probe (which tokenises poppler once and reuses it across orderings) and
    /// the gate would be measuring with two different rulers.
    #[test]
    fn pre_tokenised_entry_point_agrees_with_the_string_one() {
        let ours = "gamma alpha beta delta epsilon zeta sigma theta iota kappa";
        let from_str = order_metrics(ours, POP);
        let from_words = order_metrics_words(&words(ours), &words(POP));
        assert_eq!(from_str.pop_words, from_words.pop_words);
        assert_eq!(from_str.common, from_words.common);
        assert_eq!(from_str.in_order, from_words.in_order);
        assert!(
            from_str.transposed() > 0,
            "fixture must actually transpose something, else both rulers agree vacuously"
        );
    }
}
