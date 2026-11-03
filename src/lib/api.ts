import { invoke, isTauri } from '@tauri-apps/api/core';
export type Status = 'inbox' | 'todo' | 'doing' | 'done' | 'dropped';
export type Energy = 'low' | 'high';
export type ProjectKind = 'coding' | 'college' | 'personal';
export type ProjectStatus = 'active' | 'paused' | 'archived';
export type GoalHorizon = 'medium' | 'long';
export type DsaDifficulty = 'easy' | 'medium' | 'hard';
export type DsaStatus = 'todo' | 'solved';
export interface Task {
  id: string;
  title: string;
  notes: string;
  status: Status;
  priority: number;
  energy: Energy;
  est_min: number;
  actual_min: number;
  due_at: string | null;
  planned_day: string | null;
  scheduled_start: string | null;
  scheduled_end: string | null;
  parent_id: string | null;
  project_id: string | null;
  goal_id: string | null;
  dsa_problem_id: string | null;
  tags: string[];
  carried_count: number;
  created_at: string;
  completed_at: string | null;
  deleted_at: string | null;
  reminded_at: string | null;
}
export interface Project {
  id: string;
  name: string;
  kind: ProjectKind;
  repo_path: string | null;
  branch: string | null;
  status: ProjectStatus;
  created_at: string;
  deleted_at: string | null;
}
export interface Goal {
  id: string;
  title: string;
  horizon: GoalHorizon;
  target_date: string | null;
  note: string;
  created_at: string;
  deleted_at: string | null;
}
export interface GoalProgress {
  goal_id: string;
  total: number;
  done: number;
}
export interface Template {
  id: string;
  name: string;
  title: string;
  notes: string;
  priority: number;
  energy: Energy;
  est_min: number;
  tags: string[];
  project_id: string | null;
  created_at: string;
  deleted_at: string | null;
}
export interface SavedFilter {
  id: string;
  name: string;
  view: string;
  query: string;
  created_at: string;
}
export interface DsaProblem {
  id: string;
  title: string;
  url: string | null;
  topic: string;
  difficulty: DsaDifficulty;
  platform: string | null;
  status: DsaStatus;
  confidence: number;
  solved_at: string | null;
  next_review_at: string | null;
  review_stage: number;
  notes: string;
  created_at: string;
  deleted_at: string | null;
}
export interface TopicCoverage {
  topic: string;
  total: number;
  solved: number;
  average_confidence: number;
}
export interface Subject {
  id: string;
  name: string;
  created_at: string;
  deleted_at: string | null;
}
export interface SyllabusItem {
  id: string;
  subject_id: string;
  title: string;
  done: boolean;
  created_at: string;
  deleted_at: string | null;
}
export interface Deadline {
  id: string;
  subject_id: string | null;
  title: string;
  due_date: string;
  study_min: number;
  created_at: string;
  deleted_at: string | null;
}
export interface Settings {
  timezone: string;
  max_tasks: number;
  max_minutes: number;
  work_start: number;
  work_end: number;
}
export interface Snapshot {
  tasks: Task[];
  settings: Settings;
  today: string;
  capacity: {
    count: number;
    minutes: number;
    over_count: boolean;
    over_minutes: boolean;
  };
  tags: string[];
  projects: Project[];
  goals: Goal[];
  goal_progress: GoalProgress[];
  templates: Template[];
  saved_filters: SavedFilter[];
  dsa_problems: DsaProblem[];
  dsa_topics: TopicCoverage[];
  subjects: Subject[];
  syllabus_items: SyllabusItem[];
  deadlines: Deadline[];
}
export type TaskEdit = Pick<
  Task,
  | 'id'
  | 'title'
  | 'notes'
  | 'status'
  | 'priority'
  | 'energy'
  | 'est_min'
  | 'actual_min'
  | 'tags'
  | 'parent_id'
  | 'project_id'
  | 'goal_id'
  | 'planned_day'
> & { due_local: string | null; scheduled_local: string | null };
export type Command =
  | { command: 'ping_db' }
  | { command: 'snapshot'; input: { search: string | null } }
  | {
      command: 'create_task';
      input: { text: string; parent_id: string | null };
    }
  | { command: 'update_task'; input: TaskEdit }
  | { command: 'complete_task'; input: { id: string; completed: boolean } }
  | { command: 'delete_task'; input: { id: string; dry_run: boolean } }
  | { command: 'restore_task'; input: { id: string } }
  | {
      command: 'reschedule_task';
      input: {
        id: string;
        local_start: string | null;
        day: string | null;
        dry_run: boolean;
      };
    }
  | { command: 'rollover'; input: { dry_run: boolean } }
  | { command: 'save_settings'; input: Settings }
  | { command: 'plan_day'; input: { day: string; energy: Energy | null } }
  | {
      command: 'create_project';
      input: {
        name: string;
        kind: ProjectKind;
        repo_path: string | null;
        branch: string | null;
      };
    }
  | {
      command: 'update_project';
      input: {
        id: string;
        name: string;
        kind: ProjectKind;
        repo_path: string | null;
        branch: string | null;
        status: ProjectStatus;
      };
    }
  | { command: 'delete_project'; input: { id: string } }
  | {
      command: 'create_goal';
      input: {
        title: string;
        horizon: GoalHorizon;
        target_date: string | null;
        note: string;
      };
    }
  | {
      command: 'update_goal';
      input: {
        id: string;
        title: string;
        horizon: GoalHorizon;
        target_date: string | null;
        note: string;
      };
    }
  | { command: 'delete_goal'; input: { id: string } }
  | {
      command: 'create_template';
      input: {
        name: string;
        title: string;
        notes: string;
        priority: number;
        energy: Energy;
        est_min: number;
        tags: string[];
        project_id: string | null;
      };
    }
  | {
      command: 'create_task_from_template';
      input: { template_id: string; parent_id: string | null };
    }
  | { command: 'delete_template'; input: { id: string } }
  | { command: 'import_issue'; input: { url: string } }
  | {
      command: 'save_filter';
      input: { name: string; view: string; query: string };
    }
  | { command: 'delete_filter'; input: { id: string } }
  | {
      command: 'create_dsa_problem';
      input: {
        title: string;
        url: string | null;
        topic: string;
        difficulty: DsaDifficulty;
        platform: string | null;
        notes: string;
      };
    }
  | {
      command: 'update_dsa_problem';
      input: {
        id: string;
        title: string;
        url: string | null;
        topic: string;
        difficulty: DsaDifficulty;
        platform: string | null;
        notes: string;
      };
    }
  | {
      command: 'solve_dsa_problem';
      input: { id: string; confidence: number };
    }
  | {
      command: 'review_dsa_problem';
      input: { id: string; confidence: number };
    }
  | { command: 'delete_dsa_problem'; input: { id: string } }
  | { command: 'create_subject'; input: { name: string } }
  | { command: 'delete_subject'; input: { id: string } }
  | {
      command: 'add_syllabus_item';
      input: { subject_id: string; title: string };
    }
  | {
      command: 'set_syllabus_item_done';
      input: { id: string; done: boolean };
    }
  | { command: 'delete_syllabus_item'; input: { id: string } }
  | {
      command: 'create_deadline';
      input: {
        subject_id: string | null;
        title: string;
        due_date: string;
        study_min: number;
      };
    }
  | { command: 'delete_deadline'; input: { id: string } };
export type Response =
  | { type: 'snapshot'; data: Snapshot }
  | { type: 'task'; data: Task }
  | { type: 'tasks'; data: Task[] }
  | { type: 'settings'; data: Settings }
  | { type: 'project'; data: Project }
  | { type: 'goal'; data: Goal }
  | { type: 'template'; data: Template }
  | { type: 'filter'; data: SavedFilter }
  | { type: 'dsa_problem'; data: DsaProblem }
  | { type: 'subject'; data: Subject }
  | { type: 'syllabus_item'; data: SyllabusItem }
  | { type: 'deadline'; data: Deadline }
  | { type: 'pong'; data: string };
export const native = isTauri();
export async function dispatch(command: Command): Promise<Response> {
  if (!native)
    throw new Error(
      'Open the desktop app with npm run desktop. Browser mode cannot access your local database.',
    );
  return invoke<Response>('dispatch', { command });
}
export async function openInVscode(path: string): Promise<void> {
  if (!native)
    throw new Error(
      'Open the desktop app with npm run desktop. Browser mode cannot launch VS Code.',
    );
  return invoke<void>('open_in_vscode', { path });
}
