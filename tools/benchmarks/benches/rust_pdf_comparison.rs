use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use oxidize_pdf_benchmarks::*;
use std::path::Path;
use tempfile::tempdir;

// oxidize-pdf implementation
fn create_simple_pdf_oxidize(pages: usize, output_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    use oxidize_pdf::{Document, Page, PageFormat, text::Font, graphics::Color};
    
    let mut doc = Document::new(PageFormat::A4);
    
    for i in 1..=pages {
        let mut page = Page::new(PageFormat::A4);
        page.add_text_simple(
            &format!("Page {} - oxidize-pdf performance test", i),
            50.0,
            750.0,
            Font::Helvetica,
            12.0,
            Color::black(),
        )?;
        
        page.add_text_simple(
            "Lorem ipsum dolor sit amet, consectetur adipiscing elit.",
            50.0,
            730.0,
            Font::Helvetica,
            10.0,
            Color::black(),
        )?;
        
        doc.add_page(page);
    }
    
    doc.save(output_path)?;
    Ok(())
}

// lopdf implementation
fn create_simple_pdf_lopdf(pages: usize, output_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    use lopdf::{Document, Object, Stream, dictionary};
    
    let mut doc = Document::with_version("1.4");
    
    for i in 1..=pages {
        let page_id = doc.new_object_id();
        let font_id = doc.new_object_id();
        let resources_id = doc.new_object_id();
        let contents_id = doc.new_object_id();
        
        // Page content stream
        let content = format!(
            "BT /F1 12 Tf 50 750 Td (Page {} - lopdf performance test) Tj 0 -20 Td (Lorem ipsum dolor sit amet, consectetur adipiscing elit.) Tj ET",
            i
        );
        
        doc.objects.insert(contents_id, Object::Stream(Stream::new(
            dictionary! {},
            content.into_bytes(),
        )));
        
        // Font
        doc.objects.insert(font_id, Object::Dictionary(dictionary! {
            "Type" => "Font",
            "Subtype" => "Type1",
            "BaseFont" => "Helvetica",
        }));
        
        // Resources
        doc.objects.insert(resources_id, Object::Dictionary(dictionary! {
            "Font" => dictionary! {
                "F1" => font_id,
            },
        }));
        
        // Page
        doc.objects.insert(page_id, Object::Dictionary(dictionary! {
            "Type" => "Page",
            "Parent" => doc.page_tree_id,
            "Resources" => resources_id,
            "MediaBox" => vec![0.into(), 0.into(), 595.into(), 842.into()],
            "Contents" => contents_id,
        }));
        
        doc.get_pages_mut().insert(0, page_id);
    }
    
    doc.save(output_path)?;
    Ok(())
}

// printpdf implementation  
fn create_simple_pdf_printpdf(pages: usize, output_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    use printpdf::{PdfDocument, Mm, builtin_fonts::HELVETICA};
    
    let (doc, page1, layer1) = PdfDocument::new("PDF Comparison", Mm(210.0), Mm(297.0), "Layer 1");
    let mut current_layer = doc.get_page(page1).get_layer(layer1);
    
    // Add first page content
    let font = doc.add_builtin_font(HELVETICA)?;
    current_layer.use_text(format!("Page 1 - printpdf performance test"), 12.0, Mm(50.0), Mm(250.0), &font);
    current_layer.use_text("Lorem ipsum dolor sit amet, consectetur adipiscing elit.", 10.0, Mm(50.0), Mm(240.0), &font);
    
    // Add additional pages
    for i in 2..=pages {
        let (page_index, layer_index) = doc.add_page(Mm(210.0), Mm(297.0), "Layer 1");
        let current_layer = doc.get_page(page_index).get_layer(layer_index);
        
        current_layer.use_text(format!("Page {} - printpdf performance test", i), 12.0, Mm(50.0), Mm(250.0), &font);
        current_layer.use_text("Lorem ipsum dolor sit amet, consectetur adipiscing elit.", 10.0, Mm(50.0), Mm(240.0), &font);
    }
    
    doc.save(&mut std::io::BufWriter::new(std::fs::File::create(output_path)?))?;
    Ok(())
}

// Benchmark simple PDF creation
fn bench_simple_pdf_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("simple_pdf_creation");
    
    for pages in [1, 10, 50, 100].iter() {
        group.throughput(Throughput::Elements(*pages as u64));
        
        // oxidize-pdf benchmark
        group.bench_with_input(
            BenchmarkId::new("oxidize-pdf", pages),
            pages,
            |b, &pages| {
                b.iter(|| {
                    let dir = tempdir().unwrap();
                    let file_path = dir.path().join("test_oxidize.pdf");
                    black_box(create_simple_pdf_oxidize(pages, &file_path).unwrap());
                });
            },
        );
        
        // lopdf benchmark
        group.bench_with_input(
            BenchmarkId::new("lopdf", pages),
            pages,
            |b, &pages| {
                b.iter(|| {
                    let dir = tempdir().unwrap();
                    let file_path = dir.path().join("test_lopdf.pdf");
                    black_box(create_simple_pdf_lopdf(pages, &file_path).unwrap());
                });
            },
        );
        
        // printpdf benchmark
        group.bench_with_input(
            BenchmarkId::new("printpdf", pages),
            pages,
            |b, &pages| {
                b.iter(|| {
                    let dir = tempdir().unwrap();
                    let file_path = dir.path().join("test_printpdf.pdf");
                    black_box(create_simple_pdf_printpdf(pages, &file_path).unwrap());
                });
            },
        );
    }
    
    group.finish();
}

// Benchmark file size efficiency
fn bench_file_size_efficiency(c: &mut Criterion) {
    let mut group = c.benchmark_group("file_size_efficiency");
    
    let pages = 10;
    group.throughput(Throughput::Elements(pages));
    
    group.bench_function("oxidize-pdf_file_size", |b| {
        b.iter(|| {
            let dir = tempdir().unwrap();
            let file_path = dir.path().join("size_test_oxidize.pdf");
            create_simple_pdf_oxidize(pages as usize, &file_path).unwrap();
            black_box(get_file_size(file_path.to_str().unwrap()).unwrap());
        });
    });
    
    group.bench_function("lopdf_file_size", |b| {
        b.iter(|| {
            let dir = tempdir().unwrap();
            let file_path = dir.path().join("size_test_lopdf.pdf");
            create_simple_pdf_lopdf(pages as usize, &file_path).unwrap();
            black_box(get_file_size(file_path.to_str().unwrap()).unwrap());
        });
    });
    
    group.bench_function("printpdf_file_size", |b| {
        b.iter(|| {
            let dir = tempdir().unwrap();
            let file_path = dir.path().join("size_test_printpdf.pdf");
            create_simple_pdf_printpdf(pages as usize, &file_path).unwrap();
            black_box(get_file_size(file_path.to_str().unwrap()).unwrap());
        });
    });
    
    group.finish();
}

// Memory usage benchmark
fn bench_memory_efficiency(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_efficiency");
    
    for pages in [100, 500, 1000].iter() {
        group.throughput(Throughput::Elements(*pages as u64));
        
        group.bench_with_input(
            BenchmarkId::new("oxidize-pdf_memory", pages),
            pages,
            |b, &pages| {
                b.iter(|| {
                    let dir = tempdir().unwrap();
                    let file_path = dir.path().join("memory_test_oxidize.pdf");
                    
                    // Measure memory usage would require external tools
                    // For now, just measure time with large documents
                    black_box(create_simple_pdf_oxidize(pages, &file_path).unwrap());
                });
            },
        );
        
        group.bench_with_input(
            BenchmarkId::new("lopdf_memory", pages),
            pages,
            |b, &pages| {
                b.iter(|| {
                    let dir = tempdir().unwrap();
                    let file_path = dir.path().join("memory_test_lopdf.pdf");
                    black_box(create_simple_pdf_lopdf(pages, &file_path).unwrap());
                });
            },
        );
        
        group.bench_with_input(
            BenchmarkId::new("printpdf_memory", pages),
            pages,
            |b, &pages| {
                b.iter(|| {
                    let dir = tempdir().unwrap();
                    let file_path = dir.path().join("memory_test_printpdf.pdf");
                    black_box(create_simple_pdf_printpdf(pages, &file_path).unwrap());
                });
            },
        );
    }
    
    group.finish();
}

criterion_group!(
    benches,
    bench_simple_pdf_creation,
    bench_file_size_efficiency,
    bench_memory_efficiency,
);

criterion_main!(benches);