use std::{env, fs, time::Instant};

use vibea3::lz77::{CompressionLevel, compress};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    for path in env::args().skip(1) {
        let input = fs::read(&path)?;
        println!("{path}\tinput={}", input.len());
        for level in [
            CompressionLevel::Fast,
            CompressionLevel::Balanced,
            CompressionLevel::Best,
        ] {
            let started = Instant::now();
            let mut output = Vec::new();
            for _ in 0..3 {
                output = compress(&input, level);
            }
            println!(
                "  {level:?}\tbytes={}\tratio={:.4}\tavg_ms={:.3}",
                output.len(),
                output.len() as f64 / input.len().max(1) as f64,
                started.elapsed().as_secs_f64() * 1000.0 / 3.0,
            );
        }
    }
    Ok(())
}
