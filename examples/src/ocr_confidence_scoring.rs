//! OCR Advanced Confidence Scoring Demo
//!
//! This example demonstrates detailed confidence scoring at word and character level.
//! It shows how to analyze OCR quality and identify problematic words for post-processing.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    use oxidize_pdf::text::{MockOcrProvider, OcrOptions, OcrProvider};

    println!("🎯 OCR Advanced Confidence Scoring Demo");
    println!("======================================");

    // Initialize OCR provider
    let provider = MockOcrProvider::new();

    // Create OCR options for high-quality analysis
    let options = OcrOptions {
        language: "eng".to_string(),
        min_confidence: 0.5, // Lower threshold to see problematic words
        preserve_layout: true,
        ..Default::default()
    };

    // Create mock image data
    let image_data = create_mock_image_with_quality_issues();

    // Process image
    println!("📊 Processing image with quality issues...");
    let mut result = provider.process_image(&image_data, &options)?;

    // Enhance fragments with detailed word-level confidence data
    enhance_fragments_with_word_confidence(&mut result.fragments);

    println!("✅ Processing complete\n");

    // Demonstrate confidence analysis
    demonstrate_confidence_analysis(&result.fragments)?;
    demonstrate_low_confidence_detection(&result.fragments)?;
    demonstrate_confidence_based_filtering(&result.fragments)?;
    demonstrate_detailed_confidence_reports(&result.fragments)?;

    Ok(())
}

fn create_mock_image_with_quality_issues() -> Vec<u8> {
    // Mock JPEG with simulated quality issues
    vec![
        0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46, 0x49, 0x46, 0x00, 0x01, 0x01, 0x01, 0x00,
        48, 0x00, 48, 0x00, 0x00, // Add variation to simulate quality issues
        0x88, 0x77, 0x99, 0x66, 0xFF, 0xD9,
    ]
}

fn enhance_fragments_with_word_confidence(fragments: &mut Vec<oxidize_pdf::text::OcrTextFragment>) {
    // Simulate realistic word-level confidence scores for different scenarios
    for (i, fragment) in fragments.iter_mut().enumerate() {
        let words = fragment.text.split_whitespace().collect::<Vec<_>>();
        let mut word_confidences = Vec::new();
        let mut x_offset = 0.0;

        for (j, word) in words.iter().enumerate() {
            let word_width = word.len() as f64 * 8.0; // Approximate width

            // Simulate different confidence levels based on word characteristics
            let base_confidence = match i {
                0 => 0.95, // First fragment: high quality
                1 => 0.75, // Second fragment: medium quality
                _ => 0.60, // Others: lower quality
            };

            let confidence = simulate_word_confidence(word, base_confidence, j);

            // Create character-level confidences for some words
            let character_confidences = if confidence < 0.7 {
                Some(create_character_confidences(word, confidence))
            } else {
                None
            };

            word_confidences.push(oxidize_pdf::text::WordConfidence {
                word: word.to_string(),
                confidence,
                x_offset,
                width: word_width,
                character_confidences,
            });

            x_offset += word_width + 4.0; // Add space width
        }

        fragment.word_confidences = Some(word_confidences);
    }
}

fn simulate_word_confidence(word: &str, base_confidence: f64, position: usize) -> f64 {
    let mut confidence = base_confidence;

    // Simulate quality issues based on word characteristics
    if word.len() < 3 {
        confidence *= 0.9; // Short words are harder to recognize
    }

    if word.chars().any(|c| c.is_numeric()) {
        confidence *= 0.85; // Numbers can be confused
    }

    if word.chars().any(|c| c.is_uppercase()) && word.len() > 1 {
        confidence *= 1.1; // All caps are often clearer
    }

    // Simulate position-based degradation
    if position > 5 {
        confidence *= 0.95; // Later words might have lower quality
    }

    // Add some randomness to simulate real OCR variations
    let variation = (position as f64 * 0.1) % 0.2 - 0.1;
    confidence += variation;

    // Ensure bounds
    confidence.clamp(0.1, 1.0)
}

fn create_character_confidences(
    word: &str,
    word_confidence: f64,
) -> Vec<oxidize_pdf::text::CharacterConfidence> {
    let mut char_confidences = Vec::new();
    let mut x_offset = 0.0;

    for (i, ch) in word.chars().enumerate() {
        let char_width = 6.0; // Average character width

        // Simulate character-specific confidence issues
        let mut confidence = word_confidence;

        // Common OCR confusion patterns
        match ch {
            'o' | 'O' | '0' => confidence *= 0.7, // O/0 confusion
            'l' | 'I' | '1' => confidence *= 0.6, // l/I/1 confusion
            'm' => confidence *= 0.8,             // rn/m confusion pattern
            'S' | '5' => confidence *= 0.8,       // S/5 confusion
            _ => {}
        }

        // First and last characters are often less clear
        if i == 0 || i == word.len() - 1 {
            confidence *= 0.9;
        }

        char_confidences.push(oxidize_pdf::text::CharacterConfidence::new(
            ch,
            confidence.clamp(0.1, 1.0),
            x_offset,
            char_width,
        ));

        x_offset += char_width;
    }

    char_confidences
}

fn demonstrate_confidence_analysis(
    fragments: &[oxidize_pdf::text::OcrTextFragment],
) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Confidence Analysis Overview:");
    println!("{}", "-".repeat(50));

    for (i, fragment) in fragments.iter().enumerate() {
        println!(
            "\n📄 Fragment {}: Overall {:.1}%",
            i + 1,
            fragment.confidence * 100.0
        );
        println!("   Text: \"{}\"", fragment.text.trim());

        if let Some(avg_word_conf) = fragment.average_word_confidence() {
            println!("   Average word confidence: {:.1}%", avg_word_conf * 100.0);
        }

        if let Some(words) = &fragment.word_confidences {
            let low_conf_count = words.iter().filter(|w| w.confidence < 0.7).count();
            println!(
                "   Words: {} total, {} below 70% confidence",
                words.len(),
                low_conf_count
            );

            // Show confidence distribution
            let high_conf = words.iter().filter(|w| w.confidence >= 0.9).count();
            let med_conf = words
                .iter()
                .filter(|w| w.confidence >= 0.7 && w.confidence < 0.9)
                .count();
            let low_conf = words.iter().filter(|w| w.confidence < 0.7).count();

            println!(
                "   Distribution: {} high (≥90%), {} medium (70-89%), {} low (<70%)",
                high_conf, med_conf, low_conf
            );
        }
    }

    Ok(())
}

fn demonstrate_low_confidence_detection(
    fragments: &[oxidize_pdf::text::OcrTextFragment],
) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n⚠️  Low Confidence Word Detection:");
    println!("{}", "-".repeat(50));

    for (i, fragment) in fragments.iter().enumerate() {
        let low_conf_words = fragment.get_low_confidence_words(0.7);

        if !low_conf_words.is_empty() {
            println!(
                "\n📄 Fragment {} - {} problematic words:",
                i + 1,
                low_conf_words.len()
            );

            for word in low_conf_words {
                println!(
                    "   ⚠️  \"{}\": {:.1}% confidence",
                    word.word,
                    word.confidence * 100.0
                );

                // Show character-level issues if available
                if let Some(chars) = &word.character_confidences {
                    let low_chars: Vec<_> = chars.iter().filter(|c| c.confidence < 0.6).collect();

                    if !low_chars.is_empty() {
                        print!("       Low-conf characters: ");
                        for ch in low_chars {
                            print!("'{}' ({:.0}%) ", ch.character, ch.confidence * 100.0);
                        }
                        println!();
                    }
                }
            }
        }
    }

    Ok(())
}

fn demonstrate_confidence_based_filtering(
    fragments: &[oxidize_pdf::text::OcrTextFragment],
) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🎯 Confidence-Based Text Filtering:");
    println!("{}", "-".repeat(50));

    // Filter by different confidence thresholds
    let thresholds = [0.9, 0.8, 0.7, 0.6];

    for threshold in thresholds {
        println!("\n🔍 Words with ≥{:.0}% confidence:", threshold * 100.0);

        let mut high_conf_words = Vec::new();

        for fragment in fragments {
            if let Some(words) = &fragment.word_confidences {
                for word in words {
                    if word.confidence >= threshold {
                        high_conf_words.push(&word.word);
                    }
                }
            }
        }

        let words_str: Vec<&str> = high_conf_words.iter().map(|&s| s.as_str()).collect();
        println!(
            "   Found {} words: {}",
            high_conf_words.len(),
            words_str.join(", ")
        );

        // Calculate text recovery percentage
        let total_words = fragments
            .iter()
            .filter_map(|f| f.word_confidences.as_ref())
            .map(|words| words.len())
            .sum::<usize>();

        if total_words > 0 {
            let recovery_rate = (high_conf_words.len() as f64 / total_words as f64) * 100.0;
            println!("   Text recovery rate: {:.1}%", recovery_rate);
        }
    }

    Ok(())
}

fn demonstrate_detailed_confidence_reports(
    fragments: &[oxidize_pdf::text::OcrTextFragment],
) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n📊 Detailed Confidence Reports:");
    println!("{}", "=".repeat(50));

    for (i, fragment) in fragments.iter().enumerate() {
        if fragment.has_low_confidence_words(0.8) {
            println!("\n📄 Fragment {} Report:", i + 1);
            print!("{}", fragment.confidence_report());
        }
    }

    // Generate overall quality assessment
    println!("\n🎯 Overall Quality Assessment:");
    println!("{}", "-".repeat(40));

    let total_words: usize = fragments
        .iter()
        .filter_map(|f| f.word_confidences.as_ref())
        .map(|words| words.len())
        .sum();

    let problematic_words: usize = fragments
        .iter()
        .map(|f| f.get_low_confidence_words(0.7).len())
        .sum();

    let quality_score = if total_words > 0 {
        ((total_words - problematic_words) as f64 / total_words as f64) * 100.0
    } else {
        0.0
    };

    println!("Total words analyzed: {}", total_words);
    println!("Problematic words: {}", problematic_words);
    println!("Overall quality score: {:.1}%", quality_score);

    // Recommendations
    if quality_score < 70.0 {
        println!("\n💡 Recommendations:");
        println!("   - Image quality appears low - consider rescanning");
        println!("   - Use post-processing to correct identified issues");
        println!("   - Consider manual review of low-confidence words");
    } else if quality_score < 85.0 {
        println!("\n💡 Recommendations:");
        println!("   - Good overall quality with some issues");
        println!("   - Review words below 70% confidence");
    } else {
        println!("\n✅ Excellent OCR quality - minimal post-processing needed");
    }

    Ok(())
}
