use crate::transcribe::Transcriber;
use eframe::egui;
use std::io::Write;

// Theme
const BACKGROUND: egui::Color32 = egui::Color32::from_rgb(28, 28, 28);
const PRIMARY: egui::Color32 = egui::Color32::from_rgb(0xC2, 0xFF, 0x50);

pub struct App {
    transcriber: Transcriber,
}

impl App {
    pub fn new(transcriber: Transcriber) -> Self {
        Self { transcriber }
    }
}

impl eframe::App for App {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0; 4]
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // Poll transcriber and print to terminal
        match self.transcriber.poll() {
            Ok(text) if !text.is_empty() => {
                print!("{text}");
                let _ = std::io::stdout().flush();
            }
            Err(e) => eprintln!("Transcription error: {e}"),
            _ => {}
        }
        ui.ctx().request_repaint();

        // Draw pill with waveform
        let levels = self.transcriber.levels();
        egui::CentralPanel::default()
            .frame(egui::Frame::new().inner_margin(egui::Margin::ZERO))
            .show_inside(ui, |ui| {
                let rect = ui.available_rect_before_wrap();
                let rounding = rect.height() / 2.0;
                ui.painter().rect_filled(rect, rounding, BACKGROUND);
                draw_waveform(ui, &rect, &levels, PRIMARY);
            });
    }
}

fn draw_waveform(ui: &mut egui::Ui, rect: &egui::Rect, levels: &[f32], color: egui::Color32) {
    let num_bars = levels.len();
    let bar_w = 3.0;
    let bar_gap = 4.0;
    let total_w = num_bars as f32 * bar_w + (num_bars - 1) as f32 * bar_gap;
    let padding = (rect.width() - total_w) / 2.0;
    let max_h = rect.height() * 0.7;
    let center_y = rect.center().y;

    for (i, &level) in levels.iter().enumerate() {
        let amp = (level * 8.0).clamp(0.02, 1.0);
        let h = max_h * amp;
        let x = rect.left() + padding + i as f32 * (bar_w + bar_gap);
        let bar_rect = egui::Rect::from_center_size(
            egui::pos2(x + bar_w / 2.0, center_y),
            egui::vec2(bar_w, h),
        );
        ui.painter().rect_filled(bar_rect, bar_w / 2.0, color);
    }
}
