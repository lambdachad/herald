// Modules
pub mod input;
pub mod transcribe;

// Imports
use eframe::egui;
use std::{process::Command, sync::mpsc::Receiver};
use transcribe::Transcriber;

// Theme
const BACKGROUND: egui::Color32 = egui::Color32::from_rgb(28, 28, 28);
const PRIMARY: egui::Color32 = egui::Color32::from_rgb(0xC2, 0xFF, 0x50);

#[derive(PartialEq)]
enum State {
    Hidden,
    Recording,
    Processing,
}

pub struct App {
    state: State,
    input: Receiver<bool>,
    transcriber: Transcriber,
    transcribed_text: String,
}

impl App {
    pub fn new(transcriber: Transcriber, input: Receiver<bool>) -> Self {
        Self {
            transcriber,
            input,
            state: State::Hidden,
            transcribed_text: String::new(),
        }
    }
}

impl eframe::App for App {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0; 4]
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // Process input events from devices
        loop {
            match self.input.try_recv() {
                Ok(true) if self.state != State::Recording => {
                    self.state = State::Recording;
                }
                Ok(false) if self.state == State::Recording => {
                    self.state = State::Processing;
                }
                Ok(_) => {}
                Err(_) => break,
            }
        }

        // Poll audio device
        if let Ok(text) = self.transcriber.poll() {
            if !text.is_empty() {
                self.transcribed_text.push_str(&text);
            }
        }

        // Update based on state
        ui.ctx().request_repaint();
        match self.state {
            State::Hidden => self.transcriber.drain(),
            State::Recording => {
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
            State::Processing => {
                let rect = ui.available_rect_before_wrap();
                let rounding = rect.height() / 2.0;
                ui.painter().rect_filled(rect, rounding, BACKGROUND);
                draw_processing(ui, &rect);
                if !self.transcriber.has_pending() {
                    let trimmed = self.transcribed_text.trim();
                    if !trimmed.is_empty() {
                        let _ = Command::new("wl-copy").arg(&trimmed).spawn();
                        let _ = Command::new("wtype").args(["-M", "ctrl", "-k", "v", "-m", "ctrl"]).spawn();
                        self.transcribed_text.clear();
                    }
                    self.state = State::Hidden;
                }
            }
        }
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

fn draw_processing(ui: &mut egui::Ui, rect: &egui::Rect) {
    let dot_radius = 3.0;
    let dot_gap = 8.0;
    let total_w = 3.0 * dot_radius * 2.0 + 2.0 * dot_gap;
    let start_x = rect.center().x - total_w / 2.0;
    let center_y = rect.center().y;

    for i in 0..3 {
        let x = start_x + i as f32 * (dot_radius * 2.0 + dot_gap) + dot_radius;
        let pulse = ((ui.ctx().input(|i| i.time) * 3.0 + i as f64 * 1.2).sin() + 1.0) / 2.0;
        let alpha = (0.3 + pulse * 0.7) * 255.0;
        let color = egui::Color32::from_rgba_unmultiplied(0xC2, 0xFF, 0x50, alpha as u8);
        ui.painter()
            .circle_filled(egui::pos2(x, center_y), dot_radius, color);
    }
}
