<script lang="ts">
  import { onMount } from 'svelte';
  import { listen } from '@tauri-apps/api/event';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import {
    dispatch,
    exportData,
    importData,
    native,
    openInVscode,
    type Snapshot,
    type Task,
    type TaskEdit,
    type Settings,
    type Command,
    type DsaDifficulty,
    type DsaProblem,
    type FocusKind,
    type FocusSession,
    type Goal,
    type GoalHorizon,
    type Habit,
    type Project,
    type ProjectKind,
    type SavedFilter,
    type Status,
    type SyllabusItem,
  } from './lib/api';
  import { localInput, addDays } from './lib/dates';
  const capture = new URLSearchParams(location.search).has('capture');
  let snapshot = $state<Snapshot | null>(null);
  let view = $state('Today');
  let text = $state('');
  let search = $state('');
  let error = $state('');
  let notice = $state('');
  let busy = $state(false);
  let selected = $state(0);
  let palette = $state(false);
  let edit = $state<TaskEdit | null>(null);
  let editTags = $state('');
  let editStart = $state('');
  let preferences = $state<Settings | null>(null);
  let deleted = $state<Task | null>(null);
  let suggestions = $state<Task[] | null>(null);
  let day = $state('');
  let quickInput = $state<HTMLInputElement>();
  let searchInput = $state<HTMLInputElement>();
  let projectId = $state<string | null>(null);
  let projectEditId = $state<string | null>(null);
  let goalEditId = $state<string | null>(null);
  let issueUrl = $state('');
  let filterDraft = $state({ name: '', view: 'Inbox', query: '' });
  let projectDraft = $state({
    name: '',
    kind: 'coding' as ProjectKind,
    repo_path: '',
    branch: '',
  });
  let goalDraft = $state({
    title: '',
    horizon: 'medium' as GoalHorizon,
    target_date: '',
    note: '',
  });
  let dsaDraft = $state({
    title: '',
    topic: '',
    difficulty: 'medium' as DsaDifficulty,
    url: '',
    platform: '',
  });
  let dsaConfidence = $state(3);
  let subjectDraft = $state('');
  let selectedSubjectId = $state<string | null>(null);
  let syllabusDraft = $state('');
  let deadlineDraft = $state({ title: '', due_date: '', study_min: 60 });
  let focusDraft = $state({
    task_id: '',
    planned_min: 25,
    kind: 'focus' as FocusKind,
  });
  let focusElapsedMinutes = $state(0);
  let habitDraft = $state({ name: '', cadence_days: 1 });
  let reviewNote = $state('');
  let reviewSynced = $state(false);
  let importPath = $state('');
  let dataNotice = $state('');
  const views = [
    'Today',
    'Inbox',
    'Planner',
    'Projects',
    'Study',
    'Insights',
    'Completed',
  ];
  const boardColumns: { label: string; statuses: Status[] }[] = [
    { label: 'Backlog', statuses: ['inbox', 'todo'] },
    { label: 'Doing', statuses: ['doing'] },
    { label: 'Done', statuses: ['done'] },
  ];
  let tasks = $derived(
    (snapshot?.tasks ?? [])
      .filter((t) => {
        if (search.trim()) return true;
        if (view === 'Completed')
          return t.status === 'done' || t.status === 'dropped';
        if (t.status === 'done' || t.status === 'dropped') return false;
        if (view === 'Inbox') return t.planned_day === null;
        return t.planned_day === (view === 'Planner' ? day : snapshot?.today);
      })
      .sort(
        (a, b) =>
          a.priority - b.priority || a.created_at.localeCompare(b.created_at),
      ),
  );
  let visibleTasks = $derived(
    tasks.slice(Math.max(0, selected - 40), Math.max(80, selected + 40)),
  );
  let selectedProject = $derived(
    snapshot?.projects.find((project) => project.id === projectId) ?? null,
  );
  let projectTasks = $derived(
    projectId
      ? (snapshot?.tasks ?? []).filter((task) => task.project_id === projectId)
      : [],
  );
  let dsaProblems = $derived(snapshot?.dsa_problems ?? []);
  let dsaTopics = $derived(snapshot?.dsa_topics ?? []);
  let subjects = $derived(snapshot?.subjects ?? []);
  let selectedSubject = $derived(
    subjects.find((subject) => subject.id === selectedSubjectId) ?? null,
  );
  let subjectSyllabus = $derived(
    (snapshot?.syllabus_items ?? []).filter(
      (item) => item.subject_id === selectedSubjectId,
    ),
  );
  let subjectDeadlines = $derived(
    (snapshot?.deadlines ?? []).filter(
      (deadline) => deadline.subject_id === selectedSubjectId,
    ),
  );
  let activeFocus = $derived(snapshot?.active_focus ?? null);
  let habits = $derived(snapshot?.habits ?? []);
  let habitStreaks = $derived(snapshot?.habit_streaks ?? []);
  let todayReview = $derived(
    snapshot?.daily_reviews.find((review) => review.day === snapshot?.today) ??
      null,
  );
  let weeklyStats = $derived(snapshot?.weekly_stats ?? null);
  let days = $derived(
    snapshot
      ? Array.from({ length: 7 }, (_, i) => addDays(snapshot!.today, i))
      : [],
  );
  let hours = $derived(
    snapshot
      ? Array.from(
          { length: snapshot.settings.work_end - snapshot.settings.work_start },
          (_, i) => snapshot!.settings.work_start + i,
        )
      : [],
  );
  let tagMatches = $derived(text.match(/(?:^|\s)([#@!][^\s]*)$/)?.[1] ?? '');
  async function refresh() {
    const result = await dispatch({
      command: 'snapshot',
      input: { search: search || null },
    });
    if (result.type === 'snapshot') {
      snapshot = result.data;
      if (!day) day = result.data.today;
      selected = Math.max(0, Math.min(selected, tasks.length - 1));
    }
  }
  async function run(command: Command) {
    if (busy) return;
    busy = true;
    error = '';
    try {
      const result = await dispatch(command);
      try {
        await refresh();
      } catch (e) {
        error = `Saved, but the view could not refresh: ${String(e)}`;
      }
      return result;
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }
  async function create() {
    if (!text.trim()) return;
    const result = await run({
      command: 'create_task',
      input: { text, parent_id: null },
    });
    if (result) {
      text = '';
      if (capture) await getCurrentWindow().hide();
    }
  }
  function openEdit(task: Task) {
    edit = {
      id: task.id,
      title: task.title,
      notes: task.notes,
      status: task.status,
      priority: task.priority,
      energy: task.energy,
      est_min: task.est_min,
      actual_min: task.actual_min,
      tags: [...task.tags],
      parent_id: task.parent_id,
      project_id: task.project_id,
      goal_id: task.goal_id,
      recurrence_rule: task.recurrence_rule,
      planned_day: task.planned_day,
      due_local: localInput(task.due_at, snapshot!.settings.timezone) || null,
      scheduled_local:
        localInput(task.scheduled_start, snapshot!.settings.timezone) || null,
    };
    editTags = task.tags.join(' ');
    editStart = localInput(task.scheduled_start, snapshot!.settings.timezone);
  }
  async function saveEdit() {
    if (!edit) return;
    const result = await run({
      command: 'update_task',
      input: {
        ...edit,
        tags: editTags.split(/\s+/).filter(Boolean),
        planned_day: edit.planned_day || null,
        due_local: edit.due_local || null,
        parent_id: edit.parent_id || null,
        project_id: edit.project_id || null,
        goal_id: edit.goal_id || null,
        recurrence_rule: edit.recurrence_rule || null,
        scheduled_local: editStart || null,
      },
    });
    if (result) {
      edit = null;
      if (result.type === 'task_scheduled' && result.data.overlap.length) {
        notice = `Heads up: this block overlaps ${result.data.overlap
          .map((block) => block.title)
          .join(', ')}.`;
      }
    }
  }
  async function remove(task: Task) {
    const result = await run({
      command: 'delete_task',
      input: { id: task.id, dry_run: false },
    });
    if (result) {
      deleted = task;
      edit = null;
    }
  }
  function navigate(next: string) {
    view = next;
    selected = 0;
    palette = false;
    suggestions = null;
  }
  async function plan() {
    const result = await run({
      command: 'plan_day',
      input: { day: snapshot!.today, energy: null },
    });
    if (result?.type === 'tasks') suggestions = result.data;
  }
  async function schedule(
    id: string,
    local_start: string | null,
    target: string | null,
  ) {
    const result = await run({
      command: 'reschedule_task',
      input: { id, local_start, day: target, dry_run: false },
    });
    if (result?.type === 'task_scheduled' && result.data.overlap.length) {
      notice = `Heads up: this block overlaps ${result.data.overlap
        .map((block) => block.title)
        .join(', ')}.`;
    }
  }
  function progressFor(goalId: string) {
    return (
      snapshot?.goal_progress.find((progress) => progress.goal_id === goalId) ?? {
        done: 0,
        total: 0,
      }
    );
  }
  function editProject(project: Project) {
    projectEditId = project.id;
    projectDraft = {
      name: project.name,
      kind: project.kind,
      repo_path: project.repo_path ?? '',
      branch: project.branch ?? '',
    };
  }
  function cancelProjectEdit() {
    projectEditId = null;
    projectDraft = { name: '', kind: 'coding', repo_path: '', branch: '' };
  }
  async function saveProject() {
    if (!projectDraft.name.trim()) return;
    if (projectEditId) {
      const before = snapshot?.projects.find(
        (project) => project.id === projectEditId,
      );
      if (!before) return;
      const result = await run({
        command: 'update_project',
        input: {
          id: before.id,
          name: projectDraft.name,
          kind: projectDraft.kind,
          repo_path: projectDraft.repo_path.trim() || null,
          branch: projectDraft.branch.trim() || null,
          status: before.status,
        },
      });
      if (result?.type === 'project') {
        projectId = result.data.id;
        cancelProjectEdit();
      }
      return;
    }
    const result = await run({
      command: 'create_project',
      input: {
        name: projectDraft.name,
        kind: projectDraft.kind,
        repo_path: projectDraft.repo_path.trim() || null,
        branch: projectDraft.branch.trim() || null,
      },
    });
    if (result?.type === 'project') {
      projectId = result.data.id;
      cancelProjectEdit();
    }
  }
  async function removeProject(id: string) {
    const result = await run({ command: 'delete_project', input: { id } });
    if (result) {
      projectId = null;
      if (projectEditId === id) cancelProjectEdit();
    }
  }
  function editGoal(goal: Goal) {
    goalEditId = goal.id;
    goalDraft = {
      title: goal.title,
      horizon: goal.horizon,
      target_date: goal.target_date ?? '',
      note: goal.note,
    };
  }
  function cancelGoalEdit() {
    goalEditId = null;
    goalDraft = { title: '', horizon: 'medium', target_date: '', note: '' };
  }
  async function saveGoal() {
    if (!goalDraft.title.trim()) return;
    if (goalEditId) {
      const result = await run({
        command: 'update_goal',
        input: {
          id: goalEditId,
          title: goalDraft.title,
          horizon: goalDraft.horizon,
          target_date: goalDraft.target_date || null,
          note: goalDraft.note,
        },
      });
      if (result) cancelGoalEdit();
      return;
    }
    const result = await run({
      command: 'create_goal',
      input: {
        title: goalDraft.title,
        horizon: goalDraft.horizon,
        target_date: goalDraft.target_date || null,
        note: goalDraft.note,
      },
    });
    if (result) cancelGoalEdit();
  }
  async function removeGoal(id: string) {
    const result = await run({ command: 'delete_goal', input: { id } });
    if (result && goalEditId === id) cancelGoalEdit();
  }
  async function useTemplate(id: string) {
    const result = await run({
      command: 'create_task_from_template',
      input: { template_id: id, parent_id: null },
    });
    if (result) navigate('Inbox');
  }
  async function removeTemplate(id: string) {
    await run({ command: 'delete_template', input: { id } });
  }
  async function importIssue() {
    if (!issueUrl.trim()) return;
    const result = await run({
      command: 'import_issue',
      input: { url: issueUrl },
    });
    if (result) {
      issueUrl = '';
      navigate('Inbox');
    }
  }
  async function saveFilter() {
    const name = filterDraft.name.trim();
    if (!name) return;
    const result = await run({
      command: 'save_filter',
      input: {
        name,
        view: filterDraft.view,
        query: filterDraft.query.trim(),
      },
    });
    if (result) filterDraft = { ...filterDraft, name: '', query: '' };
  }
  async function applyFilter(filter: SavedFilter) {
    const targets = ['Today', 'Inbox', 'Planner', 'Completed'];
    view = targets.includes(filter.view) ? filter.view : 'Inbox';
    search = filter.query;
    selected = 0;
    palette = false;
    suggestions = null;
    await refresh().catch((e) => (error = String(e)));
  }
  async function removeFilter(id: string) {
    await run({ command: 'delete_filter', input: { id } });
  }
  function daysUntil(day: string) {
    if (!snapshot) return 0;
    return Math.round(
      (new Date(`${day}T12:00:00Z`).getTime() -
        new Date(`${snapshot.today}T12:00:00Z`).getTime()) /
        86_400_000,
    );
  }
  function subjectProgress(id: string) {
    const items = (snapshot?.syllabus_items ?? []).filter(
      (item) => item.subject_id === id,
    );
    return {
      done: items.filter((item) => item.done).length,
      total: items.length,
    };
  }
  async function addDsaProblem() {
    if (!dsaDraft.title.trim() || !dsaDraft.topic.trim()) return;
    const result = await run({
      command: 'create_dsa_problem',
      input: {
        title: dsaDraft.title,
        url: dsaDraft.url.trim() || null,
        topic: dsaDraft.topic,
        difficulty: dsaDraft.difficulty,
        platform: dsaDraft.platform.trim() || null,
        notes: '',
      },
    });
    if (result)
      dsaDraft = {
        title: '',
        topic: '',
        difficulty: 'medium',
        url: '',
        platform: '',
      };
  }
  async function logDsaProblem(problem: DsaProblem) {
    const command =
      problem.status === 'solved' ? 'review_dsa_problem' : 'solve_dsa_problem';
    await run({ command, input: { id: problem.id, confidence: dsaConfidence } });
  }
  async function removeDsaProblem(id: string) {
    await run({ command: 'delete_dsa_problem', input: { id } });
  }
  async function addSubject() {
    if (!subjectDraft.trim()) return;
    const result = await run({
      command: 'create_subject',
      input: { name: subjectDraft },
    });
    if (result?.type === 'subject') {
      selectedSubjectId = result.data.id;
      subjectDraft = '';
    }
  }
  async function removeSubject(id: string) {
    const result = await run({ command: 'delete_subject', input: { id } });
    if (result) selectedSubjectId = null;
  }
  async function addSyllabusItem() {
    if (!selectedSubjectId || !syllabusDraft.trim()) return;
    const result = await run({
      command: 'add_syllabus_item',
      input: { subject_id: selectedSubjectId, title: syllabusDraft },
    });
    if (result) syllabusDraft = '';
  }
  async function toggleSyllabusItem(item: SyllabusItem) {
    await run({
      command: 'set_syllabus_item_done',
      input: { id: item.id, done: !item.done },
    });
  }
  async function removeSyllabusItem(id: string) {
    await run({ command: 'delete_syllabus_item', input: { id } });
  }
  async function addDeadline() {
    if (!deadlineDraft.title.trim() || !deadlineDraft.due_date) return;
    const result = await run({
      command: 'create_deadline',
      input: {
        subject_id: selectedSubjectId,
        title: deadlineDraft.title,
        due_date: deadlineDraft.due_date,
        study_min: Number(deadlineDraft.study_min) || 60,
      },
    });
    if (result) {
      notice = 'Study tasks scheduled before the deadline.';
      deadlineDraft = { title: '', due_date: '', study_min: 60 };
    }
  }
  async function removeDeadline(id: string) {
    await run({ command: 'delete_deadline', input: { id } });
  }
  function focusTaskLabel(session: FocusSession) {
    return (
      snapshot?.tasks.find((task) => task.id === session.task_id)?.title ??
      'No task'
    );
  }
  function streakFor(habitId: string) {
    return (
      habitStreaks.find((streak) => streak.habit_id === habitId) ?? {
        current: 0,
        best: 0,
        last_done: null,
        due: false,
      }
    );
  }
  async function startFocus() {
    const result = await run({
      command: 'start_focus',
      input: {
        task_id: focusDraft.task_id || null,
        planned_min: Number(focusDraft.planned_min) || 25,
        kind: focusDraft.kind,
      },
    });
    if (result) focusDraft = { ...focusDraft, task_id: '' };
  }
  async function stopFocus(id: string) {
    await run({ command: 'stop_focus', input: { id } });
  }
  async function addHabit() {
    if (!habitDraft.name.trim()) return;
    const result = await run({
      command: 'create_habit',
      input: {
        name: habitDraft.name,
        cadence_days: Number(habitDraft.cadence_days) || 1,
      },
    });
    if (result) habitDraft = { name: '', cadence_days: 1 };
  }
  async function logHabit(habit: Habit) {
    await run({ command: 'log_habit', input: { habit_id: habit.id, day: null } });
  }
  async function removeHabit(id: string) {
    await run({ command: 'delete_habit', input: { id } });
  }
  async function saveReview() {
    const result = await run({
      command: 'save_daily_review',
      input: { day: snapshot?.today ?? null, note: reviewNote },
    });
    if (result) {
      reviewSynced = true;
      notice = 'Review saved.';
    }
  }
  async function dismissOnboarding() {
    if (!snapshot) return;
    await run({
      command: 'save_settings',
      input: { ...snapshot.settings, onboarded: true },
    });
  }
  async function exportBackup(kind: 'json' | 'markdown') {
    try {
      const path = await exportData(kind);
      dataNotice = `Exported to ${path}`;
    } catch (e) {
      error = String(e);
    }
  }
  async function importBackup() {
    if (!importPath.trim()) return;
    try {
      const summary = await importData(importPath.trim());
      dataNotice = `Imported ${summary.tasks} tasks, ${summary.projects} projects, ${summary.habits} habits; skipped ${summary.skipped} existing records.`;
      reviewSynced = false;
      await refresh();
    } catch (e) {
      error = String(e);
    }
  }
  async function openProject(path: string | null) {
    if (!path) return;
    try {
      await openInVscode(path);
    } catch (e) {
      error = String(e);
    }
  }
  async function saveAsTemplate() {
    if (!edit) return;
    const result = await run({
      command: 'create_template',
      input: {
        name: edit.title,
        title: edit.title,
        notes: edit.notes,
        priority: edit.priority,
        energy: edit.energy,
        est_min: edit.est_min,
        tags: editTags.split(/\s+/).filter(Boolean),
        project_id: edit.project_id || null,
      },
    });
    if (result) notice = 'Template saved. Find it in Projects → Templates.';
  }
  function keyboard(event: KeyboardEvent) {
    if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 'k') {
      event.preventDefault();
      palette = !palette;
      return;
    }
    if (event.key === 'Escape') {
      palette = false;
      edit = null;
      preferences = null;
      if (capture && native) void getCurrentWindow().hide();
      return;
    }
    if (
      event.target instanceof HTMLElement &&
      (['INPUT', 'TEXTAREA', 'SELECT', 'BUTTON'].includes(
        event.target.tagName,
      ) ||
        event.target.isContentEditable)
    )
      return;
    if (edit || preferences || palette) return;
    if (event.key === '/') {
      event.preventDefault();
      searchInput?.focus();
    }
    if (event.key === 'n') {
      event.preventDefault();
      quickInput?.focus();
    }
    if (event.key === 'j') selected = Math.min(tasks.length - 1, selected + 1);
    if (event.key === 'k') selected = Math.max(0, selected - 1);
    const task = tasks[selected];
    if (task && event.key === 'x')
      void run({
        command: 'complete_task',
        input: { id: task.id, completed: task.status !== 'done' },
      });
    if (task && event.key === 'e') openEdit(task);
  }
  function drop(event: DragEvent, hour: number) {
    event.preventDefault();
    const id = event.dataTransfer?.getData('text/plain');
    if (id)
      void schedule(id, `${day}T${String(hour).padStart(2, '0')}:00`, day);
  }
  function focusDialog(node: HTMLElement) {
    const previous = document.activeElement as HTMLElement | null;
    const focusables = () =>
      Array.from(
        node.querySelectorAll<HTMLElement>(
          'button:not(:disabled), input:not(:disabled), textarea, select, [tabindex="0"]',
        ),
      );
    queueMicrotask(() => focusables()[0]?.focus());
    function trap(event: KeyboardEvent) {
      if (event.key !== 'Tab') return;
      const items = focusables();
      const first = items[0];
      const last = items.at(-1);
      if (event.shiftKey && document.activeElement === first) {
        event.preventDefault();
        last?.focus();
      } else if (!event.shiftKey && document.activeElement === last) {
        event.preventDefault();
        first?.focus();
      }
    }
    node.addEventListener('keydown', trap);
    return {
      destroy() {
        node.removeEventListener('keydown', trap);
        previous?.focus();
      },
    };
  }
  function labelDay(value: string, weekday = false) {
    return new Date(`${value}T12:00:00`).toLocaleDateString(
      undefined,
      weekday
        ? { weekday: 'short', day: 'numeric' }
        : { weekday: 'long', month: 'long', day: 'numeric' },
    );
  }
  function timeLabel(value: string) {
    return new Date(value).toLocaleTimeString(undefined, {
      timeZone: snapshot?.settings.timezone,
      hour: '2-digit',
      minute: '2-digit',
    });
  }
  $effect(() => {
    if (!activeFocus) {
      focusElapsedMinutes = 0;
      return;
    }
    const update = () => {
      const end = activeFocus!.ended_at
        ? new Date(activeFocus!.ended_at).getTime()
        : Date.now();
      focusElapsedMinutes = Math.max(
        0,
        Math.floor((end - new Date(activeFocus!.started_at).getTime()) / 60000),
      );
    };
    update();
    const timer = setInterval(update, 1000);
    return () => clearInterval(timer);
  });
  $effect(() => {
    if (!reviewSynced && todayReview) {
      reviewNote = todayReview.note;
      reviewSynced = true;
    }
  });
  onMount(() => {
    if (!native) return;
    const unlisteners: (() => void)[] = [];
    let disposed = false;
    async function setup() {
      for (const name of ['dayboard:changed', 'dayboard:shown']) {
        const stop = await listen(name, () => {
          if (!document.hidden)
            void refresh().catch((e) => (error = String(e)));
          if (capture) quickInput?.focus();
        });
        if (disposed) stop();
        else unlisteners.push(stop);
      }
      const stop = await listen<string>(
        'dayboard:error',
        (e) => (error = e.payload),
      );
      if (disposed) stop();
      else unlisteners.push(stop);
      await refresh();
      if (capture) quickInput?.focus();
    }
    void setup().catch((e) => (error = String(e)));
    return () => {
      disposed = true;
      unlisteners.forEach((stop) => stop());
    };
  });
</script>

<svelte:window onkeydown={keyboard} />

{#snippet captureForm()}
  <form
    class="capture-form"
    onsubmit={(e) => {
      e.preventDefault();
      void create();
    }}
  >
    <span class="plus" aria-hidden="true">+</span>
    <input
      bind:this={quickInput}
      bind:value={text}
      aria-label="Quick add task"
      placeholder="What’s on your mind?"
      autocomplete="off"
      disabled={busy}
    />
    <button
      class="primary"
      disabled={busy || !native || !text.trim()}
      type="submit">Add task <span aria-hidden="true">↵</span></button
    >
  </form>
  <div class="capture-help">
    Try <code>Read a chapter #college p2 30m today 7pm</code>
  </div>
  {#if tagMatches}<div class="tag-options">
      {#each (snapshot?.tags ?? [])
        .filter((t) => t.startsWith(tagMatches))
        .slice(0, 6) as tag (tag)}<button
          onclick={() => {
            text = text.replace(/[#@!][^\s]*$/, tag + ' ');
            quickInput?.focus();
          }}>{tag}</button
        >{/each}
    </div>{/if}
{/snippet}

{#if capture}
  <main class="quick-window">
    <header>
      <strong>◈ Dayboard <span> / Quick capture</span></strong><button
        onclick={() => getCurrentWindow().hide()}
        aria-label="Close capture">Esc</button
      >
    </header>
    {@render captureForm()}{#if error}<p class="error" role="alert">
        {error}
      </p>{/if}
  </main>
{:else}
  <div class="app-shell">
    <aside class="sidebar">
      <a class="brand" href="#today" onclick={() => navigate('Today')}
        ><span class="brand-mark">◈</span> dayboard<span class="version"
          >β</span
        ></a
      >
      <div class="workspace-label">YOUR SPACE <span>LOCAL</span></div>
      <nav aria-label="Main navigation">
        {#each views as item, index (item)}<button
            class:active={view === item}
            onclick={() => navigate(item)}
            ><span class="nav-icon">{['☀', '▣', '▦', '◧', '◇', '◔', '✓'][index]}</span
            >{item}{#if item === 'Inbox'}<span class="count"
                >{snapshot?.tasks.filter(
                  (t) =>
                    t.planned_day === null &&
                    t.status !== 'done' &&
                    t.status !== 'dropped',
                ).length ?? 0}</span
              >{/if}</button
          >{/each}
      </nav>
      <div class="sidebar-note">
        <span class="tiny-line"></span>
        <p>A little less noise.<br />A little more done.</p>
      </div>
      <div class="sidebar-bottom">
        <button
          onclick={() => {
            if (snapshot) preferences = { ...snapshot.settings };
          }}>⚙ <span>Preferences</span></button
        >
        <div class="local-status"><span></span> On your device. Yours.</div>
      </div>
    </aside>
    <main class="main-panel">
      <header class="topbar">
        <div class="breadcrumb">
          Your workspace <span>/</span> <strong>{view}</strong>
        </div>
        <button class="palette-trigger" onclick={() => (palette = true)}
          >Find a command <kbd>⌘ / Ctrl K</kbd></button
        >
      </header>
      <div class="page-content">
        {#if !native}<div class="notice">
            <strong>Desktop app required</strong>
            <p>
              Run <code>npm run desktop</code> to use your local SQLite database.
              This browser view does not save tasks.
            </p>
          </div>{/if}
        <section class="page-heading">
          <div>
            <div class="eyebrow">
              {snapshot
                ? labelDay(snapshot.today).toUpperCase()
                : 'A QUIETER WAY TO PLAN'}
            </div>
            <h1>
              {view === 'Today'
                ? 'Make room for what matters.'
                : view === 'Inbox'
                  ? 'Out of your head. Into here.'
                  : view === 'Planner'
                    ? 'Give your day some shape.'
                    : view === 'Projects'
                      ? 'Build something over time.'
                    : view === 'Study'
                      ? 'Practice deliberately.'
                      : view === 'Insights'
                        ? 'Notice what is working.'
                        : 'Look at what you’ve done.'}
            </h1>
            <p>
              {view === 'Today'
                ? 'Choose a few good things. Let the rest wait.'
                : view === 'Inbox'
                  ? 'Capture now. Find a place for it when you’re ready.'
                  : view === 'Planner'
                    ? 'Drag a task to a time slot, or schedule it from the task menu.'
                    : view === 'Projects'
                      ? 'Projects, goal progress, templates and saved filters.'
                      : view === 'Study'
                        ? 'Track problems, review them on time, and stay ahead of deadlines.'
                        : view === 'Insights'
                          ? 'Focus sessions, habits, a short review and your data.'
                          : 'Small steps add up. Reopen a task whenever you need to.'}
            </p>
          </div>
          <div class="day-number">
            {snapshot?.today.slice(-2) ?? '—'}<span
              >{view === 'Planner'
                ? 'PLAN AHEAD'
                : view === 'Projects'
                  ? 'THE LONG VIEW'
                  : view === 'Study'
                    ? 'KEEP SHARP'
                    : view === 'Insights'
                      ? 'LOOK BACK'
                      : 'ONE DAY AT A TIME'}</span
            >
          </div>
        </section>
        {#if error}<div class="error" role="alert">
            {error}<button
              onclick={() => (error = '')}
              aria-label="Dismiss error">×</button
            >
          </div>{/if}
        {#if notice}<div class="notice" role="status">
            {notice}<button
              onclick={() => (notice = '')}
              aria-label="Dismiss notice">×</button
            >
          </div>{/if}
        {#if deleted}<div class="notice" role="status">
            “{deleted.title}” moved to trash.
            <button
              onclick={async () => {
                const r = await run({
                  command: 'restore_task',
                  input: { id: deleted!.id },
                });
                if (r) deleted = null;
              }}>Restore</button
            >
          </div>{/if}
        {#if view === 'Today' && snapshot && !snapshot.settings.onboarded}
          <section class="onboarding" aria-label="Welcome to Daybridge">
            <div>
              <h2>Welcome to Daybridge</h2>
              <p>Everything stays on this device. The short version:</p>
              <ol>
                <li>Type a task above and press Enter.</li>
                <li>
                  Add <code>#area</code>, <code>p1</code>, <code>45m</code> or
                  <code>today 7pm</code>.
                </li>
                <li>Set your timezone and caps in Preferences.</li>
              </ol>
            </div>
            <button class="primary" disabled={busy} onclick={dismissOnboarding}
              >Got it</button
            >
          </section>
        {/if}
        <section class="capture-card" aria-label="Capture a task">
          {@render captureForm()}
        </section>
        {#if view === 'Today' && snapshot}<section class="capacity-card">
            <div>
              <span class="eyebrow">TODAY’S CAPACITY</span>
              <p>
                <strong
                  >{snapshot.capacity.minutes}<small>
                    / {snapshot.capacity.cap_minutes} min</small
                  ></strong
                ><span
                  >{snapshot.capacity.available_minutes} min free · {snapshot
                    .capacity.schedule_minutes} min scheduled · {snapshot
                    .capacity.count}
                  of {snapshot.settings.max_tasks} tasks</span
                >
              </p>
              <progress
                value={snapshot.capacity.minutes}
                max={Math.max(1, snapshot.capacity.cap_minutes)}
                aria-label="Planned daily minutes"
              ></progress>
            </div>
            <div class="capacity-message">
              {#if snapshot.day_status.status === 'blocked'}<strong class="warning"
                  >Blocked day.</strong
                ><span
                  >{snapshot.day_status.reasons[0]?.message ??
                    'Classes and events claim this day.'}</span
                >{:else if snapshot.capacity.over_count || snapshot.capacity.over_minutes}<strong
                  class="warning">A full plate.</strong
                ><span>Move something to another day.</span>{:else}<strong
                  >Leave a little breathing room.</strong
                ><span>Your plan is a guide, not a race.</span>{/if}
            </div>
            <button class="subtle" disabled={busy} onclick={plan}
              >Suggest a plan ↗</button
            >
          </section>{/if}
        {#if view === 'Today' && snapshot && snapshot.today_occurrences.length}<section
            class="suggestions"
            aria-label="Today's schedule"
          >
            <div class="section-title">
              <h2>Today’s schedule</h2>
              <span
                >{snapshot.active_term?.name ?? 'No active term'} · {snapshot
                  .day_status.status}</span
              >
            </div>
            {#each snapshot.today_occurrences as block (block.item_id + block.start_utc)}<div>
                <span
                  >{block.title}
                  <small
                    >{timeLabel(block.start_utc)}–{timeLabel(block.end_utc)}{block.location
                      ? ` · ${block.location}`
                      : ''}{block.is_moved ? ' · moved' : ''}</small
                  ></span
                >
              </div>{/each}
          </section>{/if}
        {#if suggestions}<section class="suggestions">
            <div class="section-title">
              <h2>Suggested for today</h2>
              <button onclick={() => (suggestions = null)}>Close</button>
            </div>
            {#if !suggestions.length}<p>
                No tasks fit your current capacity.
              </p>{/if}{#each suggestions as task (task.id)}<div>
                <span>{task.title} <small>{task.est_min}m</small></span><button
                  disabled={busy}
                  onclick={() => schedule(task.id, null, snapshot!.today)}
                  >Add to today</button
                >
              </div>{/each}
          </section>{/if}
        {#if view === 'Planner'}<div
            class="week-strip"
            aria-label="Choose planner day"
          >
            {#each days as d (d)}<button
                class:chosen={day === d}
                onclick={() => {
                  day = d;
                  selected = 0;
                }}
                >{labelDay(d, true)}<span
                  >{snapshot?.tasks.filter(
                    (t) =>
                      t.planned_day === d &&
                      t.status !== 'done' &&
                      t.status !== 'dropped',
                  ).length ?? 0} tasks</span
                ></button
              >{/each}
          </div>{/if}
        {#if view === 'Projects'}
          <section
            class="projects-view"
            aria-label="Projects, goals, templates and filters"
          >
            <div class="panel">
              <div class="section-title">
                <h2>
                  Projects <span>{snapshot?.projects.length ?? 0}</span>
                </h2>
              </div>
              <form
                class="project-form"
                onsubmit={(event) => {
                  event.preventDefault();
                  void saveProject();
                }}
              >
                <input
                  bind:value={projectDraft.name}
                  aria-label="Project name"
                  placeholder="New project"
                />
                <select bind:value={projectDraft.kind} aria-label="Project kind">
                  <option value="coding">Coding</option>
                  <option value="college">College</option>
                  <option value="personal">Personal</option>
                </select>
                <input
                  bind:value={projectDraft.repo_path}
                  aria-label="Repository path"
                  placeholder="Repo path (optional)"
                />
                <input
                  bind:value={projectDraft.branch}
                  aria-label="Branch"
                  placeholder="Branch"
                />
                <button
                  class="primary"
                  type="submit"
                  disabled={busy || !native || !projectDraft.name.trim()}
                  >{projectEditId ? 'Save project' : 'Add project'}</button
                >{#if projectEditId}<button
                    class="subtle"
                    type="button"
                    onclick={cancelProjectEdit}>Cancel</button
                  >{/if}
              </form>
              <div class="project-grid">
                {#each snapshot?.projects ?? [] as project (project.id)}
                  <button
                    class="project-card"
                    class:chosen={projectId === project.id}
                    onclick={() => (projectId = project.id)}
                  >
                    <strong>{project.name}</strong>
                    <span>{project.kind} · {project.status}</span>
                    {#if project.repo_path}<small
                        >{project.repo_path}{project.branch
                          ? ` · ${project.branch}`
                          : ''}</small
                      >{/if}
                  </button>
                {/each}
              </div>
            </div>
            {#if selectedProject}
              <div class="board-toolbar">
                <h2>{selectedProject.name}</h2>
                <div>
                  {#if selectedProject.repo_path}<button
                      class="subtle"
                      disabled={busy}
                      onclick={() => openProject(selectedProject!.repo_path)}
                      >Open in VS Code</button
                    >{/if}<button
                    class="subtle"
                    disabled={busy}
                    onclick={() => editProject(selectedProject!)}
                    >Edit</button
                  ><button
                    class="danger"
                    disabled={busy}
                    onclick={() => removeProject(selectedProject!.id)}
                    >Delete project</button
                  >
                </div>
              </div>
              <div class="board">
                {#each boardColumns as column (column.label)}
                  <div class="board-column">
                    <h4>{column.label}</h4>
                    {#each projectTasks.filter((task) =>
                      column.statuses.includes(task.status),
                    ) as task (task.id)}
                      <button class="board-task" onclick={() => openEdit(task)}
                        >{task.title}<small>{task.est_min}m</small></button
                      >
                    {/each}
                  </div>
                {/each}
              </div>
            {/if}
            <div class="panel">
              <div class="section-title"><h2>Import a GitHub issue</h2></div>
              <form
                onsubmit={(event) => {
                  event.preventDefault();
                  void importIssue();
                }}
              >
                <input
                  bind:value={issueUrl}
                  aria-label="GitHub issue URL"
                  placeholder="https://github.com/owner/repo/issues/12"
                />
                <button
                  type="submit"
                  disabled={busy || !native || !issueUrl.trim()}
                  >Import issue</button
                >
              </form>
              <p class="muted">
                Parsed locally; no network request. A matching repo path links
                the task to its project.
              </p>
            </div>
            <div class="panel">
              <div class="section-title">
                <h2>Goals <span>{snapshot?.goals.length ?? 0}</span></h2>
              </div>
              <form
                onsubmit={(event) => {
                  event.preventDefault();
                  void saveGoal();
                }}
              >
                <input
                  bind:value={goalDraft.title}
                  aria-label="Goal title"
                  placeholder="New goal"
                />
                <select bind:value={goalDraft.horizon} aria-label="Goal horizon">
                  <option value="medium">Medium term</option>
                  <option value="long">Long term</option>
                </select>
                <input
                  type="date"
                  bind:value={goalDraft.target_date}
                  aria-label="Goal target date"
                />
                <button
                  type="submit"
                  disabled={busy || !native || !goalDraft.title.trim()}
                  >{goalEditId ? 'Save goal' : 'Add goal'}</button
                >{#if goalEditId}<button
                    class="subtle"
                    type="button"
                    onclick={cancelGoalEdit}>Cancel</button
                  >{/if}
              </form>
              {#each snapshot?.goals ?? [] as goal (goal.id)}
                {@const progress = progressFor(goal.id)}
                <div class="goal-row">
                  <div>
                    <strong>{goal.title}</strong>
                    <span
                      >{goal.horizon}{goal.target_date
                        ? ` · ${goal.target_date}`
                        : ''}</span
                    >
                  </div>
                  <progress
                    value={progress.done}
                    max={progress.total || 1}
                  ></progress>
                  <span>{progress.done}/{progress.total}</span>
                  <button
                    class="subtle"
                    disabled={busy}
                    onclick={() => editGoal(goal)}
                    aria-label={`Edit goal ${goal.title}`}>Edit</button
                  ><button
                    class="danger"
                    disabled={busy}
                    onclick={() => removeGoal(goal.id)}
                    aria-label={`Delete goal ${goal.title}`}>×</button
                  >
                </div>
              {/each}
            </div>
            <div class="panel">
              <div class="section-title">
                <h2>
                  Templates <span>{snapshot?.templates.length ?? 0}</span>
                </h2>
              </div>
              {#if !snapshot?.templates.length}<p class="muted">
                  Open a task and choose “Save as template”.
                </p>{/if}
              {#each snapshot?.templates ?? [] as template (template.id)}
                <div class="goal-row">
                  <div>
                    <strong>{template.name}</strong>
                    <span>{template.title} · {template.est_min}m</span>
                  </div>
                  <button
                    disabled={busy}
                    onclick={() => useTemplate(template.id)}
                    >Create task</button
                  ><button
                    class="danger"
                    disabled={busy}
                    onclick={() => removeTemplate(template.id)}
                    aria-label={`Delete template ${template.name}`}>×</button
                  >
                </div>
              {/each}
            </div>
            <div class="panel">
              <div class="section-title">
                <h2>
                  Saved filters <span
                    >{snapshot?.saved_filters.length ?? 0}</span
                  >
                </h2>
              </div>
              <form
                onsubmit={(event) => {
                  event.preventDefault();
                  void saveFilter();
                }}
              >
                <input
                  bind:value={filterDraft.name}
                  aria-label="Filter name"
                  placeholder="Filter name"
                />
                <select bind:value={filterDraft.view} aria-label="Filter view">
                  {#each ['Today', 'Inbox', 'Planner', 'Completed'] as option (option)}<option
                      value={option}>{option}</option
                    >{/each}
                </select>
                <input
                  bind:value={filterDraft.query}
                  aria-label="Filter search"
                  placeholder="Search text (optional)"
                />
                <button
                  type="submit"
                  disabled={busy || !native || !filterDraft.name.trim()}
                  >Save filter</button
                >
              </form>
              <p class="muted">
                Opens <strong>{filterDraft.view}</strong>{filterDraft.query
                  ? ` with search “${filterDraft.query}”`
                  : ' with no search'}.
              </p>
              {#each snapshot?.saved_filters ?? [] as filter (filter.id)}
                <div class="goal-row">
                  <div>
                    <strong>{filter.name}</strong>
                    <span
                      >{filter.view}{filter.query
                        ? ` · ${filter.query}`
                        : ''}</span
                    >
                  </div>
                  <button disabled={busy} onclick={() => applyFilter(filter)}
                    >Apply</button
                  ><button
                    class="danger"
                    disabled={busy}
                    onclick={() => removeFilter(filter.id)}
                    aria-label={`Delete filter ${filter.name}`}>×</button
                  >
                </div>
              {/each}
            </div>
          </section>
        {:else if view === 'Study'}
          <section class="study-view" aria-label="DSA practice and college prep">
            <div class="panel">
              <div class="section-title">
                <h2>DSA problems <span>{dsaProblems.length}</span></h2>
                <label class="inline"
                  >Review confidence
                  <select bind:value={dsaConfidence} aria-label="Review confidence">
                    {#each [1, 2, 3, 4, 5] as value (value)}<option value={value}
                        >{value}</option
                      >{/each}
                  </select></label
                >
              </div>
              <form
                onsubmit={(event) => {
                  event.preventDefault();
                  void addDsaProblem();
                }}
              >
                <input
                  bind:value={dsaDraft.title}
                  aria-label="DSA problem title"
                  placeholder="Problem title"
                />
                <input
                  bind:value={dsaDraft.topic}
                  aria-label="DSA topic"
                  placeholder="Topic (arrays, graphs…)"
                />
                <select bind:value={dsaDraft.difficulty} aria-label="DSA difficulty">
                  <option value="easy">Easy</option>
                  <option value="medium">Medium</option>
                  <option value="hard">Hard</option>
                </select>
                <input
                  bind:value={dsaDraft.url}
                  aria-label="Problem URL"
                  placeholder="Problem URL (optional)"
                />
                <input
                  bind:value={dsaDraft.platform}
                  aria-label="Problem platform"
                  placeholder="Platform"
                />
                <button
                  class="primary"
                  type="submit"
                  disabled={busy ||
                    !native ||
                    !dsaDraft.title.trim() ||
                    !dsaDraft.topic.trim()}>Add problem</button
                >
              </form>
              {#if !dsaProblems.length}<p class="muted">
                  Add a problem, then log it as solved to schedule its first
                  revision.
                </p>{/if}
              {#each dsaProblems as problem (problem.id)}
                <div class="goal-row">
                  <div>
                    <strong>{problem.title}</strong>
                    <span
                      >{problem.topic} · {problem.difficulty} · {problem.status ===
                      'solved'
                        ? `stage ${problem.review_stage} · next ${problem.next_review_at ?? '—'} · confidence ${problem.confidence}`
                        : 'not solved'}{problem.url
                        ? ` · ${problem.url}`
                        : ''}</span
                    >
                  </div>
                  <button
                    disabled={busy}
                    onclick={() => logDsaProblem(problem)}
                    >{problem.status === 'solved' ? 'Review' : 'Log solved'}</button
                  ><button
                    class="danger"
                    disabled={busy}
                    onclick={() => removeDsaProblem(problem.id)}
                    aria-label={`Delete problem ${problem.title}`}>×</button
                  >
                </div>
              {/each}
            </div>
            <div class="panel">
              <div class="section-title">
                <h2>Topic coverage <span>{dsaTopics.length}</span></h2>
              </div>
              {#if !dsaTopics.length}<p class="muted">
                  Solved topics will appear here.
                </p>{/if}
              <div class="topic-grid">
                {#each dsaTopics as topic (topic.topic)}
                  <div class="topic-card">
                    <strong>{topic.topic}</strong>
                    <span
                      >{topic.solved}/{topic.total} solved · avg {topic.average_confidence}</span
                    >
                    <progress value={topic.solved} max={topic.total || 1}
                    ></progress>
                  </div>
                {/each}
              </div>
            </div>
            <div class="panel">
              <div class="section-title">
                <h2>Subjects <span>{subjects.length}</span></h2>
              </div>
              <form
                onsubmit={(event) => {
                  event.preventDefault();
                  void addSubject();
                }}
              >
                <input
                  bind:value={subjectDraft}
                  aria-label="Subject name"
                  placeholder="New subject"
                />
                <button
                  type="submit"
                  disabled={busy || !native || !subjectDraft.trim()}
                  >Add subject</button
                >
              </form>
              <div class="project-grid">
                {#each subjects as subject (subject.id)}
                  {@const progress = subjectProgress(subject.id)}
                  <button
                    class="project-card"
                    class:chosen={selectedSubjectId === subject.id}
                    onclick={() => (selectedSubjectId = subject.id)}
                  >
                    <strong>{subject.name}</strong>
                    <span>{progress.done}/{progress.total} syllabus items</span>
                    <progress value={progress.done} max={progress.total || 1}
                    ></progress>
                  </button>
                {/each}
              </div>
            </div>
            {#if selectedSubject}
              <div class="panel">
                <div class="section-title">
                  <h2>{selectedSubject.name}</h2>
                  <button
                    class="danger"
                    disabled={busy}
                    onclick={() => removeSubject(selectedSubject!.id)}
                    >Delete subject</button
                  >
                </div>
                <form
                  onsubmit={(event) => {
                    event.preventDefault();
                    void addSyllabusItem();
                  }}
                >
                  <input
                    bind:value={syllabusDraft}
                    aria-label="Syllabus item"
                    placeholder="Syllabus topic"
                  />
                  <button
                    type="submit"
                    disabled={busy || !syllabusDraft.trim()}>Add item</button
                  >
                </form>
                {#each subjectSyllabus as item (item.id)}
                  <div class="syllabus-row">
                    <input
                      type="checkbox"
                      checked={item.done}
                      onchange={() => toggleSyllabusItem(item)}
                      aria-label={`Toggle ${item.title}`}
                    />
                    <span class:done={item.done}>{item.title}</span>
                    <button
                      class="danger"
                      type="button"
                      disabled={busy}
                      onclick={() => removeSyllabusItem(item.id)}
                      aria-label={`Delete ${item.title}`}>×</button
                    >
                  </div>
                {/each}
                <div class="section-title">
                  <h2>
                    Deadlines <span>{subjectDeadlines.length}</span>
                  </h2>
                </div>
                <form
                  onsubmit={(event) => {
                    event.preventDefault();
                    void addDeadline();
                  }}
                >
                  <input
                    bind:value={deadlineDraft.title}
                    aria-label="Deadline title"
                    placeholder="Midterm, submission…"
                  />
                  <input
                    type="date"
                    bind:value={deadlineDraft.due_date}
                    aria-label="Deadline date"
                  />
                  <input
                    type="number"
                    min="1"
                    max="1440"
                    bind:value={deadlineDraft.study_min}
                    aria-label="Study minutes"
                  />
                  <button
                    type="submit"
                    disabled={busy ||
                      !deadlineDraft.title.trim() ||
                      !deadlineDraft.due_date}>Add deadline</button
                  >
                </form>
                {#each subjectDeadlines as deadline (deadline.id)}
                  <div class="goal-row">
                    <div>
                      <strong>{deadline.title}</strong>
                      <span
                        >{deadline.due_date} · {Math.max(
                          0,
                          daysUntil(deadline.due_date),
                        )} days · {deadline.study_min} min planned</span
                      >
                    </div>
                    <button
                      class="danger"
                      disabled={busy}
                      onclick={() => removeDeadline(deadline.id)}
                      aria-label={`Delete deadline ${deadline.title}`}>×</button
                    >
                  </div>
                {/each}
              </div>
            {/if}
          </section>
        {:else if view === 'Insights'}
          <section
            class="insights-view"
            aria-label="Focus, habits, stats, review and data"
          >
            <div class="panel">
              <div class="section-title"><h2>Focus</h2></div>
              {#if activeFocus}
                <div class="focus-active">
                  <div>
                    <strong
                      >{activeFocus.kind === 'focus'
                        ? 'Focus session'
                        : 'Break'}</strong
                    >
                    <span
                      >{focusTaskLabel(activeFocus)} · {focusElapsedMinutes} /
                      {activeFocus.planned_min} min</span
                    >
                  </div>
                  <button
                    class="primary"
                    disabled={busy}
                    onclick={() => stopFocus(activeFocus!.id)}>Stop</button
                  >
                </div>
              {:else}
                <form
                  onsubmit={(event) => {
                    event.preventDefault();
                    void startFocus();
                  }}
                >
                  <select bind:value={focusDraft.task_id} aria-label="Focus task">
                    <option value="">No task</option>
                    {#each (snapshot?.tasks ?? []).filter((task) => task.status !== 'done' && task.status !== 'dropped') as task (task.id)}<option
                        value={task.id}>{task.title}</option
                      >{/each}
                  </select>
                  <input
                    type="number"
                    min="1"
                    max="1440"
                    bind:value={focusDraft.planned_min}
                    aria-label="Focus minutes"
                  />
                  <select bind:value={focusDraft.kind} aria-label="Focus kind">
                    <option value="focus">Focus</option>
                    <option value="break">Break</option>
                  </select>
                  <button
                    class="primary"
                    type="submit"
                    disabled={busy || !native}>Start focus</button
                  >
                </form>
              {/if}
              <p class="muted">
                Stopping a focus session adds its minutes to the linked task.
              </p>
            </div>
            <div class="panel">
              <div class="section-title">
                <h2>Habits <span>{habits.length}</span></h2>
              </div>
              <form
                onsubmit={(event) => {
                  event.preventDefault();
                  void addHabit();
                }}
              >
                <input
                  bind:value={habitDraft.name}
                  aria-label="Habit name"
                  placeholder="New habit"
                />
                <input
                  type="number"
                  min="1"
                  max="30"
                  bind:value={habitDraft.cadence_days}
                  aria-label="Habit cadence"
                />
                <button
                  type="submit"
                  disabled={busy || !native || !habitDraft.name.trim()}
                  >Add habit</button
                >
              </form>
              {#each habits as habit (habit.id)}
                {@const streak = streakFor(habit.id)}
                <div class="goal-row">
                  <div>
                    <strong>{habit.name}</strong>
                    <span
                      >every {habit.cadence_days} day(s) · streak {streak.current}
                      · best {streak.best}{streak.last_done
                        ? ` · last ${streak.last_done}`
                        : ''}</span
                    >
                  </div>
                  <button
                    disabled={busy || !streak.due}
                    onclick={() => logHabit(habit)}
                    >{streak.due ? 'Log today' : 'Done'}</button
                  ><button
                    class="danger"
                    disabled={busy}
                    onclick={() => removeHabit(habit.id)}
                    aria-label={`Delete habit ${habit.name}`}>×</button
                  >
                </div>
              {/each}
            </div>
            <div class="panel">
              <div class="section-title"><h2>Last 7 days</h2></div>
              {#if weeklyStats}
                {@const stats = weeklyStats}
                <div class="stat-grid">
                  <div><span>Completed</span><strong>{stats.completed}</strong></div>
                  <div><span>Created</span><strong>{stats.created}</strong></div>
                  <div><span>Carried</span><strong>{stats.carried}</strong></div>
                  <div>
                    <span>Focus</span><strong>{stats.focus_minutes}m</strong>
                  </div>
                  <div>
                    <span>Estimate accuracy</span><strong
                      >{stats.measured
                        ? `${Math.round(stats.estimate_accuracy * 100)}%`
                        : '—'}</strong
                    >
                  </div>
                </div>
                {#each stats.tag_stats as tag (tag.tag)}
                  <div class="goal-row">
                    <div>
                      <strong>{tag.tag}</strong>
                      <span
                        >{tag.focus_minutes} focus min · {tag.completed} done</span
                      >
                    </div>
                    <progress
                      value={tag.focus_minutes}
                      max={Math.max(1, stats.focus_minutes)}
                    ></progress>
                  </div>
                {/each}
              {/if}
            </div>
            <div class="panel">
              <div class="section-title"><h2>End-of-day review</h2></div>
              <textarea
                bind:value={reviewNote}
                rows="3"
                aria-label="Daily review note"
                placeholder="One line about today…"
              ></textarea>
              <div class="data-actions">
                <button
                  class="primary"
                  disabled={busy || !native}
                  onclick={saveReview}>Save review</button
                >
              </div>
              {#if todayReview}<p class="muted">
                  Saved for {todayReview.day}.
                </p>{/if}
            </div>
            <div class="panel">
              <div class="section-title"><h2>Data</h2></div>
              <div class="data-actions">
                <button
                  disabled={busy || !native}
                  onclick={() => exportBackup('json')}>Export JSON</button
                ><button
                  disabled={busy || !native}
                  onclick={() => exportBackup('markdown')}
                  >Export Markdown</button
                >
              </div>
              <form
                onsubmit={(event) => {
                  event.preventDefault();
                  void importBackup();
                }}
              >
                <input
                  bind:value={importPath}
                  aria-label="Import file path"
                  placeholder="Path to a Dayboard JSON export"
                />
                <button
                  type="submit"
                  disabled={busy || !native || !importPath.trim()}
                  >Import JSON</button
                >
              </form>
              {#if dataNotice}<p class="muted">{dataNotice}</p>{/if}
              <p class="muted">
                Daily backups live in the app-data folder under <code>backups/</code>.
              </p>
            </div>
          </section>
        {:else}
          <div class:planner-layout={view === 'Planner'}>
          <section class="task-section">
            <div class="section-title">
              <h2>
                {search
                  ? 'Search results'
                  : view === 'Today'
                    ? 'Your focus today'
                    : view === 'Planner'
                      ? 'Day’s tasks'
                      : view} <span>{tasks.length}</span>
              </h2>
              <form
                onsubmit={(e) => {
                  e.preventDefault();
                  void refresh().catch((e) => (error = String(e)));
                }}
              >
                <input
                  bind:this={searchInput}
                  bind:value={search}
                  aria-label="Search all tasks"
                  placeholder="Search tasks  /"
                /><button type="submit" disabled={!native}>Find</button
                >{#if search}<button
                    type="button"
                    onclick={() => {
                      search = '';
                      void refresh().catch((e) => (error = String(e)));
                    }}>Clear</button
                  >{/if}
              </form>
            </div>
            {#if !tasks.length}<div class="empty-state">
                <div>✓</div>
                <h3>
                  {view === 'Completed'
                    ? 'Your progress starts here.'
                    : 'A little space to think.'}
                </h3>
                <p>
                  {view === 'Inbox'
                    ? 'Add a thought above. It’ll be here when you need it.'
                    : 'Capture a task above, or bring one over from your inbox.'}
                </p>
                {#if view === 'Today'}<button
                    class="subtle"
                    onclick={() => navigate('Inbox')}>Visit your inbox →</button
                  >{/if}
              </div>{:else}
              <div class="task-list" role="list" aria-label="Tasks">
                {#each visibleTasks as task (task.id)}<article
                    role="listitem"
                    class="task-row"
                    class:selected={tasks[selected]?.id === task.id}
                    class:done={task.status === 'done'}
                    draggable="true"
                    ondragstart={(event) =>
                      event.dataTransfer?.setData('text/plain', task.id)}
                  >
                    <button
                      class="check-button"
                      aria-label={task.status === 'done'
                        ? `Reopen ${task.title}`
                        : `Complete ${task.title}`}
                      disabled={busy}
                      onclick={() =>
                        run({
                          command: 'complete_task',
                          input: {
                            id: task.id,
                            completed: task.status !== 'done',
                          },
                        })}>{task.status === 'done' ? '✓' : ''}</button
                    >
                    <button
                      class="task-body"
                      onclick={() => {
                        selected = tasks.indexOf(task);
                        openEdit(task);
                      }}
                      ><span class="task-title"
                        >{task.parent_id ? '↳ ' : ''}{task.title}</span
                      ><span class="task-meta"
                        >{#each task.tags as tag (tag)}<span class="tag"
                            >{tag}</span
                          >{/each}{#if task.scheduled_start}<span
                            >◷ {timeLabel(task.scheduled_start)}</span
                          >{/if}{#if task.due_at}<span
                            >Due {localInput(
                              task.due_at,
                              snapshot!.settings.timezone,
                            ).replace('T', ' ')}</span
                          >{/if}{#if task.carried_count > 0}<span
                            class:warning={task.carried_count >= 3}
                            >Carried {task.carried_count}×{task.carried_count >=
                            3
                              ? ' · Do, split or drop'
                              : ''}</span
                          >{/if}</span
                      ></button
                    >
                    <span class="estimate"
                      >{task.actual_min
                        ? `${task.actual_min}/`
                        : ''}{task.est_min}m</span
                    ><span class="priority" class:urgent={task.priority === 1}
                      >P{task.priority}</span
                    >
                    {#if view === 'Inbox'}<button
                        class="today-button"
                        disabled={busy}
                        onclick={() => schedule(task.id, null, snapshot!.today)}
                        >Today ↗</button
                      >{/if}
                  </article>{/each}
              </div>
              {#if tasks.length > 80}<div class="pagination">
                  <button
                    disabled={selected < 40}
                    onclick={() => (selected = Math.max(0, selected - 40))}
                    >Previous</button
                  ><span>Window near task {selected + 1} of {tasks.length}</span
                  ><button
                    disabled={selected + 40 >= tasks.length}
                    onclick={() => (selected += 40)}>Next</button
                  >
                </div>{/if}
            {/if}
          </section>
          {#if view === 'Planner'}<section
              class="timeline"
              aria-label="Day timeline"
            >
              <h2>Timeline <small>{snapshot?.settings.timezone}</small></h2>
              {#each hours as hour (hour)}<div
                  class="time-slot"
                  role="group"
                  aria-label={`${hour}:00 drop target`}
                  ondragover={(e) => e.preventDefault()}
                  ondrop={(e) => drop(e, hour)}
                >
                  <span>{String(hour).padStart(2, '0')}:00</span>
                  <div>
                    {#each tasks.filter((t) => t.scheduled_start && Number(localInput(t.scheduled_start, snapshot!.settings.timezone).slice(11, 13)) === hour) as task (task.id)}<button
                        onclick={() => openEdit(task)}
                        >{task.title}<small
                          >{timeLabel(task.scheduled_start!)} – {task.scheduled_end
                            ? timeLabel(task.scheduled_end)
                            : ''}</small
                        ></button
                      >{/each}
                  </div>
                </div>{/each}
            </section>{/if}
        </div>
        {/if}
        <footer class="page-footer">
          <span
            ><kbd>N</kbd> new task <kbd>J</kbd><kbd>K</kbd> navigate
            <kbd>X</kbd>
            complete <kbd>E</kbd> edit</span
          ><span>Built for your day. Not your attention.</span>
        </footer>
      </div>
    </main>
  </div>
{/if}

{#if palette}<div class="overlay">
    <div
      use:focusDialog
      class="dialog palette"
      role="dialog"
      aria-modal="true"
      aria-label="Command palette"
      tabindex="-1"
    >
      <div class="section-title">
        <h2>Where would you like to go?</h2>
        <button onclick={() => (palette = false)}>Esc</button>
      </div>
      {#each views as item (item)}<button onclick={() => navigate(item)}
          >{item}<span>↗</span></button
        >{/each}<button
        onclick={() => {
          palette = false;
          quickInput?.focus();
        }}>New task <kbd>N</kbd></button
      ><button
        onclick={() => {
          palette = false;
          if (snapshot) preferences = { ...snapshot.settings };
        }}>Preferences</button
      >
    </div>
  </div>{/if}
{#if edit}<div class="overlay">
    <div
      use:focusDialog
      class="dialog"
      role="dialog"
      aria-modal="true"
      aria-label="Edit task"
      tabindex="-1"
    >
      <form
        onsubmit={(e) => {
          e.preventDefault();
          void saveEdit();
        }}
      >
        <div class="section-title">
          <h2>Task details</h2>
          <button type="button" onclick={() => (edit = null)}>Close</button>
        </div>
        <label
          >Title<input
            bind:value={edit.title}
            required
            maxlength="500"
          /></label
        ><label
          >Notes<textarea
            bind:value={edit.notes}
            rows="3"
            placeholder="A little more context…"
          ></textarea></label
        >
        <div class="form-grid">
          <label
            >Status<select bind:value={edit.status}
              >{#each ['inbox', 'todo', 'doing', 'done', 'dropped'] as status (status)}<option
                  value={status}>{status}</option
                >{/each}</select
            ></label
          ><label
            >Priority<select bind:value={edit.priority}
              >{#each [1, 2, 3, 4] as p (p)}<option value={p}>P{p}</option
                >{/each}</select
            ></label
          ><label
            >Estimate (minutes)<input
              type="number"
              min="1"
              max="1440"
              bind:value={edit.est_min}
              required
            /></label
          ><label
            >Actual (minutes)<input
              type="number"
              min="0"
              max="1000000"
              bind:value={edit.actual_min}
              required
            /></label
          ><label
            >Energy<select bind:value={edit.energy}
              ><option value="low">Low energy</option><option value="high"
                >Deep work</option
              ></select
            ></label
          ><label
            >Planned day<input
              type="date"
              bind:value={edit.planned_day}
            /></label
          ><label
            >Due ({snapshot?.settings.timezone})<input
              type="datetime-local"
              bind:value={edit.due_local}
            /></label
          ><label
            >Parent task<select bind:value={edit.parent_id}
              ><option value="">No parent</option
              >{#each snapshot?.tasks.filter((t) => t.id !== edit?.id) ?? [] as parent (parent.id)}<option
                  value={parent.id}>{parent.title}</option
                >{/each}</select
            ></label
          ><label
            >Project<select bind:value={edit.project_id}
              ><option value="">No project</option
              >{#each snapshot?.projects ?? [] as project (project.id)}<option
                  value={project.id}>{project.name}</option
                >{/each}</select
            ></label
          ><label
            >Goal<select bind:value={edit.goal_id}
              ><option value="">No goal</option
              >{#each snapshot?.goals ?? [] as goal (goal.id)}<option
                  value={goal.id}>{goal.title}</option
                >{/each}</select
            ></label
          ><label
            >Repeats<select bind:value={edit.recurrence_rule}
              ><option value="">Does not repeat</option
              ><option value="daily">Daily</option
              ><option value="weekdays">Weekdays</option
              ><option value="weekly">Weekly</option
              ><option value="monthly">Monthly</option
              ><option value="every 3 days">Every 3 days</option></select
            ></label
          >
        </div>
        <label
          >Tags<input
            bind:value={editTags}
            placeholder="#work @deep !review"
          /></label
        >
        <div class="schedule-editor">
          <label
            >Time block ({snapshot?.settings.timezone})<input
              type="datetime-local"
              aria-label="Schedule task time"
              bind:value={editStart}
            /></label
          ><small
            >Time blocks use the task?s estimated duration. Save changes to
            apply.</small
          >
        </div>
        {#if error}<p class="error" role="alert">{error}</p>{/if}
        {#if notice}<p class="muted" role="status">{notice}</p>{/if}
        <div class="dialog-actions">
          <button
            class="subtle"
            type="button"
            disabled={busy}
            onclick={() => void saveAsTemplate()}>Save as template</button
          ><button
            class="danger"
            type="button"
            disabled={busy}
            onclick={() => {
              const task = snapshot?.tasks.find((t) => t.id === edit?.id);
              if (task) void remove(task);
            }}>Delete task</button
          ><button class="primary" type="submit" disabled={busy}
            >Save changes</button
          >
        </div>
      </form>
    </div>
  </div>{/if}
{#if preferences}<div class="overlay">
    <div
      use:focusDialog
      class="dialog preferences"
      role="dialog"
      aria-modal="true"
      aria-label="Preferences"
      tabindex="-1"
    >
      <form
        onsubmit={async (e) => {
          e.preventDefault();
          const result = await run({
            command: 'save_settings',
            input: preferences!,
          });
          if (result) preferences = null;
        }}
      >
        <div class="section-title">
          <h2>Make it your day.</h2>
          <button type="button" onclick={() => (preferences = null)}
            >Close</button
          >
        </div>
        <p>Calendar dates and reminders use this timezone.</p>
        <label
          >IANA timezone<input
            bind:value={preferences.timezone}
            required
            placeholder="Asia/Kolkata"
          /></label
        ><button
          type="button"
          onclick={() => {
            if (preferences)
              preferences.timezone =
                Intl.DateTimeFormat().resolvedOptions().timeZone;
          }}>Use this device’s timezone</button
        >
        <div class="form-grid">
          <label
            >Daily task cap<input
              type="number"
              min="1"
              max="1000"
              bind:value={preferences.max_tasks}
              required
            /></label
          ><label
            >Daily minute cap<input
              type="number"
              min="1"
              max="1440"
              bind:value={preferences.max_minutes}
              required
            /></label
          ><label
            >Work starts (hour)<input
              type="number"
              min="0"
              max="23"
              bind:value={preferences.work_start}
              required
            /></label
          ><label
            >Work ends (hour)<input
              type="number"
              min="1"
              max="24"
              bind:value={preferences.work_end}
              required
            /></label
          >
        </div>
        <label class="bridge-toggle"
          ><input type="checkbox" bind:checked={preferences.autostart} />Start
          Dayboard on login (packaged builds)</label
        >
        <p class="muted">Development builds skip the OS autostart entry.</p>
        <fieldset class="bridge-settings">
          <legend>Jarvis bridge</legend>
          <label class="bridge-toggle"
            ><input
              type="checkbox"
              bind:checked={preferences.bridge_enabled}
            />Enable local bridge (restart to apply)</label
          >
          <div class="form-grid">
            <label
              >Port<input
                type="number"
                min="1"
                max="65535"
                bind:value={preferences.bridge_port}
                required
              /></label
            ><label
              >Bearer token<input
                bind:value={preferences.bridge_token}
                placeholder="At least 16 characters"
              /></label
            >
          </div>
          <button
            type="button"
            onclick={() => {
              if (preferences)
                preferences.bridge_token = crypto.randomUUID().replaceAll('-', '');
            }}>Generate token</button
          >
          <p class="muted">
            Binds 127.0.0.1 only. Endpoints: <code>/rpc</code>,
            <code>/mcp</code>, <code>/events</code>. Off by default.
          </p>
        </fieldset>
        <p class="muted">
          Global capture: Ctrl/Cmd + Shift + Space. Closing the window keeps
          reminders running in the tray. Use the tray menu to quit.
        </p>
        {#if error}<p class="error" role="alert">{error}</p>{/if}<button
          class="primary"
          type="submit"
          disabled={busy}>Save preferences</button
        >
      </form>
    </div>
  </div>{/if}
