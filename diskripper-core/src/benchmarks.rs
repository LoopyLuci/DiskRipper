//! Performance benchmarks for DiskRipper.
//!
//! Run with: cargo bench --workspace

use std::time::Instant;

/// Benchmark result
#[derive(Debug, Clone, serde::Serialize)]
pub struct BenchmarkResult {
    pub name: String,
    pub duration_ms: f64,
    pub throughput_mbps: f64,
    pub iterations: usize,
}

/// Run all benchmarks
pub fn run_benchmarks() -> Vec<BenchmarkResult> {
    let mut results = Vec::new();

    // Audio fingerprinting benchmark
    results.push(benchmark_audio_fingerprinting());

    // Feature extraction benchmark
    results.push(benchmark_feature_extraction());

    // Checksum computation benchmark
    results.push(benchmark_checksum());

    // Parallel extraction benchmark
    results.push(benchmark_parallel_extraction());

    results
}

fn benchmark_audio_fingerprinting() -> BenchmarkResult {
    use crate::ml::audio_fingerprint::AudioFingerprinter;
    
    let start = Instant::now();
    let iterations = 100;
    
    // Generate synthetic audio
    let sample_rate = 44100;
    let duration_secs = 10;
    let num_samples = sample_rate * duration_secs;
    let audio: Vec<i16> = (0..num_samples)
        .map(|i| {
            let t = i as f64 / sample_rate as f64;
            let sample = (t * 440.0 * 2.0 * std::f64::consts::PI).sin();
            (sample * i16::MAX as f64) as i16
        })
        .collect();
    
    let fingerprinter = AudioFingerprinter::new_with_model_dir(&std::path::PathBuf::from("/tmp"));
    
    for _ in 0..iterations {
        let _ = fingerprinter.generate(&audio, sample_rate);
    }
    
    let duration = start.elapsed();
    let duration_ms = duration.as_millis() as f64;
    let throughput_mbps = (iterations * num_samples * 2) as f64 / (duration.as_secs_f64() * 1_000_000.0);
    
    BenchmarkResult {
        name: "audio_fingerprinting".to_string(),
        duration_ms,
        throughput_mbps,
        iterations,
    }
}

fn benchmark_feature_extraction() -> BenchmarkResult {
    use crate::ml::feature_extraction::AudioFeatureExtractor;
    
    let start = Instant::now();
    let iterations = 100;
    
    let sample_rate = 44100;
    let duration_secs = 10;
    let num_samples = sample_rate * duration_secs;
    let audio: Vec<i16> = (0..num_samples)
        .map(|i| {
            let t = i as f64 / sample_rate as f64;
            let sample = (t * 440.0 * 2.0 * std::f64::consts::PI).sin();
            (sample * i16::MAX as f64) as i16
        })
        .collect();
    
    for _ in 0..iterations {
        let _ = AudioFeatureExtractor::extract(&audio, sample_rate);
    }
    
    let duration = start.elapsed();
    let duration_ms = duration.as_millis() as f64;
    let throughput_mbps = (iterations * num_samples * 2) as f64 / (duration.as_secs_f64() * 1_000_000.0);
    
    BenchmarkResult {
        name: "feature_extraction".to_string(),
        duration_ms,
        throughput_mbps,
        iterations,
    }
}

fn benchmark_checksum() -> BenchmarkResult {
    use crc32fast::Hasher;
    
    let start = Instant::now();
    let iterations = 1000;
    let data_size = 1024 * 1024; // 1MB
    let data = vec![0u8; data_size];
    
    for _ in 0..iterations {
        let mut hasher = Hasher::new();
        hasher.update(&data);
        let _ = hasher.finalize();
    }
    
    let duration = start.elapsed();
    let duration_ms = duration.as_millis() as f64;
    let throughput_mbps = (iterations * data_size) as f64 / (duration.as_secs_f64() * 1_000_000.0);
    
    BenchmarkResult {
        name: "checksum_crc32".to_string(),
        duration_ms,
        throughput_mbps,
        iterations,
    }
}

fn benchmark_parallel_extraction() -> BenchmarkResult {
    use rayon::prelude::*;
    
    let start = Instant::now();
    let iterations = 100;
    let num_files = 1000;
    let file_size = 1024; // 1KB each
    
    let files: Vec<Vec<u8>> = (0..num_files)
        .map(|i| vec![i as u8; file_size])
        .collect();
    
    for _ in 0..iterations {
        let _: Vec<u32> = files.par_iter()
            .map(|data| {
                let mut hasher = crc32fast::Hasher::new();
                hasher.update(data);
                hasher.finalize()
            })
            .collect();
    }
    
    let duration = start.elapsed();
    let duration_ms = duration.as_millis() as f64;
    let throughput_mbps = (iterations * num_files * file_size) as f64 / (duration.as_secs_f64() * 1_000_000.0);
    
    BenchmarkResult {
        name: "parallel_extraction".to_string(),
        duration_ms,
        throughput_mbps,
        iterations,
    }
}
