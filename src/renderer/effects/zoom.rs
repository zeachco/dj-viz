//! Feedback buffer renderer for GPU-accelerated trail effects.
//!
//! Uses ping-pong textures and a fade/scale shader to create trails
//! without re-rendering historical frames. Supports burn-blending
//! overlay visualizations on top.

use nannou::prelude::*;
use nannou::wgpu;

const MAX_OVERLAYS: usize = 3;

/// Vertex for fullscreen quad
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct FeedbackVertex {
    position: [f32; 2],
    tex_coords: [f32; 2],
}

/// Uniform buffer for fade/scale parameters
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniforms {
    fade: f32,
    scale: f32,
    _padding: [f32; 2],
}

const FULLSCREEN_QUAD: [FeedbackVertex; 6] = [
    FeedbackVertex {
        position: [-1.0, -1.0],
        tex_coords: [0.0, 1.0],
    },
    FeedbackVertex {
        position: [1.0, -1.0],
        tex_coords: [1.0, 1.0],
    },
    FeedbackVertex {
        position: [1.0, 1.0],
        tex_coords: [1.0, 0.0],
    },
    FeedbackVertex {
        position: [-1.0, -1.0],
        tex_coords: [0.0, 1.0],
    },
    FeedbackVertex {
        position: [1.0, 1.0],
        tex_coords: [1.0, 0.0],
    },
    FeedbackVertex {
        position: [-1.0, 1.0],
        tex_coords: [0.0, 0.0],
    },
];

/// Feedback renderer using ping-pong textures for trail effects.
pub struct FeedbackRenderer {
    // Ping-pong textures
    textures: [wgpu::Texture; 2],
    texture_views: [wgpu::TextureView; 2],
    current_idx: usize,

    // For drawing visualizations to texture
    draw_renderer: nannou::draw::Renderer,

    // For fade/scale pass
    fade_pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    bind_groups: [wgpu::BindGroup; 2],
    fullscreen_quad: wgpu::Buffer,
    sampler: wgpu::Sampler,
    uniform_buffer: wgpu::Buffer,

    // Overlay textures for burn blending
    overlay_textures: Vec<wgpu::Texture>,
    overlay_texture_views: Vec<wgpu::TextureView>,
    overlay_draw_renderers: Vec<nannou::draw::Renderer>,

    // Burn blend pipeline
    burn_pipeline: wgpu::RenderPipeline,
    burn_bind_group_layout: wgpu::BindGroupLayout,

    // For displaying result to screen
    reshaper: wgpu::TextureReshaper,

    // Parameters
    pub fade: f32,
    pub scale: f32,

    // Texture size
    size: [u32; 2],
}

impl FeedbackRenderer {
    /// Create a new feedback renderer.
    ///
    /// # Arguments
    /// * `device` - wgpu device
    /// * `queue` - wgpu queue (needed for initial uniform upload)
    /// * `size` - texture dimensions [width, height]
    /// * `window_sample_count` - MSAA sample count of the window
    /// * `window_format` - texture format of the window
    pub fn new(
        device: &wgpu::Device,
        _queue: &wgpu::Queue,
        size: [u32; 2],
        window_sample_count: u32,
        window_format: wgpu::TextureFormat,
    ) -> Self {
        // Default parameters - can be tuned
        let fade = 0.97; // 3% fade per frame
        let scale = 1.003; // Slight zoom out for spiral effect

        // Create two textures for ping-pong
        let textures = [
            Self::create_texture(device, size),
            Self::create_texture(device, size),
        ];
        let texture_views = [textures[0].view().build(), textures[1].view().build()];

        // Create draw renderer for rendering nannou Draw to texture
        let draw_renderer = nannou::draw::RendererBuilder::new()
            .build_from_texture_descriptor(device, textures[0].descriptor());

        // Create sampler
        let sampler_desc = wgpu::SamplerBuilder::new()
            .mag_filter(wgpu::FilterMode::Linear)
            .min_filter(wgpu::FilterMode::Linear)
            .address_mode(wgpu::AddressMode::ClampToEdge)
            .into_descriptor();
        let sampler = device.create_sampler(&sampler_desc);

        // Create uniform buffer
        let uniforms = Uniforms {
            fade,
            scale,
            _padding: [0.0; 2],
        };
        let uniform_buffer = device.create_buffer_init(&wgpu::BufferInitDescriptor {
            label: Some("Feedback Uniforms"),
            contents: bytemuck::cast_slice(&[uniforms]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Create bind group layout
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Feedback Bind Group Layout"),
            entries: &[
                // Texture
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
                // Sampler (filtering sampler for linear interpolation)
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

        // Create bind groups for each texture
        let bind_groups = [
            Self::create_bind_group(
                device,
                &bind_group_layout,
                &texture_views[0],
                &sampler,
                &uniform_buffer,
            ),
            Self::create_bind_group(
                device,
                &bind_group_layout,
                &texture_views[1],
                &sampler,
                &uniform_buffer,
            ),
        ];

        // Create fullscreen quad buffer
        let fullscreen_quad = device.create_buffer_init(&wgpu::BufferInitDescriptor {
            label: Some("Fullscreen Quad"),
            contents: bytemuck::cast_slice(&FULLSCREEN_QUAD),
            usage: wgpu::BufferUsages::VERTEX,
        });

        // Load and create shader
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Feedback Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/feedback.wgsl").into()),
        });

        // Create pipeline layout
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Feedback Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        // Create render pipeline
        let fade_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Feedback Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<FeedbackVertex>() as wgpu::BufferAddress,
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

        // Create reshaper for final output (Bgra8UnormSrgb requires float filterable)
        let reshaper = wgpu::TextureReshaper::new(
            device,
            &texture_views[0],
            1,
            wgpu::TextureSampleType::Float { filterable: true },
            window_sample_count,
            window_format,
        );

        // Create overlay textures
        let overlay_textures: Vec<wgpu::Texture> = (0..MAX_OVERLAYS)
            .map(|_| Self::create_texture(device, size))
            .collect();
        let overlay_texture_views: Vec<wgpu::TextureView> =
            overlay_textures.iter().map(|t| t.view().build()).collect();
        let overlay_draw_renderers: Vec<nannou::draw::Renderer> = overlay_textures
            .iter()
            .map(|t| {
                nannou::draw::RendererBuilder::new()
                    .build_from_texture_descriptor(device, t.descriptor())
            })
            .collect();

        // Create burn blend shader and pipeline
        let burn_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Burn Blend Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/burn_blend.wgsl").into()),
        });

        let burn_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Burn Blend Bind Group Layout"),
                entries: &[
                    // Base texture
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
                    // Overlay texture
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
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
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu_types::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

        let burn_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Burn Blend Pipeline Layout"),
            bind_group_layouts: &[&burn_bind_group_layout],
            push_constant_ranges: &[],
        });

        let burn_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Burn Blend Pipeline"),
            layout: Some(&burn_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &burn_shader,
                entry_point: "vs_main",
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<FeedbackVertex>() as wgpu::BufferAddress,
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
                module: &burn_shader,
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
            textures,
            texture_views,
            current_idx: 0,
            draw_renderer,
            fade_pipeline,
            bind_group_layout,
            bind_groups,
            fullscreen_quad,
            sampler,
            uniform_buffer,
            overlay_textures,
            overlay_texture_views,
            overlay_draw_renderers,
            burn_pipeline,
            burn_bind_group_layout,
            reshaper,
            fade,
            scale,
            size,
        }
    }

    fn create_texture(device: &wgpu::Device, size: [u32; 2]) -> wgpu::Texture {
        wgpu::TextureBuilder::new()
            .size(size)
            .usage(wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING)
            .sample_count(1)
            .format(wgpu::TextureFormat::Bgra8UnormSrgb)
            .build(device)
    }

    fn create_bind_group(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        texture_view: &wgpu::TextureView,
        sampler: &wgpu::Sampler,
        uniform_buffer: &wgpu::Buffer,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Feedback Bind Group"),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: uniform_buffer.as_entire_binding(),
                },
            ],
        })
    }

    /// Update uniform buffer with current fade/scale values
    fn update_uniforms(&self, queue: &wgpu::Queue) {
        let uniforms = Uniforms {
            fade: self.fade,
            scale: self.scale,
            _padding: [0.0; 2],
        };
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));
    }

    /// Handle window resize by recreating textures.
    pub fn resize(
        &mut self,
        device: &wgpu::Device,
        size: [u32; 2],
        window_sample_count: u32,
        window_format: wgpu::TextureFormat,
    ) {
        if size == self.size {
            return;
        }
        self.size = size;

        // Recreate textures
        self.textures = [
            Self::create_texture(device, size),
            Self::create_texture(device, size),
        ];
        self.texture_views = [
            self.textures[0].view().build(),
            self.textures[1].view().build(),
        ];

        // Recreate draw renderer
        self.draw_renderer = nannou::draw::RendererBuilder::new()
            .build_from_texture_descriptor(device, self.textures[0].descriptor());

        // Recreate bind groups
        self.bind_groups = [
            Self::create_bind_group(
                device,
                &self.bind_group_layout,
                &self.texture_views[0],
                &self.sampler,
                &self.uniform_buffer,
            ),
            Self::create_bind_group(
                device,
                &self.bind_group_layout,
                &self.texture_views[1],
                &self.sampler,
                &self.uniform_buffer,
            ),
        ];

        // Recreate reshaper
        self.reshaper = wgpu::TextureReshaper::new(
            device,
            &self.texture_views[0],
            1,
            wgpu::TextureSampleType::Float { filterable: true },
            window_sample_count,
            window_format,
        );

        // Recreate overlay textures
        self.overlay_textures = (0..MAX_OVERLAYS)
            .map(|_| Self::create_texture(device, size))
            .collect();
        self.overlay_texture_views = self
            .overlay_textures
            .iter()
            .map(|t| t.view().build())
            .collect();
        self.overlay_draw_renderers = self
            .overlay_textures
            .iter()
            .map(|t| {
                nannou::draw::RendererBuilder::new()
                    .build_from_texture_descriptor(device, t.descriptor())
            })
            .collect();

        self.current_idx = 0;
    }

    /// Create a bind group for burn blending two textures
    fn create_burn_bind_group(
        &self,
        device: &wgpu::Device,
        base_view: &wgpu::TextureView,
        overlay_view: &wgpu::TextureView,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Burn Blend Bind Group"),
            layout: &self.burn_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(base_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(overlay_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        })
    }

    /// Render a frame with feedback effect and overlay burn blending.
    ///
    /// # Arguments
    /// * `device` - wgpu device
    /// * `queue` - wgpu queue
    /// * `primary_draw` - nannou Draw with primary visualization
    /// * `overlay_draws` - nannou Draws with overlay visualizations (up to 3)
    /// * `frame_view` - texture view of the output frame
    /// * `frame_format` - format of the output frame
    /// * `frame_sample_count` - MSAA sample count of the output frame
    pub fn render_with_overlays(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        primary_draw: &nannou::Draw,
        overlay_draws: &[&nannou::Draw],
        frame_view: &wgpu::TextureView,
        frame_format: wgpu::TextureFormat,
        frame_sample_count: u32,
    ) {
        // Update uniforms in case fade/scale changed
        self.update_uniforms(queue);

        let prev_idx = self.current_idx;
        let curr_idx = 1 - prev_idx;

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Feedback Encoder"),
        });

        // Pass 1: Render previous frame with fade/scale to current texture
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Feedback Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.texture_views[curr_idx],
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });

            render_pass.set_pipeline(&self.fade_pipeline);
            render_pass.set_bind_group(0, &self.bind_groups[prev_idx], &[]);
            render_pass.set_vertex_buffer(0, self.fullscreen_quad.slice(..));
            render_pass.draw(0..6, 0..1);
        }

        // Pass 2: Draw current primary visualization on top
        self.draw_renderer.render_to_texture(
            device,
            &mut encoder,
            primary_draw,
            &self.textures[curr_idx],
        );

        // Pass 3: Render each overlay and blend onto the result using ping-pong
        let num_overlays = overlay_draws.len().min(MAX_OVERLAYS);
        let mut read_idx = curr_idx;
        let mut write_idx = 1 - curr_idx;

        for i in 0..num_overlays {
            // Clear and render overlay to its texture
            {
                let _render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Overlay Clear Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &self.overlay_texture_views[i],
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: true,
                        },
                    })],
                    depth_stencil_attachment: None,
                });
            }

            self.overlay_draw_renderers[i].render_to_texture(
                device,
                &mut encoder,
                overlay_draws[i],
                &self.overlay_textures[i],
            );

            // Blend the overlay onto the current texture, output to the other texture
            let blend_bind_group = self.create_burn_bind_group(
                device,
                &self.texture_views[read_idx],
                &self.overlay_texture_views[i],
            );

            {
                let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Blend Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &self.texture_views[write_idx],
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: true,
                        },
                    })],
                    depth_stencil_attachment: None,
                });

                render_pass.set_pipeline(&self.burn_pipeline);
                render_pass.set_bind_group(0, &blend_bind_group, &[]);
                render_pass.set_vertex_buffer(0, self.fullscreen_quad.slice(..));
                render_pass.draw(0..6, 0..1);
            }

            // Swap for next overlay
            std::mem::swap(&mut read_idx, &mut write_idx);
        }

        // After overlays, read_idx contains the final result
        // If no overlays, final result is still in curr_idx
        let final_idx = if num_overlays > 0 { read_idx } else { curr_idx };

        // Pass 4: Copy final feedback result to frame
        let reshaper = wgpu::TextureReshaper::new(
            device,
            &self.texture_views[final_idx],
            1,
            wgpu::TextureSampleType::Float { filterable: true },
            frame_sample_count,
            frame_format,
        );
        reshaper.encode_render_pass(frame_view, &mut encoder);

        queue.submit(Some(encoder.finish()));

        // Set current_idx for next frame's feedback (should read from final result)
        self.current_idx = final_idx;
    }
}
