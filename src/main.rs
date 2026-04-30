use eframe::egui;
use herald::{app::App, transcribe::Transcriber, Result};

fn main() -> Result<()> {
    // Load model and start audio capture
    let mut transcriber = Transcriber::new()?;
    transcriber.start_capture()?;

    // Window options
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([100.0, 50.0])
            .with_resizable(false)
            .with_transparent(true)
            .with_app_id("herald")
            .with_always_on_top(),
        ..Default::default()
    };

    // Start eGUI app
    Ok(eframe::run_native(
        "Herald",
        options,
        Box::new(move |_cc| Ok(Box::new(App::new(transcriber)))),
    )?)
}
