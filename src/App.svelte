<script lang="ts">
  import { onMount } from 'svelte';
  import { listen } from '@tauri-apps/api/event';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import {
    dispatch,
    native,
    type Snapshot,
    type Task,
    type TaskEdit,
    type Settings,
    type Command,
  } from './lib/api';
  import { localInput, addDays } from './lib/dates';
  const capture = new URLSearchParams(location.search).has('capture');
  let snapshot = $state<Snapshot | null>(null);
  let view = $state('Today');
  let text = $state('');
  let search = $state('');
  let error = $state('');
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
  const views = ['Today', 'Inbox', 'Planner', 'Completed'];
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
        scheduled_local: editStart || null,
      },
    });
    if (result) edit = null;
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
    await run({
      command: 'reschedule_task',
      input: { id, local_start, day: target, dry_run: false },
    });
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
  onMount(() => {
    if (!native) return;
    const unlisteners: (() => void)[] = [];
    let disposed = false;
    async function setup() {
      for (const name of ['daybridge:changed', 'daybridge:shown']) {
        const stop = await listen(name, () => {
          if (!document.hidden)
            void refresh().catch((e) => (error = String(e)));
          if (capture) quickInput?.focus();
        });
        if (disposed) stop();
        else unlisteners.push(stop);
      }
      const stop = await listen<string>(
        'daybridge:error',
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
      <strong>◈ Daybridge <span> / Quick capture</span></strong><button
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
        ><span class="brand-mark">◈</span> daybridge<span class="version"
          >β</span
        ></a
      >
      <div class="workspace-label">YOUR SPACE <span>LOCAL</span></div>
      <nav aria-label="Main navigation">
        {#each views as item, index (item)}<button
            class:active={view === item}
            onclick={() => navigate(item)}
            ><span class="nav-icon">{['☀', '▣', '▦', '✓'][index]}</span
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
                    : 'Look at what you’ve done.'}
            </h1>
            <p>
              {view === 'Today'
                ? 'Choose a few good things. Let the rest wait.'
                : view === 'Inbox'
                  ? 'Capture now. Find a place for it when you’re ready.'
                  : view === 'Planner'
                    ? 'Drag a task to a time slot, or schedule it from the task menu.'
                    : 'Small steps add up. Reopen a task whenever you need to.'}
            </p>
          </div>
          <div class="day-number">
            {snapshot?.today.slice(-2) ?? '—'}<span
              >{view === 'Planner' ? 'PLAN AHEAD' : 'ONE DAY AT A TIME'}</span
            >
          </div>
        </section>
        {#if error}<div class="error" role="alert">
            {error}<button
              onclick={() => (error = '')}
              aria-label="Dismiss error">×</button
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
        <section class="capture-card" aria-label="Capture a task">
          {@render captureForm()}
        </section>
        {#if view === 'Today' && snapshot}<section class="capacity-card">
            <div>
              <span class="eyebrow">TODAY’S CAPACITY</span>
              <p>
                <strong
                  >{snapshot.capacity.minutes}<small>
                    / {snapshot.settings.max_minutes} min</small
                  ></strong
                ><span
                  >{snapshot.capacity.count} of {snapshot.settings.max_tasks} tasks</span
                >
              </p>
              <progress
                value={snapshot.capacity.minutes}
                max={snapshot.settings.max_minutes}
                aria-label="Planned daily minutes"
              ></progress>
            </div>
            <div class="capacity-message">
              {#if snapshot.capacity.over_count || snapshot.capacity.over_minutes}<strong
                  class="warning">A full plate.</strong
                ><span>Move something to another day.</span>{:else}<strong
                  >Leave a little breathing room.</strong
                ><span>Your plan is a guide, not a race.</span>{/if}
            </div>
            <button class="subtle" disabled={busy} onclick={plan}
              >Suggest a plan ↗</button
            >
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
        <div class="dialog-actions">
          <button
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
