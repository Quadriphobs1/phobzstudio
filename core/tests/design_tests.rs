//! Integration tests for the visualization design system.

mod design_fixtures;

use design_fixtures::*;
use phobz_visualizer::designs::{
    create_design, default_params, BarsDesign, BarsParams, CircularRadialDesign,
    CircularRadialParams, CircularRingDesign, CircularRingParams, Design, DesignConfig,
    DesignParams, DesignType, EdgeDistribution, FramePerimeterDesign, FramePerimeterParams,
};
use std::f32::consts::PI;

// ==================== Design Factory Tests ====================

#[test]
fn test_all_design_types_can_be_created() {
    for design_type in DesignType::all() {
        let design = create_design(*design_type);
        assert_eq!(design.design_type(), *design_type);
    }
}

#[test]
fn test_all_design_types_have_default_params() {
    for design_type in DesignType::all() {
        let params = default_params(*design_type);
        match (design_type, &params) {
            (DesignType::Bars, DesignParams::Bars(_)) => {}
            (DesignType::CircularRadial, DesignParams::CircularRadial(_)) => {}
            (DesignType::CircularRing, DesignParams::CircularRing(_)) => {}
            (DesignType::FramePerimeter, DesignParams::FramePerimeter(_)) => {}
            (DesignType::FrameCorners, DesignParams::FrameCorners(_)) => {}
            (DesignType::WaveformLine, DesignParams::WaveformLine(_)) => {}
            (DesignType::SpectrumMountain, DesignParams::SpectrumMountain(_)) => {}
            (DesignType::Particles, DesignParams::Particles(_)) => {}
            (DesignType::Spectrogram, DesignParams::Spectrogram(_)) => {}
            _ => panic!("Params don't match design type"),
        }
    }
}

#[test]
fn test_all_designs_generate_vertices() {
    for design_type in DesignType::all() {
        verify_design_generates_vertices(*design_type, 32);
    }
}

// ==================== Bars Design Tests ====================

#[test]
fn test_bars_vertical_layout() {
    let design = BarsDesign;
    let config = DesignConfig { width: 480, height: 640, bar_count: 16, ..test_config() };
    let params = DesignParams::Bars(BarsParams { vertical: true, ..Default::default() });
    let vertices = design.generate_vertices(&uniform_spectrum(16, 0.5), &config, &params);

    assert_eq!(vertices.len(), 16 * 6);
    let (y_min, y_max) = y_position_range(&vertices);
    assert!((y_max - y_min).abs() > 0.5, "Vertical bars should span significant Y range");
}

#[test]
fn test_bars_horizontal_layout() {
    let design = BarsDesign;
    let config = DesignConfig { width: 640, height: 480, bar_count: 16, ..test_config() };
    let params = DesignParams::Bars(BarsParams { vertical: false, ..Default::default() });
    let vertices = design.generate_vertices(&uniform_spectrum(16, 0.5), &config, &params);

    let (x_min, x_max) = x_position_range(&vertices);
    assert!((x_max - x_min).abs() > 0.5, "Horizontal bars should span significant X range");
}

#[test]
fn test_bars_mirror_changes_scaling() {
    let design = BarsDesign;
    let config = small_config();
    let spectrum = uniform_spectrum(8, 0.5);

    let v_normal = design.generate_vertices(&spectrum, &config, &DesignParams::Bars(BarsParams { mirror: false, ..Default::default() }));
    let v_mirror = design.generate_vertices(&spectrum, &config, &DesignParams::Bars(BarsParams { mirror: true, ..Default::default() }));

    assert_eq!(v_normal.len(), v_mirror.len());
    assert_ne!(v_normal[0].position, v_mirror[0].position);
}

// ==================== Circular Radial Tests ====================

#[test]
fn test_circular_radial_full_circle() {
    let design = CircularRadialDesign;
    let config = square_config();
    let params = DesignParams::CircularRadial(CircularRadialParams { arc_span: 2.0 * PI, ..Default::default() });
    let vertices = design.generate_vertices(&uniform_spectrum(64, 0.5), &config, &params);

    let (has_pos_x, has_neg_x, has_pos_y, has_neg_y) = vertices_cover_all_quadrants(&vertices);
    assert!(has_pos_x && has_neg_x && has_pos_y && has_neg_y, "Full circle should cover all quadrants");
}

#[test]
fn test_circular_radial_partial_arc() {
    let design = CircularRadialDesign;
    let config = DesignConfig { bar_count: 16, ..square_config() };
    let spectrum = uniform_spectrum(16, 0.5);

    let v_full = design.generate_vertices(&spectrum, &config, &DesignParams::CircularRadial(CircularRadialParams::default()));
    let v_half = design.generate_vertices(&spectrum, &config, &DesignParams::CircularRadial(CircularRadialParams { arc_span: PI, ..Default::default() }));

    assert_eq!(v_full.len(), v_half.len());
    assert_ne!(v_full[0].position, v_half[0].position);
}

#[test]
fn test_circular_radial_varying_radii() {
    let design = CircularRadialDesign;
    let config = DesignConfig { bar_count: 8, ..square_config() };
    let spectrum = uniform_spectrum(8, 1.0);

    let v_large = design.generate_vertices(&spectrum, &config, &DesignParams::CircularRadial(CircularRadialParams { inner_radius: 0.1, outer_radius: 0.9, ..Default::default() }));
    let v_small = design.generate_vertices(&spectrum, &config, &DesignParams::CircularRadial(CircularRadialParams { inner_radius: 0.4, outer_radius: 0.5, ..Default::default() }));

    assert!(max_distance_from_center(&v_large) > max_distance_from_center(&v_small));
}

// ==================== Circular Ring Tests ====================

#[test]
fn test_circular_ring_inward_vs_outward() {
    let design = CircularRingDesign;
    let config = DesignConfig { bar_count: 16, glow: false, beat_intensity: 0.0, ..square_config() };
    let spectrum = uniform_spectrum(16, 1.0);

    let v_outward = design.generate_vertices(&spectrum, &config, &DesignParams::CircularRing(CircularRingParams { radius: 0.35, bar_length: 0.15, inward: false, rotation: 0.0 }));
    let v_inward = design.generate_vertices(&spectrum, &config, &DesignParams::CircularRing(CircularRingParams { radius: 0.35, bar_length: 0.15, inward: true, rotation: 0.0 }));

    assert!(average_distance_from_center(&v_outward) > average_distance_from_center(&v_inward));
}

#[test]
fn test_circular_ring_rotation() {
    let design = CircularRingDesign;
    let config = DesignConfig { bar_count: 4, ..square_config() };
    let spectrum = uniform_spectrum(4, 0.5);

    let v_no_rot = design.generate_vertices(&spectrum, &config, &DesignParams::CircularRing(CircularRingParams { rotation: 0.0, ..Default::default() }));
    let v_quarter = design.generate_vertices(&spectrum, &config, &DesignParams::CircularRing(CircularRingParams { rotation: PI / 2.0, ..Default::default() }));

    let diff_x = (v_no_rot[0].position[0] - v_quarter[0].position[0]).abs();
    let diff_y = (v_no_rot[0].position[1] - v_quarter[0].position[1]).abs();
    assert!(diff_x > 0.1 || diff_y > 0.1, "Rotation should change vertex positions");
}

// ==================== Frame Perimeter Tests ====================

#[test]
fn test_frame_perimeter_distributes_across_all_edges() {
    let design = FramePerimeterDesign;
    let config = test_config();
    let params = DesignParams::FramePerimeter(FramePerimeterParams { distribution: EdgeDistribution::All, ..Default::default() });
    let vertices = design.generate_vertices(&uniform_spectrum(32, 0.5), &config, &params);

    let (mut left, mut right, mut top, mut bottom) = (false, false, false, false);
    for v in &vertices {
        if v.position[0] < -0.5 { left = true; }
        if v.position[0] > 0.5 { right = true; }
        if v.position[1] > 0.5 { top = true; }
        if v.position[1] < -0.5 { bottom = true; }
    }
    assert!(left && right && top && bottom, "All edges should have bars");
}

#[test]
fn test_frame_perimeter_top_bottom_distribution() {
    let design = FramePerimeterDesign;
    let config = small_config();
    let params = DesignParams::FramePerimeter(FramePerimeterParams { distribution: EdgeDistribution::TopBottom, ..Default::default() });
    let vertices = design.generate_vertices(&uniform_spectrum(16, 0.5), &config, &params);

    let (mut has_top, mut has_bottom) = (false, false);
    for v in &vertices {
        if v.position[1] > 0.5 { has_top = true; }
        if v.position[1] < -0.5 { has_bottom = true; }
    }
    assert!(has_top && has_bottom, "TopBottom should have bars at top and bottom");
}

#[test]
fn test_frame_perimeter_inward_vs_outward() {
    let design = FramePerimeterDesign;
    let config = DesignConfig { bar_count: 8, glow: false, ..test_config() };
    let spectrum = uniform_spectrum(8, 1.0);

    let v_inward = design.generate_vertices(&spectrum, &config, &DesignParams::FramePerimeter(FramePerimeterParams { distribution: EdgeDistribution::TopOnly, inward: true, ..Default::default() }));
    let v_outward = design.generate_vertices(&spectrum, &config, &DesignParams::FramePerimeter(FramePerimeterParams { distribution: EdgeDistribution::TopOnly, inward: false, ..Default::default() }));

    let avg_y_inward: f32 = v_inward.iter().map(|v| v.position[1]).sum::<f32>() / v_inward.len() as f32;
    let avg_y_outward: f32 = v_outward.iter().map(|v| v.position[1]).sum::<f32>() / v_outward.len() as f32;

    assert!(avg_y_inward < avg_y_outward, "Inward bars should have lower average Y");
}

// ==================== Cross-Design Tests ====================

#[test]
fn test_all_designs_produce_valid_local_positions() {
    let config = DesignConfig { bar_count: 16, glow: true, ..test_config() };
    let spectrum = uniform_spectrum(16, 0.5);

    for design_type in DesignType::all() {
        let design = create_design(*design_type);
        let params = default_params(*design_type);
        let vertices = design.generate_vertices(&spectrum, &config, &params);

        for (i, v) in vertices.iter().enumerate() {
            assert!(v.local_pos[0].abs() <= 2.0, "Design {:?} vertex {} invalid local_pos[0]", design_type, i);
            assert!(v.local_pos[1].abs() <= 2.0, "Design {:?} vertex {} invalid local_pos[1]", design_type, i);
        }
    }
}

#[test]
fn test_all_designs_handle_zero_spectrum() {
    let config = DesignConfig::default();
    let spectrum = uniform_spectrum(32, 0.0);

    for design_type in DesignType::all() {
        let design = create_design(*design_type);
        let params = default_params(*design_type);
        let vertices = design.generate_vertices(&spectrum, &config, &params);

        if *design_type == DesignType::Particles { continue; } // Particles can be empty

        assert!(!vertices.is_empty(), "Design {:?} should handle zero spectrum", design_type);
        for v in &vertices {
            assert_eq!(v.bar_height, 0.0, "Zero spectrum should produce zero bar heights");
        }
    }
}

#[test]
fn test_all_designs_handle_max_spectrum() {
    let config = DesignConfig::default();
    let spectrum = uniform_spectrum(32, 1.0);

    for design_type in DesignType::all() {
        let design = create_design(*design_type);
        let params = default_params(*design_type);
        let vertices = design.generate_vertices(&spectrum, &config, &params);

        assert!(!vertices.is_empty());
        for v in &vertices {
            assert_eq!(v.bar_height, 1.0, "Max spectrum should produce max bar heights");
        }
    }
}

// ==================== Performance Tests ====================

#[test]
fn test_high_bar_count_performance() {
    let config = DesignConfig { bar_count: 512, ..DesignConfig::default() };
    let spectrum = uniform_spectrum(512, 0.5);

    for design_type in DesignType::all() {
        let design = create_design(*design_type);
        let params = default_params(*design_type);
        let vertices = design.generate_vertices(&spectrum, &config, &params);

        let min_expected = match design_type {
            DesignType::WaveformLine | DesignType::SpectrumMountain => (512 - 1) * 6,
            DesignType::Particles => 6,
            _ => 512 * 6,
        };

        assert!(vertices.len() >= min_expected, "Design {:?} should handle 512 bars", design_type);
    }
}

#[test]
fn test_vertex_memory_layout() {
    use std::mem::size_of;
    use phobz_visualizer::Vertex;
    assert_eq!(size_of::<Vertex>(), 24, "Vertex should be 24 bytes");
}
