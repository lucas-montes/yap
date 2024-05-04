use criterion::criterion_main;

mod benchmarks;

criterion_main! {
    benchmarks::read_and_compress::benches,
}
