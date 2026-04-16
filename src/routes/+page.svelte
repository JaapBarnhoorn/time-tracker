<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount, tick } from "svelte";
  import { isPermissionGranted, requestPermission, sendNotification } from "@tauri-apps/plugin-notification";

  interface Status {
    running: boolean;
    id: number | null;
    task_name: string | null;
    started_at: string | null;
    elapsed_seconds: number;
  }

  interface TimeEntry {
    id: number;
    task_name: string;
    started_at: string;
    stopped_at: string | null;
  }

  interface ScheduledTask {
    id: number;
    task_name: string;
    occurrence: "Once" | "Daily" | "Weekly" | "BiWeekly" | "Monthly";
    start_time: string;
    day_of_week: number | null;
    day_of_month: number | null;
  }

  interface WeeklyReportEntry {
    task_name: string;
    total_seconds_per_day: number[];
    total_seconds: number;
  }

  interface WeeklyReport {
    start_date: string;
    end_date: string;
    entries: WeeklyReportEntry[];
    daily_totals: number[];
  }

  let tasks: string[] = $state([]);
  let topTasks: string[] = $state([]);
  let scheduledTasks: ScheduledTask[] = $state([]);
  let searchTerm = $state("");
  
  // Weekly Report State
  let showWeeklyReport = $state(false);
  let weeklyReport: WeeklyReport | null = $state(null);
  
  // Scheduling UI state
  let showAddSchedule = $state(false);
  let newScheduleTask = $state("");
  let newScheduleOccurrence = $state<"Once" | "Daily" | "Weekly" | "BiWeekly" | "Monthly">("Daily");
  let newScheduleTime = $state("");
  let newScheduleDayOfWeek = $state(1); // Default Mon
  let newScheduleDayOfMonth = $state(1);
  let scheduleTaskSearch = $state("");
  let status: Status = $state({ running: false, task_name: null, elapsed_seconds: 0 });
  let dailyEntries: TimeEntry[] = $state([]);
  let lastTaskName: string | null = $state(null);
  let searchInput: HTMLInputElement | undefined = $state();
  let isDark = $state(false);
  
  // Day change tracking
  let lastCheckedDay = new Date().getDate();
  let currentSystemDate = $state(new Date().toISOString().split('T')[0]);

  function checkDayChange() {
    const now = new Date();
    const currentDay = now.getDate();
    
    if (currentDay !== lastCheckedDay) {
      console.log("Dagwissel gedetecteerd, UI verversen...");
      lastCheckedDay = currentDay;
      currentSystemDate = now.toISOString().split('T')[0];
      
      const oldToday = new Date(new Date().setDate(now.getDate() - 1)).toISOString().split('T')[0];
      if (viewDateStr === oldToday) {
        setToday();
      } else {
        loadDailyEntries();
      }
      loadScheduledTasks();
    }
  }
  
  // View Date State
  let viewDate = $state(new Date());
  let viewDateStr = $derived(`${viewDate.getFullYear()}-${String(viewDate.getMonth() + 1).padStart(2, '0')}-${String(viewDate.getDate()).padStart(2, '0')}`);

  // UI State
  let showManual = $state(false);
  let showAllTasks = $state(false);
  let manualTaskSearch = $state("");
  let selectedManualTask = $state("");
  let manualStart = $state("");
  let manualEnd = $state("");
  let manualSearchInput: HTMLInputElement | undefined = $state();

  // Edit state
  let editingId = $state<number | null>(null);
  let editStart = $state("");
  let editEnd = $state("");
  let isEditingCurrentStart = $state(false);
  let entryToDelete = $state<number | null>(null);
  let scheduledToDelete = $state<number | null>(null);

  let filteredTasks = $derived(
    tasks.filter(t => t.toLowerCase().includes(searchTerm.toLowerCase()))
  );

  let filteredScheduleTasks = $derived(
    tasks.filter(t => t.toLowerCase().includes(scheduleTaskSearch.toLowerCase()))
  );

  let filteredManualTasks = $derived(
    tasks.filter(t => t.toLowerCase().includes(manualTaskSearch.toLowerCase()))
  );

  let currentSelectorTasks = $derived(
    showAllTasks || searchTerm ? filteredTasks.slice(0, 15) : topTasks
  );

  // Keyboard Navigation
  function handleKeyDown(event: KeyboardEvent) {
    if (event.target instanceof HTMLInputElement || event.target instanceof HTMLSelectElement) {
      if (event.key === "Escape") {
        searchTerm = "";
        showAllTasks = false;
        showManual = false;
        editingId = null;
        (event.target as HTMLElement).blur();
      }
      return;
    }

    const key = event.key.toLowerCase();

    // 1-9 om top taken te starten
    if (!showAllTasks && !searchTerm && !showManual && key >= "1" && key <= "9") {
      const index = parseInt(key) - 1;
      if (topTasks[index]) {
        event.preventDefault();
        startTask(topTasks[index]);
      }
    }

    if (key === "s") {
      event.preventDefault();
      stopTask();
    }

    if (key === "r") {
      event.preventDefault();
      resumeLastTask();
    }

    if (key === "/") {
      event.preventDefault();
      if (showAllTasks || searchTerm) {
        showAllTasks = false;
        searchTerm = "";
      } else {
        showAllTasks = true;
        tick().then(() => searchInput?.focus());
      }
    }

    if (key === ",") {
      event.preventDefault();
      showSettings = !showSettings;
    }

    if (key === "+") {
      event.preventDefault();
      toggleManual();
    }

    // Pijltjes voor datum navigatie
    if (event.key === "ArrowLeft") {
      changeDate(-1);
    }
    if (event.key === "ArrowRight") {
      changeDate(1);
    }
  }

  const dayNames = ["Zondag", "Maandag", "Dinsdag", "Woensdag", "Donderdag", "Vrijdag", "Zaterdag"];

  // Settings State
  let showSettings = $state(false);
  let workDays = $state([1, 2, 3, 4, 5]); // Default Mon-Fri

  async function loadSettings() {
    try {
      const saved = await invoke<string | null>("get_setting", { key: "workDays" });
      if (saved) {
        workDays = JSON.parse(saved);
      }
    } catch (e) {
      console.error("Settings laden mislukt:", e);
    }
  }

  async function toggleWorkDay(day: number) {
    if (workDays.includes(day)) {
      if (workDays.length > 1) {
        workDays = workDays.filter(d => d !== day);
      } else {
        alert("Je moet minimaal één werkdag hebben.");
        return;
      }
    } else {
      workDays = [...workDays, day].sort();
    }
    
    try {
      await invoke("set_setting", { key: "workDays", value: JSON.stringify(workDays) });
    } catch (e) {
      console.error("Settings opslaan mislukt:", e);
    }
  }

  function isWorkDay(date: Date) {
    return workDays.includes(date.getDay());
  }

  function changeDate(days: number) {
    let next = new Date(viewDate);
    let attempts = 0;
    
    do {
      next.setDate(next.getDate() + days);
      attempts++;
      // Beveiliging tegen oneindige loop als er geen werkdagen zijn (hoewel we dat checken)
      if (attempts > 31) break; 
    } while (!isWorkDay(next));

    // Don't allow navigating into the future
    if (next <= new Date()) {
      viewDate = next;
      loadDailyEntries();
    }
  }

  function setToday() {
    viewDate = new Date();
    loadDailyEntries();
  }

  async function toggleWeeklyReport() {
    showWeeklyReport = !showWeeklyReport;
    if (showWeeklyReport) {
      await loadWeeklyReport();
    }
  }

  async function loadWeeklyReport() {
    // Vind de maandag van de huidige viewDate
    const d = new Date(viewDate);
    const day = d.getDay();
    const diff = d.getDate() - day + (day === 0 ? -6 : 1); // Pas aan naar Maandag
    const monday = new Date(d.setDate(diff));
    const mondayStr = `${monday.getFullYear()}-${String(monday.getMonth() + 1).padStart(2, '0')}-${String(monday.getDate()).padStart(2, '0')}`;
    
    try {
      weeklyReport = await invoke("get_weekly_report", { startDate: mondayStr });
    } catch (e) {
      console.error("Weekrapport laden mislukt:", e);
    }
  }

  async function handleSqlUpload(event: Event) {
    const input = event.target as HTMLInputElement;
    if (!input.files || input.files.length === 0) return;
    
    const file = input.files[0];
    const reader = new FileReader();
    
    reader.onload = async (e) => {
      const sql = e.target?.result as string;
      try {
        await invoke("import_tasks", { sql });
        alert("Taken succesvol geïmporteerd!");
        await loadData();
      } catch (err) {
        alert("Fout bij importeren: " + err);
      }
    };
    
    reader.readAsText(file);
    input.value = ""; // Reset input
  }

  async function loadData() {
    try {
      tasks = await invoke("get_tasks");
    } catch (e) {
      console.error("Laden van taken mislukt:", e);
    }
    await updateTopTasks();
    await updateStatus();
    await loadDailyEntries();
    await loadScheduledTasks();
    lastTaskName = await invoke("get_last_task_name");
  }

  async function loadScheduledTasks() {
    try {
      scheduledTasks = await invoke("get_scheduled_tasks");
    } catch (e) {
      console.error("Geplande taken laden mislukt:", e);
    }
  }

  async function addScheduledTask() {
    const taskName = newScheduleTask || scheduleTaskSearch;
    if (!taskName || !newScheduleTime) return;

    const dayOfWeek = ["Weekly", "BiWeekly"].includes(newScheduleOccurrence) ? newScheduleDayOfWeek : null;
    const dayOfMonth = newScheduleOccurrence === "Monthly" ? newScheduleDayOfMonth : null;

    const payload = { 
      taskName: taskName, 
      occurrence: newScheduleOccurrence, 
      startTime: newScheduleTime,
      dayOfWeek: dayOfWeek,
      dayOfMonth: dayOfMonth
    };

    try {
      await invoke("add_scheduled_task", payload);
      newScheduleTask = "";
      scheduleTaskSearch = "";
      newScheduleTime = "";
      showAddSchedule = false;
      await loadScheduledTasks();
    } catch (e) {
      alert("Fout bij plannen: " + e);
    }
  }


  async function deleteScheduledTask(id: number) {
    if (id === undefined || id === null) return;
    
    try {
      await invoke("delete_scheduled_task", { id });
      scheduledToDelete = null;
      await loadScheduledTasks();
    } catch (e) {
      alert("Verwijderen mislukt: " + e);
    }
  }

  async function updateTopTasks() {
    try {
      const top: string[] = await invoke("get_top_tasks");
      if (top.length < 9) {
        const remaining = tasks.filter(t => !top.includes(t)).slice(0, 9 - top.length);
        topTasks = [...top, ...remaining];
      } else {
        topTasks = top;
      }
    } catch (e) {
      topTasks = tasks.slice(0, 9);
    }
  }

  async function updateStatus() {
    try {
      const newStatus: Status = await invoke("get_status");
      if (newStatus.running && newStatus.started_at) {
        const start = new Date(newStatus.started_at).getTime();
        const now = new Date().getTime();
        newStatus.elapsed_seconds = Math.floor((now - start) / 1000);
      }
      status = newStatus;
    } catch (e) {
      console.error("Status update mislukt:", e);
    }
  }

  async function loadDailyEntries() {
    try {
      dailyEntries = await invoke("get_daily_entries", { date: viewDateStr });
    } catch (e) {
      console.error("Daily entries laden mislukt:", e);
    }
  }

  async function startTask(name: string) {
    try {
      await invoke("start_task", { taskName: name });
      searchTerm = "";
      showAllTasks = false;
      setToday(); // Switch to today if we start a task
      await updateStatus();
      await loadDailyEntries();
      await updateTopTasks();
      lastTaskName = await invoke("get_last_task_name");
    } catch (e) {
      alert("Fout bij starten taak: " + e);
    }
  }

  async function resumeLastTask() {
    try {
      const lastTask: string | null = await invoke("get_last_task_name");
      if (lastTask) {
        await startTask(lastTask);
      } else {
        alert("Geen recente taak gevonden om te hervatten.");
      }
    } catch (e) {
      console.error("Resume mislukt:", e);
    }
  }

  async function stopTask() {
    try {
      await invoke("stop_task");
      await updateStatus();
      await loadDailyEntries();
      await updateTopTasks();
      lastTaskName = await invoke("get_last_task_name");
    } catch (e) {
      alert("Fout bij stoppen taak: " + e);
    }
  }

  async function addManualEntry() {
    if (!selectedManualTask || !manualStart || !manualEnd) return;
    const start = new Date(`${viewDateStr}T${manualStart}:00`).toISOString();
    const end = new Date(`${viewDateStr}T${manualEnd}:00`).toISOString();

    try {
      await invoke("add_manual_entry", { taskName: selectedManualTask, startedAt: start, stoppedAt: end });
      showManual = false;
      selectedManualTask = "";
      manualTaskSearch = "";
      await loadDailyEntries();
      await updateTopTasks();
    } catch (e) {
      alert("Fout bij toevoegen: " + e);
    }
  }

  async function deleteEntry(id: number) {
    if (id === undefined || id === null) return;
    
    try {
      await invoke("delete_time_entry", { id });
      entryToDelete = null;
      await loadDailyEntries();
      await updateStatus();
      await updateTopTasks();
    } catch (e) {
      alert("Verwijderen mislukt: " + e);
    }
  }

  function startEdit(entry: TimeEntry) {
    editingId = entry.id;
    const startDt = new Date(entry.started_at);
    editStart = `${String(startDt.getHours()).padStart(2, '0')}:${String(startDt.getMinutes()).padStart(2, '0')}`;
    if (entry.stopped_at) {
      const endDt = new Date(entry.stopped_at);
      editEnd = `${String(endDt.getHours()).padStart(2, '0')}:${String(endDt.getMinutes()).padStart(2, '0')}`;
    } else {
      editEnd = "";
    }
  }

  async function saveEdit(id: number) {
    const start = new Date(`${viewDateStr}T${editStart}:00`).toISOString();
    const end = editEnd ? new Date(`${viewDateStr}T${editEnd}:00`).toISOString() : null;

    try {
      await invoke("update_time_entry", { id, startedAt: start, stoppedAt: end });
      editingId = null;
      await loadDailyEntries();
      await updateStatus();
    } catch (e) {
      alert("Opslaan mislukt: " + e);
    }
  }

  async function saveCurrentStart() {
    if (!status.id) return;
    const today = new Date().toISOString().split('T')[0];
    const start = new Date(`${today}T${editStart}:00`).toISOString();

    try {
      await invoke("update_time_entry", { id: status.id, startedAt: start, stoppedAt: null });
      isEditingCurrentStart = false;
      await updateStatus();
      await loadDailyEntries();
    } catch (e) {
      alert("Aanpassen mislukt: " + e);
    }
  }

  function startEditingCurrent() {
    if (!status.started_at) return;
    const startDt = new Date(status.started_at);
    editStart = `${String(startDt.getHours()).padStart(2, '0')}:${String(startDt.getMinutes()).padStart(2, '0')}`;
    isEditingCurrentStart = true;
  }

  function formatTime(seconds: number) {
    const h = Math.floor(seconds / 3600);
    const m = Math.floor((seconds % 3600) / 60);
    const s = seconds % 60;
    return `${h.toString().padStart(2, '0')}:${m.toString().padStart(2, '0')}:${s.toString().padStart(2, '0')}`;
  }

  function toggleTheme() {
    isDark = !isDark;
    document.documentElement.classList.toggle("dark", isDark);
    localStorage.setItem("theme", isDark ? "dark" : "light");
  }

  async function toggleManual() {
    showManual = !showManual;
    if (showManual) {
      await tick();
      manualSearchInput?.focus();
    }
  }

  async function checkNotificationPermission() {
    let granted = await isPermissionGranted();
    if (!granted) {
      const permission = await requestPermission();
      granted = permission === "granted";
    }
    return granted;
  }

  onMount(() => {
    isDark = document.documentElement.classList.contains("dark");
    window.addEventListener("keydown", handleKeyDown);
    window.addEventListener("focus", checkDayChange);
    loadSettings();
    loadData();
    const interval = setInterval(() => {
      updateStatus();
      checkDayChange();
    }, 1000);
    return () => {
      clearInterval(interval);
      window.removeEventListener("keydown", handleKeyDown);
      window.removeEventListener("focus", checkDayChange);
    };
  });
</script>

<main class="mx-auto max-w-2xl px-6 py-10 pb-24">
  <div class="mb-8 flex items-center justify-between">
    <h1 class="text-2xl font-bold tracking-tight">Time Tracker</h1>
    <div class="flex items-center gap-2">
      <button onclick={toggleTheme} class="inline-flex h-9 w-9 items-center justify-center rounded-md border bg-card text-muted-foreground hover:bg-accent transition-colors">
        {#if isDark}
          <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="4"/><path d="M12 2v2"/><path d="M12 20v2"/><path d="m4.93 4.93 1.41 1.41"/><path d="m17.66 17.66 1.41 1.41"/><path d="M2 12h2"/><path d="M20 12h2"/><path d="m6.34 17.66-1.41 1.41"/><path d="m19.07 4.93-1.41 1.41"/></svg>
        {:else}
          <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 3a6 6 0 0 0 9 9 9 9 0 1 1-9-9Z"/></svg>
        {/if}
      </button>
      <button onclick={toggleManual} class="inline-flex h-9 w-9 items-center justify-center rounded-md border {showManual ? 'bg-primary text-primary-foreground border-primary' : 'bg-card text-muted-foreground'} hover:bg-accent transition-colors">
        <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M5 12h14"/><path d="M12 5v14"/></svg>
      </button>
      <button onclick={() => showSettings = !showSettings} class="inline-flex h-9 w-9 items-center justify-center rounded-md border {showSettings ? 'bg-primary text-primary-foreground border-primary' : 'bg-card text-muted-foreground'} hover:bg-accent transition-colors">
        <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z"/><circle cx="12" cy="12" r="3"/></svg>
      </button>
    </div>
  </div>

  {#if showSettings}
    <section class="mb-8 rounded-xl border bg-card p-6 shadow-lg animate-in fade-in slide-in-from-top-4 duration-200">
      <div class="flex items-center justify-between mb-4">
        <h3 class="text-sm font-semibold uppercase tracking-wider text-muted-foreground">Instellingen</h3>
        <button onclick={() => showSettings = false} class="text-muted-foreground hover:text-foreground">
          <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M18 6 6 18"/><path d="m6 6 12 12"/></svg>
        </button>
      </div>
      
      <div class="space-y-6">
        <div>
          <label class="text-[10px] uppercase font-bold text-muted-foreground mb-3 block">Mijn Werkdagen</label>
          <div class="flex flex-wrap gap-2">
            {#each [1, 2, 3, 4, 5, 6, 0] as day}
              <button 
                onclick={() => toggleWorkDay(day)}
                class="px-3 py-1.5 text-xs rounded-md border transition-all {workDays.includes(day) ? 'bg-primary text-primary-foreground border-primary' : 'bg-muted/50 text-muted-foreground border-transparent hover:border-muted-foreground/20'}"
              >
                {dayNames[day]}
              </button>
            {/each}
          </div>
          <p class="mt-2 text-[10px] text-muted-foreground italic">Geselecteerde dagen worden getoond bij het bladeren door rapporten.</p>
        </div>

        <div class="border-t pt-6">
          <div class="flex items-center justify-between mb-3">
            <label class="text-[10px] uppercase font-bold text-muted-foreground block">Geplande Taken</label>
            <button onclick={() => showAddSchedule = !showAddSchedule} class="text-xs text-primary font-bold hover:underline">
              {showAddSchedule ? 'Annuleren' : '+ Taak Inplannen'}
            </button>
          </div>

          {#if showAddSchedule}
            <div class="mb-4 space-y-3 rounded-lg bg-muted/30 p-4 border border-dashed animate-in fade-in slide-in-from-top-2 duration-200">
              <div class="space-y-2">
                <input 
                  type="text" 
                  bind:value={scheduleTaskSearch} 
                  oninput={() => newScheduleTask = ""}
                  placeholder="Zoek taak..." 
                  class="h-9 w-full rounded-md border bg-background px-3 text-xs" 
                />
                {#if scheduleTaskSearch && !newScheduleTask}
                  <div class="max-h-32 overflow-y-auto rounded-md border bg-popover divide-y">
                    {#each filteredScheduleTasks.slice(0, 5) as task}
                      <button onclick={() => { newScheduleTask = task; scheduleTaskSearch = task; }} class="w-full px-3 py-1.5 text-left text-xs hover:bg-accent">{task}</button>
                    {/each}
                  </div>
                {/if}
              </div>
              <div class="space-y-3">
                <label class="text-[10px] uppercase font-bold text-muted-foreground block">Herhaling & Tijd</label>
                <div class="flex flex-wrap gap-1.5">
                  {#each [
                    {val: 'Once', label: 'Eenmalig'},
                    {val: 'Daily', label: 'Dagelijks'},
                    {val: 'Weekly', label: 'Wekelijks'},
                    {val: 'BiWeekly', label: '2-Wekelijks'},
                    {val: 'Monthly', label: 'Maandelijks'}
                  ] as occ}
                    <button 
                      onclick={() => newScheduleOccurrence = occ.val}
                      class="px-2 py-1 text-[10px] rounded border transition-all {newScheduleOccurrence === occ.val ? 'bg-primary text-primary-foreground border-primary' : 'bg-muted/50 text-muted-foreground border-transparent hover:border-muted-foreground/20'}"
                    >
                      {occ.label}
                    </button>
                  {/each}
                </div>
                
                <div class="flex items-center gap-3">
                  <input type="time" bind:value={newScheduleTime} class="h-9 w-full rounded-md border bg-background px-3 text-xs" />
                </div>

                {#if ["Weekly", "BiWeekly"].includes(newScheduleOccurrence)}
                  <div class="space-y-2">
                    <label class="text-[10px] uppercase font-bold text-muted-foreground block">Op welke dag?</label>
                    <div class="flex flex-wrap gap-1">
                      {#each [1, 2, 3, 4, 5, 6, 0] as day}
                        <button 
                          onclick={() => newScheduleDayOfWeek = day}
                          class="px-2 py-1 text-[9px] rounded border transition-all {newScheduleDayOfWeek === day ? 'bg-primary text-primary-foreground border-primary' : 'bg-muted/30 text-muted-foreground border-transparent'}"
                        >
                          {dayNames[day].substring(0, 2)}
                        </button>
                      {/each}
                    </div>
                  </div>
                {/if}

                {#if newScheduleOccurrence === 'Monthly'}
                  <div class="space-y-2">
                    <label class="text-[10px] uppercase font-bold text-muted-foreground block">Op welke dag van de maand?</label>
                    <div class="flex items-center gap-2">
                      <input type="number" min="1" max="31" bind:value={newScheduleDayOfMonth} class="h-8 w-16 rounded border bg-background px-2 text-xs" />
                      <span class="text-[10px] text-muted-foreground italic">e dag van de maand</span>
                    </div>
                  </div>
                {/if}
                
                {#if newScheduleOccurrence === 'Daily'}
                  <p class="text-[9px] text-muted-foreground italic leading-tight">"Dagelijks" houdt rekening met je ingestelde werkdagen.</p>
                {/if}
              </div>
              <button onclick={addScheduledTask} disabled={!(newScheduleTask || scheduleTaskSearch) || !newScheduleTime} class="w-full h-9 bg-primary text-primary-foreground rounded-md text-xs font-bold disabled:opacity-50">Taak Inplannen</button>
            </div>
          {/if}

          <div class="space-y-2">
            {#each scheduledTasks as task}
              <div class="flex items-center justify-between rounded-lg border bg-background p-3">
                <div class="flex flex-col min-w-0 pr-4">
                  <span class="text-xs font-medium truncate">{task.task_name}</span>
                  <span class="text-[10px] text-muted-foreground uppercase font-bold mt-0.5">
                    {task.occurrence === 'Once' ? 'Eenmalig' :
                     task.occurrence === 'Daily' ? 'Dagelijks' : 
                     task.occurrence === 'Weekly' ? 'Wekelijks op ' + dayNames[task.day_of_week ?? 0] : 
                     task.occurrence === 'BiWeekly' ? '2-Wekelijks op ' + dayNames[task.day_of_week ?? 0] : 
                     'Maandelijks op de ' + task.day_of_month + 'e'} 
                    om {task.start_time}
                  </span>
                </div>
                <div class="flex items-center gap-2">
                  {#if scheduledToDelete === task.id}
                    <div class="flex items-center gap-2 animate-in fade-in zoom-in-95 duration-200">
                      <button onclick={() => deleteScheduledTask(task.id)} class="bg-destructive text-destructive-foreground p-1 rounded shadow-sm hover:bg-destructive/90 transition-colors" title="Definitief verwijderen">
                        <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 6h18"/><path d="M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6"/><path d="M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2"/></svg>
                      </button>
                      <button onclick={() => scheduledToDelete = null} class="text-muted-foreground hover:bg-muted p-1 rounded transition-colors" title="Annuleren">
                        <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M9 14 4 9l5-5"/><path d="M4 9h10.5a5.5 5.5 0 0 1 5.5 5.5v.5"/></svg>
                      </button>
                    </div>
                  {:else}
                    <button onclick={() => scheduledToDelete = task.id} class="text-destructive hover:bg-destructive/10 p-1 rounded transition-colors">
                      <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 6h18"/><path d="M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6"/><path d="M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2"/></svg>
                    </button>
                  {/if}
                </div>
              </div>
            {:else}
              <p class="text-center py-4 text-xs text-muted-foreground italic">Nog geen geplande taken.</p>
            {/each}
          </div>
        </div>

        <div class="border-t pt-6">
          <label class="text-[10px] uppercase font-bold text-muted-foreground block mb-3">Data Beheer</label>
          <div class="flex items-center gap-3">
            <label class="flex-1">
              <span class="sr-only">Kies .sql bestand</span>
              <div class="flex h-9 w-full items-center justify-center rounded-md border border-dashed border-muted-foreground/30 px-3 text-xs text-muted-foreground hover:bg-muted/30 cursor-pointer transition-colors">
                Taken importeren (.sql)
              </div>
              <input type="file" accept=".sql" onchange={handleSqlUpload} class="hidden" />
            </label>
          </div>
          <p class="mt-2 text-[9px] text-muted-foreground italic">Selecteer een .sql bestand om werksoorten in bulk toe te voegen.</p>
        </div>
      </div>
    </section>
  {/if}

  {#if showManual}
    <section class="mb-8 rounded-xl border bg-card p-6 shadow-lg animate-in fade-in slide-in-from-top-4 duration-200">
      <h3 class="text-sm font-semibold mb-4">Uren handmatig toevoegen ({viewDate.toLocaleDateString('nl-NL')})</h3>
      <div class="space-y-4">
        <div class="space-y-2">
          <div class="relative">
            <input 
              bind:this={manualSearchInput} 
              type="text" 
              bind:value={manualTaskSearch} 
              oninput={() => selectedManualTask = ""}
              placeholder="Zoek werksoort..." 
              class="flex h-10 w-full rounded-md border border-input bg-background pl-3 pr-3 py-2 text-sm focus-visible:ring-2 focus-visible:ring-ring" 
            />
          </div>
          {#if manualTaskSearch && !selectedManualTask}
            <div class="max-h-40 overflow-y-auto rounded-md border bg-popover shadow-md divide-y">
              {#each filteredManualTasks.slice(0, 10) as task}
                <button onclick={() => { selectedManualTask = task; manualTaskSearch = task; }} class="w-full px-3 py-2 text-left text-sm hover:bg-accent transition-colors">{task}</button>
              {/each}
            </div>
          {/if}
          {#if selectedManualTask}
            <div class="flex items-center justify-between rounded-md bg-primary/10 px-3 py-2 text-xs text-primary font-medium border border-primary/20">
              <span class="truncate">{selectedManualTask}</span>
              <button onclick={() => { selectedManualTask = ""; manualTaskSearch = ""; }} class="ml-2 hover:text-destructive transition-colors">
                <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M18 6 6 18"/><path d="m6 6 12 12"/></svg>
              </button>
            </div>
          {/if}
        </div>
        <div class="flex gap-4">
          <div class="flex-1">
            <label class="text-[10px] uppercase font-bold text-muted-foreground mb-1 block">Van</label>
            <input type="time" bind:value={manualStart} class="flex h-10 w-full rounded-md border border-input bg-background px-3 py-2 text-sm" />
          </div>
          <div class="flex-1">
            <label class="text-[10px] uppercase font-bold text-muted-foreground mb-1 block">Tot</label>
            <input type="time" bind:value={manualEnd} class="flex h-10 w-full rounded-md border border-input bg-background px-3 py-2 text-sm" />
          </div>
        </div>
        <div class="flex gap-2 justify-end pt-2">
          <button onclick={() => showManual = false} class="px-4 py-2 text-sm text-muted-foreground hover:text-foreground">Annuleren</button>
          <button onclick={addManualEntry} disabled={!selectedManualTask || !manualStart || !manualEnd} class="px-4 py-2 text-sm bg-primary text-primary-foreground rounded-md font-medium disabled:opacity-50 shadow-sm">Toevoegen</button>
        </div>
      </div>
    </section>
  {/if}

  <section class="relative mb-8 overflow-hidden rounded-xl border bg-card p-6 shadow-sm transition-all {status.running ? 'border-primary/50 bg-primary/5 shadow-primary/10' : ''}">
    {#if status.running}
      <div class="flex flex-col items-center justify-between gap-4 sm:flex-row">
        <div class="flex flex-col flex-1 min-w-0">
          <span class="text-xs font-medium uppercase tracking-wider text-muted-foreground">Actieve sessie</span>
          <h2 class="text-lg font-semibold line-clamp-1">{status.task_name}</h2>
          
          <div class="flex items-center gap-4 mt-2">
            <div class="text-4xl font-bold tabular-nums text-primary">
              {formatTime(status.elapsed_seconds)}
            </div>
            
            <div class="flex flex-col border-l pl-4 border-muted">
              <span class="text-[10px] font-bold uppercase text-muted-foreground">Gestart om</span>
              {#if isEditingCurrentStart}
                <div class="flex items-center gap-1 mt-1">
                  <input type="time" bind:value={editStart} class="h-7 w-20 rounded border bg-background px-1.5 text-xs" />
                  <button onclick={saveCurrentStart} class="text-xs text-primary font-bold ml-1">✓</button>
                  <button onclick={() => isEditingCurrentStart = false} class="text-xs text-muted-foreground ml-1">✕</button>
                </div>
              {:else}
                <button onclick={startEditingCurrent} class="group flex items-center gap-1.5 text-sm font-medium hover:text-primary transition-colors mt-0.5">
                  <span>{new Date(status.started_at!).toLocaleTimeString([], {hour: '2-digit', minute:'2-digit'})}</span>
                  <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="opacity-0 group-hover:opacity-100"><path d="M17 3a2.85 2.83 0 1 1 4 4L7.5 20.5 2 22l1.5-5.5Z"/></svg>
                </button>
              {/if}
            </div>
          </div>
        </div>
        <button onclick={stopTask} class="inline-flex h-10 items-center justify-center rounded-md bg-destructive px-6 py-2 text-sm font-bold text-destructive-foreground hover:bg-destructive/90 shadow-sm transition-all">
          Stop <span class="ml-2 opacity-70 font-normal">[S]</span>
        </button>
      </div>
    {:else}
      <div class="flex flex-col items-center justify-center py-4 text-center">
        <div class="mb-3 flex h-12 w-12 items-center justify-center rounded-full bg-muted/50 text-muted-foreground">
          <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><polyline points="12 6 12 12 16 14"/></svg>
        </div>
        <h2 class="text-lg font-medium">Geen actieve taak</h2>
        <p class="text-sm text-muted-foreground">Druk op [R] om je laatste taak te hervatten</p>
      </div>
    {/if}
  </section>

  <section class="mb-10 space-y-4">
    <div class="flex items-center justify-between">
      <h3 class="text-sm font-semibold text-muted-foreground uppercase tracking-wider">
        {#if searchTerm} Zoekresultaten {:else if showAllTasks} Alle Taken {:else} Favoriete Taken {/if}
      </h3>
      <button onclick={() => { if (showAllTasks || searchTerm) { showAllTasks = false; searchTerm = ""; } else { showAllTasks = true; tick().then(() => searchInput?.focus()); } }} class="text-xs font-bold text-primary hover:underline">
        {showAllTasks || searchTerm ? 'Terug naar favorieten' : 'Toon alle taken [/]'}
      </button>
    </div>

    {#if showAllTasks || searchTerm}
      <div class="relative animate-in fade-in duration-200">
        <div class="absolute inset-y-0 left-3 flex items-center pointer-events-none text-muted-foreground">
          <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="11" cy="11" r="8"/><path d="m21 21-4.3-4.3"/></svg>
        </div>
        <input bind:this={searchInput} type="text" bind:value={searchTerm} placeholder="Zoek werksoort..." class="flex h-11 w-full rounded-md border border-input bg-background pl-10 pr-4 py-2 text-sm focus-visible:ring-2 focus-visible:ring-ring transition-all shadow-sm" />
      </div>
    {/if}
    
    <div class="grid gap-2 max-h-[320px] overflow-y-auto pr-2 custom-scrollbar">
      {#each currentSelectorTasks as task, i}
        <button onclick={() => startTask(task)} class="group flex items-center justify-between rounded-lg border bg-card px-4 py-3 text-left text-sm transition-all hover:border-primary/50 hover:bg-accent shadow-sm">
          <div class="flex items-center flex-1 min-w-0">
            {#if !showAllTasks && !searchTerm}
              <span class="mr-3 flex h-5 w-5 shrink-0 items-center justify-center rounded-md bg-muted text-[10px] font-bold text-muted-foreground group-hover:bg-primary group-hover:text-primary-foreground transition-colors">{i + 1}</span>
            {/if}
            <span class="font-medium truncate">{task}</span>
          </div>
          <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="ml-2 opacity-0 transition-opacity group-hover:opacity-100"><polyline points="9 18 15 12 9 6"/></svg>
        </button>
      {/each}
    </div>
  </section>

  <section class="space-y-4 rounded-xl border bg-muted/30 p-6 shadow-sm">
    <div class="flex items-center justify-between border-b pb-3">
      <div class="flex items-center gap-2">
        <button onclick={() => changeDate(-1)} class="p-1 hover:bg-muted rounded-md transition-colors" title="Vorige dag [←]">
          <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m15 18-6-6 6-6"/></svg>
        </button>
        <h3 class="font-semibold tracking-tight min-w-[120px] text-center">
          {dayNames[viewDate.getDay()]} {viewDate.toLocaleDateString('nl-NL', {day: 'numeric', month: 'long'})}
          {#if viewDateStr === new Date().toISOString().split('T')[0]}
            <span class="ml-1 text-[10px] text-primary uppercase font-bold">(Vandaag)</span>
          {/if}
        </h3>
        <button 
          onclick={() => changeDate(1)} 
          disabled={viewDateStr === new Date().toISOString().split('T')[0]}
          class="p-1 hover:bg-muted rounded-md transition-colors disabled:opacity-20" 
          title="Volgende dag [→]"
        >
          <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m9 18 6-6-6-6"/></svg>
        </button>
      </div>
      {#if viewDateStr !== new Date().toISOString().split('T')[0]}
        <button onclick={setToday} class="text-[10px] font-bold text-primary hover:underline uppercase">Terug naar vandaag</button>
      {/if}
      <button onclick={toggleWeeklyReport} class="text-[10px] font-bold text-primary hover:underline uppercase ml-auto">Weekoverzicht</button>
    </div>
    
    <div class="space-y-1">
      {#each dailyEntries as entry}
        <div class="group relative flex flex-col py-3 border-b border-muted/50 last:border-0">
          <div class="flex items-start justify-between">
            <div class="flex flex-col flex-1 min-w-0 pr-4">
              <span class="text-sm font-medium truncate">{entry.task_name}</span>
              {#if editingId === entry.id}
                <div class="flex items-center gap-2 mt-2">
                  <input type="time" bind:value={editStart} class="h-8 rounded border bg-background px-2 text-xs" />
                  <span class="text-xs">→</span>
                  <input type="time" bind:value={editEnd} class="h-8 rounded border bg-background px-2 text-xs" />
                  <button onclick={() => saveEdit(entry.id)} class="text-xs text-primary font-bold ml-2">Save</button>
                  <button onclick={() => editingId = null} class="text-xs text-muted-foreground">Esc</button>
                </div>
              {:else}
                <button onclick={() => startEdit(entry)} class="flex items-center gap-2 text-xs text-muted-foreground mt-1 hover:text-primary transition-colors">
                  <span>{new Date(entry.started_at).toLocaleTimeString([], {hour: '2-digit', minute:'2-digit'})}</span>
                  <span>→</span>
                  <span>{entry.stopped_at ? new Date(entry.stopped_at).toLocaleTimeString([], {hour: '2-digit', minute:'2-digit'}) : '...'}</span>
                  <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="opacity-0 group-hover:opacity-100"><path d="M17 3a2.85 2.83 0 1 1 4 4L7.5 20.5 2 22l1.5-5.5Z"/></svg>
                </button>
              {/if}
            </div>
            <div class="flex items-center gap-3">
              {#if entry.stopped_at}
                {@const diff = Math.floor((new Date(entry.stopped_at).getTime() - new Date(entry.started_at).getTime()) / 1000)}
                <span class="text-xs font-mono bg-muted px-2 py-0.5 rounded">{formatTime(diff)}</span>
              {/if}
              
              {#if entryToDelete === entry.id}
                <div class="flex items-center gap-2 animate-in fade-in zoom-in-95 duration-200">
                  <button onclick={() => deleteEntry(entry.id)} class="bg-destructive text-destructive-foreground p-1 rounded shadow-sm hover:bg-destructive/90 transition-colors" title="Definitief verwijderen">
                    <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 6h18"/><path d="M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6"/><path d="M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2"/></svg>
                  </button>
                  <button onclick={() => entryToDelete = null} class="text-muted-foreground hover:bg-muted p-1 rounded transition-colors" title="Annuleren">
                    <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M9 14 4 9l5-5"/><path d="M4 9h10.5a5.5 5.5 0 0 1 5.5 5.5v.5"/></svg>
                  </button>
                </div>
              {:else}
                <button onclick={() => entryToDelete = entry.id} class="opacity-0 group-hover:opacity-100 text-destructive p-1 hover:bg-destructive/10 rounded transition-all">
                  <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 6h18"/><path d="M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6"/><path d="M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2"/></svg>
                </button>
              {/if}
            </div>
          </div>
        </div>
      {:else}
        <div class="py-10 text-center text-sm text-muted-foreground">
          Nog geen uren geschreven voor deze dag.
        </div>
      {/each}
    </div>
  </section>

  <div class="fixed bottom-0 left-0 right-0 border-t bg-background/80 backdrop-blur-md px-6 py-3">
    <div class="mx-auto max-w-2xl flex items-center justify-center gap-6 text-[10px] uppercase font-bold text-muted-foreground tracking-widest">
      <div class="flex items-center gap-1.5"><span class="bg-muted px-1.5 py-0.5 rounded text-foreground">1-9</span> Favoriet</div>
      {#if !status.running && lastTaskName}
        <div class="flex items-center gap-1.5 animate-in fade-in zoom-in-95 duration-200"><span class="bg-muted px-1.5 py-0.5 rounded text-foreground">R</span> Resume</div>
      {/if}
      <div class="flex items-center gap-1.5"><span class="bg-muted px-1.5 py-0.5 rounded text-foreground">← →</span> Bladeren</div>
      <div class="flex items-center gap-1.5"><span class="bg-muted px-1.5 py-0.5 rounded text-foreground">/</span> Zoek</div>
      <div class="flex items-center gap-1.5"><span class="bg-muted px-1.5 py-0.5 rounded text-foreground">,</span> Inst.</div>
      <div class="flex items-center gap-1.5"><span class="bg-muted px-1.5 py-0.5 rounded text-foreground">+</span> Handm.</div>
    </div>
  </div>

  {#if showWeeklyReport && weeklyReport}
    <div class="fixed inset-0 z-50 flex items-center justify-center bg-background/80 backdrop-blur-sm p-4 animate-in fade-in duration-200">
      <div class="w-full max-w-4xl max-h-[90vh] overflow-hidden rounded-xl border bg-card shadow-2xl flex flex-col">
        <div class="flex items-center justify-between border-b p-4 px-6">
          <div>
            <h2 class="text-lg font-bold">Weekoverzicht</h2>
            <p class="text-xs text-muted-foreground">{new Date(weeklyReport.start_date).toLocaleDateString('nl-NL', {day: 'numeric', month: 'long'})} t/m {new Date(weeklyReport.end_date).toLocaleDateString('nl-NL', {day: 'numeric', month: 'long'})}</p>
          </div>
          <button onclick={() => showWeeklyReport = false} class="text-muted-foreground hover:text-foreground">
            <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M18 6 6 18"/><path d="m6 6 12 12"/></svg>
          </button>
        </div>
        
        <div class="overflow-auto p-6">
          <table class="w-full border-collapse text-left text-xs">
            <thead>
              <tr class="border-b text-muted-foreground uppercase font-bold tracking-wider">
                <th class="py-3 pr-4">Werksoort</th>
                {#each ["Ma", "Di", "Wo", "Do", "Vr", "Za", "Zo"] as day}
                  <th class="py-3 px-2 text-center w-16">{day}</th>
                {/each}
                <th class="py-3 pl-4 text-right w-20">Totaal</th>
              </tr>
            </thead>
            <tbody class="divide-y">
              {#each weeklyReport.entries as entry}
                <tr class="hover:bg-muted/50 transition-colors group">
                  <td class="py-3 pr-4 font-medium truncate max-w-[200px]" title={entry.task_name}>{entry.task_name}</td>
                  {#each entry.total_seconds_per_day as seconds}
                    <td class="py-3 px-2 text-center tabular-nums {seconds === 0 ? 'text-muted/30' : ''}">
                      {seconds > 0 ? (seconds/3600).toFixed(1) : '-'}
                    </td>
                  {/each}
                  <td class="py-3 pl-4 text-right font-bold tabular-nums">
                    {(entry.total_seconds/3600).toFixed(1)}u
                  </td>
                </tr>
              {/each}
            </tbody>
            <tfoot>
              <tr class="border-t bg-muted/20 font-bold">
                <td class="py-4 pr-4 uppercase text-[10px] tracking-widest text-muted-foreground">Dagtotalen</td>
                {#each weeklyReport.daily_totals as total}
                  <td class="py-4 px-2 text-center tabular-nums">
                    {(total/3600).toFixed(1)}u
                  </td>
                {/each}
                <td class="py-4 pl-4 text-right text-primary tabular-nums">
                  {(weeklyReport.daily_totals.reduce((a, b) => a + b, 0)/3600).toFixed(1)}u
                </td>
              </tr>
            </tfoot>
          </table>
          
          <div class="mt-8 flex justify-center">
            <p class="text-[10px] text-muted-foreground italic max-w-md text-center">Tijden worden afgerond op één decimaal (bijv. 1.5 uur = 1 uur en 30 minuten) voor eenvoudige invoer in administratieve systemen.</p>
          </div>
        </div>
      </div>
    </div>
  {/if}
</main>

<style>
  .custom-scrollbar::-webkit-scrollbar { width: 6px; }
  .custom-scrollbar::-webkit-scrollbar-track { background: transparent; }
  .custom-scrollbar::-webkit-scrollbar-thumb { background: hsl(var(--muted)); border-radius: 10px; }
  .custom-scrollbar::-webkit-scrollbar-thumb:hover { background: #a1a1aa; }
  :global(.dark) .custom-scrollbar::-webkit-scrollbar-thumb { background: #3f3f46; }
</style>
