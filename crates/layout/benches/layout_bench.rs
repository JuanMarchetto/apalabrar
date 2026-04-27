use criterion::{Criterion, criterion_group, criterion_main};

fn placeholder_bench(c: &mut Criterion) {
    c.bench_function("placeholder", |b| b.iter(|| 1 + 1));
}

criterion_group!(benches, placeholder_bench);
criterion_main!(benches);
