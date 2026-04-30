use crate::input::InputEvent;
use crate::niri;
use crate::transcribe::Transcriber;
use eframe::egui;
use std::io::Write;
use std::sync::mpsc::{Receiver, TryRecvError};

// Theme
const BACKGROUND: egui::Color32 = egui::Color32::from_rgb(28, 28, 28);
const PRIMARY: egui::Color32 = egui::Color32::from_rgb(0xC2, 0xFF, 0x50);

enum State {
    Idle,
    Recording,
    Processing,
}

pub struct App {
    transcriber: Transcriber,
    input_rx: Receiver<InputEvent>,
    state: State,
    text: String,
}

impl App {
    pub fn new(transcriber: Transcriber, input_rx: Receiver<InputEvent>) -> Self {
        Self {
            transcriber,
            input_rx,
            state: State::Idle,
            text: String::new(),
        }
    }
}

impl eframe::App for App {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0; 4]
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // Process input events
        loop {
            match self.input_rx.try_recv() {
                Ok(InputEvent::KeyDown) if !matches!(self.state, State::Recording) => {
                    self.state = State::Recording;
                    self.text.clear();
                    niri::show_window();
                }
                Ok(InputEvent::KeyUp) if matches!(self.state, State::Recording) => {
                    self.state = State::Processing;
                }
                Ok(_) => {}
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => break,
            }
        }

        // Always poll — audio runs continuously
        match self.transcriber.poll() {
            Ok(text) if !text.is_empty() => {
                self.text.push_str(&text);
                print!("{}", text);
                let _ = std::io::stdout().flush();
            }
            Err(e) => eprintln!("Transcription error: {e}"),
            _ => {}
        }

        // If processing and no more pending audio, hide
        if matches!(self.state, State::Processing) {
            if !self.transcriber.has_pending() {
                self.state = State::Idle;
                niri::hide_window();
            }
        }

        ui.ctx().request_repaint();

        // Only draw when not idle
        if matches!(self.state, State::Idle) {
            return;
        }

        // Draw pill
        let levels = self.transcriber.levels();
        egui::CentralPanel::default()
            .frame(egui::Frame::new().inner_margin(egui::Margin::ZERO))
            .show_inside(ui, |ui| {
                let rect = ui.available_rect_before_wrap();
                let rounding = rect.height() / 2.0;
                ui.painter().rect_filled(rect, rounding, BACKGROUND);

                if matches!(self.state, State::Recording) {
                    draw_waveform(ui, &rect, &levels, PRIMARY);
                } else {
                    draw_processing(ui, &rect);
                }
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
