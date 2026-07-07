# Design Doc: Manual "Werksoorten" Management

- **Date:** 2026-07-07
- **Status:** Proposed

## 1. Background

The time tracker application currently supports bulk-importing "werksoorten" (tasks) from a JSON file. The user needs a way to manage these tasks directly within the application to handle "werksoorten" that change frequently, like Jira tickets for a sprint.

The user wants to keep the existing JSON import functionality for a base set of tasks, but also be able to add and delete tasks manually.

## 2. Requirements

1.  **Keep JSON Import:** The existing functionality to import tasks from a JSON file should be preserved.
2.  **Add New Tasks:** Users must be able to add new tasks (e.g., Jira tickets) individually.
3.  **Delete Tasks:** Users must be able to delete any task from their list, whether it was imported or added manually.
4.  **Preserve History:** Deleting a task should not delete the time entries associated with it. The task name will remain on historical entries.

## 3. Proposed Design

The solution will focus on adding simple, inline management features to the existing UI.

### 3.1. Frontend Changes (`src/routes/+page.svelte`)

-   **Add Task UI:**
    -   In the "All Tasks" view (toggled by the `/` key), an input field and an "Add" button will be added above the task list.
    -   The user can type a new task name and click "Add" or press Enter.
    -   On submission, the component will call the existing `add_task` Tauri command.
    -   After a successful call, the local list of tasks will be refreshed.

-   **Delete Task UI:**
    -   In the "All Tasks" list, a delete icon (e.g., a trash can) will appear on hover next to each task.
    -   Clicking the icon will trigger a confirmation dialog (`window.confirm`).
    -   If the user confirms, the component will call a new `delete_task` Tauri command, passing the task name.
    -   After a successful call, the local list of tasks will be refreshed.

### 3.2. Backend Changes (`src-tauri/`)

-   **`add_task` command (already exists):**
    -   The existing `add_task` command in `src-tauri/src/commands.rs` will be used. It takes a task `name` and adds it to the database via the `TimerService` and `SqliteRepository`. No changes are needed here.

-   **New `delete_task` command:**
    -   A new command `delete_task(name: String)` will be created in `src-tauri/src/commands.rs`.
    -   This command will call a corresponding `delete_task` method in the `TimerService`.
    -   The `TimerService` will call `repo.delete_task(&name)` in `src-tauri/src/services/timer_service.rs`.
    -   The `SqliteRepository` will implement the `delete_task` method in `src-tauri/src/repository/sqlite.rs`, which will execute a `DELETE FROM tasks WHERE name = ?` SQL query.

## 4. Database

-   No schema changes are required. The `tasks` table already exists.
-   The deletion of a task will not cascade to the `time_entries` table to preserve historical data.

## 5. User Flow

1.  User opens the app and navigates to the "All Tasks" view.
2.  **To add:** They type "JIRA-123" into the new input field and press Enter. The task "JIRA-123" appears in their list.
3.  **To delete:** They hover over an old task, "Old Project", and click the delete icon. They confirm the action. The task "Old Project" is removed from the list.
