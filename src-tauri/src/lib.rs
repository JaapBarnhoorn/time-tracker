pub mod domain;
pub mod repository;
pub mod services;
pub mod state;
pub mod commands;

use std::sync::{Arc, Mutex};
use crate::repository::sqlite::SqliteRepository;
use crate::services::timer_service::TimerService;
use crate::state::AppState;
use tauri::{AppHandle, Manager, menu::{Menu, MenuItem}, tray::{TrayIcon, TrayIconBuilder, TrayIconEvent}};
use chrono::{Timelike, Datelike, Local};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_global_shortcut::Builder::new()
            .with_handler(|app, shortcut, event| {
                use tauri_plugin_global_shortcut::ShortcutState;
                if event.state() == ShortcutState::Pressed {
                    let s = shortcut.to_string().to_lowercase();
                    if s.contains("shift") && s.contains("t") && (s.contains("super") || s.contains("alt") || s.contains("command") || s.contains("option")) {
                        if let Some(window) = app.get_webview_window("main") {
                            if window.is_visible().unwrap_or(false) {
                                let _ = window.hide();
                            } else {
                                let _ = window.show();
                                let _ = window.unminimize();
                                let _ = window.set_focus();
                            }
                        }
                    }
                }
            })
            .build())
        .setup(|app| {
            use tauri_plugin_global_shortcut::{Shortcut, GlobalShortcutExt, Modifiers, Code};
            let app_handle = app.handle().clone();
            
            // Registreer twee varianten voor maximale kans van slagen
            // Variant 1: Command+Shift+T (Super+Shift+T)
            let sc1 = Shortcut::new(Some(Modifiers::SUPER | Modifiers::SHIFT), Code::KeyT);
            if let Err(e) = app.global_shortcut().register(sc1) {
                println!("Fout bij registreren sc1: {}", e);
            }

            // Variant 2: Alt+Shift+T (Option+Shift+T) - vaak veiliger op macOS
            let sc2 = Shortcut::new(Some(Modifiers::ALT | Modifiers::SHIFT), Code::KeyT);
            if let Err(e) = app.global_shortcut().register(sc2) {
                println!("Fout bij registreren sc2: {}", e);
            }
            
            // Tray Icon Setup
            let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let show_i = MenuItem::with_id(app, "show", "Open Tracker", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_i, &quit_i])?;

            let _tray = TrayIconBuilder::with_id("main_tray")
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app: &AppHandle, event| match event.id.as_ref() {
                    "quit" => { app.exit(0); }
                    "show" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray: &TrayIcon, event| {
                    if let TrayIconEvent::Click { .. } = event {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.unminimize();
                            let _ = window.set_focus();
                        }
                    }
                })
                .build(app)?;

            let data_dir = app_handle.path().app_data_dir().expect("failed to get app data dir");
            std::fs::create_dir_all(&data_dir).expect("failed to create app data dir");
            
            let db_path = data_dir.join("timer.db");
            let repo = Arc::new(Mutex::new(SqliteRepository::new(&db_path).expect("failed to init db")));
            let timer_service = TimerService::new(repo);
            
            app.manage(AppState { timer_service });
            
            let app_handle_for_thread = app_handle.clone();
            std::thread::spawn(move || {
                let mut reminder_counter = 0;
                let mut last_reminder_task_id = -1;
                let mut last_checked_minute = -1;
                let mut cached_work_days: Vec<u32> = vec![1, 2, 3, 4, 5];
                
                loop {
                    std::thread::sleep(std::time::Duration::from_secs(1));
                    
                    if let Some(state) = app_handle_for_thread.try_state::<AppState>() {
                        let now = Local::now();
                        let minute = now.minute() as i32;
                        
                        // Update cached work days once per minute
                        if minute != last_checked_minute {
                            if let Ok(Some(saved)) = state.timer_service.get_setting("workDays".to_string()) {
                                cached_work_days = serde_json::from_str(&saved).unwrap_or(vec![1, 2, 3, 4, 5]);
                            }
                            last_checked_minute = minute;
                        }

                        let today_str = now.format("%Y-%m-%d").to_string();
                        let now_time_str = now.format("%H:%M").to_string();
                        let weekday = now.weekday(); 
                        
                        // Check Scheduled Tasks
                        if let Ok(scheduled_tasks) = state.timer_service.get_scheduled_tasks() {
                            for task in scheduled_tasks {
                                // Check if task is due today based on occurrence
                                let is_due_today = match task.occurrence {
                                    crate::domain::scheduled_task::Occurrence::Once => true,
                                    crate::domain::scheduled_task::Occurrence::Daily => {
                                        // Smart "Daily" only on work days
                                        cached_work_days.contains(&((weekday.number_from_monday() % 7) as u32))
                                    },
                                    crate::domain::scheduled_task::Occurrence::Weekly => {
                                        task.day_of_week == Some((weekday.number_from_monday() % 7) as u32)
                                    },
                                    crate::domain::scheduled_task::Occurrence::BiWeekly => {
                                        // Simple BiWeekly: same weekday and even week number (or odd, depending on start)
                                        // To be more precise we'd need a reference date, but let's use week number for now
                                        task.day_of_week == Some((weekday.number_from_monday() % 7) as u32) && (now.iso_week().week() % 2 == 0)
                                    },
                                    crate::domain::scheduled_task::Occurrence::Monthly => {
                                        task.day_of_month == Some(now.day())
                                    },
                                };

                                if is_due_today && task.last_run.as_deref() != Some(&today_str) {
                                    // Parse start_time
                                    let parts: Vec<&str> = task.start_time.split(':').collect();
                                    if parts.len() == 2 {
                                        let h: u32 = parts[0].parse().unwrap_or(0);
                                        let m: u32 = parts[1].parse().unwrap_or(0);
                                        
                                        let scheduled_dt = now.with_hour(h).and_then(|dt| dt.with_minute(m)).and_then(|dt| dt.with_second(0)).unwrap_or(now);
                                        let diff = scheduled_dt.signed_duration_since(now).num_minutes();

                                        // 5 Minute Reminder
                                        if diff == 5 && last_reminder_task_id != task.id.unwrap_or(-1) {
                                            let _ = tauri_plugin_notification::NotificationExt::notification(&app_handle_for_thread)
                                                .builder()
                                                .title("Geplande taak komt eraan")
                                                .body(format!("Over 5 minuten begint: {}. Je huidige taak wordt dan gestopt.", task.task_name))
                                                .show();
                                            last_reminder_task_id = task.id.unwrap_or(-1);
                                        }

                                        // Auto Start
                                        if now_time_str == task.start_time {
                                            let _ = state.timer_service.start(task.task_name.clone());
                                            
                                            if let crate::domain::scheduled_task::Occurrence::Once = task.occurrence {
                                                // If Once, delete it after starting
                                                let _ = state.timer_service.delete_scheduled_task(task.id.unwrap());
                                            } else {
                                                // Otherwise just mark as run today
                                                let _ = state.timer_service.update_scheduled_task_last_run(task.id.unwrap(), today_str.clone());
                                            }
                                            
                                            let _ = tauri_plugin_notification::NotificationExt::notification(&app_handle_for_thread)
                                                .builder()
                                                .title("Geplande taak gestart")
                                                .body(format!("De taak '{}' is automatisch gestart.", task.task_name))
                                                .show();
                                        }
                                    }
                                }
                            }
                        }

                        // Regular Reminder
                        if let Ok(status) = state.timer_service.status() {
                            if status.running {
                                reminder_counter += 1;
                                if reminder_counter >= 1800 {
                                    let _ = tauri_plugin_notification::NotificationExt::notification(&app_handle_for_thread)
                                        .builder()
                                        .title("Time Tracker Reminder")
                                        .body(format!("Werk je nog aan: {}?", status.task_name.unwrap_or_default()))
                                        .show();
                                    reminder_counter = 0;
                                }
                            } else {
                                reminder_counter = 0;
                            }
                        }
                    }
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_weekly_report,
            commands::get_scheduled_tasks,
            commands::add_scheduled_task,
            commands::delete_scheduled_task,
            commands::get_setting,
            commands::set_setting,
            commands::start_task,
            commands::stop_task,
            commands::add_task,
            commands::get_status,
            commands::get_tasks,
            commands::get_daily_entries,
            commands::get_top_tasks,
            commands::get_last_task_name,
            commands::add_manual_entry,
            commands::update_time_entry,
            commands::delete_time_entry
        ])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            if let tauri::RunEvent::Reopen { .. } = event {
                if let Some(window) = app_handle.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.unminimize();
                    let _ = window.set_focus();
                }
            }
        });
}
