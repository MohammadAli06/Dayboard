pub mod model;
pub mod parser;
pub mod planner;
use chrono::{DateTime, NaiveDate, TimeZone, Utc};
use chrono_tz::Tz;
use model::*;
use planner::Planner;
use regex::Regex;
use rusqlite::{params, Connection, Transaction};
use rusqlite_migration::{Migrations, M};
use serde::de::DeserializeOwned;
use std::{path::Path, sync::LazyLock, time::Duration};

static ISSUE_URL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^(?:https?://)?(?:www\.)?github\.com/([A-Za-z0-9_.-]+)/([A-Za-z0-9_.-]+)/issues/(\d+)(?:[/?#].*)?$").unwrap()
});

fn validate_text(value: &str, max: usize, label: &str) -> Result<String> {
    let value = value.trim();
    if value.is_empty() || value.len() > max {
        return Err(Error::Validation(format!(
            "Enter a {label} between 1 and {max} bytes"
        )));
    }
    Ok(value.to_string())
}

fn clean_optional(value: Option<String>) -> Option<String> {
    value
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

fn validate_note(value: String) -> Result<String> {
    if value.len() > 100_000 {
        Err(Error::Validation(
            "Notes are limited to 100000 bytes".into(),
        ))
    } else {
        Ok(value)
    }
}

fn ensure_live_entity(deleted_at: Option<DateTime<Utc>>, label: &str) -> Result<()> {
    if deleted_at.is_some() {
        Err(Error::Validation(format!(
            "Restore this {label} before editing it"
        )))
    } else {
        Ok(())
    }
}

fn validate_confidence(confidence: u8) -> Result<u8> {
    if (1..=5).contains(&confidence) {
        Ok(confidence)
    } else {
        Err(Error::Validation(
            "Confidence must be between 1 and 5".into(),
        ))
    }
}

fn review_interval(stage: u8, confidence: u8) -> i64 {
    let base: i64 = match stage.clamp(1, 4) {
        1 => 1,
        2 => 3,
        3 => 7,
        _ => 21,
    };
    let adjusted = match confidence {
        1 => (base + 1) / 2,
        2 => base - 1,
        3 => base,
        4 => base + 2,
        _ => base + 7,
    };
    adjusted.max(1)
}

fn review_date(now: DateTime<Utc>, tz: Tz, stage: u8, confidence: u8) -> NaiveDate {
    now.with_timezone(&tz).date_naive() + chrono::Duration::days(review_interval(stage, confidence))
}

fn local_five_pm(day: NaiveDate, tz: Tz) -> Option<DateTime<Utc>> {
    tz.from_local_datetime(&day.and_hms_opt(17, 0, 0).unwrap())
        .earliest()
        .map(|at| at.with_timezone(&Utc))
}

fn revision_task(problem: &DsaProblem, day: NaiveDate, tz: Tz, now: DateTime<Utc>) -> Task {
    Task {
        id: ulid::Ulid::new().to_string(),
        title: format!("Revise: {}", problem.title),
        notes: format!("DSA revision for {} ({})", problem.title, problem.topic),
        status: Status::Todo,
        priority: 3,
        energy: Energy::Low,
        est_min: 30,
        actual_min: 0,
        due_at: local_five_pm(day, tz),
        planned_day: Some(day),
        scheduled_start: None,
        scheduled_end: None,
        parent_id: None,
        project_id: None,
        goal_id: None,
        dsa_problem_id: Some(problem.id.clone()),
        tags: vec!["!revise".into()],
        carried_count: 0,
        created_at: now,
        completed_at: None,
        deleted_at: None,
        reminded_at: None,
    }
}

fn study_tasks(deadline: &Deadline, today: NaiveDate, tz: Tz, now: DateTime<Utc>) -> Vec<Task> {
    let days = (deadline.due_date - today).num_days();
    let window = if days <= 0 { 1 } else { days.min(14) };
    let per_day = deadline
        .study_min
        .max(1)
        .div_ceil(window as u32)
        .clamp(1, 1440);
    (0..window)
        .map(|offset| {
            let day = today + chrono::Duration::days(offset);
            Task {
                id: ulid::Ulid::new().to_string(),
                title: format!("Study: {}", deadline.title),
                notes: format!(
                    "Study block for {} (due {})",
                    deadline.title, deadline.due_date
                ),
                status: Status::Todo,
                priority: 3,
                energy: Energy::Low,
                est_min: per_day,
                actual_min: 0,
                due_at: local_five_pm(day, tz),
                planned_day: Some(day),
                scheduled_start: None,
                scheduled_end: None,
                parent_id: None,
                project_id: None,
                goal_id: None,
                dsa_problem_id: None,
                tags: vec!["!study".into()],
                carried_count: 0,
                created_at: now,
                completed_at: None,
                deleted_at: None,
                reminded_at: None,
            }
        })
        .collect()
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Validation(String),
    #[error("Database error: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("Data error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Migration error: {0}")]
    Migration(#[from] rusqlite_migration::Error),
}
pub type Result<T> = std::result::Result<T, Error>;
pub struct Core {
    conn: Connection,
}
impl Core {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        Self::from_connection(Connection::open(path)?)
    }
    pub fn memory() -> Result<Self> {
        Self::from_connection(Connection::open_in_memory()?)
    }
    fn from_connection(mut conn: Connection) -> Result<Self> {
        conn.busy_timeout(Duration::from_secs(5))?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
        Migrations::new(vec![
            M::up(include_str!("schema.sql")),
            M::up(include_str!("schema_v2.sql")),
            M::up(include_str!("schema_v3.sql")),
        ])
        .to_latest(&mut conn)?;
        Ok(Self { conn })
    }
    pub fn settings(&self) -> Result<Settings> {
        let raw: String = self.conn.query_row(
            "SELECT value FROM settings WHERE key='preferences'",
            [],
            |r| r.get(0),
        )?;
        Ok(serde_json::from_str(&raw)?)
    }
    pub fn today_count(&self, now: DateTime<Utc>) -> Result<usize> {
        let settings = self.settings()?;
        let tz: Tz = settings
            .timezone
            .parse()
            .map_err(|_| Error::Validation("Invalid IANA timezone".into()))?;
        let today = now.with_timezone(&tz).date_naive();
        Ok(self.conn.query_row(
            "SELECT count(*) FROM tasks WHERE deleted=0 AND planned_day=?1 AND json_extract(data,'$.status') NOT IN ('done','dropped')",
            [today.to_string()],
            |r| r.get::<_, i64>(0),
        )? as usize)
    }
    pub fn tasks(&self) -> Result<Vec<Task>> {
        let mut stmt = self
            .conn
            .prepare("SELECT data FROM tasks ORDER BY created_at DESC")?;
        let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
        rows.map(|r| Ok(serde_json::from_str(&r?)?)).collect()
    }
    fn task(&self, id: &str) -> Result<Task> {
        let raw = self
            .conn
            .query_row("SELECT data FROM tasks WHERE id=?1", [id], |r| {
                r.get::<_, String>(0)
            })
            .map_err(|e| {
                if matches!(e, rusqlite::Error::QueryReturnedNoRows) {
                    Error::Validation("Task not found".into())
                } else {
                    e.into()
                }
            })?;
        Ok(serde_json::from_str(&raw)?)
    }
    fn load_entities<T: DeserializeOwned>(&self, sql: &str) -> Result<Vec<T>> {
        let mut stmt = self.conn.prepare(sql)?;
        let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
        rows.map(|r| Ok(serde_json::from_str(&r?)?)).collect()
    }
    fn load_one<T: DeserializeOwned>(&self, sql: &str, id: &str, label: &str) -> Result<T> {
        let raw = self
            .conn
            .query_row(sql, [id], |r| r.get::<_, String>(0))
            .map_err(|e| {
                if matches!(e, rusqlite::Error::QueryReturnedNoRows) {
                    Error::Validation(format!("{label} not found"))
                } else {
                    e.into()
                }
            })?;
        Ok(serde_json::from_str(&raw)?)
    }
    pub fn projects(&self) -> Result<Vec<Project>> {
        self.load_entities(
            "SELECT data FROM projects WHERE deleted=0 ORDER BY json_extract(data,'$.created_at')",
        )
    }
    pub fn goals(&self) -> Result<Vec<Goal>> {
        self.load_entities(
            "SELECT data FROM goals WHERE deleted=0 ORDER BY json_extract(data,'$.created_at')",
        )
    }
    pub fn templates(&self) -> Result<Vec<Template>> {
        self.load_entities(
            "SELECT data FROM templates WHERE deleted=0 ORDER BY json_extract(data,'$.created_at')",
        )
    }
    pub fn saved_filters(&self) -> Result<Vec<SavedFilter>> {
        self.load_entities("SELECT data FROM saved_filters ORDER BY created_at")
    }
    pub fn dsa_problems(&self) -> Result<Vec<DsaProblem>> {
        self.load_entities(
            "SELECT data FROM dsa_problems WHERE deleted=0 ORDER BY json_extract(data,'$.created_at')",
        )
    }
    pub fn subjects(&self) -> Result<Vec<Subject>> {
        self.load_entities(
            "SELECT data FROM subjects WHERE deleted=0 ORDER BY json_extract(data,'$.created_at')",
        )
    }
    pub fn syllabus_items(&self) -> Result<Vec<SyllabusItem>> {
        self.load_entities(
            "SELECT data FROM syllabus_items WHERE deleted=0 ORDER BY json_extract(data,'$.created_at')",
        )
    }
    pub fn deadlines(&self) -> Result<Vec<Deadline>> {
        self.load_entities(
            "SELECT data FROM deadlines WHERE deleted=0 ORDER BY json_extract(data,'$.due_date')",
        )
    }
    fn dsa_problem(&self, id: &str) -> Result<DsaProblem> {
        self.load_one(
            "SELECT data FROM dsa_problems WHERE id=?1",
            id,
            "DSA problem",
        )
    }
    fn subject(&self, id: &str) -> Result<Subject> {
        self.load_one("SELECT data FROM subjects WHERE id=?1", id, "Subject")
    }
    fn syllabus_item(&self, id: &str) -> Result<SyllabusItem> {
        self.load_one(
            "SELECT data FROM syllabus_items WHERE id=?1",
            id,
            "Syllabus item",
        )
    }
    fn deadline(&self, id: &str) -> Result<Deadline> {
        self.load_one("SELECT data FROM deadlines WHERE id=?1", id, "Deadline")
    }
    fn project(&self, id: &str) -> Result<Project> {
        self.load_one("SELECT data FROM projects WHERE id=?1", id, "Project")
    }
    fn goal(&self, id: &str) -> Result<Goal> {
        self.load_one("SELECT data FROM goals WHERE id=?1", id, "Goal")
    }
    fn template(&self, id: &str) -> Result<Template> {
        self.load_one("SELECT data FROM templates WHERE id=?1", id, "Template")
    }
    fn saved_filter(&self, id: &str) -> Result<SavedFilter> {
        self.load_one("SELECT data FROM saved_filters WHERE id=?1", id, "Filter")
    }
    pub fn execute(
        &mut self,
        command: Command,
        actor: Actor,
        now: DateTime<Utc>,
    ) -> Result<Response> {
        let settings = self.settings()?;
        let tz: Tz = settings
            .timezone
            .parse()
            .map_err(|_| Error::Validation("Invalid IANA timezone".into()))?;
        let today = now.with_timezone(&tz).date_naive();
        match command {
            Command::PingDb => Ok(Response::Pong(self.conn.query_row(
                "SELECT sqlite_version()",
                [],
                |r| r.get(0),
            )?)),
            Command::Snapshot { search } => {
                let all = self.tasks()?;
                let mut tasks = all
                    .iter()
                    .filter(|t| t.deleted_at.is_none())
                    .cloned()
                    .collect::<Vec<_>>();
                let tags = {
                    let mut s = self.conn.prepare("SELECT name FROM tags ORDER BY name")?;
                    let rows = s.query_map([], |r| r.get(0))?;
                    rows.collect::<std::result::Result<Vec<String>, _>>()?
                };
                if let Some(q) = search.filter(|q| !q.trim().is_empty()) {
                    let query = q
                        .split_whitespace()
                        .map(|s| format!("\"{}\"*", s.replace('"', "\"\"")))
                        .collect::<Vec<_>>()
                        .join(" AND ");
                    let mut stmt = self
                        .conn
                        .prepare("SELECT id FROM task_search WHERE task_search MATCH ?1")?;
                    let ids = stmt
                        .query_map([query], |r| r.get::<_, String>(0))?
                        .collect::<std::result::Result<Vec<_>, _>>()?;
                    tasks.retain(|t| ids.contains(&t.id));
                }
                let projects = self.projects()?;
                let goals = self.goals()?;
                let templates = self.templates()?;
                let saved_filters = self.saved_filters()?;
                let dsa_problems = self.dsa_problems()?;
                let subjects = self.subjects()?;
                let syllabus_items = self.syllabus_items()?;
                let deadlines = self.deadlines()?;
                let mut topics: std::collections::BTreeMap<String, (usize, usize, u32)> =
                    std::collections::BTreeMap::new();
                for problem in &dsa_problems {
                    let entry = topics.entry(problem.topic.clone()).or_insert((0, 0, 0));
                    entry.0 += 1;
                    if problem.status == DsaStatus::Solved {
                        entry.1 += 1;
                        entry.2 += problem.confidence as u32;
                    }
                }
                let dsa_topics = topics
                    .into_iter()
                    .map(|(topic, (total, solved, sum))| TopicCoverage {
                        topic,
                        total,
                        solved,
                        average_confidence: if solved == 0 {
                            0.0
                        } else {
                            (sum as f64 / solved as f64 * 10.0).round() / 10.0
                        },
                    })
                    .collect();
                let goal_progress = goals
                    .iter()
                    .map(|goal| {
                        let linked = all.iter().filter(|t| {
                            t.deleted_at.is_none() && t.goal_id.as_deref() == Some(goal.id.as_str())
                        });
                        GoalProgress {
                            goal_id: goal.id.clone(),
                            total: linked.clone().count(),
                            done: linked.filter(|t| t.status == Status::Done).count(),
                        }
                    })
                    .collect();
                Ok(Response::Snapshot(Snapshot {
                    capacity: planner::capacity(&all, today, &settings),
                    tasks,
                    settings,
                    today,
                    tags,
                    projects,
                    goals,
                    goal_progress,
                    templates,
                    saved_filters,
                    dsa_problems,
                    dsa_topics,
                    subjects,
                    syllabus_items,
                    deadlines,
                }))
            }
            Command::ParseQuickAdd { text } => Ok(Response::Parsed(parser::parse(&text, now, tz)?)),
            Command::CreateTask { text, parent_id } => {
                let p = parser::parse(&text, now, tz)?;
                if let Some(id) = &parent_id {
                    if self.task(id)?.deleted_at.is_some() {
                        return Err(Error::Validation("Parent was deleted".into()));
                    }
                }
                let task = Task {
                    id: ulid::Ulid::new().to_string(),
                    title: p.title,
                    notes: String::new(),
                    status: if p.planned_day.is_some() {
                        Status::Todo
                    } else {
                        Status::Inbox
                    },
                    priority: p.priority,
                    energy: p.energy,
                    est_min: p.est_min,
                    actual_min: 0,
                    due_at: p.due_at,
                    planned_day: p.planned_day,
                    scheduled_start: None,
                    scheduled_end: None,
                    parent_id,
                    project_id: None,
                    goal_id: None,
                    dsa_problem_id: None,
                    tags: p.tags,
                    carried_count: 0,
                    created_at: now,
                    completed_at: None,
                    deleted_at: None,
                    reminded_at: None,
                };
                self.save(None, &task, actor, "create_task", now)?;
                Ok(Response::Task(task))
            }
            Command::UpdateTask(edit) => {
                if edit.title.trim().is_empty()
                    || edit.title.len() > 500
                    || edit.notes.len() > 100_000
                    || !(1..=4).contains(&edit.priority)
                    || edit.est_min == 0
                    || edit.est_min > 1440
                    || edit.actual_min > 1_000_000
                {
                    return Err(Error::Validation(
                        "Check title, priority and minutes (estimate 1–1440)".into(),
                    ));
                }
                let before = self.task(&edit.id)?;
                Self::ensure_live(&before)?;
                let mut parent = edit.parent_id.clone();
                let mut visited = std::collections::HashSet::new();
                while let Some(id) = parent {
                    if id == edit.id || !visited.insert(id.clone()) {
                        return Err(Error::Validation("Subtasks cannot form a cycle".into()));
                    }
                    let p = self.task(&id)?;
                    Self::ensure_live(&p)?;
                    parent = p.parent_id;
                }
                let mut task = before.clone();
                if let Some(id) = &edit.project_id {
                    ensure_live_entity(self.project(id)?.deleted_at, "project")?;
                }
                if let Some(id) = &edit.goal_id {
                    ensure_live_entity(self.goal(id)?.deleted_at, "goal")?;
                }
                task.title = edit.title.trim().into();
                task.notes = edit.notes;
                task.status = edit.status;
                task.priority = edit.priority;
                task.energy = edit.energy;
                task.est_min = edit.est_min;
                task.actual_min = edit.actual_min;
                task.tags = edit.tags;
                task.tags.sort();
                task.tags.dedup();
                task.parent_id = edit.parent_id;
                task.project_id = edit.project_id;
                task.goal_id = edit.goal_id;
                task.planned_day = edit.planned_day;
                task.due_at = edit
                    .due_local
                    .as_deref()
                    .filter(|s| !s.is_empty())
                    .map(|s| parser::local_datetime(s, tz))
                    .transpose()?;
                task.scheduled_start = edit
                    .scheduled_local
                    .as_deref()
                    .filter(|s| !s.is_empty())
                    .map(|s| parser::local_datetime(s, tz))
                    .transpose()?;
                task.scheduled_end = task
                    .scheduled_start
                    .map(|start| start + chrono::Duration::minutes(task.est_min as i64));
                if let Some(start) = task.scheduled_start {
                    task.planned_day = Some(start.with_timezone(&tz).date_naive());
                    if task.status == Status::Inbox {
                        task.status = Status::Todo;
                    }
                }
                if task.due_at != before.due_at || task.scheduled_start != before.scheduled_start {
                    task.reminded_at = None;
                }
                task.completed_at = if task.status == Status::Done {
                    before.completed_at.or(Some(now))
                } else {
                    None
                };
                if task.status == Status::Inbox && task.planned_day.is_some() {
                    task.status = Status::Todo;
                }
                self.save(Some(&before), &task, actor, "update_task", now)?;
                Ok(Response::Task(task))
            }
            Command::CompleteTask { id, completed } => {
                let before = self.task(&id)?;
                Self::ensure_live(&before)?;
                let mut task = before.clone();
                task.status = if completed {
                    Status::Done
                } else if task.planned_day.is_some() {
                    Status::Todo
                } else {
                    Status::Inbox
                };
                task.completed_at = completed.then_some(now);
                self.save(Some(&before), &task, actor, "complete_task", now)?;
                Ok(Response::Task(task))
            }
            Command::DeleteTask { id, dry_run } => {
                let before = self.task(&id)?;
                let mut task = before.clone();
                task.deleted_at = Some(now);
                if !dry_run {
                    self.save(Some(&before), &task, actor, "delete_task", now)?;
                }
                Ok(Response::Task(task))
            }
            Command::RestoreTask { id } => {
                let before = self.task(&id)?;
                let mut task = before.clone();
                task.deleted_at = None;
                self.save(Some(&before), &task, actor, "restore_task", now)?;
                Ok(Response::Task(task))
            }
            Command::RescheduleTask {
                id,
                local_start,
                day,
                dry_run,
            } => {
                let before = self.task(&id)?;
                Self::ensure_live(&before)?;
                let mut task = before.clone();
                task.scheduled_start = local_start
                    .as_deref()
                    .map(|s| parser::local_datetime(s, tz))
                    .transpose()?;
                task.scheduled_end = task
                    .scheduled_start
                    .map(|s| s + chrono::Duration::minutes(task.est_min as i64));
                task.planned_day = task
                    .scheduled_start
                    .map(|s| s.with_timezone(&tz).date_naive())
                    .or(day);
                if task.status == Status::Inbox && task.planned_day.is_some() {
                    task.status = Status::Todo;
                }
                task.reminded_at = None;
                if !dry_run {
                    self.save(Some(&before), &task, actor, "reschedule_task", now)?;
                }
                Ok(Response::Task(task))
            }
            Command::Rollover { dry_run } => {
                let tasks = self.tasks()?;
                let tx = self.conn.transaction()?;
                let mut changed = vec![];
                for before in tasks.iter().filter(|t| {
                    t.deleted_at.is_none()
                        && t.status.active()
                        && t.planned_day.is_some_and(|d| d < today)
                }) {
                    let mut task = before.clone();
                    task.carried_count = task
                        .carried_count
                        .saturating_add((today - task.planned_day.unwrap()).num_days() as u32);
                    task.planned_day = Some(today);
                    task.scheduled_start = None;
                    task.scheduled_end = None;
                    if !dry_run {
                        Self::persist(&tx, &task)?;
                        Self::event(
                            &tx,
                            actor,
                            "task",
                            "rollover",
                            &task.id,
                            Some(before),
                            Some(&task),
                            now,
                        )?;
                    }
                    changed.push(task);
                }
                tx.commit()?;
                Ok(Response::Tasks(changed))
            }
            Command::SaveSettings(s) => {
                if s.timezone.parse::<Tz>().is_err()
                    || s.max_tasks == 0
                    || s.max_tasks > 1000
                    || s.max_minutes == 0
                    || s.max_minutes > 1440
                    || s.work_start >= s.work_end
                    || s.work_end > 24
                {
                    return Err(Error::Validation(
                        "Use an IANA timezone, positive caps and a valid workday".into(),
                    ));
                }
                let tx = self.conn.transaction()?;
                tx.execute(
                    "UPDATE settings SET value=?1 WHERE key='preferences'",
                    [serde_json::to_string(&s)?],
                )?;
                Self::event(
                    &tx,
                    actor,
                    "settings",
                    "save_settings",
                    "preferences",
                    Some(&settings),
                    Some(&s),
                    now,
                )?;
                tx.commit()?;
                Ok(Response::Settings(s))
            }
            Command::PlanDay { day, energy } => Ok(Response::Tasks(planner::RulePlanner.suggest(
                &self.tasks()?,
                day,
                &settings,
                energy,
            ))),
            Command::CreateProject {
                name,
                kind,
                repo_path,
                branch,
            } => {
                let project = Project {
                    id: ulid::Ulid::new().to_string(),
                    name: validate_text(&name, 200, "project name")?,
                    kind,
                    repo_path: clean_optional(repo_path),
                    branch: clean_optional(branch),
                    status: ProjectStatus::Active,
                    created_at: now,
                    deleted_at: None,
                };
                self.save_project(None, &project, actor, "create_project", now)?;
                Ok(Response::Project(project))
            }
            Command::UpdateProject {
                id,
                name,
                kind,
                repo_path,
                branch,
                status,
            } => {
                let before = self.project(&id)?;
                ensure_live_entity(before.deleted_at, "project")?;
                let mut project = before.clone();
                project.name = validate_text(&name, 200, "project name")?;
                project.kind = kind;
                project.repo_path = clean_optional(repo_path);
                project.branch = clean_optional(branch);
                project.status = status;
                self.save_project(Some(&before), &project, actor, "update_project", now)?;
                Ok(Response::Project(project))
            }
            Command::DeleteProject { id } => {
                let before = self.project(&id)?;
                ensure_live_entity(before.deleted_at, "project")?;
                let mut project = before.clone();
                project.deleted_at = Some(now);
                self.save_project(Some(&before), &project, actor, "delete_project", now)?;
                Ok(Response::Project(project))
            }
            Command::CreateGoal {
                title,
                horizon,
                target_date,
                note,
            } => {
                let goal = Goal {
                    id: ulid::Ulid::new().to_string(),
                    title: validate_text(&title, 300, "goal title")?,
                    horizon,
                    target_date,
                    note: validate_note(note)?,
                    created_at: now,
                    deleted_at: None,
                };
                self.save_goal(None, &goal, actor, "create_goal", now)?;
                Ok(Response::Goal(goal))
            }
            Command::UpdateGoal {
                id,
                title,
                horizon,
                target_date,
                note,
            } => {
                let before = self.goal(&id)?;
                ensure_live_entity(before.deleted_at, "goal")?;
                let mut goal = before.clone();
                goal.title = validate_text(&title, 300, "goal title")?;
                goal.horizon = horizon;
                goal.target_date = target_date;
                goal.note = validate_note(note)?;
                self.save_goal(Some(&before), &goal, actor, "update_goal", now)?;
                Ok(Response::Goal(goal))
            }
            Command::DeleteGoal { id } => {
                let before = self.goal(&id)?;
                ensure_live_entity(before.deleted_at, "goal")?;
                let mut goal = before.clone();
                goal.deleted_at = Some(now);
                self.save_goal(Some(&before), &goal, actor, "delete_goal", now)?;
                Ok(Response::Goal(goal))
            }
            Command::CreateTemplate {
                name,
                title,
                notes,
                priority,
                energy,
                est_min,
                tags,
                project_id,
            } => {
                if title.trim().is_empty()
                    || title.len() > 500
                    || !(1..=4).contains(&priority)
                    || !(1..=1440).contains(&est_min)
                {
                    return Err(Error::Validation(
                        "Check template title, priority and estimate (1–1440)".into(),
                    ));
                }
                let mut tags = tags;
                tags.sort();
                tags.dedup();
                Self::validate_tags(&tags)?;
                if let Some(id) = &project_id {
                    ensure_live_entity(self.project(id)?.deleted_at, "project")?;
                }
                let template = Template {
                    id: ulid::Ulid::new().to_string(),
                    name: validate_text(&name, 200, "template name")?,
                    title: title.trim().into(),
                    notes: validate_note(notes)?,
                    priority,
                    energy,
                    est_min,
                    tags,
                    project_id,
                    created_at: now,
                    deleted_at: None,
                };
                self.save_template(None, &template, actor, "create_template", now)?;
                Ok(Response::Template(template))
            }
            Command::CreateTaskFromTemplate {
                template_id,
                parent_id,
            } => {
                let template = self.template(&template_id)?;
                ensure_live_entity(template.deleted_at, "template")?;
                if let Some(id) = &template.project_id {
                    ensure_live_entity(self.project(id)?.deleted_at, "project")?;
                }
                let task = Task {
                    id: ulid::Ulid::new().to_string(),
                    title: template.title.clone(),
                    notes: template.notes.clone(),
                    status: Status::Inbox,
                    priority: template.priority,
                    energy: template.energy,
                    est_min: template.est_min,
                    actual_min: 0,
                    due_at: None,
                    planned_day: None,
                    scheduled_start: None,
                    scheduled_end: None,
                    parent_id,
                    project_id: template.project_id.clone(),
                    goal_id: None,
                    dsa_problem_id: None,
                    tags: template.tags.clone(),
                    carried_count: 0,
                    created_at: now,
                    completed_at: None,
                    deleted_at: None,
                    reminded_at: None,
                };
                self.save(None, &task, actor, "create_task", now)?;
                Ok(Response::Task(task))
            }
            Command::DeleteTemplate { id } => {
                let before = self.template(&id)?;
                ensure_live_entity(before.deleted_at, "template")?;
                let mut template = before.clone();
                template.deleted_at = Some(now);
                self.save_template(Some(&before), &template, actor, "delete_template", now)?;
                Ok(Response::Template(template))
            }
            Command::ImportIssue { url } => {
                let trimmed = url.trim();
                let caps = ISSUE_URL.captures(trimmed).ok_or_else(|| {
                    Error::Validation(
                        "Paste a GitHub issue URL like https://github.com/owner/repo/issues/12"
                            .into(),
                    )
                })?;
                let owner = caps[1].to_string();
                let repo = caps[2].to_string();
                let number = caps[3].to_string();
                let project_id = self
                    .projects()?
                    .into_iter()
                    .find(|project| {
                        project
                            .repo_path
                            .as_deref()
                            .and_then(|path| Path::new(path).file_name())
                            .and_then(|name| name.to_str())
                            .is_some_and(|name| name.eq_ignore_ascii_case(repo.as_str()))
                    })
                    .map(|project| project.id);
                let task = Task {
                    id: ulid::Ulid::new().to_string(),
                    title: format!("{owner}/{repo}#{number}"),
                    notes: format!("GitHub issue: {trimmed}"),
                    status: Status::Inbox,
                    priority: 3,
                    energy: Energy::Low,
                    est_min: 25,
                    actual_min: 0,
                    due_at: None,
                    planned_day: None,
                    scheduled_start: None,
                    scheduled_end: None,
                    parent_id: None,
                    project_id,
                    goal_id: None,
                    dsa_problem_id: None,
                    tags: vec!["!issue".into()],
                    carried_count: 0,
                    created_at: now,
                    completed_at: None,
                    deleted_at: None,
                    reminded_at: None,
                };
                self.save(None, &task, actor, "import_issue", now)?;
                Ok(Response::Task(task))
            }
            Command::SaveFilter { name, view, query } => {
                if query.len() > 500 {
                    return Err(Error::Validation("Filter query is too long".into()));
                }
                let filter = SavedFilter {
                    id: ulid::Ulid::new().to_string(),
                    name: validate_text(&name, 100, "filter name")?,
                    view: validate_text(&view, 40, "view")?,
                    query: query.trim().to_string(),
                    created_at: now,
                };
                self.save_filter(None, &filter, actor, "save_filter", now)?;
                Ok(Response::Filter(filter))
            }
            Command::DeleteFilter { id } => {
                let filter = self.saved_filter(&id)?;
                self.delete_filter(&filter, actor, now)?;
                Ok(Response::Filter(filter))
            }
            Command::CreateDsaProblem {
                title,
                url,
                topic,
                difficulty,
                platform,
                notes,
            } => {
                let problem = DsaProblem {
                    id: ulid::Ulid::new().to_string(),
                    title: validate_text(&title, 500, "problem title")?,
                    url: clean_optional(url),
                    topic: validate_text(&topic, 80, "topic")?,
                    difficulty,
                    platform: clean_optional(platform),
                    status: DsaStatus::Todo,
                    confidence: 3,
                    solved_at: None,
                    next_review_at: None,
                    review_stage: 0,
                    notes: validate_note(notes)?,
                    created_at: now,
                    deleted_at: None,
                };
                self.save_dsa_problem(None, &problem, actor, "create_dsa_problem", now)?;
                Ok(Response::DsaProblem(problem))
            }
            Command::UpdateDsaProblem {
                id,
                title,
                url,
                topic,
                difficulty,
                platform,
                notes,
            } => {
                let before = self.dsa_problem(&id)?;
                ensure_live_entity(before.deleted_at, "problem")?;
                let mut problem = before.clone();
                problem.title = validate_text(&title, 500, "problem title")?;
                problem.url = clean_optional(url);
                problem.topic = validate_text(&topic, 80, "topic")?;
                problem.difficulty = difficulty;
                problem.platform = clean_optional(platform);
                problem.notes = validate_note(notes)?;
                self.save_dsa_problem(Some(&before), &problem, actor, "update_dsa_problem", now)?;
                Ok(Response::DsaProblem(problem))
            }
            Command::SolveDsaProblem { id, confidence } => {
                let before = self.dsa_problem(&id)?;
                ensure_live_entity(before.deleted_at, "problem")?;
                if before.status == DsaStatus::Solved {
                    return Err(Error::Validation(
                        "This problem is already solved; review it instead".into(),
                    ));
                }
                let confidence = validate_confidence(confidence)?;
                let mut problem = before.clone();
                problem.status = DsaStatus::Solved;
                problem.confidence = confidence;
                problem.solved_at = Some(now);
                problem.review_stage = 1;
                let day = review_date(now, tz, 1, confidence);
                problem.next_review_at = Some(day);
                let task = revision_task(&problem, day, tz, now);
                self.save_dsa_review(&before, &problem, &task, &[], actor, now)?;
                Ok(Response::DsaProblem(problem))
            }
            Command::ReviewDsaProblem { id, confidence } => {
                let before = self.dsa_problem(&id)?;
                ensure_live_entity(before.deleted_at, "problem")?;
                if before.status != DsaStatus::Solved {
                    return Err(Error::Validation(
                        "Solve this problem before reviewing it".into(),
                    ));
                }
                let confidence = validate_confidence(confidence)?;
                let mut problem = before.clone();
                problem.confidence = confidence;
                problem.review_stage = problem.review_stage.saturating_add(1).clamp(1, 4);
                let day = review_date(now, tz, problem.review_stage, confidence);
                problem.next_review_at = Some(day);
                let task = revision_task(&problem, day, tz, now);
                let mut superseded = Vec::new();
                for old in self.tasks()?.into_iter().filter(|task| {
                    task.deleted_at.is_none()
                        && task.status.active()
                        && task.dsa_problem_id.as_deref() == Some(problem.id.as_str())
                }) {
                    let prior = old.clone();
                    let mut completed = old;
                    completed.status = Status::Done;
                    completed.completed_at = Some(now);
                    superseded.push((prior, completed));
                }
                self.save_dsa_review(&before, &problem, &task, &superseded, actor, now)?;
                Ok(Response::DsaProblem(problem))
            }
            Command::DeleteDsaProblem { id } => {
                let before = self.dsa_problem(&id)?;
                ensure_live_entity(before.deleted_at, "problem")?;
                let mut problem = before.clone();
                problem.deleted_at = Some(now);
                self.save_dsa_problem(Some(&before), &problem, actor, "delete_dsa_problem", now)?;
                Ok(Response::DsaProblem(problem))
            }
            Command::CreateSubject { name } => {
                let subject = Subject {
                    id: ulid::Ulid::new().to_string(),
                    name: validate_text(&name, 200, "subject name")?,
                    created_at: now,
                    deleted_at: None,
                };
                self.save_subject(None, &subject, actor, "create_subject", now)?;
                Ok(Response::Subject(subject))
            }
            Command::DeleteSubject { id } => {
                let before = self.subject(&id)?;
                ensure_live_entity(before.deleted_at, "subject")?;
                let mut subject = before.clone();
                subject.deleted_at = Some(now);
                self.save_subject(Some(&before), &subject, actor, "delete_subject", now)?;
                Ok(Response::Subject(subject))
            }
            Command::AddSyllabusItem { subject_id, title } => {
                ensure_live_entity(self.subject(&subject_id)?.deleted_at, "subject")?;
                let item = SyllabusItem {
                    id: ulid::Ulid::new().to_string(),
                    subject_id,
                    title: validate_text(&title, 300, "syllabus item")?,
                    done: false,
                    created_at: now,
                    deleted_at: None,
                };
                self.save_syllabus_item(None, &item, actor, "add_syllabus_item", now)?;
                Ok(Response::SyllabusItem(item))
            }
            Command::SetSyllabusItemDone { id, done } => {
                let before = self.syllabus_item(&id)?;
                ensure_live_entity(before.deleted_at, "syllabus item")?;
                let mut item = before.clone();
                item.done = done;
                self.save_syllabus_item(
                    Some(&before),
                    &item,
                    actor,
                    "set_syllabus_item_done",
                    now,
                )?;
                Ok(Response::SyllabusItem(item))
            }
            Command::DeleteSyllabusItem { id } => {
                let before = self.syllabus_item(&id)?;
                ensure_live_entity(before.deleted_at, "syllabus item")?;
                let mut item = before.clone();
                item.deleted_at = Some(now);
                self.save_syllabus_item(Some(&before), &item, actor, "delete_syllabus_item", now)?;
                Ok(Response::SyllabusItem(item))
            }
            Command::CreateDeadline {
                subject_id,
                title,
                due_date,
                study_min,
            } => {
                if let Some(id) = &subject_id {
                    ensure_live_entity(self.subject(id)?.deleted_at, "subject")?;
                }
                if !(1..=1440).contains(&study_min) {
                    return Err(Error::Validation(
                        "Study minutes must be between 1 and 1440".into(),
                    ));
                }
                let deadline = Deadline {
                    id: ulid::Ulid::new().to_string(),
                    subject_id,
                    title: validate_text(&title, 300, "deadline title")?,
                    due_date,
                    study_min,
                    created_at: now,
                    deleted_at: None,
                };
                let tasks = study_tasks(&deadline, today, tz, now);
                self.save_deadline(None, &deadline, &tasks, actor, "create_deadline", now)?;
                Ok(Response::Deadline(deadline))
            }
            Command::DeleteDeadline { id } => {
                let before = self.deadline(&id)?;
                ensure_live_entity(before.deleted_at, "deadline")?;
                let mut deadline = before.clone();
                deadline.deleted_at = Some(now);
                self.save_deadline(Some(&before), &deadline, &[], actor, "delete_deadline", now)?;
                Ok(Response::Deadline(deadline))
            }
        }
    }
    fn ensure_live(task: &Task) -> Result<()> {
        if task.deleted_at.is_some() {
            Err(Error::Validation(
                "Restore this task before editing it".into(),
            ))
        } else {
            Ok(())
        }
    }
    fn save(
        &mut self,
        before: Option<&Task>,
        task: &Task,
        actor: Actor,
        action: &str,
        now: DateTime<Utc>,
    ) -> Result<()> {
        let tx = self.conn.transaction()?;
        Self::persist(&tx, task)?;
        Self::event(
            &tx,
            actor,
            "task",
            action,
            &task.id,
            before,
            Some(task),
            now,
        )?;
        tx.commit()?;
        Ok(())
    }
    fn validate_tags(tags: &[String]) -> Result<()> {
        if tags.len() > 30
            || tags.iter().any(|t| {
                t.len() > 64
                    || t.len() < 2
                    || !matches!(t.chars().next(), Some('#' | '@' | '!'))
                    || t.chars().any(char::is_whitespace)
            })
        {
            return Err(Error::Validation(
                "Use up to 30 tags beginning with #, @ or ! (no spaces, max 64 bytes each)".into(),
            ));
        }
        Ok(())
    }
    fn persist(tx: &Transaction<'_>, task: &Task) -> Result<()> {
        Self::validate_tags(&task.tags)?;
        tx.execute("INSERT INTO tasks(id,data,status,planned_day,due_at,created_at,deleted) VALUES(?1,?2,?3,?4,?5,?6,?7) ON CONFLICT(id) DO UPDATE SET data=excluded.data,status=excluded.status,planned_day=excluded.planned_day,due_at=excluded.due_at,deleted=excluded.deleted", params![task.id,serde_json::to_string(task)?,serde_json::to_string(&task.status)?,task.planned_day.map(|d| d.to_string()),task.due_at.map(|d| d.to_rfc3339()),task.created_at.to_rfc3339(),task.deleted_at.is_some()])?;
        tx.execute("DELETE FROM task_search WHERE id=?1", [&task.id])?;
        if task.deleted_at.is_none() {
            tx.execute(
                "INSERT INTO task_search(id,title,notes,tags) VALUES(?1,?2,?3,?4)",
                params![task.id, task.title, task.notes, task.tags.join(" ")],
            )?;
        }
        tx.execute("DELETE FROM task_tags WHERE task_id=?1", [&task.id])?;
        for tag in &task.tags {
            tx.execute(
                "INSERT OR IGNORE INTO tags(id,name) VALUES(?1,?2)",
                params![ulid::Ulid::new().to_string(), tag],
            )?;
            tx.execute("INSERT OR IGNORE INTO task_tags(task_id,tag_id) SELECT ?1,id FROM tags WHERE name=?2", params![task.id,tag])?;
        }
        Ok(())
    }
    #[allow(clippy::too_many_arguments)]
    fn event<T: serde::Serialize>(
        tx: &Transaction<'_>,
        actor: Actor,
        entity: &str,
        action: &str,
        id: &str,
        before: Option<&T>,
        after: Option<&T>,
        now: DateTime<Utc>,
    ) -> Result<()> {
        tx.execute("INSERT INTO events(id,ts,actor,entity,entity_id,action,payload) VALUES(?1,?2,?3,?4,?5,?6,?7)", params![ulid::Ulid::new().to_string(),now.to_rfc3339(),serde_json::to_value(actor)?.as_str().unwrap(),entity,id,action,serde_json::to_string(&serde_json::json!({"before":before,"after":after}))?])?;
        Ok(())
    }
    fn save_project(
        &mut self,
        before: Option<&Project>,
        project: &Project,
        actor: Actor,
        action: &str,
        now: DateTime<Utc>,
    ) -> Result<()> {
        let tx = self.conn.transaction()?;
        tx.execute(
            "INSERT INTO projects(id,data,deleted) VALUES(?1,?2,?3) ON CONFLICT(id) DO UPDATE SET data=excluded.data,deleted=excluded.deleted",
            params![
                project.id,
                serde_json::to_string(project)?,
                project.deleted_at.is_some()
            ],
        )?;
        Self::event(
            &tx,
            actor,
            "project",
            action,
            &project.id,
            before,
            Some(project),
            now,
        )?;
        tx.commit()?;
        Ok(())
    }
    fn save_goal(
        &mut self,
        before: Option<&Goal>,
        goal: &Goal,
        actor: Actor,
        action: &str,
        now: DateTime<Utc>,
    ) -> Result<()> {
        let tx = self.conn.transaction()?;
        tx.execute(
            "INSERT INTO goals(id,data,deleted) VALUES(?1,?2,?3) ON CONFLICT(id) DO UPDATE SET data=excluded.data,deleted=excluded.deleted",
            params![
                goal.id,
                serde_json::to_string(goal)?,
                goal.deleted_at.is_some()
            ],
        )?;
        Self::event(
            &tx,
            actor,
            "goal",
            action,
            &goal.id,
            before,
            Some(goal),
            now,
        )?;
        tx.commit()?;
        Ok(())
    }
    fn save_template(
        &mut self,
        before: Option<&Template>,
        template: &Template,
        actor: Actor,
        action: &str,
        now: DateTime<Utc>,
    ) -> Result<()> {
        let tx = self.conn.transaction()?;
        tx.execute(
            "INSERT INTO templates(id,data,deleted) VALUES(?1,?2,?3) ON CONFLICT(id) DO UPDATE SET data=excluded.data,deleted=excluded.deleted",
            params![
                template.id,
                serde_json::to_string(template)?,
                template.deleted_at.is_some()
            ],
        )?;
        Self::event(
            &tx,
            actor,
            "template",
            action,
            &template.id,
            before,
            Some(template),
            now,
        )?;
        tx.commit()?;
        Ok(())
    }
    fn save_filter(
        &mut self,
        _before: Option<&SavedFilter>,
        filter: &SavedFilter,
        actor: Actor,
        action: &str,
        now: DateTime<Utc>,
    ) -> Result<()> {
        let tx = self.conn.transaction()?;
        tx.execute(
            "INSERT INTO saved_filters(id,data,created_at) VALUES(?1,?2,?3) ON CONFLICT(id) DO UPDATE SET data=excluded.data",
            params![
                filter.id,
                serde_json::to_string(filter)?,
                filter.created_at.to_rfc3339()
            ],
        )?;
        Self::event(
            &tx,
            actor,
            "filter",
            action,
            &filter.id,
            None::<&SavedFilter>,
            Some(filter),
            now,
        )?;
        tx.commit()?;
        Ok(())
    }
    fn delete_filter(
        &mut self,
        filter: &SavedFilter,
        actor: Actor,
        now: DateTime<Utc>,
    ) -> Result<()> {
        let tx = self.conn.transaction()?;
        tx.execute("DELETE FROM saved_filters WHERE id=?1", [&filter.id])?;
        Self::event(
            &tx,
            actor,
            "filter",
            "delete_filter",
            &filter.id,
            Some(filter),
            None::<&SavedFilter>,
            now,
        )?;
        tx.commit()?;
        Ok(())
    }
    fn persist_dsa_problem(tx: &Transaction<'_>, problem: &DsaProblem) -> Result<()> {
        tx.execute("INSERT INTO dsa_problems(id,data,deleted) VALUES(?1,?2,?3) ON CONFLICT(id) DO UPDATE SET data=excluded.data,deleted=excluded.deleted", params![problem.id,serde_json::to_string(problem)?,problem.deleted_at.is_some()])?;
        Ok(())
    }
    fn save_dsa_problem(
        &mut self,
        before: Option<&DsaProblem>,
        problem: &DsaProblem,
        actor: Actor,
        action: &str,
        now: DateTime<Utc>,
    ) -> Result<()> {
        let tx = self.conn.transaction()?;
        Self::persist_dsa_problem(&tx, problem)?;
        Self::event(
            &tx,
            actor,
            "dsa_problem",
            action,
            &problem.id,
            before,
            Some(problem),
            now,
        )?;
        tx.commit()?;
        Ok(())
    }
    fn save_dsa_review(
        &mut self,
        before: &DsaProblem,
        after: &DsaProblem,
        task: &Task,
        superseded: &[(Task, Task)],
        actor: Actor,
        now: DateTime<Utc>,
    ) -> Result<()> {
        let tx = self.conn.transaction()?;
        Self::persist_dsa_problem(&tx, after)?;
        Self::event(
            &tx,
            actor,
            "dsa_problem",
            "review_dsa_problem",
            &after.id,
            Some(before),
            Some(after),
            now,
        )?;
        for (prior, completed) in superseded {
            Self::persist(&tx, completed)?;
            Self::event(
                &tx,
                actor,
                "task",
                "complete_task",
                &completed.id,
                Some(prior),
                Some(completed),
                now,
            )?;
        }
        Self::persist(&tx, task)?;
        Self::event(
            &tx,
            actor,
            "task",
            "create_task",
            &task.id,
            None::<&Task>,
            Some(task),
            now,
        )?;
        tx.commit()?;
        Ok(())
    }
    fn save_subject(
        &mut self,
        before: Option<&Subject>,
        subject: &Subject,
        actor: Actor,
        action: &str,
        now: DateTime<Utc>,
    ) -> Result<()> {
        let tx = self.conn.transaction()?;
        tx.execute("INSERT INTO subjects(id,data,deleted) VALUES(?1,?2,?3) ON CONFLICT(id) DO UPDATE SET data=excluded.data,deleted=excluded.deleted", params![subject.id,serde_json::to_string(subject)?,subject.deleted_at.is_some()])?;
        Self::event(
            &tx,
            actor,
            "subject",
            action,
            &subject.id,
            before,
            Some(subject),
            now,
        )?;
        tx.commit()?;
        Ok(())
    }
    fn save_syllabus_item(
        &mut self,
        before: Option<&SyllabusItem>,
        item: &SyllabusItem,
        actor: Actor,
        action: &str,
        now: DateTime<Utc>,
    ) -> Result<()> {
        let tx = self.conn.transaction()?;
        tx.execute("INSERT INTO syllabus_items(id,data,deleted) VALUES(?1,?2,?3) ON CONFLICT(id) DO UPDATE SET data=excluded.data,deleted=excluded.deleted", params![item.id,serde_json::to_string(item)?,item.deleted_at.is_some()])?;
        Self::event(
            &tx,
            actor,
            "syllabus_item",
            action,
            &item.id,
            before,
            Some(item),
            now,
        )?;
        tx.commit()?;
        Ok(())
    }
    fn save_deadline(
        &mut self,
        before: Option<&Deadline>,
        deadline: &Deadline,
        tasks: &[Task],
        actor: Actor,
        action: &str,
        now: DateTime<Utc>,
    ) -> Result<()> {
        let tx = self.conn.transaction()?;
        tx.execute("INSERT INTO deadlines(id,data,deleted) VALUES(?1,?2,?3) ON CONFLICT(id) DO UPDATE SET data=excluded.data,deleted=excluded.deleted", params![deadline.id,serde_json::to_string(deadline)?,deadline.deleted_at.is_some()])?;
        Self::event(
            &tx,
            actor,
            "deadline",
            action,
            &deadline.id,
            before,
            Some(deadline),
            now,
        )?;
        for task in tasks {
            Self::persist(&tx, task)?;
            Self::event(
                &tx,
                actor,
                "task",
                "create_task",
                &task.id,
                None::<&Task>,
                Some(task),
                now,
            )?;
        }
        tx.commit()?;
        Ok(())
    }
    pub fn next_reminder(&self) -> Result<Option<(Task, DateTime<Utc>)>> {
        Ok(self
            .tasks()?
            .into_iter()
            .filter(|t| t.deleted_at.is_none() && t.status.active() && t.reminded_at.is_none())
            .filter_map(|t| t.scheduled_start.or(t.due_at).map(|at| (t, at)))
            .min_by_key(|(_, at)| *at))
    }
    pub fn mark_reminded(&mut self, id: &str, now: DateTime<Utc>) -> Result<()> {
        self.record_reminder(id, now, "reminder_delivered")
    }
    pub fn mark_reminder_failed(&mut self, id: &str, now: DateTime<Utc>) -> Result<()> {
        self.record_reminder(id, now, "reminder_failed")
    }
    fn record_reminder(&mut self, id: &str, now: DateTime<Utc>, action: &str) -> Result<()> {
        let before = self.task(id)?;
        let mut task = before.clone();
        task.reminded_at = Some(now);
        self.save(Some(&before), &task, Actor::Ui, action, now)
    }
    pub fn event_count(&self) -> Result<i64> {
        Ok(self
            .conn
            .query_row("SELECT count(*) FROM events", [], |r| r.get(0))?)
    }
    pub fn checkpoint(&self) -> Result<()> {
        self.conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE)")?;
        Ok(())
    }
}
