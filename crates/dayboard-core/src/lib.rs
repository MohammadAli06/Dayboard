pub mod model;
pub mod parser;
pub mod planner;
use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use model::*;
use planner::Planner;
use rusqlite::{params, Connection, Transaction};
use rusqlite_migration::{Migrations, M};
use std::{path::Path, time::Duration};

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
        Migrations::new(vec![M::up(include_str!("schema.sql"))]).to_latest(&mut conn)?;
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
                Ok(Response::Snapshot(Snapshot {
                    capacity: planner::capacity(&all, today, &settings),
                    tasks,
                    settings,
                    today,
                    tags,
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
        Self::event(&tx, actor, action, &task.id, before, Some(task), now)?;
        tx.commit()?;
        Ok(())
    }
    fn persist(tx: &Transaction<'_>, task: &Task) -> Result<()> {
        if task.tags.len() > 30
            || task.tags.iter().any(|t| {
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
    fn event<T: serde::Serialize>(
        tx: &Transaction<'_>,
        actor: Actor,
        action: &str,
        id: &str,
        before: Option<&T>,
        after: Option<&T>,
        now: DateTime<Utc>,
    ) -> Result<()> {
        tx.execute("INSERT INTO events(id,ts,actor,entity,entity_id,action,payload) VALUES(?1,?2,?3,?4,?5,?6,?7)", params![ulid::Ulid::new().to_string(),now.to_rfc3339(),serde_json::to_value(actor)?.as_str().unwrap(),if action == "save_settings" {"settings"} else {"task"},id,action,serde_json::to_string(&serde_json::json!({"before":before,"after":after}))?])?;
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
