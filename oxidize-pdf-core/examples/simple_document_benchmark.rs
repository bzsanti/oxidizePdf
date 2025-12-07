use oxidize_pdf::{Document, Font, Page, Result};
use std::env;
use std::time::Instant;

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    let page_count = if args.len() > 1 {
        args[1].parse().unwrap_or(100)
    } else {
        100
    };

    let start_time = Instant::now();

    let mut doc = Document::new();
    doc.set_title("Simple Document Benchmark - Realistic Content");

    // Contenido realista para cada página
    let lorem_paragraphs = [
        "Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat. Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur.",

        "Excepteur sint occaecat cupidatat non proident, sunt in culpa qui officia deserunt mollit anim id est laborum. Sed ut perspiciatis unde omnis iste natus error sit voluptatem accusantium doloremque laudantium, totam rem aperiam, eaque ipsa quae ab illo inventore veritatis et quasi architecto beatae vitae dicta sunt explicabo.",

        "Nemo enim ipsam voluptatem quia voluptas sit aspernatur aut odit aut fugit, sed quia consequuntur magni dolores eos qui ratione voluptatem sequi nesciunt. Neque porro quisquam est, qui dolorem ipsum quia dolor sit amet, consectetur, adipisci velit, sed quia non numquam eius modi tempora incidunt ut labore et dolore magnam aliquam quaerat voluptatem.",

        "At vero eos et accusamus et iusto odio dignissimos ducimus qui blanditiis praesentium voluptatum deleniti atque corrupti quos dolores et quas molestias excepturi sint occaecati cupiditate non provident, similique sunt in culpa qui officia deserunt mollitia animi, id est laborum et dolorum fuga."
    ];

    for i in 0..page_count {
        let mut page = Page::a4();
        let mut y_pos = 750.0;

        // Header con título y número de página
        page.text()
            .set_font(Font::HelveticaBold, 16.0)
            .at(50.0, y_pos)
            .write(&format!(
                "Document Section {} - Page {}",
                (i / 10) + 1,
                i + 1
            ))?;

        y_pos -= 30.0;

        // Línea separadora simulada con texto
        page.text()
            .set_font(Font::Helvetica, 10.0)
            .at(50.0, y_pos)
            .write("_".repeat(80).as_str())?;

        y_pos -= 40.0;

        // Contenido principal - múltiples párrafos con diferentes fonts
        for (j, paragraph) in lorem_paragraphs.iter().enumerate() {
            // Título del párrafo
            page.text()
                .set_font(Font::HelveticaBold, 12.0)
                .at(50.0, y_pos)
                .write(&format!("Paragraph {}", j + 1))?;

            y_pos -= 20.0;

            // Contenido del párrafo - dividido en líneas para simular wrapping
            let words: Vec<&str> = paragraph.split_whitespace().collect();
            let mut line = String::new();
            let words_per_line = 12;

            for (word_idx, word) in words.iter().enumerate() {
                line.push_str(word);
                line.push(' ');

                if (word_idx + 1) % words_per_line == 0 || word_idx == words.len() - 1 {
                    page.text()
                        .set_font(Font::Helvetica, 10.0)
                        .at(70.0, y_pos)
                        .write(line.trim())?;

                    y_pos -= 15.0;
                    line.clear();

                    if y_pos < 100.0 {
                        break; // Avoid going off page
                    }
                }
            }

            y_pos -= 10.0;

            if y_pos < 100.0 {
                break; // Page is full
            }
        }

        // Footer
        if y_pos > 50.0 {
            page.text()
                .set_font(Font::Courier, 8.0)
                .at(50.0, 50.0)
                .write(&format!(
                    "Page {} of {} | oxidize-pdf Simple Document Benchmark",
                    i + 1,
                    page_count
                ))?;
        }

        doc.add_page(page);
    }

    let generation_time = start_time.elapsed();

    // Tiempo de escritura separado
    let write_start = Instant::now();
    doc.save("examples/results/simple_document_benchmark.pdf")?;
    let write_time = write_start.elapsed();

    let total_time = start_time.elapsed();

    // Output parseable para el benchmark
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
