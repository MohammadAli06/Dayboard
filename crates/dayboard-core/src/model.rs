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
    #[serde(default)]
    pub project_id: Option<String>,
    #[serde(default)]
    pub goal_id: Option<String>,
    #[serde(default)]
    pub dsa_problem_id: Option<String>,
    pub tags: Vec<String>,
    pub carried_count: u32,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub reminded_at: Option<DateTime<Utc>>,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProjectKind {
    Coding,
    College,
    Personal,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProjectStatus {
    Active,
    Paused,
    Archived,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub kind: ProjectKind,
    pub repo_path: Option<String>,
    pub branch: Option<String>,
    pub status: ProjectStatus,
    pub created_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GoalHorizon {
    Medium,
    Long,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Goal {
    pub id: String,
    pub title: String,
    pub horizon: GoalHorizon,
    pub target_date: Option<NaiveDate>,
    pub note: String,
    pub created_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GoalProgress {
    pub goal_id: String,
    pub total: usize,
    pub done: usize,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Template {
    pub id: String,
    pub name: String,
    pub title: String,
    pub notes: String,
    pub priority: u8,
    pub energy: Energy,
    pub est_min: u32,
    pub tags: Vec<String>,
    pub project_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SavedFilter {
    pub id: String,
    pub name: String,
    pub view: String,
    pub query: String,
    pub created_at: DateTime<Utc>,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DsaDifficulty {
    Easy,
    Medium,
    Hard,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DsaStatus {
    Todo,
    Solved,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DsaProblem {
    pub id: String,
    pub title: String,
    pub url: Option<String>,
    pub topic: String,
    pub difficulty: DsaDifficulty,
    pub platform: Option<String>,
    pub status: DsaStatus,
    pub confidence: u8,
    pub solved_at: Option<DateTime<Utc>>,
    pub next_review_at: Option<NaiveDate>,
    pub review_stage: u8,
    pub notes: String,
    pub created_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TopicCoverage {
    pub topic: String,
    pub total: usize,
    pub solved: usize,
    pub average_confidence: f64,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Subject {
    pub id: String,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SyllabusItem {
    pub id: String,
    pub subject_id: String,
    pub title: String,
    pub done: bool,
    pub created_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Deadline {
    pub id: String,
    pub subject_id: Option<String>,
    pub title: String,
    pub due_date: NaiveDate,
    pub study_min: u32,
    pub created_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
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
    #[serde(default)]
    pub project_id: Option<String>,
    #[serde(default)]
    pub goal_id: Option<String>,
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
    CreateProject {
        name: String,
        kind: ProjectKind,
        repo_path: Option<String>,
        branch: Option<String>,
    },
    UpdateProject {
        id: String,
        name: String,
        kind: ProjectKind,
        repo_path: Option<String>,
        branch: Option<String>,
        status: ProjectStatus,
    },
    DeleteProject {
        id: String,
    },
    CreateGoal {
        title: String,
        horizon: GoalHorizon,
        target_date: Option<NaiveDate>,
        note: String,
    },
    UpdateGoal {
        id: String,
        title: String,
        horizon: GoalHorizon,
        target_date: Option<NaiveDate>,
        note: String,
    },
    DeleteGoal {
        id: String,
    },
    CreateTemplate {
        name: String,
        title: String,
        notes: String,
        priority: u8,
        energy: Energy,
        est_min: u32,
        tags: Vec<String>,
        project_id: Option<String>,
    },
    CreateTaskFromTemplate {
        template_id: String,
        parent_id: Option<String>,
    },
    DeleteTemplate {
        id: String,
    },
    ImportIssue {
        url: String,
    },
    SaveFilter {
        name: String,
        view: String,
        query: String,
    },
    DeleteFilter {
        id: String,
    },
    CreateDsaProblem {
        title: String,
        url: Option<String>,
        topic: String,
        difficulty: DsaDifficulty,
        platform: Option<String>,
        notes: String,
    },
    UpdateDsaProblem {
        id: String,
        title: String,
        url: Option<String>,
        topic: String,
        difficulty: DsaDifficulty,
        platform: Option<String>,
        notes: String,
    },
    SolveDsaProblem {
        id: String,
        confidence: u8,
    },
    ReviewDsaProblem {
        id: String,
        confidence: u8,
    },
    DeleteDsaProblem {
        id: String,
    },
    CreateSubject {
        name: String,
    },
    DeleteSubject {
        id: String,
    },
    AddSyllabusItem {
        subject_id: String,
        title: String,
    },
    SetSyllabusItemDone {
        id: String,
        done: bool,
    },
    DeleteSyllabusItem {
        id: String,
    },
    CreateDeadline {
        subject_id: Option<String>,
        title: String,
        due_date: NaiveDate,
        study_min: u32,
    },
    DeleteDeadline {
        id: String,
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
    pub projects: Vec<Project>,
    pub goals: Vec<Goal>,
    pub goal_progress: Vec<GoalProgress>,
    pub templates: Vec<Template>,
    pub saved_filters: Vec<SavedFilter>,
    pub dsa_problems: Vec<DsaProblem>,
    pub dsa_topics: Vec<TopicCoverage>,
    pub subjects: Vec<Subject>,
    pub syllabus_items: Vec<SyllabusItem>,
    pub deadlines: Vec<Deadline>,
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
    Project(Project),
    Goal(Goal),
    Template(Template),
    Filter(SavedFilter),
    DsaProblem(DsaProblem),
    Subject(Subject),
    SyllabusItem(SyllabusItem),
    Deadline(Deadline),
}
