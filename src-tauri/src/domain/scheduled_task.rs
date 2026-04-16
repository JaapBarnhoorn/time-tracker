use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Occurrence {
    Once,
    Daily,
    Weekly,
    BiWeekly,
    Monthly,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ScheduledTask {
    pub id: Option<i64>,
    pub task_name: String,
    pub occurrence: Occurrence,
    pub start_time: String, // "HH:MM"
    pub day_of_week: Option<u32>, // 0-6 (Mon-Sun)
    pub day_of_month: Option<u32>, // 1-31
    pub last_run: Option<String>, // "YYYY-MM-DD"
}
