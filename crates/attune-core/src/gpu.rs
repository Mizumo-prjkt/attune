//! GPU-Accelerated Audio Profiling & Spectral Compute Pipeline (Vulkan / Direct3D / Metal via wgpu).
//! Executes parallel windowed spectral extraction, 12-bin CQT chromagrams, 4-band energy
//! calculations, and aggression profiling across audio cue zones on GPU compute shaders.

use std::sync::{Arc, Mutex, OnceLock};

const WGSL_COMPUTE_SHADER: &str = r#"
struct Uniforms {
    sample_rate: f32,
    hop_samples: u32,
    fft_len: u32,
    num_frames: u32,
    total_samples: u32,
    pad0: u32,
    pad1: u32,
    pad2: u32,
};

struct GpuFrameMetricsRaw {
    chroma_0_3: vec4<f32>,
    chroma_4_7: vec4<f32>,
    chroma_8_11: vec4<f32>,
    energy_power: vec4<f32>,
    energy_db: vec4<f32>,
    metrics_scalars: vec4<f32>,
    voicing_pad: vec4<f32>,
};

@group(0) @binding(0) var<uniform> u: Uniforms;
@group(0) @binding(1) var<storage, read> samples: array<f32>;
@group(0) @binding(2) var<storage, read_write> outputs: array<GpuFrameMetricsRaw>;

const PI: f32 = 3.141592653589793;

@compute @workgroup_size(64, 1, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let frame_idx = global_id.x;
    if (frame_idx >= u.num_frames) {
        return;
    }

    let start_sample = frame_idx * u.hop_samples;
    let fft_len = u.fft_len;

    var chroma = array<f32, 12>(0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
    var e_power = array<f32, 4>(0.0, 0.0, 0.0, 0.0);

    let bin_hz = u.sample_rate / f32(fft_len);
    let num_bins = fft_len / 2u;

    var max_mag: f32 = 0.0;
    var max_bin: u32 = 0u;
    var total_energy: f32 = 0.0;

    // Analyze spectral bins using sliding windowed discrete projection
    for (var k: u32 = 1u; k < num_bins; k = k + 1u) {
        let freq = f32(k) * bin_hz;
        if (freq < 20.0 || freq > 20000.0) {
            continue;
        }

        // Discrete projection
        var re: f32 = 0.0;
        var im: f32 = 0.0;
        let step = max(1u, fft_len / 256u);
        let omega = 2.0 * PI * f32(k) / f32(fft_len);

        for (var n: u32 = 0u; n < fft_len; n = n + step) {
            let s_idx = start_sample + n;
            var s: f32 = 0.0;
            if (s_idx < u.total_samples) {
                let win = 0.5 * (1.0 - cos(2.0 * PI * f32(n) / f32(fft_len - 1u)));
                s = samples[s_idx] * win;
            }
            let angle = omega * f32(n);
            re = re + s * cos(angle);
            im = im - s * sin(angle);
        }

        let mag = sqrt(re * re + im * im) * f32(step);
        let pwr = mag * mag;
        total_energy = total_energy + pwr;

        if (mag > max_mag) {
            max_mag = mag;
            max_bin = k;
        }

        // 4-Band Energy Accumulation
        if (freq >= 20.0 && freq < 150.0) {
            e_power[0] = e_power[0] + pwr;
        } else if (freq >= 150.0 && freq < 600.0) {
            e_power[1] = e_power[1] + pwr;
        } else if (freq >= 600.0 && freq < 4000.0) {
            e_power[2] = e_power[2] + pwr;
        } else if (freq >= 4000.0 && freq <= 20000.0) {
            e_power[3] = e_power[3] + pwr;
        }

        // 12-Bin Chroma Accumulation (frequencies >= 32.7 Hz / C1)
        if (freq >= 32.7) {
            let midi = 69.0 + 12.0 * log2(freq / 440.0);
            let note = (u32(round(midi)) + 120u) % 12u;
            chroma[note] = chroma[note] + mag;
        }
    }

    // Normalize chroma
    var max_c: f32 = 0.0;
    for (var c: u32 = 0u; c < 12u; c = c + 1u) {
        if (chroma[c] > max_c) {
            max_c = chroma[c];
        }
    }
    if (max_c > 1e-6) {
        for (var c: u32 = 0u; c < 12u; c = c + 1u) {
            chroma[c] = chroma[c] / max_c;
        }
    }

    // 4-band dB conversion
    var e_db = vec4<f32>(-90.0, -90.0, -90.0, -90.0);
    e_db.x = 10.0 * log2(max(1e-9, e_power[0])) * 0.30102999566;
    e_db.y = 10.0 * log2(max(1e-9, e_power[1])) * 0.30102999566;
    e_db.z = 10.0 * log2(max(1e-9, e_power[2])) * 0.30102999566;
    e_db.w = 10.0 * log2(max(1e-9, e_power[3])) * 0.30102999566;

    let tot_p = e_power[0] + e_power[1] + e_power[2] + e_power[3];
    var mid_high_ratio: f32 = 0.0;
    if (tot_p > 1e-6) {
        mid_high_ratio = clamp((e_power[2] + e_power[3]) / tot_p, 0.0, 1.0);
    }
    let roughness = clamp(max_mag / (sqrt(tot_p + 1e-6) + 1e-4), 0.0, 1.0);
    let aggression = clamp(0.4 * roughness + 0.6 * mid_high_ratio, 0.0, 1.0);
    let dom_pitch = f32(max_bin) * bin_hz;
    let voicing = clamp(max_mag / (sqrt(total_energy + 1e-6) + 1e-4), 0.0, 1.0);

    outputs[frame_idx].chroma_0_3 = vec4<f32>(chroma[0], chroma[1], chroma[2], chroma[3]);
    outputs[frame_idx].chroma_4_7 = vec4<f32>(chroma[4], chroma[5], chroma[6], chroma[7]);
    outputs[frame_idx].chroma_8_11 = vec4<f32>(chroma[8], chroma[9], chroma[10], chroma[11]);
    outputs[frame_idx].energy_power = vec4<f32>(e_power[0], e_power[1], e_power[2], e_power[3]);
    outputs[frame_idx].energy_db = e_db;
    outputs[frame_idx].metrics_scalars = vec4<f32>(total_energy, roughness, aggression, dom_pitch);
    outputs[frame_idx].voicing_pad = vec4<f32>(voicing, 0.0, 0.0, 0.0);
}
"#;

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GpuUniforms {
    pub sample_rate: f32,
    pub hop_samples: u32,
    pub fft_len: u32,
    pub num_frames: u32,
    pub total_samples: u32,
    pub pad0: u32,
    pub pad1: u32,
    pub pad2: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GpuFrameMetricsRaw {
    pub chroma_0_3: [f32; 4],
    pub chroma_4_7: [f32; 4],
    pub chroma_8_11: [f32; 4],
    pub energy_power: [f32; 4],
    pub energy_db: [f32; 4],
    pub metrics_scalars: [f32; 4], // [spectral_flux, roughness, aggression, dominant_pitch_hz]
    pub voicing_pad: [f32; 4],     // [voicing, 0.0, 0.0, 0.0]
}

#[derive(Debug, Clone)]
pub struct GpuAnalyzedFrame {
    pub chroma_12: [f32; 12],
    pub energy_power: [f32; 4],
    pub energy_4band_db: [f32; 4],
    pub spectral_flux: f32,
    pub roughness: f32,
    pub aggression: f32,
    pub dominant_pitch_hz: f32,
    pub voicing: f32,
}

pub struct GpuSpectralProfiler {
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipeline: wgpu::ComputePipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    pub adapter_name: String,
    pub backend_name: String,
}

static GPU_PROFILER: OnceLock<Option<Arc<GpuSpectralProfiler>>> = OnceLock::new();
static GPU_MUTEX: Mutex<()> = Mutex::new(());

impl GpuSpectralProfiler {
    /// Attempts to initialize the GPU compute device. Returns None gracefully if no GPU is available.
    pub fn new() -> Option<Self> {
        let instance = wgpu::Instance::default();
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
        }))?;

        let adapter_info = adapter.get_info();
        let adapter_name = adapter_info.name;
        let backend_name = format!("{:?}", adapter_info.backend);

        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("Attune GPU Spectral Profiler"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_defaults(),
                memory_hints: wgpu::MemoryHints::Performance,
            },
            None,
        ))
        .ok()?;

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Attune Spectral WGSL Shader"),
            source: wgpu::ShaderSource::Wgsl(WGSL_COMPUTE_SHADER.into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Attune Spectral Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Attune Spectral Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Attune Spectral Compute Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });

        log::info!(
            "Initialized Attune GPU Spectral Compute Pipeline on {} ({})",
            adapter_name,
            backend_name
        );

        Some(Self {
            device,
            queue,
            pipeline,
            bind_group_layout,
            adapter_name,
            backend_name,
        })
    }

    /// Executes the compute shader across all frames of an audio cue zone in parallel.
    pub fn profile_cue_zone(
        &self,
        samples: &[f32],
        sr: u32,
        hop_size: usize,
        fft_len: usize,
    ) -> Option<Vec<GpuAnalyzedFrame>> {
        if samples.len() < fft_len {
            return None;
        }

        let num_frames = (samples.len() - fft_len) / hop_size + 1;
        if num_frames == 0 {
            return None;
        }

        use wgpu::util::DeviceExt;

        let uniforms = GpuUniforms {
            sample_rate: sr as f32,
            hop_samples: hop_size as u32,
            fft_len: fft_len as u32,
            num_frames: num_frames as u32,
            total_samples: samples.len() as u32,
            pad0: 0,
            pad1: 0,
            pad2: 0,
        };

        let uniform_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Uniform Buffer"),
                contents: bytemuck::bytes_of(&uniforms),
                usage: wgpu::BufferUsages::UNIFORM,
            });

        let samples_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Samples Storage Buffer"),
                contents: bytemuck::cast_slice(samples),
                usage: wgpu::BufferUsages::STORAGE,
            });

        let output_size = (num_frames * std::mem::size_of::<GpuFrameMetricsRaw>()) as u64;
        let output_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Output Storage Buffer"),
            size: output_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let staging_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Staging Buffer"),
            size: output_size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Spectral Bind Group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: samples_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: output_buffer.as_entire_binding(),
                },
            ],
        });

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Spectral Compute Encoder"),
            });

        {
            let mut compute_pass =
                encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("Spectral Compute Pass"),
                    timestamp_writes: None,
                });
            compute_pass.set_pipeline(&self.pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);
            let workgroup_count = ((num_frames as u32) + 63) / 64;
            compute_pass.dispatch_workgroups(workgroup_count, 1, 1);
        }

        encoder.copy_buffer_to_buffer(&output_buffer, 0, &staging_buffer, 0, output_size);
        self.queue.submit(Some(encoder.finish()));

        // Map staging buffer to CPU
        let buffer_slice = staging_buffer.slice(..);
        let (sender, receiver) = std::sync::mpsc::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |res| {
            let _ = sender.send(res);
        });

        self.device.poll(wgpu::Maintain::Wait);

        if receiver.recv().ok()?.is_err() {
            return None;
        }

        let data = buffer_slice.get_mapped_range();
        let raw_outputs: &[GpuFrameMetricsRaw] = bytemuck::cast_slice(&data);

        let mut frames = Vec::with_capacity(num_frames);
        for raw in raw_outputs {
            let mut chroma = [0.0f32; 12];
            chroma[0..4].copy_from_slice(&raw.chroma_0_3);
            chroma[4..8].copy_from_slice(&raw.chroma_4_7);
            chroma[8..12].copy_from_slice(&raw.chroma_8_11);

            frames.push(GpuAnalyzedFrame {
                chroma_12: chroma,
                energy_power: raw.energy_power,
                energy_4band_db: raw.energy_db,
                spectral_flux: raw.metrics_scalars[0],
                roughness: raw.metrics_scalars[1],
                aggression: raw.metrics_scalars[2],
                dominant_pitch_hz: raw.metrics_scalars[3],
                voicing: raw.voicing_pad[0],
            });
        }

        drop(data);
        staging_buffer.unmap();

        Some(frames)
    }
}

/// Returns the singleton GPU spectral profiler instance if hardware GPU compute is available.
pub fn get_or_init_gpu_profiler() -> Option<Arc<GpuSpectralProfiler>> {
    let _guard = GPU_MUTEX.lock().ok()?;
    GPU_PROFILER
        .get_or_init(|| GpuSpectralProfiler::new().map(Arc::new))
        .clone()
}

/// Helper to execute GPU spectral profiling, returning None if GPU is disabled or unavailable.
pub fn try_gpu_spectral_profile(
    samples: &[f32],
    sr: u32,
    hop_size: usize,
    fft_len: usize,
) -> Option<Vec<GpuAnalyzedFrame>> {
    let profiler = get_or_init_gpu_profiler()?;
    profiler.profile_cue_zone(samples, sr, hop_size, fft_len)
}

/// Checks if GPU acceleration is supported and initialized on this host.
pub fn is_gpu_acceleration_available() -> bool {
    get_or_init_gpu_profiler().is_some()
}

/// Returns the GPU adapter and backend name if hardware GPU compute is available (e.g. ("Intel(R) HD Graphics 630", "Vulkan")).
pub fn get_gpu_device_info() -> Option<(String, String)> {
    let profiler = get_or_init_gpu_profiler()?;
    Some((profiler.adapter_name.clone(), profiler.backend_name.clone()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gpu_raw_struct_memory_alignment() {
        assert_eq!(std::mem::size_of::<GpuUniforms>(), 32);
        assert_eq!(std::mem::size_of::<GpuFrameMetricsRaw>(), 112);
        assert_eq!(std::mem::align_of::<GpuFrameMetricsRaw>(), 4);
    }

    #[test]
    fn test_gpu_spectral_profile_runs_or_falls_back_gracefully() {
        let dummy_samples = vec![0.1f32; 8192];
        let sr = 44100;
        let hop_size = 2205;
        let fft_len = 2048;

        // Must never panic or crash even if running in a headless container without GPU
        let res = try_gpu_spectral_profile(&dummy_samples, sr, hop_size, fft_len);
        if let Some(frames) = res {
            assert!(!frames.is_empty());
            assert_eq!(frames[0].chroma_12.len(), 12);
        }
    }

    #[test]
    fn test_gpu_device_info_query() {
        if let Some((adapter, backend)) = get_gpu_device_info() {
            println!("Detected GPU: {adapter} via {backend}");
            assert!(!adapter.is_empty());
            assert!(!backend.is_empty());
        }
    }
}
