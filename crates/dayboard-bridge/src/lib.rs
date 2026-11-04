pub mod http;

use chrono::{NaiveDate, TimeZone, Utc};
use dayboard_core::{
    model::{Actor, Command, DayStatusKind, Response, Snapshot, Task, UndoOutcome},
    Core,
};
use serde_json::{json, Value};
use std::{
    path::Path,
    sync::{mpsc, Arc, Mutex},
};

pub type Notifier = Arc<dyn Fn(&str) + Send + Sync>;

#[derive(Default)]
pub struct EventHub {
    clients: Mutex<Vec<mpsc::Sender<String>>>,
}

impl EventHub {
    pub fn subscribe(&self) -> mpsc::Receiver<String> {
        let (sender, receiver) = mpsc::channel();
        if let Ok(mut clients) = self.clients.lock() {
            clients.push(sender);
        }
        receiver
    }

    pub fn publish(&self, data: &str) {
        let message = format!("data: {data}\n\n");
        if let Ok(mut clients) = self.clients.lock() {
            clients.retain(|client| client.send(message.clone()).is_ok());
        }
    }
}

pub struct Bridge {
    core: Mutex<Core>,
    token: String,
    notifier: Notifier,
    hub: EventHub,
}

impl Bridge {
    pub fn new(core: Core, token: String, notifier: Notifier) -> Self {
        Self {
            core: Mutex::new(core),
            token,
            notifier,
            hub: EventHub::default(),
        }
    }
    pub fn token(&self) -> &str {
        &self.token
    }
    pub fn hub(&self) -> &EventHub {
        &self.hub
    }
    pub fn snapshot_search(&self, search: Option<String>) -> Result<Snapshot, String> {
        let mut core = self
            .core
            .lock()
            .map_err(|_| "Database lock unavailable".to_string())?;
        match core
            .execute(Command::Snapshot { search }, Actor::Jarvis, Utc::now())
            .map_err(|error| error.to_string())?
        {
            Response::Snapshot(snapshot) => Ok(*snapshot),
            _ => Err("Unexpected core response".into()),
        }
    }
    fn execute(&self, command: Command) -> Result<Response, String> {
        let changed = !matches!(
            &command,
            Command::PingDb
                | Command::Snapshot { .. }
                | Command::ParseQuickAdd { .. }
                | Command::PlanDay { .. }
                | Command::ListScheduleItems
                | Command::ListOccurrences { .. }
                | Command::ListAcademicEvents { .. }
                | Command::GetDayStatus { .. }
                | Command::ListConflicts { .. }
                | Command::WeekLoad { .. }
                | Command::PreviewPrepPlan { .. }
        );
        let response = {
            let mut core = self
                .core
                .lock()
                .map_err(|_| "Database lock unavailable".to_string())?;
            core.execute(command, Actor::Jarvis, Utc::now())
                .map_err(|error| error.to_string())?
        };
        if changed {
            (self.notifier)("changed");
            self.hub.publish("{\"type\":\"changed\"}");
        }
        Ok(response)
    }
    fn undo(&self) -> Result<Option<UndoOutcome>, String> {
        let result = {
            let mut core = self
                .core
                .lock()
                .map_err(|_| "Database lock unavailable".to_string())?;
            core.undo_last_any(Actor::Jarvis, Utc::now())
                .map_err(|error| error.to_string())?
        };
        if result.is_some() {
            (self.notifier)("changed");
            self.hub.publish("{\"type\":\"changed\"}");
        }
        Ok(result)
    }

    fn rpc_typed(&self, command: &str, input: Value) -> Result<Value, (i64, String)> {
        let mut value = json!({ "command": command });
        if !input.is_null() {
            value["input"] = input;
        }
        let command: Command = serde_json::from_value(value)
            .map_err(|error| (-32602, format!("Invalid command: {error}")))?;
        let response = self.execute(command).map_err(|error| (-32603, error))?;
        serde_json::to_value(response).map_err(|error| (-32603, error.to_string()))
    }
    fn rpc_suggest(&self, params: &Value) -> Result<Value, (i64, String)> {
        let task_id = params
            .get("task_id")
            .and_then(Value::as_str)
            .ok_or((-32602, "Missing task_id".to_string()))?;
        let snapshot = self
            .snapshot_search(None)
            .map_err(|error| (-32603, error))?;
        let task = snapshot
            .tasks
            .iter()
            .find(|task| task.id == task_id)
            .cloned()
            .ok_or((-32603, "Task not found".to_string()))?;
        let mut date = snapshot.today;
        for _ in 0..60 {
            if let Response::DayStatus(status) = self
                .execute(Command::GetDayStatus { date })
                .map_err(|error| (-32603, error))?
            {
                if status.status != DayStatusKind::Blocked
                    && status.available_min.saturating_sub(status.used_min) >= task.est_min
                {
                    return Ok(json!({
                        "task_id": task.id,
                        "title": task.title,
                        "suggested_date": date,
                        "available_min": status.available_min,
                        "used_min": status.used_min
                    }));
                }
            }
            date += chrono::Duration::days(1);
        }
        Ok(json!({
            "task_id": task.id,
            "title": task.title,
            "suggested_date": Value::Null
        }))
    }
    fn rpc_free_slots(&self, params: &Value) -> Result<Value, (i64, String)> {
        let snapshot = self
            .snapshot_search(None)
            .map_err(|error| (-32603, error))?;
        let date = match params.get("date").and_then(Value::as_str) {
            Some(text) => NaiveDate::parse_from_str(text, "%Y-%m-%d")
                .map_err(|_| (-32602, "Invalid date".to_string()))?,
            None => snapshot.today,
        };
        let tz: chrono_tz::Tz = snapshot
            .settings
            .timezone
            .parse()
            .map_err(|_| (-32603, "Invalid timezone".to_string()))?;
        let (occurrences, events) = {
            let mut core = self
                .core
                .lock()
                .map_err(|_| (-32603, "Database lock unavailable".to_string()))?;
            let occurrences = match core
                .execute(
                    Command::ListOccurrences {
                        from: date,
                        to: date,
                    },
                    Actor::Jarvis,
                    Utc::now(),
                )
                .map_err(|error| (-32603, error.to_string()))?
            {
                Response::Occurrences(items) => items,
                _ => Vec::new(),
            };
            let events = match core
                .execute(
                    Command::ListAcademicEvents {
                        from: date,
                        to: date,
                    },
                    Actor::Jarvis,
                    Utc::now(),
                )
                .map_err(|error| (-32603, error.to_string()))?
            {
                Response::AcademicEvents(items) => items,
                _ => Vec::new(),
            };
            (occurrences, events)
        };
        let mut busy: Vec<(chrono::DateTime<Utc>, chrono::DateTime<Utc>)> = occurrences
            .iter()
            .map(|occurrence| (occurrence.start_utc, occurrence.end_utc))
            .collect();
        for event in events {
            if !event.all_day && event.start_at.with_timezone(&tz).date_naive() == date {
                busy.push((event.start_at, event.end_at));
            }
        }
        for task in &snapshot.tasks {
            if let (Some(start), Some(end)) = (task.scheduled_start, task.scheduled_end) {
                if start.with_timezone(&tz).date_naive() == date {
                    busy.push((start, end));
                }
            }
        }
        let work_start = local_on(tz, date, snapshot.settings.work_start as u32)?;
        let work_end = local_on(tz, date, snapshot.settings.work_end as u32)?;
        busy.sort();
        let mut slots = Vec::new();
        let mut cursor = work_start;
        for (start, end) in busy {
            if end <= cursor || start >= work_end {
                continue;
            }
            if start > cursor {
                slots.push(json!({ "start": cursor, "end": start }));
            }
            if end > cursor {
                cursor = end;
            }
        }
        if cursor < work_end {
            slots.push(json!({ "start": cursor, "end": work_end }));
        }
        Ok(json!({ "date": date, "slots": slots }))
    }

    pub fn handle_rpc(&self, request: &Value) -> Option<Value> {
        let id = request.get("id").cloned();
        let method = request.get("method").and_then(Value::as_str).unwrap_or("");
        let params = request.get("params").cloned().unwrap_or(Value::Null);
        let is_notification = id.is_none();
        let outcome: Result<Value, (i64, String)> = match method {
            "initialize" => Ok(json!({
                "protocolVersion": "2024-11-05",
                "capabilities": { "tools": {} },
                "serverInfo": { "name": "dayboard", "version": env!("CARGO_PKG_VERSION") }
            })),
            "notifications/initialized" | "notifications/cancelled" => return None,
            "ping" => Ok(json!({})),
            "tools/list" => Ok(json!({ "tools": tools() })),
            "tools/call" => self.call_tool(&params),
            "dayboard.command" => self.rpc_command(&params),
            "dayboard.today_summary" => self.rpc_summary(),
            "dayboard.overdue" => self.rpc_overdue(),
            "dayboard.weekly_stats" => self.rpc_weekly(),
            "dayboard.dsa_weak_topics" => self.rpc_topics(),
            "dayboard.search" => self.rpc_search(&params),
            "dayboard.undo" => self.rpc_undo(),
            "dayboard.list_schedule" => self.rpc_typed("list_schedule_items", Value::Null),
            "dayboard.create_schedule_item" => {
                self.rpc_typed("create_schedule_item", params.clone())
            }
            "dayboard.skip_occurrence" => self.rpc_typed("skip_occurrence", params.clone()),
            "dayboard.get_day_status" => self.rpc_typed("get_day_status", params.clone()),
            "dayboard.list_conflicts" => self.rpc_typed("list_conflicts", params.clone()),
            "dayboard.suggest_reschedule" => self.rpc_suggest(&params),
            "dayboard.preview_prep_plan" => self.rpc_typed("preview_prep_plan", params.clone()),
            "dayboard.find_free_slots" => self.rpc_free_slots(&params),
            "dayboard.schema" => Ok(json!({
                "command": schemars::schema_for!(Command),
                "response": schemars::schema_for!(Response)
            })),
            _ => Err((-32601, format!("Unknown method: {method}"))),
        };
        if is_notification {
            return None;
        }
        let id = id.unwrap_or(Value::Null);
        Some(match outcome {
            Ok(result) => json!({ "jsonrpc": "2.0", "id": id, "result": result }),
            Err((code, message)) => {
                json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
            }
        })
    }

    fn call_tool(&self, params: &Value) -> Result<Value, (i64, String)> {
        let name = params
            .get("name")
            .and_then(Value::as_str)
            .ok_or((-32602, "Missing tool name".to_string()))?;
        let arguments = params
            .get("arguments")
            .cloned()
            .unwrap_or_else(|| json!({}));
        let outcome = match name {
            "dayboard_command" => {
                let value = arguments.get("command").cloned().unwrap_or(Value::Null);
                let command: Command = serde_json::from_value(value)
                    .map_err(|error| (-32602, format!("Invalid command: {error}")))?;
                self.execute(command)
                    .map(|response| serde_json::to_value(response).unwrap_or(Value::Null))
                    .map_err(|error| (-32603, error))
            }
            "dayboard_today_summary" => self.rpc_summary(),
            "dayboard_overdue" => self.rpc_overdue(),
            "dayboard_weekly_stats" => self.rpc_weekly(),
            "dayboard_dsa_weak_topics" => self.rpc_topics(),
            "dayboard_search" => {
                let query = arguments
                    .get("query")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string();
                self.rpc_search(&json!({ "query": query }))
            }
            "dayboard_undo" => self.rpc_undo(),
            "dayboard_list_schedule" => self.rpc_typed("list_schedule_items", Value::Null),
            "dayboard_create_schedule_item" => {
                self.rpc_typed("create_schedule_item", arguments.clone())
            }
            "dayboard_skip_occurrence" => self.rpc_typed("skip_occurrence", arguments.clone()),
            "dayboard_get_day_status" => self.rpc_typed("get_day_status", arguments.clone()),
            "dayboard_list_conflicts" => self.rpc_typed("list_conflicts", arguments.clone()),
            "dayboard_suggest_reschedule" => self.rpc_suggest(&arguments),
            "dayboard_preview_prep_plan" => self.rpc_typed("preview_prep_plan", arguments.clone()),
            "dayboard_find_free_slots" => self.rpc_free_slots(&arguments),
            _ => Err((-32601, format!("Unknown tool: {name}"))),
        };
        Ok(match outcome {
            Ok(value) => json!({
                "content": [{ "type": "text", "text": value.to_string() }],
                "isError": false
            }),
            Err((_, message)) => json!({
                "content": [{ "type": "text", "text": message }],
                "isError": true
            }),
        })
    }

    fn rpc_command(&self, params: &Value) -> Result<Value, (i64, String)> {
        let value = params
            .get("command")
            .cloned()
            .ok_or((-32602, "Missing command".to_string()))?;
        let command: Command = serde_json::from_value(value)
            .map_err(|error| (-32602, format!("Invalid command: {error}")))?;
        let response = self.execute(command).map_err(|error| (-32603, error))?;
        serde_json::to_value(response).map_err(|error| (-32603, error.to_string()))
    }
    fn rpc_summary(&self) -> Result<Value, (i64, String)> {
        let snapshot = self
            .snapshot_search(None)
            .map_err(|error| (-32603, error))?;
        Ok(today_summary(&snapshot))
    }
    fn rpc_overdue(&self) -> Result<Value, (i64, String)> {
        let snapshot = self
            .snapshot_search(None)
            .map_err(|error| (-32603, error))?;
        Ok(overdue_summary(&snapshot))
    }
    fn rpc_weekly(&self) -> Result<Value, (i64, String)> {
        let snapshot = self
            .snapshot_search(None)
            .map_err(|error| (-32603, error))?;
        serde_json::to_value(&snapshot.weekly_stats).map_err(|error| (-32603, error.to_string()))
    }
    fn rpc_topics(&self) -> Result<Value, (i64, String)> {
        let snapshot = self
            .snapshot_search(None)
            .map_err(|error| (-32603, error))?;
        Ok(weak_topics(&snapshot))
    }
    fn rpc_search(&self, params: &Value) -> Result<Value, (i64, String)> {
        let query = params
            .get("query")
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim()
            .to_string();
        if query.is_empty() {
            return Ok(json!({ "query": "", "tasks": [] }));
        }
        let snapshot = self
            .snapshot_search(Some(query.clone()))
            .map_err(|error| (-32603, error))?;
        Ok(json!({
            "query": query,
            "tasks": snapshot.tasks.iter().map(compact_task).collect::<Vec<_>>()
        }))
    }
    fn rpc_undo(&self) -> Result<Value, (i64, String)> {
        self.undo()
            .map(|result| {
                result
                    .map(|value| serde_json::to_value(value).unwrap_or(Value::Null))
                    .unwrap_or(Value::Null)
            })
            .map_err(|error| (-32603, error))
    }
}

pub fn tools() -> Value {
    json!([
        {
            "name": "dayboard_command",
            "description": "Run a typed Dayboard command (create_task, update_task, complete_task, reschedule_task, plan_day, create_project, ...).",
            "inputSchema": {
                "type": "object",
                "properties": { "command": { "type": "object" } },
                "required": ["command"]
            }
        },
        {
            "name": "dayboard_today_summary",
            "description": "Today's planned tasks, capacity and inbox/overdue counts.",
            "inputSchema": { "type": "object", "properties": {} }
        },
        {
            "name": "dayboard_overdue",
            "description": "Active tasks past their due time or planned day.",
            "inputSchema": { "type": "object", "properties": {} }
        },
        {
            "name": "dayboard_weekly_stats",
            "description": "Last 7 days: completed, created, carried, focus minutes, estimate accuracy and per-tag stats.",
            "inputSchema": { "type": "object", "properties": {} }
        },
        {
            "name": "dayboard_dsa_weak_topics",
            "description": "DSA topics with unsolved problems or low average confidence.",
            "inputSchema": { "type": "object", "properties": {} }
        },
        {
            "name": "dayboard_search",
            "description": "Full-text search across task titles, notes and tags.",
            "inputSchema": {
                "type": "object",
                "properties": { "query": { "type": "string" } },
                "required": ["query"]
            }
        },
        {
            "name": "dayboard_undo",
            "description": "Undo the newest task, schedule, term or academic-event mutation if it has not changed since.",
            "inputSchema": { "type": "object", "properties": {} }
        },
        {
            "name": "dayboard_list_schedule",
            "description": "List recurring weekly schedule items for the active term.",
            "inputSchema": { "type": "object", "properties": {} }
        },
        {
            "name": "dayboard_create_schedule_item",
            "description": "Create a recurring block (class, work, routine, other) with weekdays bitmask Mon=1..Sun=64 and HH:MM times.",
            "inputSchema": { "type": "object", "properties": { "title": { "type": "string" }, "kind": { "type": "string" }, "weekdays": { "type": "number" }, "start_time": { "type": "string" }, "end_time": { "type": "string" }, "term_id": { "type": ["string", "null"] }, "location": { "type": ["string", "null"] }, "notes": { "type": ["string", "null"] }, "subject_id": { "type": ["string", "null"] }, "remind_before_min": { "type": ["number", "null"] } }, "required": ["title", "kind", "weekdays", "start_time", "end_time"] }
        },
        {
            "name": "dayboard_skip_occurrence",
            "description": "Skip one occurrence of a schedule item on a date (YYYY-MM-DD).",
            "inputSchema": { "type": "object", "properties": { "item_id": { "type": "string" }, "date": { "type": "string" } }, "required": ["item_id", "date"] }
        },
        {
            "name": "dayboard_get_day_status",
            "description": "Day status, available and used minutes with rule reasons for a date.",
            "inputSchema": { "type": "object", "properties": { "date": { "type": "string" } }, "required": ["date"] }
        },
        {
            "name": "dayboard_list_conflicts",
            "description": "Open day-load conflicts in a date range with suggested actions.",
            "inputSchema": { "type": "object", "properties": { "from": { "type": "string" }, "to": { "type": "string" } }, "required": ["from", "to"] }
        },
        {
            "name": "dayboard_suggest_reschedule",
            "description": "Suggest the next day with enough free minutes for a task.",
            "inputSchema": { "type": "object", "properties": { "task_id": { "type": "string" } }, "required": ["task_id"] }
        },
        {
            "name": "dayboard_preview_prep_plan",
            "description": "Preview study tasks for an exam spread across open days before it.",
            "inputSchema": { "type": "object", "properties": { "event_id": { "type": "string" } }, "required": ["event_id"] }
        },
        {
            "name": "dayboard_find_free_slots",
            "description": "Free work-window slots for a date, minus classes, exams and scheduled tasks.",
            "inputSchema": { "type": "object", "properties": { "date": { "type": "string" } } }
        }
    ])
}

fn local_on(
    tz: chrono_tz::Tz,
    date: NaiveDate,
    hour: u32,
) -> Result<chrono::DateTime<Utc>, (i64, String)> {
    let naive = date
        .and_hms_opt(hour, 0, 0)
        .ok_or((-32602, "Invalid hour".to_string()))?;
    tz.from_local_datetime(&naive)
        .earliest()
        .or_else(|| {
            tz.from_local_datetime(&(naive + chrono::Duration::hours(1)))
                .earliest()
        })
        .map(|at| at.with_timezone(&Utc))
        .ok_or((-32602, "No such local time".to_string()))
}

fn compact_task(task: &Task) -> Value {
    json!({
        "id": task.id,
        "title": task.title,
        "status": task.status,
        "priority": task.priority,
        "est_min": task.est_min,
        "actual_min": task.actual_min,
        "planned_day": task.planned_day,
        "due_at": task.due_at,
        "scheduled_start": task.scheduled_start,
        "tags": task.tags,
        "carried_count": task.carried_count,
        "recurrence_rule": task.recurrence_rule
    })
}

fn is_overdue(task: &Task, today: NaiveDate, now: chrono::DateTime<Utc>) -> bool {
    task.status.active()
        && (task.due_at.is_some_and(|due| due < now)
            || task.planned_day.is_some_and(|day| day < today))
}

pub fn today_summary(snapshot: &Snapshot) -> Value {
    let now = Utc::now();
    let tasks: Vec<Value> = snapshot
        .tasks
        .iter()
        .filter(|task| task.planned_day == Some(snapshot.today) && task.status.active())
        .map(compact_task)
        .collect();
    let overdue = snapshot
        .tasks
        .iter()
        .filter(|task| is_overdue(task, snapshot.today, now))
        .count();
    let inbox = snapshot
        .tasks
        .iter()
        .filter(|task| task.planned_day.is_none() && task.status.active())
        .count();
    json!({
        "today": snapshot.today,
        "capacity": snapshot.capacity,
        "tasks": tasks,
        "counts": { "today": tasks.len(), "inbox": inbox, "overdue": overdue }
    })
}

pub fn overdue_summary(snapshot: &Snapshot) -> Value {
    let now = Utc::now();
    let tasks: Vec<Value> = snapshot
        .tasks
        .iter()
        .filter(|task| is_overdue(task, snapshot.today, now))
        .map(compact_task)
        .collect();
    json!({ "count": tasks.len(), "tasks": tasks })
}

pub fn weak_topics(snapshot: &Snapshot) -> Value {
    let mut topics = snapshot.dsa_topics.clone();
    topics.sort_by(|a, b| {
        a.average_confidence
            .partial_cmp(&b.average_confidence)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| b.total.cmp(&a.total))
    });
    let values: Vec<Value> = topics
        .iter()
        .map(|topic| {
            json!({
                "topic": topic.topic,
                "total": topic.total,
                "solved": topic.solved,
                "average_confidence": topic.average_confidence,
                "weak": topic.solved < topic.total || topic.average_confidence < 3.0
            })
        })
        .collect();
    json!({ "topics": values })
}

pub fn start(db_path: &Path, port: u16, token: String, notifier: Notifier) -> Result<u16, String> {
    let core = Core::open(db_path).map_err(|error| error.to_string())?;
    let bridge = Arc::new(Bridge::new(core, token, notifier));
    http::spawn(bridge, port).map_err(|error| error.to_string())
}

pub fn serve_stdio(bridge: &Bridge) -> std::io::Result<()> {
    use std::io::{BufRead, Write};
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let value: Value = match serde_json::from_str(&line) {
            Ok(value) => value,
            Err(_) => {
                writeln!(
                    stdout,
                    "{}",
                    json!({
                        "jsonrpc": "2.0",
                        "id": null,
                        "error": { "code": -32700, "message": "Parse error" }
                    })
                )?;
                stdout.flush()?;
                continue;
            }
        };
        if let Some(response) = bridge.handle_rpc(&value) {
            writeln!(stdout, "{response}")?;
            stdout.flush()?;
        }
    }
    Ok(())
}
