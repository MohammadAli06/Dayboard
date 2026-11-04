import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import {
  cleanup,
  render,
  screen,
  waitFor,
  fireEvent,
} from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import type { Command, Snapshot, Task } from './lib/api';
import App from './App.svelte';

const mocked = vi.hoisted(() => ({
  invoke: vi.fn(),
  listen: vi.fn(async () => () => {}),
  hide: vi.fn(),
}));
vi.mock('@tauri-apps/api/core', () => ({
  isTauri: () => true,
  invoke: mocked.invoke,
}));
vi.mock('@tauri-apps/api/event', () => ({ listen: mocked.listen }));
vi.mock('@tauri-apps/api/window', () => ({
  getCurrentWindow: () => ({ hide: mocked.hide }),
}));
const task: Task = {
  id: '01TEST',
  title: 'Read a chapter',
  notes: '',
  status: 'todo',
  priority: 2,
  energy: 'low',
  est_min: 30,
  actual_min: 0,
  due_at: null,
  planned_day: '2026-11-02',
  scheduled_start: null,
  scheduled_end: null,
  parent_id: null,
  project_id: null,
  goal_id: null,
  dsa_problem_id: null,
  recurrence_rule: null,
  tags: ['#college'],
  carried_count: 0,
  created_at: '2026-11-02T00:00:00Z',
  completed_at: null,
  deleted_at: null,
  reminded_at: null,
};
let snapshot: Snapshot;
beforeEach(() => {
  vi.clearAllMocks();
  snapshot = {
    tasks: [structuredClone(task)],
    settings: {
      timezone: 'UTC',
      max_tasks: 8,
      max_minutes: 360,
      work_start: 9,
      work_end: 18,
      bridge_enabled: false,
      bridge_port: 8765,
      bridge_token: '',
      autostart: false,
      onboarded: true,
      exam_day_new_task_cap: 0,
      day_before_exam_capacity_pct: 50,
      prep_window_days: 5,
      non_study_cap_in_prep_pct: 60,
      heavy_day_threshold_pct: 85,
      calibrate_estimates: true,
      calibration_min_samples: 5,
    },
    today: '2026-11-02',
    capacity: {
      count: 1,
      minutes: 30,
      over_count: false,
      over_minutes: false,
      available_minutes: 450,
      schedule_minutes: 0,
      cap_minutes: 480,
    },
    tags: ['#college'],
    projects: [],
    goals: [],
    goal_progress: [],
    templates: [],
    saved_filters: [],
    dsa_problems: [],
    dsa_topics: [],
    subjects: [],
    syllabus_items: [],
    deadlines: [],
    focus_sessions: [],
    active_focus: null,
    habits: [],
    habit_streaks: [],
    daily_reviews: [],
    weekly_stats: {
      week_start: '2026-10-27',
      completed: 0,
      created: 0,
      carried: 0,
      focus_minutes: 0,
      estimate_accuracy: 0,
      measured: 0,
      tag_stats: [],
    },
    schedule_items: [],
    today_occurrences: [],
    week_occurrences: [],
    terms: [],
    active_term: null,
    academic_events: [],
    day_status: {
      date: '2026-11-02',
      status: 'normal',
      available_min: 480,
      used_min: 30,
      reasons: [],
    },
    upcoming_exams: [],
    conflicts: [],
  };
  mocked.invoke.mockImplementation(
    async (_name: string, args: { command: Command }) => {
      if (args.command.command === 'snapshot')
        return { type: 'snapshot', data: structuredClone(snapshot) };
      if (args.command.command === 'complete_task')
        snapshot.tasks[0].status = 'done';
      return { type: 'task', data: structuredClone(task) };
    },
  );
});
afterEach(cleanup);
describe('desktop task flows', () => {
  it('keeps unscheduled todo tasks reachable in the inbox', async () => {
    snapshot.tasks[0].planned_day = null;
    const user = userEvent.setup();
    render(App);
    await screen.findByText('A little space to think.');
    await user.click(screen.getByRole('button', { name: /▣Inbox/ }));
    expect(await screen.findByText('Read a chapter')).toBeTruthy();
  });
  it('sends capture unchanged to Rust and clears it only after success', async () => {
    const user = userEvent.setup();
    render(App);
    await screen.findByText('Read a chapter');
    await user.type(
      screen.getByLabelText('Quick add task'),
      'Write tests #work p1 45m tomorrow',
    );
    await user.click(screen.getByRole('button', { name: /Add task/ }));
    await waitFor(() =>
      expect(mocked.invoke).toHaveBeenCalledWith('dispatch', {
        command: {
          command: 'create_task',
          input: { text: 'Write tests #work p1 45m tomorrow', parent_id: null },
        },
      }),
    );
    await waitFor(() =>
      expect(
        (screen.getByLabelText('Quick add task') as HTMLInputElement).value,
      ).toBe(''),
    );
  });
  it('preserves failed capture and presents the command error', async () => {
    const user = userEvent.setup();
    render(App);
    await screen.findByText('Read a chapter');
    mocked.invoke.mockImplementation(
      async (_name: string, args: { command: Command }) => {
        if (args.command.command === 'create_task')
          throw new Error('Invalid date');
        return { type: 'snapshot', data: snapshot };
      },
    );
    await user.type(screen.getByLabelText('Quick add task'), 'Read 2026-02-30');
    await user.click(screen.getByRole('button', { name: /Add task/ }));
    await screen.findByRole('alert');
    expect(screen.getByRole('alert').textContent).toContain('Invalid date');
    expect(
      (screen.getByLabelText('Quick add task') as HTMLInputElement).value,
    ).toBe('Read 2026-02-30');
  });
  it('completes a task and removes it from Today', async () => {
    const user = userEvent.setup();
    render(App);
    await screen.findByText('Read a chapter');
    await user.click(
      screen.getByRole('button', { name: 'Complete Read a chapter' }),
    );
    await waitFor(() =>
      expect(screen.queryByText('Read a chapter')).toBeNull(),
    );
    expect(mocked.invoke).toHaveBeenCalledWith('dispatch', {
      command: {
        command: 'complete_task',
        input: { id: task.id, completed: true },
      },
    });
  });
  it('stages time-block edits without writing until Save', async () => {
    const user = userEvent.setup();
    render(App);
    await screen.findByText('Read a chapter');
    await user.click(
      screen.getByRole('button', { name: /Read a chapter.*#college/ }),
    );
    const input = screen.getByLabelText('Schedule task time');
    await fireEvent.input(input, { target: { value: '2026-11-02T11:00' } });
    expect(
      mocked.invoke.mock.calls.some(
        ([, args]) => args.command.command === 'reschedule_task',
      ),
    ).toBe(false);
    await user.click(screen.getByRole('button', { name: 'Close' }));
    expect(screen.queryByRole('dialog')).toBeNull();
  });
  it('saves details and time block through one atomic command', async () => {
    const user = userEvent.setup();
    render(App);
    await screen.findByText('Read a chapter');
    await user.click(
      screen.getByRole('button', { name: /Read a chapter.*#college/ }),
    );
    await fireEvent.input(screen.getByLabelText('Schedule task time'), {
      target: { value: '2026-11-02T11:00' },
    });
    await user.click(screen.getByRole('button', { name: 'Save changes' }));
    await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
    const writes = mocked.invoke.mock.calls.filter(
      ([, args]) => args.command.command !== 'snapshot',
    );
    expect(writes).toHaveLength(1);
    expect(writes[0][1].command).toMatchObject({
      command: 'update_task',
      input: { id: task.id, scheduled_local: '2026-11-02T11:00' },
    });
  });
  it('does not invite duplicate capture when saving succeeds but refresh fails', async () => {
    const user = userEvent.setup();
    render(App);
    await screen.findByText('Read a chapter');
    mocked.invoke.mockImplementation(
      async (_name: string, args: { command: Command }) => {
        if (args.command.command === 'snapshot')
          throw new Error('Database temporarily unavailable');
        return { type: 'task', data: task };
      },
    );
    await user.type(screen.getByLabelText('Quick add task'), 'A new task');
    await user.click(screen.getByRole('button', { name: /Add task/ }));
    await waitFor(() =>
      expect(
        (screen.getByLabelText('Quick add task') as HTMLInputElement).value,
      ).toBe(''),
    );
    expect(screen.getByRole('alert').textContent).toContain('Saved, but');
  });
  it('creates a project and shows it in the Projects view', async () => {
    const user = userEvent.setup();
    render(App);
    await screen.findByText('Read a chapter');
    await user.click(screen.getByRole('button', { name: /Projects/ }));
    mocked.invoke.mockImplementation(
      async (_name: string, args: { command: Command }) => {
        if (args.command.command === 'snapshot')
          return { type: 'snapshot', data: structuredClone(snapshot) };
        if (args.command.command === 'create_project') {
          const project = {
            id: '01PROJECT',
            name: args.command.input.name,
            kind: 'coding' as const,
            repo_path: null,
            branch: null,
            status: 'active' as const,
            created_at: '2026-11-02T00:00:00Z',
            deleted_at: null,
          };
          snapshot.projects = [project];
          return { type: 'project', data: project };
        }
        return { type: 'task', data: structuredClone(task) };
      },
    );
    await user.type(screen.getByLabelText('Project name'), 'Dayboard');
    await user.click(screen.getByRole('button', { name: /Add project/ }));
    await waitFor(() =>
      expect(screen.getAllByText('Dayboard').length).toBeGreaterThan(0),
    );
    expect(mocked.invoke).toHaveBeenCalledWith('dispatch', {
      command: {
        command: 'create_project',
        input: {
          name: 'Dayboard',
          kind: 'coding',
          repo_path: null,
          branch: null,
        },
      },
    });
  });
  it('imports a GitHub issue URL through the core command', async () => {
    const user = userEvent.setup();
    render(App);
    await screen.findByText('Read a chapter');
    await user.click(screen.getByRole('button', { name: /Projects/ }));
    await user.type(
      screen.getByLabelText('GitHub issue URL'),
      'https://github.com/acme/app/issues/7',
    );
    await user.click(screen.getByRole('button', { name: /Import issue/ }));
    await waitFor(() =>
      expect(mocked.invoke).toHaveBeenCalledWith('dispatch', {
        command: {
          command: 'import_issue',
          input: { url: 'https://github.com/acme/app/issues/7' },
        },
      }),
    );
  });
  it('adds a DSA problem in the Study view', async () => {
    const user = userEvent.setup();
    render(App);
    await screen.findByText('Read a chapter');
    await user.click(screen.getByRole('button', { name: /Study/ }));
    await user.type(screen.getByLabelText('DSA problem title'), 'Two Sum');
    await user.type(screen.getByLabelText('DSA topic'), 'arrays');
    await user.click(screen.getByRole('button', { name: /Add problem/ }));
    await waitFor(() =>
      expect(mocked.invoke).toHaveBeenCalledWith('dispatch', {
        command: {
          command: 'create_dsa_problem',
          input: {
            title: 'Two Sum',
            url: null,
            topic: 'arrays',
            difficulty: 'medium',
            platform: null,
            notes: '',
          },
        },
      }),
    );
  });
  it('logs a solved DSA problem with the chosen confidence', async () => {
    snapshot.dsa_problems = [
      {
        id: '01DSA',
        title: 'Two Sum',
        url: null,
        topic: 'arrays',
        difficulty: 'easy',
        platform: null,
        status: 'todo',
        confidence: 3,
        solved_at: null,
        next_review_at: null,
        review_stage: 0,
        notes: '',
        created_at: '2026-11-02T00:00:00Z',
        deleted_at: null,
      },
    ];
    const user = userEvent.setup();
    render(App);
    await screen.findByText('Read a chapter');
    await user.click(screen.getByRole('button', { name: /Study/ }));
    await user.click(screen.getByRole('button', { name: 'Log solved' }));
    await waitFor(() =>
      expect(mocked.invoke).toHaveBeenCalledWith('dispatch', {
        command: {
          command: 'solve_dsa_problem',
          input: { id: '01DSA', confidence: 3 },
        },
      }),
    );
  });
  it('creates a habit and saves a review in Insights', async () => {
    const user = userEvent.setup();
    render(App);
    await screen.findByText('Read a chapter');
    await user.click(screen.getByRole('button', { name: /Insights/ }));
    await user.type(screen.getByLabelText('Habit name'), 'Gym');
    await user.click(screen.getByRole('button', { name: /Add habit/ }));
    await waitFor(() =>
      expect(mocked.invoke).toHaveBeenCalledWith('dispatch', {
        command: {
          command: 'create_habit',
          input: { name: 'Gym', cadence_days: 1 },
        },
      }),
    );
    await user.type(screen.getByLabelText('Daily review note'), 'Shipped phase 5');
    await user.click(screen.getByRole('button', { name: 'Save review' }));
    await waitFor(() =>
      expect(mocked.invoke).toHaveBeenCalledWith('dispatch', {
        command: {
          command: 'save_daily_review',
          input: { day: '2026-11-02', note: 'Shipped phase 5' },
        },
      }),
    );
  });
  it('saves a recurrence rule from the task editor', async () => {
    const user = userEvent.setup();
    render(App);
    await screen.findByText('Read a chapter');
    await user.click(
      screen.getByRole('button', { name: /Read a chapter.*#college/ }),
    );
    await user.selectOptions(screen.getByLabelText('Repeats'), 'daily');
    await user.click(screen.getByRole('button', { name: 'Save changes' }));
    await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
    const writes = mocked.invoke.mock.calls.filter(
      ([, args]) => args.command.command === 'update_task',
    );
    expect(writes).toHaveLength(1);
    expect(writes[0][1].command).toMatchObject({
      command: 'update_task',
      input: { recurrence_rule: 'daily' },
    });
  });
  it('dismisses the first-run welcome card', async () => {
    snapshot.settings.onboarded = false;
    const user = userEvent.setup();
    render(App);
    await screen.findByText('Welcome to Daybridge');
    mocked.invoke.mockImplementation(
      async (_name: string, args: { command: Command }) => {
        if (args.command.command === 'save_settings') {
          snapshot.settings = args.command.input;
          return { type: 'settings', data: snapshot.settings };
        }
        return { type: 'snapshot', data: structuredClone(snapshot) };
      },
    );
    await user.click(screen.getByRole('button', { name: 'Got it' }));
    await waitFor(() =>
      expect(screen.queryByText('Welcome to Daybridge')).toBeNull(),
    );
  });
  it('saves a filter for an explicit view and applies it', async () => {
    const user = userEvent.setup();
    render(App);
    await screen.findByText('Read a chapter');
    await user.click(screen.getByRole('button', { name: /Projects/ }));
    mocked.invoke.mockImplementation(
      async (_name: string, args: { command: Command }) => {
        if (args.command.command === 'snapshot')
          return { type: 'snapshot', data: structuredClone(snapshot) };
        if (args.command.command === 'save_filter') {
          const filter = {
            id: '01FILTER',
            name: args.command.input.name,
            view: args.command.input.view,
            query: args.command.input.query,
            created_at: '2026-11-02T00:00:00Z',
          };
          snapshot.saved_filters = [filter];
          return { type: 'filter', data: filter };
        }
        return { type: 'task', data: structuredClone(task) };
      },
    );
    await user.type(screen.getByLabelText('Filter name'), 'Inbox graph');
    await user.selectOptions(screen.getByLabelText('Filter view'), 'Inbox');
    await user.type(screen.getByLabelText('Filter search'), 'graph');
    await user.click(screen.getByRole('button', { name: /Save filter/ }));
    await waitFor(() => expect(screen.getByText('Inbox graph')).toBeTruthy());
    await user.click(screen.getByRole('button', { name: 'Apply' }));
    await waitFor(() =>
      expect(
        (screen.getByLabelText('Search all tasks') as HTMLInputElement).value,
      ).toBe('graph'),
    );
    expect(screen.getByText('Out of your head. Into here.')).toBeTruthy();
    expect(mocked.invoke).toHaveBeenCalledWith('dispatch', {
      command: { command: 'snapshot', input: { search: 'graph' } },
    });
  });
});
