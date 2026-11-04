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
    #[serde(default)]
    pub recurrence_rule: Option<String>,
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
pub struct UndoResult {
    pub action: String,
    pub task: Task,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FocusKind {
    Focus,
    Break,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FocusSession {
    pub id: String,
    pub task_id: Option<String>,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub planned_min: u32,
    pub kind: FocusKind,
    pub created_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Habit {
    pub id: String,
    pub name: String,
    pub cadence_days: u32,
    pub created_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct HabitLog {
    pub id: String,
    pub habit_id: String,
    pub day: NaiveDate,
    pub created_at: DateTime<Utc>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct HabitStreak {
    pub habit_id: String,
    pub current: u32,
    pub best: u32,
    pub last_done: Option<NaiveDate>,
    pub due: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DailyReview {
    pub id: String,
    pub day: NaiveDate,
    pub note: String,
    pub created_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TagStat {
    pub tag: String,
    pub focus_minutes: u32,
    pub completed: usize,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WeeklyStats {
    pub week_start: NaiveDate,
    pub completed: usize,
    pub created: usize,
    pub carried: usize,
    pub focus_minutes: u32,
    pub estimate_accuracy: f64,
    pub measured: usize,
    pub tag_stats: Vec<TagStat>,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ScheduleKind {
    Class,
    Work,
    Routine,
    Other,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExceptionAction {
    Skip,
    Move,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ScheduleItem {
    pub id: String,
    pub title: String,
    pub kind: ScheduleKind,
    pub weekdays: u32,
    pub start_time: String,
    pub end_time: String,
    pub location: Option<String>,
    pub notes: Option<String>,
    pub subject_id: Option<String>,
    pub remind_before_min: Option<u32>,
    pub active: bool,
    pub term_id: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct ScheduleException {
    pub id: String,
    pub item_id: String,
    pub date: NaiveDate,
    pub action: ExceptionAction,
    pub new_start: Option<String>,
    pub new_end: Option<String>,
    pub note: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct Occurrence {
    pub item_id: String,
    pub title: String,
    pub kind: ScheduleKind,
    pub date: NaiveDate,
    pub start_utc: DateTime<Utc>,
    pub end_utc: DateTime<Utc>,
    pub location: Option<String>,
    pub is_moved: bool,
    pub remind_before_min: Option<u32>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Term {
    pub id: String,
    pub name: String,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub active: bool,
    pub archived: bool,
    pub created_at: DateTime<Utc>,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AcademicKind {
    Midterm,
    Practical,
    Oral,
    Final,
    AssignmentDue,
    Holiday,
    Other,
}
impl AcademicKind {
    pub fn is_exam(self) -> bool {
        matches!(
            self,
            Self::Midterm | Self::Practical | Self::Oral | Self::Final
        )
    }
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SuppressClasses {
    None,
    OverlapOnly,
    WholeDay,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AcademicEvent {
    pub id: String,
    pub term_id: String,
    pub kind: AcademicKind,
    pub title: String,
    pub subject_id: Option<String>,
    pub start_at: DateTime<Utc>,
    pub end_at: DateTime<Utc>,
    pub all_day: bool,
    pub prep_days: u32,
    pub suppress_classes: SuppressClasses,
    pub notes: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DismissedConflict {
    pub task_id: String,
    pub date: NaiveDate,
    pub rule_id: String,
    pub created_at: DateTime<Utc>,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DayStatusKind {
    Free,
    Normal,
    Heavy,
    Blocked,
    PrepCritical,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct DayReason {
    pub rule_id: String,
    pub message: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DayStatus {
    pub date: NaiveDate,
    pub status: DayStatusKind,
    pub available_min: u32,
    pub used_min: u32,
    pub reasons: Vec<DayReason>,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SuggestionAction {
    Move,
    Defer,
    Split,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ConflictSuggestion {
    pub action: SuggestionAction,
    pub target_date: Option<NaiveDate>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Conflict {
    pub task_id: String,
    pub date: NaiveDate,
    pub rule_id: String,
    pub reason: String,
    pub suggestion: ConflictSuggestion,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ProposedTask {
    pub title: String,
    pub day: NaiveDate,
    pub est_min: u32,
    pub tags: Vec<String>,
    pub subject_id: Option<String>,
    pub syllabus_item_id: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PrepPlan {
    pub event_id: String,
    pub event_title: String,
    pub tasks: Vec<ProposedTask>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WeekLoadEntry {
    pub date: NaiveDate,
    pub status: DayStatusKind,
    pub available_min: u32,
    pub used_min: u32,
    pub schedule_min: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct UndoOutcome {
    pub entity: String,
    pub action: String,
    pub id: String,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WakeupKind {
    Task,
    Schedule,
    Exam,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Wakeup {
    pub kind: WakeupKind,
    pub at: DateTime<Utc>,
    pub title: String,
    pub entity_id: String,
    pub key: String,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ImportSummary {
    #[serde(default)]
    pub tasks: usize,
    #[serde(default)]
    pub projects: usize,
    #[serde(default)]
    pub goals: usize,
    #[serde(default)]
    pub templates: usize,
    #[serde(default)]
    pub saved_filters: usize,
    #[serde(default)]
    pub dsa_problems: usize,
    #[serde(default)]
    pub subjects: usize,
    #[serde(default)]
    pub syllabus_items: usize,
    #[serde(default)]
    pub deadlines: usize,
    #[serde(default)]
    pub habits: usize,
    #[serde(default)]
    pub habit_logs: usize,
    #[serde(default)]
    pub focus_sessions: usize,
    #[serde(default)]
    pub daily_reviews: usize,
    #[serde(default)]
    pub schedule_items: usize,
    #[serde(default)]
    pub schedule_exceptions: usize,
    #[serde(default)]
    pub terms: usize,
    #[serde(default)]
    pub academic_events: usize,
    #[serde(default)]
    pub dismissed_conflicts: usize,
    #[serde(default)]
    pub skipped: usize,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportBundle {
    pub format: String,
    pub version: u32,
    pub exported_at: DateTime<Utc>,
    #[serde(default)]
    pub tasks: Vec<Task>,
    #[serde(default)]
    pub projects: Vec<Project>,
    #[serde(default)]
    pub goals: Vec<Goal>,
    #[serde(default)]
    pub templates: Vec<Template>,
    #[serde(default)]
    pub saved_filters: Vec<SavedFilter>,
    #[serde(default)]
    pub dsa_problems: Vec<DsaProblem>,
    #[serde(default)]
    pub subjects: Vec<Subject>,
    #[serde(default)]
    pub syllabus_items: Vec<SyllabusItem>,
    #[serde(default)]
    pub deadlines: Vec<Deadline>,
    #[serde(default)]
    pub habits: Vec<Habit>,
    #[serde(default)]
    pub habit_logs: Vec<HabitLog>,
    #[serde(default)]
    pub focus_sessions: Vec<FocusSession>,
    #[serde(default)]
    pub daily_reviews: Vec<DailyReview>,
    #[serde(default)]
    pub schedule_items: Vec<ScheduleItem>,
    #[serde(default)]
    pub schedule_exceptions: Vec<ScheduleException>,
    #[serde(default)]
    pub terms: Vec<Term>,
    #[serde(default)]
    pub academic_events: Vec<AcademicEvent>,
    #[serde(default)]
    pub dismissed_conflicts: Vec<DismissedConflict>,
}
fn default_bridge_port() -> u16 {
    8765
}
fn default_day_before_pct() -> u32 {
    50
}
fn default_prep_window_days() -> u32 {
    5
}
fn default_non_study_pct() -> u32 {
    60
}
fn default_heavy_pct() -> u32 {
    85
}
fn default_calibration_true() -> bool {
    true
}
fn default_calibration_samples() -> u32 {
    5
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Settings {
    pub timezone: String,
    pub max_tasks: u32,
    pub max_minutes: u32,
    pub work_start: u8,
    pub work_end: u8,
    #[serde(default)]
    pub bridge_enabled: bool,
    #[serde(default = "default_bridge_port")]
    pub bridge_port: u16,
    #[serde(default)]
    pub bridge_token: String,
    #[serde(default)]
    pub autostart: bool,
    #[serde(default)]
    pub onboarded: bool,
    #[serde(default)]
    pub exam_day_new_task_cap: u32,
    #[serde(default = "default_day_before_pct")]
    pub day_before_exam_capacity_pct: u32,
    #[serde(default = "default_prep_window_days")]
    pub prep_window_days: u32,
    #[serde(default = "default_non_study_pct")]
    pub non_study_cap_in_prep_pct: u32,
    #[serde(default = "default_heavy_pct")]
    pub heavy_day_threshold_pct: u32,
    #[serde(default = "default_calibration_true")]
    pub calibrate_estimates: bool,
    #[serde(default = "default_calibration_samples")]
    pub calibration_min_samples: u32,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            timezone: "UTC".into(),
            max_tasks: 8,
            max_minutes: 360,
            work_start: 9,
            work_end: 18,
            bridge_enabled: false,
            bridge_port: default_bridge_port(),
            bridge_token: String::new(),
            autostart: false,
            onboarded: false,
            exam_day_new_task_cap: 0,
            day_before_exam_capacity_pct: default_day_before_pct(),
            prep_window_days: default_prep_window_days(),
            non_study_cap_in_prep_pct: default_non_study_pct(),
            heavy_day_threshold_pct: default_heavy_pct(),
            calibrate_estimates: default_calibration_true(),
            calibration_min_samples: default_calibration_samples(),
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
    pub recurrence: Option<String>,
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
    #[serde(default)]
    pub recurrence_rule: Option<String>,
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
    StartFocus {
        task_id: Option<String>,
        planned_min: u32,
        kind: FocusKind,
    },
    StopFocus {
        id: String,
    },
    CreateHabit {
        name: String,
        cadence_days: u32,
    },
    LogHabit {
        habit_id: String,
        day: Option<NaiveDate>,
    },
    DeleteHabit {
        id: String,
    },
    SaveDailyReview {
        day: Option<NaiveDate>,
        note: String,
    },
    CreateScheduleItem {
        title: String,
        kind: ScheduleKind,
        weekdays: u32,
        start_time: String,
        end_time: String,
        term_id: Option<String>,
        location: Option<String>,
        notes: Option<String>,
        subject_id: Option<String>,
        remind_before_min: Option<u32>,
    },
    UpdateScheduleItem {
        id: String,
        title: String,
        kind: ScheduleKind,
        weekdays: u32,
        start_time: String,
        end_time: String,
        location: Option<String>,
        notes: Option<String>,
        subject_id: Option<String>,
        remind_before_min: Option<u32>,
        active: bool,
    },
    DeleteScheduleItem {
        id: String,
    },
    ListScheduleItems,
    ListOccurrences {
        from: NaiveDate,
        to: NaiveDate,
    },
    SkipOccurrence {
        item_id: String,
        date: NaiveDate,
    },
    MoveOccurrence {
        item_id: String,
        date: NaiveDate,
        new_start: String,
        new_end: Option<String>,
    },
    ClearException {
        item_id: String,
        date: NaiveDate,
    },
    CreateTerm {
        name: String,
        start_date: NaiveDate,
        end_date: NaiveDate,
    },
    UpdateTerm {
        id: String,
        name: String,
        start_date: NaiveDate,
        end_date: NaiveDate,
    },
    ArchiveTerm {
        id: String,
    },
    SetActiveTerm {
        id: String,
    },
    CopyScheduleToTerm {
        from_term_id: String,
        to_term_id: String,
    },
    CreateAcademicEvent {
        term_id: Option<String>,
        kind: AcademicKind,
        title: String,
        subject_id: Option<String>,
        start_at: DateTime<Utc>,
        end_at: DateTime<Utc>,
        all_day: bool,
        prep_days: u32,
        suppress_classes: Option<SuppressClasses>,
        notes: Option<String>,
    },
    UpdateAcademicEvent {
        id: String,
        kind: AcademicKind,
        title: String,
        subject_id: Option<String>,
        start_at: DateTime<Utc>,
        end_at: DateTime<Utc>,
        all_day: bool,
        prep_days: u32,
        suppress_classes: SuppressClasses,
        notes: Option<String>,
    },
    DeleteAcademicEvent {
        id: String,
    },
    ListAcademicEvents {
        from: NaiveDate,
        to: NaiveDate,
    },
    GetDayStatus {
        date: NaiveDate,
    },
    ListConflicts {
        from: NaiveDate,
        to: NaiveDate,
    },
    ApplySuggestion {
        task_id: String,
        date: NaiveDate,
        action: SuggestionAction,
        dry_run: bool,
    },
    DismissConflict {
        task_id: String,
        date: NaiveDate,
        rule_id: String,
    },
    PreviewPrepPlan {
        event_id: String,
    },
    ApplyPrepPlan {
        event_id: String,
        dry_run: bool,
    },
    WeekLoad {
        from: NaiveDate,
        to: NaiveDate,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TaskScheduled {
    pub task: Task,
    pub overlap: Vec<Occurrence>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Capacity {
    pub count: usize,
    pub minutes: u32,
    pub over_count: bool,
    pub over_minutes: bool,
    pub available_minutes: u32,
    pub schedule_minutes: u32,
    pub cap_minutes: u32,
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
    pub focus_sessions: Vec<FocusSession>,
    pub active_focus: Option<FocusSession>,
    pub habits: Vec<Habit>,
    pub habit_streaks: Vec<HabitStreak>,
    pub daily_reviews: Vec<DailyReview>,
    pub weekly_stats: WeeklyStats,
    pub schedule_items: Vec<ScheduleItem>,
    pub today_occurrences: Vec<Occurrence>,
    pub week_occurrences: Vec<Occurrence>,
    pub terms: Vec<Term>,
    pub active_term: Option<Term>,
    pub academic_events: Vec<AcademicEvent>,
    pub day_status: DayStatus,
    pub upcoming_exams: Vec<AcademicEvent>,
    pub conflicts: Vec<Conflict>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum Response {
    Pong(String),
    Snapshot(Box<Snapshot>),
    Parsed(ParsedTask),
    Task(Task),
    TaskScheduled(TaskScheduled),
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
    Focus(FocusSession),
    Habit(Habit),
    HabitLog(HabitLog),
    Review(DailyReview),
    ScheduleItem(ScheduleItem),
    ScheduleItems(Vec<ScheduleItem>),
    Occurrences(Vec<Occurrence>),
    Term(Term),
    Terms(Vec<Term>),
    AcademicEvent(AcademicEvent),
    AcademicEvents(Vec<AcademicEvent>),
    DayStatus(DayStatus),
    Conflicts(Vec<Conflict>),
    PrepPlan(PrepPlan),
    WeekLoad(Vec<WeekLoadEntry>),
    Undone(UndoOutcome),
}
