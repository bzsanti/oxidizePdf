//! Integration tests for the unstable analysis SPI.
#![cfg(feature = "unstable-spi")]

use oxidize_pdf::pipeline::{ChunkGroup, ChunkingStrategy};
use oxidize_pdf::pipeline::{Element, ElementData, ElementMetadata};

/// A strategy that emits exactly one chunk per element.
struct OnePerElement;

impl ChunkingStrategy for OnePerElement {
    fn chunk(&self, elements: &[Element]) -> Vec<ChunkGroup> {
        elements
            .iter()
            .map(|e| ChunkGroup::new(vec![e.clone()], None))
            .collect()
    }
}

fn para(text: &str) -> Element {
    Element::Paragraph(ElementData {
        text: text.to_string(),
        metadata: ElementMetadata::default(),
    })
}

#[test]
fn custom_strategy_is_object_safe_and_groups_per_element() {
    let strategy: Box<dyn ChunkingStrategy> = Box::new(OnePerElement);
    let elements = vec![para("alpha"), para("bravo"), para("charlie")];
    let groups = strategy.chunk(&elements);
    assert_eq!(groups.len(), 3, "one chunk per element");
    assert_eq!(groups[0].elements.len(), 1);
    assert_eq!(groups[0].elements[0].text(), "alpha");
    assert_eq!(groups[2].elements[0].text(), "charlie");
}

use oxidize_pdf::pipeline::{HybridChunkConfig, HybridChunker, MergePolicy};

#[test]
fn hybrid_chunker_is_the_default_strategy() {
    let elements = vec![para("alpha one two three"), para("bravo four five six")];
    let chunker = HybridChunker::new(HybridChunkConfig {
        max_tokens: 4,
        overlap_tokens: 0,
        merge_adjacent: true,
        propagate_headings: true,
        merge_policy: MergePolicy::AnyInlineContent,
    });

    // Inherent API: Vec<HybridChunk>.
    let hybrid = HybridChunker::chunk(&chunker, &elements);
    // Trait API: Vec<ChunkGroup>, same grouping.
    let groups = ChunkingStrategy::chunk(&chunker, &elements);

    assert_eq!(groups.len(), hybrid.len(), "same number of chunks");
    for (g, h) in groups.iter().zip(hybrid.iter()) {
        let g_text: Vec<&str> = g.elements.iter().map(|e| e.text()).collect();
        let h_text: Vec<&str> = h.elements().iter().map(|e| e.text()).collect();
        assert_eq!(g_text, h_text, "same element grouping");
        assert_eq!(g.heading_context, h.heading_context);
    }
}
