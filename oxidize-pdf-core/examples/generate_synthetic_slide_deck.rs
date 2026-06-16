//! Generates a synthetic slide-deck PDF that reproduces the layout patterns
//! known to break the spatial-cluster table detector.
//!
//! Output path defaults to `tests/fixtures/synthetic_slide_deck.pdf` relative
//! to the workspace root; pass a path argument to override.
//!
//! Layout (16:9, 960 × 540 pt):
//!   Page 1: cover slide (title + subtitle).
//!   Page 2: 4-column "Origin / Challenge / Solution / Impact" layout with
//!           shape-grid arrangement — the worst case for the spatial table
//!           detector at `min_table_confidence = 0.5`.
//!   Page 3: prose-heavy summary slide (control: should NOT activate the
//!           table detector at any threshold).
//!   Page 4: closing slide with a single large title.
//!
//! Run:
//!   cargo run --example generate_synthetic_slide_deck -p oxidize-pdf

use chrono::{TimeZone, Utc};
use oxidize_pdf::{Document, Font, Page};
use std::env;

const PAGE_W: f64 = 960.0;
const PAGE_H: f64 = 540.0;
const DEFAULT_OUT: &str = "oxidize-pdf-core/tests/fixtures/synthetic_slide_deck.pdf";

fn cover(doc: &mut Document) -> Result<(), Box<dyn std::error::Error>> {
    let mut p = Page::new(PAGE_W, PAGE_H);
    p.text()
        .set_font(Font::HelveticaBold, 36.0)
        .at(120.0, 320.0)
        .write("Synthetic Deck — Title Slide")?;
    p.text()
        .set_font(Font::Helvetica, 18.0)
        .at(120.0, 280.0)
        .write("Generated fixture for slide-export tests")?;
    p.text()
        .set_font(Font::Helvetica, 12.0)
        .at(120.0, 60.0)
        .write("JUNE 2026")?;
    doc.add_page(p);
    Ok(())
}

fn four_column(doc: &mut Document) -> Result<(), Box<dyn std::error::Error>> {
    let mut p = Page::new(PAGE_W, PAGE_H);

    p.text()
        .set_font(Font::HelveticaBold, 22.0)
        .at(60.0, 490.0)
        .write("Slide With Four-Column Layout")?;
    p.text()
        .set_font(Font::Helvetica, 14.0)
        .at(60.0, 465.0)
        .write("Reproduces the shape-grid pattern that mis-fires the spatial table detector")?;

    let cols = [
        "Origin",
        "Current Challenge",
        "Proposed Solution",
        "Expected Impact",
    ];
    let col_xs = [60.0, 290.0, 520.0, 750.0];
    for (i, h) in cols.iter().enumerate() {
        p.text()
            .set_font(Font::HelveticaBold, 13.0)
            .at(col_xs[i], 420.0)
            .write(h)?;
    }

    let rows: [[&str; 4]; 3] = [
        [
            "Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore.",
            "Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo.",
            "Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur.",
            "Excepteur sint occaecat cupidatat non proident, sunt in culpa qui officia deserunt mollit anim.",
        ],
        [
            "Sed ut perspiciatis unde omnis iste natus error sit voluptatem accusantium doloremque.",
            "Nemo enim ipsam voluptatem quia voluptas sit aspernatur aut odit aut fugit, sed quia.",
            "Neque porro quisquam est, qui dolorem ipsum quia dolor sit amet, consectetur, adipisci.",
            "Ut enim ad minima veniam, quis nostrum exercitationem ullam corporis suscipit laboriosam.",
        ],
        [
            "At vero eos et accusamus et iusto odio dignissimos ducimus qui blanditiis praesentium.",
            "Et harum quidem rerum facilis est et expedita distinctio. Nam libero tempore.",
            "Cum soluta nobis est eligendi optio cumque nihil impedit quo minus id quod maxime.",
            "Temporibus autem quibusdam et aut officiis debitis aut rerum necessitatibus saepe eveniet.",
        ],
    ];

    let row_ys = [390.0, 280.0, 170.0];
    for (ri, row) in rows.iter().enumerate() {
        let y_top = row_ys[ri];
        for (ci, cell) in row.iter().enumerate() {
            wrap_cell_text(&mut p, cell, col_xs[ci], y_top, 210.0)?;
        }
    }

    doc.add_page(p);
    Ok(())
}

fn wrap_cell_text(
    p: &mut Page,
    text: &str,
    x: f64,
    y_top: f64,
    width: f64,
) -> Result<(), Box<dyn std::error::Error>> {
    let max_chars_per_line: usize = (width / 6.0) as usize;
    let words = text.split_whitespace().collect::<Vec<_>>();
    let mut line = String::new();
    let mut y = y_top;
    let line_height = 14.0;
    for w in words {
        if line.is_empty() {
            line.push_str(w);
        } else if line.len() + 1 + w.len() <= max_chars_per_line {
            line.push(' ');
            line.push_str(w);
        } else {
            p.text()
                .set_font(Font::Helvetica, 10.0)
                .at(x, y)
                .write(&line)?;
            y -= line_height;
            line.clear();
            line.push_str(w);
        }
    }
    if !line.is_empty() {
        p.text()
            .set_font(Font::Helvetica, 10.0)
            .at(x, y)
            .write(&line)?;
    }
    Ok(())
}

fn summary(doc: &mut Document) -> Result<(), Box<dyn std::error::Error>> {
    let mut p = Page::new(PAGE_W, PAGE_H);
    p.text()
        .set_font(Font::HelveticaBold, 22.0)
        .at(60.0, 490.0)
        .write("Summary Slide With Continuous Prose")?;
    let paragraphs = [
        "Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat.",
        "Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur. Excepteur sint occaecat cupidatat non proident, sunt in culpa qui officia deserunt mollit anim id est laborum.",
        "Sed ut perspiciatis unde omnis iste natus error sit voluptatem accusantium doloremque laudantium, totam rem aperiam, eaque ipsa quae ab illo inventore veritatis et quasi architecto beatae vitae dicta sunt explicabo.",
        "Nemo enim ipsam voluptatem quia voluptas sit aspernatur aut odit aut fugit, sed quia consequuntur magni dolores eos qui ratione voluptatem sequi nesciunt.",
    ];
    let mut y = 440.0;
    for para in paragraphs.iter() {
        wrap_cell_text(&mut p, para, 60.0, y, 840.0)?;
        let lines = ((para.len() as f64) / ((840.0 / 6.0) as f64))
            .ceil()
            .max(1.0);
        y -= lines * 14.0 + 8.0;
    }
    doc.add_page(p);
    Ok(())
}

fn closing(doc: &mut Document) -> Result<(), Box<dyn std::error::Error>> {
    let mut p = Page::new(PAGE_W, PAGE_H);
    p.text()
        .set_font(Font::HelveticaBold, 48.0)
        .at(330.0, 280.0)
        .write("Closing Slide")?;
    p.text()
        .set_font(Font::Helvetica, 14.0)
        .at(400.0, 240.0)
        .write("End of synthetic fixture")?;
    doc.add_page(p);
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out = env::args()
        .nth(1)
        .unwrap_or_else(|| DEFAULT_OUT.to_string());
    let mut doc = Document::new();
    doc.set_title("Synthetic Slide Deck Fixture");
    // Pin /CreationDate so the only time-dependent bytes are /ModDate (which
    // `save()` rewrites unconditionally) and the XMP attribute ordering
    // (driven by an unordered map in the XMP serializer). Tests should assert
    // extracted-content equality, not byte equality.
    let pinned = Utc.with_ymd_and_hms(2026, 6, 15, 0, 0, 0).unwrap();
    doc.set_creation_date(pinned);
    cover(&mut doc)?;
    four_column(&mut doc)?;
    summary(&mut doc)?;
    closing(&mut doc)?;
    doc.save(&out)?;
    println!("wrote {}", out);
    Ok(())
}
