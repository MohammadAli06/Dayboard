use crate::model::*;
use chrono::{
    DateTime, Datelike, Duration, LocalResult, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Utc,
};
use chrono_tz::Tz;

pub fn weekday_bit(date: NaiveDate) -> u32 {
    1 << date.weekday().num_days_from_monday()
}

pub fn parse_hhmm(value: &str) -> Result<(u32, u32), String> {
    let mut parts = value.split(':');
    let hour: u32 = parts
        .next()
        .unwrap_or("")
        .parse()
        .map_err(|_| format!("Invalid time {value}; use HH:MM"))?;
    let minute: u32 = parts
        .next()
        .unwrap_or("")
        .parse()
        .map_err(|_| format!("Invalid time {value}; use HH:MM"))?;
    if parts.next().is_some() || hour > 23 || minute > 59 {
        return Err(format!("Invalid time {value}; use HH:MM"));
    }
    Ok((hour, minute))
}

pub fn validate_times(start: &str, end: &str) -> Result<(), String> {
    let (sh, sm) = parse_hhmm(start)?;
    let (eh, em) = parse_hhmm(end)?;
    if (eh, em) <= (sh, sm) {
        return Err("End time must be after start time".into());
    }
    Ok(())
}

fn local_naive(date: NaiveDate, hhmm: &str) -> Result<NaiveDateTime, String> {
    let (hour, minute) = parse_hhmm(hhmm)?;
    Ok(date.and_time(NaiveTime::from_hms_opt(hour, minute, 0).unwrap()))
}

/// Nonexistent local times (DST spring-forward) shift forward; ambiguous times
/// (fall-back) use the first occurrence.
fn resolve(tz: Tz, local: NaiveDateTime) -> DateTime<Utc> {
    match tz.from_local_datetime(&local) {
        LocalResult::Single(at) => at.with_timezone(&Utc),
        LocalResult::Ambiguous(first, _) => first.with_timezone(&Utc),
        LocalResult::None => {
            for shift in 1..=180 {
                if let Some(at) = tz
                    .from_local_datetime(&(local + Duration::minutes(shift)))
                    .earliest()
                {
                    return at.with_timezone(&Utc);
                }
            }
            local.and_utc()
        }
    }
}

pub fn local_bounds(
    date: NaiveDate,
    start: &str,
    end: &str,
    tz: Tz,
) -> Result<(DateTime<Utc>, DateTime<Utc>), String> {
    let start = resolve(tz, local_naive(date, start)?);
    let end = resolve(tz, local_naive(date, end)?);
    Ok((start, end))
}

fn suppressed(
    date: NaiveDate,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    events: &[AcademicEvent],
    tz: Tz,
    term_id: &str,
) -> bool {
    events
        .iter()
        .filter(|event| event.term_id == term_id)
        .any(|event| {
            let event_day = event.start_at.with_timezone(&tz).date_naive();
            match event.suppress_classes {
                SuppressClasses::WholeDay => event_day == date,
                SuppressClasses::OverlapOnly => {
                    if event_day != date {
                        return false;
                    }
                    start < event.end_at && end > event.start_at
                }
                SuppressClasses::None => false,
            }
        })
}

pub fn occurrences(
    from: NaiveDate,
    to: NaiveDate,
    items: &[ScheduleItem],
    exceptions: &[ScheduleException],
    terms: &[Term],
    events: &[AcademicEvent],
    tz: Tz,
) -> Vec<Occurrence> {
    let mut out = Vec::new();
    if to < from {
        return out;
    }
    let Some(active) = terms.iter().find(|term| term.active && !term.archived) else {
        return out;
    };
    for item in items {
        if item.deleted_at.is_some() || !item.active || item.weekdays == 0 {
            continue;
        }
        if item.term_id != active.id {
            continue;
        }
        let term = active;
        let first = from.max(term.start_date);
        let last = to.min(term.end_date);
        let mut day = first;
        while day <= last {
            if item.weekdays & weekday_bit(day) != 0 {
                let mut start_time = item.start_time.as_str();
                let mut end_time = item.end_time.as_str();
                let mut is_moved = false;
                if let Some(exception) = exceptions
                    .iter()
                    .find(|exception| exception.item_id == item.id && exception.date == day)
                {
                    if exception.action == ExceptionAction::Skip {
                        day += Duration::days(1);
                        continue;
                    }
                    start_time = exception.new_start.as_deref().unwrap_or(start_time);
                    end_time = exception.new_end.as_deref().unwrap_or(end_time);
                    is_moved = true;
                }
                if let Ok((start, end)) = local_bounds(day, start_time, end_time, tz) {
                    if !suppressed(day, start, end, events, tz, &active.id) {
                        out.push(Occurrence {
                            item_id: item.id.clone(),
                            title: item.title.clone(),
                            kind: item.kind,
                            date: day,
                            start_utc: start,
                            end_utc: end,
                            location: item.location.clone(),
                            is_moved,
                            remind_before_min: item.remind_before_min,
                        });
                    }
                }
            }
            day += Duration::days(1);
        }
    }
    out.sort_by(|a, b| {
        a.start_utc
            .cmp(&b.start_utc)
            .then(a.item_id.cmp(&b.item_id))
    });
    out
}

pub fn overlapping(
    occurrences: &[Occurrence],
    start: DateTime<Utc>,
    end: DateTime<Utc>,
) -> Vec<Occurrence> {
    occurrences
        .iter()
        .filter(|occurrence| start < occurrence.end_utc && end > occurrence.start_utc)
        .cloned()
        .collect()
}

pub fn minutes(occurrences: &[Occurrence]) -> u32 {
    occurrences
        .iter()
        .map(|occurrence| {
            (occurrence.end_utc - occurrence.start_utc)
                .num_minutes()
                .max(0) as u32
        })
        .sum()
}

pub fn validate_item(
    title: &str,
    weekdays: u32,
    start: &str,
    end: &str,
    remind: Option<u32>,
) -> Result<(), String> {
    if title.trim().is_empty() || title.len() > 200 {
        return Err("Enter a schedule title between 1 and 200 bytes".into());
    }
    if weekdays == 0 || weekdays > 127 {
        return Err("Choose at least one weekday".into());
    }
    validate_times(start, end)?;
    if remind.is_some_and(|value| value > 7 * 24 * 60) {
        return Err("Reminder lead time must be under 7 days".into());
    }
    Ok(())
}

pub fn next_non_blocked(from: NaiveDate, blocked: impl Fn(NaiveDate) -> bool) -> NaiveDate {
    let mut day = from;
    for _ in 0..366 {
        if !blocked(day) {
            return day;
        }
        day += Duration::days(1);
    }
    day
}
