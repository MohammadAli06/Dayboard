CREATE TABLE projects(id TEXT PRIMARY KEY, data TEXT NOT NULL CHECK(json_valid(data)), deleted INTEGER NOT NULL DEFAULT 0);
CREATE INDEX projects_deleted ON projects(deleted);
CREATE TABLE goals(id TEXT PRIMARY KEY, data TEXT NOT NULL CHECK(json_valid(data)), deleted INTEGER NOT NULL DEFAULT 0);
CREATE INDEX goals_deleted ON goals(deleted);
CREATE TABLE templates(id TEXT PRIMARY KEY, data TEXT NOT NULL CHECK(json_valid(data)), deleted INTEGER NOT NULL DEFAULT 0);
CREATE INDEX templates_deleted ON templates(deleted);
CREATE TABLE saved_filters(id TEXT PRIMARY KEY, data TEXT NOT NULL CHECK(json_valid(data)), created_at TEXT NOT NULL);
