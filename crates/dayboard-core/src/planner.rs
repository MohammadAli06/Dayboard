use crate::model::*;
use chrono::NaiveDate;
pub trait Planner {
    fn suggest(
        &self,
        tasks: &[Task],
        day: NaiveDate,
        settings: &Settings,
        energy: Option<Energy>,
        budget_min: u32,
    ) -> Vec<Task>;
}
pub struct RulePlanner;
impl Planner for RulePlanner {
    fn suggest(
        &self,
        tasks: &[Task],
        day: NaiveDate,
        settings: &Settings,
        energy: Option<Energy>,
        budget_min: u32,
    ) -> Vec<Task> {
        let mut candidates: Vec<_> = tasks
            .iter()
            .filter(|t| {
                t.deleted_at.is_none()
                    && t.status.active()
                    && t.planned_day.is_none_or(|d| d <= day)
                    && energy.is_none_or(|e| t.energy == e)
            })
            .cloned()
            .collect();
        candidates.sort_by_key(|t| (t.due_at.is_none(), t.due_at, t.priority, t.created_at));
        let mut minutes = 0;
        let mut result = vec![];
        for t in candidates {
            if result.len() < settings.max_tasks as usize
                && minutes + t.est_min <= budget_min
                && minutes + t.est_min <= settings.max_minutes
            {
                minutes += t.est_min;
                result.push(t);
            }
        }
        result
    }
}
pub fn capacity(
    tasks: &[Task],
    day: NaiveDate,
    settings: &Settings,
    cap_minutes: u32,
    schedule_minutes: u32,
) -> Capacity {
    let selected: Vec<_> = tasks
        .iter()
        .filter(|t| t.deleted_at.is_none() && t.status.active() && t.planned_day == Some(day))
        .collect();
    let minutes: u32 = selected.iter().map(|t| t.est_min).sum();
    Capacity {
        count: selected.len(),
        minutes,
        over_count: selected.len() > settings.max_tasks as usize,
        over_minutes: minutes > cap_minutes,
        available_minutes: cap_minutes.saturating_sub(minutes),
        schedule_minutes,
        cap_minutes,
    }
}
