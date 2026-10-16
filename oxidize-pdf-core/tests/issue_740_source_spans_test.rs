use oxidize_pdf::ai::DocumentChunker;

#[test]
fn sparse_page_ids_are_not_synthesized() {
    let chunks = DocumentChunker::new(1, 0)
        .chunk_text_with_pages(&[(1, "a".into()), (5, "b".into())])
        .unwrap();
    assert_eq!(chunks.len(), 2);
    assert_eq!(chunks[0].content, "a");
    assert_eq!(chunks[0].page_numbers, [1]);
    assert_eq!(chunks[1].content, "b");
    assert_eq!(chunks[1].page_numbers, [5]);
    assert_eq!(chunks[1].metadata.position.first_page, 5);
    assert_eq!(chunks[1].metadata.position.last_page, 5);
}

#[test]
fn repeated_overlap_keeps_third_page_provenance() {
    let chunks = DocumentChunker::new(50, 25)
        .chunk_text_with_pages(&[
            (1, "one ".repeat(200)),
            (2, "two ".repeat(200)),
            (3, "three ".repeat(200)),
        ])
        .unwrap();
    let last = chunks.last().unwrap();
    assert!(last.content.split_whitespace().all(|w| w == "three"));
    assert_eq!(last.tokens, 50);
    assert_eq!(last.page_numbers, [3]);
    assert_eq!(last.metadata.position.first_page, 3);
    assert_eq!(last.metadata.position.last_page, 3);
    assert_eq!(last.metadata.position.start_char, 2504);
    assert_eq!(last.metadata.position.end_char, 2803);
    assert!(chunks.iter().any(|c| c.page_numbers == [1, 2]));
    assert!(chunks.iter().any(|c| c.page_numbers == [2, 3]));
}

#[test]
fn unicode_whitespace_offsets_refer_to_original_characters() {
    let source = "  é\t猫  beta\n\nγ ";
    let chunks = DocumentChunker::new(2, 1).chunk_text(source).unwrap();
    let expected = [("é 猫", 2, 5), ("猫 beta", 4, 11), ("beta γ", 7, 14)];
    assert_eq!(chunks.len(), expected.len());
    for (chunk, (content, start, end)) in chunks.iter().zip(expected) {
        assert_eq!(chunk.content, content);
        assert_eq!(
            (
                chunk.metadata.position.start_char,
                chunk.metadata.position.end_char
            ),
            (start, end)
        );
        assert!(chunk.page_numbers.is_empty());
        assert_eq!(chunk.metadata.position.first_page, 0);
        assert_eq!(chunk.metadata.position.last_page, 0);
    }
}

#[test]
fn crossing_pages_uses_supplied_order_and_skips_empty_pages() {
    let pages = [
        (9, "  α\tβ ".into()),
        (42, " \n".into()),
        (3, "猫  dog".into()),
    ];
    for overlap in [0, 1] {
        let chunks = DocumentChunker::new(3, overlap)
            .chunk_text_with_pages(&pages)
            .unwrap();
        assert_eq!(chunks[0].content, "α β 猫");
        assert_eq!(chunks[0].page_numbers, [9, 3]);
        assert_eq!(chunks[0].metadata.position.first_page, 9);
        assert_eq!(chunks[0].metadata.position.last_page, 3);
        assert_eq!(
            (
                chunks[0].metadata.position.start_char,
                chunks[0].metadata.position.end_char
            ),
            (2, 13)
        );
        let last = chunks.last().unwrap();
        assert_eq!(last.page_numbers, [3]);
        assert_eq!(last.metadata.position.end_char, 18);
        assert_eq!(last.content, if overlap == 0 { "dog" } else { "猫 dog" });
    }
}

#[test]
fn source_slices_reconstruct_chunks_across_sentence_boundaries() {
    let source = "  Alpha beta.\n Γάμμα\u{2003}猫!   delta epsilon zeta eta theta.";
    for overlap in [0, 2] {
        let chunks = DocumentChunker::new(4, overlap).chunk_text(source).unwrap();
        for chunk in &chunks {
            let p = &chunk.metadata.position;
            let original: String = source
                .chars()
                .skip(p.start_char)
                .take(p.end_char - p.start_char)
                .collect();
            assert_eq!(
                original.split_whitespace().collect::<Vec<_>>().join(" "),
                chunk.content
            );
        }
        assert!(chunks
            .iter()
            .any(|c| c.metadata.sentence_boundary_respected));
        assert!(chunks.last().unwrap().content.ends_with("theta."));
    }
}

#[test]
fn empty_and_plain_text_controls() {
    let chunker = DocumentChunker::new(10, 0);
    assert!(chunker.chunk_text_with_pages(&[]).unwrap().is_empty());
    assert!(chunker
        .chunk_text_with_pages(&[(7, " \t\n".into())])
        .unwrap()
        .is_empty());
    let chunks = chunker.chunk_text("alpha beta").unwrap();
    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0].content, "alpha beta");
    assert_eq!(chunks[0].tokens, 2);
    assert_eq!(chunks[0].metadata.position.start_char, 0);
    assert_eq!(chunks[0].metadata.position.end_char, 10);
}

#[test]
fn empty_pages_still_count_in_source_coordinates() {
    let chunks = DocumentChunker::new(2, 0)
        .chunk_text_with_pages(&[(1, String::new()), (2, String::new()), (8, "é".into())])
        .unwrap();
    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0].content, "é");
    assert_eq!(chunks[0].page_numbers, [8]);
    assert_eq!(
        (
            chunks[0].metadata.position.start_char,
            chunks[0].metadata.position.end_char
        ),
        (4, 5)
    );
}
