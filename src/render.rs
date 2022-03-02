use std::borrow::BorrowMut;
use std::ops::{Add, Mul};

use cgmath::{prelude::*, Matrix4};
use color_eyre::eyre::eyre;
use color_eyre::Help;
use wgpu::util::DeviceExt;

use crate::game_state;

pub struct RenderState {
    instance: wgpu::Instance,
    surface: wgpu::Surface,
    surface_config: wgpu::SurfaceConfiguration,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    shader: wgpu::ShaderModule,
    pipeline_layout: wgpu::PipelineLayout,
    pipeline: wgpu::RenderPipeline,
    transform_bind_group_layout: wgpu::BindGroupLayout,
    vertex_buffer: wgpu::Buffer,
}

impl RenderState {
    pub fn new(
        instance: wgpu::Instance,
        window: &winit::window::Window,
    ) -> color_eyre::Result<Self> {
        let surface = unsafe { instance.create_surface(window) };
        let adapter =
            futures::executor::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
            }))
            .ok_or_else(|| eyre!("failed to get adapter from wgpu").note("you probably don't have a graphics card that supports VULKAN/DX12 (or any other wgpu primary targets, if new ones have been added),\nor maybe this application just doesn't have access to it"))?;
        let preferred_format = surface.get_preferred_format(&adapter).unwrap();
        let winit::dpi::PhysicalSize { width, height } = window.inner_size();
        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: preferred_format,
            width,
            height,
            present_mode: wgpu::PresentMode::Mailbox,
        };
        let (device, queue) = futures::executor::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("the device, for rendering"),
                features: wgpu::Features::default(),
                limits: wgpu::Limits::downlevel_defaults(),
            },
            None,
        )).note("you have a graphics card, we have access to it, it just doesn't support the needed features/limits to get this thing running")?;
        surface.configure(&device, &surface_config);
        let shader = device.create_shader_module(&wgpu::include_wgsl!("shader.wgsl"));
        let transform_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("transform_bind_group_layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: (16 * std::mem::size_of::<f32>() as u64).try_into().ok(),
                    },
                    count: None,
                }],
            });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("render pipeline"),
            bind_group_layouts: &[&transform_bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("render pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: 2 * std::mem::size_of::<f32>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x2],
                }],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Cw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[wgpu::ColorTargetState {
                    format: surface_config.format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                }],
            }),
            multiview: None,
        });
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("vertex buffer"),
            contents: bytemuck::cast_slice(&[
                [0.0f32, 0.0f32],
                [1.0, 0.0],
                [0.0, 1.0],
                [1.0, 0.0],
                [1.0, 1.0],
                [0.0, 1.0],
            ]),
            usage: wgpu::BufferUsages::VERTEX,
        });
        Ok(Self {
            instance,
            adapter,
            surface,
            surface_config,
            device,
            queue,
            shader,
            pipeline_layout,
            pipeline,
            transform_bind_group_layout,
            vertex_buffer,
        })
    }

    pub fn render(
        &mut self,
        interpolate: f64,
        state: &game_state::GameState,
        last_state: &game_state::GameState,
    ) -> color_eyre::Result<()> {
        let center = last_state.center.lerp(state.center, interpolate);
        let angle = lerp(last_state.current_angle, state.current_angle, interpolate);
        let mut transform = Matrix4::from_nonuniform_scale(0.1, state.arm_length, 1.0);
        transform.concat_self(&Matrix4::from_angle_z(angle));
        transform.concat_self(&Matrix4::from_translation(center.extend(0.0)));
        let buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("transform buffer"),
                contents: bytemuck::cast_slice(AsRef::<[_; 16]>::as_ref(&transform)),
                usage: wgpu::BufferUsages::UNIFORM,
            });

        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("transform bind group"),
            layout: &self.transform_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
        });

        let frame = self.surface.get_current_texture()?;
        let frame_view = frame.texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some("render target"),
            ..Default::default()
        });

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("render pass encoder"),
            });
        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("render pass"),
                color_attachments: &[wgpu::RenderPassColorAttachment {
                    view: &frame_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: true,
                    },
                }],
                depth_stencil_attachment: None,
            });
            rpass.set_pipeline(&self.pipeline);
            rpass.set_bind_group(0, &bind_group, &[]);
            rpass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            rpass.draw(0..6, 0..1);
        }
        Ok(())
    }
}

struct PipelineAndRelated {}

fn lerp<T: Add<T> + Mul<f64, Output = T>>(from: T, to: T, interp_by: f64) -> <T as Add<T>>::Output {
    (from * interp_by) + (to * (1.0 - interp_by))
}
