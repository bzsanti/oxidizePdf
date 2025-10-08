//! lopdf Realistic Document Benchmark
//!
//! Generates IDENTICAL content to oxidize-pdf's realistic_document_benchmark.rs
//! for fair performance comparison.

use lopdf::{dictionary, Document, Object, Stream};
use std::env;
use std::time::Instant;

const NUM_PAGES: usize = 1000;

/// Generate realistic paragraph text (IDENTICAL to oxidize-pdf version)
fn generate_paragraph(page_num: usize, para_idx: usize) -> String {
    let topics = [
        "financial performance and quarterly revenue growth exceeded expectations",
        "strategic initiatives focused on market expansion and customer acquisition",
        "operational efficiency improvements through process automation",
        "technology infrastructure modernization and cloud migration progress",
        "employee development programs and talent retention strategies",
        "sustainability efforts and corporate social responsibility outcomes",
        "competitive market analysis and industry positioning assessment",
        "customer satisfaction metrics and service quality improvements",
        "product innovation pipeline and research development activities",
        "risk management frameworks and compliance regulatory updates",
    ];

    let contexts = [
        "According to our latest analysis",
        "Research indicates that",
        "The data clearly demonstrates",
        "Industry experts suggest that",
        "Our findings reveal that",
        "Evidence shows that",
        "Recent studies confirm that",
        "Market trends indicate that",
        "Performance metrics demonstrate",
        "Strategic analysis suggests",
    ];

    let outcomes = [
        "resulting in significant competitive advantages and market differentiation.",
        "leading to improved stakeholder value and long-term sustainable growth.",
        "creating opportunities for expansion into new market segments.",
        "enabling enhanced operational capabilities and service delivery.",
        "fostering innovation and driving organizational transformation.",
        "supporting strategic objectives and business continuity planning.",
        "strengthening our position in key markets and customer segments.",
        "delivering measurable improvements across all performance indicators.",
        "establishing best practices and industry-leading benchmarks.",
        "generating substantial returns on investment and shareholder value.",
    ];

    let topic_idx = (page_num * 7 + para_idx * 3) % topics.len();
    let context_idx = (page_num * 11 + para_idx * 5) % contexts.len();
    let outcome_idx = (page_num * 13 + para_idx * 7) % outcomes.len();

    format!(
        "{}, {} with quantifiable metrics showing {}% improvement, {}",
        contexts[context_idx],
        topics[topic_idx],
        5 + ((page_num + para_idx) * 7) % 45,
        outcomes[outcome_idx]
    )
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let page_count = if args.len() > 1 {
        args[1].parse().unwrap_or(NUM_PAGES)
    } else {
        NUM_PAGES
    };

    println!("🚀 lopdf Realistic Document Benchmark");
    println!("   Pages to generate: {}", page_count);
    println!("   Content: Varied paragraphs (identical to oxidize-pdf)\n");

    let start_time = Instant::now();

    // Create document
    let mut doc = Document::with_version("1.5");

    let pages_id = doc.new_object_id();

    // Add Helvetica font
    let font_id = doc.add_object(dictionary! {
        "Type" => "Font",
        "Subtype" => "Type1",
        "BaseFont" => "Helvetica",
    });

    let mut page_ids = Vec::new();

    // Generate pages
    for page_num in 0..page_count {
        let mut content = String::new();

        // Title
        content.push_str("BT\n");
        content.push_str("/F1 14 Tf\n");
        content.push_str("50 800 Td\n");
        content.push_str(&format!("(Realistic Business Report - Page {}) Tj\n", page_num + 1));
        content.push_str("ET\n");

        // Add 5 paragraphs (matching oxidize-pdf)
        content.push_str("BT\n");
        content.push_str("/F1 10 Tf\n");

        for para_idx in 0..5 {
            let y_pos = 750 - (para_idx * 100);
            let text = generate_paragraph(page_num, para_idx);

            // PDF text escaping
            let escaped = text.replace('\\', "\\\\")
                             .replace('(', "\\(")
                             .replace(')', "\\)");

            content.push_str(&format!("50 {} Td\n", y_pos));
            content.push_str(&format!("({}) Tj\n", escaped));
        }
        content.push_str("ET\n");

        // Simple chart (5 bars - matching oxidize-pdf complexity)
        content.push_str("0.27 0.51 0.71 rg\n"); // Blue color
        for i in 0..5 {
            let height = ((page_num + i) * 13) % 100 + 20;
            let x = 100 + i * 80;
            content.push_str(&format!("{} 300 60 {} re f\n", x, height));
        }

        // Create content stream
        let content_id = doc.add_object(Stream::new(
            dictionary! {},
            content.into_bytes(),
        ));

        // Create page object
        let page_id = doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "Contents" => content_id,
            "Resources" => dictionary! {
                "Font" => dictionary! {
                    "F1" => font_id,
                },
            },
            "MediaBox" => vec![0.into(), 0.into(), 595.into(), 842.into()],
        });

        page_ids.push(page_id);

        // Progress indicator
        if (page_num + 1) % 100 == 0 {
            print!(".");
            std::io::Write::flush(&mut std::io::stdout()).ok();
        }
    }

    println!();

    // Create Pages dictionary
    let pages_dict = dictionary! {
        "Type" => "Pages",
        "Count" => page_count as i64,
        "Kids" => page_ids.into_iter().map(Object::Reference).collect::<Vec<_>>(),
    };
    doc.objects.insert(pages_id, Object::Dictionary(pages_dict));

    // Create Catalog
    let catalog_id = doc.add_object(dictionary! {
        "Type" => "Catalog",
        "Pages" => pages_id,
    });

    doc.trailer.set("Root", catalog_id);

    // Write to buffer
    let mut buffer = Vec::new();
    doc.save_to(&mut buffer).expect("Failed to save PDF");

    let duration = start_time.elapsed();
    let pages_per_sec = page_count as f64 / duration.as_secs_f64();

    // Results
    println!("\n📊 lopdf Results:");
    println!("   Pages generated: {}", page_count);
    println!("   Time elapsed:    {:.2}s", duration.as_secs_f64());
    println!("   Performance:     {:.2} pages/second", pages_per_sec);
    println!("   File size:       {} bytes ({:.2} MB)",
             buffer.len(),
             buffer.len() as f64 / 1024.0 / 1024.0);
    println!("   Avg per page:    {} bytes", buffer.len() / page_count);

    // Save to file
    std::fs::create_dir_all("results").ok();
    std::fs::write("results/lopdf_realistic.pdf", &buffer)
        .expect("Failed to write PDF file");

    println!("\n✅ PDF saved to: results/lopdf_realistic.pdf");

    // Save results as JSON
    let results = serde_json::json!({
        "library": "lopdf",
        "benchmark": "realistic_document",
        "pages": page_count,
        "duration_secs": duration.as_secs_f64(),
        "pages_per_second": pages_per_sec,
        "file_size_bytes": buffer.len(),
        "bytes_per_page": buffer.len() / page_count,
    });

    std::fs::write(
        "results/lopdf_realistic.json",
        serde_json::to_string_pretty(&results).unwrap()
    ).ok();
}
