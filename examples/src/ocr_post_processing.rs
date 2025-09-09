//! OCR Post-Processing and Auto-Correction Demo
//!
//! This example demonstrates automatic correction of OCR errors using various strategies:
//! - Character substitution corrections
//! - Pattern-based corrections  
//! - Dictionary-based corrections
//! - Edit distance calculations

use std::collections::HashSet;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    use oxidize_pdf::text::{MockOcrProvider, OcrOptions, OcrProvider};

    println!("🔧 OCR Post-Processing and Auto-Correction Demo");
    println!("=============================================");

    // Create OCR provider
    let provider = MockOcrProvider::new();
    let options = OcrOptions::default();

    // Create problematic OCR text with common errors
    let problematic_image = create_problematic_ocr_data();

    // Process with OCR (will return mock results that we'll enhance)
    let mut result = provider.process_image(&problematic_image, &options)?;

    // Simulate realistic OCR errors in the fragments
    enhance_with_ocr_errors(&mut result.fragments);

    println!("📄 Original OCR Output (with errors):");
    println!("{}", "=".repeat(50));
    for (i, fragment) in result.fragments.iter().enumerate() {
        println!("Fragment {}: \"{}\"", i + 1, fragment.text);
    }

    // Create post-processor with dictionary
    let dictionary = create_sample_dictionary();
    let post_processor = oxidize_pdf::text::OcrPostProcessor::new().with_dictionary(dictionary);

    println!("\n🔧 Post-Processing Analysis:");
    println!("{}", "=".repeat(50));

    // Process each fragment for corrections
    for (i, fragment) in result.fragments.iter().enumerate() {
        println!("\n📄 Fragment {}: \"{}\"", i + 1, fragment.text);

        let candidates = post_processor.process_fragment(fragment);

        if candidates.is_empty() {
            println!("   ✅ No corrections needed");
            continue;
        }

        println!("   🔍 Found {} correction candidates:", candidates.len());

        for candidate in candidates {
            println!(
                "      ❌ \"{}\" (confidence: {:.1}%)",
                candidate.word,
                candidate.confidence * 100.0
            );

            if candidate.suggested_corrections.is_empty() {
                println!("         💭 No suggestions available");
            } else {
                println!("         💡 Suggestions:");
                for (j, suggestion) in candidate.suggested_corrections.iter().enumerate() {
                    println!(
                        "            {}. \"{}\" ({:.1}% confidence) - {}",
                        j + 1,
                        suggestion.corrected_word,
                        suggestion.correction_confidence * 100.0,
                        format_correction_type(&suggestion.correction_type)
                    );

                    if let Some(explanation) = &suggestion.explanation {
                        println!("               Reason: {}", explanation);
                    }
                }
            }
        }
    }

    // Demonstrate automatic correction application
    demonstrate_auto_correction(&result.fragments, &post_processor)?;

    // Show correction statistics
    demonstrate_correction_statistics(&result.fragments, &post_processor)?;

    // Performance analysis
    demonstrate_correction_performance(&post_processor)?;

    Ok(())
}

fn create_problematic_ocr_data() -> Vec<u8> {
    // Mock image that will produce OCR errors
    vec![
        0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46, 0x49, 0x46, 0x00, 0x01, 0x01, 0x01, 0x00,
        48, 0x00, 48, 0x00, 0x00, // Poor quality simulation
        0x11, 0x22, 0x33, 0x44, 0xFF, 0xD9,
    ]
}

fn enhance_with_ocr_errors(fragments: &mut Vec<oxidize_pdf::text::OcrTextFragment>) {
    // Simulate common OCR errors
    let error_texts = vec![
        "Th1s d0cument c0nta1ns s0me err0rs", // 0/O, 1/l confusions
        "The qu1ck br0wn f0x jumps",          // Mixed confusions
        "C0rnpany rep0rt f0r 2O24",           // Company -> C0rnpany, 2024 -> 2O24
        "Arnount: $1,234.56 d0llars",         // Amount -> Arnount
    ];

    // Replace fragment texts with error-prone versions
    for (i, fragment) in fragments.iter_mut().enumerate() {
        if i < error_texts.len() {
            fragment.text = error_texts[i].to_string();

            // Create word-level confidence data with errors
            let words: Vec<&str> = fragment.text.split_whitespace().collect();
            let mut word_confidences = Vec::new();
            let mut x_offset = 0.0;

            for word in words {
                let word_width = word.len() as f64 * 8.0;

                // Lower confidence for words with errors
                let confidence = if has_ocr_errors(word) {
                    0.45 + (word.len() as f64 * 0.05) // 45-65% for error words
                } else {
                    0.85 + (rand_factor() * 0.1) // 85-95% for clean words
                };

                // Create character confidences for low-confidence words
                let character_confidences = if confidence < 0.7 {
                    Some(create_character_confidences_for_word(word, confidence))
                } else {
                    None
                };

                word_confidences.push(oxidize_pdf::text::WordConfidence {
                    word: word.to_string(),
                    confidence: confidence.clamp(0.1, 1.0),
                    x_offset,
                    width: word_width,
                    character_confidences,
                });

                x_offset += word_width + 4.0;
            }

            fragment.word_confidences = Some(word_confidences);
        }
    }
}

fn has_ocr_errors(word: &str) -> bool {
    // Common OCR error patterns
    word.contains('0') ||  // 0/O confusion
    word.contains('1') ||  // 1/l/I confusion  
    word.contains('5') ||  // 5/S confusion
    word.contains("rn") || // rn/m confusion
    word.contains("cl") // cl/d confusion
}

fn rand_factor() -> f64 {
    // Simple pseudo-random factor based on static seed
    static mut SEED: u64 = 12345;
    unsafe {
        SEED = SEED.wrapping_mul(1103515245).wrapping_add(12345);
        (SEED % 1000) as f64 / 1000.0
    }
}

fn create_character_confidences_for_word(
    word: &str,
    base_confidence: f64,
) -> Vec<oxidize_pdf::text::CharacterConfidence> {
    let mut char_confidences = Vec::new();
    let mut x_offset = 0.0;

    for ch in word.chars() {
        let char_width = 6.0;

        // Lower confidence for problematic characters
        let confidence = match ch {
            '0' | 'O' | 'o' => base_confidence * 0.7, // O/0 confusion
            '1' | 'l' | 'I' => base_confidence * 0.6, // 1/l/I confusion
            '5' | 'S' => base_confidence * 0.8,       // 5/S confusion
            _ => base_confidence,
        };

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

fn create_sample_dictionary() -> HashSet<String> {
    let words = vec![
        // Common words
        "the", "quick", "brown", "fox", "jumps", "over", "lazy", "dog", "this", "document",
        "contains", "some", "errors", "company", "report", "amount", "dollars", "for", "and", "or",
        "but", "with", "from", "to", // Numbers spelled out
        "one", "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten",
        // Years
        "2024", "2023", "2022", "2021", "2020", // Business terms
        "invoice", "receipt", "payment", "total", "subtotal", "tax", "discount",
    ];

    words.into_iter().map(|s| s.to_string()).collect()
}

fn format_correction_type(correction_type: &oxidize_pdf::text::CorrectionType) -> String {
    use oxidize_pdf::text::CorrectionType;
    match correction_type {
        CorrectionType::CharacterSubstitution => "Character substitution".to_string(),
        CorrectionType::DictionaryCorrection => "Dictionary correction".to_string(),
        CorrectionType::ContextualCorrection => "Contextual correction".to_string(),
        CorrectionType::PatternCorrection => "Pattern correction".to_string(),
        CorrectionType::ManualReview => "Manual review needed".to_string(),
    }
}

fn demonstrate_auto_correction(
    fragments: &[oxidize_pdf::text::OcrTextFragment],
    post_processor: &oxidize_pdf::text::OcrPostProcessor,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🎯 Automatic Correction Application:");
    println!("{}", "=".repeat(50));

    for (i, fragment) in fragments.iter().enumerate() {
        let candidates = post_processor.process_fragment(fragment);

        if candidates.is_empty() {
            continue;
        }

        println!("\nFragment {}: \"{}\"", i + 1, fragment.text);

        // Apply best corrections (highest confidence)
        let mut corrected_text = fragment.text.clone();
        let mut corrections_applied = Vec::new();

        for candidate in candidates {
            if let Some(best_suggestion) = candidate.suggested_corrections.first() {
                if best_suggestion.correction_confidence > 0.75 {
                    corrected_text =
                        corrected_text.replace(&candidate.word, &best_suggestion.corrected_word);
                    corrections_applied.push((
                        candidate.word.clone(),
                        best_suggestion.corrected_word.clone(),
                    ));
                }
            }
        }

        if !corrections_applied.is_empty() {
            println!("Corrected: \"{}\"", corrected_text);
            println!("Changes:");
            for (original, corrected) in corrections_applied {
                println!("   \"{}\" → \"{}\"", original, corrected);
            }
        }
    }

    Ok(())
}

fn demonstrate_correction_statistics(
    fragments: &[oxidize_pdf::text::OcrTextFragment],
    post_processor: &oxidize_pdf::text::OcrPostProcessor,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n📊 Correction Statistics:");
    println!("{}", "=".repeat(50));

    let mut total_words = 0;
    let mut words_needing_correction = 0;
    let mut words_with_suggestions = 0;
    let mut high_confidence_corrections = 0;

    let mut correction_type_counts = std::collections::HashMap::new();

    for fragment in fragments {
        if let Some(words) = &fragment.word_confidences {
            total_words += words.len();
        }

        let candidates = post_processor.process_fragment(fragment);
        words_needing_correction += candidates.len();

        for candidate in candidates {
            if !candidate.suggested_corrections.is_empty() {
                words_with_suggestions += 1;

                for suggestion in &candidate.suggested_corrections {
                    if suggestion.correction_confidence > 0.8 {
                        high_confidence_corrections += 1;
                    }

                    *correction_type_counts
                        .entry(suggestion.correction_type.clone())
                        .or_insert(0) += 1;
                }
            }
        }
    }

    println!("Total words analyzed: {}", total_words);
    println!(
        "Words needing correction: {} ({:.1}%)",
        words_needing_correction,
        words_needing_correction as f64 / total_words as f64 * 100.0
    );
    println!(
        "Words with suggestions: {} ({:.1}%)",
        words_with_suggestions,
        words_with_suggestions as f64 / words_needing_correction as f64 * 100.0
    );
    println!(
        "High-confidence corrections: {}",
        high_confidence_corrections
    );

    println!("\nCorrection types breakdown:");
    for (correction_type, count) in correction_type_counts {
        println!("   {}: {}", format_correction_type(&correction_type), count);
    }

    Ok(())
}

fn demonstrate_correction_performance(
    post_processor: &oxidize_pdf::text::OcrPostProcessor,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n⚡ Correction Performance Analysis:");
    println!("{}", "=".repeat(50));

    // Test various correction algorithms
    let test_words = vec![
        ("th1s", "this"),       // Character substitution
        ("c0mpany", "company"), // Multiple substitutions
        ("arnount", "amount"),  // Dictionary correction
        ("teh", "the"),         // Transposition (edit distance)
        ("recieve", "receive"), // Common misspelling
    ];

    let start_time = std::time::Instant::now();

    let test_words_len = test_words.len();

    for (error_word, expected) in &test_words {
        let suggestions = post_processor.generate_suggestions(error_word);

        println!("\nError: \"{}\" → Expected: \"{}\"", error_word, expected);

        if suggestions.is_empty() {
            println!("   No suggestions found");
        } else {
            println!("   Top suggestions:");
            for (i, suggestion) in suggestions.iter().take(3).enumerate() {
                let marker = if suggestion.corrected_word == *expected {
                    "✅"
                } else {
                    "  "
                };
                println!(
                    "   {} {}. \"{}\" ({:.1}%) - {}",
                    marker,
                    i + 1,
                    suggestion.corrected_word,
                    suggestion.correction_confidence * 100.0,
                    format_correction_type(&suggestion.correction_type)
                );
            }
        }
    }

    let processing_time = start_time.elapsed();

    println!("\nPerformance metrics:");
    println!("   Processing time: {:.2}ms", processing_time.as_millis());
    println!(
        "   Words per second: {:.0}",
        test_words_len as f64 / processing_time.as_secs_f64()
    );

    // Memory usage estimation
    let dict_size = post_processor.dictionary.as_ref().map_or(0, |d| d.len());
    println!("   Dictionary size: {} words", dict_size);
    println!(
        "   Character correction rules: {}",
        post_processor.character_corrections.len()
    );
    println!(
        "   Pattern correction rules: {}",
        post_processor.pattern_corrections.len()
    );

    Ok(())
}
