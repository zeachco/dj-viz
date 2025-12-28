//! Intensity effects for emotional music visualization.
//!
//! Applies post-processing effects based on music intensity:
//! - Vignette (darkened edges for focus)
//! - Color warmth (shift toward reds/oranges)
//! - Saturation boost (more vivid colors)
//! - Pulse effect (brightness oscillation)

use nannou::prelude::*;
use nannou::wgpu;

/// Uniform buffer for intensity effect parameters
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct IntensityUniforms {
    intensity: f32,
    momentum: f32,
    bass: f32,
    time: f32,
}

/// Vertex for fullscreen quad
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct IntensityVertex {
    position: [f32; 2],
    tex_coords: [f32; 2],
}

const FULLSCREEN_QUAD: [IntensityVertex; 6] = [
    IntensityVertex {
        position: [-1.0, -1.0],
        tex_coords: [0.0, 1.0],
    },
    IntensityVertex {
        position: [1.0, -1.0],
        tex_coords: [1.0, 1.0],
    },
    IntensityVertex {
        position: [1.0, 1.0],
        tex_coords: [1.0, 0.0],
    },
    IntensityVertex {
        position: [-1.0, -1.0],
        tex_coords: [0.0, 1.0],
    },
    IntensityVertex {
        position: [1.0, 1.0],
        tex_coords: [1.0, 0.0],
    },
    IntensityVertex {
        position: [-1.0, 1.0],
        tex_coords: [0.0, 0.0],
    },
];

/// Post-processing effect that applies intensity-based visual enhancements.
pub struct IntensityEffect {
    pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    uniform_buffer: wgpu::Buffer,
    fullscreen_quad: wgpu::Buffer,
    sampler: wgpu::Sampler,

    // Current effect parameters
    intensity: f32,
    momentum: f32,
    bass: f32,
    time: f32,
}

impl IntensityEffect {
    /// Create a new intensity effect renderer.
    pub fn new(device: &wgpu::Device) -> Self {
        // Create sampler
        let sampler_desc = wgpu::SamplerBuilder::new()
            .mag_filter(wgpu::FilterMode::Linear)
            .min_filter(wgpu::FilterMode::Linear)
            .address_mode(wgpu::AddressMode::ClampToEdge)
            .into_descriptor();
        let sampler = device.create_sampler(&sampler_desc);

        // Create uniform buffer
        let uniforms = IntensityUniforms {
            intensity: 0.0,
            momentum: 0.0,
            bass: 0.0,
            time: 0.0,
        };
        let uniform_buffer = device.create_buffer_init(&wgpu::BufferInitDescriptor {
            label: Some("Intensity Uniforms"),
            contents: bytemuck::cast_slice(&[uniforms]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Create bind group layout
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Intensity Bind Group Layout"),
            entries: &[
                // Input texture
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                // Sampler
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu_types::SamplerBindingType::Filtering),
                    count: None,
                },
                // Uniforms
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        // Create fullscreen quad buffer
        let fullscreen_quad = device.create_buffer_init(&wgpu::BufferInitDescriptor {
            label: Some("Intensity Fullscreen Quad"),
            contents: bytemuck::cast_slice(&FULLSCREEN_QUAD),
            usage: wgpu::BufferUsages::VERTEX,
        });

        // Load shader
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Intensity Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/intensity.wgsl").into()),
        });

        // Create pipeline layout
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Intensity Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        // Create render pipeline
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Intensity Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<IntensityVertex>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        wgpu::VertexAttribute {
                            offset: 0,
                            shader_location: 0,
                            format: wgpu::VertexFormat::Float32x2,
                        },
                        wgpu::VertexAttribute {
                            offset: 8,
                            shader_location: 1,
                            format: wgpu::VertexFormat::Float32x2,
                        },
                    ],
                }],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Bgra8UnormSrgb,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        Self {
            pipeline,
            bind_group_layout,
            uniform_buffer,
            fullscreen_quad,
            sampler,
            intensity: 0.0,
            momentum: 0.0,
            bass: 0.0,
            time: 0.0,
        }
    }

    /// Update effect parameters from audio analysis.
    pub fn update(&mut self, intensity: f32, momentum: f32, bass: f32, delta_time: f32) {
        self.intensity = intensity;
        self.momentum = momentum;
        self.bass = bass;
        self.time += delta_time;
    }

    /// Apply intensity effects to the input texture, writing to output texture.
    pub fn apply(
        &self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        queue: &wgpu::Queue,
        input_view: &wgpu::TextureView,
        output_view: &wgpu::TextureView,
    ) {
        // Update uniforms
        let uniforms = IntensityUniforms {
            intensity: self.intensity,
            momentum: self.momentum,
            bass: self.bass,
            time: self.time,
        };
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));

        // Create bind group for this render
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Intensity Bind Group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(input_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.uniform_buffer.as_entire_binding(),
                },
            ],
        });

        // Render intensity effect
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Intensity Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: output_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });

            render_pass.set_pipeline(&self.pipeline);
            render_pass.set_bind_group(0, &bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.fullscreen_quad.slice(..));
            render_pass.draw(0..6, 0..1);
        }
    }
}
