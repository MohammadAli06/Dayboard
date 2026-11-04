CREATE TABLE terms(
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  start_date TEXT NOT NULL,
  end_date TEXT NOT NULL,
  active INTEGER NOT NULL DEFAULT 0,
  archived INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL
);
ALTER TABLE schedule_items ADD COLUMN term_id TEXT REFERENCES terms(id);
CREATE INDEX schedule_items_term ON schedule_items(term_id);
CREATE TABLE academic_events(
  id TEXT PRIMARY KEY,
  term_id TEXT NOT NULL REFERENCES terms(id),
  kind TEXT NOT NULL CHECK(kind IN ('midterm','practical','oral','final','assignment_due','holiday','other')),
  title TEXT NOT NULL,
  subject_id TEXT REFERENCES subjects(id),
  start_at TEXT NOT NULL,
  end_at TEXT NOT NULL,
  all_day INTEGER NOT NULL DEFAULT 0,
  prep_days INTEGER NOT NULL DEFAULT 5,
  suppress_classes TEXT NOT NULL DEFAULT 'overlap_only' CHECK(suppress_classes IN ('none','overlap_only','whole_day')),
  notes TEXT NOT NULL DEFAULT ''
);
CREATE INDEX academic_events_start ON academic_events(start_at);
CREATE INDEX academic_events_term ON academic_events(term_id);
CREATE TABLE dismissed_conflicts(
  task_id TEXT NOT NULL,
  date TEXT NOT NULL,
  rule_id TEXT NOT NULL,
  created_at TEXT NOT NULL,
  PRIMARY KEY(task_id,date,rule_id)
);
