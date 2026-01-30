//! Shared test fixtures for design tests.

use phobz_visualizer::designs::{create_design, default_params, DesignConfig, DesignType};

/// Create a standard test configuration.
pub fn test_config() -> DesignConfig {
    DesignConfig {
        width: 640,
        height: 480,
        color: [0.0, 1.0, 0.5],
        background: [0.0, 0.0, 0.0],
        bar_count: 32,
        glow: true,
        beat_intensity: 0.0,
    }
}

/// Create a square test configuration (for circular designs).
pub fn square_config() -> DesignConfig {
    DesignConfig {
        width: 640,
        height: 640,
        bar_count: 64,
        ..test_config()
    }
}

/// Create a small test configuration.
pub fn small_config() -> DesignConfig {
    DesignConfig {
        width: 640,
        height: 480,
        bar_count: 16,
        glow: false,
        ..test_config()
    }
}

/// Generate a uniform spectrum with the given value and size.
pub fn uniform_spectrum(size: usize, value: f32) -> Vec<f32> {
    vec![value; size]
}

/// Generate a gradient spectrum (0.0 to ~1.0).
pub fn gradient_spectrum(size: usize) -> Vec<f32> {
    (0..size).map(|i| i as f32 / size as f32).collect()
}

/// Verify that a design generates vertices correctly.
pub fn verify_design_generates_vertices(design_type: DesignType, spectrum_size: usize) {
    let design = create_design(design_type);
    let params = default_params(design_type);
    let config = test_config();
    let spectrum = uniform_spectrum(spectrum_size, 0.5);

    let vertices = design.generate_vertices(&spectrum, &config, &params);
    assert!(
        !vertices.is_empty(),
        "Design {:?} generated no vertices",
        design_type
    );
}

/// Check if vertices cover all quadrants (for circular designs).
pub fn vertices_cover_all_quadrants(
    vertices: &[phobz_visualizer::Vertex],
) -> (bool, bool, bool, bool) {
    let mut has_pos_x = false;
    let mut has_neg_x = false;
    let mut has_pos_y = false;
    let mut has_neg_y = false;

    for v in vertices {
        if v.position[0] > 0.1 {
            has_pos_x = true;
        }
        if v.position[0] < -0.1 {
            has_neg_x = true;
        }
        if v.position[1] > 0.1 {
            has_pos_y = true;
        }
        if v.position[1] < -0.1 {
            has_neg_y = true;
        }
    }

    (has_pos_x, has_neg_x, has_pos_y, has_neg_y)
}

/// Calculate average distance from center for vertices.
pub fn average_distance_from_center(vertices: &[phobz_visualizer::Vertex]) -> f32 {
    vertices
        .iter()
        .map(|v| (v.position[0].powi(2) + v.position[1].powi(2)).sqrt())
        .sum::<f32>()
        / vertices.len() as f32
}

/// Calculate max distance from center for vertices.
pub fn max_distance_from_center(vertices: &[phobz_visualizer::Vertex]) -> f32 {
    vertices
        .iter()
        .map(|v| (v.position[0].powi(2) + v.position[1].powi(2)).sqrt())
        .fold(0.0f32, f32::max)
}

/// Get the range of X positions.
pub fn x_position_range(vertices: &[phobz_visualizer::Vertex]) -> (f32, f32) {
    let x_positions: Vec<f32> = vertices.iter().map(|v| v.position[0]).collect();
    (
        x_positions.iter().cloned().fold(f32::INFINITY, f32::min),
        x_positions
            .iter()
            .cloned()
            .fold(f32::NEG_INFINITY, f32::max),
    )
}

/// Get the range of Y positions.
pub fn y_position_range(vertices: &[phobz_visualizer::Vertex]) -> (f32, f32) {
    let y_positions: Vec<f32> = vertices.iter().map(|v| v.position[1]).collect();
    (
        y_positions.iter().cloned().fold(f32::INFINITY, f32::min),
        y_positions
            .iter()
            .cloned()
            .fold(f32::NEG_INFINITY, f32::max),
    )
}
