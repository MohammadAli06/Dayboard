use crate::{model::*, Error, Result};
use chrono::{DateTime, Datelike, Duration, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Utc};
use chrono_tz::Tz;
use regex::Regex;
use std::sync::LazyLock;
static ESTIMATE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(?:(\d+)h)?(?:(\d+)m)?$").unwrap());
static TIME: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(\d{1,2})(?::(\d{2}))?(am|pm)$").unwrap());

pub fn local_datetime(value: &str, tz: Tz) -> Result<DateTime<Utc>> {
    let dt = NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M")
        .map_err(|_| Error::Validation("Use a valid local date and time".into()))?;
    tz.from_local_datetime(&dt).single().map(|v| v.with_timezone(&Utc)).ok_or_else(|| Error::Validation("This local time is ambiguous or does not exist because of daylight saving. Choose another time.".into()))
}
pub fn parse(text: &str, now: DateTime<Utc>, tz: Tz) -> Result<ParsedTask> {
    let today = now.with_timezone(&tz).date_naive();
    let mut parsed = ParsedTask {
        title: String::new(),
        priority: 3,
        energy: Energy::Low,
        est_min: 25,
        tags: vec![],
        due_at: None,
        planned_day: None,
    };
    let mut words = Vec::new();
    let mut time = None;
    let tokens: Vec<_> = text.split_whitespace().collect();
    let mut i = 0;
    while i < tokens.len() {
        let token = tokens[i];
        let lower = token.to_lowercase();
        i += 1;
        if token.len() > 1 && matches!(token.chars().next(), Some('#' | '@' | '!')) {
            if lower == "@deep" || lower == "@high" {
                parsed.energy = Energy::High;
            }
            if !parsed.tags.contains(&lower) {
                parsed.tags.push(lower);
            }
            continue;
        }
        if lower.len() == 2 && lower.starts_with('p') && lower.as_bytes()[1].is_ascii_digit() {
            let p = lower.as_bytes()[1] - b'0';
            if !(1..=4).contains(&p) {
                return Err(Error::Validation("Priority must be p1 through p4".into()));
            }
            parsed.priority = p;
            continue;
        }
        if let Some(c) = ESTIMATE.captures(&lower) {
            if c.get(1).is_some() || c.get(2).is_some() {
                let hours: u32 = c
                    .get(1)
                    .map_or(Ok(0), |v| v.as_str().parse())
                    .map_err(|_| Error::Validation("Estimate is too large".into()))?;
                let mins: u32 = c
                    .get(2)
                    .map_or(Ok(0), |v| v.as_str().parse())
                    .map_err(|_| Error::Validation("Estimate is too large".into()))?;
                parsed.est_min = hours
                    .checked_mul(60)
                    .and_then(|h| h.checked_add(mins))
                    .filter(|m| (1..=1440).contains(m))
                    .ok_or_else(|| {
                        Error::Validation("Estimate must be between 1 and 1440 minutes".into())
                    })?;
                continue;
            }
        }
        let day = match lower.as_str() {
            "today" => Some(today),
            "tomorrow" => Some(today + Duration::days(1)),
            "next"
                if tokens
                    .get(i)
                    .is_some_and(|s| s.eq_ignore_ascii_case("week")) =>
            {
                i += 1;
                Some(today + Duration::days(7))
            }
            "mon" | "tue" | "wed" | "thu" | "fri" | "sat" | "sun" => {
                let target = ["mon", "tue", "wed", "thu", "fri", "sat", "sun"]
                    .iter()
                    .position(|s| *s == lower)
                    .unwrap() as i64;
                Some(
                    today
                        + Duration::days(
                            (target - today.weekday().num_days_from_monday() as i64).rem_euclid(7),
                        ),
                )
            }
            _ => None,
        };
        if let Some(day) = day {
            parsed.planned_day = Some(day);
            continue;
        }
        if lower.len() == 10 && lower.as_bytes()[4] == b'-' && lower.as_bytes()[7] == b'-' {
            parsed.planned_day = Some(
                NaiveDate::parse_from_str(&lower, "%Y-%m-%d")
                    .map_err(|_| Error::Validation("Invalid date; use YYYY-MM-DD".into()))?,
            );
            continue;
        }
        if let Some(c) = TIME.captures(&lower) {
            let hour: u32 = c[1].parse().unwrap();
            let minute: u32 = c.get(2).map_or(0, |m| m.as_str().parse().unwrap());
            if !(1..=12).contains(&hour) || minute > 59 {
                return Err(Error::Validation("Invalid 12-hour time".into()));
            }
            time =
                NaiveTime::from_hms_opt(hour % 12 + if &c[3] == "pm" { 12 } else { 0 }, minute, 0);
            continue;
        }
        if lower.len() == 5 && lower.as_bytes()[2] == b':' {
            time = Some(
                NaiveTime::parse_from_str(&lower, "%H:%M")
                    .map_err(|_| Error::Validation("Invalid time; use HH:MM".into()))?,
            );
            continue;
        }
        words.push(token);
    }
    parsed.title = words.join(" ");
    if parsed.title.is_empty() || parsed.title.len() > 500 {
        return Err(Error::Validation(
            "Enter a title between 1 and 500 bytes".into(),
        ));
    }
    if parsed.planned_day.is_some() || time.is_some() {
        let day = parsed.planned_day.unwrap_or(today);
        parsed.planned_day = Some(day);
        let naive = day.and_time(time.unwrap_or(NaiveTime::from_hms_opt(17, 0, 0).unwrap()));
        parsed.due_at = Some(
            tz.from_local_datetime(&naive)
                .single()
                .ok_or_else(|| {
                    Error::Validation(
                        "Ambiguous or nonexistent local time; choose another time".into(),
                    )
                })?
                .with_timezone(&Utc),
        );
    }
    Ok(parsed)
}
