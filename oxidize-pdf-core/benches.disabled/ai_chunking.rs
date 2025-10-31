use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use oxidize_pdf::ai::DocumentChunker;

fn generate_document(pages: usize, words_per_page: usize) -> Vec<(usize, String)> {
    (1..=pages)
        .map(|page_num| {
            let words: Vec<String> = (0..words_per_page).map(|i| format!("word{}", i)).collect();
            (page_num, words.join(" "))
        })
        .collect()
}

fn benchmark_chunking_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("chunking_scaling");

    for num_pages in [1, 10, 50, 100, 500].iter() {
        let page_texts = generate_document(*num_pages, 200);
        let chunker = DocumentChunker::new(512, 50);

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}_pages", num_pages)),
            num_pages,
            |b, _| {
                b.iter(|| {
                    chunker
                        .chunk_text_with_pages(black_box(&page_texts))
                        .unwrap()
                });
            },
        );
    }

    group.finish();
}

fn benchmark_chunk_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("chunk_sizes");
    let page_texts = generate_document(100, 200);

    for chunk_size in [256, 512, 1024, 2048].iter() {
        let chunker = DocumentChunker::new(*chunk_size, 50);

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}_tokens", chunk_size)),
            chunk_size,
            |b, _| {
                b.iter(|| {
                    chunker
                        .chunk_text_with_pages(black_box(&page_texts))
                        .unwrap()
                });
            },
        );
    }

    group.finish();
}

fn benchmark_overlap_impact(c: &mut Criterion) {
    let mut group = c.benchmark_group("overlap_impact");
    let page_texts = generate_document(100, 200);

    for overlap in [0, 25, 50, 100, 200].iter() {
        let chunker = DocumentChunker::new(512, *overlap);

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}_overlap", overlap)),
            overlap,
            |b, _| {
                b.iter(|| {
                    chunker
                        .chunk_text_with_pages(black_box(&page_texts))
                        .unwrap()
                });
            },
        );
    }

    group.finish();
}

fn benchmark_sentence_boundaries(c: &mut Criterion) {
    let mut group = c.benchmark_group("sentence_boundaries");

    // Document with sentences
    let page_texts: Vec<(usize, String)> = (1..=100)
        .map(|page_num| {
            let sentences: Vec<String> = (0..10)
                .map(|i| format!("This is sentence number {} on page {}.", i, page_num))
                .collect();
            (page_num, sentences.join(" "))
        })
        .collect();

    let chunker = DocumentChunker::new(512, 50);

    group.bench_function("with_sentences", |b| {
        b.iter(|| {
            chunker
                .chunk_text_with_pages(black_box(&page_texts))
                .unwrap()
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    benchmark_chunking_scaling,
    benchmark_chunk_sizes,
    benchmark_overlap_impact,
    benchmark_sentence_boundaries
);

criterion_main!(benches);
