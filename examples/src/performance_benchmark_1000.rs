use oxidize_pdf::{Document, Font, Page, Result};
use std::env;
use std::time::Instant;

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    let page_count = if args.len() > 1 {
        args[1].parse().unwrap_or(1000)
    } else {
        1000
    };

    let start_time = Instant::now();

    let mut doc = Document::new();
    doc.set_title("Performance Benchmark");

    for i in 0..page_count {
        let mut page = Page::a4();

        // Texto mínimo pero representativo
        page.text()
            .set_font(Font::Helvetica, 12.0)
            .at(50.0, 750.0)
            .write(&format!("Page {} of {}", i + 1, page_count))?;

        page.text()
            .set_font(Font::Helvetica, 10.0)
            .at(50.0, 700.0)
            .write("Lorem ipsum dolor sit amet, consectetur adipiscing elit.")?;

        doc.add_page(page);
    }

    let generation_time = start_time.elapsed();

    // Tiempo de escritura separado
    let write_start = Instant::now();
    doc.save("examples/results/performance_benchmark_1000.pdf")?;
    let write_time = write_start.elapsed();

    let total_time = start_time.elapsed();

    // Output simple para parsing
    println!("PAGES={}", page_count);
    println!("GENERATION_MS={}", generation_time.as_millis());
    println!("WRITE_MS={}", write_time.as_millis());
    println!("TOTAL_MS={}", total_time.as_millis());
    println!(
        "PAGES_PER_SEC={:.2}",
        page_count as f64 / total_time.as_secs_f64()
    );

    Ok(())
}
