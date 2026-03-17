//! Memory system benchmarks for OpenRustClaw.
//!
//! Measures:
//! - Store/retrieve latency
//! - Search throughput (BM25 + vector)
//! - Hybrid search performance
//! - Cache hit/miss ratios

use std::collections::HashMap;

use chrono::Utc;
use criterion::{
    criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion, Throughput,
};
use openrustclaw_core::types::{
    CoreEntry, MemoryEntry, MemoryQuery, MemorySource, MemoryType, ScoredMemory, SourceType,
};
use openrustclaw_memory::{
    core_memory::CoreMemoryManager, policies::MemoryPolicies, search, ContextManager, RecallMemory,
};
use uuid::Uuid;

/// Generate a test memory entry with variable content size
fn generate_memory_entry(content_size: usize, memory_type: MemoryType) -> MemoryEntry {
    let content: String = (0..content_size)
        .map(|i| format!("word{} ", i % 1000))
        .collect();

    MemoryEntry {
        id: Uuid::new_v4(),
        memory_type,
        content: content.trim().to_string(),
        content_hash: format!("hash_{}", Uuid::new_v4()),
        source: Some("benchmark".to_string()),
        source_type: Some(SourceType::Document),
        session_id: Some(Uuid::new_v4()),
        user_id: Some("benchmark_user".to_string()),
        namespace: "benchmark".to_string(),
        importance: rand::random::<f32>(),
        confidence: 0.9,
        access_count: 0,
        last_accessed: None,
        created_at: Utc::now(),
        expires_at: None,
        metadata: serde_json::json!({}),
    }
}

/// Generate a test core entry
fn generate_core_entry(key_len: usize, value_len: usize) -> CoreEntry {
    let key: String = (0..key_len).map(|_| 'k').collect();
    let value: String = (0..value_len).map(|_| 'v').collect();
    let token_count = (key.len() + value.len() + 2) / 4 + 1;

    CoreEntry {
        key,
        value,
        importance: rand::random::<f32>(),
        token_count,
        updated_at: Utc::now(),
    }
}

/// Generate scored memory results for RRF benchmark
fn generate_scored_memories(count: usize, prefix: &str) -> Vec<ScoredMemory> {
    (0..count)
        .map(|i| {
            let content = format!("{} content about topic {}", prefix, i);
            ScoredMemory {
                entry: MemoryEntry {
                    id: Uuid::new_v4(),
                    memory_type: MemoryType::Semantic,
                    content,
                    content_hash: format!("hash_{}_{}", prefix, i),
                    source: Some("benchmark".to_string()),
                    source_type: None,
                    session_id: None,
                    user_id: None,
                    namespace: "benchmark".to_string(),
                    importance: 0.8,
                    confidence: 0.9,
                    access_count: i as u32,
                    last_accessed: None,
                    created_at: Utc::now(),
                    expires_at: None,
                    metadata: serde_json::json!({}),
                },
                score: 1.0 - (i as f32 / count as f32),
            }
        })
        .collect()
}

// =============================================================================
// Core Memory Benchmarks
// =============================================================================

fn bench_core_memory_render(c: &mut Criterion) {
    let mut group = c.benchmark_group("core_memory/render");

    for entry_count in [5, 10, 20] {
        group.bench_with_input(
            BenchmarkId::from_parameter(entry_count),
            &entry_count,
            |b, &count| {
                let entries: Vec<CoreEntry> = (0..count)
                    .map(|i| generate_core_entry(10, 50 + i * 10))
                    .collect();

                b.iter(|| {
                    let result = CoreMemoryManager::render(&entries);
                    criterion::black_box(result);
                });
            },
        );
    }

    group.finish();
}

fn bench_core_memory_budget_check(c: &mut Criterion) {
    let mut group = c.benchmark_group("core_memory/budget_check");

    let manager = CoreMemoryManager::new(500, 20);

    for entry_count in [5, 10, 20] {
        group.bench_with_input(
            BenchmarkId::from_parameter(entry_count),
            &entry_count,
            |b, &count| {
                let entries: Vec<CoreEntry> = (0..count)
                    .map(|i| CoreEntry {
                        key: format!("key_{}", i),
                        value: format!("value with some content {}", i),
                        importance: 0.5,
                        token_count: 10,
                        updated_at: Utc::now(),
                    })
                    .collect();

                let new_entry = generate_core_entry(10, 100);

                b.iter(|| {
                    let result = manager.would_exceed_budget(&entries, &new_entry);
                    criterion::black_box(result);
                });
            },
        );
    }

    group.finish();
}

fn bench_core_memory_eviction(c: &mut Criterion) {
    let mut group = c.benchmark_group("core_memory/eviction");

    for entry_count in [10, 50, 100] {
        group.bench_with_input(
            BenchmarkId::from_parameter(entry_count),
            &entry_count,
            |b, &count| {
                let entries: Vec<CoreEntry> = (0..count)
                    .map(|i| CoreEntry {
                        key: format!("key_{}", i),
                        value: format!("value {}", i),
                        importance: rand::random::<f32>(),
                        token_count: 10,
                        updated_at: Utc::now(),
                    })
                    .collect();

                b.iter(|| {
                    let result = CoreMemoryManager::eviction_candidate(&entries);
                    criterion::black_box(result);
                });
            },
        );
    }

    group.finish();
}

// =============================================================================
// Search Benchmarks
// =============================================================================

fn bench_reciprocal_rank_fusion(c: &mut Criterion) {
    let mut group = c.benchmark_group("search/reciprocal_rank_fusion");

    for result_count in [10, 50, 100, 500] {
        group.throughput(Throughput::Elements(result_count as u64));

        group.bench_with_input(
            BenchmarkId::from_parameter(result_count),
            &result_count,
            |b, &count| {
                let bm25_results = generate_scored_memories(count, "bm25");
                let vector_results = generate_scored_memories(count, "vector");

                b.iter(|| {
                    let fused = search::reciprocal_rank_fusion(
                        bm25_results.clone(),
                        vector_results.clone(),
                        60.0,
                    );
                    criterion::black_box(fused);
                });
            },
        );
    }

    group.finish();
}

fn bench_mmr_rerank(c: &mut Criterion) {
    let mut group = c.benchmark_group("search/mmr_rerank");

    for (total_count, limit) in [(50, 10), (100, 20), (500, 50)] {
        group.throughput(Throughput::Elements(total_count as u64));

        group.bench_with_input(
            BenchmarkId::new("total", total_count),
            &(total_count, limit),
            |b, &(total, lim)| {
                let results = generate_scored_memories(total, "mmr");

                b.iter(|| {
                    let reranked = search::mmr_rerank(results.clone(), lim, 0.5);
                    criterion::black_box(reranked);
                });
            },
        );
    }

    group.finish();
}

// =============================================================================
// Memory Policies Benchmarks
// =============================================================================

fn bench_content_hash(c: &mut Criterion) {
    let mut group = c.benchmark_group("policies/content_hash");

    for content_size in [100, 1000, 10000, 100000] {
        group.throughput(Throughput::Bytes(content_size as u64));

        group.bench_with_input(
            BenchmarkId::from_parameter(content_size),
            &content_size,
            |b, &size| {
                let content: String = (0..size).map(|_| 'x').collect();

                b.iter(|| {
                    let hash = MemoryPolicies::content_hash(&content);
                    criterion::black_box(hash);
                });
            },
        );
    }

    group.finish();
}

fn bench_cosine_similarity(c: &mut Criterion) {
    let mut group = c.benchmark_group("policies/cosine_similarity");

    for dim in [128, 512, 1024, 4096] {
        group.throughput(Throughput::Elements(dim as u64));

        group.bench_with_input(
            BenchmarkId::from_parameter(dim),
            &dim,
            |b, &d| {
                let vec_a: Vec<f32> = (0..d).map(|i| (i as f32).sin()).collect();
                let vec_b: Vec<f32> = (0..d).map(|i| (i as f32).cos()).collect();

                b.iter(|| {
                    let similarity = MemoryPolicies::cosine_similarity(&vec_a, &vec_b);
                    criterion::black_box(similarity);
                });
            },
        );
    }

    group.finish();
}

fn bench_decay_score(c: &mut Criterion) {
    let mut group = c.benchmark_group("policies/decay_score");

    let policies = MemoryPolicies::default();

    group.bench_function("various_ages", |b| {
        b.iter(|| {
            for days in [1.0, 7.0, 30.0, 90.0, 365.0] {
                let score = policies.decay_score(1.0, days);
                criterion::black_box(score);
            }
        });
    });

    group.finish();
}

// =============================================================================
// Context Manager Benchmarks
// =============================================================================

fn bench_context_build(c: &mut Criterion) {
    let mut group = c.benchmark_group("context/build");

    for msg_count in [10, 50, 100] {
        group.bench_with_input(
            BenchmarkId::from_parameter(msg_count),
            &msg_count,
            |b, &count| {
                let manager = ContextManager::new(8000);
                let core_memory: Vec<CoreEntry> =
                    (0..5).map(|i| generate_core_entry(10, 50)).collect();

                let messages: Vec<openrustclaw_core::types::Message> = (0..count)
                    .map(|i| openrustclaw_core::types::Message::user(format!("Message {}", i)))
                    .collect();

                b.iter(|| {
                    let ctx = manager.build("You are a helpful assistant.", &core_memory, &[], &messages);
                    criterion::black_box(ctx);
                });
            },
        );
    }

    group.finish();
}

fn bench_estimate_tokens(c: &mut Criterion) {
    let mut group = c.benchmark_group("context/estimate_tokens");

    for text_size in [100, 1000, 10000, 100000] {
        group.throughput(Throughput::Bytes(text_size as u64));

        group.bench_with_input(
            BenchmarkId::from_parameter(text_size),
            &text_size,
            |b, &size| {
                let text: String = (0..size).map(|_| 'x').collect();

                b.iter(|| {
                    let tokens = ContextManager::estimate_tokens(&text);
                    criterion::black_box(tokens);
                });
            },
        );
    }

    group.finish();
}

// =============================================================================
// Recall Memory Benchmarks
// =============================================================================

fn bench_prepare_entry(c: &mut Criterion) {
    let mut group = c.benchmark_group("recall/prepare_entry");

    let policies = MemoryPolicies::default();
    let recall = RecallMemory::new(policies);

    for content_size in [100, 1000, 10000] {
        group.throughput(Throughput::Bytes(content_size as u64));

        group.bench_with_input(
            BenchmarkId::from_parameter(content_size),
            &content_size,
            |b, &size| {
                let content: String = (0..size).map(|_| 'a').collect();

                b.iter(|| {
                    let entry = recall.prepare_entry(
                        &content,
                        MemoryType::Semantic,
                        MemorySource::BackgroundIngestion,
                        Some("user123"),
                        Some(Uuid::new_v4()),
                        Some("default"),
                    );
                    criterion::black_box(entry);
                });
            },
        );
    }

    group.finish();
}

fn bench_apply_decay(c: &mut Criterion) {
    let mut group = c.benchmark_group("recall/apply_decay");

    let policies = MemoryPolicies::default();
    let recall = RecallMemory::new(policies);

    for result_count in [10, 100, 1000] {
        group.throughput(Throughput::Elements(result_count as u64));

        group.bench_with_input(
            BenchmarkId::from_parameter(result_count),
            &result_count,
            |b, &count| {
                let mut results = generate_scored_memories(count, "decay");

                b.iter(|| {
                    let mut r = results.clone();
                    recall.apply_decay(&mut r);
                    criterion::black_box(r);
                });
            },
        );
    }

    group.finish();
}

// =============================================================================
// Cache Simulation Benchmarks
// =============================================================================

fn bench_cache_simulation(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache/simulation");

    // Simulate a cache with different hit rates
    for cache_size in [100, 1000, 10000] {
        group.bench_with_input(
            BenchmarkId::new("lru", cache_size),
            &cache_size,
            |b, &size| {
                use dashmap::DashMap;

                let cache: DashMap<String, MemoryEntry> = DashMap::with_capacity(size);

                // Pre-populate cache
                for i in 0..size {
                    let entry = generate_memory_entry(100, MemoryType::Semantic);
                    cache.insert(format!("key_{}", i), entry);
                }

                b.iter(|| {
                    // Mix of hits and misses
                    let key = format!("key_{}", rand::random::<usize>() % (size * 2));
                    let result = cache.get(&key).is_some();
                    criterion::black_box(result);
                });
            },
        );
    }

    group.finish();
}

// =============================================================================
// Criterion Groups
// =============================================================================

criterion_group!(
    memory_benches,
    bench_core_memory_render,
    bench_core_memory_budget_check,
    bench_core_memory_eviction,
    bench_reciprocal_rank_fusion,
    bench_mmr_rerank,
    bench_content_hash,
    bench_cosine_similarity,
    bench_decay_score,
    bench_context_build,
    bench_estimate_tokens,
    bench_prepare_entry,
    bench_apply_decay,
    bench_cache_simulation,
);

criterion_main!(memory_benches);
