import { invoke, isTauri } from '@tauri-apps/api/core';
export type Status = 'inbox' | 'todo' | 'doing' | 'done' | 'dropped';
export type Energy = 'low' | 'high';
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
  tags: string[];
  carried_count: number;
  created_at: string;
  completed_at: string | null;
  deleted_at: string | null;
  reminded_at: string | null;
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
  | { command: 'plan_day'; input: { day: string; energy: Energy | null } };
export type Response =
  | { type: 'snapshot'; data: Snapshot }
  | { type: 'task'; data: Task }
  | { type: 'tasks'; data: Task[] }
  | { type: 'settings'; data: Settings }
  | { type: 'pong'; data: string };
export const native = isTauri();
export async function dispatch(command: Command): Promise<Response> {
  if (!native)
    throw new Error(
      'Open the desktop app with npm run desktop. Browser mode cannot access your local database.',
    );
  return invoke<Response>('dispatch', { command });
}
