#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use chrono::{TimeZone, Utc};
use dayboard_core::{
    model::{Actor, Command, ImportSummary, Response},
    Core,
};
use std::sync::Arc;
use tauri::{Emitter, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};
use tauri_plugin_notification::NotificationExt;

struct State {
    core: tokio::sync::Mutex<Core>,
    changed: Arc<tokio::sync::Notify>,
    data_dir: std::path::PathBuf,
}
#[tauri::command]
async fn dispatch(
    app: tauri::AppHandle,
    state: tauri::State<'_, State>,
    command: Command,
) -> Result<Response, String> {
    let is_read = matches!(
        command,
        Command::PingDb
            | Command::Snapshot { .. }
            | Command::ParseQuickAdd { .. }
            | Command::PlanDay { .. }
    );
    // Never hold this lock across notification, tray or webview work: those
    // calls can block waiting for the UI thread, which would deadlock it.
    let (response, remaining) = {
        let mut core = state.core.lock().await;
        let response = core
            .execute(command, Actor::Ui, Utc::now())
            .map_err(|e| e.to_string())?;
        let remaining = if is_read {
            None
        } else {
            Some(core.today_count(Utc::now()).unwrap_or(0))
        };
        (response, remaining)
    };
    if let Some(remaining) = remaining {
        state.changed.notify_one();
        let _ = app.emit("daybridge:changed", ());
        update_tray(&app, remaining);
    }
    Ok(response)
}
fn show(app: &tauri::AppHandle, label: &str) {
    if let Some(w) = app.get_webview_window(label) {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
        let _ = w.emit("daybridge:shown", ());
    }
}
fn rolling_backup(core: &Core, data: &std::path::Path) {
    let directory = data.join("backups");
    if std::fs::create_dir_all(&directory).is_err() {
        return;
    }
    let stamp = Utc::now().format("%Y-%m-%d").to_string();
    let path = directory.join(format!("daybridge-{stamp}.sqlite3"));
    if !path.exists() {
        let _ = core.backup_to(&path);
    }
    let mut files: Vec<std::path::PathBuf> = std::fs::read_dir(&directory)
        .into_iter()
        .flatten()
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "sqlite3")
        })
        .collect();
    files.sort();
    while files.len() > 7 {
        let oldest = files.remove(0);
        let _ = std::fs::remove_file(oldest);
    }
}
#[tauri::command]
async fn export_data(state: tauri::State<'_, State>, kind: String) -> Result<String, String> {
    let (contents, extension) = {
        let core = state.core.lock().await;
        match kind.as_str() {
            "markdown" => (core.export_markdown().map_err(|e| e.to_string())?, "md"),
            _ => (core.export_json().map_err(|e| e.to_string())?, "json"),
        }
    };
    let directory = state.data_dir.join("exports");
    std::fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
    let stamp = Utc::now().format("%Y%m%d-%H%M%S");
    let path = directory.join(format!("daybridge-{stamp}.{extension}"));
    std::fs::write(&path, contents).map_err(|e| e.to_string())?;
    Ok(path.display().to_string())
}
#[tauri::command]
async fn import_data(
    app: tauri::AppHandle,
    state: tauri::State<'_, State>,
    path: String,
) -> Result<ImportSummary, String> {
    let raw = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let (summary, remaining) = {
        let mut core = state.core.lock().await;
        let summary = core
            .import_json(&raw, Actor::Ui, Utc::now())
            .map_err(|e| e.to_string())?;
        (summary, core.today_count(Utc::now()).unwrap_or(0))
    };
    state.changed.notify_one();
    let _ = app.emit("daybridge:changed", ());
    update_tray(&app, remaining);
    Ok(summary)
}
fn percent_encode_path(value: &str) -> String {
    let mut out = String::new();
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~' | b'/') {
            out.push(byte as char);
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}
#[tauri::command]
fn open_in_vscode(path: String) -> Result<(), String> {
    let path = path.trim();
    if path.is_empty() {
        return Err("This project has no repository path yet".into());
    }
    if !std::path::Path::new(path).exists() {
        return Err(format!("Repository path does not exist: {path}"));
    }
    let normalized = percent_encode_path(&path.replace('\\', "/"));
    let file = if normalized.starts_with('/') {
        normalized
    } else {
        format!("/{normalized}")
    };
    let url = format!("vscode://file{file}");
    let mut command;
    #[cfg(target_os = "windows")]
    {
        command = std::process::Command::new("rundll32");
        command.args(["url.dll,FileProtocolHandler", &url]);
    }
    #[cfg(target_os = "macos")]
    {
        command = std::process::Command::new("open");
        command.arg(&url);
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        command = std::process::Command::new("xdg-open");
        command.arg(&url);
    }
    command
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("Could not open VS Code: {e}"))
}
fn update_tray(app: &tauri::AppHandle, remaining: usize) {
    let icon = remaining_icon(remaining);
    let tooltip = format!("Daybridge · {remaining} remaining today");
    let title = remaining.to_string();
    let handle = app.clone();
    // Tray handles are owned by the UI thread; marshalling the update keeps a
    // background reminder from blocking on Win32 SendMessage.
    let _ = app.run_on_main_thread(move || {
        if let Some(tray) = handle.tray_by_id("daybridge") {
            let _ = tray.set_icon(Some(icon));
            let _ = tray.set_tooltip(Some(tooltip));
            let _ = tray.set_title(Some(title));
        }
    });
}
fn remaining_icon(count: usize) -> tauri::image::Image<'static> {
    const DIGITS: [[u8; 5]; 11] = [
        [7, 5, 5, 5, 7],
        [2, 6, 2, 2, 7],
        [7, 1, 7, 4, 7],
        [7, 1, 7, 1, 7],
        [5, 5, 7, 1, 1],
        [7, 4, 7, 1, 7],
        [7, 4, 7, 5, 7],
        [7, 1, 1, 1, 1],
        [7, 5, 7, 5, 7],
        [7, 5, 7, 1, 7],
        [0, 2, 7, 2, 0],
    ];
    let label = if count > 99 {
        "99+".to_string()
    } else {
        count.to_string()
    };
    let scale = if label.len() > 2 { 2 } else { 3 };
    let width = label.len() * 4 * scale - scale;
    let mut pixels = [27, 32, 26, 255].repeat(32 * 32);
    for (i, ch) in label.chars().enumerate() {
        let glyph = &DIGITS[ch.to_digit(10).unwrap_or(10) as usize];
        for (row, bits) in glyph.iter().enumerate() {
            for col in 0..3 {
                if bits & (1 << (2 - col)) != 0 {
                    for dy in 0..scale {
                        for dx in 0..scale {
                            let x = (32 - width) / 2 + i * 4 * scale + col * scale + dx;
                            let y = (32 - 5 * scale) / 2 + row * scale + dy;
                            pixels[(y * 32 + x) * 4..(y * 32 + x) * 4 + 4]
                                .copy_from_slice(&[196, 216, 165, 255]);
                        }
                    }
                }
            }
        }
    }
    tauri::image::Image::new_owned(pixels, 32, 32)
}
fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| show(app, "main")))
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().with_handler(|app, _, event| { if event.state() == ShortcutState::Pressed { show(app, "capture"); } }).build())
        .invoke_handler(tauri::generate_handler![
            dispatch,
            open_in_vscode,
            export_data,
            import_data
        ])
        .setup(|app| {
            let data = std::env::var_os("DAYBRIDGE_DATA_DIR").map(std::path::PathBuf::from).unwrap_or(app.path().app_data_dir()?); std::fs::create_dir_all(&data)?;
            let mut core = Core::open(data.join("daybridge.sqlite3"))?;
            core.execute(Command::Rollover { dry_run: false }, Actor::Ui, Utc::now())?;
            rolling_backup(&core, &data);
            let settings = core.settings()?;
            let remaining = core.today_count(Utc::now()).unwrap_or(0);
            let changed = Arc::new(tokio::sync::Notify::new());
            app.manage(State {core: tokio::sync::Mutex::new(core), changed: changed.clone(), data_dir: data.clone()});
            let open = tauri::menu::MenuItem::with_id(app, "open", "Open Daybridge", true, None::<&str>)?;
            let capture = tauri::menu::MenuItem::with_id(app, "capture", "Quick capture", true, None::<&str>)?;
            let quit = tauri::menu::MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = tauri::menu::Menu::with_items(app, &[&open, &capture, &quit])?;
            let icon = app.default_window_icon().cloned().ok_or("Missing application icon")?;
            tauri::tray::TrayIconBuilder::with_id("daybridge").icon(icon).menu(&menu).show_menu_on_left_click(true)
                .on_menu_event(|app, event| match event.id.as_ref() {"open" => show(app, "main"), "capture" => show(app, "capture"), "quit" => { let state = app.state::<State>(); if let Ok(core) = state.core.try_lock() { let _ = core.checkpoint(); } app.exit(0);}, _ => ()}).build(app)?;
            if let Err(e) = app.global_shortcut().register("CommandOrControl+Shift+Space") { eprintln!("Quick capture shortcut unavailable: {e}. Use the tray menu."); }
            update_tray(app.handle(), remaining);
            if settings.bridge_enabled {
                let handle = app.handle().clone();
                let changed_for_bridge = changed.clone();
                let notifier: dayboard_bridge::Notifier = Arc::new(move |_| {
                    changed_for_bridge.notify_one();
                    let _ = handle.emit("daybridge:changed", ());
                });
                match dayboard_bridge::start(
                    &data.join("daybridge.sqlite3"),
                    settings.bridge_port,
                    settings.bridge_token.clone(),
                    notifier,
                ) {
                    Ok(port) => eprintln!("Daybridge bridge listening on 127.0.0.1:{port}"),
                    Err(error) => {
                        eprintln!("Daybridge bridge failed to start: {error}");
                        let _ = app.emit(
                            "daybridge:error",
                            format!("Bridge failed to start: {error}"),
                        );
                    }
                }
            }
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                loop {
                    // No polling: mutations, next reminder, or local midnight are the only wakeups.
                    let now = Utc::now();
                    let (next, midnight, rolled, remaining) = {
                        let state = handle.state::<State>();
                        let mut core = state.core.lock().await;
                        let rolled = matches!(core.execute(Command::Rollover { dry_run: false }, Actor::Ui, now), Ok(Response::Tasks(tasks)) if !tasks.is_empty());
                        let tz = core.settings().ok().and_then(|s| s.timezone.parse::<chrono_tz::Tz>().ok()).unwrap_or(chrono_tz::UTC);
                        let tomorrow = now.with_timezone(&tz).date_naive().succ_opt().unwrap();
                        // Midnight can be skipped in some zones; choose the first valid local hour.
                        let boundary = (0..24).find_map(|h| tz.from_local_datetime(&tomorrow.and_hms_opt(h,0,0).unwrap()).earliest()).unwrap().with_timezone(&Utc);
                        (core.next_reminder().ok().flatten(), boundary, rolled, core.today_count(now).unwrap_or(0))
                    };
                    if rolled { let _ = handle.emit("daybridge:changed", ()); }
                    update_tray(&handle, remaining);
                    let wake = next.as_ref().map(|(_,at)| *at).unwrap_or(midnight).min(midnight);
                    let delay = (wake - Utc::now()).to_std().unwrap_or_default();
                    tokio::select! { _ = changed.notified() => continue, _ = tokio::time::sleep(delay) => {} }
                    let now = Utc::now();
                    let due = {
                        let state = handle.state::<State>();
                        let core = state.core.lock().await;
                        match core.next_reminder() { Ok(Some((task, at))) if at <= now => Some(task), _ => None }
                    };
                    let mut storage_failed = false;
                    if let Some(task) = due {
                        // Deliver outside the lock: notification and tray work can
                        // block on the UI thread, which must stay free to run commands.
                        let shown = handle.notification().builder().title("Time for your task").body(&task.title).show();
                        let mut notice = None;
                        let state = handle.state::<State>();
                        let mut core = state.core.lock().await;
                        match shown {
                            Ok(()) => if let Err(e) = core.mark_reminded(&task.id, now) { storage_failed = true; notice = Some(format!("Could not save reminder state: {e}")); },
                            Err(e) => { eprintln!("Notification unavailable: {e}"); if core.mark_reminder_failed(&task.id, now).is_err() { storage_failed = true; } notice = Some("A reminder could not be delivered. Check OS notification permissions, then reschedule the task to retry.".to_string()); }
                        }
                        drop(core);
                        if let Some(message) = notice { let _ = handle.emit("daybridge:error", message); }
                    }
                    let remaining = {
                        let state = handle.state::<State>();
                        let mut core = state.core.lock().await;
                        let _ = core.execute(Command::Rollover {dry_run:false}, Actor::Ui, Utc::now());
                        core.today_count(Utc::now()).unwrap_or(0)
                    };
                    update_tray(&handle, remaining); let _ = handle.emit("daybridge:changed", ());
                    if storage_failed { changed.notified().await; }
                }
            });
            Ok(())
        })
        .on_window_event(|window, event| { if let tauri::WindowEvent::CloseRequested { api, .. } = event { api.prevent_close(); let _ = window.hide(); } })
        .run(tauri::generate_context!()).expect("Daybridge failed to start");
}
