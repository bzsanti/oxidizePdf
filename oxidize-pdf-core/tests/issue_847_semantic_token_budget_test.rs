use oxidize_pdf::pipeline::{
    Element, ElementData, SemanticChunkConfig, SemanticChunker, TokenCounter,
};
use std::sync::Arc;

// Counts separators too: unlike a word proxy, joining costs an extra token.
struct Characters;
impl TokenCounter for Characters {
    fn count(&self, text: &str) -> usize {
        text.chars().count()
    }
    fn name(&self) -> &'static str {
        "characters"
    }
}
fn para(text: &str) -> Element {
    Element::Paragraph(ElementData {
        text: text.into(),
        metadata: Default::default(),
    })
}
fn chunker(max: usize, overlap: usize) -> SemanticChunker {
    SemanticChunker::new(SemanticChunkConfig {
        max_tokens: max,
        overlap_tokens: overlap,
        respect_element_boundaries: true,
    })
    .with_token_counter(Arc::new(Characters))
}
#[test]
fn joined_elements_are_measured_before_merging() {
    let chunks = chunker(2, 0).chunk(&[para("a"), para("b")]);
    assert_eq!(
        chunks.iter().map(|c| c.text()).collect::<Vec<_>>(),
        ["a", "b"]
    );
}
#[test]
fn sentence_joins_obey_selected_counter() {
    let chunks = chunker(4, 0).chunk(&[para("a. b. c.")]);
    assert_eq!(
        chunks.iter().map(|c| c.text()).collect::<Vec<_>>(),
        ["a.", "b.", "c."]
    );
    assert!(chunks.iter().all(|c| !c.is_oversized()));
}
#[test]
fn overlap_must_fit_both_overlap_and_chunk_budget() {
    let chunks = chunker(5, 3).chunk(&[para("aa"), para("b"), para("ccc")]);
    assert_eq!(
        chunks.iter().map(|c| c.text()).collect::<Vec<_>>(),
        ["aa\nb", "b\nccc"]
    );
    assert!(chunks.iter().all(|c| c.token_estimate() <= 5));
}
#[test]
fn overlap_that_cannot_fit_is_dropped() {
    let chunks = chunker(3, 2).chunk(&[para("aa"), para("bb"), para("cc")]);
    assert_eq!(
        chunks.iter().map(|c| c.text()).collect::<Vec<_>>(),
        ["aa", "bb", "cc"]
    );
    assert!(chunks.iter().all(|c| !c.is_oversized()));
}
#[test]
fn final_indivisible_sentence_is_marked_oversized() {
    let chunks = chunker(2, 0).chunk(&[para("abc")]);
    assert_eq!(chunks[0].token_estimate(), 3);
    assert!(chunks[0].is_oversized());
}
#[test]
fn zero_count_elements_are_not_discarded_or_used_as_empty_sentinel() {
    let chunks = chunker(1, 0).chunk(&[para(""), para("a")]);
    assert_eq!(chunks.len(), 2);
    assert_eq!(chunks.iter().map(|c| c.elements().len()).sum::<usize>(), 2);
    assert!(chunks.iter().all(|c| c.token_estimate() <= 1));
}
#[cfg(feature = "tiktoken")]
#[test]
fn cl100k_published_core_regression() {
    use oxidize_pdf::pipeline::TiktokenCounter;
    let counter = Arc::new(TiktokenCounter::cl100k_base());
    assert_eq!(counter.count("a\nb"), 3);
    let chunks = SemanticChunker::new(SemanticChunkConfig::new(2).with_overlap(0))
        .with_token_counter(counter.clone())
        .chunk(&[para("a"), para("b")]);
    assert_eq!(chunks.len(), 2);
    for c in chunks {
        assert_eq!(c.token_estimate(), counter.count(&c.text()));
        assert!(!c.is_oversized());
    }
}

#[test]
fn word_proxy_retains_overlap_when_both_limits_allow_it() {
    let chunks = SemanticChunker::new(SemanticChunkConfig::new(3).with_overlap(1)).chunk(&[
        para("one two"),
        para("three"),
        para("four five"),
    ]);
    assert_eq!(
        chunks.iter().map(|c| c.text()).collect::<Vec<_>>(),
        ["one two\nthree", "three\nfour five"]
    );
    assert!(chunks
        .iter()
        .all(|c| c.token_estimate() == 3 && !c.is_oversized()));
}

#[test]
fn structural_elements_remain_whole_and_excess_is_reported_with_either_policy() {
    for respect in [true, false] {
        let heading = Element::Title(ElementData {
            text: "long title".into(),
            metadata: Default::default(),
        });
        let chunks = SemanticChunker::new(SemanticChunkConfig {
            max_tokens: 2,
            overlap_tokens: 0,
            respect_element_boundaries: respect,
        })
        .with_token_counter(Arc::new(Characters))
        .chunk(&[para("x"), heading.clone()]);
        let headings: Vec<_> = chunks
            .iter()
            .flat_map(|c| c.elements())
            .filter(|e| matches!(e, Element::Title(_)))
            .collect();
        assert_eq!(headings, vec![&heading]);
        let oversized = chunks
            .iter()
            .find(|c| c.text().contains("long title"))
            .unwrap();
        assert!(oversized.is_oversized());
        assert_eq!(oversized.token_estimate(), oversized.text().chars().count());
    }
}

#[test]
fn zero_budget_and_empty_input_terminate_without_losing_nonempty_text() {
    assert!(chunker(0, 0).chunk(&[]).is_empty());
    let chunks = chunker(0, usize::MAX).chunk(&[para("a"), para("b")]);
    assert_eq!(
        chunks.iter().map(|c| c.text()).collect::<Vec<_>>(),
        ["a", "b"]
    );
    assert!(chunks.iter().all(|c| c.is_oversized()));
}
