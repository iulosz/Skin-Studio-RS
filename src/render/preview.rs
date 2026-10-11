use std::num::NonZeroU64;

use bytemuck::{Pod, Zeroable};
use eframe::egui;
use eframe::egui_wgpu::{self, wgpu};

use crate::material::SkinMaterial;

const SHADER: &str = include_str!("skin_preview.wgsl");

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct PreviewUniforms {
    // viewport width/height in physical pixels, then viewport origin in physical pixels
    viewport: [f32; 4],
    // linear-ish RGB base colour + pore strength
    color_pores: [f32; 4],
    // roughness, specular, subsurface approximation, micro-relief
    surface: [f32; 4],
    // hemoglobin tint, freckle strength, oiliness and pigment variation
    pigment_detail: [f32; 4],
}

struct SkinRenderResources {
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    uniform_buffer: wgpu::Buffer,
}

impl SkinRenderResources {
    fn paint(&self, render_pass: &mut wgpu::RenderPass<'_>) {
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.bind_group, &[]);
        render_pass.draw(0..3, 0..1);
    }
}

/// Initial custom GPU renderer for the material look-development sphere.
/// Geometry is ray-intersected in WGSL for this first milestone; the next stage
/// replaces this preview primitive with imported triangle meshes and a PBR pipeline.
pub struct SkinPreview {
    available: bool,
}

impl SkinPreview {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let Some(state) = cc.wgpu_render_state.as_ref() else {
            return Self { available: false };
        };

        let device = &state.device;
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Skin Studio procedural preview shader"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Skin Studio preview uniforms layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: NonZeroU64::new(std::mem::size_of::<PreviewUniforms>() as u64),
                },
                count: None,
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Skin Studio preview pipeline layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Skin Studio procedural material preview pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
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

        let initial = PreviewUniforms::zeroed();
        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Skin Studio preview uniforms"),
            size: std::mem::size_of::<PreviewUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        state.queue.write_buffer(&uniform_buffer, 0, bytemuck::bytes_of(&initial));

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Skin Studio preview bind group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        state
            .renderer
            .write()
            .callback_resources
            .insert(SkinRenderResources {
                pipeline,
                bind_group,
                uniform_buffer,
            });

        Self { available: true }
    }

    pub fn show(&self, ui: &mut egui::Ui, params: SkinMaterial) {
        if !self.available {
            ui.centered_and_justified(|ui| {
                ui.label("O backend WGPU não foi inicializado. Verifique o suporte gráfico do sistema.");
            });
            return;
        }

        let desired_size = ui.available_size().max(egui::vec2(320.0, 320.0));
        let (rect, _response) = ui.allocate_exact_size(desired_size, egui::Sense::hover());
        let pixels_per_point = ui.ctx().pixels_per_point();
        let uniform = PreviewUniforms {
            viewport: [
                rect.width() * pixels_per_point,
                rect.height() * pixels_per_point,
                rect.min.x * pixels_per_point,
                rect.min.y * pixels_per_point,
            ],
            color_pores: [
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
            SkinPreviewCallback { uniform },
        ));
    }
}

struct SkinPreviewCallback {
    uniform: PreviewUniforms,
}

impl egui_wgpu::CallbackTrait for SkinPreviewCallback {
    fn prepare(
        &self,
        _device: &wgpu::Device,
        queue: &wgpu::Queue,
        _screen_descriptor: &egui_wgpu::ScreenDescriptor,
        _encoder: &mut wgpu::CommandEncoder,
        callback_resources: &mut egui_wgpu::CallbackResources,
    ) -> Vec<wgpu::CommandBuffer> {
        let resources: &SkinRenderResources = callback_resources.get().expect(
            "Skin Studio renderer resources were not initialized",
        );
        queue.write_buffer(
            &resources.uniform_buffer,
            0,
            bytemuck::bytes_of(&self.uniform),
        );
        Vec::new()
    }

    fn paint(
        &self,
        _info: egui::PaintCallbackInfo,
        render_pass: &mut wgpu::RenderPass<'static>,
        callback_resources: &egui_wgpu::CallbackResources,
    ) {
        let resources: &SkinRenderResources = callback_resources.get().expect(
            "Skin Studio renderer resources were not initialized",
        );
        resources.paint(render_pass);
    }
}
