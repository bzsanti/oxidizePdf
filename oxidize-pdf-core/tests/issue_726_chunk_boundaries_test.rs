use oxidize_pdf::streaming::chunk_processor::{
    process_in_chunks, ChunkOptions, ChunkProcessor, ChunkType, ContentChunk,
};
use std::io::{self, Read};

struct Fragmented<'a> {
    bytes: &'a [u8],
    cuts: Vec<usize>,
    offset: usize,
}
impl Read for Fragmented<'_> {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        let end = self
            .cuts
            .iter()
            .copied()
            .find(|&n| n > self.offset)
            .unwrap_or(self.bytes.len());
        let n = out.len().min(end - self.offset);
        out[..n].copy_from_slice(&self.bytes[self.offset..self.offset + n]);
        self.offset += n;
        Ok(n)
    }
}
fn run(input: &[u8], cuts: Vec<usize>, options: ChunkOptions) -> Vec<ContentChunk> {
    let mut chunks = Vec::new();
    process_in_chunks(
        Fragmented {
            bytes: input,
            cuts,
            offset: 0,
        },
        options,
        |c| {
            chunks.push(c);
            Ok(())
        },
    )
    .unwrap();
    chunks
}
fn bytes(chunks: &[ContentChunk]) -> Vec<u8> {
    chunks.iter().flat_map(|c| c.data.iter().copied()).collect()
}

#[test]
fn sentinel_survives_every_buffer_size() {
    let input = b"BT /F1 12 Tf (SENTINEL) Tj ET";
    for size in [1, 2, 3, 7, 16, 4096] {
        let chunks = run(
            input,
            vec![],
            ChunkOptions {
                buffer_size: size,
                ..Default::default()
            },
        );
        assert_eq!(bytes(&chunks), input, "buffer {size}");
    }
}
#[test]
fn all_split_positions_preserve_bytes_types_and_offsets() {
    let input=b"q\nBT /F1 12 Tf (Tj \\( escaped \\) (nested) ET) Tj [<4254> -20 (x)] TJ ET\n1 2 m 3 4 l S Q\n% BT fake\n";
    let options = ChunkOptions {
        max_chunk_size: 7,
        ..Default::default()
    };
    let baseline = run(input, vec![], options.clone());
    assert_eq!(bytes(&baseline), input);
    for split in 1..input.len() {
        let chunks = run(input, vec![split], options.clone());
        assert_eq!(bytes(&chunks), input, "split {split}");
        let mut position = 0;
        for c in &chunks {
            assert_eq!(c.position, position);
            assert_eq!(c.size, c.data.len());
            assert!(c.size <= 7);
            position += c.size as u64;
        }
        assert_eq!(
            chunks
                .iter()
                .map(|c| (&c.chunk_type, &c.data, c.position))
                .collect::<Vec<_>>(),
            baseline
                .iter()
                .map(|c| (&c.chunk_type, &c.data, c.position))
                .collect::<Vec<_>>()
        );
    }
}
#[test]
fn explicit_text_filter_keeps_complete_text_not_keyword_lookalikes() {
    let input = b"1 2 m\nBT (SENTINEL Tj ET m) Tj ET\n3 4 l S";
    let expected = b"\nBT (SENTINEL Tj ET m) Tj ET";
    for size in [1, 2, 5, 4096] {
        let chunks = run(
            input,
            vec![],
            ChunkOptions {
                buffer_size: size,
                max_chunk_size: 3,
                chunk_types: vec![ChunkType::Text],
                ..Default::default()
            },
        );
        assert_eq!(bytes(&chunks), expected, "buffer {size}");
        assert!(chunks.iter().all(|c| c.is_text()));
    }
    let chunks = run(
        b"/TjName (BT Tj m) bogus",
        vec![],
        ChunkOptions {
            chunk_types: vec![ChunkType::Text],
            ..Default::default()
        },
    );
    assert!(chunks.is_empty());
}
#[test]
fn default_preserves_unknown_and_binary_bytes() {
    for input in [
        b"xyz123\x00\xff\xfe".as_slice(),
        b"\xff\xd8\xff\xe0BT ET Tj m\x00",
        b"\x89PNG\r\n\x1a\nBT Tj",
    ] {
        for size in [1, 2, 4096] {
            assert_eq!(
                bytes(&run(
                    input,
                    vec![],
                    ChunkOptions {
                        buffer_size: size,
                        max_chunk_size: 2,
                        ..Default::default()
                    }
                )),
                input
            );
        }
    }
}
#[test]
fn invalid_direct_options_are_errors() {
    for options in [
        ChunkOptions {
            max_chunk_size: 0,
            ..Default::default()
        },
        ChunkOptions {
            buffer_size: 0,
            ..Default::default()
        },
    ] {
        assert!(ChunkProcessor::new(options)
            .process_content(b"BT (x) Tj ET")
            .is_err());
    }
}

#[test]
fn incomplete_input_and_comments_are_preserved_by_default() {
    for input in [
        b"BT (unfinished \\".as_slice(),
        b"% BT Tj m no newline",
        b"[ /Tj (BT) <4254>",
        b"<< /Key (m Tj) >> /X Do\r\n",
    ] {
        for size in [1, 3, 64] {
            assert_eq!(
                bytes(&run(
                    input,
                    vec![],
                    ChunkOptions {
                        buffer_size: size,
                        max_chunk_size: 5,
                        ..Default::default()
                    }
                )),
                input
            );
        }
    }
}

#[test]
fn inline_image_payload_is_never_interpreted_as_operators() {
    let input =
        b"q BI /W 1 /H 1 /BPC 8 /CS /RGB ID \x00 EI BT (fake) Tj ET \xff EI Q BT (real) Tj ET";
    for split in 1..input.len() {
        let chunks = run(
            input,
            vec![split],
            ChunkOptions {
                buffer_size: 3,
                max_chunk_size: 7,
                ..Default::default()
            },
        );
        assert_eq!(bytes(&chunks), input);
        assert!(!chunks.iter().any(|c| c.is_text()));
    }
    let result = process_in_chunks(
        &input[..],
        ChunkOptions {
            chunk_types: vec![ChunkType::Text],
            ..Default::default()
        },
        |_| Ok(()),
    );
    assert!(
        matches!(result,Err(oxidize_pdf::PdfError::InvalidStructure(message)) if message.contains("inline image"))
    );
}

#[test]
fn splitting_disabled_never_splits_an_operation_silently() {
    let options = ChunkOptions {
        max_chunk_size: 8,
        split_large_objects: false,
        ..Default::default()
    };
    assert!(process_in_chunks(&b"BT (long string) Tj ET"[..], options, |_| Ok(())).is_err());
    let options = ChunkOptions {
        max_chunk_size: 8,
        split_large_objects: false,
        ..Default::default()
    };
    let chunks = run(b"1 2 m 3 4 l S", vec![], options);
    assert_eq!(bytes(&chunks), b"1 2 m 3 4 l S");
    assert!(chunks.iter().all(|c| c.size <= 8));
    assert!(chunks.iter().any(|c| c.data.ends_with(b"m")));
}

#[test]
fn interrupted_reads_retry_and_callback_failure_stops_processing() {
    struct Interrupted<'a> {
        bytes: &'a [u8],
        once: bool,
    }
    impl Read for Interrupted<'_> {
        fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
            if !self.once {
                self.once = true;
                return Err(io::ErrorKind::Interrupted.into());
            }
            self.bytes.read(out)
        }
    }
    let input = b"BT (x) Tj ET";
    let mut output = Vec::new();
    process_in_chunks(
        Interrupted {
            bytes: input,
            once: false,
        },
        ChunkOptions::default(),
        |c| {
            output.extend(c.data);
            Ok(())
        },
    )
    .unwrap();
    assert_eq!(output, input);
    let mut calls = 0;
    let result = process_in_chunks(
        &input[..],
        ChunkOptions {
            max_chunk_size: 1,
            ..Default::default()
        },
        |_| {
            calls += 1;
            Err(oxidize_pdf::PdfError::InvalidStructure("stop".into()))
        },
    );
    assert!(
        matches!(result,Err(oxidize_pdf::PdfError::InvalidStructure(message)) if message=="stop")
    );
    assert_eq!(calls, 1);
}

#[test]
fn oversized_unfinished_records_and_excessive_nesting_error() {
    let input = [b'('; 130];
    assert!(process_in_chunks(&input[..], ChunkOptions::default(), |_| Ok(())).is_err());
    let input = vec![b'a'; 16 * 1024 * 1024 + 1];
    assert!(
        matches!(process_in_chunks(&input[..],ChunkOptions::default(),|_|Ok(())),Err(oxidize_pdf::PdfError::InvalidStructure(message)) if message.contains("16 MiB"))
    );
}

#[test]
fn positions_and_page_numbers_include_deliberately_filtered_content() {
    let options = ChunkOptions {
        chunk_types: vec![ChunkType::Text],
        ..Default::default()
    };
    let mut processor = ChunkProcessor::new(options);
    processor.set_page(7);
    assert!(processor.process_content(b"1 2 m").unwrap().is_empty());
    let chunks = processor.process_content(b"BT (x) Tj ET").unwrap();
    assert_eq!(chunks[0].position, 5);
    assert_eq!(chunks[0].page_number, 7);
    assert_eq!(bytes(&chunks), b"BT (x) Tj ET");
    processor.reset();
    let chunks = processor.process_content(b"BT (x) Tj ET").unwrap();
    assert_eq!(chunks[0].position, 0);
    assert_eq!(chunks[0].page_number, 0);
}

#[test]
fn text_filter_survives_every_split_with_strings_names_and_comments() {
    let input=b"q\n% ignored marker Tj\r\nBT /TjName 12 Tf (m (nested) \\( Tj \\) ) Tj [<4254> -10 (ET)] TJ ET\nQ";
    for max_chunk_size in [1, 2, 7, 4096] {
        let options = ChunkOptions {
            max_chunk_size,
            chunk_types: vec![ChunkType::Text],
            ..Default::default()
        };
        let baseline = run(input, vec![], options.clone());
        assert_eq!(bytes(&baseline),b"\n% ignored marker Tj\r\nBT /TjName 12 Tf (m (nested) \\( Tj \\) ) Tj [<4254> -10 (ET)] TJ ET");
        for split in 1..input.len() {
            let chunks = run(input, vec![split], options.clone());
            assert_eq!(bytes(&chunks), bytes(&baseline));
            assert_eq!(
                chunks
                    .iter()
                    .map(|c| (&c.chunk_type, c.position, c.size))
                    .collect::<Vec<_>>(),
                baseline
                    .iter()
                    .map(|c| (&c.chunk_type, c.position, c.size))
                    .collect::<Vec<_>>()
            );
        }
    }
}

#[test]
fn classification_uses_complete_tokens_and_keeps_opaque_image_identity() {
    for data in [
        b"\xff\xd8\xff\xe0TjBTm".as_slice(),
        b"\x89PNG\r\n\x1a\nTjBTm",
    ] {
        let chunks = run(
            data,
            vec![],
            ChunkOptions {
                buffer_size: 1,
                max_chunk_size: 2,
                chunk_types: vec![ChunkType::Image],
                ..Default::default()
            },
        );
        assert_eq!(bytes(&chunks), data);
        assert!(chunks.iter().all(|c| c.is_image()));
    }
    for data in [b"T".as_slice(), b"/Tj (BT m) bogus", b"/X Do"] {
        let chunks = run(
            data,
            vec![],
            ChunkOptions {
                buffer_size: 1,
                ..Default::default()
            },
        );
        assert_eq!(bytes(&chunks), data);
        assert!(chunks.iter().all(|c| c.chunk_type == ChunkType::Unknown));
    }
    let data = b"1 2 m 3 4 l S";
    let chunks = run(
        data,
        vec![],
        ChunkOptions {
            buffer_size: 1,
            ..Default::default()
        },
    );
    assert_eq!(bytes(&chunks), data);
    assert!(chunks.iter().all(|c| c.chunk_type == ChunkType::Graphics));
}

#[test]
fn selecting_no_types_does_not_require_classifying_inline_binary() {
    let input = b"BI ID \xff Tj BT ET EI";
    assert!(run(
        input,
        vec![],
        ChunkOptions {
            chunk_types: vec![],
            buffer_size: 1,
            ..Default::default()
        }
    )
    .is_empty());
}
