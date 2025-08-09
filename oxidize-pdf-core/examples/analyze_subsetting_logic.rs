use std::collections::HashSet;

fn main() {
    println!("=== Font Subsetting Logic Analysis ===\n");

    // Simulate different scenarios
    let scenarios = vec![
        ("Minimal", 10, 3000, 350_000),  // 10 chars, 3000 glyphs, 350KB font
        ("Small Font", 50, 500, 80_000), // 50 chars, 500 glyphs, 80KB font
        ("Large Usage", 1500, 3000, 350_000), // 1500 chars, 3000 glyphs, 350KB font
        ("Medium", 100, 2000, 250_000),  // 100 chars, 2000 glyphs, 250KB font
        ("Unicode Heavy", 300, 5000, 500_000), // 300 chars, 5000 glyphs, 500KB font
    ];

    for (name, used_chars, total_glyphs, font_size) in scenarios {
        println!("Scenario: {}", name);
        println!("  Used characters: {}", used_chars);
        println!("  Total glyphs: {}", total_glyphs);
        println!(
            "  Font size: {} bytes ({:.1} KB)",
            font_size,
            font_size as f64 / 1024.0
        );

        // Simulate the subsetting logic from truetype_subsetter.rs

        // Check 1: Empty or small char set (line 103)
        if used_chars == 0 || used_chars < 10 {
            println!("  Decision: Return full font (too few characters)");
            println!("  Reason: used_chars.is_empty() || used_chars.len() < 10");
            println!();
            continue;
        }

        // Simulate needed glyphs calculation
        let needed_glyphs = used_chars + 1; // +1 for .notdef

        // Check 2: Subset ratio and font size (lines 127-128)
        let subset_ratio = needed_glyphs as f32 / total_glyphs as f32;

        println!(
            "  Subset ratio: {:.1}% of glyphs needed",
            subset_ratio * 100.0
        );

        if subset_ratio > 0.5 || font_size < 100_000 {
            println!("  Decision: Keep full font");
            if subset_ratio > 0.5 {
                println!("  Reason: Using more than 50% of glyphs");
            } else {
                println!("  Reason: Font is smaller than 100KB");
            }
        } else {
            println!("  Decision: Should subset font");
            println!(
                "  Reason: Using only {:.1}% of glyphs in a large font",
                subset_ratio * 100.0
            );
            println!("  BUT: Actual subsetting NOT implemented!");
            println!("  Result: Returns full font anyway (see lines 162-164)");
        }

        println!();
    }

    println!("=== Key Findings ===");
    println!("1. Subsetting is decided based on:");
    println!("   - Less than 10 characters → Keep full font");
    println!("   - Using >50% of glyphs → Keep full font");
    println!("   - Font <100KB → Keep full font");
    println!("   - Otherwise → Should subset");
    println!();
    println!("2. PROBLEM: Even when subsetting is decided,");
    println!("   the code returns the full font (lines 162-164)");
    println!();
    println!("3. The build_subset_font() method exists but is NEVER called");
    println!("   It has all the logic to actually subset the font:");
    println!("   - Rebuild glyf table");
    println!("   - Update loca table");
    println!("   - Rebuild cmap");
    println!("   - Update hmtx");
    println!();
    println!("4. To fix: Replace lines 162-164 with a call to build_subset_font()");
}
