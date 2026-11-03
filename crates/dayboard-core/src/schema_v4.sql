CREATE TABLE focus_sessions(id TEXT PRIMARY KEY, data TEXT NOT NULL CHECK(json_valid(data)), deleted INTEGER NOT NULL DEFAULT 0);
CREATE INDEX focus_sessions_deleted ON focus_sessions(deleted);
CREATE TABLE habits(id TEXT PRIMARY KEY, data TEXT NOT NULL CHECK(json_valid(data)), deleted INTEGER NOT NULL DEFAULT 0);
CREATE INDEX habits_deleted ON habits(deleted);
CREATE TABLE habit_logs(id TEXT PRIMARY KEY, data TEXT NOT NULL CHECK(json_valid(data)));
CREATE INDEX habit_logs_habit ON habit_logs(json_extract(data,'$.habit_id'),json_extract(data,'$.day'));
CREATE TABLE daily_reviews(id TEXT PRIMARY KEY, data TEXT NOT NULL CHECK(json_valid(data)), deleted INTEGER NOT NULL DEFAULT 0);
CREATE INDEX daily_reviews_deleted ON daily_reviews(deleted);
