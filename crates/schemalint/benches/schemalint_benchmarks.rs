use std::fs;
use std::path::PathBuf;

use criterion::{criterion_group, criterion_main, Criterion};

use schemalint::cache::{hash_bytes, Cache};
use schemalint::normalize::normalize;
use schemalint::profile::load;
use schemalint::rules::registry::RuleSet;

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("benches/fixtures")
        .join(name)
}

fn load_profile() -> schemalint::profile::Profile {
    let bytes = fs::read(fixture_path("openai_profile.toml")).unwrap();
    load(&bytes).unwrap()
}

fn bench_single_schema(c: &mut Criterion) {
    let profile = load_profile();
    let ruleset = RuleSet::from_profile(&profile);
    let bytes = fs::read(fixture_path("single_large_schema.json")).unwrap();
    let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();

    let mut group = c.benchmark_group("single_schema");
    group.bench_function("parse_normalize_and_lint", |b| {
        b.iter_batched(
            || value.clone(),
            |v| {
                let normalized = normalize(v).unwrap();
                let _diags = ruleset.check_all(&normalized.arena, &profile);
            },
            criterion::BatchSize::SmallInput,
        )
    });
    group.finish();
}

fn bench_cold_start(c: &mut Criterion) {
    let profile = load_profile();
    let ruleset = RuleSet::from_profile(&profile);
    let schemas_dir = fixture_path("project_500_schemas");

    let mut schema_bytes = Vec::new();
    for entry in fs::read_dir(&schemas_dir).unwrap() {
        let path = entry.unwrap().path();
        let bytes = fs::read(&path).unwrap();
        schema_bytes.push(bytes);
    }
    schema_bytes.sort();

    let mut group = c.benchmark_group("cold_start");
    group.bench_function("500_schemas_no_cache", |b| {
        b.iter(|| {
            for bytes in &schema_bytes {
                let value: serde_json::Value = serde_json::from_slice(bytes).unwrap();
                let normalized = normalize(value).unwrap();
                let _diags = ruleset.check_all(&normalized.arena, &profile);
            }
        })
    });
    group.finish();
}

fn bench_incremental(c: &mut Criterion) {
    let profile = load_profile();
    let ruleset = RuleSet::from_profile(&profile);
    let schemas_dir = fixture_path("project_500_schemas");

    let mut schema_bytes = Vec::new();
    for entry in fs::read_dir(&schemas_dir).unwrap() {
        let path = entry.unwrap().path();
        let bytes = fs::read(&path).unwrap();
        schema_bytes.push(bytes);
    }
    schema_bytes.sort();

    let mut cache = Cache::new();
    for bytes in &schema_bytes[..schema_bytes.len() - 1] {
        let value: serde_json::Value = serde_json::from_slice(bytes).unwrap();
        let normalized = normalize(value).unwrap();
        cache.insert(hash_bytes(bytes), bytes.to_vec(), normalized);
    }

    let mut group = c.benchmark_group("incremental");
    group.bench_function("500_schemas_one_changed_with_cache", |b| {
        b.iter(|| {
            for bytes in &schema_bytes {
                let hash = hash_bytes(bytes);
                let _diags = if let Some(cached) = cache.get(hash, bytes) {
                    ruleset.check_all(&cached.arena, &profile)
                } else {
                    let value: serde_json::Value = serde_json::from_slice(bytes).unwrap();
                    let normalized = normalize(value).unwrap();
                    ruleset.check_all(&normalized.arena, &profile)
                };
            }
        })
    });
    group.finish();
}

criterion_group!(
    benches,
    bench_single_schema,
    bench_cold_start,
    bench_incremental
);
criterion_main!(benches);
