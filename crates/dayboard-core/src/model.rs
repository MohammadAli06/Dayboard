use chrono::{DateTime, NaiveDate, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Inbox,
    Todo,
    Doing,
    Done,
    Dropped,
}
impl Status {
    pub fn active(self) -> bool {
        !matches!(self, Self::Done | Self::Dropped)
    }
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Energy {
    Low,
    High,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Actor {
    User,
    Ui,
    Jarvis,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Task {
    pub id: String,
    pub title: String,
    pub notes: String,
    pub status: Status,
    pub priority: u8,
    pub energy: Energy,
    pub est_min: u32,
    pub actual_min: u32,
    pub due_at: Option<DateTime<Utc>>,
    pub planned_day: Option<NaiveDate>,
    pub scheduled_start: Option<DateTime<Utc>>,
    pub scheduled_end: Option<DateTime<Utc>>,
    pub parent_id: Option<String>,
    pub tags: Vec<String>,
    pub carried_count: u32,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub reminded_at: Option<DateTime<Utc>>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Settings {
    pub timezone: String,
    pub max_tasks: u32,
    pub max_minutes: u32,
    pub work_start: u8,
    pub work_end: u8,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            timezone: "UTC".into(),
            max_tasks: 8,
            max_minutes: 360,
            work_start: 9,
            work_end: 18,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ParsedTask {
    pub title: String,
    pub priority: u8,
    pub energy: Energy,
    pub est_min: u32,
    pub tags: Vec<String>,
    pub due_at: Option<DateTime<Utc>>,
    pub planned_day: Option<NaiveDate>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TaskEdit {
    pub id: String,
    pub title: String,
    pub notes: String,
    pub status: Status,
    pub priority: u8,
    pub energy: Energy,
    pub est_min: u32,
    pub actual_min: u32,
    pub tags: Vec<String>,
    pub parent_id: Option<String>,
    pub due_local: Option<String>,
    pub scheduled_local: Option<String>,
    pub planned_day: Option<NaiveDate>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "command", content = "input", rename_all = "snake_case")]
pub enum Command {
    PingDb,
    Snapshot {
        search: Option<String>,
    },
    ParseQuickAdd {
        text: String,
    },
    CreateTask {
        text: String,
        parent_id: Option<String>,
    },
    UpdateTask(TaskEdit),
    CompleteTask {
        id: String,
        completed: bool,
    },
    DeleteTask {
        id: String,
        dry_run: bool,
    },
    RestoreTask {
        id: String,
    },
    RescheduleTask {
        id: String,
        local_start: Option<String>,
        day: Option<NaiveDate>,
        dry_run: bool,
    },
    Rollover {
        dry_run: bool,
    },
    SaveSettings(Settings),
    PlanDay {
        day: NaiveDate,
        energy: Option<Energy>,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Capacity {
    pub count: usize,
    pub minutes: u32,
    pub over_count: bool,
    pub over_minutes: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Snapshot {
    pub tasks: Vec<Task>,
    pub settings: Settings,
    pub today: NaiveDate,
    pub capacity: Capacity,
    pub tags: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum Response {
    Pong(String),
    Snapshot(Snapshot),
    Parsed(ParsedTask),
    Task(Task),
    Tasks(Vec<Task>),
    Settings(Settings),
}
