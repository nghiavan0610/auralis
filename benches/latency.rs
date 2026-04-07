//! Latency benchmarks for Auralis components
//!
//! This module measures the latency of various operations including:
//! - Configuration operations
//! - Model initialization
//! - Component creation
//! - Serialization/deserialization

use auralis::domain::models::{STTSegment, Translation};
use auralis::infrastructure::container::{AuralisContainer, ContainerConfig};
use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use std::path::PathBuf;

/// Benchmark configuration operations
fn bench_config_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("config_operations");

    // Benchmark default config creation
    group.bench_function("default_config", |b| {
        b.iter(|| {
            let config = ContainerConfig::default();
            black_box(config)
        })
    });

    // Benchmark config with custom languages
    group.bench_function("config_with_languages", |b| {
        b.iter(|| {
            let config = ContainerConfig::default()
                .with_languages("zh".to_string(), "fr".to_string());
            black_box(config)
        })
    });

    // Benchmark config with custom models dir
    group.bench_function("config_with_models_dir", |b| {
        b.iter(|| {
            let config = ContainerConfig::default()
                .with_models_dir(PathBuf::from("/tmp/models"));
            black_box(config)
        })
    });

    // Benchmark path resolution
    group.bench_function("config_resolve_paths", |b| {
        b.iter(|| {
            let mut config = ContainerConfig::default();
            config.resolve_model_paths();
            black_box(config)
        })
    });

    group.finish();
}

/// Benchmark container operations
fn bench_container_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("container_operations");

    // Benchmark container creation
    group.bench_function("container_new", |b| {
        b.iter(|| {
            let container = AuralisContainer::new();
            black_box(container)
        })
    });

    // Benchmark container with config
    group.bench_function("container_with_config", |b| {
        b.iter(|| {
            let config = ContainerConfig::default();
            let container = AuralisContainer::with_config(config);
            black_box(container)
        })
    });

    // Benchmark model checking
    group.bench_function("container_check_models", |b| {
        let container = AuralisContainer::new();
        b.iter(|| {
            let status = container.check_models();
            black_box(status)
        })
    });

    // Benchmark config retrieval
    group.bench_function("container_get_config", |b| {
        let container = AuralisContainer::new();
        b.iter(|| {
            let config = container.config();
            black_box(config)
        })
    });

    group.finish();
}

/// Benchmark serialization operations
fn bench_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("serialization");

    // Benchmark STT segment serialization
    let stt_segment = STTSegment::new(
        "Hello world, this is a test of the speech to text system".to_string(),
        0.95,
        0,
        1000,
        true,
    );

    group.bench_function("stt_segment_serialize", |b| {
        b.iter(|| {
            let json = serde_json::to_string(&stt_segment).unwrap();
            black_box(json)
        })
    });

    group.bench_function("stt_segment_deserialize", |b| {
        let json = serde_json::to_string(&stt_segment).unwrap();
        b.iter(|| {
            let segment: STTSegment = serde_json::from_str(&json).unwrap();
            black_box(segment)
        })
    });

    // Benchmark Translation serialization
    let translation = Translation::new(
        "en".to_string(),
        "es".to_string(),
        "Hello world, this is a comprehensive test".to_string(),
        "Hola mundo, esta es una prueba completa".to_string(),
        0.92,
    );

    group.bench_function("translation_serialize", |b| {
        b.iter(|| {
            let json = serde_json::to_string(&translation).unwrap();
            black_box(json)
        })
    });

    group.bench_function("translation_deserialize", |b| {
        let json = serde_json::to_string(&translation).unwrap();
        b.iter(|| {
            let translation: Translation = serde_json::from_str(&json).unwrap();
            black_box(translation)
        })
    });

    group.finish();
}

/// Benchmark model validation operations
fn bench_validation(c: &mut Criterion) {
    let mut group = c.benchmark_group("validation");

    // Benchmark STT segment validation
    let valid_stt = STTSegment::new("Valid text".to_string(), 0.85, 0, 500, true);

    group.bench_function("stt_segment_validate", |b| {
        b.iter(|| {
            let result = valid_stt.validate();
            black_box(result)
        })
    });

    // Benchmark Translation validation
    let valid_translation = Translation::new(
        "en".to_string(),
        "zh".to_string(),
        "Hello world".to_string(),
        "你好世界".to_string(),
        0.9,
    );

    group.bench_function("translation_validate", |b| {
        b.iter(|| {
            let result = valid_translation.validate();
            black_box(result)
        })
    });

    group.finish();
}

/// Benchmark with different data sizes
fn bench_data_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("data_sizes");

    let text_sizes = vec![
        ("small", "Hello"),
        ("medium", "Hello world, this is a test of the system"),
        ("large", "Hello world, this is a comprehensive test of the speech to text and translation system that handles multiple languages and provides real-time processing capabilities"),
    ];

    for (size_name, text) in text_sizes {
        // Benchmark STT segment creation
        group.bench_with_input(BenchmarkId::new("stt_segment_creation", size_name), text, |b, text| {
            b.iter(|| {
                let segment = STTSegment::new(
                    text.to_string(),
                    0.95,
                    0,
                    1000,
                    true,
                );
                black_box(segment)
            })
        });

        // Benchmark STT segment serialization
        let segment = STTSegment::new(text.to_string(), 0.95, 0, 1000, true);
        group.bench_with_input(BenchmarkId::new("stt_segment_serialize", size_name), &segment, |b, segment| {
            b.iter(|| {
                let json = serde_json::to_string(&segment).unwrap();
                black_box(json)
            })
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_config_operations,
    bench_container_operations,
    bench_serialization,
    bench_validation,
    bench_data_sizes
);

criterion_main!(benches);
