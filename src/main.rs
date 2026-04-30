use eframe::egui;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([180.0, 60.0])
            .with_resizable(false)
            .with_transparent(true)
            .with_app_id("herald")
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
        let pill_frame = egui::Frame::new()
            .inner_margin(egui::Margin::ZERO);
        
        egui::CentralPanel::default()
            .frame(pill_frame)
            .show_inside(ui, |ui| {
                let rect = ui.available_rect_before_wrap();
                let rounding = rect.height() / 2.0;
                ui.painter().rect_filled(
                    rect,
                    rounding,
                    egui::Color32::from_rgba_unmultiplied(20, 20, 20, 255),
                );
            });
    }
}
