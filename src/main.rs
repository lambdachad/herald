use eframe::egui;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([400.0, 80.0])
            .with_decorations(false)
            .with_transparent(true)
            .with_always_on_top(),
        ..Default::default()
    };

    eframe::run_native(
        "Herald",
        options,
        Box::new(|_cc| Ok(Box::new(App {}))),
    )
}

struct App {}

impl eframe::App for App {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0; 4]
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // Transparent panel background
        let panel_frame = egui::Frame::new()
            .fill(egui::Color32::TRANSPARENT)
            .inner_margin(egui::Margin::ZERO);

        egui::CentralPanel::default()
            .frame(panel_frame)
            .show_inside(ui, |ui| {
                let rect = ui.available_rect_before_wrap();
                let rounding = rect.height() / 2.0;
                ui.painter().rect_filled(
                    rect,
                    rounding,
                    egui::Color32::from_rgba_unmultiplied(20, 20, 20, 230),
                );
            });
    }
}
