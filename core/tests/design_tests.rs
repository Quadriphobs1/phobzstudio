//! Integration tests for the visualization design system.

use phobz_visualizer::designs::{
    create_design, default_params, BarsDesign, BarsParams, CircularRadialDesign,
    CircularRadialParams, CircularRingDesign, CircularRingParams, Design, DesignConfig,
    DesignParams, DesignType, EdgeDistribution, FramePerimeterDesign, FramePerimeterParams,
};
use std::f32::consts::PI;

// ==================== Design Factory Integration Tests ====================

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
        // Verify params match design type
        match (design_type, &params) {
            (DesignType::Bars, DesignParams::Bars(_)) => {}
            (DesignType::CircularRadial, DesignParams::CircularRadial(_)) => {}
            (DesignType::CircularRing, DesignParams::CircularRing(_)) => {}
            (DesignType::FramePerimeter, DesignParams::FramePerimeter(_)) => {}
            (DesignType::FrameCorners, DesignParams::FrameCorners(_)) => {}
            (DesignType::WaveformLine, DesignParams::WaveformLine(_)) => {}
            (DesignType::SpectrumMountain, DesignParams::SpectrumMountain(_)) => {}
            (DesignType::Particles, DesignParams::Particles(_)) => {}
            _ => panic!("Params don't match design type"),
        }
    }
}

#[test]
fn test_all_designs_generate_vertices_for_same_spectrum() {
    let config = DesignConfig {
        width: 640,
        height: 480,
        color: [0.0, 1.0, 0.5],
        background: [0.0, 0.0, 0.0],
        bar_count: 32,
        glow: true,
        beat_intensity: 0.5,
    };
    let spectrum: Vec<f32> = (0..32).map(|i| i as f32 / 32.0).collect();

    for design_type in DesignType::all() {
        let design = create_design(*design_type);
        let params = default_params(*design_type);
        let vertices = design.generate_vertices(&spectrum, &config, &params);

        assert!(
            !vertices.is_empty(),
            "Design {:?} generated no vertices",
            design_type
        );

        // Different designs have different vertex generation patterns:
        // - Bar-based: 6 vertices per bar
        // - Line-based (WaveformLine, SpectrumMountain): 6 vertices per segment (n-1 segments)
        // - Particles: variable based on energy
        // - FrameCorners: 2 quads per bar (horizontal + vertical)
        let min_expected = match design_type {
            DesignType::WaveformLine | DesignType::SpectrumMountain => (spectrum.len() - 1) * 6,
            DesignType::Particles => 6, // At least one particle
            DesignType::FrameCorners => spectrum.len() * 6, // 2 quads per spectrum value, but only bar_count/4 per corner
            _ => spectrum.len() * 6,
        };
        assert!(
            vertices.len() >= min_expected,
            "Design {:?} generated fewer vertices than expected: {} < {}",
            design_type,
            vertices.len(),
            min_expected
        );
    }
}

// ==================== Bars Design Integration Tests ====================

#[test]
fn test_bars_vertical_layout() {
    let design = BarsDesign;
    let config = DesignConfig {
        width: 480,  // Narrower than tall
        height: 640, // Taller than wide
        bar_count: 16,
        ..Default::default()
    };
    let params = DesignParams::Bars(BarsParams {
        vertical: true,
        ..Default::default()
    });
    let spectrum: Vec<f32> = vec![0.5; 16];

    let vertices = design.generate_vertices(&spectrum, &config, &params);
    assert_eq!(vertices.len(), 16 * 6);

    // For vertical layout, bars should be arranged vertically
    // Check that y positions span the height
    let y_positions: Vec<f32> = vertices.iter().map(|v| v.position[1]).collect();
    let y_min = y_positions.iter().cloned().fold(f32::INFINITY, f32::min);
    let y_max = y_positions.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    assert!(
        (y_max - y_min).abs() > 0.5,
        "Vertical bars should span significant Y range"
    );
}

#[test]
fn test_bars_horizontal_layout() {
    let design = BarsDesign;
    let config = DesignConfig {
        width: 640,
        height: 480,
        bar_count: 16,
        ..Default::default()
    };
    let params = DesignParams::Bars(BarsParams {
        vertical: false,
        ..Default::default()
    });
    let spectrum: Vec<f32> = vec![0.5; 16];

    let vertices = design.generate_vertices(&spectrum, &config, &params);

    // For horizontal layout, bars should be arranged horizontally
    let x_positions: Vec<f32> = vertices.iter().map(|v| v.position[0]).collect();
    let x_min = x_positions.iter().cloned().fold(f32::INFINITY, f32::min);
    let x_max = x_positions.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    assert!(
        (x_max - x_min).abs() > 0.5,
        "Horizontal bars should span significant X range"
    );
}

#[test]
fn test_bars_mirror_changes_scaling() {
    let design = BarsDesign;
    let config = DesignConfig {
        width: 640,
        height: 480,
        bar_count: 8,
        glow: false,
        beat_intensity: 0.0,
        ..Default::default()
    };
    let params_normal = DesignParams::Bars(BarsParams {
        mirror: false,
        ..Default::default()
    });
    let params_mirror = DesignParams::Bars(BarsParams {
        mirror: true,
        ..Default::default()
    });
    let spectrum: Vec<f32> = vec![0.5; 8];

    let vertices_normal = design.generate_vertices(&spectrum, &config, &params_normal);
    let vertices_mirror = design.generate_vertices(&spectrum, &config, &params_mirror);

    // Same vertex count, but different positions due to scaling change
    assert_eq!(vertices_normal.len(), vertices_mirror.len());
    // Mirror mode centers bars with smaller scale, so positions differ
    assert_ne!(vertices_normal[0].position, vertices_mirror[0].position);
}

// ==================== Circular Radial Integration Tests ====================

#[test]
fn test_circular_radial_full_circle() {
    let design = CircularRadialDesign;
    let config = DesignConfig {
        width: 640,
        height: 640, // Square for circular designs
        bar_count: 64,
        ..Default::default()
    };
    let params = DesignParams::CircularRadial(CircularRadialParams {
        arc_span: 2.0 * PI,
        ..Default::default()
    });
    let spectrum: Vec<f32> = vec![0.5; 64];

    let vertices = design.generate_vertices(&spectrum, &config, &params);

    // Full circle should have bars distributed around 360 degrees
    // Vertices should be spread in all quadrants
    let mut has_positive_x = false;
    let mut has_negative_x = false;
    let mut has_positive_y = false;
    let mut has_negative_y = false;

    for v in &vertices {
        if v.position[0] > 0.1 {
            has_positive_x = true;
        }
        if v.position[0] < -0.1 {
            has_negative_x = true;
        }
        if v.position[1] > 0.1 {
            has_positive_y = true;
        }
        if v.position[1] < -0.1 {
            has_negative_y = true;
        }
    }

    assert!(has_positive_x, "Full circle should have positive X vertices");
    assert!(has_negative_x, "Full circle should have negative X vertices");
    assert!(has_positive_y, "Full circle should have positive Y vertices");
    assert!(has_negative_y, "Full circle should have negative Y vertices");
}

#[test]
fn test_circular_radial_partial_arc() {
    let design = CircularRadialDesign;
    let config = DesignConfig {
        width: 640,
        height: 640,
        bar_count: 16,
        ..Default::default()
    };
    let params_full = DesignParams::CircularRadial(CircularRadialParams::default());
    let params_half = DesignParams::CircularRadial(CircularRadialParams {
        arc_span: PI, // Half circle
        ..Default::default()
    });
    let spectrum: Vec<f32> = vec![0.5; 16];

    let vertices_full = design.generate_vertices(&spectrum, &config, &params_full);
    let vertices_half = design.generate_vertices(&spectrum, &config, &params_half);

    // Both should generate same vertex count for same bar count
    assert_eq!(vertices_full.len(), vertices_half.len());
    // But positions should differ due to different arc spans
    assert_ne!(vertices_full[0].position, vertices_half[0].position);
}

#[test]
fn test_circular_radial_varying_radii() {
    let design = CircularRadialDesign;
    let config = DesignConfig {
        width: 640,
        height: 640,
        bar_count: 8,
        ..Default::default()
    };

    // Small inner radius, large outer radius
    let params_large = DesignParams::CircularRadial(CircularRadialParams {
        inner_radius: 0.1,
        outer_radius: 0.9,
        ..Default::default()
    });

    // Large inner radius, small outer radius range
    let params_small = DesignParams::CircularRadial(CircularRadialParams {
        inner_radius: 0.4,
        outer_radius: 0.5,
        ..Default::default()
    });

    let spectrum: Vec<f32> = vec![1.0; 8]; // Max height

    let vertices_large = design.generate_vertices(&spectrum, &config, &params_large);
    let vertices_small = design.generate_vertices(&spectrum, &config, &params_small);

    // Large radius range should produce vertices further from center
    let max_dist_large = vertices_large
        .iter()
        .map(|v| (v.position[0].powi(2) + v.position[1].powi(2)).sqrt())
        .fold(0.0f32, f32::max);

    let max_dist_small = vertices_small
        .iter()
        .map(|v| (v.position[0].powi(2) + v.position[1].powi(2)).sqrt())
        .fold(0.0f32, f32::max);

    assert!(
        max_dist_large > max_dist_small,
        "Larger outer radius should produce further vertices"
    );
}

// ==================== Circular Ring Integration Tests ====================

#[test]
fn test_circular_ring_inward_vs_outward() {
    let design = CircularRingDesign;
    let config = DesignConfig {
        width: 640,
        height: 640,
        bar_count: 16,
        glow: false,
        beat_intensity: 0.0,
        ..Default::default()
    };

    let params_outward = DesignParams::CircularRing(CircularRingParams {
        radius: 0.35,
        bar_length: 0.15,
        inward: false,
        rotation: 0.0,
    });

    let params_inward = DesignParams::CircularRing(CircularRingParams {
        radius: 0.35,
        bar_length: 0.15,
        inward: true,
        rotation: 0.0,
    });

    let spectrum: Vec<f32> = vec![1.0; 16];

    let vertices_outward = design.generate_vertices(&spectrum, &config, &params_outward);
    let vertices_inward = design.generate_vertices(&spectrum, &config, &params_inward);

    // Calculate average distance from center
    let avg_dist_outward: f32 = vertices_outward
        .iter()
        .map(|v| (v.position[0].powi(2) + v.position[1].powi(2)).sqrt())
        .sum::<f32>()
        / vertices_outward.len() as f32;

    let avg_dist_inward: f32 = vertices_inward
        .iter()
        .map(|v| (v.position[0].powi(2) + v.position[1].powi(2)).sqrt())
        .sum::<f32>()
        / vertices_inward.len() as f32;

    // Outward bars should extend further from center than inward
    assert!(
        avg_dist_outward > avg_dist_inward,
        "Outward bars should have greater average distance: {} vs {}",
        avg_dist_outward,
        avg_dist_inward
    );
}

#[test]
fn test_circular_ring_rotation() {
    let design = CircularRingDesign;
    let config = DesignConfig {
        width: 640,
        height: 640,
        bar_count: 4,
        ..Default::default()
    };

    let params_no_rot = DesignParams::CircularRing(CircularRingParams {
        rotation: 0.0,
        ..Default::default()
    });

    let params_quarter_rot = DesignParams::CircularRing(CircularRingParams {
        rotation: PI / 2.0,
        ..Default::default()
    });

    let spectrum: Vec<f32> = vec![0.5; 4];

    let vertices_no_rot = design.generate_vertices(&spectrum, &config, &params_no_rot);
    let vertices_quarter_rot = design.generate_vertices(&spectrum, &config, &params_quarter_rot);

    // First bar position should differ by rotation
    let first_bar_no_rot = &vertices_no_rot[0];
    let first_bar_quarter_rot = &vertices_quarter_rot[0];

    assert!(
        (first_bar_no_rot.position[0] - first_bar_quarter_rot.position[0]).abs() > 0.1
            || (first_bar_no_rot.position[1] - first_bar_quarter_rot.position[1]).abs() > 0.1,
        "Rotation should change vertex positions"
    );
}

// ==================== Frame Perimeter Integration Tests ====================

#[test]
fn test_frame_perimeter_distributes_bars_across_all_edges() {
    let design = FramePerimeterDesign;
    let config = DesignConfig {
        width: 640,
        height: 480,
        bar_count: 32,
        ..Default::default()
    };
    let params = DesignParams::FramePerimeter(FramePerimeterParams {
        distribution: EdgeDistribution::All,
        ..Default::default()
    });
    let spectrum: Vec<f32> = vec![0.5; 32];

    let vertices = design.generate_vertices(&spectrum, &config, &params);

    // Should have vertices spread across all quadrants (edges)
    let mut has_left = false;
    let mut has_right = false;
    let mut has_top = false;
    let mut has_bottom = false;

    for v in &vertices {
        if v.position[0] < -0.5 {
            has_left = true;
        }
        if v.position[0] > 0.5 {
            has_right = true;
        }
        if v.position[1] > 0.5 {
            has_top = true;
        }
        if v.position[1] < -0.5 {
            has_bottom = true;
        }
    }

    assert!(has_left, "All distribution should have bars on left edge");
    assert!(has_right, "All distribution should have bars on right edge");
    assert!(has_top, "All distribution should have bars on top edge");
    assert!(has_bottom, "All distribution should have bars on bottom edge");
}

#[test]
fn test_frame_perimeter_top_bottom_distribution() {
    let design = FramePerimeterDesign;
    let config = DesignConfig {
        width: 640,
        height: 480,
        bar_count: 16,
        glow: false, // Disable glow to get exact positions
        ..Default::default()
    };
    let params = DesignParams::FramePerimeter(FramePerimeterParams {
        distribution: EdgeDistribution::TopBottom,
        ..Default::default()
    });
    let spectrum: Vec<f32> = vec![0.5; 16];

    let vertices = design.generate_vertices(&spectrum, &config, &params);

    // TopBottom bars should be at top/bottom edges (high/low Y values)
    // Verify bars are near top or bottom of screen
    let mut has_top = false;
    let mut has_bottom = false;
    for v in &vertices {
        if v.position[1] > 0.5 {
            has_top = true;
        }
        if v.position[1] < -0.5 {
            has_bottom = true;
        }
    }
    assert!(has_top, "TopBottom should have bars near top edge");
    assert!(has_bottom, "TopBottom should have bars near bottom edge");
}

#[test]
fn test_frame_perimeter_inward_vs_outward() {
    let design = FramePerimeterDesign;
    let config = DesignConfig {
        width: 640,
        height: 480,
        bar_count: 8,
        glow: false,
        ..Default::default()
    };

    let params_inward = DesignParams::FramePerimeter(FramePerimeterParams {
        distribution: EdgeDistribution::TopOnly,
        inward: true,
        ..Default::default()
    });

    let params_outward = DesignParams::FramePerimeter(FramePerimeterParams {
        distribution: EdgeDistribution::TopOnly,
        inward: false,
        ..Default::default()
    });

    let spectrum: Vec<f32> = vec![1.0; 8];

    let vertices_inward = design.generate_vertices(&spectrum, &config, &params_inward);
    let vertices_outward = design.generate_vertices(&spectrum, &config, &params_outward);

    // Inward bars should extend toward center (lower Y values in NDC)
    // Outward bars should extend away from center (toward edge)
    let avg_y_inward: f32 = vertices_inward.iter().map(|v| v.position[1]).sum::<f32>()
        / vertices_inward.len() as f32;
    let avg_y_outward: f32 = vertices_outward.iter().map(|v| v.position[1]).sum::<f32>()
        / vertices_outward.len() as f32;

    // Inward from top edge means bars extend downward (lower Y in NDC = toward center)
    assert!(
        avg_y_inward < avg_y_outward,
        "Inward bars should have lower average Y (toward center): {} vs {}",
        avg_y_inward,
        avg_y_outward
    );
}

// ==================== Cross-Design Comparison Tests ====================

#[test]
fn test_all_designs_produce_valid_local_positions() {
    let config = DesignConfig {
        width: 640,
        height: 480,
        bar_count: 16,
        glow: true,
        ..Default::default()
    };
    let spectrum: Vec<f32> = vec![0.5; 16];

    for design_type in DesignType::all() {
        let design = create_design(*design_type);
        let params = default_params(*design_type);
        let vertices = design.generate_vertices(&spectrum, &config, &params);

        for (i, v) in vertices.iter().enumerate() {
            // Local positions should be in reasonable range for glow expansion
            assert!(
                v.local_pos[0].abs() <= 2.0,
                "Design {:?} vertex {} has invalid local_pos[0]: {}",
                design_type,
                i,
                v.local_pos[0]
            );
            assert!(
                v.local_pos[1].abs() <= 2.0,
                "Design {:?} vertex {} has invalid local_pos[1]: {}",
                design_type,
                i,
                v.local_pos[1]
            );
        }
    }
}

#[test]
fn test_all_designs_handle_zero_spectrum() {
    let config = DesignConfig::default();
    let spectrum: Vec<f32> = vec![0.0; 32];

    for design_type in DesignType::all() {
        let design = create_design(*design_type);
        let params = default_params(*design_type);
        let vertices = design.generate_vertices(&spectrum, &config, &params);

        // Particles design may skip low-energy particles, so it can return empty
        if *design_type == DesignType::Particles {
            // Particles can legitimately be empty with zero spectrum
            continue;
        }

        // Should still generate vertices even with zero values
        assert!(
            !vertices.is_empty(),
            "Design {:?} should handle zero spectrum",
            design_type
        );

        // All bar heights should be 0
        for v in &vertices {
            assert_eq!(
                v.bar_height, 0.0,
                "Zero spectrum should produce zero bar heights"
            );
        }
    }
}

#[test]
fn test_all_designs_handle_max_spectrum() {
    let config = DesignConfig::default();
    let spectrum: Vec<f32> = vec![1.0; 32];

    for design_type in DesignType::all() {
        let design = create_design(*design_type);
        let params = default_params(*design_type);
        let vertices = design.generate_vertices(&spectrum, &config, &params);

        assert!(!vertices.is_empty());

        // All bar heights should be 1.0
        for v in &vertices {
            assert_eq!(v.bar_height, 1.0, "Max spectrum should produce max bar heights");
        }
    }
}

// ==================== Performance Characteristic Tests ====================

#[test]
fn test_high_bar_count_performance() {
    let config = DesignConfig {
        bar_count: 512,
        ..Default::default()
    };
    let spectrum: Vec<f32> = vec![0.5; 512];

    for design_type in DesignType::all() {
        let design = create_design(*design_type);
        let params = default_params(*design_type);
        let vertices = design.generate_vertices(&spectrum, &config, &params);

        // Different designs have different vertex patterns
        let min_expected = match design_type {
            DesignType::WaveformLine | DesignType::SpectrumMountain => (512 - 1) * 6,
            DesignType::Particles => 6, // At least some particles
            _ => 512 * 6,
        };

        // Should handle high bar counts
        assert!(
            vertices.len() >= min_expected,
            "Design {:?} should handle 512 bars, got {} vertices (expected >= {})",
            design_type,
            vertices.len(),
            min_expected
        );
    }
}

#[test]
fn test_vertex_memory_layout() {
    use std::mem::size_of;
    use phobz_visualizer::Vertex;

    // Vertex should be 24 bytes for GPU alignment
    // position: [f32; 2] = 8 bytes
    // local_pos: [f32; 2] = 8 bytes
    // bar_height: f32 = 4 bytes
    // bar_index: f32 = 4 bytes
    assert_eq!(size_of::<Vertex>(), 24, "Vertex should be 24 bytes");
}
