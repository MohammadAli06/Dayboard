use crate::load::{self, LoadInput};
use crate::model::*;
use crate::schedule;
use crate::{Core, Error, Result};
use chrono::{DateTime, Duration, NaiveDate, Utc};
use chrono_tz::Tz;
use rusqlite::{params, OptionalExtension};

pub(crate) fn kind_str(kind: ScheduleKind) -> &'static str {
    match kind {
        ScheduleKind::Class => "class",
        ScheduleKind::Work => "work",
        ScheduleKind::Routine => "routine",
        ScheduleKind::Other => "other",
    }
}
fn kind_from(value: &str) -> ScheduleKind {
    match value {
        "work" => ScheduleKind::Work,
        "routine" => ScheduleKind::Routine,
        "other" => ScheduleKind::Other,
        _ => ScheduleKind::Class,
    }
}
pub(crate) fn action_str(action: ExceptionAction) -> &'static str {
    match action {
        ExceptionAction::Skip => "skip",
        ExceptionAction::Move => "move",
    }
}
fn action_from(value: &str) -> ExceptionAction {
    if value == "move" {
        ExceptionAction::Move
    } else {
        ExceptionAction::Skip
    }
}
pub(crate) fn academic_str(kind: AcademicKind) -> &'static str {
    match kind {
        AcademicKind::Midterm => "midterm",
        AcademicKind::Practical => "practical",
        AcademicKind::Oral => "oral",
        AcademicKind::Final => "final",
        AcademicKind::AssignmentDue => "assignment_due",
        AcademicKind::Holiday => "holiday",
        AcademicKind::Other => "other",
    }
}
fn academic_from(value: &str) -> AcademicKind {
    match value {
        "midterm" => AcademicKind::Midterm,
        "practical" => AcademicKind::Practical,
        "oral" => AcademicKind::Oral,
        "final" => AcademicKind::Final,
        "assignment_due" => AcademicKind::AssignmentDue,
        "holiday" => AcademicKind::Holiday,
        _ => AcademicKind::Other,
    }
}
pub(crate) fn suppress_str(value: SuppressClasses) -> &'static str {
    match value {
        SuppressClasses::None => "none",
        SuppressClasses::OverlapOnly => "overlap_only",
        SuppressClasses::WholeDay => "whole_day",
    }
}
fn suppress_from(value: &str) -> SuppressClasses {
    match value {
        "none" => SuppressClasses::None,
        "whole_day" => SuppressClasses::WholeDay,
        _ => SuppressClasses::OverlapOnly,
    }
}
fn dt(value: String) -> Result<DateTime<Utc>> {
    Ok(DateTime::parse_from_rfc3339(&value)
        .map_err(|error| Error::Validation(format!("Invalid stored timestamp: {error}")))?
        .with_timezone(&Utc))
}
fn day(value: String) -> Result<NaiveDate> {
    NaiveDate::parse_from_str(&value, "%Y-%m-%d")
        .map_err(|error| Error::Validation(format!("Invalid stored date: {error}")))
}

pub(crate) struct CalendarData {
    pub items: Vec<ScheduleItem>,
    pub exceptions: Vec<ScheduleException>,
    pub terms: Vec<Term>,
    pub events: Vec<AcademicEvent>,
}

impl Core {
    pub fn schedule_items(&self) -> Result<Vec<ScheduleItem>> {
        let mut stmt = self.conn.prepare(
            "SELECT id,title,kind,weekdays,start_time,end_time,location,notes,subject_id,remind_before_min,active,term_id,created_at,updated_at,deleted FROM schedule_items ORDER BY start_time,title",
        )?;
        let mut rows = stmt.query([])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            let updated_at = dt(row.get(13)?)?;
            let deleted = row.get::<_, i64>(14)? != 0;
            out.push(ScheduleItem {
                id: row.get(0)?,
                title: row.get(1)?,
                kind: kind_from(&row.get::<_, String>(2)?),
                weekdays: row.get::<_, i64>(3)? as u32,
                start_time: row.get(4)?,
                end_time: row.get(5)?,
                location: row.get(6)?,
                notes: row.get(7)?,
                subject_id: row.get(8)?,
                remind_before_min: row.get::<_, Option<i64>>(9)?.map(|v| v as u32),
                active: row.get::<_, i64>(10)? != 0,
                term_id: row.get::<_, Option<String>>(11)?.unwrap_or_default(),
                created_at: dt(row.get(12)?)?,
                updated_at,
                deleted_at: if deleted { Some(updated_at) } else { None },
            });
        }
        Ok(out)
    }
    pub fn schedule_exceptions(&self) -> Result<Vec<ScheduleException>> {
        let mut stmt = self.conn.prepare(
            "SELECT id,item_id,date,action,new_start,new_end,note FROM schedule_exceptions ORDER BY date",
        )?;
        let mut rows = stmt.query([])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            out.push(ScheduleException {
                id: row.get(0)?,
                item_id: row.get(1)?,
                date: day(row.get(2)?)?,
                action: action_from(&row.get::<_, String>(3)?),
                new_start: row.get(4)?,
                new_end: row.get(5)?,
                note: row.get(6)?,
            });
        }
        Ok(out)
    }
    pub fn terms(&self) -> Result<Vec<Term>> {
        let mut stmt = self.conn.prepare(
            "SELECT id,name,start_date,end_date,active,archived,created_at FROM terms ORDER BY start_date",
        )?;
        let mut rows = stmt.query([])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            out.push(Term {
                id: row.get(0)?,
                name: row.get(1)?,
                start_date: day(row.get(2)?)?,
                end_date: day(row.get(3)?)?,
                active: row.get::<_, i64>(4)? != 0,
                archived: row.get::<_, i64>(5)? != 0,
                created_at: dt(row.get(6)?)?,
            });
        }
        Ok(out)
    }
    pub fn active_term(&self) -> Result<Option<Term>> {
        let terms = self.terms()?;
        Ok(terms
            .iter()
            .find(|term| term.active && !term.archived)
            .or_else(|| terms.iter().find(|term| !term.archived))
            .cloned())
    }
    pub fn academic_events(&self) -> Result<Vec<AcademicEvent>> {
        let mut stmt = self.conn.prepare(
            "SELECT id,term_id,kind,title,subject_id,start_at,end_at,all_day,prep_days,suppress_classes,notes FROM academic_events ORDER BY start_at",
        )?;
        let mut rows = stmt.query([])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            out.push(AcademicEvent {
                id: row.get(0)?,
                term_id: row.get(1)?,
                kind: academic_from(&row.get::<_, String>(2)?),
                title: row.get(3)?,
                subject_id: row.get(4)?,
                start_at: dt(row.get(5)?)?,
                end_at: dt(row.get(6)?)?,
                all_day: row.get::<_, i64>(7)? != 0,
                prep_days: row.get::<_, i64>(8)? as u32,
                suppress_classes: suppress_from(&row.get::<_, String>(9)?),
                notes: row.get(10)?,
            });
        }
        Ok(out)
    }
    pub fn dismissed_conflicts(&self) -> Result<Vec<DismissedConflict>> {
        let mut stmt = self
            .conn
            .prepare("SELECT task_id,date,rule_id,created_at FROM dismissed_conflicts")?;
        let mut rows = stmt.query([])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            out.push(DismissedConflict {
                task_id: row.get(0)?,
                date: day(row.get(1)?)?,
                rule_id: row.get(2)?,
                created_at: dt(row.get(3)?)?,
            });
        }
        Ok(out)
    }
    pub(crate) fn calendar_data(&self) -> Result<CalendarData> {
        Ok(CalendarData {
            items: self.schedule_items()?,
            exceptions: self.schedule_exceptions()?,
            terms: self.terms()?,
            events: self.academic_events()?,
        })
    }
    fn term_for_new_item(&self, term_id: Option<String>) -> Result<Term> {
        match term_id {
            Some(id) => {
                let term = self
                    .terms()?
                    .into_iter()
                    .find(|term| term.id == id)
                    .ok_or_else(|| Error::Validation("Term not found".into()))?;
                if term.archived {
                    return Err(Error::Validation(
                        "Archived terms are read-only; choose another term".into(),
                    ));
                }
                Ok(term)
            }
            None => self
                .active_term()?
                .filter(|term| !term.archived)
                .ok_or_else(|| Error::Validation("Create a term first".into())),
        }
    }
    fn save_schedule_item(
        &mut self,
        before: Option<&ScheduleItem>,
        item: &ScheduleItem,
        actor: Actor,
        action: &str,
        now: DateTime<Utc>,
    ) -> Result<()> {
        let tx = self.conn.transaction()?;
        tx.execute(
            "INSERT INTO schedule_items(id,title,kind,weekdays,start_time,end_time,start_date,end_date,location,notes,subject_id,remind_before_min,active,term_id,created_at,updated_at,deleted) VALUES(?1,?2,?3,?4,?5,?6,NULL,NULL,?7,?8,?9,?10,?11,?12,?13,?14,?15) ON CONFLICT(id) DO UPDATE SET title=excluded.title,kind=excluded.kind,weekdays=excluded.weekdays,start_time=excluded.start_time,end_time=excluded.end_time,location=excluded.location,notes=excluded.notes,subject_id=excluded.subject_id,remind_before_min=excluded.remind_before_min,active=excluded.active,term_id=excluded.term_id,updated_at=excluded.updated_at,deleted=excluded.deleted",
            params![
                item.id,
                item.title,
                kind_str(item.kind),
                item.weekdays as i64,
                item.start_time,
                item.end_time,
                item.location,
                item.notes,
                item.subject_id,
                item.remind_before_min.map(|v| v as i64),
                item.active as i64,
                item.term_id,
                item.created_at.to_rfc3339(),
                item.updated_at.to_rfc3339(),
                item.deleted_at.is_some() as i64
            ],
        )?;
        Self::event(
            &tx,
            actor,
            "schedule_item",
            action,
            &item.id,
            before,
            Some(item),
            now,
        )?;
        tx.commit()?;
        Ok(())
    }
    fn save_exception(
        &mut self,
        before: Option<&ScheduleException>,
        after: Option<&ScheduleException>,
        actor: Actor,
        action: &str,
        now: DateTime<Utc>,
    ) -> Result<()> {
        let tx = self.conn.transaction()?;
        if let Some(exception) = after {
            tx.execute(
                "INSERT INTO schedule_exceptions(id,item_id,date,action,new_start,new_end,note) VALUES(?1,?2,?3,?4,?5,?6,?7) ON CONFLICT(item_id,date) DO UPDATE SET action=excluded.action,new_start=excluded.new_start,new_end=excluded.new_end,note=excluded.note",
                params![
                    exception.id,
                    exception.item_id,
                    exception.date.to_string(),
                    action_str(exception.action),
                    exception.new_start,
                    exception.new_end,
                    exception.note
                ],
            )?;
        } else if let Some(exception) = before {
            tx.execute(
                "DELETE FROM schedule_exceptions WHERE item_id=?1 AND date=?2",
                params![exception.item_id, exception.date.to_string()],
            )?;
        }
        let key = before
            .or(after)
            .map(|exception| format!("{}:{}", exception.item_id, exception.date))
            .unwrap_or_default();
        Self::event(
            &tx,
            actor,
            "schedule_exception",
            action,
            &key,
            before,
            after,
            now,
        )?;
        tx.commit()?;
        Ok(())
    }
    fn save_term(
        &mut self,
        before: Option<&Term>,
        term: &Term,
        actor: Actor,
        action: &str,
        now: DateTime<Utc>,
    ) -> Result<()> {
        let tx = self.conn.transaction()?;
        tx.execute(
            "INSERT INTO terms(id,name,start_date,end_date,active,archived,created_at) VALUES(?1,?2,?3,?4,?5,?6,?7) ON CONFLICT(id) DO UPDATE SET name=excluded.name,start_date=excluded.start_date,end_date=excluded.end_date,active=excluded.active,archived=excluded.archived",
            params![
                term.id,
                term.name,
                term.start_date.to_string(),
                term.end_date.to_string(),
                term.active as i64,
                term.archived as i64,
                term.created_at.to_rfc3339()
            ],
        )?;
        Self::event(
            &tx,
            actor,
            "term",
            action,
            &term.id,
            before,
            Some(term),
            now,
        )?;
        tx.commit()?;
        Ok(())
    }
    fn save_academic_event(
        &mut self,
        before: Option<&AcademicEvent>,
        event: Option<&AcademicEvent>,
        actor: Actor,
        action: &str,
        now: DateTime<Utc>,
    ) -> Result<()> {
        let tx = self.conn.transaction()?;
        if let Some(event) = event {
            tx.execute(
                "INSERT INTO academic_events(id,term_id,kind,title,subject_id,start_at,end_at,all_day,prep_days,suppress_classes,notes) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11) ON CONFLICT(id) DO UPDATE SET term_id=excluded.term_id,kind=excluded.kind,title=excluded.title,subject_id=excluded.subject_id,start_at=excluded.start_at,end_at=excluded.end_at,all_day=excluded.all_day,prep_days=excluded.prep_days,suppress_classes=excluded.suppress_classes,notes=excluded.notes",
                params![
                    event.id,
                    event.term_id,
                    academic_str(event.kind),
                    event.title,
                    event.subject_id,
                    event.start_at.to_rfc3339(),
                    event.end_at.to_rfc3339(),
                    event.all_day as i64,
                    event.prep_days as i64,
                    suppress_str(event.suppress_classes),
                    event.notes
                ],
            )?;
        } else if let Some(event) = before {
            tx.execute("DELETE FROM academic_events WHERE id=?1", [&event.id])?;
        }
        let id = before
            .or(event)
            .map(|event| event.id.as_str())
            .unwrap_or("");
        Self::event(&tx, actor, "academic_event", action, id, before, event, now)?;
        tx.commit()?;
        Ok(())
    }
    fn event_payload(
        &mut self,
        actor: Actor,
        entity: &str,
        action: &str,
        id: &str,
        payload: serde_json::Value,
        now: DateTime<Utc>,
    ) -> Result<()> {
        let tx = self.conn.transaction()?;
        tx.execute(
            "INSERT INTO events(id,ts,actor,entity,entity_id,action,payload) VALUES(?1,?2,?3,?4,?5,?6,?7)",
            params![
                ulid::Ulid::new().to_string(),
                now.to_rfc3339(),
                serde_json::to_value(actor)?.as_str().unwrap(),
                entity,
                id,
                action,
                serde_json::to_string(&payload)?
            ],
        )?;
        tx.commit()?;
        Ok(())
    }
    fn find_item(&self, id: &str) -> Result<ScheduleItem> {
        self.schedule_items()?
            .into_iter()
            .find(|item| item.id == id)
            .ok_or_else(|| Error::Validation("Schedule item not found".into()))
    }
    fn event_by_id(&self, id: &str) -> Result<AcademicEvent> {
        self.academic_events()?
            .into_iter()
            .find(|event| event.id == id)
            .ok_or_else(|| Error::Validation("Academic event not found".into()))
    }
    fn find_exception(&self, item_id: &str, date: NaiveDate) -> Result<Option<ScheduleException>> {
        Ok(self
            .schedule_exceptions()?
            .into_iter()
            .find(|exception| exception.item_id == item_id && exception.date == date))
    }
    fn load_input<'a>(
        settings: &'a Settings,
        tz: Tz,
        tasks: &'a [Task],
        data: &'a CalendarData,
    ) -> LoadInput<'a> {
        LoadInput {
            settings,
            items: &data.items,
            exceptions: &data.exceptions,
            terms: &data.terms,
            events: &data.events,
            tasks,
            tz,
        }
    }
    pub(crate) fn overlaps(
        &self,
        start: Option<DateTime<Utc>>,
        end: Option<DateTime<Utc>>,
        tz: Tz,
    ) -> Result<Vec<Occurrence>> {
        let (Some(start), Some(end)) = (start, end) else {
            return Ok(Vec::new());
        };
        let data = self.calendar_data()?;
        let occurrences = schedule::occurrences(
            start.with_timezone(&tz).date_naive(),
            end.with_timezone(&tz).date_naive(),
            &data.items,
            &data.exceptions,
            &data.terms,
            &data.events,
            tz,
        );
        Ok(schedule::overlapping(&occurrences, start, end))
    }
    pub(crate) fn execute_calendar(
        &mut self,
        command: Command,
        actor: Actor,
        now: DateTime<Utc>,
        settings: &Settings,
        tz: Tz,
        _today: NaiveDate,
    ) -> Result<Response> {
        let data = self.calendar_data()?;
        match command {
            Command::CreateScheduleItem {
                title,
                kind,
                weekdays,
                start_time,
                end_time,
                term_id,
                location,
                notes,
                subject_id,
                remind_before_min,
            } => {
                schedule::validate_item(
                    &title,
                    weekdays,
                    &start_time,
                    &end_time,
                    remind_before_min,
                )
                .map_err(Error::Validation)?;
                let term = self.term_for_new_item(term_id)?;
                if let Some(id) = &subject_id {
                    ensure_subject(self, id)?;
                }
                let item = ScheduleItem {
                    id: ulid::Ulid::new().to_string(),
                    title: title.trim().into(),
                    kind,
                    weekdays,
                    start_time,
                    end_time,
                    location: location
                        .map(|v| v.trim().to_string())
                        .filter(|v| !v.is_empty()),
                    notes: notes
                        .map(|v| v.trim().to_string())
                        .filter(|v| !v.is_empty()),
                    subject_id,
                    remind_before_min,
                    active: true,
                    term_id: term.id,
                    created_at: now,
                    updated_at: now,
                    deleted_at: None,
                };
                self.save_schedule_item(None, &item, actor, "create_schedule_item", now)?;
                Ok(Response::ScheduleItem(item))
            }
            Command::UpdateScheduleItem {
                id,
                title,
                kind,
                weekdays,
                start_time,
                end_time,
                location,
                notes,
                subject_id,
                remind_before_min,
                active,
            } => {
                let before = self.find_item(&id)?;
                ensure_term_writable(&data.terms, &before.term_id)?;
                if before.deleted_at.is_some() {
                    return Err(Error::Validation(
                        "Restore this schedule item before editing it".into(),
                    ));
                }
                schedule::validate_item(
                    &title,
                    weekdays,
                    &start_time,
                    &end_time,
                    remind_before_min,
                )
                .map_err(Error::Validation)?;
                if let Some(subject) = &subject_id {
                    ensure_subject(self, subject)?;
                }
                let mut item = before.clone();
                item.title = title.trim().into();
                item.kind = kind;
                item.weekdays = weekdays;
                item.start_time = start_time;
                item.end_time = end_time;
                item.location = location
                    .map(|v| v.trim().to_string())
                    .filter(|v| !v.is_empty());
                item.notes = notes
                    .map(|v| v.trim().to_string())
                    .filter(|v| !v.is_empty());
                item.subject_id = subject_id;
                item.remind_before_min = remind_before_min;
                item.active = active;
                item.updated_at = now;
                self.save_schedule_item(Some(&before), &item, actor, "update_schedule_item", now)?;
                Ok(Response::ScheduleItem(item))
            }
            Command::DeleteScheduleItem { id } => {
                let before = self.find_item(&id)?;
                ensure_term_writable(&data.terms, &before.term_id)?;
                if before.deleted_at.is_some() {
                    return Err(Error::Validation(
                        "This schedule item is already deleted".into(),
                    ));
                }
                let mut item = before.clone();
                item.deleted_at = Some(now);
                item.active = false;
                item.updated_at = now;
                self.save_schedule_item(Some(&before), &item, actor, "delete_schedule_item", now)?;
                Ok(Response::ScheduleItem(item))
            }
            Command::ListScheduleItems => Ok(Response::ScheduleItems(
                self.schedule_items()?
                    .into_iter()
                    .filter(|item| item.deleted_at.is_none())
                    .collect(),
            )),
            Command::ListOccurrences { from, to } => {
                check_range(from, to, 400)?;
                Ok(Response::Occurrences(schedule::occurrences(
                    from,
                    to,
                    &data.items,
                    &data.exceptions,
                    &data.terms,
                    &data.events,
                    tz,
                )))
            }
            Command::SkipOccurrence { item_id, date } => {
                let item = self.find_item(&item_id)?;
                ensure_term_writable(&data.terms, &item.term_id)?;
                if item.deleted_at.is_some() {
                    return Err(Error::Validation("This schedule item is deleted".into()));
                }
                validate_occurrence_date(&item, &data.terms, date)?;
                let before = self.find_exception(&item_id, date)?;
                let exception = ScheduleException {
                    id: before
                        .as_ref()
                        .map(|existing| existing.id.clone())
                        .unwrap_or_else(|| ulid::Ulid::new().to_string()),
                    item_id,
                    date,
                    action: ExceptionAction::Skip,
                    new_start: None,
                    new_end: None,
                    note: None,
                };
                self.save_exception(
                    before.as_ref(),
                    Some(&exception),
                    actor,
                    "skip_occurrence",
                    now,
                )?;
                Ok(Response::Occurrences(Vec::new()))
            }
            Command::MoveOccurrence {
                item_id,
                date,
                new_start,
                new_end,
            } => {
                let item = self.find_item(&item_id)?;
                ensure_term_writable(&data.terms, &item.term_id)?;
                if item.deleted_at.is_some() {
                    return Err(Error::Validation("This schedule item is deleted".into()));
                }
                validate_occurrence_date(&item, &data.terms, date)?;
                let end = match new_end {
                    Some(end) => end,
                    None => {
                        let (sh, sm) =
                            schedule::parse_hhmm(&item.start_time).map_err(Error::Validation)?;
                        let (eh, em) =
                            schedule::parse_hhmm(&item.end_time).map_err(Error::Validation)?;
                        let duration = (eh * 60 + em) as i64 - (sh * 60 + sm) as i64;
                        let (nh, nm) =
                            schedule::parse_hhmm(&new_start).map_err(Error::Validation)?;
                        let total = (nh * 60 + nm) as i64 + duration.max(1);
                        if total >= 24 * 60 {
                            return Err(Error::Validation(
                                "The moved block would end after midnight".into(),
                            ));
                        }
                        format!("{:02}:{:02}", total / 60, total % 60)
                    }
                };
                schedule::validate_times(&new_start, &end).map_err(Error::Validation)?;
                let before = self.find_exception(&item_id, date)?;
                let exception = ScheduleException {
                    id: before
                        .as_ref()
                        .map(|existing| existing.id.clone())
                        .unwrap_or_else(|| ulid::Ulid::new().to_string()),
                    item_id,
                    date,
                    action: ExceptionAction::Move,
                    new_start: Some(new_start),
                    new_end: Some(end),
                    note: None,
                };
                self.save_exception(
                    before.as_ref(),
                    Some(&exception),
                    actor,
                    "move_occurrence",
                    now,
                )?;
                Ok(Response::Occurrences(Vec::new()))
            }
            Command::ClearException { item_id, date } => {
                let item = self.find_item(&item_id)?;
                ensure_term_writable(&data.terms, &item.term_id)?;
                let before = self
                    .find_exception(&item_id, date)?
                    .ok_or_else(|| Error::Validation("No exception on that date".into()))?;
                self.save_exception(Some(&before), None, actor, "clear_exception", now)?;
                Ok(Response::Occurrences(Vec::new()))
            }
            Command::CreateTerm {
                name,
                start_date,
                end_date,
            } => {
                let name = validate_term(&name, start_date, end_date)?;
                let any_active = data.terms.iter().any(|term| term.active && !term.archived);
                let term = Term {
                    id: ulid::Ulid::new().to_string(),
                    name,
                    start_date,
                    end_date,
                    active: !any_active,
                    archived: false,
                    created_at: now,
                };
                self.save_term(None, &term, actor, "create_term", now)?;
                Ok(Response::Term(term))
            }
            Command::UpdateTerm {
                id,
                name,
                start_date,
                end_date,
            } => {
                let before = data
                    .terms
                    .iter()
                    .find(|term| term.id == id)
                    .cloned()
                    .ok_or_else(|| Error::Validation("Term not found".into()))?;
                if before.archived {
                    return Err(Error::Validation("Archived terms are read-only".into()));
                }
                let name = validate_term(&name, start_date, end_date)?;
                let mut term = before.clone();
                term.name = name;
                term.start_date = start_date;
                term.end_date = end_date;
                self.save_term(Some(&before), &term, actor, "update_term", now)?;
                Ok(Response::Term(term))
            }
            Command::ArchiveTerm { id } => {
                let before = data
                    .terms
                    .iter()
                    .find(|term| term.id == id)
                    .cloned()
                    .ok_or_else(|| Error::Validation("Term not found".into()))?;
                let mut term = before.clone();
                term.archived = true;
                term.active = false;
                self.save_term(Some(&before), &term, actor, "archive_term", now)?;
                if before.active {
                    if let Some(next) = data
                        .terms
                        .iter()
                        .find(|candidate| candidate.id != id && !candidate.archived)
                        .cloned()
                    {
                        let mut next = next;
                        next.active = true;
                        self.save_term(None, &next, actor, "set_active_term", now)?;
                    }
                }
                Ok(Response::Term(term))
            }
            Command::SetActiveTerm { id } => {
                let before = data
                    .terms
                    .iter()
                    .find(|term| term.id == id)
                    .cloned()
                    .ok_or_else(|| Error::Validation("Term not found".into()))?;
                if before.archived {
                    return Err(Error::Validation(
                        "Archived terms are read-only; choose another term".into(),
                    ));
                }
                self.conn.execute("UPDATE terms SET active=0", [])?;
                let mut term = before.clone();
                term.active = true;
                let previous = data
                    .terms
                    .iter()
                    .find(|candidate| candidate.active && candidate.id != id)
                    .map(|candidate| candidate.id.clone());
                self.save_term(Some(&before), &term, actor, "set_active_term", now)?;
                self.event_payload(
                    actor,
                    "term",
                    "set_active_term_meta",
                    &term.id,
                    serde_json::json!({ "previous_active": previous }),
                    now,
                )?;
                Ok(Response::Term(term))
            }
            Command::CopyScheduleToTerm {
                from_term_id,
                to_term_id,
            } => {
                if from_term_id == to_term_id {
                    return Err(Error::Validation(
                        "Choose a different destination term".into(),
                    ));
                }
                let target = data
                    .terms
                    .iter()
                    .find(|term| term.id == to_term_id)
                    .cloned()
                    .ok_or_else(|| Error::Validation("Destination term not found".into()))?;
                if target.archived {
                    return Err(Error::Validation(
                        "Archived terms are read-only; choose another destination".into(),
                    ));
                }
                let mut copied = Vec::new();
                for item in data
                    .items
                    .iter()
                    .filter(|item| item.term_id == from_term_id && item.deleted_at.is_none())
                {
                    let mut copy = item.clone();
                    copy.id = ulid::Ulid::new().to_string();
                    copy.term_id = to_term_id.clone();
                    copy.created_at = now;
                    copy.updated_at = now;
                    copy.deleted_at = None;
                    self.save_schedule_item(None, &copy, actor, "copy_schedule_item", now)?;
                    copied.push(copy);
                }
                Ok(Response::ScheduleItems(copied))
            }
            Command::CreateAcademicEvent {
                term_id,
                kind,
                title,
                subject_id,
                start_at,
                end_at,
                all_day,
                prep_days,
                suppress_classes,
                notes,
            } => {
                let term = match term_id {
                    Some(id) => data
                        .terms
                        .iter()
                        .find(|term| term.id == id)
                        .cloned()
                        .ok_or_else(|| Error::Validation("Term not found".into()))?,
                    None => self.term_for_new_item(None)?,
                };
                if term.archived {
                    return Err(Error::Validation("Archived terms are read-only".into()));
                }
                let event = AcademicEvent {
                    id: ulid::Ulid::new().to_string(),
                    term_id: term.id,
                    kind,
                    title: event_title(&title)?,
                    subject_id,
                    start_at,
                    end_at: event_end(start_at, end_at, all_day)?,
                    all_day,
                    prep_days: prep_days.min(60),
                    suppress_classes: suppress_classes.unwrap_or(
                        if kind == AcademicKind::Holiday {
                            SuppressClasses::WholeDay
                        } else {
                            SuppressClasses::OverlapOnly
                        },
                    ),
                    notes: notes.unwrap_or_default(),
                };
                if let Some(subject) = &event.subject_id {
                    ensure_subject(self, subject)?;
                }
                self.save_academic_event(None, Some(&event), actor, "create_academic_event", now)?;
                Ok(Response::AcademicEvent(event))
            }
            Command::UpdateAcademicEvent {
                id,
                kind,
                title,
                subject_id,
                start_at,
                end_at,
                all_day,
                prep_days,
                suppress_classes,
                notes,
            } => {
                let before = self.event_by_id(&id)?;
                let term = data
                    .terms
                    .iter()
                    .find(|term| term.id == before.term_id)
                    .cloned()
                    .ok_or_else(|| Error::Validation("Term not found".into()))?;
                if term.archived {
                    return Err(Error::Validation("Archived terms are read-only".into()));
                }
                if let Some(subject) = &subject_id {
                    ensure_subject(self, subject)?;
                }
                let mut event = before.clone();
                event.kind = kind;
                event.title = event_title(&title)?;
                event.subject_id = subject_id;
                event.start_at = start_at;
                event.end_at = event_end(start_at, end_at, all_day)?;
                event.all_day = all_day;
                event.prep_days = prep_days.min(60);
                event.suppress_classes = suppress_classes;
                event.notes = notes.unwrap_or_default();
                self.save_academic_event(
                    Some(&before),
                    Some(&event),
                    actor,
                    "update_academic_event",
                    now,
                )?;
                Ok(Response::AcademicEvent(event))
            }
            Command::DeleteAcademicEvent { id } => {
                let before = self.event_by_id(&id)?;
                ensure_term_writable(&data.terms, &before.term_id)?;
                self.save_academic_event(Some(&before), None, actor, "delete_academic_event", now)?;
                Ok(Response::AcademicEvent(before))
            }
            Command::ListAcademicEvents { from, to } => {
                check_range(from, to, 800)?;
                Ok(Response::AcademicEvents(
                    data.events
                        .iter()
                        .filter(|event| {
                            let start = event.start_at.with_timezone(&tz).date_naive();
                            start >= from && start <= to
                        })
                        .cloned()
                        .collect(),
                ))
            }
            Command::GetDayStatus { date } => {
                let tasks = self.tasks()?;
                let input = Self::load_input(settings, tz, &tasks, &data);
                Ok(Response::DayStatus(load::day_status(&input, date)))
            }
            Command::ListConflicts { from, to } => {
                check_range(from, to, 60)?;
                let tasks = self.tasks()?;
                let input = Self::load_input(settings, tz, &tasks, &data);
                let dismissed = self.dismissed_conflicts()?;
                Ok(Response::Conflicts(load::conflicts(
                    &input, from, to, &dismissed,
                )))
            }
            Command::ApplySuggestion {
                task_id,
                date,
                action,
                dry_run,
            } => {
                let before = self.task(&task_id)?;
                if before.deleted_at.is_some() || !before.status.active() {
                    return Err(Error::Validation(
                        "Only active tasks can be rescheduled".into(),
                    ));
                }
                let tasks = self.tasks()?;
                let input = Self::load_input(settings, tz, &tasks, &data);
                let target = load::suggest_reschedule(&input, &before, date + Duration::days(1))
                    .unwrap_or(date + Duration::days(1));
                match action {
                    SuggestionAction::Move | SuggestionAction::Defer => {
                        let mut task = before.clone();
                        task.planned_day = Some(target);
                        task.status = Status::Todo;
                        task.scheduled_start = None;
                        task.scheduled_end = None;
                        if !dry_run {
                            self.save(Some(&before), &task, actor, "apply_suggestion", now)?;
                        }
                        Ok(Response::Task(task))
                    }
                    SuggestionAction::Split => {
                        let half = (before.est_min / 2).max(15);
                        let remainder = before.est_min.saturating_sub(half).max(15);
                        let mut first = before.clone();
                        first.est_min = half;
                        first.planned_day = Some(date);
                        first.status = Status::Todo;
                        let mut second = before.clone();
                        second.id = ulid::Ulid::new().to_string();
                        second.title = format!("{} (part 2)", before.title);
                        second.est_min = remainder;
                        second.planned_day = Some(target);
                        second.status = Status::Todo;
                        second.created_at = now;
                        second.scheduled_start = None;
                        second.scheduled_end = None;
                        second.actual_min = 0;
                        second.completed_at = None;
                        second.reminded_at = None;
                        second.carried_count = 0;
                        if !dry_run {
                            self.save(Some(&before), &first, actor, "apply_suggestion", now)?;
                            self.save(None, &second, actor, "apply_suggestion", now)?;
                        }
                        Ok(Response::Tasks(vec![first, second]))
                    }
                }
            }
            Command::DismissConflict {
                task_id,
                date,
                rule_id,
            } => {
                let rule = rule_id.trim();
                if rule.is_empty() {
                    return Err(Error::Validation("Missing conflict rule".into()));
                }
                self.conn.execute(
                    "INSERT OR IGNORE INTO dismissed_conflicts(task_id,date,rule_id,created_at) VALUES(?1,?2,?3,?4)",
                    params![task_id, date.to_string(), rule, now.to_rfc3339()],
                )?;
                self.event_payload(
                    actor,
                    "dismissed_conflict",
                    "dismiss_conflict",
                    &format!("{task_id}:{date}:{rule}"),
                    serde_json::json!({ "task_id": task_id, "date": date, "rule_id": rule }),
                    now,
                )?;
                Ok(Response::Conflicts(Vec::new()))
            }
            Command::PreviewPrepPlan { event_id } => {
                let event = self.event_by_id(&event_id)?;
                let tasks = self.tasks()?;
                let input = Self::load_input(settings, tz, &tasks, &data);
                let syllabus = self.syllabus_items()?;
                Ok(Response::PrepPlan(load::prep_plan(
                    &input, &event, &syllabus,
                )))
            }
            Command::ApplyPrepPlan { event_id, dry_run } => {
                let event = self.event_by_id(&event_id)?;
                let tasks = self.tasks()?;
                let input = Self::load_input(settings, tz, &tasks, &data);
                let syllabus = self.syllabus_items()?;
                let plan = load::prep_plan(&input, &event, &syllabus);
                let mut created = Vec::new();
                for proposal in plan.tasks {
                    let task = Task {
                        id: ulid::Ulid::new().to_string(),
                        title: proposal.title,
                        notes: String::new(),
                        status: Status::Todo,
                        priority: 2,
                        energy: Energy::High,
                        est_min: proposal.est_min.max(1),
                        actual_min: 0,
                        due_at: None,
                        planned_day: Some(proposal.day),
                        scheduled_start: None,
                        scheduled_end: None,
                        parent_id: None,
                        project_id: None,
                        goal_id: None,
                        dsa_problem_id: None,
                        recurrence_rule: None,
                        tags: {
                            let mut tags = proposal.tags;
                            if !tags.iter().any(|tag| tag == "!study") {
                                tags.push("!study".into());
                            }
                            tags
                        },
                        carried_count: 0,
                        created_at: now,
                        completed_at: None,
                        deleted_at: None,
                        reminded_at: None,
                    };
                    if !dry_run {
                        self.save(None, &task, actor, "apply_prep_plan", now)?;
                    }
                    created.push(task);
                }
                Ok(Response::Tasks(created))
            }
            Command::WeekLoad { from, to } => {
                check_range(from, to, 90)?;
                let tasks = self.tasks()?;
                let input = Self::load_input(settings, tz, &tasks, &data);
                Ok(Response::WeekLoad(load::week_load(&input, from, to)))
            }
            _ => Err(Error::Validation(
                "Command is not part of the calendar feature".into(),
            )),
        }
    }

    pub fn next_wakeup(&self) -> Result<Option<Wakeup>> {
        let settings = self.settings()?;
        let tz: Tz = settings
            .timezone
            .parse()
            .map_err(|_| Error::Validation("Invalid IANA timezone".into()))?;
        let today = Utc::now().with_timezone(&tz).date_naive();
        let mut candidates: Vec<Wakeup> = self
            .tasks()?
            .into_iter()
            .filter(|task| {
                task.deleted_at.is_none() && task.status.active() && task.reminded_at.is_none()
            })
            .filter_map(|task| {
                task.scheduled_start.or(task.due_at).map(|at| Wakeup {
                    kind: WakeupKind::Task,
                    at,
                    title: task.title.clone(),
                    entity_id: task.id.clone(),
                    key: task.id.clone(),
                })
            })
            .collect();
        let data = self.calendar_data()?;
        let occurrences = schedule::occurrences(
            today,
            today + Duration::days(7),
            &data.items,
            &data.exceptions,
            &data.terms,
            &data.events,
            tz,
        );
        for occurrence in occurrences {
            let Some(lead) = occurrence.remind_before_min else {
                continue;
            };
            let key = format!("{}:{}", occurrence.item_id, occurrence.date);
            if self.reminder_delivered("schedule_occurrence", &key)? {
                continue;
            }
            let at = occurrence.start_utc - Duration::minutes(lead as i64);
            candidates.push(Wakeup {
                kind: WakeupKind::Schedule,
                at,
                title: occurrence.title,
                entity_id: occurrence.item_id,
                key,
            });
        }
        let active_term = data
            .terms
            .iter()
            .find(|term| term.active && !term.archived)
            .map(|term| term.id.clone());
        for event in data
            .events
            .iter()
            .filter(|event| event.kind.is_exam())
            .filter(|event| Some(&event.term_id) == active_term.as_ref())
        {
            let exam_day = event.start_at.with_timezone(&tz).date_naive();
            if exam_day < today || exam_day > today + Duration::days(60) {
                continue;
            }
            let offsets = [(7_i64, "09:00"), (3, "09:00"), (1, "09:00"), (0, "08:00")];
            for (days, time) in offsets {
                let day = exam_day - Duration::days(days);
                let Ok((at, _)) = schedule::local_bounds(day, time, "23:59", tz) else {
                    continue;
                };
                let key = format!("{}:T{}", event.id, days);
                if self.reminder_delivered("academic_event", &key)? {
                    continue;
                }
                candidates.push(Wakeup {
                    kind: WakeupKind::Exam,
                    at,
                    title: event.title.clone(),
                    entity_id: event.id.clone(),
                    key,
                });
            }
        }
        Ok(candidates.into_iter().min_by_key(|wakeup| wakeup.at))
    }
    fn reminder_delivered(&self, entity: &str, key: &str) -> Result<bool> {
        let exists: i64 = self.conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM events WHERE entity=?1 AND entity_id=?2 AND action='reminder_delivered')",
            params![entity, key],
            |row| row.get(0),
        )?;
        Ok(exists != 0)
    }
    pub fn mark_wakeup_delivered(&mut self, wakeup: &Wakeup, now: DateTime<Utc>) -> Result<()> {
        let entity = match wakeup.kind {
            WakeupKind::Task => "task",
            WakeupKind::Schedule => "schedule_occurrence",
            WakeupKind::Exam => "academic_event",
        };
        self.event_payload(
            Actor::Ui,
            entity,
            "reminder_delivered",
            &wakeup.key,
            serde_json::json!({ "title": wakeup.title, "entity_id": wakeup.entity_id }),
            now,
        )
    }
    pub fn undo_last_any(
        &mut self,
        actor: Actor,
        now: DateTime<Utc>,
    ) -> Result<Option<UndoOutcome>> {
        let row = self
            .conn
            .query_row(
                "SELECT entity, action, entity_id, payload FROM events WHERE entity IN ('task','schedule_item','schedule_exception','term','academic_event','dismissed_conflict') AND action NOT IN ('undo','reminder_delivered','reminder_failed','set_active_term_meta') ORDER BY ts DESC, rowid DESC LIMIT 1",
                [],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                    ))
                },
            )
            .optional()?;
        let Some((entity, action, id, payload)) = row else {
            return Ok(None);
        };
        let payload: serde_json::Value = serde_json::from_str(&payload)?;
        let before = payload
            .get("before")
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        let after = payload
            .get("after")
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        match entity.as_str() {
            "task" => {
                let current = self.task(&id)?;
                if serde_json::to_value(&current)? != after {
                    return Err(Error::Validation(
                        "The latest change cannot be undone because the task has changed since"
                            .into(),
                    ));
                }
                let mut restored: Task = if before.is_null() {
                    let mut deleted = current.clone();
                    deleted.deleted_at = Some(now);
                    deleted
                } else {
                    serde_json::from_value(before)?
                };
                if restored.id.is_empty() {
                    restored.id = current.id.clone();
                }
                self.save(Some(&current), &restored, actor, "undo", now)?;
            }
            "schedule_item" => {
                let current = self.find_item(&id)?;
                let expected = if after.is_null() {
                    serde_json::Value::Null
                } else {
                    after.clone()
                };
                let matches = if expected.is_null() {
                    current.deleted_at.is_some()
                } else {
                    serde_json::to_value(&current)? == expected
                };
                if !matches {
                    return Err(Error::Validation(
                        "The latest change cannot be undone because the item changed since".into(),
                    ));
                }
                let mut restored: ScheduleItem = if before.is_null() {
                    let mut deleted = current.clone();
                    deleted.deleted_at = Some(now);
                    deleted.active = false;
                    deleted
                } else {
                    serde_json::from_value(before)?
                };
                self.save_schedule_item(Some(&current), &restored, actor, "undo", now)?;
                let _ = &mut restored;
            }
            "schedule_exception" => {
                let key = id.split_once(':').and_then(|(item, date)| {
                    NaiveDate::parse_from_str(date, "%Y-%m-%d")
                        .ok()
                        .map(|d| (item, d))
                });
                let Some((item_id, date)) = key else {
                    return Ok(None);
                };
                let current = self.find_exception(item_id, date)?;
                let current_value = match &current {
                    Some(exception) => serde_json::to_value(exception)?,
                    None => serde_json::Value::Null,
                };
                if current_value != after {
                    return Err(Error::Validation(
                        "The latest change cannot be undone because the exception changed since"
                            .into(),
                    ));
                }
                let restore: Option<ScheduleException> = if before.is_null() {
                    None
                } else {
                    Some(serde_json::from_value(before)?)
                };
                self.save_exception(current.as_ref(), restore.as_ref(), actor, "undo", now)?;
            }
            "term" => {
                let current = self
                    .terms()?
                    .into_iter()
                    .find(|term| term.id == id)
                    .ok_or_else(|| Error::Validation("Term not found".into()))?;
                if serde_json::to_value(&current)? != after {
                    return Err(Error::Validation(
                        "The latest change cannot be undone because the term changed since".into(),
                    ));
                }
                let restored: Term = serde_json::from_value(before)?;
                self.save_term(Some(&current), &restored, actor, "undo", now)?;
            }
            "academic_event" => {
                let current = self.event_by_id(&id).ok();
                let current_value = match &current {
                    Some(event) => serde_json::to_value(event)?,
                    None => serde_json::Value::Null,
                };
                if current_value != after {
                    return Err(Error::Validation(
                        "The latest change cannot be undone because the event changed since".into(),
                    ));
                }
                let restore: Option<AcademicEvent> = if before.is_null() {
                    None
                } else {
                    Some(serde_json::from_value(before)?)
                };
                self.save_academic_event(current.as_ref(), restore.as_ref(), actor, "undo", now)?;
            }
            "dismissed_conflict" => {
                let parts: Vec<&str> = id.splitn(3, ':').collect();
                if parts.len() == 3 {
                    self.conn.execute(
                        "DELETE FROM dismissed_conflicts WHERE task_id=?1 AND date=?2 AND rule_id=?3",
                        params![parts[0], parts[1], parts[2]],
                    )?;
                }
            }
            _ => return Ok(None),
        }
        Ok(Some(UndoOutcome { entity, action, id }))
    }
}

fn ensure_term_writable(terms: &[Term], term_id: &str) -> Result<()> {
    let term = terms
        .iter()
        .find(|term| term.id == term_id)
        .ok_or_else(|| Error::Validation("Term not found".into()))?;
    if term.archived {
        return Err(Error::Validation(
            "Archived terms are read-only; choose the active term".into(),
        ));
    }
    Ok(())
}
fn ensure_subject(core: &Core, id: &str) -> Result<()> {
    let subject = core
        .subjects()?
        .into_iter()
        .find(|subject| subject.id == id)
        .ok_or_else(|| Error::Validation("Subject not found".into()))?;
    crate::ensure_live_entity(subject.deleted_at, "subject")
}
fn validate_term(name: &str, start: NaiveDate, end: NaiveDate) -> Result<String> {
    let name = name.trim();
    if name.is_empty() || name.len() > 200 {
        return Err(Error::Validation(
            "Enter a term name between 1 and 200 bytes".into(),
        ));
    }
    if end < start {
        return Err(Error::Validation(
            "The term must end on or after it starts".into(),
        ));
    }
    Ok(name.to_string())
}
fn event_title(title: &str) -> Result<String> {
    let title = title.trim();
    if title.is_empty() || title.len() > 200 {
        return Err(Error::Validation(
            "Enter an event title between 1 and 200 bytes".into(),
        ));
    }
    Ok(title.to_string())
}
fn event_end(start: DateTime<Utc>, end: DateTime<Utc>, all_day: bool) -> Result<DateTime<Utc>> {
    if end < start || (!all_day && end == start) {
        return Err(Error::Validation(
            "The event must end after it starts".into(),
        ));
    }
    Ok(end)
}
fn check_range(from: NaiveDate, to: NaiveDate, max_days: i64) -> Result<()> {
    if to < from {
        return Err(Error::Validation(
            "The end date must follow the start date".into(),
        ));
    }
    if (to - from).num_days() > max_days {
        return Err(Error::Validation(format!(
            "Choose a range of at most {max_days} days"
        )));
    }
    Ok(())
}
fn validate_occurrence_date(item: &ScheduleItem, terms: &[Term], date: NaiveDate) -> Result<()> {
    let term = terms
        .iter()
        .find(|term| term.id == item.term_id)
        .ok_or_else(|| Error::Validation("Term not found".into()))?;
    if date < term.start_date || date > term.end_date {
        return Err(Error::Validation(
            "That date is outside the item's term".into(),
        ));
    }
    if item.weekdays & schedule::weekday_bit(date) == 0 {
        return Err(Error::Validation(
            "This item does not occur on that weekday".into(),
        ));
    }
    Ok(())
}
