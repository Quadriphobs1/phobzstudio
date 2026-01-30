//! Benchmarks for GPU rendering operations.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use phobz_visualizer::gpu::{RenderConfig, WaveformRenderer};

fn bench_render_frame(c: &mut Criterion) {
    let mut group = c.benchmark_group("GPU Rendering");

    let config = RenderConfig {
        width: 1920,
        height: 1080,
        bar_count: 64,
        color: [0.0, 1.0, 0.53],
        background: [0.0, 0.0, 0.0],
        vertical: false,
        mirror: false,
        glow: true,
    };

    let renderer = match pollster::block_on(WaveformRenderer::new(config)) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Skipping GPU benchmarks: {}", e);
            return;
        }
    };

    let bar_heights: Vec<f32> = (0..64).map(|i| i as f32 / 64.0).collect();

    group.bench_function("render_frame_1080p", |b| {
        b.iter(|| {
            black_box(renderer.render_frame(&bar_heights, 0.5));
        });
    });

    group.finish();
}

fn bench_render_resolutions(c: &mut Criterion) {
    let mut group = c.benchmark_group("Resolution Scaling");

    let resolutions = [
        (640, 360, "360p"),
        (1280, 720, "720p"),
        (1920, 1080, "1080p"),
    ];

    for (width, height, name) in resolutions {
        let config = RenderConfig {
            width,
            height,
            bar_count: 64,
            color: [0.0, 1.0, 0.53],
            background: [0.0, 0.0, 0.0],
            vertical: false,
            mirror: false,
            glow: true,
        };

        let renderer = match pollster::block_on(WaveformRenderer::new(config)) {
            Ok(r) => r,
            Err(_) => continue,
        };

        let bar_heights: Vec<f32> = (0..64).map(|i| i as f32 / 64.0).collect();

        group.bench_with_input(
            BenchmarkId::new("render", name),
            &renderer,
            |b, renderer| {
                b.iter(|| {
                    black_box(renderer.render_frame(&bar_heights, 0.5));
                });
            },
        );
    }

    group.finish();
}

fn bench_bar_counts(c: &mut Criterion) {
    let mut group = c.benchmark_group("Bar Count Scaling");

    for bar_count in [16, 32, 64, 128] {
        let config = RenderConfig {
            width: 1920,
            height: 1080,
            bar_count,
            color: [0.0, 1.0, 0.53],
            background: [0.0, 0.0, 0.0],
            vertical: false,
            mirror: false,
            glow: true,
        };

        let renderer = match pollster::block_on(WaveformRenderer::new(config)) {
            Ok(r) => r,
            Err(_) => continue,
        };

        let bar_heights: Vec<f32> = (0..bar_count)
            .map(|i| i as f32 / bar_count as f32)
            .collect();

        group.bench_with_input(
            BenchmarkId::new("render", bar_count),
            &(renderer, bar_heights),
            |b, (renderer, heights)| {
                b.iter(|| {
                    black_box(renderer.render_frame(heights, 0.5));
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_render_frame,
    bench_render_resolutions,
    bench_bar_counts
);
criterion_main!(benches);
