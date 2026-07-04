//! System tray: playback controls that work with the window hidden.
//! Menu clicks emit events to the frontend, which calls the same player
//! functions the on-screen buttons use — Rust→frontend IPC in the
//! opposite direction from `invoke`.

use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter, Manager};

pub fn setup(app: &AppHandle) -> tauri::Result<()> {
    let play_pause = MenuItem::with_id(app, "play-pause", "Play / Pause", true, None::<&str>)?;
    let next = MenuItem::with_id(app, "next", "Next", true, None::<&str>)?;
    let prev = MenuItem::with_id(app, "prev", "Previous", true, None::<&str>)?;
    let show = MenuItem::with_id(app, "show", "Show window", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let sep1 = PredefinedMenuItem::separator(app)?;
    let sep2 = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(app, &[&play_pause, &next, &prev, &sep1, &show, &sep2, &quit])?;

    TrayIconBuilder::with_id("main-tray")
        .tooltip("Music Player")
        .icon(
            app.default_window_icon()
                .expect("bundled window icon")
                .clone(),
        )
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "play-pause" => {
                let _ = app.emit("media:play-pause", ());
            }
            "next" => {
                let _ = app.emit("media:next", ());
            }
            "prev" => {
                let _ = app.emit("media:prev", ());
            }
            "show" => show_main_window(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .build(app)?;

    Ok(())
}

pub fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}
