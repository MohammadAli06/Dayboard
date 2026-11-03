CREATE TABLE dsa_problems(id TEXT PRIMARY KEY, data TEXT NOT NULL CHECK(json_valid(data)), deleted INTEGER NOT NULL DEFAULT 0);
CREATE INDEX dsa_problems_deleted ON dsa_problems(deleted);
CREATE TABLE subjects(id TEXT PRIMARY KEY, data TEXT NOT NULL CHECK(json_valid(data)), deleted INTEGER NOT NULL DEFAULT 0);
CREATE INDEX subjects_deleted ON subjects(deleted);
CREATE TABLE syllabus_items(id TEXT PRIMARY KEY, data TEXT NOT NULL CHECK(json_valid(data)), deleted INTEGER NOT NULL DEFAULT 0);
CREATE INDEX syllabus_items_deleted ON syllabus_items(deleted);
CREATE TABLE deadlines(id TEXT PRIMARY KEY, data TEXT NOT NULL CHECK(json_valid(data)), deleted INTEGER NOT NULL DEFAULT 0);
CREATE INDEX deadlines_deleted ON deadlines(deleted);
