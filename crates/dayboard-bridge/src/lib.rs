pub mod http;

use chrono::{NaiveDate, Utc};
use dayboard_core::{
    model::{Actor, Command, Response, Snapshot, Task, UndoResult},
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
    fn undo(&self) -> Result<Option<UndoResult>, String> {
        let result = {
            let mut core = self
                .core
                .lock()
                .map_err(|_| "Database lock unavailable".to_string())?;
            core.undo_last(Actor::Jarvis, Utc::now())
                .map_err(|error| error.to_string())?
        };
        if result.is_some() {
            (self.notifier)("changed");
            self.hub.publish("{\"type\":\"changed\"}");
        }
        Ok(result)
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
                "serverInfo": { "name": "daybridge", "version": env!("CARGO_PKG_VERSION") }
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
            "daybridge_command" => {
                let value = arguments.get("command").cloned().unwrap_or(Value::Null);
                let command: Command = serde_json::from_value(value)
                    .map_err(|error| (-32602, format!("Invalid command: {error}")))?;
                self.execute(command)
                    .map(|response| serde_json::to_value(response).unwrap_or(Value::Null))
                    .map_err(|error| (-32603, error))
            }
            "daybridge_today_summary" => self.rpc_summary(),
            "daybridge_overdue" => self.rpc_overdue(),
            "daybridge_weekly_stats" => self.rpc_weekly(),
            "daybridge_dsa_weak_topics" => self.rpc_topics(),
            "daybridge_search" => {
                let query = arguments
                    .get("query")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string();
                self.rpc_search(&json!({ "query": query }))
            }
            "daybridge_undo" => self.rpc_undo(),
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
            "name": "daybridge_command",
            "description": "Run a typed Daybridge command (create_task, update_task, complete_task, reschedule_task, plan_day, create_project, ...).",
            "inputSchema": {
                "type": "object",
                "properties": { "command": { "type": "object" } },
                "required": ["command"]
            }
        },
        {
            "name": "daybridge_today_summary",
            "description": "Today's planned tasks, capacity and inbox/overdue counts.",
            "inputSchema": { "type": "object", "properties": {} }
        },
        {
            "name": "daybridge_overdue",
            "description": "Active tasks past their due time or planned day.",
            "inputSchema": { "type": "object", "properties": {} }
        },
        {
            "name": "daybridge_weekly_stats",
            "description": "Last 7 days: completed, created, carried, focus minutes, estimate accuracy and per-tag stats.",
            "inputSchema": { "type": "object", "properties": {} }
        },
        {
            "name": "daybridge_dsa_weak_topics",
            "description": "DSA topics with unsolved problems or low average confidence.",
            "inputSchema": { "type": "object", "properties": {} }
        },
        {
            "name": "daybridge_search",
            "description": "Full-text search across task titles, notes and tags.",
            "inputSchema": {
                "type": "object",
                "properties": { "query": { "type": "string" } },
                "required": ["query"]
            }
        },
        {
            "name": "daybridge_undo",
            "description": "Undo the newest task mutation if the task has not changed since.",
            "inputSchema": { "type": "object", "properties": {} }
        }
    ])
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
