# Manual "Werksoorten" Management Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Allow users to manually add and delete "werksoorten" (tasks) from the application.

**Architecture:** We will add a new `delete_task` command to the Tauri backend and update the Svelte frontend to include UI for adding and deleting tasks.

**Tech Stack:** Svelte, TypeScript, Tauri, Rust, SQLite

---

### Task 1: Backend - Add `delete_task` functionality

**Files:**
- Modify: `src-tauri/src/repository/sqlite.rs`
- Modify: `src-tauri/src/services/timer_service.rs`
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Add `delete_task` method to `SqliteRepository`**

Modify `src-tauri/src/repository/sqlite.rs` to add the `delete_task` method.

```rust
// In src-tauri/src/repository/sqlite.rs, inside the impl SqliteRepository block

    pub fn delete_task(&self, name: &str) -> Result<()> {
        self.conn.execute("DELETE FROM tasks WHERE name = ?1", [name])?;
        Ok(())
    }
```

- [ ] **Step 2: Add `delete_task` method to `TimerService`**

Modify `src-tauri/src/services/timer_service.rs` to expose the `delete_task` functionality.

```rust
// In src-tauri/src/services/timer_service.rs, inside the impl TimerService block

    pub fn delete_task(&self, name: String) -> Result<()> {
        let repo = self.repo.lock().unwrap();
        repo.delete_task(&name)
    }
```

- [ ] **Step 3: Create `delete_task` Tauri command**

Modify `src-tauri/src/commands.rs` to create the `delete_task` command.

```rust
// In src-tauri/src/commands.rs

#[tauri::command]
pub fn delete_task(name: String, state: State<AppState>) -> Result<(), String> {
    state.timer_service.delete_task(name).map_err(|e| e.to_string())
}
```

- [ ] **Step 4: Register the new command**

Modify `src-tauri/src/lib.rs` to add the `delete_task` command to the `generate_handler!`.

```rust
// In src-tauri/src/lib.rs

// ...
        .invoke_handler(tauri::generate_handler![
            commands::import_tasks,
            commands::get_weekly_report,
            commands::get_scheduled_tasks,
            commands::add_scheduled_task,
            commands::delete_scheduled_task,
            commands::get_setting,
            commands::set_setting,
            commands::start_task,
            commands::stop_task,
            commands::add_task,
            commands::delete_task, // Add this line
            commands::get_status,
            commands::get_tasks,
            commands::get_daily_entries,
            commands::get_top_tasks,
            commands::get_last_task_name,
            commands::get_earliest_entry_date,
            commands::get_all_time_entries,
            commands::add_manual_entry,
            commands::update_time_entry,
            commands::delete_time_entry
        ])
// ...
```

### Task 2: Frontend - Implement Add and Delete UI

**Files:**
- Modify: `src/routes/+page.svelte`

- [ ] **Step 1: Add state for the new task input**

In the `<script>` section of `src/routes/+page.svelte`, add a new state variable for the "add task" input field.

```svelte
// ...
  let searchInput: HTMLInputElement | undefined = $state();
  let newTaskName = $state(""); // Add this line
  let isDark = $state(false);
// ...
```

- [ ] **Step 2: Create `addTask` and `deleteTask` functions**

In the `<script>` section of `src/routes/+page.svelte`, add the functions to call the backend commands.

```svelte
// ...

  async function addTask() {
    if (!newTaskName.trim()) return;
    try {
      await invoke("add_task", { name: newTaskName.trim() });
      newTaskName = "";
      await loadData(); // Reload tasks
    } catch (e) {
      alert("Fout bij toevoegen taak: " + e);
    }
  }

  async function deleteTask(name: string) {
    if (!window.confirm(`Weet je zeker dat je de taak "'${name}'" wilt verwijderen? Dit kan niet ongedaan worden gemaakt.`)) {
      return;
    }
    try {
      await invoke("delete_task", { name });
      await loadData(); // Reload tasks
    } catch (e) {
      alert("Fout bij verwijderen taak: " + e);
    }
  }

// ...
```

- [ ] **Step 3: Add the UI for adding a new task**

In the markup of `src/routes/+page.svelte`, inside the `{#if showAllTasks || searchTerm}` block, add the input field and button for adding a new task.

```svelte
    {#if showAllTasks || searchTerm}
      <div class="relative animate-in fade-in duration-200">
        <div class="absolute inset-y-0 left-3 flex items-center pointer-events-none text-muted-foreground">
          <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="11" cy="11" r="8"/><path d="m21 21-4.3-4.3"/></svg>
        </div>
        <input bind:this={searchInput} type="text" bind:value={searchTerm} placeholder="Zoek werksoort..." class="flex h-11 w-full rounded-md border border-input bg-background pl-10 pr-4 py-2 text-sm focus-visible:ring-2 focus-visible:ring-ring transition-all shadow-sm" />
      </div>

      <!-- ADD THIS NEW BLOCK -->
      <div class="flex gap-2 animate-in fade-in duration-200">
        <input type="text" bind:value={newTaskName} placeholder="Nieuwe taak toevoegen..." on:keydown={(e) => e.key === 'Enter' && addTask()} class="flex h-10 w-full rounded-md border border-input bg-background px-3 py-2 text-sm" />
        <button onclick={addTask} class="px-4 py-2 text-sm bg-primary text-primary-foreground rounded-md font-medium shadow-sm">Toevoegen</button>
      </div>
      <!-- END OF NEW BLOCK -->
    {/if}
```

- [ ] **Step 4: Add the delete button to the task list**

In the markup of `src/routes/+page.svelte`, refactor the task list item to be a `div` and include a delete button.

```svelte
    <div class="grid gap-2 max-h-[320px] overflow-y-auto pr-2 custom-scrollbar">
      {#each currentSelectorTasks as task, i}
        <div class="group flex items-center justify-between rounded-lg border bg-card px-4 py-3 text-left text-sm transition-all hover:border-primary/50 hover:bg-accent shadow-sm">
          <button onclick={() => startTask(task)} class="flex items-center flex-1 min-w-0 text-left">
            {#if !showAllTasks && !searchTerm}
              <span class="mr-3 flex h-5 w-5 shrink-0 items-center justify-center rounded-md bg-muted text-[10px] font-bold text-muted-foreground group-hover:bg-primary group-hover:text-primary-foreground transition-colors">{i + 1}</span>
            {/if}
            <span class="font-medium truncate">{task}</span>
          </button>
          <div class="flex items-center pl-2">
            {#if (showAllTasks || searchTerm)}
              <button onclick|stopPropagation={(e) => { e.preventDefault(); deleteTask(task); }} class="p-1 text-destructive opacity-0 group-hover:opacity-100 transition-opacity rounded-md hover:bg-destructive/10" title="Verwijder taak">
                <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 6h18"/><path d="M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6"/><path d="M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2"/></svg>
              </button>
            {/if}
            <button onclick={() => startTask(task)} class="p-1 text-muted-foreground group-hover:text-primary opacity-0 group-hover:opacity-100 transition-opacity" title="Start taak">
              <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="9 18 15 12 9 6"/></svg>
            </button>
          </div>
        </div>
      {/each}
    </div>
```
