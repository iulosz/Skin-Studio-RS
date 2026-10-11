use eframe::egui;
use crate::{
    material::{SkinMaterial, SkinPreset, load_preset, save_preset},
    render::SkinPreview,
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
                self.status = format!("Preset loaded: {}", self.preset_path);
            }
            Err(error) => self.status = format!("Preset load failed: {error}"),
        }
    }

    fn toolbar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading("SKIN STUDIO");
            ui.label(egui::RichText::new("RS  ·  0.1.0").weak());
            ui.separator();
            if ui.button("Reset material").clicked() {
                self.params = SkinMaterial::default();
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
                ui.label("Scene and model controls");
                ui.separator();
                ui.label("Mesh import and real-world scale will be added with the geometry system.");
                ui.label("Initial preview primitive: procedural sphere.");
            }
            StudioTab::Color => {
                ui.label("Base pigmentation");
                ui.color_edit_button_rgb(&mut self.params.base_color);
                ui.add_space(8.0);
                ui.label("RGB channels can be tuned through the color picker.");
                ui.label("Regional masks and layered pigmentation are part of the porting roadmap.");
            }
            StudioTab::Detail => {
                ui.add(egui::Slider::new(&mut self.params.pore_strength, 0.0..=1.0).text("Pore intensity"));
                ui.add(egui::Slider::new(&mut self.params.micro_relief, 0.0..=1.0).text("Micro-relief"));
                ui.label("Current preview uses procedural GPU noise. Anatomical regional masks are not ported yet.");
            }
            StudioTab::Reference => {
                ui.label("Reference-driven Skin Line Network");
                ui.separator();
                ui.label("The original reference-height generator and its calibration will be reimplemented as a separate Rust module.");
            }
            StudioTab::Surface => {
                ui.add(egui::Slider::new(&mut self.params.roughness, 0.03..=1.0).text("Roughness"));
                ui.add(egui::Slider::new(&mut self.params.specular, 0.0..=1.0).text("Specular"));
                ui.add(egui::Slider::new(&mut self.params.subsurface, 0.0..=0.7).text("Subsurface (preview)"));
                ui.label("SSS is currently an inexpensive screen-space approximation, not production-quality diffusion.");
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
                            ui.heading("Material Preview");
                            ui.label(egui::RichText::new("sphere · procedural").weak().small());
                        });
                        ui.add_space(6.0);
                        egui::Frame::group(ui.style()).show(ui, |ui| {
                            let size = ui.available_size();
                            ui.allocate_ui_with_layout(
                                size,
                                egui::Layout::top_down(egui::Align::Min),
                                |ui| self.preview.show(ui, self.params),
                            );
                        });
                        ui.add_space(6.0);
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
