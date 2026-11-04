use crate::model::*;
use crate::schedule;
use chrono::{Duration, NaiveDate};
use chrono_tz::Tz;

pub struct LoadInput<'a> {
    pub settings: &'a Settings,
    pub items: &'a [ScheduleItem],
    pub exceptions: &'a [ScheduleException],
    pub terms: &'a [Term],
    pub events: &'a [AcademicEvent],
    pub tasks: &'a [Task],
    pub tz: Tz,
}

const BUFFER_MIN: u32 = 0;
const EXAM_CLUSTER_DAYS: i64 = 3;
const PREP_TASK_MIN: u32 = 45;

pub fn occurrences_for(input: &LoadInput, from: NaiveDate, to: NaiveDate) -> Vec<Occurrence> {
    schedule::occurrences(
        from,
        to,
        input.items,
        input.exceptions,
        input.terms,
        input.events,
        input.tz,
    )
}

fn window_minutes(settings: &Settings) -> u32 {
    if settings.work_end > settings.work_start {
        (settings.work_end - settings.work_start) as u32 * 60
    } else {
        0
    }
}

fn active_term<'a>(input: &'a LoadInput) -> Option<&'a Term> {
    input
        .terms
        .iter()
        .find(|term| term.active && !term.archived)
}

fn event_in_scope(input: &LoadInput, event: &AcademicEvent) -> bool {
    active_term(input).is_some_and(|term| term.id == event.term_id)
}

fn event_on(event: &AcademicEvent, date: NaiveDate, tz: Tz) -> bool {
    let start = event.start_at.with_timezone(&tz).date_naive();
    let end = event.end_at.with_timezone(&tz).date_naive();
    if event.all_day {
        date >= start && date <= end
    } else {
        date == start
    }
}

fn event_minutes(event: &AcademicEvent, date: NaiveDate, tz: Tz) -> u32 {
    if event.all_day || event.start_at.with_timezone(&tz).date_naive() != date {
        return 0;
    }
    (event.end_at - event.start_at).num_minutes().max(0) as u32
}

fn exams_on<'a>(input: &'a LoadInput, date: NaiveDate) -> Vec<&'a AcademicEvent> {
    input
        .events
        .iter()
        .filter(|event| event_in_scope(input, event))
        .filter(|event| event.kind.is_exam() && event_on(event, date, input.tz))
        .collect()
}

fn holiday_on(input: &LoadInput, date: NaiveDate) -> bool {
    input
        .events
        .iter()
        .filter(|event| event_in_scope(input, event))
        .any(|event| event.kind == AcademicKind::Holiday && event_on(event, date, input.tz))
}

fn in_prep_window(input: &LoadInput, date: NaiveDate) -> bool {
    input.events.iter().any(|event| {
        if !event.kind.is_exam() || !event_in_scope(input, event) {
            return false;
        }
        let exam_day = event.start_at.with_timezone(&input.tz).date_naive();
        let prep_days = event.prep_days.max(input.settings.prep_window_days);
        date < exam_day && date >= exam_day - Duration::days(prep_days as i64)
    })
}

pub fn estimate_calibration(tasks: &[Task], tag: &str, min_samples: u32) -> Option<f64> {
    let ratios: Vec<f64> = tasks
        .iter()
        .filter(|task| task.status == Status::Done && task.est_min > 0 && task.actual_min > 0)
        .filter(|task| task.tags.iter().any(|candidate| candidate == tag))
        .map(|task| task.actual_min as f64 / task.est_min as f64)
        .collect();
    if ratios.len() < min_samples.max(1) as usize || ratios.is_empty() {
        return None;
    }
    let mean = ratios.iter().sum::<f64>() / ratios.len() as f64;
    Some(mean.clamp(0.7, 2.0))
}

pub fn task_minutes(input: &LoadInput, task: &Task) -> u32 {
    if !input.settings.calibrate_estimates {
        return task.est_min;
    }
    for tag in &task.tags {
        if let Some(factor) =
            estimate_calibration(input.tasks, tag, input.settings.calibration_min_samples)
        {
            return ((task.est_min as f64) * factor).round().max(1.0) as u32;
        }
    }
    task.est_min
}

fn planned_tasks<'a>(input: &'a LoadInput, date: NaiveDate) -> Vec<&'a Task> {
    input
        .tasks
        .iter()
        .filter(|task| {
            task.deleted_at.is_none() && task.status.active() && task.planned_day == Some(date)
        })
        .collect()
}

fn used_minutes(input: &LoadInput, date: NaiveDate) -> u32 {
    planned_tasks(input, date)
        .iter()
        .map(|task| task_minutes(input, task))
        .sum()
}

pub fn day_status(input: &LoadInput, date: NaiveDate) -> DayStatus {
    let window = window_minutes(input.settings);
    let occurrences = occurrences_for(input, date, date);
    let schedule_min = schedule::minutes(&occurrences);
    let events: Vec<&AcademicEvent> = input
        .events
        .iter()
        .filter(|event| event_in_scope(input, event) && event_on(event, date, input.tz))
        .collect();
    let event_min: u32 = events
        .iter()
        .map(|event| event_minutes(event, date, input.tz))
        .sum();
    let mut reasons = Vec::new();
    let mut available = window
        .saturating_sub(schedule_min)
        .saturating_sub(event_min)
        .saturating_sub(BUFFER_MIN);
    let used = used_minutes(input, date);
    let exam = exams_on(input, date);
    let holiday = holiday_on(input, date);
    let exam_tomorrow = !exams_on(input, date + Duration::days(1)).is_empty();
    let prep = in_prep_window(input, date);
    let status = if !exam.is_empty() {
        available = 0;
        reasons.push(DayReason {
            rule_id: "exam_day".into(),
            message: format!("Exam day: {}", exam[0].title),
        });
        DayStatusKind::Blocked
    } else if holiday {
        available = 0;
        reasons.push(DayReason {
            rule_id: "holiday".into(),
            message: "Holiday: classes suppressed".into(),
        });
        DayStatusKind::Blocked
    } else if exam_tomorrow {
        available = available * input.settings.day_before_exam_capacity_pct / 100;
        reasons.push(DayReason {
            rule_id: "day_before_exam".into(),
            message: format!(
                "Day before an exam: capacity reduced to {}%",
                input.settings.day_before_exam_capacity_pct
            ),
        });
        DayStatusKind::PrepCritical
    } else if prep {
        reasons.push(DayReason {
            rule_id: "prep_window".into(),
            message: "Exam preparation window".into(),
        });
        DayStatusKind::PrepCritical
    } else if available > 0
        && used * 100 >= available.saturating_mul(input.settings.heavy_day_threshold_pct)
    {
        reasons.push(DayReason {
            rule_id: "heavy_day".into(),
            message: "Day is at or above the heavy-day threshold".into(),
        });
        DayStatusKind::Heavy
    } else if used == 0 && schedule_min == 0 && event_min == 0 {
        reasons.push(DayReason {
            rule_id: "free_day".into(),
            message: "No classes and nothing planned".into(),
        });
        DayStatusKind::Free
    } else {
        DayStatusKind::Normal
    };
    DayStatus {
        date,
        status,
        available_min: available,
        used_min: used,
        reasons,
    }
}

pub fn available_minutes(input: &LoadInput, date: NaiveDate) -> u32 {
    day_status(input, date).available_min
}

pub fn week_load(input: &LoadInput, from: NaiveDate, to: NaiveDate) -> Vec<WeekLoadEntry> {
    let mut out = Vec::new();
    let mut date = from;
    while date <= to {
        let status = day_status(input, date);
        let schedule_min = schedule::minutes(&occurrences_for(input, date, date));
        out.push(WeekLoadEntry {
            date,
            status: status.status,
            available_min: status.available_min,
            used_min: status.used_min,
            schedule_min,
        });
        date += Duration::days(1);
    }
    out
}

pub fn start_by_date(input: &LoadInput, task: &Task) -> Option<NaiveDate> {
    let due = task.due_at?.with_timezone(&input.tz).date_naive();
    let needed = task_minutes(input, task);
    let mut accumulated = 0;
    let mut day = due - Duration::days(1);
    for _ in 0..90 {
        if day < due {
            let status = day_status(input, day);
            if status.status != DayStatusKind::Blocked {
                accumulated += status.available_min.saturating_sub(status.used_min);
                if accumulated >= needed {
                    return Some(day);
                }
            }
            day -= Duration::days(1);
        }
    }
    Some(due - Duration::days(90))
}

fn next_open_day(
    input: &LoadInput,
    from: NaiveDate,
    minutes: u32,
    blocked: impl Fn(NaiveDate) -> bool,
) -> NaiveDate {
    let mut day = from;
    for _ in 0..60 {
        if !blocked(day) {
            let status = day_status(input, day);
            if status.status != DayStatusKind::Blocked
                && status.available_min.saturating_sub(status.used_min) >= minutes
            {
                return day;
            }
        }
        day += Duration::days(1);
    }
    from
}

pub fn conflicts(
    input: &LoadInput,
    from: NaiveDate,
    to: NaiveDate,
    dismissed: &[DismissedConflict],
) -> Vec<Conflict> {
    let mut out = Vec::new();
    let dismissed_rule = |task_id: &str, date: NaiveDate, rule: &str| {
        dismissed
            .iter()
            .any(|entry| entry.task_id == task_id && entry.date == date && entry.rule_id == rule)
    };
    let push = |out: &mut Vec<Conflict>,
                task_id: &str,
                date: NaiveDate,
                rule: &str,
                reason: String,
                action: SuggestionAction,
                target: Option<NaiveDate>| {
        if dismissed_rule(task_id, date, rule) {
            return;
        }
        if out.iter().any(|existing: &Conflict| {
            existing.task_id == task_id && existing.date == date && existing.rule_id == rule
        }) {
            return;
        }
        out.push(Conflict {
            task_id: task_id.to_string(),
            date,
            rule_id: rule.to_string(),
            reason,
            suggestion: ConflictSuggestion {
                action,
                target_date: target,
            },
        });
    };
    let mut date = from;
    while date <= to {
        let status = day_status(input, date);
        let planned = planned_tasks(input, date);
        let exams = exams_on(input, date);
        if status.status == DayStatusKind::Blocked {
            for task in &planned {
                let target = next_open_day(input, date + Duration::days(1), task.est_min, |day| {
                    day_status(input, day).status == DayStatusKind::Blocked
                });
                push(
                    &mut out,
                    &task.id,
                    date,
                    "task_on_blocked_day",
                    format!(
                        "Planned on a blocked day ({})",
                        status
                            .reasons
                            .first()
                            .map(|r| r.message.as_str())
                            .unwrap_or("blocked")
                    ),
                    SuggestionAction::Move,
                    Some(target),
                );
            }
        }
        let used: u32 = planned.iter().map(|task| task_minutes(input, task)).sum();
        if used > status.available_min {
            if let Some(task) = planned.iter().min_by_key(|task| task.priority) {
                let target = next_open_day(input, date + Duration::days(1), task.est_min, |day| {
                    day_status(input, day).status == DayStatusKind::Blocked
                });
                push(
                    &mut out,
                    &task.id,
                    date,
                    "task_over_capacity_day",
                    format!(
                        "{used} planned minutes exceed {} available",
                        status.available_min
                    ),
                    SuggestionAction::Move,
                    Some(target),
                );
            }
        }
        for task in input.tasks.iter().filter(|task| {
            task.deleted_at.is_none()
                && task.status.active()
                && task
                    .due_at
                    .is_some_and(|due| due.with_timezone(&input.tz).date_naive() == date)
        }) {
            if !exams.is_empty() {
                push(
                    &mut out,
                    &task.id,
                    date,
                    "due_on_exam_day",
                    format!("Due on the exam day for {}", exams[0].title),
                    SuggestionAction::Defer,
                    Some(date + Duration::days(1)),
                );
            }
            if in_prep_window(input, date)
                || input.events.iter().any(|event| {
                    event.kind.is_exam()
                        && event_in_scope(input, event)
                        && date <= event.start_at.with_timezone(&input.tz).date_naive()
                        && date
                            >= event.start_at.with_timezone(&input.tz).date_naive()
                                - Duration::days(
                                    event.prep_days.max(input.settings.prep_window_days) as i64,
                                )
                })
            {
                if let Some(start_by) = start_by_date(input, task) {
                    if task.planned_day.is_none_or(|planned| planned > start_by) {
                        push(
                            &mut out,
                            &task.id,
                            date,
                            "due_in_exam_window_without_start_by",
                            format!("Start by {start_by} to finish before the exam window"),
                            SuggestionAction::Move,
                            Some(start_by),
                        );
                    }
                }
            }
        }
        if in_prep_window(input, date) && !planned.is_empty() {
            let non_study: Vec<&Task> = planned
                .iter()
                .copied()
                .filter(|task| !task.tags.iter().any(|tag| tag == "!study"))
                .collect();
            let cap = status.available_min * input.settings.non_study_cap_in_prep_pct / 100;
            let sum: u32 = non_study.iter().map(|task| task_minutes(input, task)).sum();
            if sum > cap {
                if let Some(task) = non_study.iter().max_by_key(|task| task.priority) {
                    push(
                        &mut out,
                        &task.id,
                        date,
                        "low_priority_in_prep_window",
                        format!("Non-study work ({sum}m) exceeds {cap}m during exam prep"),
                        SuggestionAction::Split,
                        None,
                    );
                }
            }
        }
        let cluster = input
            .events
            .iter()
            .filter(|event| event_in_scope(input, event))
            .filter(|event| event.kind.is_exam() && event_on(event, date, input.tz))
            .count();
        if cluster > 1 {
            if let Some(task) = planned.first() {
                push(
                    &mut out,
                    &task.id,
                    date,
                    "too_many_exams_in_n_days",
                    format!("{cluster} exams on one day"),
                    SuggestionAction::Defer,
                    None,
                );
            }
        }
        let _ = EXAM_CLUSTER_DAYS;
        date += Duration::days(1);
    }
    out
}

pub fn prep_plan(input: &LoadInput, event: &AcademicEvent, syllabus: &[SyllabusItem]) -> PrepPlan {
    let exam_day = event.start_at.with_timezone(&input.tz).date_naive();
    let prep_days = event.prep_days.max(input.settings.prep_window_days) as i64;
    let first = exam_day - Duration::days(prep_days);
    let mut open_days = Vec::new();
    let mut day = first;
    while day < exam_day {
        let status = day_status(input, day);
        if status.status != DayStatusKind::Blocked {
            open_days.push((day, status.available_min.saturating_sub(status.used_min)));
        }
        day += Duration::days(1);
    }
    let mut items: Vec<(String, Option<String>)> = syllabus
        .iter()
        .filter(|item| {
            !item.done
                && item.deleted_at.is_none()
                && event
                    .subject_id
                    .as_ref()
                    .is_none_or(|subject| &item.subject_id == subject)
        })
        .map(|item| (item.title.clone(), Some(item.id.clone())))
        .collect();
    if items.is_empty() {
        items.push((format!("Revise {}", event.title), None));
    }
    let mut tasks = Vec::new();
    let per_day = if open_days.is_empty() {
        0
    } else {
        items.len().div_ceil(open_days.len())
    };
    let mut index = 0;
    for (day, budget) in open_days {
        let mut remaining = budget;
        for _ in 0..per_day {
            if index >= items.len() {
                break;
            }
            let minutes = remaining.min(PREP_TASK_MIN).clamp(15, PREP_TASK_MIN);
            if minutes == 0 {
                break;
            }
            remaining = remaining.saturating_sub(minutes);
            let (title, syllabus_item_id) = items[index].clone();
            tasks.push(ProposedTask {
                title,
                day,
                est_min: minutes,
                tags: vec!["!study".into()],
                subject_id: event.subject_id.clone(),
                syllabus_item_id,
            });
            index += 1;
        }
        if index >= items.len() {
            break;
        }
    }
    let revision_day = exam_day - Duration::days(1);
    tasks.push(ProposedTask {
        title: format!("Revise {}", event.title),
        day: revision_day,
        est_min: 30,
        tags: vec!["!study".into()],
        subject_id: event.subject_id.clone(),
        syllabus_item_id: None,
    });
    PrepPlan {
        event_id: event.id.clone(),
        event_title: event.title.clone(),
        tasks,
    }
}

pub fn suggest_reschedule(input: &LoadInput, task: &Task, from: NaiveDate) -> Option<NaiveDate> {
    let minutes = task_minutes(input, task);
    let mut day = from;
    for _ in 0..60 {
        let status = day_status(input, day);
        if status.status != DayStatusKind::Blocked
            && status.available_min.saturating_sub(status.used_min) >= minutes
        {
            return Some(day);
        }
        day += Duration::days(1);
    }
    None
}

pub fn exam_for_day(input: &LoadInput, date: NaiveDate) -> Option<AcademicEvent> {
    input
        .events
        .iter()
        .find(|event| event.kind.is_exam() && event_on(event, date, input.tz))
        .cloned()
}

pub fn exam_on(event: &AcademicEvent, date: NaiveDate, tz: Tz) -> bool {
    event.kind.is_exam() && event_on(event, date, tz)
}
