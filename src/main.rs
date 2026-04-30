use eframe::egui;
use herald::{input, transcribe::Transcriber, App, Result};

fn main() -> Result<()> {
    // Setup
    let transcriber = Transcriber::new()?;
    let input = input::listen_keys();

    // Start eGUI app
    Ok(eframe::run_native(
        "Herald",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([100.0, 50.0])
                .with_resizable(false)
                .with_transparent(true)
                .with_app_id("herald")
                .with_always_on_top(),
            ..Default::default()
        },
        Box::new(move |_cc| Ok(Box::new(App::new(transcriber, input)))),
    )?)
}
