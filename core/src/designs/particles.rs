//! Particles visualization design.
//!
//! Beat-reactive particles that pulse and move based on audio spectrum.

use super::{Design, DesignConfig, DesignParams, DesignType, Vertex};

/// Rendering context for particles calculations.
struct ParticleContext {
    width: f32,
    height: f32,
    beat_scale: f32,
    local_expand: f32,
}

impl ParticleContext {
    fn new(config: &DesignConfig) -> Self {
        let glow_expand = if config.glow { 0.3 } else { 0.0 };
        Self {
            width: config.width as f32,
            height: config.height as f32,
            beat_scale: 1.0 + config.beat_intensity * 0.15,
            local_expand: 1.0 + glow_expand,
        }
    }

    #[inline]
    fn to_ndc(&self, x: f32, y: f32) -> [f32; 2] {
        [(x / self.width) * 2.0 - 1.0, 1.0 - (y / self.height) * 2.0]
    }

    /// Push a particle quad.
    fn push_particle(
        &self,
        vertices: &mut Vec<Vertex>,
        cx: f32,
        cy: f32,
        size: f32,
        value: f32,
        index: f32,
    ) {
        let half_size = size * 0.5;

        let positions = [
            self.to_ndc(cx - half_size, cy - half_size), // top-left
            self.to_ndc(cx + half_size, cy - half_size), // top-right
            self.to_ndc(cx - half_size, cy + half_size), // bottom-left
            self.to_ndc(cx + half_size, cy + half_size), // bottom-right
        ];

        let local = self.local_expand;
        let local_positions = [
            [-local, -local],
            [local, -local],
            [-local, local],
            [local, local],
        ];
        let indices = [0, 2, 1, 1, 2, 3]; // Two triangles

        for &idx in &indices {
            vertices.push(Vertex {
                position: positions[idx],
                local_pos: local_positions[idx],
                bar_height: value,
                bar_index: index,
            });
        }
    }
}

/// Simple pseudo-random number generator for deterministic particle placement.
struct Rng {
    state: u32,
}

impl Rng {
    fn new(seed: u32) -> Self {
        Self { state: seed }
    }

    fn next(&mut self) -> f32 {
        // xorshift32
        self.state ^= self.state << 13;
        self.state ^= self.state >> 17;
        self.state ^= self.state << 5;
        (self.state as f32) / (u32::MAX as f32)
    }

    fn next_range(&mut self, min: f32, max: f32) -> f32 {
        min + self.next() * (max - min)
    }
}

/// Beat-reactive particle visualization.
pub struct ParticlesDesign;

impl Design for ParticlesDesign {
    fn design_type(&self) -> DesignType {
        DesignType::Particles
    }

    fn generate_vertices(
        &self,
        spectrum: &[f32],
        config: &DesignConfig,
        params: &DesignParams,
    ) -> Vec<Vertex> {
        let params = match params {
            DesignParams::Particles(p) => p,
            _ => return Vec::new(),
        };

        let bar_count = spectrum.len().min(config.bar_count as usize);
        if bar_count == 0 {
            return Vec::new();
        }

        let ctx = ParticleContext::new(config);
        let particle_count = params.count as usize;
        let mut vertices = Vec::with_capacity(particle_count * 6);

        // Calculate average energy from spectrum
        let energy: f32 = spectrum
            .iter()
            .take(bar_count)
            .map(|v| v.clamp(0.0, 1.0))
            .sum::<f32>()
            / bar_count as f32;
        let energy_boost = 1.0 + energy * 0.5;

        // Create deterministic seed from spectrum
        let seed = spectrum.iter().take(4).fold(0u32, |acc, v| {
            acc.wrapping_add((v * 1000.0) as u32).wrapping_mul(31)
        });
        let mut rng = Rng::new(seed.max(1));

        // Define spawn area based on pattern
        let (cx, cy, spread_x, spread_y) = match params.pattern {
            ParticlePattern::Random => (
                ctx.width * 0.5,
                ctx.height * 0.5,
                ctx.width * 0.45,
                ctx.height * 0.45,
            ),
            ParticlePattern::Center => (
                ctx.width * 0.5,
                ctx.height * 0.5,
                ctx.width * 0.25,
                ctx.height * 0.25,
            ),
            ParticlePattern::Ring => (
                ctx.width * 0.5,
                ctx.height * 0.5,
                ctx.width * 0.35,
                ctx.height * 0.35,
            ),
            ParticlePattern::Burst => (
                ctx.width * 0.5,
                ctx.height * 0.5,
                ctx.width * 0.4,
                ctx.height * 0.4,
            ),
        };

        for i in 0..particle_count {
            // Get spectrum value for this particle (cycle through spectrum)
            let spectrum_idx = i % bar_count;
            let value = spectrum[spectrum_idx].clamp(0.0, 1.0);

            // Skip particles with very low energy (creates dynamic appearance)
            if value < 0.1 && config.beat_intensity < 0.3 {
                continue;
            }

            // Calculate particle position based on pattern
            let (px, py) = match params.pattern {
                ParticlePattern::Random => {
                    let x = cx + rng.next_range(-spread_x, spread_x);
                    let y = cy + rng.next_range(-spread_y, spread_y);
                    (x, y)
                }
                ParticlePattern::Center => {
                    let angle = rng.next() * std::f32::consts::TAU;
                    let dist = rng.next() * spread_x * value * ctx.beat_scale;
                    (cx + angle.cos() * dist, cy + angle.sin() * dist)
                }
                ParticlePattern::Ring => {
                    let angle = (i as f32 / particle_count as f32) * std::f32::consts::TAU;
                    let base_dist = spread_x * 0.8;
                    let dist = base_dist + rng.next_range(-20.0, 20.0) * value;
                    (
                        cx + angle.cos() * dist * energy_boost,
                        cy + angle.sin() * dist * energy_boost,
                    )
                }
                ParticlePattern::Burst => {
                    let angle = rng.next() * std::f32::consts::TAU;
                    let dist = spread_x * value * ctx.beat_scale * energy_boost;
                    (cx + angle.cos() * dist, cy + angle.sin() * dist)
                }
            };

            // Calculate particle size based on value and beat
            let base_size = rng.next_range(params.size_range.0, params.size_range.1);
            let size = base_size * (0.5 + value * 0.5) * ctx.beat_scale * ctx.local_expand;

            // Push the particle
            ctx.push_particle(&mut vertices, px, py, size, value, i as f32);
        }

        vertices
    }
}

/// Particle distribution pattern.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ParticlePattern {
    /// Particles randomly distributed across screen.
    #[default]
    Random,
    /// Particles emanate from center.
    Center,
    /// Particles arranged in a ring.
    Ring,
    /// Particles burst outward from center (beat reactive).
    Burst,
}
