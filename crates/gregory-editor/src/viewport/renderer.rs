//! 3D ビューの wgpu の資源（パイプライン・バッファ）と描画。

use std::num::NonZeroU64;

use bytemuck::{Pod, Zeroable};

use crate::mesh_buffers::{LineVertex, PointInstance, SceneGeometry, SurfaceVertex};

/// シェーダの `Uniforms` と同じ並び。
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct Uniforms {
    pub view_proj: [[f32; 4]; 4],
    pub eye: [f32; 4],
    pub right: [f32; 4],
    pub up: [f32; 4],
    pub forward: [f32; 4],
    pub surface_color: [f32; 4],
    pub params: [f32; 4],
}

/// 何を描くか。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Visibility {
    pub surface: bool,
    pub cage: bool,
}

/// 大きさが足りなくなったら作り直す、書き込み用のバッファ。
struct GrowableBuffer {
    buffer: wgpu::Buffer,
    capacity: u64,
    usage: wgpu::BufferUsages,
    label: &'static str,
}

impl GrowableBuffer {
    const MIN_CAPACITY: u64 = 256;

    fn new(device: &wgpu::Device, label: &'static str, usage: wgpu::BufferUsages) -> Self {
        let usage = usage | wgpu::BufferUsages::COPY_DST;
        Self {
            buffer: create_buffer(device, label, usage, Self::MIN_CAPACITY),
            capacity: Self::MIN_CAPACITY,
            usage,
            label,
        }
    }

    fn write(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, bytes: &[u8]) {
        let len = bytes.len() as u64;
        if len > self.capacity {
            let capacity = len.checked_next_power_of_two().unwrap_or(len);
            self.buffer = create_buffer(device, self.label, self.usage, capacity);
            self.capacity = capacity;
        }
        if !bytes.is_empty() {
            queue.write_buffer(&self.buffer, 0, bytes);
        }
    }
}

fn create_buffer(
    device: &wgpu::Device,
    label: &'static str,
    usage: wgpu::BufferUsages,
    size: u64,
) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(label),
        size,
        usage,
        mapped_at_creation: false,
    })
}

fn count_u32(len: usize) -> u32 {
    u32::try_from(len).unwrap_or(u32::MAX)
}

/// egui_wgpu の `CallbackResources` に1つだけ置く。
pub struct SceneRenderer {
    surface_pipeline: wgpu::RenderPipeline,
    line_pipeline: wgpu::RenderPipeline,
    point_pipeline: wgpu::RenderPipeline,
    uniform_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    surface_vertices: GrowableBuffer,
    surface_indices: GrowableBuffer,
    line_vertices: GrowableBuffer,
    point_instances: GrowableBuffer,
    surface_index_count: u32,
    line_vertex_count: u32,
    point_count: u32,
    uploaded_revision: Option<u64>,
    output_is_srgb: bool,
}

impl SceneRenderer {
    pub fn new(
        device: &wgpu::Device,
        target_format: wgpu::TextureFormat,
        depth_format: Option<wgpu::TextureFormat>,
    ) -> Self {
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("gregory_scene"),
            source: wgpu::ShaderSource::Wgsl(include_str!("scene.wgsl").into()),
        });
        let uniform_size = std::mem::size_of::<Uniforms>() as u64;
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("gregory_scene_uniforms"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: NonZeroU64::new(uniform_size),
                },
                count: None,
            }],
        });
        let uniform_buffer = create_buffer(
            device,
            "gregory_scene_uniforms",
            wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            uniform_size,
        );
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("gregory_scene_uniforms"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("gregory_scene"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });
        let factory = PipelineFactory {
            device,
            module: &module,
            layout: &layout,
            target_format,
            depth_format,
        };

        let surface_attributes = wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3];
        let surface_pipeline = factory.create(&PipelineSpec {
            label: "gregory_surface",
            vertex_entry: "vs_surface",
            fragment_entry: "fs_surface",
            topology: wgpu::PrimitiveTopology::TriangleList,
            buffer: wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<SurfaceVertex>() as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &surface_attributes,
            },
            always_on_top: false,
        });
        let line_attributes = wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3];
        let line_pipeline = factory.create(&PipelineSpec {
            label: "gregory_cage_lines",
            vertex_entry: "vs_line",
            fragment_entry: "fs_color",
            topology: wgpu::PrimitiveTopology::LineList,
            buffer: wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<LineVertex>() as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &line_attributes,
            },
            always_on_top: true,
        });
        let point_attributes = wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3];
        let point_pipeline = factory.create(&PipelineSpec {
            label: "gregory_cage_points",
            vertex_entry: "vs_point",
            fragment_entry: "fs_color",
            topology: wgpu::PrimitiveTopology::TriangleList,
            buffer: wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<PointInstance>() as u64,
                step_mode: wgpu::VertexStepMode::Instance,
                attributes: &point_attributes,
            },
            always_on_top: true,
        });

        Self {
            surface_pipeline,
            line_pipeline,
            point_pipeline,
            uniform_buffer,
            bind_group,
            surface_vertices: GrowableBuffer::new(
                device,
                "gregory_surface_vertices",
                wgpu::BufferUsages::VERTEX,
            ),
            surface_indices: GrowableBuffer::new(
                device,
                "gregory_surface_indices",
                wgpu::BufferUsages::INDEX,
            ),
            line_vertices: GrowableBuffer::new(
                device,
                "gregory_cage_lines",
                wgpu::BufferUsages::VERTEX,
            ),
            point_instances: GrowableBuffer::new(
                device,
                "gregory_cage_points",
                wgpu::BufferUsages::VERTEX,
            ),
            surface_index_count: 0,
            line_vertex_count: 0,
            point_count: 0,
            uploaded_revision: None,
            output_is_srgb: target_format.is_srgb(),
        }
    }

    /// 形が変わったときだけ頂点を送り直し、カメラの値は毎フレーム送る。
    pub fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        geometry: &SceneGeometry,
        revision: u64,
        uniforms: &Uniforms,
    ) {
        if self.uploaded_revision != Some(revision) {
            self.upload(device, queue, geometry);
            self.uploaded_revision = Some(revision);
        }
        let mut uniforms = *uniforms;
        uniforms.params[3] = if self.output_is_srgb { 1.0 } else { 0.0 };
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&uniforms));
    }

    fn upload(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, geometry: &SceneGeometry) {
        let surface = &geometry.surface;
        self.surface_vertices
            .write(device, queue, bytemuck::cast_slice(&surface.vertices));
        self.surface_indices
            .write(device, queue, bytemuck::cast_slice(&surface.indices));
        self.line_vertices
            .write(device, queue, bytemuck::cast_slice(&geometry.cage_lines));
        self.point_instances
            .write(device, queue, bytemuck::cast_slice(&geometry.cage_points));
        self.surface_index_count = count_u32(surface.indices.len());
        // LineList は2頂点で1本なので、端数の頂点は描かない
        self.line_vertex_count = count_u32(geometry.cage_lines.len() & !1);
        self.point_count = count_u32(geometry.cage_points.len());
    }

    pub fn paint(&self, pass: &mut wgpu::RenderPass<'static>, visibility: Visibility) {
        pass.set_bind_group(0, &self.bind_group, &[]);
        if visibility.surface && self.surface_index_count > 0 {
            pass.set_pipeline(&self.surface_pipeline);
            pass.set_vertex_buffer(0, self.surface_vertices.buffer.slice(..));
            pass.set_index_buffer(
                self.surface_indices.buffer.slice(..),
                wgpu::IndexFormat::Uint32,
            );
            pass.draw_indexed(0..self.surface_index_count, 0, 0..1);
        }
        if visibility.cage && self.line_vertex_count > 0 {
            pass.set_pipeline(&self.line_pipeline);
            pass.set_vertex_buffer(0, self.line_vertices.buffer.slice(..));
            pass.draw(0..self.line_vertex_count, 0..1);
        }
        if visibility.cage && self.point_count > 0 {
            pass.set_pipeline(&self.point_pipeline);
            pass.set_vertex_buffer(0, self.point_instances.buffer.slice(..));
            pass.draw(0..6, 0..self.point_count);
        }
    }
}

struct PipelineSpec<'a> {
    label: &'static str,
    vertex_entry: &'static str,
    fragment_entry: &'static str,
    topology: wgpu::PrimitiveTopology,
    buffer: wgpu::VertexBufferLayout<'a>,
    /// 曲面との前後に関係なく手前に描く（深度を書かない）。
    always_on_top: bool,
}

struct PipelineFactory<'a> {
    device: &'a wgpu::Device,
    module: &'a wgpu::ShaderModule,
    layout: &'a wgpu::PipelineLayout,
    target_format: wgpu::TextureFormat,
    depth_format: Option<wgpu::TextureFormat>,
}

impl PipelineFactory<'_> {
    fn create(&self, spec: &PipelineSpec<'_>) -> wgpu::RenderPipeline {
        let depth_stencil = self.depth_format.map(|format| wgpu::DepthStencilState {
            format,
            depth_write_enabled: Some(!spec.always_on_top),
            depth_compare: Some(if spec.always_on_top {
                wgpu::CompareFunction::Always
            } else {
                wgpu::CompareFunction::LessEqual
            }),
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        });
        self.device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(spec.label),
                layout: Some(self.layout),
                vertex: wgpu::VertexState {
                    module: self.module,
                    entry_point: Some(spec.vertex_entry),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    buffers: &[Some(spec.buffer.clone())],
                },
                primitive: wgpu::PrimitiveState {
                    topology: spec.topology,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: None,
                    unclipped_depth: false,
                    polygon_mode: wgpu::PolygonMode::Fill,
                    conservative: false,
                },
                depth_stencil,
                multisample: wgpu::MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                fragment: Some(wgpu::FragmentState {
                    module: self.module,
                    entry_point: Some(spec.fragment_entry),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: self.target_format,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview_mask: None,
                cache: None,
            })
    }
}
