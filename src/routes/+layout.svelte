<script lang="ts">
  import "../app.css";
  import { onMount } from "svelte";

  let { children } = $props();

  onMount(() => {
    const savedTheme = localStorage.getItem("theme");
    const darkModeMediaQuery = window.matchMedia("(prefers-color-scheme: dark)");
    
    function applyTheme(isDark: boolean) {
      if (isDark) {
        document.documentElement.classList.add("dark");
      } else {
        document.documentElement.classList.remove("dark");
      }
    }

    if (savedTheme) {
      applyTheme(savedTheme === "dark");
    } else {
      applyTheme(darkModeMediaQuery.matches);
    }

    // Still listen for system changes if no manual preference is set
    darkModeMediaQuery.addEventListener("change", (e) => {
      if (!localStorage.getItem("theme")) {
        applyTheme(e.matches);
      }
    });
  });
</script>

<div class="min-h-screen bg-background text-foreground transition-colors duration-300">
  {@render children()}
</div>
