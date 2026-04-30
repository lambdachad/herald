use evdev::{Device, EventType, KeyCode};
use std::sync::mpsc::{self, Receiver};
use std::thread;

#[derive(Debug)]
pub enum InputEvent {
    KeyDown,
    KeyUp,
}

/// Watch for Super key press/release on all keyboards.
pub fn spawn_key_watcher() -> Receiver<InputEvent> {
    let (tx, rx) = mpsc::channel();

    let keyboards: Vec<Device> = evdev::enumerate()
        .filter(|(_, dev)| dev.supported_events().contains(EventType::KEY))
        .filter(|(_, dev)| {
            dev.supported_keys().map_or(false, |keys| {
                keys.contains(KeyCode::KEY_LEFTMETA) || keys.contains(KeyCode::KEY_RIGHTMETA)
            })
        })
        .map(|(_, dev)| dev)
        .collect();

    if keyboards.is_empty() {
        eprintln!("No keyboard with Super key found");
        return rx;
    }

    eprintln!("Watching {} keyboard(s) for Super key", keyboards.len());

    for mut dev in keyboards {
        let tx = tx.clone();
        thread::spawn(move || loop {
            for ev in dev.fetch_events().unwrap() {
                if ev.event_type() == EventType::KEY
                    && (ev.code() == KeyCode::KEY_LEFTMETA.0
                        || ev.code() == KeyCode::KEY_RIGHTMETA.0)
                {
                    let event = match ev.value() {
                        1 => InputEvent::KeyDown,
                        0 => InputEvent::KeyUp,
                        _ => continue,
                    };
                    let _ = tx.send(event);
                }
            }
        });
    }

    rx
}
