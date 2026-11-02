#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use chrono::{TimeZone, Utc};
use dayboard_core::{
    model::{Actor, Command, Response},
    Core,
};
use std::sync::{Arc, Mutex};
use tauri::{Emitter, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};
use tauri_plugin_notification::NotificationExt;

struct State {
    core: Mutex<Core>,
    changed: Arc<tokio::sync::Notify>,
}
#[tauri::command]
fn dispatch(
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
    let response = state
        .core
        .lock()
        .map_err(|_| "Database lock unavailable")?
        .execute(command, Actor::Ui, Utc::now())
        .map_err(|e| e.to_string())?;
    if !is_read {
        state.changed.notify_one();
        let _ = app.emit("daybridge:changed", ());
        update_tray(&app);
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
fn update_tray(app: &tauri::AppHandle) {
    let state = app.state::<State>();
    if let Ok(mut core) = state.core.lock() {
        if let Ok(Response::Snapshot(s)) =
            core.execute(Command::Snapshot { search: None }, Actor::Ui, Utc::now())
        {
            if let Some(tray) = app.tray_by_id("daybridge") {
                let _ = tray.set_icon(Some(remaining_icon(s.capacity.count)));
                let _ = tray.set_tooltip(Some(format!(
                    "Daybridge · {} remaining today",
                    s.capacity.count
                )));
                let _ = tray.set_title(Some(s.capacity.count.to_string()));
            }
        }
    };
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
        .invoke_handler(tauri::generate_handler![dispatch])
        .setup(|app| {
            let data = std::env::var_os("DAYBRIDGE_DATA_DIR").map(std::path::PathBuf::from).unwrap_or(app.path().app_data_dir()?); std::fs::create_dir_all(&data)?;
            let mut core = Core::open(data.join("daybridge.sqlite3"))?;
            core.execute(Command::Rollover { dry_run: false }, Actor::Ui, Utc::now())?;
            let changed = Arc::new(tokio::sync::Notify::new());
            app.manage(State {core: Mutex::new(core), changed: changed.clone()});
            let open = tauri::menu::MenuItem::with_id(app, "open", "Open Daybridge", true, None::<&str>)?;
            let capture = tauri::menu::MenuItem::with_id(app, "capture", "Quick capture", true, None::<&str>)?;
            let quit = tauri::menu::MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = tauri::menu::Menu::with_items(app, &[&open, &capture, &quit])?;
            let icon = app.default_window_icon().cloned().ok_or("Missing application icon")?;
            tauri::tray::TrayIconBuilder::with_id("daybridge").icon(icon).menu(&menu).show_menu_on_left_click(true)
                .on_menu_event(|app, event| match event.id.as_ref() {"open" => show(app, "main"), "capture" => show(app, "capture"), "quit" => { if let Ok(core) = app.state::<State>().core.lock() {let _ = core.checkpoint();} app.exit(0);}, _ => ()}).build(app)?;
            if let Err(e) = app.global_shortcut().register("CommandOrControl+Shift+Space") { eprintln!("Quick capture shortcut unavailable: {e}. Use the tray menu."); }
            update_tray(app.handle());
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                loop {
                    // No polling: mutations, next reminder, or local midnight are the only wakeups.
                    let (next, midnight, rolled) = {
                        let state = handle.state::<State>();
                        let mut core = match state.core.lock() {Ok(c) => c, Err(_) => break};
                        let rolled = matches!(core.execute(Command::Rollover { dry_run: false }, Actor::Ui, Utc::now()), Ok(Response::Tasks(tasks)) if !tasks.is_empty());
                        let tz = core.settings().ok().and_then(|s| s.timezone.parse::<chrono_tz::Tz>().ok()).unwrap_or(chrono_tz::UTC);
                        let tomorrow = Utc::now().with_timezone(&tz).date_naive().succ_opt().unwrap();
                        // Midnight can be skipped in some zones; choose the first valid local hour.
                        let boundary = (0..24).find_map(|h| tz.from_local_datetime(&tomorrow.and_hms_opt(h,0,0).unwrap()).earliest()).unwrap().with_timezone(&Utc);
                        (core.next_reminder().ok().flatten(), boundary, rolled)
                    };
                    if rolled { update_tray(&handle); let _ = handle.emit("daybridge:changed", ()); }
                    let wake = next.as_ref().map(|(_,at)| *at).unwrap_or(midnight).min(midnight);
                    let delay = (wake - Utc::now()).to_std().unwrap_or_default();
                    tokio::select! { _ = changed.notified() => continue, _ = tokio::time::sleep(delay) => {} }
                    let mut storage_failed = false;
                    {
                        let state = handle.state::<State>();
                        if let Ok(mut core) = state.core.lock() {
                            let now = Utc::now();
                            if let Ok(Some((task, at))) = core.next_reminder() { if at <= now {
                                match handle.notification().builder().title("Time for your task").body(&task.title).show() {
                                    Ok(()) => { if let Err(e) = core.mark_reminded(&task.id, now) { storage_failed = true; let _ = handle.emit("daybridge:error", format!("Could not save reminder state: {e}")); } },
                                    Err(e) => { eprintln!("Notification unavailable: {e}"); if core.mark_reminder_failed(&task.id, now).is_err() { storage_failed = true; } let _ = handle.emit("daybridge:error", "A reminder could not be delivered. Check OS notification permissions, then reschedule the task to retry."); }
                                }
                            } }
                            let _ = core.execute(Command::Rollover {dry_run:false}, Actor::Ui, now);
                        };
                    }
                    update_tray(&handle); let _ = handle.emit("daybridge:changed", ());
                    if storage_failed { changed.notified().await; }
                }
            });
            Ok(())
        })
        .on_window_event(|window, event| { if let tauri::WindowEvent::CloseRequested { api, .. } = event { api.prevent_close(); let _ = window.hide(); } })
        .run(tauri::generate_context!()).expect("Daybridge failed to start");
}
