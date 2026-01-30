// GPU FFT Compute Shader
// Implements radix-2 Cooley-Tukey DIT FFT with bit-reversal.

// Constants
const PI: f32 = 3.14159265358979323846;
const WORKGROUP_SIZE: u32 = 256u;

// FFT parameters passed from CPU
struct FftParams {
    n: u32,              // FFT size (must be power of 2)
    stage: u32,          // Current butterfly stage (0 to log2(n)-1)
    direction: i32,      // 1 = forward FFT, -1 = inverse FFT
    log2_n: u32,         // log2(n) for bit reversal
}

@group(0) @binding(0) var<storage, read> input_data: array<vec2<f32>>;
@group(0) @binding(1) var<storage, read_write> output_data: array<vec2<f32>>;
@group(0) @binding(2) var<uniform> params: FftParams;

// Bit reversal function
fn bit_reverse(x: u32, bits: u32) -> u32 {
    var result: u32 = 0u;
    var val = x;
    for (var i: u32 = 0u; i < bits; i = i + 1u) {
        result = (result << 1u) | (val & 1u);
        val = val >> 1u;
    }
    return result;
}

// Complex multiplication: (a + bi) * (c + di) = (ac - bd) + (ad + bc)i
fn complex_mul(a: vec2<f32>, b: vec2<f32>) -> vec2<f32> {
    return vec2<f32>(
        a.x * b.x - a.y * b.y,
        a.x * b.y + a.y * b.x
    );
}

// Bit-reversal permutation shader
// Run this ONCE before FFT stages
@compute @workgroup_size(WORKGROUP_SIZE)
fn bit_reverse_permute(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    let n = params.n;

    if idx >= n {
        return;
    }

    let rev_idx = bit_reverse(idx, params.log2_n);

    // Only swap if idx < rev_idx to avoid double-swapping
    // We read from input and write to output (they can be same buffer with proper sync)
    output_data[idx] = input_data[rev_idx];
}

// Cooley-Tukey butterfly operation for a single stage
// Each invocation handles one butterfly pair
@compute @workgroup_size(WORKGROUP_SIZE)
fn fft_butterfly(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let thread_idx = global_id.x;
    let n = params.n;
    let stage = params.stage;

    // Number of butterflies per group at this stage
    let butterfly_size = 1u << (stage + 1u);  // 2^(stage+1)
    let half_size = 1u << stage;              // 2^stage

    // Total number of butterflies is n/2
    let num_butterflies = n >> 1u;

    if thread_idx >= num_butterflies {
        return;
    }

    // Determine which butterfly group and position within group
    let group = thread_idx / half_size;
    let pos = thread_idx % half_size;

    // Calculate indices for the butterfly pair
    let idx_top = group * butterfly_size + pos;
    let idx_bot = idx_top + half_size;

    // Calculate twiddle factor: W_N^k = e^(-2πik/N) for forward FFT
    // k = pos * (N / butterfly_size)
    let k = pos * (n / butterfly_size);
    let angle = -2.0 * PI * f32(k) / f32(n) * f32(params.direction);
    let twiddle = vec2<f32>(cos(angle), sin(angle));

    // Load values
    let top = input_data[idx_top];
    let bot = input_data[idx_bot];

    // Apply twiddle factor to bottom
    let twiddle_bot = complex_mul(twiddle, bot);

    // Butterfly: out_top = top + W*bot, out_bot = top - W*bot
    output_data[idx_top] = top + twiddle_bot;
    output_data[idx_bot] = top - twiddle_bot;
}

// Compute magnitude spectrum from complex FFT output
struct MagnitudeParams {
    n: u32,              // Number of complex values
    scale: f32,          // Scaling factor (typically 1/sqrt(n))
    db_mode: u32,        // 0 = linear, 1 = decibels
    _padding: u32,
}

@group(0) @binding(0) var<storage, read> complex_input: array<vec2<f32>>;
@group(0) @binding(1) var<storage, read_write> magnitude_output: array<f32>;
@group(0) @binding(2) var<uniform> mag_params: MagnitudeParams;

@compute @workgroup_size(WORKGROUP_SIZE)
fn compute_magnitude(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;

    // Only compute positive frequencies (first half)
    if idx >= mag_params.n / 2u {
        return;
    }

    let c = complex_input[idx];
    var mag = sqrt(c.x * c.x + c.y * c.y) * mag_params.scale;

    // Convert to dB if requested
    if mag_params.db_mode > 0u {
        mag = 20.0 * log(max(mag, 1e-10)) / log(10.0);
        mag = max(mag, -80.0);  // Floor at -80 dB
    }

    magnitude_output[idx] = mag;
}

// Apply Hann window to input samples
struct WindowParams {
    n: u32,
    _padding: vec3<u32>,
}

@group(0) @binding(0) var<storage, read> raw_samples: array<f32>;
@group(0) @binding(1) var<storage, read_write> windowed_output: array<vec2<f32>>;
@group(0) @binding(2) var<uniform> window_params: WindowParams;

@compute @workgroup_size(WORKGROUP_SIZE)
fn apply_window(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    let n = window_params.n;

    if idx >= n {
        return;
    }

    // Hann window: 0.5 * (1 - cos(2π * i / (N-1)))
    let t = f32(idx) / f32(n - 1u);
    let window = 0.5 * (1.0 - cos(2.0 * PI * t));

    // Apply window and convert to complex (imaginary part = 0)
    windowed_output[idx] = vec2<f32>(raw_samples[idx] * window, 0.0);
}

// Group frequency bins into logarithmically-spaced bands
struct BandParams {
    num_bins: u32,       // Number of FFT bins (fft_size / 2)
    num_bands: u32,      // Number of output bands
    sample_rate: u32,    // Sample rate in Hz
    min_freq: f32,       // Minimum frequency (typically 20 Hz)
}

@group(0) @binding(0) var<storage, read> spectrum_input: array<f32>;
@group(0) @binding(1) var<storage, read_write> bands_output: array<f32>;
@group(0) @binding(2) var<uniform> band_params: BandParams;

@compute @workgroup_size(64)
fn compute_bands(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let band_idx = global_id.x;

    if band_idx >= band_params.num_bands {
        return;
    }

    let num_bins = band_params.num_bins;
    let num_bands = band_params.num_bands;
    let sample_rate = f32(band_params.sample_rate);
    let max_freq = sample_rate / 2.0;
    let min_freq = band_params.min_freq;

    // Logarithmically spaced band edges
    let log_min = log(min_freq);
    let log_max = log(max_freq);

    let t0 = f32(band_idx) / f32(num_bands);
    let t1 = f32(band_idx + 1u) / f32(num_bands);

    let freq_low = exp(log_min + t0 * (log_max - log_min));
    let freq_high = exp(log_min + t1 * (log_max - log_min));

    // Convert to bin indices
    let fft_size = num_bins * 2u;
    let bin_low = u32(freq_low * f32(fft_size) / sample_rate);
    let bin_high = u32(freq_high * f32(fft_size) / sample_rate);

    let bin_low_clamped = min(bin_low, num_bins - 1u);
    let bin_high_clamped = min(bin_high, num_bins);

    // Average magnitudes in this band
    var sum: f32 = 0.0;
    var count: u32 = 0u;

    for (var i = bin_low_clamped; i < bin_high_clamped; i = i + 1u) {
        sum = sum + spectrum_input[i];
        count = count + 1u;
    }

    if count > 0u {
        bands_output[band_idx] = sum / f32(count);
    } else {
        bands_output[band_idx] = spectrum_input[bin_low_clamped];
    }
}
