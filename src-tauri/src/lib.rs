use std::{sync::{Arc, Mutex, OnceLock}, thread, time::{Duration, Instant}};

use log::{error, info};
use serde::{Deserialize, Serialize};
use tauri::{menu::{MenuBuilder, MenuItemBuilder}, tray::TrayIconBuilder, AppHandle, Emitter, Manager};

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ScheduleEventPayload {
    seconds_remaining: u64,
}

#[derive(Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ScheduleMode {
    CountdownMinutes(u64),
    SpecificTime { hour: u32, minute: u32 },
}

struct SchedulerState {
    target: Option<Instant>,
    cancelled: bool,
}

impl SchedulerState {
    fn new() -> Self {
        Self { target: None, cancelled: false }
    }
}

static SCHEDULER: OnceLock<Arc<Mutex<SchedulerState>>> = OnceLock::new();

fn scheduler() -> Arc<Mutex<SchedulerState>> {
    SCHEDULER.get_or_init(|| Arc::new(Mutex::new(SchedulerState::new()))).clone()
}

pub fn compute_target_from_mode(mode: ScheduleMode) -> Instant {
    match mode {
        ScheduleMode::CountdownMinutes(mins) => Instant::now() + Duration::from_secs(mins * 60),
        ScheduleMode::SpecificTime { hour, minute } => {
            use chrono::prelude::*;
            let now = Local::now();
            let mut target = now.date_naive().and_hms_opt(hour, minute, 0).unwrap();
            if target < now.naive_local() {
                target = (now.date_naive() + chrono::Days::new(1)).and_hms_opt(hour, minute, 0).unwrap();
            }
            let dur = (target - now.naive_local()).to_std().unwrap_or(Duration::from_secs(0));
            Instant::now() + dur
        }
    }
}

fn spawn_schedule_thread(app: AppHandle, mode: ScheduleMode) {
    let target = compute_target_from_mode(mode);
    {
        let sched = scheduler();
        let mut s = sched.lock().unwrap();
        s.target = Some(target);
        s.cancelled = false;
    }
    info!("shutdown scheduled");
    app.emit("shutdown-scheduled", ScheduleEventPayload { seconds_remaining: target.saturating_duration_since(Instant::now()).as_secs() }).ok();

    thread::spawn(move || {
        loop {
            // Check cancellation
            {
                let s = scheduler();
                let st = s.lock().unwrap();
                if st.cancelled {
                    info!("shutdown cancelled");
                    let _ = app.emit("shutdown-cancelled", ScheduleEventPayload { seconds_remaining: 0 });
                    return;
                }
            }

            let now = Instant::now();
            if now >= target {
                let _ = app.emit("shutdown-executing", ScheduleEventPayload { seconds_remaining: 0 });
                if let Err(e) = execute_shutdown() {
                    error!("shutdown command failed: {}", e);
                }
                {
                    let s = scheduler();
                    let mut st = s.lock().unwrap();
                    st.target = None;
                }
                return;
            } else {
                let remaining = target.saturating_duration_since(now).as_secs();
                if remaining == 60 {
                    let _ = app.emit("shutdown-warning", ScheduleEventPayload { seconds_remaining: remaining });
                }
                let _ = app.emit("shutdown-tick", ScheduleEventPayload { seconds_remaining: remaining });
                thread::sleep(Duration::from_secs(1));
            }
        }
    });
}

fn execute_shutdown() -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        use std::process::Command;
        Command::new("shutdown").args(["/s", "/t", "0"]).spawn().map_err(|e| e.to_string())?;
        return Ok(());
    }
    #[cfg(target_os = "linux")]
    {
        use std::process::Command;
        if let Err(e) = Command::new("systemctl").arg("poweroff").spawn() { return Err(e.to_string()); }
        return Ok(());
    }
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        if let Err(e) = Command::new("osascript").args(["-e", "tell application \"System Events\" to shut down"]).spawn() { return Err(e.to_string()); }
        return Ok(());
    }
}

#[tauri::command]
fn schedule_countdown(app: AppHandle, minutes: u64) -> Result<(), String> {
    if minutes == 0 { return Err("minutes must be > 0".into()); }
    spawn_schedule_thread(app, ScheduleMode::CountdownMinutes(minutes));
    Ok(())
}

#[tauri::command]
fn schedule_at(app: AppHandle, hour: u32, minute: u32) -> Result<(), String> {
    if hour > 23 || minute > 59 { return Err("invalid time".into()); }
    spawn_schedule_thread(app, ScheduleMode::SpecificTime { hour, minute });
    Ok(())
}

#[tauri::command]
fn cancel_shutdown() -> Result<(), String> {
    let sched = scheduler();
    let mut st = sched.lock().unwrap();
    st.cancelled = true;
    Ok(())
}

#[derive(Serialize)]
struct RemainingResult { seconds: Option<u64> }

#[tauri::command]
fn remaining_seconds() -> RemainingResult {
    let sched = scheduler();
    let st = sched.lock().unwrap();
    RemainingResult { seconds: st.target.map(|t| t.saturating_duration_since(Instant::now()).as_secs()) }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_log::Builder::new().build())
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            let show_item = MenuItemBuilder::with_id("show", "打开窗口").build(app)?;
            let cancel_item = MenuItemBuilder::with_id("cancel", "取消关机").build(app)?;
            let quit_item = MenuItemBuilder::with_id("quit", "退出").build(app)?;
            let menu = MenuBuilder::new(app).items(&[&show_item, &cancel_item, &quit_item]).build()?;

            let _tray = TrayIconBuilder::new()
                .menu(&menu)
                .on_menu_event(move |app, event| match event.id().as_ref() {
                    "show" => {
                        if let Some(win) = app.get_webview_window("main") { let _ = win.show(); let _ = win.set_focus(); }
                    }
                    "cancel" => { let _ = cancel_shutdown(); }
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, _event| {
                    let app = tray.app_handle();
                    if let Some(win) = app.get_webview_window("main") { let _ = win.show(); let _ = win.set_focus(); }
                })
                .build(app)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![schedule_countdown, schedule_at, cancel_shutdown, remaining_seconds])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
