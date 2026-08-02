use criterion::{Criterion, criterion_group, criterion_main};
use vibea3::lz77::{CompressionLevel, compress};

fn lz77(c: &mut Criterion) {
    let sample = std::env::var("VIBEA3_KBIN_SAMPLE")
        .unwrap_or_else(|_| "target/official-slices/musicdb-official-full.kbin".into());
    let data = std::fs::read(sample)
        .or_else(|_| {
            std::fs::read(
                std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/musicdb.xml"),
            )
        })
        .unwrap_or_else(|_| vec![b'a'; 64 * 1024]);
    let mut group = c.benchmark_group("lz77-packet");
    group.throughput(criterion::Throughput::Bytes(data.len() as u64));
    for level in [
        CompressionLevel::Fast,
        CompressionLevel::Balanced,
        CompressionLevel::Best,
    ] {
        group.bench_with_input(format!("{level:?}"), &level, |b, level| {
            b.iter(|| compress(&data, *level))
        });
    }
    group.finish();
}

criterion_group!(benches, lz77);
criterion_main!(benches);
