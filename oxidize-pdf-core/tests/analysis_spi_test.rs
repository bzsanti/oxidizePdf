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
