use tauri::State;
use crate::state::AppState;
use crate::domain::timer::{StatusResponse, WeeklyReport};
use crate::domain::time_entry::TimeEntry;
use crate::domain::scheduled_task::{ScheduledTask, Occurrence};
use chrono::{DateTime, Utc};

#[tauri::command]
pub fn get_scheduled_tasks(state: State<AppState>) -> Result<Vec<ScheduledTask>, String> {
    state.timer_service.get_scheduled_tasks().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn add_scheduled_task(task_name: String, occurrence: Occurrence, start_time: String, state: State<AppState>) -> Result<(), String> {
    state.timer_service.add_scheduled_task(task_name, occurrence, start_time).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_scheduled_task(id: i64, state: State<AppState>) -> Result<(), String> {
    state.timer_service.delete_scheduled_task(id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_setting(key: String, state: State<AppState>) -> Result<Option<String>, String> {
    state.timer_service.get_setting(key).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_setting(key: String, value: String, state: State<AppState>) -> Result<(), String> {
    state.timer_service.set_setting(key, value).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn start_task(task_name: String, state: State<AppState>) -> Result<(), String> {
    state.timer_service.start(task_name).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn stop_task(state: State<AppState>) -> Result<(), String> {
    state.timer_service.stop().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn add_manual_entry(task_name: String, started_at: DateTime<Utc>, stopped_at: DateTime<Utc>, state: State<AppState>) -> Result<(), String> {
    state.timer_service.add_manual_entry(task_name, started_at, stopped_at).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_time_entry(id: i64, started_at: DateTime<Utc>, stopped_at: Option<DateTime<Utc>>, state: State<AppState>) -> Result<(), String> {
    state.timer_service.update_entry(id, started_at, stopped_at).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_time_entry(id: i64, state: State<AppState>) -> Result<(), String> {
    state.timer_service.delete_entry(id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_weekly_report(start_date: String, state: State<AppState>) -> Result<WeeklyReport, String> {
    state.timer_service.get_weekly_report(start_date).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_status(state: State<AppState>) -> Result<StatusResponse, String> {
    state.timer_service.status().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn add_task(name: String, state: State<AppState>) -> Result<(), String> {
    state.timer_service.add_task(name).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_tasks(state: State<AppState>) -> Result<Vec<String>, String> {
    state.timer_service.get_all_tasks().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_daily_entries(date: String, state: State<AppState>) -> Result<Vec<TimeEntry>, String> {
    state.timer_service.get_daily_entries(date).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_top_tasks(state: State<AppState>) -> Result<Vec<String>, String> {
    state.timer_service.get_top_tasks().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_last_task_name(state: State<AppState>) -> Result<Option<String>, String> {
    state.timer_service.get_last_task_name().map_err(|e| e.to_string())
}
