//! Core Library Performance Benchmarks
//!
//! Benchmarks for fundamental operations in the oxidize-pdf core library,
//! focusing on data structures and parsing components that were recently
//! enhanced with comprehensive test coverage.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use oxidize_pdf::objects::{Array, Object};
use oxidize_pdf::parser::object_stream::{ObjectStream, XRefEntryType};
use oxidize_pdf::parser::objects::{PdfDictionary, PdfName, PdfObject, PdfStream};
use std::collections::HashMap;

/// Generate test objects for benchmarking
fn generate_test_objects(count: usize) -> Vec<Object> {
    let mut objects = Vec::with_capacity(count);
    for i in 0..count {
        match i % 4 {
            0 => objects.push(Object::Integer(i as i64)),
            1 => objects.push(Object::Boolean(i % 2 == 0)),
            2 => objects.push(Object::String(format!("String {}", i))),
            _ => objects.push(Object::Name(format!("Name{}", i))),
        }
    }
    objects
}

/// Benchmark Array operations
fn benchmark_array_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("array_operations");
    group.sample_size(50);

    // Test different array sizes
    let sizes = vec![10, 100, 1000, 10000];

    for size in sizes {
        let test_objects = generate_test_objects(size);

        // Benchmark array creation and population
        group.bench_with_input(
            BenchmarkId::new("create_and_populate", size),
            &test_objects,
            |b, objects| {
                b.iter(|| {
                    let mut array = Array::new();
                    for obj in objects {
                        array.push(black_box(obj.clone()));
                    }
                    black_box(array)
                });
            },
        );

        // Benchmark array iteration
        let populated_array = {
            let mut array = Array::new();
            for obj in &test_objects {
                array.push(obj.clone());
            }
            array
        };

        group.bench_with_input(
            BenchmarkId::new("iterate", size),
            &populated_array,
            |b, array| {
                b.iter(|| {
                    let mut count = 0;
                    for item in array.iter() {
                        count += 1;
                        black_box(item);
                    }
                    black_box(count)
                });
            },
        );

        // Benchmark random access
        group.bench_with_input(
            BenchmarkId::new("random_access", size),
            &populated_array,
            |b, array| {
                b.iter(|| {
                    let mut sum = 0;
                    for i in (0..array.len()).step_by(7) {
                        // Step by 7 for pseudo-random access
                        if let Some(item) = array.get(i) {
                            sum += 1;
                            black_box(item);
                        }
                    }
                    black_box(sum)
                });
            },
        );

        // Benchmark array modification (insert/remove)
        group.bench_with_input(
            BenchmarkId::new("modifications", size.min(1000)), // Limit to prevent slow benchmarks
            &test_objects[..size.min(1000)].to_vec(),
            |b, objects| {
                b.iter(|| {
                    let mut array = Array::from(objects.clone());
                    // Insert at beginning (expensive)
                    array.insert(0, Object::Integer(9999));
                    // Remove from middle
                    if array.len() > 2 {
                        array.remove(array.len() / 2);
                    }
                    // Pop from end (cheap)
                    array.pop();
                    black_box(array)
                });
            },
        );
    }

    group.finish();
}

/// Benchmark Array type conversions
fn benchmark_array_conversions(c: &mut Criterion) {
    let mut group = c.benchmark_group("array_conversions");
    let sizes = vec![100, 1000, 5000];

    for size in sizes {
        let test_objects = generate_test_objects(size);

        // Vec -> Array conversion
        group.bench_with_input(
            BenchmarkId::new("vec_to_array", size),
            &test_objects,
            |b, objects| {
                b.iter(|| {
                    let array = Array::from(black_box(objects.clone()));
                    black_box(array)
                });
            },
        );

        // Array -> Vec conversion
        let test_array = Array::from(test_objects.clone());
        group.bench_with_input(
            BenchmarkId::new("array_to_vec", size),
            &test_array,
            |b, array| {
                b.iter(|| {
                    let vec: Vec<Object> = black_box(array.clone()).into();
                    black_box(vec)
                });
            },
        );

        // FromIterator implementation
        group.bench_with_input(
            BenchmarkId::new("from_iterator", size),
            &test_objects,
            |b, objects| {
                b.iter(|| {
                    let array: Array = black_box(objects.iter().cloned()).collect();
                    black_box(array)
                });
            },
        );
    }

    group.finish();
}

/// Create test stream for ObjectStream benchmarks
fn create_test_object_stream(object_count: u32) -> PdfStream {
    let mut dict = PdfDictionary::new();
    dict.insert("N".to_string(), PdfObject::Integer(object_count as i64));
    dict.insert("First".to_string(), PdfObject::Integer(10));
    dict.insert(
        "Type".to_string(),
        PdfObject::Name(PdfName::new("ObjStm".to_string())),
    );

    // Create mock stream data
    let mut data = Vec::new();
    for i in 0..object_count {
        data.extend_from_slice(format!("{} {} ", i + 1, i * 10).as_bytes());
    }

    // Add some mock object data
    for _i in 0..object_count {
        data.extend_from_slice("<<>> ".to_string().as_bytes());
    }

    PdfStream { dict, data }
}

/// Benchmark ObjectStream operations
fn benchmark_object_stream(c: &mut Criterion) {
    let mut group = c.benchmark_group("object_stream");
    group.sample_size(20);

    let object_counts = vec![10, 50, 100];

    for count in object_counts {
        let test_stream = create_test_object_stream(count);

        // Benchmark ObjectStream parsing
        group.bench_with_input(
            BenchmarkId::new("parse_stream", count),
            &test_stream,
            |b, stream| {
                b.iter(|| {
                    // Simulate parsing attempt (will likely fail but measures effort)
                    // Simulate parsing attempt (will likely fail but measures effort)
                    let _ = ObjectStream::parse(black_box(stream.clone()));
                });
            },
        );

        // Benchmark object retrieval simulation
        group.bench_with_input(
            BenchmarkId::new("object_lookup_simulation", count),
            &count,
            |b, &obj_count| {
                b.iter(|| {
                    let mut cache = HashMap::new();
                    for i in 0..obj_count {
                        cache.insert(i, PdfObject::Integer(i as i64));
                    }

                    // Simulate random lookups
                    let mut found = 0;
                    for i in (0..obj_count).step_by(3) {
                        if cache.get(&i).is_some() {
                            found += 1;
                        }
                    }
                    black_box(found)
                });
            },
        );
    }

    group.finish();
}

/// Benchmark XRefEntryType operations
fn benchmark_xref_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("xref_operations");
    group.sample_size(100);

    let entry_types = [
        XRefEntryType::Free {
            next_free_obj: 42,
            generation: 1,
        },
        XRefEntryType::InUse {
            offset: 1024,
            generation: 2,
        },
        XRefEntryType::Compressed {
            stream_obj_num: 10,
            index_in_stream: 5,
        },
    ];

    // Benchmark entry type conversions
    for (i, entry_type) in entry_types.iter().enumerate() {
        group.bench_with_input(
            BenchmarkId::new("to_simple_entry", i),
            entry_type,
            |b, entry| {
                b.iter(|| {
                    let simple = black_box(entry).to_simple_entry();
                    black_box(simple)
                });
            },
        );
    }

    // Benchmark batch conversions
    let large_entry_list: Vec<XRefEntryType> = (0..1000)
        .map(|i| match i % 3 {
            0 => XRefEntryType::Free {
                next_free_obj: i,
                generation: 0,
            },
            1 => XRefEntryType::InUse {
                offset: i as u64 * 100,
                generation: 1,
            },
            _ => XRefEntryType::Compressed {
                stream_obj_num: i / 10,
                index_in_stream: i % 10,
            },
        })
        .collect();

    group.bench_function("batch_conversion_1000", |b| {
        b.iter(|| {
            let converted: Vec<_> = large_entry_list
                .iter()
                .map(|entry| black_box(entry).to_simple_entry())
                .collect();
            black_box(converted)
        });
    });

    group.finish();
}

/// Benchmark PDF Dictionary operations
fn benchmark_dictionary_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("dictionary_operations");
    let sizes = vec![10, 100, 500];

    for size in sizes {
        // Create test dictionary
        let mut test_dict = PdfDictionary::new();
        for i in 0..size {
            let key = format!("Key{}", i);
            let value = match i % 4 {
                0 => PdfObject::Integer(i as i64),
                1 => PdfObject::Boolean(i % 2 == 0),
                2 => PdfObject::Name(PdfName::new(format!("Name{}", i))),
                _ => PdfObject::String(oxidize_pdf::parser::objects::PdfString::new(
                    format!("String{}", i).into_bytes(),
                )),
            };
            test_dict.insert(key, value);
        }

        // Benchmark dictionary lookups
        group.bench_with_input(
            BenchmarkId::new("lookup_operations", size),
            &test_dict,
            |b, dict| {
                b.iter(|| {
                    let mut found = 0;
                    for i in (0..size).step_by(3) {
                        let key = format!("Key{}", i);
                        if dict.get(&key).is_some() {
                            found += 1;
                        }
                    }
                    black_box(found)
                });
            },
        );

        // Benchmark dictionary iteration
        group.bench_with_input(
            BenchmarkId::new("iterate_entries", size),
            &test_dict,
            |b, dict| {
                b.iter(|| {
                    let mut count = 0;
                    for (key, value) in dict.0.iter() {
                        count += 1;
                        black_box((key, value));
                    }
                    black_box(count)
                });
            },
        );
    }

    group.finish();
}

/// Benchmark string and name operations
fn benchmark_string_name_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("string_name_operations");

    let test_strings: Vec<String> = (0..1000)
        .map(|i| format!("TestString{}_with_some_length_{}", i, "x".repeat(i % 50)))
        .collect();

    // Benchmark PdfName creation
    group.bench_function("pdf_name_creation", |b| {
        b.iter(|| {
            let names: Vec<PdfName> = test_strings
                .iter()
                .map(|s| PdfName::new(black_box(s.clone())))
                .collect();
            black_box(names)
        });
    });

    // Benchmark PdfObject creation with strings
    group.bench_function("pdf_object_strings", |b| {
        b.iter(|| {
            let objects: Vec<PdfObject> = test_strings
                .iter()
                .enumerate()
                .map(|(i, s)| {
                    if i % 2 == 0 {
                        PdfObject::String(oxidize_pdf::parser::objects::PdfString::new(
                            black_box(s.clone()).into_bytes(),
                        ))
                    } else {
                        PdfObject::Name(PdfName::new(black_box(s.clone())))
                    }
                })
                .collect();
            black_box(objects)
        });
    });

    group.finish();
}

criterion_group!(
    core_benches,
    benchmark_array_operations,
    benchmark_array_conversions,
    benchmark_object_stream,
    benchmark_xref_operations,
    benchmark_dictionary_operations,
    benchmark_string_name_operations
);
criterion_main!(core_benches);
