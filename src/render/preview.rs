use std::{num::NonZeroU64, sync::Arc};

use bytemuck::{Pod, Zeroable};
use eframe::egui;
use eframe::egui_wgpu::{self, wgpu};
use glam::{Mat4, Vec3};
use wgpu::util::DeviceExt;

use crate::{
    geometry::{MeshData, MeshVertex},
    material::SkinMaterial,
};

const MESH_SHADER: &str = include_str!("mesh_preview.wgsl");
const GRID_SHADER: &str = include_str!("grid.wgsl");
const DISPLAY_SHADER: &str = include_str!("display.wgsl");

const MESH_VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 2] =
    wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3];

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct ViewportUniforms {
    view_projection: [[f32; 4]; 4],
    camera_position: [f32; 4],
    base_color: [f32; 4],
    surface: [f32; 4],
    pigment_detail: [f32; 4],
}

struct GpuMesh {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    index_count: u32,
    revision: u64,
}

struct ViewportTarget {
    _color_texture: wgpu::Texture,
    color_view: wgpu::TextureView,
    _depth_texture: wgpu::Texture,
    depth_view: wgpu::TextureView,
    display_bind_group: wgpu::BindGroup,
    width: u32,
    height: u32,
}

impl ViewportTarget {
    fn new(
        device: &wgpu::Device,
        display_layout: &wgpu::BindGroupLayout,
        sampler: &wgpu::Sampler,
        width: u32,
        height: u32,
    ) -> Self {
        let color_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Skin Studio 3D viewport color"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let color_view = color_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let depth_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Skin Studio 3D viewport depth"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let depth_view = depth_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let display_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Skin Studio viewport display texture binding"),
            layout: display_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&color_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
            ],
        });
        Self {
            _color_texture: color_texture,
            color_view,
            _depth_texture: depth_texture,
            depth_view,
            display_bind_group,
            width,
            height,
        }
    }
}

struct SkinRenderResources {
    mesh_pipeline: wgpu::RenderPipeline,
    grid_pipeline: wgpu::RenderPipeline,
    display_pipeline: wgpu::RenderPipeline,
    uniform_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,
    display_bind_group_layout: wgpu::BindGroupLayout,
    display_sampler: wgpu::Sampler,
    grid_buffer: wgpu::Buffer,
    grid_vertex_count: u32,
    mesh: Option<GpuMesh>,
    target: Option<ViewportTarget>,
}

fn mesh_vertex_layout() -> wgpu::VertexBufferLayout<'static> {
    wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<MeshVertex>() as u64,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &MESH_VERTEX_ATTRIBUTES,
    }
}

impl SkinRenderResources {
    fn new(state: &eframe::egui_wgpu::RenderState) -> Self {
        let device = &state.device;

        let uniform_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Skin Studio mesh preview uniforms"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: NonZeroU64::new(
                        std::mem::size_of::<ViewportUniforms>() as u64,
                    ),
                },
                count: None,
            }],
        });
        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Skin Studio viewport uniforms"),
            size: std::mem::size_of::<ViewportUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Skin Studio viewport uniform binding"),
            layout: &uniform_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        let mesh_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Skin Studio PBR-like mesh preview shader"),
            source: wgpu::ShaderSource::Wgsl(MESH_SHADER.into()),
        });
        let mesh_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Skin Studio mesh pipeline layout"),
            bind_group_layouts: &[Some(&uniform_layout)],
            immediate_size: 0,
        });
        let mesh_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Skin Studio depth-tested mesh pipeline"),
            layout: Some(&mesh_layout),
            vertex: wgpu::VertexState {
                module: &mesh_shader,
                entry_point: Some("vs_main"),
                buffers: &[mesh_vertex_layout()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &mesh_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::TextureFormat::Rgba8UnormSrgb.into())],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::LessEqual),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        let grid_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Skin Studio viewport grid shader"),
            source: wgpu::ShaderSource::Wgsl(GRID_SHADER.into()),
        });
        let grid_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Skin Studio depth-tested grid pipeline"),
            layout: Some(&mesh_layout),
            vertex: wgpu::VertexState {
                module: &grid_shader,
                entry_point: Some("vs_main"),
                buffers: &[mesh_vertex_layout()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &grid_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::TextureFormat::Rgba8UnormSrgb.into())],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineList,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: Some(false),
                depth_compare: Some(wgpu::CompareFunction::LessEqual),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        let display_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Skin Studio viewport texture layout"),
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
        let display_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Skin Studio viewport linear sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        let display_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Skin Studio viewport composition shader"),
            source: wgpu::ShaderSource::Wgsl(DISPLAY_SHADER.into()),
        });
        let display_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Skin Studio viewport composition layout"),
                bind_group_layouts: &[Some(&display_layout)],
                immediate_size: 0,
            });
        let display_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Skin Studio viewport composition pipeline"),
            layout: Some(&display_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &display_shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &display_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(state.target_format.into())],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        let grid_vertices = make_grid_vertices();
        let grid_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Skin Studio viewport grid vertices"),
            contents: bytemuck::cast_slice(&grid_vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        Self {
            mesh_pipeline,
            grid_pipeline,
            display_pipeline,
            uniform_buffer,
            uniform_bind_group,
            display_bind_group_layout: display_layout,
            display_sampler,
            grid_buffer,
            grid_vertex_count: grid_vertices.len() as u32,
            mesh: None,
            target: None,
        }
    }
}

fn make_grid_vertices() -> Vec<MeshVertex> {
    let mut vertices = Vec::with_capacity(204);
    let half = 2.5_f32;
    let floor_y = -0.88_f32;
    let steps = 50_i32;
    for step in -steps..=steps {
        let offset = step as f32 * 0.1;
        vertices.push(MeshVertex {
            position: [offset, floor_y, -half],
            normal: [0.0; 3],
        });
        vertices.push(MeshVertex {
            position: [offset, floor_y, half],
            normal: [0.0; 3],
        });
        vertices.push(MeshVertex {
            position: [-half, floor_y, offset],
            normal: [0.0; 3],
        });
        vertices.push(MeshVertex {
            position: [half, floor_y, offset],
            normal: [0.0; 3],
        });
    }
    vertices
}

#[derive(Clone, Copy)]
struct OrbitCamera {
    yaw: f32,
    pitch: f32,
    distance: f32,
    target: Vec3,
}

impl Default for OrbitCamera {
    fn default() -> Self {
        Self {
            yaw: 0.35,
            pitch: 0.12,
            distance: 3.0,
            target: Vec3::ZERO,
        }
    }
}

impl OrbitCamera {
    fn reset(&mut self) {
        *self = Self::default();
    }

    fn position(&self) -> Vec3 {
        let direction = Vec3::new(
            self.yaw.sin() * self.pitch.cos(),
            self.pitch.sin(),
            self.yaw.cos() * self.pitch.cos(),
        );
        self.target + direction * self.distance
    }

    fn view_projection(&self, aspect: f32) -> Mat4 {
        let view = Mat4::look_at_rh(self.position(), self.target, Vec3::Y);
        let projection = Mat4::perspective_rh(45_f32.to_radians(), aspect.max(0.1), 0.01, 100.0);
        projection * view
    }

    fn orbit(&mut self, delta: egui::Vec2) {
        self.yaw += delta.x * 0.008;
        self.pitch = (self.pitch - delta.y * 0.008).clamp(-1.48, 1.48);
    }

    fn pan(&mut self, delta: egui::Vec2) {
        let right = Vec3::new(self.yaw.cos(), 0.0, -self.yaw.sin());
        let scale = self.distance * 0.0018;
        self.target -= right * delta.x * scale;
        self.target += Vec3::Y * delta.y * scale;
    }

    fn zoom(&mut self, scroll: f32) {
        self.distance = (self.distance * (-scroll * 0.001).exp()).clamp(1.15, 30.0);
    }
}

pub struct SkinPreview {
    available: bool,
    mesh: Arc<MeshData>,
    mesh_revision: u64,
    camera: OrbitCamera,
}

impl SkinPreview {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mesh = Arc::new(MeshData::default_sphere());
        let Some(state) = cc.wgpu_render_state.as_ref() else {
            return Self {
                available: false,
                mesh,
                mesh_revision: 1,
                camera: OrbitCamera::default(),
            };
        };
        state
            .renderer
            .write()
            .callback_resources
            .insert(SkinRenderResources::new(state));

        Self {
            available: true,
            mesh,
            mesh_revision: 1,
            camera: OrbitCamera::default(),
        }
    }

    pub fn set_mesh(&mut self, mesh: MeshData) {
        self.mesh = Arc::new(mesh);
        self.mesh_revision = self.mesh_revision.wrapping_add(1);
        self.camera.reset();
    }

    pub fn mesh_name(&self) -> &str {
        &self.mesh.name
    }

    pub fn vertex_count(&self) -> usize {
        self.mesh.vertex_count()
    }

    pub fn triangle_count(&self) -> usize {
        self.mesh.triangle_count()
    }

    pub fn reset_camera(&mut self) {
        self.camera.reset();
    }

    pub fn show(&mut self, ui: &mut egui::Ui, params: SkinMaterial, show_grid: bool) {
        if !self.available {
            ui.centered_and_justified(|ui| {
                ui.label("O backend WGPU não foi inicializado. Verifique o suporte gráfico do sistema.");
            });
            return;
        }

        let desired_size = ui.available_size().max(egui::vec2(320.0, 320.0));
        let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::drag());

        if response.dragged() {
            let delta = ui.input(|input| input.pointer.delta());
            if ui.input(|input| input.modifiers.shift) {
                self.camera.pan(delta);
            } else {
                self.camera.orbit(delta);
            }
        }
        if response.hovered() {
            let scroll = ui.input(|input| input.smooth_scroll_delta.y);
            if scroll.abs() > 0.0 {
                self.camera.zoom(scroll);
            }
        }

        let pixels_per_point = ui.ctx().pixels_per_point();
        let width = (rect.width() * pixels_per_point).round().clamp(1.0, 4096.0) as u32;
        let height = (rect.height() * pixels_per_point).round().clamp(1.0, 4096.0) as u32;
        let view_projection = self
            .camera
            .view_projection(width as f32 / height.max(1) as f32);
        let uniform = ViewportUniforms {
            view_projection: view_projection.to_cols_array_2d(),
            camera_position: self.camera.position().extend(1.0).to_array(),
            base_color: [
                params.base_color[0],
                params.base_color[1],
                params.base_color[2],
                params.pore_strength,
            ],
            surface: [
                params.roughness,
                params.specular,
                params.subsurface,
                params.micro_relief,
            ],
            pigment_detail: [
                params.hemoglobin_strength,
                params.freckle_strength,
                params.oiliness,
                params.pigment_variation,
            ],
        };

        ui.painter().add(egui_wgpu::Callback::new_paint_callback(
            rect,
            MeshPreviewCallback {
                uniform,
                width,
                height,
                mesh: Arc::clone(&self.mesh),
                mesh_revision: self.mesh_revision,
                show_grid,
            },
        ));
    }
}

struct MeshPreviewCallback {
    uniform: ViewportUniforms,
    width: u32,
    height: u32,
    mesh: Arc<MeshData>,
    mesh_revision: u64,
    show_grid: bool,
}

impl eframe::egui_wgpu::CallbackTrait for MeshPreviewCallback {
    fn prepare(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        _screen_descriptor: &egui_wgpu::ScreenDescriptor,
        encoder: &mut wgpu::CommandEncoder,
        callback_resources: &mut egui_wgpu::CallbackResources,
    ) -> Vec<wgpu::CommandBuffer> {
        let resources: &mut SkinRenderResources = callback_resources
            .get_mut()
            .expect("Skin Studio renderer resources were not initialized");

        let needs_target = resources
            .target
            .as_ref()
            .is_none_or(|target| target.width != self.width || target.height != self.height);
        if needs_target {
            resources.target = Some(ViewportTarget::new(
                device,
                &resources.display_bind_group_layout,
                &resources.display_sampler,
                self.width,
                self.height,
            ));
        }

        let needs_mesh = resources
            .mesh
            .as_ref()
            .is_none_or(|mesh| mesh.revision != self.mesh_revision);
        if needs_mesh {
            let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Skin Studio active mesh vertices"),
                contents: bytemuck::cast_slice(&self.mesh.vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });
            let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Skin Studio active mesh indices"),
                contents: bytemuck::cast_slice(&self.mesh.indices),
                usage: wgpu::BufferUsages::INDEX,
            });
            resources.mesh = Some(GpuMesh {
                vertex_buffer,
                index_buffer,
                index_count: self.mesh.indices.len() as u32,
                revision: self.mesh_revision,
            });
        }

        queue.write_buffer(
            &resources.uniform_buffer,
            0,
            bytemuck::bytes_of(&self.uniform),
        );
        let target = resources
            .target
            .as_ref()
            .expect("viewport target should be available after creation");
        let mesh = resources
            .mesh
            .as_ref()
            .expect("preview mesh should be available after upload");

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Skin Studio 3D mesh viewport pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &target.color_view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.075,
                        g: 0.088,
                        b: 0.105,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &target.depth_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });

        render_pass.set_bind_group(0, &resources.uniform_bind_group, &[]);
        if self.show_grid {
            render_pass.set_pipeline(&resources.grid_pipeline);
            render_pass.set_vertex_buffer(0, resources.grid_buffer.slice(..));
            render_pass.draw(0..resources.grid_vertex_count, 0..1);
        }
        render_pass.set_pipeline(&resources.mesh_pipeline);
        render_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
        render_pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        render_pass.draw_indexed(0..mesh.index_count, 0, 0..1);
        drop(render_pass);

        Vec::new()
    }

    fn paint(
        &self,
        _info: egui::PaintCallbackInfo,
        render_pass: &mut wgpu::RenderPass<'static>,
        callback_resources: &egui_wgpu::CallbackResources,
    ) {
        let resources: &SkinRenderResources = callback_resources
            .get()
            .expect("Skin Studio renderer resources were not initialized");
        let target = resources
            .target
            .as_ref()
            .expect("viewport texture should be available before composition");
        render_pass.set_pipeline(&resources.display_pipeline);
        render_pass.set_bind_group(0, &target.display_bind_group, &[]);
        render_pass.draw(0..3, 0..1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn camera_view_projection_is_finite_for_landscape_and_portrait_viewports() {
        let camera = OrbitCamera::default();
        for aspect in [16.0 / 9.0, 1.0, 9.0 / 16.0] {
            let matrix = camera.view_projection(aspect).to_cols_array();
            assert!(matrix.iter().all(|value| value.is_finite()));
        }
    }

    #[test]
    fn zoom_is_clamped_to_a_useful_range() {
        let mut camera = OrbitCamera::default();
        camera.zoom(100_000.0);
        assert!(camera.distance >= 1.15);
        camera.zoom(-100_000.0);
        assert!(camera.distance <= 30.0);
    }
}
