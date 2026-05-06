use crate::{constants::files, utils::dirs};
use clash_verge_logging::{Type, logging};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::WebviewWindow;

#[derive(Serialize, Deserialize, Default, Debug)]
struct WindowStateData {
    x: Option<i32>,
    y: Option<i32>,
    width: Option<u32>,
    height: Option<u32>,
}

fn window_state_path() -> Option<PathBuf> {
    dirs::app_home_dir().ok().map(|d| d.join(files::WINDOW_STATE))
}

fn window_rect_intersects_any_monitor(window: &WebviewWindow, x: i32, y: i32, width: u32, height: u32) -> bool {
    let Ok(monitors) = window.available_monitors() else {
        return true;
    };
    if monitors.is_empty() {
        return true;
    }
    let win_right = x.saturating_add(width as i32);
    let win_bottom = y.saturating_add(height as i32);
    for m in monitors {
        let pos = m.position();
        let size = m.size();
        let mx2 = pos.x.saturating_add(size.width as i32);
        let my2 = pos.y.saturating_add(size.height as i32);
        if x < mx2 && win_right > pos.x && y < my2 && win_bottom > pos.y {
            return true;
        }
    }
    false
}

pub fn save_window_state(window: &WebviewWindow) {
    let Some(path) = window_state_path() else {
        logging!(warn, Type::Window, "Cannot determine window state path, skipping save");
        return;
    };

    let data = WindowStateData {
        x: window.outer_position().ok().map(|p| p.x),
        y: window.outer_position().ok().map(|p| p.y),
        width: window.inner_size().ok().map(|s| s.width),
        height: window.inner_size().ok().map(|s| s.height),
    };

    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    match serde_json::to_string_pretty(&data) {
        Ok(json) => {
            if let Err(e) = std::fs::write(&path, json) {
                logging!(warn, Type::Window, "Failed to save window state: {e}");
            } else {
                logging!(debug, Type::Window, "Window state saved to {path:?}");
            }
        }
        Err(e) => logging!(warn, Type::Window, "Failed to serialize window state: {e}"),
    }
}

pub fn restore_window_state(window: &WebviewWindow) {
    let Some(path) = window_state_path() else { return };

    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return,
    };

    let data: WindowStateData = match serde_json::from_str(&content) {
        Ok(d) => d,
        Err(e) => {
            logging!(warn, Type::Window, "Failed to parse window state: {e}");
            return;
        }
    };

    let w = data.width.unwrap_or(940);
    let h = data.height.unwrap_or(700);

    if let (Some(x), Some(y)) = (data.x, data.y) {
        if window_rect_intersects_any_monitor(window, x, y, w, h) {
            if let Err(e) = window.set_position(tauri::PhysicalPosition::new(x, y)) {
                logging!(warn, Type::Window, "Failed to restore window position: {e}");
            }
        } else {
            logging!(
                warn,
                Type::Window,
                "Saved window geometry ({x},{y}) is off all monitors; centering main window"
            );
            if let Err(e) = window.center() {
                logging!(warn, Type::Window, "Failed to center window after bad state: {e}");
            }
        }
    }

    if let (Some(w), Some(h)) = (data.width, data.height) {
        if let Err(e) = window.set_size(tauri::PhysicalSize::new(w, h)) {
            logging!(warn, Type::Window, "Failed to restore window size: {e}");
        }
    }

    logging!(debug, Type::Window, "Window state restored from {path:?}");
}
