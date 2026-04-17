use rusqlite::{params, Connection};
use anyhow::{Result, Context};
use chrono::{DateTime, Utc};
use std::path::Path;
use crate::domain::time_entry::TimeEntry;
use crate::domain::scheduled_task::{ScheduledTask, Occurrence};
use crate::domain::timer::{WeeklyReport, WeeklyReportEntry};
use std::collections::HashMap;

pub struct SqliteRepository {
    conn: Connection,
}

impl SqliteRepository {
    pub fn new(db_path: &Path) -> Result<Self> {
        let conn = Connection::open(db_path)
            .with_context(|| format!("Kon database niet openen op {:?}", db_path))?;
        
        let repo = Self { conn };
        repo.init_db().context("Database initialisatie mislukt")?;
        Ok(repo)
    }

    fn init_db(&self) -> Result<()> {
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS time_entries (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                task_name TEXT NOT NULL,
                started_at TEXT NOT NULL,
                stopped_at TEXT
            )",
            [],
        )?;

        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS tasks (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT UNIQUE NOT NULL,
                created_at TEXT NOT NULL
            )",
            [],
        )?;

        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            )",
            [],
        )?;

        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS scheduled_tasks (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                task_name TEXT NOT NULL,
                occurrence TEXT NOT NULL, -- JSON serialized enum
                start_time TEXT NOT NULL, -- HH:MM
                day_of_week INTEGER, -- 0-6
                day_of_month INTEGER, -- 1-31
                last_run TEXT, -- YYYY-MM-DD
                created_at TEXT NOT NULL
            )",
            [],
        )?;

        // Migration: Add columns if they don't exist
        let _ = self.conn.execute("ALTER TABLE scheduled_tasks ADD COLUMN day_of_week INTEGER", []);
        let _ = self.conn.execute("ALTER TABLE scheduled_tasks ADD COLUMN day_of_month INTEGER", []);

        self.seed_defaults()?;
        Ok(())
    }

    fn seed_defaults(&self) -> Result<()> {
        let existing = self.get_setting("workDays")?;
        if existing.is_none() {
            self.set_setting("workDays", "[1,2,3,4,5]")?;
        }
        Ok(())
    }

    pub fn get_scheduled_tasks(&self) -> Result<Vec<ScheduledTask>> {
        let mut stmt = self.conn.prepare("SELECT id, task_name, occurrence, start_time, day_of_week, day_of_month, last_run FROM scheduled_tasks")?;
        let rows = stmt.query_map([], |row| {
            let occ_str: String = row.get(2)?;
            let occurrence: Occurrence = serde_json::from_str(&occ_str).unwrap_or(Occurrence::Daily);
            Ok(ScheduledTask {
                id: Some(row.get(0)?),
                task_name: row.get(1)?,
                occurrence,
                start_time: row.get(3)?,
                day_of_week: row.get(4)?,
                day_of_month: row.get(5)?,
                last_run: row.get(6)?,
            })
        })?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        Ok(results)
    }

    pub fn add_scheduled_task(&self, task_name: &str, occurrence: Occurrence, start_time: &str, day_of_week: Option<u32>, day_of_month: Option<u32>) -> Result<()> {
        let occ_str = serde_json::to_string(&occurrence).unwrap();
        self.conn.execute(
            "INSERT INTO scheduled_tasks (task_name, occurrence, start_time, day_of_week, day_of_month, created_at) VALUES (?, ?, ?, ?, ?, ?)",
            params![task_name, occ_str, start_time, day_of_week, day_of_month, Utc::now().to_rfc3339()],
        )?;
        Ok(())
    }

    pub fn update_scheduled_task_last_run(&self, id: i64, date: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE scheduled_tasks SET last_run = ? WHERE id = ?",
            params![date, id],
        )?;
        Ok(())
    }

    pub fn delete_scheduled_task(&self, id: i64) -> Result<()> {
        self.conn.execute("DELETE FROM scheduled_tasks WHERE id = ?", params![id])?;
        Ok(())
    }

    pub fn get_setting(&self, key: &str) -> Result<Option<String>> {
        let mut stmt = self.conn.prepare("SELECT value FROM settings WHERE key = ?")?;
        let mut rows = stmt.query(params![key])?;
        if let Some(row) = rows.next()? {
            Ok(Some(row.get(0)?))
        } else {
            Ok(None)
        }
    }

    pub fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES (?, ?)",
            params![key, value],
        )?;
        Ok(())
    }

    pub fn get_all_tasks(&self) -> Result<Vec<String>> {
        let mut stmt = self.conn.prepare("SELECT name FROM tasks ORDER BY name ASC")?;
        let rows = stmt.query_map([], |row| row.get(0))?;
        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        Ok(results)
    }

    pub fn add_task(&self, name: &str) -> Result<()> {
        self.conn.execute(
            "INSERT OR IGNORE INTO tasks (name, created_at) VALUES (?, ?)",
            params![name, Utc::now().to_rfc3339()],
        )?;
        Ok(())
    }

    pub fn add_tasks_bulk(&self, names: Vec<String>) -> Result<usize> {
        let mut added_count = 0;
        self.conn.execute("BEGIN TRANSACTION", [])?;
        for name in names {
            self.conn.execute(
                "INSERT OR IGNORE INTO tasks (name, created_at) VALUES (?, ?)",
                params![name, Utc::now().to_rfc3339()],
            )?;
            if self.conn.changes() > 0 {
                added_count += 1;
            }
        }
        self.conn.execute("COMMIT TRANSACTION", [])?;
        println!("Database seed voltooid: {} nieuwe taken toegevoegd.", added_count);
        Ok(added_count)
    }

    pub fn delete_task(&self, id: i64) -> Result<()> {
        self.conn.execute("DELETE FROM tasks WHERE id = ?", params![id])?;
        Ok(())
    }

    pub fn start_task(&self, task_name: &str) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO time_entries (task_name, started_at) VALUES (?, ?)",
            params![task_name, Utc::now().to_rfc3339()],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn stop_task(&self, id: i64) -> Result<()> {
        self.conn.execute(
            "UPDATE time_entries SET stopped_at = ? WHERE id = ? AND stopped_at IS NULL",
            params![Utc::now().to_rfc3339(), id],
        )?;
        Ok(())
    }

    pub fn add_manual_entry(&self, task_name: &str, started_at: DateTime<Utc>, stopped_at: DateTime<Utc>) -> Result<()> {
        self.conn.execute(
            "INSERT INTO time_entries (task_name, started_at, stopped_at) VALUES (?, ?, ?)",
            params![task_name, started_at.to_rfc3339(), stopped_at.to_rfc3339()],
        )?;
        Ok(())
    }

    pub fn update_entry(&self, id: i64, started_at: DateTime<Utc>, stopped_at: Option<DateTime<Utc>>) -> Result<()> {
        self.conn.execute(
            "UPDATE time_entries SET started_at = ?, stopped_at = ? WHERE id = ?",
            params![started_at.to_rfc3339(), stopped_at.map(|dt| dt.to_rfc3339()), id],
        )?;
        Ok(())
    }

    pub fn delete_time_entry(&self, id: i64) -> Result<()> {
        self.conn.execute("DELETE FROM time_entries WHERE id = ?", params![id])?;
        Ok(())
    }

    pub fn get_top_tasks(&self) -> Result<Vec<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT task_name, COUNT(*) as count FROM time_entries GROUP BY task_name ORDER BY count DESC LIMIT 9"
        )?;
        let rows = stmt.query_map([], |row| row.get(0))?;
        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        Ok(results)
    }

    pub fn get_last_task_name(&self) -> Result<Option<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT task_name FROM time_entries ORDER BY started_at DESC LIMIT 1"
        )?;
        let mut rows = stmt.query([])?;
        if let Some(row) = rows.next()? {
            Ok(Some(row.get(0)?))
        } else {
            Ok(None)
        }
    }

    pub fn get_running_task(&self) -> Result<Option<(i64, String, DateTime<Utc>)>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, task_name, started_at FROM time_entries WHERE stopped_at IS NULL LIMIT 1"
        )?;
        
        let mut rows = stmt.query([])?;

        if let Some(row) = rows.next()? {
            let id: i64 = row.get(0)?;
            let name: String = row.get(1)?;
            let started_at_str: String = row.get(2)?;
            let started_at = DateTime::parse_from_rfc3339(&started_at_str)
                .map(|dt| dt.with_timezone(&Utc))
                .context("Tijdstempel parsing fout")?;
            
            Ok(Some((id, name, started_at)))
        } else {
            Ok(None)
        }
    }

    pub fn get_weekly_report(&self, start_date: &str) -> Result<WeeklyReport> {
        let mut stmt = self.conn.prepare(
            "SELECT task_name, started_at, stopped_at FROM time_entries 
             WHERE started_at >= ? AND started_at < date(?, '+7 days')
             ORDER BY started_at ASC"
        )?;
        
        let start_dt = DateTime::parse_from_rfc3339(&(format!("{}T00:00:00Z", start_date)))
            .map(|dt| dt.with_timezone(&Utc))
            .context("Ongeldige startdatum voor weekrapport")?;

        let rows = stmt.query_map(params![start_date, start_date], |row| {
            let name: String = row.get(0)?;
            let started_at_str: String = row.get(1)?;
            let stopped_at_str: Option<String> = row.get(2)?;
            
            let started_at = DateTime::parse_from_rfc3339(&started_at_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or(Utc::now());
            
            let stopped_at = stopped_at_str.and_then(|s| {
                DateTime::parse_from_rfc3339(&s)
                    .map(|dt| dt.with_timezone(&Utc))
                    .ok()
            });

            Ok((name, started_at, stopped_at))
        })?;

        let mut entries_map: HashMap<String, WeeklyReportEntry> = HashMap::new();
        let mut daily_totals = vec![0u64; 7];

        for row in rows {
            let (name, started_at, stopped_at) = row?;
            if let Some(stopped_at) = stopped_at {
                let seconds = stopped_at.signed_duration_since(started_at).num_seconds().max(0) as u64;
                
                let day_offset = (started_at.signed_duration_since(start_dt).num_days() as usize).min(6);
                daily_totals[day_offset] += seconds;

                let entry = entries_map.entry(name.clone()).or_insert(WeeklyReportEntry {
                    task_name: name,
                    total_seconds_per_day: vec![0u64; 7],
                    total_seconds: 0,
                });
                
                entry.total_seconds_per_day[day_offset] += seconds;
                entry.total_seconds += seconds;
            }
        }

        let mut entries: Vec<WeeklyReportEntry> = entries_map.into_values().collect();
        entries.sort_by(|a, b| b.total_seconds.cmp(&a.total_seconds));

        Ok(WeeklyReport {
            start_date: start_date.to_string(),
            end_date: start_dt.checked_add_signed(chrono::Duration::days(6)).unwrap().format("%Y-%m-%d").to_string(),
            entries,
            daily_totals,
        })
    }

    pub fn get_daily_entries(&self, date: &str) -> Result<Vec<TimeEntry>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, task_name, started_at, stopped_at FROM time_entries WHERE started_at LIKE ? ORDER BY started_at DESC"
        )?;
        
        let rows = stmt.query_map(params![format!("{}%", date)], |row| {
            let started_at_str: String = row.get(2)?;
            let stopped_at_str: Option<String> = row.get(3)?;
            
            let started_at = DateTime::parse_from_rfc3339(&started_at_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or(Utc::now());
            
            let stopped_at = stopped_at_str.and_then(|s| {
                DateTime::parse_from_rfc3339(&s)
                    .map(|dt| dt.with_timezone(&Utc))
                    .ok()
            });

            Ok(TimeEntry {
                id: Some(row.get(0)?),
                task_name: row.get(1)?,
                started_at,
                stopped_at,
            })
        })?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row.context("Fout bij ophalen van record")?);
        }
        Ok(results)
    }
}
