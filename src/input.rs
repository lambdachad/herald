use evdev::{EventType, KeyCode};
use std::sync::mpsc::{self, Receiver};
use std::thread;

pub fn listen_keys() -> Receiver<bool> {
    // Start channel
    let (tx, rx) = mpsc::channel();
    let keyboards = evdev::enumerate()
        .filter(|(_, dev)| dev.supported_events().contains(EventType::KEY))
        .filter(|(_, dev)| {
            dev.supported_keys().map_or(false, |keys| {
                keys.contains(KeyCode::KEY_LEFTMETA) || keys.contains(KeyCode::KEY_RIGHTMETA)
            })
        })
        .map(|(_, dev)| dev)
        .collect::<Vec<_>>();

    // Create thread for each input device
    for mut dev in keyboards {
        let tx = tx.clone();
        thread::spawn(move || loop {
            for event in dev.fetch_events().unwrap() {
                if event.event_type() == EventType::KEY
                    && (event.code() == KeyCode::KEY_LEFTMETA.0
                        || event.code() == KeyCode::KEY_RIGHTMETA.0)
                {
                    let _ = tx.send(event.value() == 1);
                }
            }
        });
    }
    rx
}
