mod app;
mod material;
mod render;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        renderer: eframe::Renderer::Wgpu,
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("Skin Studio RS")
            .with_inner_size([1440.0, 900.0])
            .with_min_inner_size([1080.0, 680.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Skin Studio RS",
        options,
        Box::new(|cc| Ok(Box::new(app::SkinStudioApp::new(cc)))),
    )
}
