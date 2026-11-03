use chrono::{DateTime, Utc};
use dayboard_core::{model::*, parser, Core};
fn now() -> DateTime<Utc> {
    "2026-11-02T10:00:00Z".parse().unwrap()
}
fn create(core: &mut Core, text: &str) -> Task {
    match core
        .execute(
            Command::CreateTask {
                text: text.into(),
                parent_id: None,
            },
            Actor::User,
            now(),
        )
        .unwrap()
    {
        Response::Task(t) => t,
        _ => panic!("wrong response"),
    }
}
#[test]
fn parser_table_36_cases() {
    let valid = [
        ("Read", "Read", 3, 25, None),
        ("Read p1", "Read", 1, 25, None),
        ("Read p2", "Read", 2, 25, None),
        ("Read p3", "Read", 3, 25, None),
        ("Read p4", "Read", 4, 25, None),
        ("Read P1", "Read", 1, 25, None),
        ("Read 45m", "Read", 3, 45, None),
        ("Read 1h", "Read", 3, 60, None),
        ("Read 1h30m", "Read", 3, 90, None),
        ("Read 2h5m", "Read", 3, 125, None),
        ("Read 1m", "Read", 3, 1, None),
        ("Read 24h", "Read", 3, 1440, None),
        (
            "Read today",
            "Read",
            3,
            25,
            Some("2026-11-02T17:00:00+00:00"),
        ),
        (
            "Read tomorrow",
            "Read",
            3,
            25,
            Some("2026-11-03T17:00:00+00:00"),
        ),
        ("Read fri", "Read", 3, 25, Some("2026-11-06T17:00:00+00:00")),
        ("Read mon", "Read", 3, 25, Some("2026-11-02T17:00:00+00:00")),
        ("Read sun", "Read", 3, 25, Some("2026-11-08T17:00:00+00:00")),
        (
            "Read next week",
            "Read",
            3,
            25,
            Some("2026-11-09T17:00:00+00:00"),
        ),
        (
            "Read 2026-11-05",
            "Read",
            3,
            25,
            Some("2026-11-05T17:00:00+00:00"),
        ),
        ("Read 7pm", "Read", 3, 25, Some("2026-11-02T19:00:00+00:00")),
        (
            "Read 12am",
            "Read",
            3,
            25,
            Some("2026-11-02T00:00:00+00:00"),
        ),
        (
            "Read 12pm",
            "Read",
            3,
            25,
            Some("2026-11-02T12:00:00+00:00"),
        ),
        (
            "Read 7:30pm",
            "Read",
            3,
            25,
            Some("2026-11-02T19:30:00+00:00"),
        ),
        (
            "Read 23:59",
            "Read",
            3,
            25,
            Some("2026-11-02T23:59:00+00:00"),
        ),
        ("Read #college", "Read", 3, 25, None),
        ("Read @deep", "Read", 3, 25, None),
        ("Read !review", "Read", 3, 25, None),
        ("  Read   a chapter ", "Read a chapter", 3, 25, None),
        (
            "Solve #dsa @deep p1 45m today 7pm",
            "Solve",
            1,
            45,
            Some("2026-11-02T19:00:00+00:00"),
        ),
        ("Read p2 p1", "Read", 1, 25, None),
        ("Read #x #x", "Read", 3, 25, None),
        (
            "Read https://example.com",
            "Read https://example.com",
            3,
            25,
            None,
        ),
        ("Read tue", "Read", 3, 25, Some("2026-11-03T17:00:00+00:00")),
        ("Read wed", "Read", 3, 25, Some("2026-11-04T17:00:00+00:00")),
        ("Read thu", "Read", 3, 25, Some("2026-11-05T17:00:00+00:00")),
        ("Read sat", "Read", 3, 25, Some("2026-11-07T17:00:00+00:00")),
    ];
    for (input, title, priority, minutes, due) in valid {
        let p = parser::parse(input, now(), chrono_tz::UTC).unwrap();
        assert_eq!(
            (p.title.as_str(), p.priority, p.est_min),
            (title, priority, minutes),
            "{input}"
        );
        assert_eq!(p.due_at.map(|d| d.to_rfc3339()).as_deref(), due, "{input}");
    }
}
#[test]
fn invalid_parser_inputs_and_dst() {
    for input in [
        "",
        "  ",
        "p1 #dsa 30m",
        "Read p0",
        "Read p9",
        "Read 0m",
        "Read 25h",
        "Read 9999999999999999h",
        "Read 13pm",
        "Read 7:99pm",
        "Read 25:00",
        "Read 2026-02-30",
    ] {
        assert!(
            parser::parse(input, now(), chrono_tz::UTC).is_err(),
            "{input}"
        );
    }
    for date in ["2026-03-08T02:30", "2026-11-01T01:30"] {
        assert!(parser::local_datetime(date, chrono_tz::America::New_York).is_err());
    }
}
#[test]
fn timezone_and_tags() {
    let p = parser::parse(
        "Study #DSA #dsa @deep !review today 7pm",
        now(),
        chrono_tz::Asia::Kolkata,
    )
    .unwrap();
    assert_eq!(p.due_at.unwrap().to_rfc3339(), "2026-11-02T13:30:00+00:00");
    assert_eq!(p.tags, vec!["#dsa", "@deep", "!review"]);
    assert_eq!(p.energy, Energy::High);
}
#[test]
fn mutation_audit_dry_run_restore_and_persistence() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("tasks.db");
    let mut core = Core::open(&path).unwrap();
    let task = create(&mut core, "Study #dsa today");
    assert_eq!(core.event_count().unwrap(), 1);
    core.execute(
        Command::DeleteTask {
            id: task.id.clone(),
            dry_run: true,
        },
        Actor::Jarvis,
        now(),
    )
    .unwrap();
    assert!(core.tasks().unwrap()[0].deleted_at.is_none());
    assert_eq!(core.event_count().unwrap(), 1);
    core.execute(
        Command::DeleteTask {
            id: task.id.clone(),
            dry_run: false,
        },
        Actor::Jarvis,
        now(),
    )
    .unwrap();
    assert!(core.tasks().unwrap()[0].deleted_at.is_some());
    core.execute(
        Command::RestoreTask {
            id: task.id.clone(),
        },
        Actor::User,
        now(),
    )
    .unwrap();
    core.execute(
        Command::CompleteTask {
            id: task.id,
            completed: true,
        },
        Actor::Ui,
        now(),
    )
    .unwrap();
    drop(core);
    let core = Core::open(&path).unwrap();
    assert_eq!(core.tasks().unwrap()[0].status, Status::Done);
    assert_eq!(core.event_count().unwrap(), 4);
}
#[test]
fn rollover_is_idempotent_and_preserves_deadline() {
    let mut core = Core::memory().unwrap();
    let t = create(&mut core, "Work 2026-10-30");
    core.execute(Command::Rollover { dry_run: true }, Actor::Ui, now())
        .unwrap();
    assert_eq!(core.tasks().unwrap()[0].carried_count, 0);
    core.execute(Command::Rollover { dry_run: false }, Actor::Ui, now())
        .unwrap();
    core.execute(Command::Rollover { dry_run: false }, Actor::Ui, now())
        .unwrap();
    let task = &core.tasks().unwrap()[0];
    assert_eq!(task.carried_count, 3);
    assert_eq!(task.due_at, t.due_at);
    assert_eq!(task.planned_day.unwrap().to_string(), "2026-11-02");
    assert_eq!(core.event_count().unwrap(), 2);
}
#[test]
fn search_is_literal_and_deleted_tasks_disappear() {
    let mut core = Core::memory().unwrap();
    let task = create(&mut core, "Review graph algorithms #dsa");
    create(&mut core, "Read a chapter");
    match core
        .execute(
            Command::Snapshot {
                search: Some("graph".into()),
            },
            Actor::Ui,
            now(),
        )
        .unwrap()
    {
        Response::Snapshot(s) => assert_eq!(s.tasks.len(), 1),
        _ => panic!(),
    }
    for q in ["' OR 1=1; --", "\"", "*", "graph OR chapter"] {
        assert!(core
            .execute(
                Command::Snapshot {
                    search: Some(q.into())
                },
                Actor::Ui,
                now()
            )
            .is_ok());
    }
    core.execute(
        Command::DeleteTask {
            id: task.id,
            dry_run: false,
        },
        Actor::Ui,
        now(),
    )
    .unwrap();
    match core
        .execute(
            Command::Snapshot {
                search: Some("graph".into()),
            },
            Actor::Ui,
            now(),
        )
        .unwrap()
    {
        Response::Snapshot(s) => assert!(s.tasks.is_empty()),
        _ => panic!(),
    }
}
#[test]
fn validation_failure_rolls_back_task_and_event() {
    let mut core = Core::memory().unwrap();
    let too_long = format!("Study #{}", "x".repeat(65));
    assert!(core
        .execute(
            Command::CreateTask {
                text: too_long,
                parent_id: None
            },
            Actor::User,
            now()
        )
        .is_err());
    assert!(core.tasks().unwrap().is_empty());
    assert_eq!(core.event_count().unwrap(), 0);
}
#[test]
fn planner_respects_capacity_and_energy() {
    let mut core = Core::memory().unwrap();
    create(&mut core, "Deep @deep p1 60m");
    create(&mut core, "Easy p2 30m");
    let mut settings = core.settings().unwrap();
    settings.max_minutes = 45;
    core.execute(Command::SaveSettings(settings), Actor::User, now())
        .unwrap();
    match core
        .execute(
            Command::PlanDay {
                day: now().date_naive(),
                energy: None,
            },
            Actor::Ui,
            now(),
        )
        .unwrap()
    {
        Response::Tasks(t) => assert_eq!(t[0].title, "Easy"),
        _ => panic!(),
    }
    match core
        .execute(
            Command::PlanDay {
                day: now().date_naive(),
                energy: Some(Energy::High),
            },
            Actor::Ui,
            now(),
        )
        .unwrap()
    {
        Response::Tasks(t) => assert!(t.is_empty()),
        _ => panic!(),
    }
}
#[test]
fn reminders_rearm_on_reschedule_and_ignore_completed() {
    let mut core = Core::memory().unwrap();
    let task = create(&mut core, "Read today 7pm");
    assert!(core.next_reminder().unwrap().is_some());
    core.mark_reminded(&task.id, now()).unwrap();
    assert!(core.next_reminder().unwrap().is_none());
    core.execute(
        Command::RescheduleTask {
            id: task.id.clone(),
            local_start: Some("2026-11-02T20:00".into()),
            day: None,
            dry_run: false,
        },
        Actor::Ui,
        now(),
    )
    .unwrap();
    assert_eq!(
        core.next_reminder().unwrap().unwrap().1.to_rfc3339(),
        "2026-11-02T20:00:00+00:00"
    );
    core.execute(
        Command::CompleteTask {
            id: task.id,
            completed: true,
        },
        Actor::Ui,
        now(),
    )
    .unwrap();
    assert!(core.next_reminder().unwrap().is_none());
}
#[test]
fn invalid_settings_dont_mutate() {
    let mut core = Core::memory().unwrap();
    let mut s = core.settings().unwrap();
    s.timezone = "not/a-zone".into();
    assert!(core
        .execute(Command::SaveSettings(s), Actor::Ui, now())
        .is_err());
    assert_eq!(core.event_count().unwrap(), 0);
    assert_eq!(core.settings().unwrap().timezone, "UTC");
}

fn edit_for(task: &Task) -> TaskEdit {
    TaskEdit {
        id: task.id.clone(),
        title: task.title.clone(),
        notes: task.notes.clone(),
        status: task.status,
        priority: task.priority,
        energy: task.energy,
        est_min: task.est_min,
        actual_min: task.actual_min,
        tags: task.tags.clone(),
        parent_id: task.parent_id.clone(),
        project_id: task.project_id.clone(),
        goal_id: task.goal_id.clone(),
        recurrence_rule: task.recurrence_rule.clone(),
        scheduled_local: task
            .scheduled_start
            .map(|d| d.format("%Y-%m-%dT%H:%M").to_string()),
        due_local: task.due_at.map(|d| d.format("%Y-%m-%dT%H:%M").to_string()),
        planned_day: task.planned_day,
    }
}
#[test]
fn subtask_cycles_are_rejected() {
    let mut core = Core::memory().unwrap();
    let parent = create(&mut core, "Parent");
    let child = match core
        .execute(
            Command::CreateTask {
                text: "Child".into(),
                parent_id: Some(parent.id.clone()),
            },
            Actor::User,
            now(),
        )
        .unwrap()
    {
        Response::Task(t) => t,
        _ => panic!(),
    };
    let mut edit = edit_for(&parent);
    edit.parent_id = Some(child.id);
    assert!(core
        .execute(Command::UpdateTask(edit), Actor::User, now())
        .is_err());
    assert_eq!(core.event_count().unwrap(), 2);
}
#[test]
fn audit_failure_rolls_back_data_and_fts() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("rollback.db");
    let mut core = Core::open(&path).unwrap();
    let task = create(&mut core, "Original title");
    let conn = rusqlite::Connection::open(&path).unwrap();
    conn.execute_batch("CREATE TRIGGER reject_event BEFORE INSERT ON events BEGIN SELECT RAISE(ABORT,'injected failure'); END;").unwrap();
    let mut edit = edit_for(&task);
    edit.title = "Changed title".into();
    assert!(core
        .execute(Command::UpdateTask(edit), Actor::User, now())
        .is_err());
    assert_eq!(core.tasks().unwrap()[0].title, "Original title");
    assert_eq!(core.event_count().unwrap(), 1);
    let found: i64 = conn
        .query_row(
            "SELECT count(*) FROM task_search WHERE task_search MATCH 'Original'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(found, 1);
    assert!(conn.execute("DELETE FROM events", []).is_err());
    assert!(conn
        .execute("UPDATE events SET action='tamper'", [])
        .is_err());
}
#[test]
fn capacity_excludes_completed_and_deleted() {
    let mut core = Core::memory().unwrap();
    let done = create(&mut core, "Done 60m today");
    let deleted = create(&mut core, "Deleted 60m today");
    create(&mut core, "Active 45m today");
    core.execute(
        Command::CompleteTask {
            id: done.id,
            completed: true,
        },
        Actor::User,
        now(),
    )
    .unwrap();
    core.execute(
        Command::DeleteTask {
            id: deleted.id,
            dry_run: false,
        },
        Actor::User,
        now(),
    )
    .unwrap();
    match core
        .execute(Command::Snapshot { search: None }, Actor::User, now())
        .unwrap()
    {
        Response::Snapshot(s) => {
            assert_eq!(s.capacity.count, 1);
            assert_eq!(s.capacity.minutes, 45);
        }
        _ => panic!(),
    }
    assert_eq!(core.today_count(now()).unwrap(), 1);
}
#[test]
fn schedule_uses_local_timezone_and_duration_tracks_estimate() {
    let mut core = Core::memory().unwrap();
    let t = create(&mut core, "Read 30m");
    let mut s = core.settings().unwrap();
    s.timezone = "Asia/Kolkata".into();
    core.execute(Command::SaveSettings(s), Actor::User, now())
        .unwrap();
    core.execute(
        Command::RescheduleTask {
            id: t.id,
            local_start: Some("2026-11-03T00:15".into()),
            day: None,
            dry_run: false,
        },
        Actor::User,
        now(),
    )
    .unwrap();
    let scheduled = &core.tasks().unwrap()[0];
    assert_eq!(
        scheduled.scheduled_start.unwrap().to_rfc3339(),
        "2026-11-02T18:45:00+00:00"
    );
    assert_eq!(scheduled.planned_day.unwrap().to_string(), "2026-11-03");
    let mut edit = edit_for(scheduled);
    edit.scheduled_local = Some("2026-11-03T00:15".into());
    edit.est_min = 90;
    edit.tags = vec!["#read".into(), "#read".into()];
    core.execute(Command::UpdateTask(edit), Actor::User, now())
        .unwrap();
    let tasks = core.tasks().unwrap();
    assert_eq!(tasks[0].tags.len(), 1);
    assert_eq!(
        (tasks[0].scheduled_end.unwrap() - tasks[0].scheduled_start.unwrap()).num_minutes(),
        90
    );
}
#[test]
fn rollover_observes_timezone_calendar_day() {
    let mut core = Core::memory().unwrap();
    create(&mut core, "Read today");
    let mut s = core.settings().unwrap();
    s.timezone = "Asia/Kolkata".into();
    core.execute(Command::SaveSettings(s), Actor::User, now())
        .unwrap();
    core.execute(
        Command::Rollover { dry_run: false },
        Actor::Ui,
        "2026-11-02T19:00:00Z".parse().unwrap(),
    )
    .unwrap();
    assert_eq!(
        core.tasks().unwrap()[0].planned_day.unwrap().to_string(),
        "2026-11-03"
    );
    assert_eq!(core.tasks().unwrap()[0].carried_count, 1);
}
#[test]
fn projects_goals_templates_and_filters_round_trip() {
    let mut core = Core::memory().unwrap();
    let project = match core
        .execute(
            Command::CreateProject {
                name: "Daybridge".into(),
                kind: ProjectKind::Coding,
                repo_path: Some("C:/src/daybridge".into()),
                branch: Some("main".into()),
            },
            Actor::User,
            now(),
        )
        .unwrap()
    {
        Response::Project(p) => p,
        _ => panic!(),
    };
    let goal = match core
        .execute(
            Command::CreateGoal {
                title: "Ship phase 3".into(),
                horizon: GoalHorizon::Medium,
                target_date: Some("2026-11-20".parse().unwrap()),
                note: "Keep it local".into(),
            },
            Actor::User,
            now(),
        )
        .unwrap()
    {
        Response::Goal(g) => g,
        _ => panic!(),
    };
    let template = match core
        .execute(
            Command::CreateTemplate {
                name: "Bug fix".into(),
                title: "Fix bug".into(),
                notes: "Reproduce, fix, test".into(),
                priority: 2,
                energy: Energy::Low,
                est_min: 30,
                tags: vec!["!bug".into()],
                project_id: Some(project.id.clone()),
            },
            Actor::User,
            now(),
        )
        .unwrap()
    {
        Response::Template(t) => t,
        _ => panic!(),
    };
    let filter = match core
        .execute(
            Command::SaveFilter {
                name: "Graph search".into(),
                view: "Inbox".into(),
                query: "graph".into(),
            },
            Actor::User,
            now(),
        )
        .unwrap()
    {
        Response::Filter(f) => f,
        _ => panic!(),
    };
    let task = create(&mut core, "Linked task");
    let mut edit = edit_for(&task);
    edit.project_id = Some(project.id.clone());
    edit.goal_id = Some(goal.id.clone());
    core.execute(Command::UpdateTask(edit), Actor::User, now())
        .unwrap();
    let from_template = match core
        .execute(
            Command::CreateTaskFromTemplate {
                template_id: template.id.clone(),
                parent_id: None,
            },
            Actor::User,
            now(),
        )
        .unwrap()
    {
        Response::Task(t) => t,
        _ => panic!(),
    };
    assert_eq!(
        from_template.project_id.as_deref(),
        Some(project.id.as_str())
    );
    match core
        .execute(Command::Snapshot { search: None }, Actor::User, now())
        .unwrap()
    {
        Response::Snapshot(s) => {
            assert_eq!(s.projects.len(), 1);
            assert_eq!(s.goals.len(), 1);
            assert_eq!(s.templates.len(), 1);
            assert_eq!(s.saved_filters.len(), 1);
            assert_eq!(s.saved_filters[0].id, filter.id);
            let progress = s
                .goal_progress
                .iter()
                .find(|p| p.goal_id == goal.id)
                .unwrap();
            assert_eq!((progress.total, progress.done), (1, 0));
        }
        _ => panic!(),
    }
    core.execute(
        Command::DeleteProject { id: project.id },
        Actor::User,
        now(),
    )
    .unwrap();
    core.execute(Command::DeleteGoal { id: goal.id }, Actor::User, now())
        .unwrap();
    core.execute(
        Command::DeleteTemplate {
            id: template.id.clone(),
        },
        Actor::User,
        now(),
    )
    .unwrap();
    core.execute(Command::DeleteFilter { id: filter.id }, Actor::User, now())
        .unwrap();
    match core
        .execute(Command::Snapshot { search: None }, Actor::User, now())
        .unwrap()
    {
        Response::Snapshot(s) => {
            assert!(s.projects.is_empty());
            assert!(s.goals.is_empty());
            assert!(s.templates.is_empty());
            assert!(s.saved_filters.is_empty());
            assert!(s.tasks.iter().all(|t| t.deleted_at.is_none()));
        }
        _ => panic!(),
    }
    assert!(core
        .execute(
            Command::CreateTaskFromTemplate {
                template_id: template.id,
                parent_id: None,
            },
            Actor::User,
            now(),
        )
        .is_err());
}
#[test]
fn github_issue_urls_create_tasks_and_reject_junk() {
    let mut core = Core::memory().unwrap();
    let project = match core
        .execute(
            Command::CreateProject {
                name: "Bridge".into(),
                kind: ProjectKind::Coding,
                repo_path: Some("C:/src/bridge".into()),
                branch: None,
            },
            Actor::User,
            now(),
        )
        .unwrap()
    {
        Response::Project(p) => p,
        _ => panic!(),
    };
    for url in [
        "https://github.com/acme/bridge/issues/42",
        "github.com/acme/bridge/issues/7?foo=1",
    ] {
        match core
            .execute(Command::ImportIssue { url: url.into() }, Actor::User, now())
            .unwrap()
        {
            Response::Task(t) => {
                assert_eq!(t.project_id.as_deref(), Some(project.id.as_str()));
                assert!(t.notes.contains(url));
            }
            _ => panic!(),
        }
    }
    for url in [
        "",
        "https://example.com/x",
        "https://github.com/acme/bridge/pull/1",
    ] {
        assert!(core
            .execute(Command::ImportIssue { url: url.into() }, Actor::User, now())
            .is_err());
    }
}
#[test]
fn existing_database_gains_phase_three_tables() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("legacy.db");
    {
        let conn = rusqlite::Connection::open(&path).unwrap();
        conn.execute_batch(include_str!("../src/schema.sql"))
            .unwrap();
        conn.execute_batch(
            r#"INSERT INTO tasks(id,data,status,planned_day,due_at,created_at,deleted) VALUES('01LEGACY','{"id":"01LEGACY","title":"Old task","notes":"","status":"todo","priority":3,"energy":"low","est_min":25,"actual_min":0,"due_at":null,"planned_day":null,"scheduled_start":null,"scheduled_end":null,"parent_id":null,"tags":[],"carried_count":0,"created_at":"2026-11-01T10:00:00Z","completed_at":null,"deleted_at":null,"reminded_at":null}','"todo"',NULL,NULL,'2026-11-01T10:00:00Z',0);
             PRAGMA user_version=1;"#,
        )
        .unwrap();
    }
    let mut core = Core::open(&path).unwrap();
    let tasks = core.tasks().unwrap();
    assert_eq!(tasks.len(), 1);
    assert!(tasks[0].project_id.is_none());
    assert!(tasks[0].dsa_problem_id.is_none());
    assert!(core.projects().unwrap().is_empty());
    assert!(core.dsa_problems().unwrap().is_empty());
    assert!(core.subjects().unwrap().is_empty());
    assert!(core.deadlines().unwrap().is_empty());
    assert!(core.focus_sessions().unwrap().is_empty());
    assert!(core.habits().unwrap().is_empty());
    assert!(core.daily_reviews().unwrap().is_empty());
    assert!(core
        .execute(
            Command::CreateProject {
                name: "Migrated".into(),
                kind: ProjectKind::Personal,
                repo_path: None,
                branch: None,
            },
            Actor::User,
            now(),
        )
        .is_ok());
}
fn dsa(core: &mut Core, title: &str, topic: &str) -> DsaProblem {
    match core
        .execute(
            Command::CreateDsaProblem {
                title: title.into(),
                url: None,
                topic: topic.into(),
                difficulty: DsaDifficulty::Medium,
                platform: None,
                notes: String::new(),
            },
            Actor::User,
            now(),
        )
        .unwrap()
    {
        Response::DsaProblem(p) => p,
        _ => panic!(),
    }
}
#[test]
fn dsa_solve_and_review_schedule_revisions() {
    let mut core = Core::memory().unwrap();
    let problem = dsa(&mut core, "Two Sum", "arrays");
    assert_eq!(problem.status, DsaStatus::Todo);
    let solved = match core
        .execute(
            Command::SolveDsaProblem {
                id: problem.id.clone(),
                confidence: 3,
            },
            Actor::User,
            now(),
        )
        .unwrap()
    {
        Response::DsaProblem(p) => p,
        _ => panic!(),
    };
    assert_eq!(solved.status, DsaStatus::Solved);
    assert_eq!((solved.review_stage, solved.confidence), (1, 3));
    assert_eq!(solved.next_review_at.unwrap().to_string(), "2026-11-03");
    let revisions: Vec<_> = core
        .tasks()
        .unwrap()
        .into_iter()
        .filter(|t| t.dsa_problem_id.as_deref() == Some(problem.id.as_str()))
        .collect();
    assert_eq!(revisions.len(), 1);
    assert_eq!(revisions[0].title, "Revise: Two Sum");
    assert_eq!(revisions[0].planned_day.unwrap().to_string(), "2026-11-03");
    assert_eq!(revisions[0].status, Status::Todo);
    let reviewed = match core
        .execute(
            Command::ReviewDsaProblem {
                id: problem.id.clone(),
                confidence: 5,
            },
            Actor::User,
            now(),
        )
        .unwrap()
    {
        Response::DsaProblem(p) => p,
        _ => panic!(),
    };
    assert_eq!(reviewed.review_stage, 2);
    assert_eq!(reviewed.next_review_at.unwrap().to_string(), "2026-11-12");
    let all = core.tasks().unwrap();
    let revisions: Vec<_> = all
        .iter()
        .filter(|t| t.dsa_problem_id.as_deref() == Some(problem.id.as_str()))
        .collect();
    assert_eq!(revisions.len(), 2);
    assert_eq!(
        revisions
            .iter()
            .filter(|t| t.status == Status::Done)
            .count(),
        1
    );
    assert_eq!(revisions.iter().filter(|t| t.status.active()).count(), 1);
    assert!(core
        .execute(
            Command::SolveDsaProblem {
                id: problem.id.clone(),
                confidence: 3,
            },
            Actor::User,
            now(),
        )
        .is_err());
    assert!(core
        .execute(
            Command::ReviewDsaProblem {
                id: problem.id,
                confidence: 9,
            },
            Actor::User,
            now(),
        )
        .is_err());
}
#[test]
fn dsa_confidence_adjusts_interval_and_topics_are_derived() {
    let mut core = Core::memory().unwrap();
    let easy = dsa(&mut core, "Binary Search", "arrays");
    let hard = dsa(&mut core, "Course Schedule", "graphs");
    let low = match core
        .execute(
            Command::SolveDsaProblem {
                id: easy.id.clone(),
                confidence: 1,
            },
            Actor::User,
            now(),
        )
        .unwrap()
    {
        Response::DsaProblem(p) => p,
        _ => panic!(),
    };
    assert_eq!(low.next_review_at.unwrap().to_string(), "2026-11-03");
    let high = match core
        .execute(
            Command::SolveDsaProblem {
                id: hard.id,
                confidence: 5,
            },
            Actor::User,
            now(),
        )
        .unwrap()
    {
        Response::DsaProblem(p) => p,
        _ => panic!(),
    };
    assert_eq!(high.next_review_at.unwrap().to_string(), "2026-11-10");
    dsa(&mut core, "Sliding Window", "arrays");
    match core
        .execute(Command::Snapshot { search: None }, Actor::User, now())
        .unwrap()
    {
        Response::Snapshot(s) => {
            let arrays = s.dsa_topics.iter().find(|t| t.topic == "arrays").unwrap();
            assert_eq!((arrays.total, arrays.solved), (2, 1));
            assert_eq!(arrays.average_confidence, 1.0);
            let graphs = s.dsa_topics.iter().find(|t| t.topic == "graphs").unwrap();
            assert_eq!((graphs.total, graphs.solved), (1, 1));
            assert_eq!(graphs.average_confidence, 5.0);
        }
        _ => panic!(),
    }
}
#[test]
fn deadlines_back_schedule_study_tasks() {
    let mut core = Core::memory().unwrap();
    let subject = match core
        .execute(
            Command::CreateSubject {
                name: "DBMS".into(),
            },
            Actor::User,
            now(),
        )
        .unwrap()
    {
        Response::Subject(s) => s,
        _ => panic!(),
    };
    let _deadline = match core
        .execute(
            Command::CreateDeadline {
                subject_id: Some(subject.id.clone()),
                title: "Midterm".into(),
                due_date: "2026-11-06".parse().unwrap(),
                study_min: 120,
            },
            Actor::User,
            now(),
        )
        .unwrap()
    {
        Response::Deadline(d) => d,
        _ => panic!(),
    };
    let mut days: Vec<_> = core
        .tasks()
        .unwrap()
        .into_iter()
        .filter(|t| t.title == "Study: Midterm")
        .map(|t| (t.planned_day.unwrap().to_string(), t.est_min))
        .collect();
    days.sort();
    assert_eq!(
        days,
        vec![
            ("2026-11-02".into(), 30),
            ("2026-11-03".into(), 30),
            ("2026-11-04".into(), 30),
            ("2026-11-05".into(), 30),
        ]
    );
    assert!(days.iter().all(|(day, _)| day.as_str() < "2026-11-06"));
    core.execute(
        Command::CreateDeadline {
            subject_id: None,
            title: "Quiz".into(),
            due_date: "2026-11-01".parse().unwrap(),
            study_min: 45,
        },
        Actor::User,
        now(),
    )
    .unwrap();
    let quiz: Vec<_> = core
        .tasks()
        .unwrap()
        .into_iter()
        .filter(|t| t.title == "Study: Quiz")
        .collect();
    assert_eq!(quiz.len(), 1);
    assert_eq!(quiz[0].est_min, 45);
    assert_eq!(quiz[0].planned_day.unwrap().to_string(), "2026-11-02");
}
#[test]
fn subjects_and_syllabus_toggle_and_soft_delete() {
    let mut core = Core::memory().unwrap();
    let subject = match core
        .execute(
            Command::CreateSubject {
                name: "Operating Systems".into(),
            },
            Actor::User,
            now(),
        )
        .unwrap()
    {
        Response::Subject(s) => s,
        _ => panic!(),
    };
    let item = match core
        .execute(
            Command::AddSyllabusItem {
                subject_id: subject.id.clone(),
                title: "Deadlocks".into(),
            },
            Actor::User,
            now(),
        )
        .unwrap()
    {
        Response::SyllabusItem(i) => i,
        _ => panic!(),
    };
    core.execute(
        Command::SetSyllabusItemDone {
            id: item.id.clone(),
            done: true,
        },
        Actor::User,
        now(),
    )
    .unwrap();
    match core
        .execute(Command::Snapshot { search: None }, Actor::User, now())
        .unwrap()
    {
        Response::Snapshot(s) => {
            assert_eq!(s.subjects.len(), 1);
            assert_eq!(s.syllabus_items.len(), 1);
            assert!(s.syllabus_items[0].done);
        }
        _ => panic!(),
    }
    core.execute(
        Command::DeleteSubject {
            id: subject.id.clone(),
        },
        Actor::User,
        now(),
    )
    .unwrap();
    assert!(core
        .execute(
            Command::AddSyllabusItem {
                subject_id: subject.id,
                title: "Scheduling".into(),
            },
            Actor::User,
            now(),
        )
        .is_err());
    match core
        .execute(Command::Snapshot { search: None }, Actor::User, now())
        .unwrap()
    {
        Response::Snapshot(s) => assert!(s.subjects.is_empty()),
        _ => panic!(),
    }
}
#[test]
fn parser_extracts_recurrence_tokens() {
    for (input, title, expected) in [
        ("Write repeat daily", "Write", Some("daily")),
        ("Review repeat weekdays", "Review", Some("weekdays")),
        ("Standup repeat weekly", "Standup", Some("weekly")),
        ("Pay rent repeat monthly", "Pay rent", Some("monthly")),
        ("Run every 3 days", "Run", Some("every 3 days")),
        ("Read every day", "Read", Some("daily")),
    ] {
        let parsed = parser::parse(input, now(), chrono_tz::UTC).unwrap();
        assert_eq!(parsed.title, title, "{input}");
        assert_eq!(parsed.recurrence.as_deref(), expected, "{input}");
    }
    for input in ["Daily standup", "Weekly review", "Monthly report"] {
        let parsed = parser::parse(input, now(), chrono_tz::UTC).unwrap();
        assert_eq!(parsed.title, input);
        assert!(parsed.recurrence.is_none());
    }
    let plain = parser::parse("Read a book", now(), chrono_tz::UTC).unwrap();
    assert!(plain.recurrence.is_none());
}
#[test]
fn focus_session_rolls_minutes_into_task() {
    let mut core = Core::memory().unwrap();
    let task = create(&mut core, "Write tests");
    let session = match core
        .execute(
            Command::StartFocus {
                task_id: Some(task.id.clone()),
                planned_min: 25,
                kind: FocusKind::Focus,
            },
            Actor::User,
            now(),
        )
        .unwrap()
    {
        Response::Focus(session) => session,
        _ => panic!(),
    };
    assert!(core
        .execute(
            Command::StartFocus {
                task_id: None,
                planned_min: 5,
                kind: FocusKind::Break,
            },
            Actor::User,
            now(),
        )
        .is_err());
    let later = now() + chrono::Duration::minutes(30);
    let stopped = match core
        .execute(
            Command::StopFocus {
                id: session.id.clone(),
            },
            Actor::User,
            later,
        )
        .unwrap()
    {
        Response::Focus(session) => session,
        _ => panic!(),
    };
    assert!(stopped.ended_at.is_some());
    let after = core
        .tasks()
        .unwrap()
        .into_iter()
        .find(|task_after| task_after.id == task.id)
        .unwrap();
    assert_eq!(after.actual_min, 30);
    assert!(core
        .execute(Command::StopFocus { id: session.id }, Actor::User, later)
        .is_err());
    match core
        .execute(Command::Snapshot { search: None }, Actor::User, later)
        .unwrap()
    {
        Response::Snapshot(s) => {
            assert!(s.active_focus.is_none());
            assert_eq!(s.weekly_stats.focus_minutes, 30);
        }
        _ => panic!(),
    }
}
#[test]
fn habits_log_and_streaks() {
    let mut core = Core::memory().unwrap();
    let habit = match core
        .execute(
            Command::CreateHabit {
                name: "DSA daily".into(),
                cadence_days: 1,
            },
            Actor::User,
            now(),
        )
        .unwrap()
    {
        Response::Habit(habit) => habit,
        _ => panic!(),
    };
    for day in ["2026-10-31", "2026-11-01", "2026-11-02"] {
        core.execute(
            Command::LogHabit {
                habit_id: habit.id.clone(),
                day: Some(day.parse().unwrap()),
            },
            Actor::User,
            now(),
        )
        .unwrap();
    }
    assert!(core
        .execute(
            Command::LogHabit {
                habit_id: habit.id.clone(),
                day: Some("2026-11-02".parse().unwrap()),
            },
            Actor::User,
            now(),
        )
        .is_err());
    match core
        .execute(Command::Snapshot { search: None }, Actor::User, now())
        .unwrap()
    {
        Response::Snapshot(s) => {
            let streak = s
                .habit_streaks
                .iter()
                .find(|streak| streak.habit_id == habit.id)
                .unwrap();
            assert_eq!((streak.current, streak.best), (3, 3));
            assert_eq!(streak.last_done.unwrap().to_string(), "2026-11-02");
            assert!(!streak.due);
        }
        _ => panic!(),
    }
}
fn recurring_task(core: &mut Core, title: &str, rule: &str) -> Task {
    let task = create(core, title);
    let mut edit = edit_for(&task);
    edit.recurrence_rule = Some(rule.into());
    core.execute(Command::UpdateTask(edit), Actor::User, now())
        .unwrap();
    task
}
#[test]
fn recurrence_creates_the_next_occurrence() {
    for (rule, completed_at, expected) in [
        ("daily", "2026-11-02T10:00:00Z", "2026-11-03"),
        ("weekdays", "2026-11-02T10:00:00Z", "2026-11-03"),
        ("weekdays", "2026-11-06T10:00:00Z", "2026-11-09"),
        ("weekly", "2026-11-02T10:00:00Z", "2026-11-09"),
        ("monthly", "2026-11-02T10:00:00Z", "2026-12-02"),
        ("every 3 days", "2026-11-02T10:00:00Z", "2026-11-05"),
    ] {
        let mut core = Core::memory().unwrap();
        let task = recurring_task(&mut core, "Recurring task", rule);
        core.execute(
            Command::CompleteTask {
                id: task.id.clone(),
                completed: true,
            },
            Actor::User,
            completed_at.parse().unwrap(),
        )
        .unwrap();
        let next = core
            .tasks()
            .unwrap()
            .into_iter()
            .find(|candidate| candidate.title == "Recurring task" && candidate.id != task.id)
            .expect(rule);
        assert_eq!(
            next.planned_day.unwrap().to_string(),
            expected,
            "{rule} completed at {completed_at}"
        );
        assert_eq!(next.recurrence_rule.as_deref(), Some(rule));
        assert_eq!(next.status, Status::Todo);
    }
}
#[test]
fn reviews_upsert_and_weekly_stats_are_derived() {
    let mut core = Core::memory().unwrap();
    let first = match core
        .execute(
            Command::SaveDailyReview {
                day: None,
                note: "Good day".into(),
            },
            Actor::User,
            now(),
        )
        .unwrap()
    {
        Response::Review(review) => review,
        _ => panic!(),
    };
    let second = match core
        .execute(
            Command::SaveDailyReview {
                day: None,
                note: "Updated".into(),
            },
            Actor::User,
            now(),
        )
        .unwrap()
    {
        Response::Review(review) => review,
        _ => panic!(),
    };
    assert_eq!(first.id, second.id);
    assert_eq!(core.daily_reviews().unwrap().len(), 1);
    assert_eq!(core.daily_reviews().unwrap()[0].note, "Updated");
    assert!(core
        .execute(
            Command::SaveDailyReview {
                day: Some("2026-11-05".parse().unwrap()),
                note: "Future".into(),
            },
            Actor::User,
            now(),
        )
        .is_err());
    let task = create(&mut core, "Write tests #dsa");
    let session = match core
        .execute(
            Command::StartFocus {
                task_id: Some(task.id.clone()),
                planned_min: 25,
                kind: FocusKind::Focus,
            },
            Actor::User,
            now(),
        )
        .unwrap()
    {
        Response::Focus(session) => session,
        _ => panic!(),
    };
    let later = now() + chrono::Duration::minutes(30);
    core.execute(Command::StopFocus { id: session.id }, Actor::User, later)
        .unwrap();
    core.execute(
        Command::CompleteTask {
            id: task.id,
            completed: true,
        },
        Actor::User,
        later,
    )
    .unwrap();
    match core
        .execute(Command::Snapshot { search: None }, Actor::User, later)
        .unwrap()
    {
        Response::Snapshot(s) => {
            assert_eq!(s.weekly_stats.completed, 1);
            assert_eq!(s.weekly_stats.focus_minutes, 30);
            assert_eq!(s.weekly_stats.measured, 1);
            assert!((s.weekly_stats.estimate_accuracy - 0.8).abs() < 0.001);
            let tag = s
                .weekly_stats
                .tag_stats
                .iter()
                .find(|stat| stat.tag == "#dsa")
                .unwrap();
            assert_eq!((tag.focus_minutes, tag.completed), (30, 1));
        }
        _ => panic!(),
    }
}
#[test]
fn export_import_round_trip_skips_existing_ids() {
    let mut core = Core::memory().unwrap();
    create(&mut core, "One #work");
    create(&mut core, "Two");
    core.execute(
        Command::CreateProject {
            name: "Daybridge".into(),
            kind: ProjectKind::Coding,
            repo_path: None,
            branch: None,
        },
        Actor::User,
        now(),
    )
    .unwrap();
    core.execute(
        Command::CreateHabit {
            name: "Gym".into(),
            cadence_days: 2,
        },
        Actor::User,
        now(),
    )
    .unwrap();
    let json = core.export_json().unwrap();
    let mut other = Core::memory().unwrap();
    let summary = other.import_json(&json, Actor::User, now()).unwrap();
    assert_eq!(summary.tasks, 2);
    assert_eq!(summary.projects, 1);
    assert_eq!(summary.habits, 1);
    assert_eq!(other.tasks().unwrap().len(), 2);
    let again = other.import_json(&json, Actor::User, now()).unwrap();
    assert_eq!(again.tasks, 0);
    assert!(again.skipped >= 2);
    match other
        .execute(
            Command::Snapshot {
                search: Some("One".into()),
            },
            Actor::User,
            now(),
        )
        .unwrap()
    {
        Response::Snapshot(s) => assert_eq!(s.tasks.len(), 1),
        _ => panic!(),
    }
    let markdown = core.export_markdown().unwrap();
    assert!(markdown.contains("One"));
    assert!(markdown.contains("## Projects"));
}
#[test]
fn backup_creates_a_readable_copy() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("source.db");
    let mut core = Core::open(&source).unwrap();
    create(&mut core, "Backup me");
    let backup = dir.path().join("backup.db");
    core.backup_to(&backup).unwrap();
    let copy = Core::open(&backup).unwrap();
    assert_eq!(copy.tasks().unwrap().len(), 1);
}
