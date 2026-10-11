use std::time::Duration;

use eframe::egui;
use crate::{
    material::{CoordinateUnit, SkinMaterial, SkinPreset, load_preset, save_preset},
    reference_height::ReferenceHeightSettings,
    render::SkinPreview,
    skin_detail::PoreMapSettings,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum StudioTab {
    Setup,
    Color,
    Detail,
    Reference,
    Surface,
    Hair,
    Bake,
}

impl StudioTab {
    const ALL: [Self; 7] = [
        Self::Setup,
        Self::Color,
        Self::Detail,
        Self::Reference,
        Self::Surface,
        Self::Hair,
        Self::Bake,
    ];

    fn title(self) -> &'static str {
        match self {
            Self::Setup => "Setup",
            Self::Color => "Color",
            Self::Detail => "Detail",
            Self::Reference => "Reference",
            Self::Surface => "Surface",
            Self::Hair => "Hair",
            Self::Bake => "Bake",
        }
    }
}

pub struct SkinStudioApp {
    active_tab: StudioTab,
    preview: SkinPreview,
    params: SkinMaterial,
    show_grid: bool,
    status: String,
    preset_path: String,
    preset_name: String,
}

impl SkinStudioApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        Self {
            active_tab: StudioTab::Color,
            preview: SkinPreview::new(cc),
            params: SkinMaterial::default(),
            show_grid: true,
            status: "Ready · GPU material preview".to_owned(),
            preset_path: "skin_studio_preset.json".to_owned(),
            preset_name: "My Skin".to_owned(),
        }
    }

    fn save_current_preset(&mut self) {
        let preset = match SkinPreset::new(self.preset_name.trim(), self.params) {
            Ok(preset) => preset,
            Err(error) => {
                self.status = format!("Preset not saved: {error}");
                return;
            }
        };
        match save_preset(&self.preset_path, &preset) {
            Ok(()) => self.status = format!("Preset saved: {}", self.preset_path),
            Err(error) => self.status = format!("Preset save failed: {error}"),
        }
    }

    fn load_current_preset(&mut self) {
        match load_preset(&self.preset_path) {
            Ok(preset) => {
                self.params = preset.material;
                self.preset_name = preset.name;
                self.regenerate_pore_map();
                self.start_reference_map_generation();
                self.status = format!("Preset loaded: {} · height maps regenerated", self.preset_path);
            }
            Err(error) => self.status = format!("Preset load failed: {error}"),
        }
    }

    fn regenerate_pore_map(&mut self) {
        let settings = PoreMapSettings {
            resolution: self.params.pore_map_resolution,
            density: self.params.pore_map_density,
            relief: self.params.pore_map_relief,
            seed: self.params.pore_seed,
        };
        match self.preview.regenerate_pore_map(settings) {
            Ok(()) => {
                self.status = format!(
                    "Pore height map regenerated · {} px · density {:.1} · seed {}",
                    settings.resolution, settings.density, settings.seed
                );
            }
            Err(error) => {
                self.status = format!("Pore height map failed: {error}");
            }
        }
    }

    fn start_reference_map_generation(&mut self) {
        let settings = ReferenceHeightSettings {
            resolution: self.params.reference_resolution,
            seed: self.params.reference_seed,
            pores: self.params.reference_pores,
        };
        self.preview.request_reference_map_generation(settings);
        self.status = format!(
            "Generating reference skin height · {} px · seed {}",
            settings.resolution, settings.seed
        );
    }

    fn import_mesh(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("3D meshes", &["obj", "gltf", "glb"])
            .pick_file()
        else {
            return;
        };

        match crate::geometry::load_mesh(&path) {
            Ok(mesh) => {
                let name = mesh.name.clone();
                let vertices = mesh.vertex_count();
                let triangles = mesh.triangle_count();
                self.preview.set_mesh(mesh);
                self.status = format!("Imported {name} · {vertices} vertices · {triangles} triangles");
            }
            Err(error) => {
                self.status = format!("Mesh import failed: {error}");
            }
        }
    }

    fn toolbar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading("SKIN STUDIO");
            ui.label(egui::RichText::new("RS  ·  0.1.0").weak());
            ui.separator();
            if ui.button("Reset material").clicked() {
                self.params = SkinMaterial::default();
                self.regenerate_pore_map();
                self.status = "Material values restored to defaults".to_owned();
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(egui::RichText::new("CUSTOM WGPU PREVIEW").small().weak());
            });
        });
        ui.horizontal(|ui| {
            ui.label("Preset name");
            ui.add(egui::TextEdit::singleline(&mut self.preset_name).desired_width(130.0));
            ui.label("JSON file");
            ui.add(egui::TextEdit::singleline(&mut self.preset_path).desired_width(250.0));
            if ui.button("Save preset").clicked() {
                self.save_current_preset();
            }
            if ui.button("Load preset").clicked() {
                self.load_current_preset();
            }
        });
    }

    fn left_navigation(&mut self, ui: &mut egui::Ui) {
        ui.heading("Workspace");
        ui.add_space(8.0);
        for tab in StudioTab::ALL {
            if ui.selectable_label(self.active_tab == tab, tab.title()).clicked() {
                self.active_tab = tab;
            }
        }
        ui.add_space(18.0);
        ui.separator();
        ui.label(egui::RichText::new("Project").strong());
        ui.label("Skin material");
        ui.label(egui::RichText::new("Unsaved session").weak().small());
        ui.add_space(12.0);
        ui.checkbox(&mut self.show_grid, "Viewport grid");
        ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
            ui.label(egui::RichText::new("Standalone Rust application").weak().small());
            ui.label(egui::RichText::new("Blender is not required").weak().small());
        });
    }

    fn properties(&mut self, ui: &mut egui::Ui) {
        ui.heading(self.active_tab.title());
        ui.add_space(6.0);
        egui::ScrollArea::vertical().show(ui, |ui| match self.active_tab {
            StudioTab::Setup => {
                ui.label(egui::RichText::new("Scene and model").strong());
                ui.separator();
                if ui.button("Import OBJ / glTF / GLB").clicked() {
                    self.import_mesh();
                }
                ui.add_space(8.0);
                ui.label(format!("Active mesh: {}", self.preview.mesh_name()));
                ui.label(format!("Vertices: {}", self.preview.vertex_count()));
                ui.label(format!("Triangles: {}", self.preview.triangle_count()));
                ui.add_space(6.0);
                ui.label("Imported meshes are centered and scaled to fit the preview. Triangle surfaces use a depth buffer.");
            }
            StudioTab::Color => {
                ui.label(egui::RichText::new("Base pigmentation").strong());
                ui.color_edit_button_rgb(&mut self.params.base_color);
                ui.add_space(8.0);
                ui.add(
                    egui::Slider::new(&mut self.params.hemoglobin_strength, 0.0..=1.0)
                        .text("Hemoglobin tint"),
                );
                ui.add(
                    egui::Slider::new(&mut self.params.pigment_variation, 0.0..=1.0)
                        .text("Pigment variation"),
                );
                ui.add_space(6.0);
                ui.label("These controls update the GPU preview immediately. Regional anatomy and calibrated pigment layers are not implemented yet.");
            }
            StudioTab::Detail => {
                ui.label(egui::RichText::new("Skin Line Network").strong());
                ui.add(
                    egui::Slider::new(&mut self.params.skin_line_strength, 0.0..=1.5)
                        .text("Network strength"),
                );
                ui.add(
                    egui::Slider::new(&mut self.params.skin_line_scale, 4.0..=400.0)
                        .text("Cell scale"),
                );
                ui.add(
                    egui::Slider::new(&mut self.params.skin_line_thickness, 0.05..=1.0)
                        .text("Groove thickness"),
                );
                ui.add(
                    egui::Slider::new(&mut self.params.skin_line_relief, 0.1..=8.0)
                        .text("Network relief"),
                );
                ui.add(
                    egui::Slider::new(&mut self.params.skin_line_shading, 0.0..=1.0)
                        .text("Groove shading"),
                );
                ui.label("This is the cellular sulci-cutis network, separate from pore openings and directional crosshatch.");
                ui.separator();
                ui.label(egui::RichText::new("Layered pore system").strong());
                ui.add(
                    egui::Slider::new(&mut self.params.pore_strength, 0.0..=2.0)
                        .text("Pore strength"),
                );
                ui.add(
                    egui::Slider::new(&mut self.params.pore_scale, 0.25..=4.0)
                        .text("Pore pattern scale"),
                );
                ui.add(
                    egui::Slider::new(&mut self.params.pore_color_strength, 0.0..=1.0)
                        .text("Pore color"),
                );
                ui.add(
                    egui::Slider::new(&mut self.params.pore_micro_detail, 0.0..=1.0)
                        .text("Pore micro detail"),
                );
                ui.add(
                    egui::Slider::new(&mut self.params.pore_micro_scale, 40.0..=320.0)
                        .text("Pore micro scale"),
                );
                ui.add(
                    egui::Slider::new(&mut self.params.pore_irregularity, 0.0..=1.0)
                        .text("Pore irregularity"),
                );
                ui.add(
                    egui::Slider::new(&mut self.params.pore_size_variation, 0.0..=1.0)
                        .text("Pore size variation"),
                );

                ui.separator();
                ui.label(egui::RichText::new("Tileable pore height map").strong());
                ui.add(
                    egui::Slider::new(&mut self.params.pore_map_resolution, 256..=4096)
                        .text("Resolution"),
                );
                ui.add(
                    egui::Slider::new(&mut self.params.pore_map_density, 5.0..=100.0)
                        .text("Density"),
                );
                ui.add(
                    egui::Slider::new(&mut self.params.pore_map_relief, 0.5..=10.0)
                        .text("Height-map relief"),
                );
                ui.add(
                    egui::Slider::new(&mut self.params.pore_seed, 0..=999_999)
                        .text("Seed"),
                );
                if ui.button("Generate Pore Map").clicked() {
                    self.regenerate_pore_map();
                }

                ui.separator();
                ui.label(egui::RichText::new("Fine surface detail").strong());
                ui.add(
                    egui::Slider::new(&mut self.params.fine_detail_strength, 0.0..=0.5)
                        .text("Fine detail strength"),
                );
                ui.add(
                    egui::Slider::new(&mut self.params.fine_detail_scale, 10.0..=500.0)
                        .text("Fine detail scale"),
                );
                ui.add(
                    egui::Slider::new(&mut self.params.micro_grain_strength, 0.0..=0.20)
                        .text("Micro grain strength"),
                );
                ui.add(
                    egui::Slider::new(&mut self.params.micro_grain_scale, 50.0..=1000.0)
                        .text("Micro grain scale"),
                );
                ui.add(
                    egui::Slider::new(&mut self.params.micro_grain_roughness, 0.0..=0.15)
                        .text("Micro grain roughness"),
                );

                ui.collapsing("Regional pore distribution", |ui| {
                    ui.add(
                        egui::Slider::new(&mut self.params.regional_relief, 0.0..=1.0)
                            .text("Regional relief"),
                    );
                    ui.add(
                        egui::Slider::new(&mut self.params.pore_vis_nose, 0.0..=2.0)
                            .text("Nose"),
                    );
                    ui.add(
                        egui::Slider::new(&mut self.params.pore_vis_cheeks, 0.0..=2.0)
                            .text("Cheeks"),
                    );
                    ui.add(
                        egui::Slider::new(&mut self.params.pore_vis_forehead, 0.0..=2.0)
                            .text("Forehead"),
                    );
                    ui.add(
                        egui::Slider::new(&mut self.params.pore_vis_chin, 0.0..=2.0)
                            .text("Chin"),
                    );
                    ui.add(
                        egui::Slider::new(&mut self.params.pore_vis_ears, 0.0..=2.0)
                            .text("Ears"),
                    );
                    ui.add(
                        egui::Slider::new(&mut self.params.pore_vis_eye_area, 0.0..=2.0)
                            .text("Eye area"),
                    );
                    ui.add(
                        egui::Slider::new(&mut self.params.pore_vis_neck, 0.0..=2.0)
                            .text("Neck"),
                    );
                    ui.label(
                        egui::RichText::new(
                            "Approximate masks use bbox coordinates and assume a centered head facing -Y. They are not a substitute for painted anatomical masks.",
                        )
                        .weak()
                        .small(),
                    );
                });
                ui.separator();
                ui.label(egui::RichText::new("Real-world scale").strong());
                egui::ComboBox::from_label("Mesh units")
                    .selected_text(self.params.coord_unit.label())
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut self.params.coord_unit,
                            CoordinateUnit::Meter,
                            "Meters",
                        );
                        ui.selectable_value(
                            &mut self.params.coord_unit,
                            CoordinateUnit::Centimeter,
                            "Centimeters",
                        );
                        ui.selectable_value(
                            &mut self.params.coord_unit,
                            CoordinateUnit::Millimeter,
                            "Millimeters",
                        );
                        ui.selectable_value(
                            &mut self.params.coord_unit,
                            CoordinateUnit::Custom,
                            "Custom",
                        );
                    });
                if self.params.coord_unit == CoordinateUnit::Custom {
                    ui.add(
                        egui::Slider::new(
                            &mut self.params.coord_custom_meters_per_unit,
                            0.00001..=1000.0,
                        )
                        .logarithmic(true)
                        .text("Meters per unit"),
                    );
                }
                ui.label("Pore size is evaluated in physical coordinates and does not follow the viewport's fit-to-frame scale.");
                ui.label("Ported from Skin Studio 2.4.76: three Voronoi pore populations, domain warp, generated tileable height, separate fine bump and micro grain.");
            }
            StudioTab::Reference => {
                ui.label(egui::RichText::new("Reference skin-height map").strong());
                ui.label("Skin style · tileable cellular network, shallow furrows, junction pores and filtered micro-grain.");
                ui.add(
                    egui::Slider::new(&mut self.params.reference_strength, 0.0..=1.0)
                        .text("Reference relief strength"),
                );
                ui.add(
                    egui::Slider::new(&mut self.params.reference_relief, 0.1..=8.0)
                        .text("Reference relief"),
                );
                ui.add(
                    egui::Slider::new(&mut self.params.reference_cell_mm, 0.02..=200.0)
                        .logarithmic(true)
                        .text("Reference cell (mm)"),
                );
                ui.add(
                    egui::Slider::new(&mut self.params.reference_pores, 0.0..=3.0)
                        .text("Junction pores"),
                );
                ui.add(
                    egui::Slider::new(&mut self.params.reference_resolution, 256..=4096)
                        .text("Map resolution"),
                );
                ui.add(
                    egui::Slider::new(&mut self.params.reference_seed, 0..=999_999)
                        .text("Map seed"),
                );
                ui.add(
                    egui::Slider::new(&mut self.params.reference_color, 0.0..=1.0)
                        .text("Cavity color"),
                );
                ui.add(
                    egui::Slider::new(&mut self.params.reference_rough, 0.0..=0.6)
                        .text("Cavity roughness"),
                );
                ui.add(
                    egui::Slider::new(&mut self.params.reference_regional, 0.0..=1.0)
                        .text("Regional influence"),
                );
                if self.preview.reference_map_generation_pending() {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label("Generating tileable height map in background…");
                    });
                } else if ui.button("Generate Reference Height").clicked() {
                    self.start_reference_map_generation();
                }
                ui.label("The original LEATHER style and its independent calibration have not been ported yet; this generator implements the SKIN style.");
                ui.label("This map is a separate bump layer; it does not replace the small/medium/large pore populations.");
            }
            StudioTab::Surface => {
                ui.label(egui::RichText::new("Surface response").strong());
                ui.add(
                    egui::Slider::new(&mut self.params.roughness, 0.03..=1.0)
                        .text("Roughness"),
                );
                ui.add(
                    egui::Slider::new(&mut self.params.specular, 0.0..=1.0)
                        .text("Specular"),
                );
                ui.add(
                    egui::Slider::new(&mut self.params.oiliness, 0.0..=1.0)
                        .text("Oiliness"),
                );
                ui.add(
                    egui::Slider::new(&mut self.params.spec_pore_occlusion, 0.0..=1.0)
                        .text("Pore specular occlusion"),
                );
                ui.add(
                    egui::Slider::new(&mut self.params.subsurface, 0.0..=0.7)
                        .text("Subsurface (preview)"),
                );
                ui.label("Oiliness adjusts highlight strength and effective roughness. Subsurface remains an inexpensive visual approximation.");
            }
            StudioTab::Hair => {
                ui.label("Scalp hair and facial vellus");
                ui.separator();
                ui.label("The original addon blends scalp/facial-hair detail into its Base Color bake. Hair masks and strand rendering remain future work.");
            }
            StudioTab::Bake => {
                ui.label("Texture baking");
                ui.separator();
                ui.label("Planned maps: Base Color, Tangent Normal and Roughness. Export is not implemented in this prototype.");
                ui.label("The independent baker will need UV support, padding, color-space handling and repeatable tests.");
            }
        });
    }
}

impl eframe::App for SkinStudioApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        if let Some(result) = self.preview.poll_reference_map_generation() {
            self.status = match result {
                Ok(()) => "Reference height map generated and uploaded on next frame".to_owned(),
                Err(error) => format!("Reference height generation failed: {error}"),
            };
        }
        if self.preview.reference_map_generation_pending() {
            ui.ctx().request_repaint_after(Duration::from_millis(33));
        }
        egui::Frame::central_panel(ui.style()).show(ui, |ui| {
            self.toolbar(ui);
            ui.separator();

            let remaining = ui.available_size();
            let left_width = 158.0_f32.min(remaining.x * 0.22);
            let right_width = 280.0_f32.min(remaining.x * 0.34);
            let middle_width = (remaining.x - left_width - right_width - 28.0).max(250.0);
            let main_height = remaining.y;

            ui.horizontal(|ui| {
                ui.allocate_ui_with_layout(
                    egui::vec2(left_width, main_height),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| self.left_navigation(ui),
                );
                ui.separator();
                ui.allocate_ui_with_layout(
                    egui::vec2(middle_width, main_height),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        ui.horizontal(|ui| {
                            ui.heading("3D Viewport");
                            ui.label(
                                egui::RichText::new(self.preview.mesh_name())
                                    .weak()
                                    .small(),
                            );
                            if ui.button("Reset view").clicked() {
                                self.preview.reset_camera();
                            }
                        });
                        ui.add_space(6.0);
                        egui::Frame::group(ui.style()).show(ui, |ui| {
                            let size = ui.available_size();
                            ui.allocate_ui_with_layout(
                                size,
                                egui::Layout::top_down(egui::Align::Min),
                                |ui| self.preview.show(ui, self.params, self.show_grid),
                            );
                        });
                        ui.add_space(6.0);
                        ui.label(egui::RichText::new("Drag: orbit · Shift+drag: pan · Wheel: zoom").weak().small());
                        ui.label(egui::RichText::new(&self.status).weak().small());
                    },
                );
                ui.separator();
                ui.allocate_ui_with_layout(
                    egui::vec2(right_width, main_height),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| self.properties(ui),
                );
            });
        });
    }
}
