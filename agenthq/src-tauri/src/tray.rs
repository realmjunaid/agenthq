//! Windows tray. Status lines are labels only — they do not claim a color
//! the menu cannot show. Exit is the only path that quits the process.

use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};

use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{Emitter, Manager};

use crate::database::repository::Db;

pub static PAUSED: AtomicBool = AtomicBool::new(false);

pub fn install(app: &tauri::App) -> Result<(), String> {
    let handle = app.handle().clone();
    let open = MenuItem::with_id(app, "open", "Open Dashboard", true, None::<&str>)
        .map_err(|e| e.to_string())?;
    let pause = MenuItem::with_id(app, "pause", "Pause Monitoring", true, None::<&str>)
        .map_err(|e| e.to_string())?;
    let settings = MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)
        .map_err(|e| e.to_string())?;
    let exit =
        MenuItem::with_id(app, "exit", "Exit", true, None::<&str>).map_err(|e| e.to_string())?;
    let menu =
        Menu::with_items(app, &[&open, &pause, &settings, &exit]).map_err(|e| e.to_string())?;
    let mut builder = TrayIconBuilder::with_id("main")
        .tooltip("AgentHQ")
        .menu(&menu)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => show_main(app),
            "settings" => {
                show_main(app);
                let _ = app.emit("agenthq://navigate", "settings");
            }
            "pause" => toggle_pause(app),
            "exit" => app.exit(0),
            _ => {}
        });
    if let Some(icon) = handle.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app).map_err(|e| e.to_string())?;
    let _ = refresh_menu(&handle);
    Ok(())
}

pub fn show_main(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

fn toggle_pause(app: &tauri::AppHandle) {
    let next = !PAUSED.load(Ordering::SeqCst);
    PAUSED.store(next, Ordering::SeqCst);
    if let Some(db) = app.try_state::<Db>() {
        let _ = db.set_setting("monitoring.paused", if next { "1" } else { "0" });
    }
    let _ = refresh_menu(app);
}

pub fn refresh_menu(app: &tauri::AppHandle) -> Result<(), String> {
    let Some(tray) = app.tray_by_id("main") else {
        return Ok(());
    };
    let paused = PAUSED.load(Ordering::SeqCst);
    let mut items: Vec<MenuItem<tauri::Wry>> = vec![];
    let title = MenuItem::with_id(app, "title", "AI Agent Center", false, None::<&str>)
        .map_err(|e| e.to_string())?;
    items.push(title);
    if let Some(db) = app.try_state::<Db>() {
        if let Ok(rows) = db.list_agents() {
            for row in rows {
                if !row.installed && !row.running {
                    continue;
                }
                let state = if row.running {
                    "Running"
                } else if row.installed {
                    "Offline"
                } else {
                    "Unknown"
                };
                let item = MenuItem::with_id(
                    app,
                    format!("agent-{}", row.id),
                    format!("{} — {state}", row.name),
                    false,
                    None::<&str>,
                )
                .map_err(|e| e.to_string())?;
                items.push(item);
            }
        }
    }
    let open = MenuItem::with_id(app, "open", "Open Dashboard", true, None::<&str>)
        .map_err(|e| e.to_string())?;
    let pause_label = if paused {
        "Resume Monitoring"
    } else {
        "Pause Monitoring"
    };
    let pause = MenuItem::with_id(app, "pause", pause_label, true, None::<&str>)
        .map_err(|e| e.to_string())?;
    let settings = MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)
        .map_err(|e| e.to_string())?;
    let exit =
        MenuItem::with_id(app, "exit", "Exit", true, None::<&str>).map_err(|e| e.to_string())?;
    items.push(open);
    items.push(pause);
    items.push(settings);
    items.push(exit);
    let refs: Vec<&dyn tauri::menu::IsMenuItem<tauri::Wry>> =
        items.iter().map(|item| item as _).collect();
    let menu = Menu::with_items(app, &refs).map_err(|e| e.to_string())?;
    tray.set_menu(Some(menu)).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn set_run_at_login(enabled: bool) -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let status = if enabled {
        Command::new("reg")
            .args([
                "add",
                r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run",
                "/v",
                "AgentHQ",
                "/t",
                "REG_SZ",
                "/d",
                &exe.to_string_lossy(),
                "/f",
            ])
            .status()
    } else {
        Command::new("reg")
            .args([
                "delete",
                r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run",
                "/v",
                "AgentHQ",
                "/f",
            ])
            .status()
    }
    .map_err(|e| e.to_string())?;
    if !status.success() && enabled {
        return Err(format!("registry update failed: {status}"));
    }
    Ok(())
}

pub fn close_to_tray(app: &tauri::AppHandle) -> bool {
    app.try_state::<Db>()
        .and_then(|db| db.get_setting("tray.close_to_tray").ok().flatten())
        .map(|v| v == "1")
        .unwrap_or(true)
}
