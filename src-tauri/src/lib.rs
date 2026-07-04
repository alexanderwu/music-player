mod db;
mod models;
mod scanner;
mod tray;

use tauri::{Manager, WindowEvent};
use tauri_plugin_store::StoreExt;

/// Whether the system tray came up. Close-to-tray is only safe when there is
/// actually a tray to reopen the window from.
struct TrayActive(bool);

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_window_state::Builder::new().build())
        .setup(|app| {
            let db = db::init(app.handle())?;

            // Re-grant asset-protocol access to the folders scanned in past
            // sessions (and the cover cache) — the scope is runtime state,
            // not persisted by Tauri.
            {
                let conn = db.0.lock().unwrap();
                for folder in db::folder_paths(&conn)? {
                    let _ = app.asset_protocol_scope().allow_directory(&folder, true);
                }
            }
            let covers = app.path().app_data_dir()?.join("covers");
            std::fs::create_dir_all(&covers)?;
            let _ = app.asset_protocol_scope().allow_directory(&covers, false);

            app.manage(db);

            // A missing tray host (some Linux desktops) shouldn't kill the app.
            let tray_active = match tray::setup(app.handle()) {
                Ok(()) => true,
                Err(error) => {
                    eprintln!("tray unavailable: {error}");
                    false
                }
            };
            app.manage(TrayActive(tray_active));

            #[cfg(desktop)]
            setup_media_keys(app.handle());

            Ok(())
        })
        .on_window_event(|window, event| {
            // Close-to-tray: intercept the close request and hide instead,
            // unless the user turned the setting off. Quit lives in the tray.
            if let WindowEvent::CloseRequested { api, .. } = event {
                let app = window.app_handle();
                let close_to_tray = app
                    .store("settings.json")
                    .ok()
                    .and_then(|store| store.get("closeToTray"))
                    .and_then(|value| value.as_bool())
                    .unwrap_or(true);
                if close_to_tray && app.state::<TrayActive>().0 {
                    let _ = window.hide();
                    api.prevent_close();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            scanner::scan_folder,
            scanner::handle_dropped_paths,
            db::get_tracks,
            db::search_tracks,
            db::get_playlists,
            db::create_playlist,
            db::rename_playlist,
            db::delete_playlist,
            db::get_playlist_tracks,
            db::add_to_playlist,
            db::remove_from_playlist,
            db::set_playlist_order,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Media keys via global shortcuts. Registration can fail (e.g. another app
/// holds the key, or a Wayland compositor without the right protocol) — the
/// app must still work, so failures only log.
#[cfg(desktop)]
fn setup_media_keys(app: &tauri::AppHandle) {
    use tauri_plugin_global_shortcut::{Builder, Shortcut, ShortcutState};

    let parse = |s: &str| s.parse::<Shortcut>().expect("valid media key name");
    let play_pause = parse("MediaPlayPause");
    let next = parse("MediaTrackNext");
    let prev = parse("MediaTrackPrevious");

    let builder = Builder::new()
        .with_shortcuts([play_pause, next, prev])
        .expect("valid media key shortcuts")
        .with_handler(move |app, shortcut, event| {
            use tauri::Emitter;
            if event.state == ShortcutState::Pressed {
                if shortcut == &play_pause {
                    let _ = app.emit("media:play-pause", ());
                } else if shortcut == &next {
                    let _ = app.emit("media:next", ());
                } else if shortcut == &prev {
                    let _ = app.emit("media:prev", ());
                }
            }
        });

    if let Err(error) = app.plugin(builder.build()) {
        eprintln!("media keys unavailable: {error}");
    }
}
