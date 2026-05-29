//! The wgpu render pipeline.
//!
//! Each frame is a two-pass affair that gives kefirdrop its MilkDrop-style
//! feedback trails:
//!
//! 1. **Scene pass** renders the active preset into one of two ping-pong
//!    `rgba16float` offscreen textures, while sampling the *other* one (the
//!    previous frame) via `sample_prev` in the shader.
//! 2. **Blit pass** copies the freshly rendered texture onto the swapchain.
//!
//! After presenting, the two textures swap roles. The scene pass fully
//! overwrites its target (no blending), so feedback comes purely from sampling
//! the previous frame inside the shader.

use anyhow::{Context, Result};
use wgpu::util::DeviceExt;

use super::shader;
use super::uniforms::Uniforms;

/// Offscreen accumulation format — float so trails can fade smoothly without banding.
const TARGET_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;

pub struct Renderer {
    width: u32,
    height: u32,

    prelude: String,

    uniform_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,
    uniform_layout: wgpu::BindGroupLayout,

    /// Layout for a sampled texture + sampler (used by both the scene's
    /// previous-frame input and the blit source).
    tex_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,

    targets: [wgpu::Texture; 2],
    target_views: [wgpu::TextureView; 2],
    /// Bind group `i` samples `targets[i]` (as the scene's previous frame).
    scene_tex_groups: [wgpu::BindGroup; 2],
    /// Bind group `i` samples `targets[i]` (as the blit source).
    blit_tex_groups: [wgpu::BindGroup; 2],

    scene_pipeline: wgpu::RenderPipeline,
    blit_pipeline: wgpu::RenderPipeline,

    /// Index of the texture the next scene pass writes into.
    write_index: usize,
}

impl Renderer {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        surface_format: wgpu::TextureFormat,
        width: u32,
        height: u32,
        prelude: String,
        first_preset: &str,
    ) -> Result<Self> {
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("uniforms"),
            contents: bytemuck::bytes_of(&Uniforms::default()),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let uniform_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("uniform-layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("uniform-bind-group"),
            layout: &uniform_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        let tex_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("texture-layout"),
            entries: &[
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
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("frame-sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let scene_pipeline =
            build_scene_pipeline(device, &uniform_layout, &tex_layout, &prelude, first_preset)?;
        let blit_pipeline = build_blit_pipeline(device, &tex_layout, surface_format);

        let (targets, target_views, scene_tex_groups, blit_tex_groups) =
            create_targets(device, &tex_layout, &sampler, width, height);

        let renderer = Self {
            width,
            height,
            prelude,
            uniform_buffer,
            uniform_bind_group,
            uniform_layout,
            tex_layout,
            sampler,
            targets,
            target_views,
            scene_tex_groups,
            blit_tex_groups,
            scene_pipeline,
            blit_pipeline,
            write_index: 0,
        };
        renderer.clear_targets(device, queue);
        Ok(renderer)
    }

    /// Recreate the offscreen targets for a new surface size.
    pub fn resize(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, width: u32, height: u32) {
        if width == 0 || height == 0 || (width == self.width && height == self.height) {
            return;
        }
        self.width = width;
        self.height = height;
        let (targets, views, scene_groups, blit_groups) =
            create_targets(device, &self.tex_layout, &self.sampler, width, height);
        self.targets = targets;
        self.target_views = views;
        self.scene_tex_groups = scene_groups;
        self.blit_tex_groups = blit_groups;
        self.write_index = 0;
        self.clear_targets(device, queue);
    }

    /// Swap in a new preset shader, rebuilding the scene pipeline. On failure
    /// (e.g. a shader typo during hot-reload) the current pipeline is kept.
    pub fn set_preset(&mut self, device: &wgpu::Device, preset_body: &str) -> Result<()> {
        let pipeline = build_scene_pipeline(
            device,
            &self.uniform_layout,
            &self.tex_layout,
            &self.prelude,
            preset_body,
        )?;
        self.scene_pipeline = pipeline;
        Ok(())
    }

    /// Render one frame and present it onto `surface_view`.
    pub fn render(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        surface_view: &wgpu::TextureView,
        uniforms: &Uniforms,
    ) {
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(uniforms));

        let w = self.write_index;
        let r = 1 - w; // previous frame

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("frame"),
        });

        // Scene pass: render the preset into targets[w], sampling targets[r].
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("scene"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.target_views[w],
                    resolve_target: None,
                    ops: wgpu::Operations {
                        // The shader fully overwrites every pixel, so the prior
                        // contents are irrelevant; Load avoids a needless clear.
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_pipeline(&self.scene_pipeline);
            pass.set_bind_group(0, &self.uniform_bind_group, &[]);
            pass.set_bind_group(1, &self.scene_tex_groups[r], &[]);
            pass.draw(0..3, 0..1);
        }

        // Blit pass: copy targets[w] onto the swapchain.
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("blit"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: surface_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_pipeline(&self.blit_pipeline);
            pass.set_bind_group(0, &self.blit_tex_groups[w], &[]);
            pass.draw(0..3, 0..1);
        }

        queue.submit(std::iter::once(encoder.finish()));

        // Swap: next frame writes into what we just sampled as "previous".
        self.write_index = r;
    }

    /// Clear both offscreen targets to black so the first frames sample defined data.
    fn clear_targets(&self, device: &wgpu::Device, queue: &wgpu::Queue) {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("clear"),
        });
        for view in &self.target_views {
            encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("clear-target"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
        }
        queue.submit(std::iter::once(encoder.finish()));
    }
}

/// Create the two ping-pong textures and their bind groups.
fn create_targets(
    device: &wgpu::Device,
    tex_layout: &wgpu::BindGroupLayout,
    sampler: &wgpu::Sampler,
    width: u32,
    height: u32,
) -> (
    [wgpu::Texture; 2],
    [wgpu::TextureView; 2],
    [wgpu::BindGroup; 2],
    [wgpu::BindGroup; 2],
) {
    let make_texture = |label: &str| {
        device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: TARGET_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        })
    };

    let targets = [make_texture("target-0"), make_texture("target-1")];
    let target_views = [
        targets[0].create_view(&wgpu::TextureViewDescriptor::default()),
        targets[1].create_view(&wgpu::TextureViewDescriptor::default()),
    ];

    let make_group = |label: &str, view: &wgpu::TextureView| {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(label),
            layout: tex_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
            ],
        })
    };

    let scene_tex_groups = [
        make_group("scene-tex-0", &target_views[0]),
        make_group("scene-tex-1", &target_views[1]),
    ];
    let blit_tex_groups = [
        make_group("blit-tex-0", &target_views[0]),
        make_group("blit-tex-1", &target_views[1]),
    ];

    (targets, target_views, scene_tex_groups, blit_tex_groups)
}

fn build_scene_pipeline(
    device: &wgpu::Device,
    uniform_layout: &wgpu::BindGroupLayout,
    tex_layout: &wgpu::BindGroupLayout,
    prelude: &str,
    preset_body: &str,
) -> Result<wgpu::RenderPipeline> {
    let source = shader::assemble(prelude, preset_body);
    let module = create_shader_module_checked(device, "scene-shader", &source)
        .context("failed to compile preset shader")?;

    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("scene-layout"),
        bind_group_layouts: &[uniform_layout, tex_layout],
        push_constant_ranges: &[],
    });

    Ok(
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("scene-pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: TARGET_FORMAT,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        }),
    )
}

fn build_blit_pipeline(
    device: &wgpu::Device,
    tex_layout: &wgpu::BindGroupLayout,
    surface_format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("blit-shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("../../assets/shaders/blit.wgsl").into()),
    });

    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("blit-layout"),
        bind_group_layouts: &[tex_layout],
        push_constant_ranges: &[],
    });

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("blit-pipeline"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &module,
            entry_point: Some("vs_main"),
            buffers: &[],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &module,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format: surface_format,
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
        cache: None,
    })
}

/// Compile a shader module, surfacing WGSL parse/validation errors as a `Result`
/// instead of a panic, so a bad preset (e.g. during hot-reload) is recoverable.
fn create_shader_module_checked(
    device: &wgpu::Device,
    label: &str,
    source: &str,
) -> Result<wgpu::ShaderModule> {
    device.push_error_scope(wgpu::ErrorFilter::Validation);
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(label),
        source: wgpu::ShaderSource::Wgsl(source.into()),
    });
    if let Some(err) = pollster::block_on(device.pop_error_scope()) {
        anyhow::bail!("shader compilation error: {err}");
    }
    Ok(module)
}
