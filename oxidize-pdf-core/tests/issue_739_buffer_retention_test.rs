use oxidize_pdf::streaming::{stream_text, TextStreamOptions, TextStreamer};

fn streamer(limit: usize) -> TextStreamer {
    TextStreamer::new(TextStreamOptions {
        max_buffer_size: limit,
        sort_by_position: false,
        ..Default::default()
    })
}

fn feed(streamer: &mut TextStreamer, text: &str) -> String {
    let content = format!("BT /F1 12 Tf ({text}) Tj ET");
    let returned = streamer.process_chunk(content.as_bytes()).unwrap();
    assert_eq!(returned.len(), 1);
    returned[0].text.clone()
}

#[test]
fn retains_newest_chunk_from_issue_reproduction() {
    let mut s = streamer(10);
    assert_eq!(feed(&mut s, "aaaaaa"), "aaaaaa");
    assert_eq!(feed(&mut s, "bbbbbb"), "bbbbbb");
    assert_eq!(s.extract_text(), "bbbbbb");
}

#[test]
fn below_exact_and_over_limit_history() {
    let mut s = streamer(10);
    assert_eq!(feed(&mut s, "aaa"), "aaa");
    assert_eq!(s.extract_text(), "aaa");
    assert_eq!(feed(&mut s, "bbb"), "bbb");
    assert_eq!(feed(&mut s, "cccc"), "cccc");
    assert_eq!(s.extract_text(), "aaa bbb cccc");
    assert_eq!(feed(&mut s, "dd"), "dd");
    assert_eq!(s.extract_text(), "bbb cccc dd");
    assert_eq!(feed(&mut s, "eeeeee"), "eeeeee");
    assert_eq!(s.extract_text(), "dd eeeeee");
    s.clear_buffer();
    assert!(s.get_buffered_chunks().is_empty());
    assert_eq!(feed(&mut s, "new"), "new");
    assert_eq!(s.extract_text(), "new");
}

#[test]
fn oversized_and_zero_limits_do_not_suppress_returned_text() {
    for limit in [0, 3] {
        let mut s = streamer(limit);
        assert_eq!(feed(&mut s, "oversized"), "oversized");
        assert!(s.get_buffered_chunks().is_empty());
    }
    let mut s = streamer(3);
    assert_eq!(feed(&mut s, "old"), "old");
    assert_eq!(feed(&mut s, "oversized"), "oversized");
    assert!(s.get_buffered_chunks().is_empty());
    assert_eq!(feed(&mut s, "ok"), "ok");
    assert_eq!(s.extract_text(), "ok");
}

#[test]
fn batch_retention_is_a_suffix_and_uses_utf8_bytes() {
    let mut s = streamer(4);
    let chunks = s
        .process_chunk("BT /F1 12 Tf (old) Tj (éé) Tj ET".as_bytes())
        .unwrap();
    assert_eq!(
        chunks.iter().map(|c| c.text.as_str()).collect::<Vec<_>>(),
        ["old", "éé"]
    );
    assert_eq!(s.extract_text(), "éé");
    assert_eq!(feed(&mut s, "z"), "z");
    assert_eq!(s.extract_text(), "z");
}

#[test]
fn callback_delivery_and_filter_controls_are_independent_of_retention() {
    let mut seen = Vec::new();
    stream_text(
        vec![
            b"BT /F1 12 Tf (aaaaaa) Tj ET".to_vec(),
            b"BT /F1 12 Tf (bbbbbb) Tj ET".to_vec(),
        ],
        |chunk| {
            seen.push(chunk.text);
            Ok(())
        },
    )
    .unwrap();
    assert_eq!(seen, ["aaaaaa", "bbbbbb"]);
    let mut filtered = TextStreamer::new(TextStreamOptions {
        min_font_size: 20.0,
        ..Default::default()
    });
    assert!(filtered
        .process_chunk(b"BT /F1 12 Tf (hidden) Tj ET")
        .unwrap()
        .is_empty());
    assert!(filtered.extract_text().is_empty());
    let mut normal = streamer(100);
    assert_eq!(feed(&mut normal, "visible"), "visible");
    assert_eq!(normal.extract_text(), "visible");
}
