//! Memory Usage Benchmarks
//!
//! Benchmarks for measuring memory allocation patterns, peak usage,
//! and memory efficiency of the oxidize-pdf library components.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use oxidize_pdf::objects::{Array, Object};
use oxidize_pdf::parser::objects::{PdfDictionary, PdfName, PdfObject, PdfStream};
use oxidize_pdf_test_suite::generators::test_pdf_builder::TestPdfBuilder;
use std::collections::HashMap;

/// Memory allocation patterns for Array operations
fn benchmark_array_memory_patterns(c: &mut Criterion) {
    let mut group = c.benchmark_group("array_memory");
    group.sample_size(20); // Reduce samples for memory-intensive benchmarks

    let sizes = vec![100, 1000, 10000, 50000];

    for size in sizes {
        // Benchmark memory allocation during array growth
        group.bench_with_input(
            BenchmarkId::new("incremental_growth", size),
            &size,
            |b, &size| {
                b.iter(|| {
                    let mut array = Array::new();
                    for i in 0..size {
                        // Simulate mixed object types to stress allocator
                        let obj = match i % 5 {
                            0 => Object::Integer(i as i64),
                            1 => Object::Boolean(i % 2 == 0),
                            2 => Object::String(format!("String_{i}")),
                            3 => Object::Name(format!("Name_{i}")),
                            _ => Object::Real(i as f64 * 0.1),
                        };
                        array.push(black_box(obj));
                    }
                    black_box(array)
                });
            },
        );

        // Benchmark pre-allocated vs incremental
        group.bench_with_input(
            BenchmarkId::new("pre_allocated", size),
            &size,
            |b, &size| {
                b.iter(|| {
                    let objects: Vec<Object> = (0..size)
                        .map(|i| match i % 3 {
                            0 => Object::Integer(i as i64),
                            1 => Object::String(format!("Str{i}")),
                            _ => Object::Boolean(i % 2 == 0),
                        })
                        .collect();

                    let array = Array::from(black_box(objects));
                    black_box(array)
                });
            },
        );

        // Benchmark memory usage during large array cloning
        if size <= 10000 {
            // Limit for clone benchmarks
            let test_objects: Vec<Object> = (0..size)
                .map(|i| Object::String(format!("CloneTest_{i}")))
                .collect();
            let large_array = Array::from(test_objects);

            group.bench_with_input(
                BenchmarkId::new("clone_large_array", size),
                &large_array,
                |b, array| {
                    b.iter(|| {
                        let cloned = black_box(array).clone();
                        black_box(cloned)
                    });
                },
            );
        }
    }

    group.finish();
}

/// Memory usage patterns for Dictionary operations
fn benchmark_dictionary_memory_patterns(c: &mut Criterion) {
    let mut group = c.benchmark_group("dictionary_memory");
    group.sample_size(15);

    let sizes = vec![50, 500, 2000, 5000];

    for size in sizes {
        // Memory allocation during dictionary growth
        group.bench_with_input(
            BenchmarkId::new("incremental_insertion", size),
            &size,
            |b, &size| {
                b.iter(|| {
                    let mut dict = PdfDictionary::new();
                    for i in 0..size {
                        let key = format!("Key_{i:06}"); // Fixed width for consistent memory
                        let value = match i % 4 {
                            0 => PdfObject::Integer(i as i64),
                            1 => PdfObject::Boolean(i % 2 == 0),
                            2 => PdfObject::Name(PdfName::new(format!("Name_{i}"))),
                            _ => PdfObject::String(oxidize_pdf::parser::objects::PdfString::new(
                                format!("Value_{i}").into_bytes(),
                            )),
                        };
                        dict.insert(black_box(key), black_box(value));
                    }
                    black_box(dict)
                });
            },
        );

        // Memory usage with large string values
        group.bench_with_input(
            BenchmarkId::new("large_string_values", size),
            &size,
            |b, &size| {
                b.iter(|| {
                    let mut dict = PdfDictionary::new();
                    for i in 0..size {
                        let key = format!("LargeKey_{i}");
                        // Create larger string values to test memory pressure
                        let large_value = "x".repeat(100 + (i % 500));
                        dict.insert(
                            black_box(key),
                            PdfObject::String(oxidize_pdf::parser::objects::PdfString::new(
                                black_box(large_value).into_bytes(),
                            )),
                        );
                    }
                    black_box(dict)
                });
            },
        );

        // Memory patterns during dictionary cloning
        if size <= 2000 {
            let mut test_dict = PdfDictionary::new();
            for i in 0..size {
                test_dict.insert(
                    format!("Key{i}"),
                    PdfObject::String(oxidize_pdf::parser::objects::PdfString::new(
                        format!("ComplexValue_{i}_with_content").into_bytes(),
                    )),
                );
            }

            group.bench_with_input(
                BenchmarkId::new("clone_dictionary", size),
                &test_dict,
                |b, dict| {
                    b.iter(|| {
                        let cloned = black_box(dict).clone();
                        black_box(cloned)
                    });
                },
            );
        }
    }

    group.finish();
}

/// Memory patterns for ObjectStream operations
fn benchmark_object_stream_memory(c: &mut Criterion) {
    let mut group = c.benchmark_group("object_stream_memory");
    group.sample_size(10);

    let object_counts = vec![10, 50, 100, 200];

    for count in object_counts {
        // Memory allocation during stream creation
        group.bench_with_input(
            BenchmarkId::new("stream_creation", count),
            &count,
            |b, &obj_count| {
                b.iter(|| {
                    let mut dict = PdfDictionary::new();
                    dict.insert("N".to_string(), PdfObject::Integer(obj_count as i64));
                    dict.insert("First".to_string(), PdfObject::Integer(20));
                    dict.insert(
                        "Type".to_string(),
                        PdfObject::Name(PdfName::new("ObjStm".to_string())),
                    );

                    // Create substantial stream data
                    let mut data = Vec::with_capacity(obj_count * 50);
                    for i in 0..obj_count {
                        data.extend_from_slice(format!("{} {} ", i + 1, i * 15).as_bytes());
                    }

                    // Add object data
                    for i in 0..obj_count {
                        let obj_data =
                            format!("<< /Type /Test /Index {} /Data {} >> ", i, "x".repeat(20));
                        data.extend_from_slice(obj_data.as_bytes());
                    }

                    let stream = PdfStream {
                        dict: black_box(dict),
                        data: black_box(data),
                    };
                    black_box(stream)
                });
            },
        );

        // Memory usage during ObjectStream parsing simulation
        group.bench_with_input(
            BenchmarkId::new("parsing_simulation", count),
            &count,
            |b, &obj_count| {
                b.iter(|| {
                    // Create a mock object cache to simulate memory usage
                    let mut object_cache: HashMap<u32, PdfObject> = HashMap::new();

                    for i in 0..obj_count {
                        let complex_object = match i % 4 {
                            0 => {
                                let mut inner_dict = PdfDictionary::new();
                                inner_dict.insert(
                                    "Type".to_string(),
                                    PdfObject::Name(PdfName::new("Page".to_string())),
                                );
                                inner_dict
                                    .insert("Index".to_string(), PdfObject::Integer(i as i64));
                                PdfObject::Dictionary(inner_dict)
                            }
                            1 => {
                                let large_string = format!(
                                    "LargeContent_{}_with_substantial_data_{}",
                                    i,
                                    "x".repeat(100)
                                );
                                PdfObject::String(oxidize_pdf::parser::objects::PdfString::new(
                                    large_string.into_bytes(),
                                ))
                            }
                            2 => {
                                let array_data: Vec<PdfObject> = (0..10)
                                    .map(|j| PdfObject::Integer((i * 10 + j) as i64))
                                    .collect();
                                {
                                    let mut pdf_array =
                                        oxidize_pdf::parser::objects::PdfArray::new();
                                    for item in array_data {
                                        pdf_array.push(item);
                                    }
                                    PdfObject::Array(pdf_array)
                                }
                            }
                            _ => PdfObject::Boolean(i % 2 == 0),
                        };

                        object_cache.insert(black_box(i as u32), black_box(complex_object));
                    }

                    black_box(object_cache)
                });
            },
        );
    }

    group.finish();
}

/// Memory usage patterns for PDF generation
fn benchmark_pdf_generation_memory(c: &mut Criterion) {
    let mut group = c.benchmark_group("pdf_generation_memory");
    group.sample_size(10);

    let page_counts = vec![1, 10, 50, 100];

    for pages in page_counts {
        // Memory allocation during PDF construction
        group.bench_with_input(
            BenchmarkId::new("multi_page_construction", pages),
            &pages,
            |b, &page_count| {
                b.iter(|| {
                    let mut builder = TestPdfBuilder::new();
                    for i in 0..page_count {
                        let page_content = format!(
                            "Page {} content with substantial text to simulate real document content. \
                             This page contains various elements and text to create realistic memory pressure \
                             during PDF generation. Page number: {}",
                            i + 1, i + 1
                        );
                        builder.add_text_page(&page_content, 12.0);

                        // Every 5th page, add graphics to increase memory complexity
                        if i % 5 == 0 {
                            builder.add_graphics_page();
                        }
                    }

                    let pdf_data = black_box(builder).build();
                    black_box(pdf_data)
                });
            },
        );

        // Memory usage for text-heavy documents
        group.bench_with_input(
            BenchmarkId::new("text_heavy_document", pages),
            &pages,
            |b, &page_count| {
                b.iter(|| {
                    let mut builder = TestPdfBuilder::new();
                    for i in 0..page_count {
                        // Create substantial text content
                        let large_text = format!(
                            "Page {} - Lorem ipsum dolor sit amet, consectetur adipiscing elit. \
                             Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. \
                             Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris. \
                             {} {} {}",
                            i + 1,
                            "Repeated content ".repeat(50),
                            "More text content ".repeat(30),
                            "Final section ".repeat(20)
                        );
                        builder.add_text_page(&large_text, 10.0);
                    }

                    let pdf_data = black_box(builder).build();
                    black_box(pdf_data)
                });
            },
        );
    }

    group.finish();
}

/// Memory patterns for string and name object creation
fn benchmark_string_memory_patterns(c: &mut Criterion) {
    let mut group = c.benchmark_group("string_memory");
    group.sample_size(30);

    let counts = vec![100, 1000, 5000];
    let string_sizes = vec![10, 100, 500, 1000];

    for count in counts {
        for str_size in &string_sizes {
            // Memory allocation for PdfObject strings
            group.bench_with_input(
                BenchmarkId::new("pdf_strings", format!("{count}x{str_size}")),
                &(count, *str_size),
                |b, &(count, size)| {
                    b.iter(|| {
                        let strings: Vec<PdfObject> = (0..count)
                            .map(|i| {
                                let content = format!("String{}_{}", i, "x".repeat(size));
                                PdfObject::String(oxidize_pdf::parser::objects::PdfString::new(
                                    black_box(content).into_bytes(),
                                ))
                            })
                            .collect();
                        black_box(strings)
                    });
                },
            );

            // Memory allocation for PdfName objects
            group.bench_with_input(
                BenchmarkId::new("pdf_names", format!("{count}x{str_size}")),
                &(count, *str_size),
                |b, &(count, size)| {
                    b.iter(|| {
                        let names: Vec<PdfObject> = (0..count)
                            .map(|i| {
                                let name_content =
                                    format!("Name{}_{}", i, "N".repeat(size.min(50))); // Names typically shorter
                                PdfObject::Name(PdfName::new(black_box(name_content)))
                            })
                            .collect();
                        black_box(names)
                    });
                },
            );
        }
    }

    group.finish();
}

/// Memory usage patterns for nested object structures
fn benchmark_nested_structure_memory(c: &mut Criterion) {
    let mut group = c.benchmark_group("nested_memory");
    group.sample_size(10);

    let depths = vec![5, 10, 15, 20];

    for depth in depths {
        // Deeply nested dictionaries
        group.bench_with_input(
            BenchmarkId::new("nested_dictionaries", depth),
            &depth,
            |b, &depth| {
                b.iter(|| {
                    fn create_nested_dict(remaining_depth: usize) -> PdfObject {
                        let mut dict = PdfDictionary::new();
                        dict.insert(
                            "Level".to_string(),
                            PdfObject::Integer(remaining_depth as i64),
                        );
                        dict.insert(
                            "Data".to_string(),
                            PdfObject::String(oxidize_pdf::parser::objects::PdfString::new(
                                format!("Level_{remaining_depth}_content").into_bytes(),
                            )),
                        );

                        if remaining_depth > 0 {
                            dict.insert(
                                "Child".to_string(),
                                create_nested_dict(remaining_depth - 1),
                            );
                        }

                        PdfObject::Dictionary(dict)
                    }

                    let nested = create_nested_dict(black_box(depth));
                    black_box(nested)
                });
            },
        );

        // Deeply nested arrays
        group.bench_with_input(
            BenchmarkId::new("nested_arrays", depth),
            &depth,
            |b, &depth| {
                b.iter(|| {
                    fn create_nested_array(remaining_depth: usize) -> PdfObject {
                        let mut array_items = vec![
                            PdfObject::Integer(remaining_depth as i64),
                            PdfObject::String(oxidize_pdf::parser::objects::PdfString::new(
                                format!("ArrayLevel_{remaining_depth}").into_bytes(),
                            )),
                        ];

                        if remaining_depth > 0 {
                            array_items.push(create_nested_array(remaining_depth - 1));
                        }

                        {
                            let mut pdf_array = oxidize_pdf::parser::objects::PdfArray::new();
                            for item in array_items {
                                pdf_array.push(item);
                            }
                            PdfObject::Array(pdf_array)
                        }
                    }

                    let nested = create_nested_array(black_box(depth));
                    black_box(nested)
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    memory_benches,
    benchmark_array_memory_patterns,
    benchmark_dictionary_memory_patterns,
    benchmark_object_stream_memory,
    benchmark_pdf_generation_memory,
    benchmark_string_memory_patterns,
    benchmark_nested_structure_memory
);
criterion_main!(memory_benches);
