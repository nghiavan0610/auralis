//! Memory benchmarks for Auralis components
//!
//! This module measures memory usage patterns including:
//! - Object allocation sizes
//! - Collection growth patterns
//! - Memory churn during operations
//! - Stack vs heap allocation patterns

use auralis::domain::models::{STTSegment, Translation};
use auralis::infrastructure::container::{AuralisContainer, ContainerConfig, ModelStatus};
use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use std::mem;

/// Benchmark memory allocation for basic types
fn bench_memory_allocation(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_allocation");

    // Measure STT segment allocation
    group.bench_function("stt_segment_allocation", |b| {
        b.iter(|| {
            let segment = STTSegment::new(
                "Hello world, this is a test of memory allocation".to_string(),
                0.95,
                0,
                1000,
                true,
            );
            black_box(segment);
            black_box(mem::size_of_val(&segment));
        })
    });

    // Measure Translation allocation
    group.bench_function("translation_allocation", |b| {
        b.iter(|| {
            let translation = Translation::new(
                "en".to_string(),
                "es".to_string(),
                "Hello world, this is a test of memory allocation".to_string(),
                "Hola mundo, esta es una prueba de asignación de memoria".to_string(),
                0.92,
            );
            black_box(translation);
            black_box(mem::size_of_val(&translation));
        })
    });

    // Measure ContainerConfig allocation
    group.bench_function("container_config_allocation", |b| {
        b.iter(|| {
            let config = ContainerConfig::default();
            black_box(config);
            black_box(mem::size_of_val(&config));
        })
    });

    // Measure ModelStatus allocation
    group.bench_function("model_status_allocation", |b| {
        b.iter(|| {
            let status = ModelStatus::default();
            black_box(status);
            black_box(mem::size_of_val(&status));
        })
    });

    group.finish();
}

/// Benchmark vector operations with different capacities
fn bench_vector_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("vector_operations");

    let sizes = vec![10, 100, 1000, 10000];

    for size in sizes {
        // Benchmark vector creation with segments
        group.bench_with_input(BenchmarkId::new("create_stt_segments", size), &size, |b, &size| {
            b.iter(|| {
                let mut segments = Vec::with_capacity(size);
                for i in 0..size {
                    let segment = STTSegment::new(
                        format!("Segment number {}", i),
                        0.9,
                        i * 100,
                        (i + 1) * 100,
                        true,
                    );
                    segments.push(segment);
                }
                black_box(segments);
            })
        });

        // Benchmark vector creation with translations
        group.bench_with_input(BenchmarkId::new("create_translations", size), &size, |b, &size| {
            b.iter(|| {
                let mut translations = Vec::with_capacity(size);
                for i in 0..size {
                    let translation = Translation::new(
                        "en".to_string(),
                        "es".to_string(),
                        format!("Text number {}", i),
                        format!("Texto número {}", i),
                        0.9,
                    );
                    translations.push(translation);
                }
                black_box(translations);
            })
        });

        // Benchmark vector iteration
        let segments: Vec<STTSegment> = (0..size)
            .map(|i| STTSegment::new(
                format!("Segment {}", i),
                0.9,
                i * 100,
                (i + 1) * 100,
                true,
            ))
            .collect();

        group.bench_with_input(BenchmarkId::new("iterate_stt_segments", size), &size, |b, _| {
            b.iter(|| {
                let mut count = 0;
                for segment in &segments {
                    black_box(segment);
                    count += 1;
                }
                black_box(count);
            })
        });
    }

    group.finish();
}

/// Benchmark string operations
fn bench_string_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("string_operations");

    // Benchmark string cloning
    let text = "Hello world, this is a comprehensive test string for memory benchmarking";

    group.bench_function("string_clone", |b| {
        let owned = text.to_string();
        b.iter(|| {
            let cloned = owned.clone();
            black_box(cloned);
        })
    });

    // Benchmark string formatting
    group.bench_function("string_format", |b| {
        b.iter(|| {
            let formatted = format!("Source: {}, Target: {}, Text: {}", "en", "es", text);
            black_box(formatted);
        })
    });

    // Benchmark repeated concatenation
    group.bench_function("repeated_concatenation", |b| {
        b.iter(|| {
            let mut result = String::new();
            for i in 0..10 {
                result.push_str(&format!("Segment {}: ", i));
                result.push_str(text);
                result.push_str(" | ");
            }
            black_box(result);
        })
    });

    group.finish();
}

/// Benchmark JSON serialization throughput
fn bench_serialization_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("serialization_throughput");

    // Create a large collection of segments
    let segments: Vec<STTSegment> = (0..1000)
        .map(|i| STTSegment::new(
            format!("This is segment number {} with some sample text to simulate real-world usage", i),
            0.9,
            i * 100,
            (i + 1) * 100,
            true,
        ))
        .collect();

    group.throughput(Throughput::Elements(segments.len() as u64));

    group.bench_function("serialize_1000_segments", |b| {
        b.iter(|| {
            let json = serde_json::to_string(&segments).unwrap();
            black_box(json);
        })
    });

    // Create a large collection of translations
    let translations: Vec<Translation> = (0..1000)
        .map(|i| Translation::new(
            "en".to_string(),
            "es".to_string(),
            format!("Original text number {}", i),
            format!("Texto traducido número {}", i),
            0.9,
        ))
        .collect();

    group.throughput(Throughput::Elements(translations.len() as u64));

    group.bench_function("serialize_1000_translations", |b| {
        b.iter(|| {
            let json = serde_json::to_string(&translations).unwrap();
            black_box(json);
        })
    });

    group.finish();
}

/// Benchmark memory pressure scenarios
fn bench_memory_pressure(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_pressure");

    // Simulate rapid allocation and deallocation
    group.bench_function("rapid_allocation_deallocation", |b| {
        b.iter(|| {
            for i in 0..100 {
                let segment = STTSegment::new(
                    format!("Temporary segment {}", i),
                    0.9,
                    i * 10,
                    i * 10 + 50,
                    true,
                );
                black_box(segment);
                // segment is dropped here
            }
        })
    });

    // Simulate growing collections
    group.bench_function("growing_vector", |b| {
        b.iter(|| {
            let mut segments = Vec::new();
            for i in 0..1000 {
                let segment = STTSegment::new(
                    format!("Segment {}", i),
                    0.9,
                    i,
                    i + 1,
                    true,
                );
                segments.push(segment);
            }
            black_box(segments);
        })
    });

    // Simulate collection operations
    group.bench_function("filter_and_collect", |b| {
        let segments: Vec<STTSegment> = (0..1000)
            .map(|i| STTSegment::new(
                format!("Segment {}", i),
                if i % 2 == 0 { 0.9 } else { 0.7 },
                i * 10,
                i * 10 + 50,
                true,
            ))
            .collect();

        b.iter(|| {
            let filtered: Vec<_> = segments.iter()
                .filter(|s| s.confidence > 0.8)
                .collect();
            black_box(filtered);
        })
    });

    group.finish();
}

/// Benchmark stack vs heap allocation patterns
fn bench_allocation_patterns(c: &mut Criterion) {
    let mut group = c.benchmark_group("allocation_patterns");

    // Small strings that may be stored on stack (with SmallString optimization)
    group.bench_function("small_string_allocation", |b| {
        b.iter(|| {
            let text = "Hi";
            let segment = STTSegment::new(
                text.to_string(),
                0.9,
                0,
                50,
                true,
            );
            black_box(segment);
        })
    });

    // Large strings that require heap allocation
    group.bench_function("large_string_allocation", |b| {
        b.iter(|| {
            let text = "This is a very long string that will definitely require heap allocation because it exceeds the typical inline string optimization limits";
            let segment = STTSegment::new(
                text.to_string(),
                0.9,
                0,
                5000,
                true,
            );
            black_box(segment);
        })
    });

    // Array of stack-allocated values
    group.bench_function("stack_array_allocation", |b| {
        b.iter(|| {
            let values: [u64; 10] = [0, 100, 200, 300, 400, 500, 600, 700, 800, 900];
            black_box(values);
        })
    });

    // Vec of heap-allocated values
    group.bench_function("heap_vec_allocation", |b| {
        b.iter(|| {
            let values: Vec<u64> = vec![0, 100, 200, 300, 400, 500, 600, 700, 800, 900];
            black_box(values);
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_memory_allocation,
    bench_vector_operations,
    bench_string_operations,
    bench_serialization_throughput,
    bench_memory_pressure,
    bench_allocation_patterns
);

criterion_main!(benches);
