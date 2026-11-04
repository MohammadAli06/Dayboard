CREATE TABLE schedule_items(
  id TEXT PRIMARY KEY,
  title TEXT NOT NULL,
  kind TEXT NOT NULL CHECK(kind IN ('class','work','routine','other')),
  weekdays INTEGER NOT NULL CHECK(weekdays BETWEEN 1 AND 127),
  start_time TEXT NOT NULL,
  end_time TEXT NOT NULL,
  start_date TEXT,
  end_date TEXT,
  location TEXT,
  notes TEXT,
  subject_id TEXT REFERENCES subjects(id),
  remind_before_min INTEGER,
  active INTEGER NOT NULL DEFAULT 1,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  deleted INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX schedule_items_active ON schedule_items(active,deleted);
CREATE TABLE schedule_exceptions(
  id TEXT PRIMARY KEY,
  item_id TEXT NOT NULL REFERENCES schedule_items(id) ON DELETE CASCADE,
  date TEXT NOT NULL,
  action TEXT NOT NULL CHECK(action IN ('skip','move')),
  new_start TEXT,
  new_end TEXT,
  note TEXT,
  UNIQUE(item_id,date)
);
CREATE INDEX schedule_exceptions_date ON schedule_exceptions(date);
