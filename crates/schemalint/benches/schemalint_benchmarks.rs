use criterion::{criterion_group, criterion_main};

fn bench_single_schema(c: &mut criterion::Criterion) {
    c.bench_function("single_schema", |b| {
        b.iter(|| {})
    });
}

criterion_group!(benches, bench_single_schema);
criterion_main!(benches);
