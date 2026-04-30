use niri_ipc::socket::Socket;
use niri_ipc::{Action, PositionChange, Request};

const WINDOW_TITLE: &str = "Herald";
const WINDOW_W: f64 = 100.0;
const WINDOW_H: f64 = 50.0;
const BOTTOM_MARGIN: f64 = 80.0;

fn find_window_id() -> Option<u64> {
    let mut socket = Socket::connect().ok()?;
    let reply = socket.send(Request::Windows).ok()?;
    if let niri_ipc::Response::Windows(windows) = reply.ok()? {
        for w in &windows {
            if w.title.as_deref() == Some(WINDOW_TITLE) {
                return Some(w.id);
            }
        }
    }
    None
}

fn focused_output() -> Option<niri_ipc::Output> {
    let mut socket = Socket::connect().ok()?;
    let reply = socket.send(Request::FocusedOutput).ok()?;
    if let niri_ipc::Response::FocusedOutput(Some(output)) = reply.ok()? {
        return Some(output);
    }
    None
}

pub fn hide_window() {
    if let Some(id) = find_window_id() {
        if let Ok(mut socket) = Socket::connect() {
            let _ = socket.send(Request::Action(Action::MoveFloatingWindow {
                id: Some(id),
                x: PositionChange::SetFixed(-9999.0),
                y: PositionChange::SetFixed(-9999.0),
            }));
        }
    }
}

pub fn show_window() {
    let Some(id) = find_window_id() else { return };
    let Some(output) = focused_output() else {
        return;
    };

    let Some(ref logical) = output.logical else {
        return;
    };
    let x = logical.x as f64 + (logical.width as f64 - WINDOW_W) / 2.0;
    let y = logical.y as f64 + logical.height as f64 - WINDOW_H - BOTTOM_MARGIN;

    if let Ok(mut socket) = Socket::connect() {
        let _ = socket.send(Request::Action(Action::MoveFloatingWindow {
            id: Some(id),
            x: PositionChange::SetFixed(x),
            y: PositionChange::SetFixed(y),
        }));
    }
}
