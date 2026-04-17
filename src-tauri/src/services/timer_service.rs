use crate::domain::timer::{StatusResponse, WeeklyReport};
use crate::repository::sqlite::SqliteRepository;
use crate::domain::time_entry::TimeEntry;
use crate::domain::scheduled_task::{ScheduledTask, Occurrence};
use chrono::{DateTime, Utc};
use std::sync::{Arc, Mutex};
use anyhow::{Result, Context};

pub struct TimerService {
    repo: Arc<Mutex<SqliteRepository>>,
}

impl TimerService {
    pub fn new(repo: Arc<Mutex<SqliteRepository>>) -> Self {
        Self { repo }
    }

    pub fn get_setting(&self, key: String) -> Result<Option<String>> {
        let repo = self.repo.lock().unwrap();
        repo.get_setting(&key)
    }

    pub fn set_setting(&self, key: String, value: String) -> Result<()> {
        let repo = self.repo.lock().unwrap();
        repo.set_setting(&key, &value)
    }

    pub fn get_scheduled_tasks(&self) -> Result<Vec<ScheduledTask>> {
        let repo = self.repo.lock().unwrap();
        repo.get_scheduled_tasks()
    }

    pub fn add_scheduled_task(&self, task_name: String, occurrence: Occurrence, start_time: String, day_of_week: Option<u32>, day_of_month: Option<u32>) -> Result<()> {
        let repo = self.repo.lock().unwrap();
        repo.add_scheduled_task(&task_name, occurrence, &start_time, day_of_week, day_of_month)
    }

    pub fn delete_scheduled_task(&self, id: i64) -> Result<()> {
        let repo = self.repo.lock().unwrap();
        repo.delete_scheduled_task(id)
    }

    pub fn import_tasks(&self, json_data: String) -> Result<usize> {
        let names: Vec<String> = serde_json::from_str(&json_data)
            .with_context(|| "Ongeldig JSON formaat voor taken import")?;
        let repo = self.repo.lock().unwrap();
        repo.add_tasks_bulk(names)
    }

    pub fn update_scheduled_task_last_run(&self, id: i64, date: String) -> Result<()> {
        let repo = self.repo.lock().unwrap();
        repo.update_scheduled_task_last_run(id, &date)
    }

    pub fn get_all_tasks(&self) -> Result<Vec<String>> {
        let repo = self.repo.lock().unwrap();
        repo.get_all_tasks()
    }

    pub fn add_task(&self, name: String) -> Result<()> {
        let repo = self.repo.lock().unwrap();
        repo.add_task(&name)
    }

    pub fn start(&self, task_name: String) -> Result<()> {
        let repo = self.repo.lock().unwrap();
        if let Some((id, _, _)) = repo.get_running_task()? {
            repo.stop_task(id)?;
        }
        repo.start_task(&task_name)?;
        Ok(())
    }

    pub fn stop(&self) -> Result<()> {
        let repo = self.repo.lock().unwrap();
        if let Some((id, _, _)) = repo.get_running_task()? {
            repo.stop_task(id)?;
        }
        Ok(())
    }

    pub fn add_manual_entry(&self, task_name: String, started_at: DateTime<Utc>, stopped_at: DateTime<Utc>) -> Result<()> {
        let repo = self.repo.lock().unwrap();
        repo.add_manual_entry(&task_name, started_at, stopped_at)
    }

    pub fn update_entry(&self, id: i64, started_at: DateTime<Utc>, stopped_at: Option<DateTime<Utc>>) -> Result<()> {
        let repo = self.repo.lock().unwrap();
        repo.update_entry(id, started_at, stopped_at)
    }

    pub fn delete_entry(&self, id: i64) -> Result<()> {
        let repo = self.repo.lock().unwrap();
        repo.delete_time_entry(id)
    }

    pub fn status(&self) -> Result<StatusResponse> {
        let repo = self.repo.lock().unwrap();
        match repo.get_running_task()? {
            Some((id, name, started_at)) => Ok(StatusResponse {
                running: true,
                id: Some(id),
                task_name: Some(name),
                started_at: Some(started_at),
                elapsed_seconds: Utc::now().signed_duration_since(started_at).num_seconds() as u64,
            }),
            None => Ok(StatusResponse {
                running: false,
                id: None,
                task_name: None,
                started_at: None,
                elapsed_seconds: 0,
            }),
        }
    }

    pub fn get_daily_entries(&self, date: String) -> Result<Vec<TimeEntry>> {
        let repo = self.repo.lock().unwrap();
        repo.get_daily_entries(&date)
    }

    pub fn get_weekly_report(&self, start_date: String) -> Result<WeeklyReport> {
        let repo = self.repo.lock().unwrap();
        repo.get_weekly_report(&start_date)
    }

    pub fn get_top_tasks(&self) -> Result<Vec<String>> {
        let repo = self.repo.lock().unwrap();
        repo.get_top_tasks()
    }

    pub fn get_last_task_name(&self) -> Result<Option<String>> {
        let repo = self.repo.lock().unwrap();
        repo.get_last_task_name()
    }
}
