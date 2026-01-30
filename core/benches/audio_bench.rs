//! Benchmarks for audio processing operations.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use phobz_visualizer::audio::synth::{generate_sine, generate_test_beat, generate_white_noise};
use phobz_visualizer::audio::{analyze_audio, detect_beats, SpectrumAnalyzer};

const SAMPLE_RATE: u32 = 44100;

fn bench_fft_analysis(c: &mut Criterion) {
    let mut group = c.benchmark_group("FFT Analysis");

    for fft_size in [512, 1024, 2048, 4096] {
        let samples = generate_sine(1000.0, SAMPLE_RATE, 1.0, 1.0);

        group.throughput(Throughput::Elements(fft_size as u64));
        group.bench_with_input(BenchmarkId::new("analyze", fft_size), &fft_size, |b, &size| {
            let mut analyzer = SpectrumAnalyzer::new(size);
            b.iter(|| {
                black_box(analyzer.analyze(&samples));
            });
        });
    }

    group.finish();
}

fn bench_spectrum_bands(c: &mut Criterion) {
    let mut group = c.benchmark_group("Spectrum Bands");

    let samples = generate_white_noise(SAMPLE_RATE, 1.0, 1.0, 42);
    let mut analyzer = SpectrumAnalyzer::new(2048);

    for num_bands in [16, 32, 64, 128] {
        group.bench_with_input(
            BenchmarkId::new("analyze_bands", num_bands),
            &num_bands,
            |b, &bands| {
                b.iter(|| {
                    black_box(analyzer.analyze_bands(&samples, SAMPLE_RATE, bands));
                });
            },
        );
    }

    group.finish();
}

fn bench_beat_detection(c: &mut Criterion) {
    let mut group = c.benchmark_group("Beat Detection");

    for duration in [2.0, 5.0, 10.0] {
        let samples = generate_test_beat(120.0, SAMPLE_RATE, duration);
        let num_samples = samples.len();

        group.throughput(Throughput::Elements(num_samples as u64));
        group.bench_with_input(
            BenchmarkId::new("detect_beats", format!("{}s", duration)),
            &samples,
            |b, samples| {
                b.iter(|| {
                    black_box(detect_beats(samples, SAMPLE_RATE, 0.3));
                });
            },
        );
    }

    group.finish();
}

fn bench_full_analysis(c: &mut Criterion) {
    let mut group = c.benchmark_group("Full Audio Analysis");

    for duration in [2.0, 5.0] {
        let samples = generate_test_beat(120.0, SAMPLE_RATE, duration);
        let num_samples = samples.len();

        group.throughput(Throughput::Elements(num_samples as u64));
        group.bench_with_input(
            BenchmarkId::new("analyze_audio", format!("{}s", duration)),
            &samples,
            |b, samples| {
                b.iter(|| {
                    black_box(analyze_audio(samples, SAMPLE_RATE, 30.0, 64));
                });
            },
        );
    }

    group.finish();
}

fn bench_synth_generation(c: &mut Criterion) {
    let mut group = c.benchmark_group("Audio Synthesis");

    group.bench_function("sine_1s", |b| {
        b.iter(|| {
            black_box(generate_sine(440.0, SAMPLE_RATE, 1.0, 1.0));
        });
    });

    group.bench_function("white_noise_1s", |b| {
        b.iter(|| {
            black_box(generate_white_noise(SAMPLE_RATE, 1.0, 1.0, 42));
        });
    });

    group.bench_function("test_beat_2s", |b| {
        b.iter(|| {
            black_box(generate_test_beat(120.0, SAMPLE_RATE, 2.0));
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_fft_analysis,
    bench_spectrum_bands,
    bench_beat_detection,
    bench_full_analysis,
    bench_synth_generation,
);
criterion_main!(benches);
