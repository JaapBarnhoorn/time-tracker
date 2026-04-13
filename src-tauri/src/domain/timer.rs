use serde::Serialize;
use chrono::{DateTime, Utc};

#[derive(Serialize, Clone)]
pub struct StatusResponse {
    pub running: bool,
    pub id: Option<i64>,
    pub task_name: Option<String>,
    pub started_at: Option<DateTime<Utc>>,
    pub elapsed_seconds: u64,
}

#[derive(Serialize, Clone)]
pub struct WeeklyReportEntry {
    pub task_name: String,
    pub total_seconds_per_day: Vec<u64>, // Index 0-6 (Mon-Sun or matching week days)
    pub total_seconds: u64,
}

#[derive(Serialize, Clone)]
pub struct WeeklyReport {
    pub start_date: String,
    pub end_date: String,
    pub entries: Vec<WeeklyReportEntry>,
    pub daily_totals: Vec<u64>,
}
