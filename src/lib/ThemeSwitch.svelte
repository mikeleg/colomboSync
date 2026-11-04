<script lang="ts">
  import { applyTheme, loadTheme, type Theme } from "./theme";

  const OPTIONS: { value: Theme; label: string; icon: string }[] = [
    { value: "auto", label: "Auto", icon: '<circle cx="12" cy="12" r="9"/><path d="M12 3a9 9 0 0 1 0 18z" fill="currentColor" stroke="none"/>' },
    {
      value: "light",
      label: "Chiaro",
      icon: '<circle cx="12" cy="12" r="4"/><path d="M12 2v2M12 20v2M2 12h2M20 12h2M4.9 4.9l1.4 1.4M17.7 17.7l1.4 1.4M4.9 19.1l1.4-1.4M17.7 6.3l1.4-1.4"/>',
    },
    { value: "dark", label: "Scuro", icon: '<path d="M20 14.5A8.5 8.5 0 0 1 9.5 4a8.5 8.5 0 1 0 10.5 10.5z"/>' },
  ];

  const saved = loadTheme();
  let theme = $state(saved);
  applyTheme(saved);

  function choose(value: Theme) {
    theme = value;
    applyTheme(value);
  }
</script>

<div class="seg" role="group" aria-label="Tema">
  {#each OPTIONS as o (o.value)}
    <button type="button" aria-pressed={theme === o.value} onclick={() => choose(o.value)}>
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
        {@html o.icon}
      </svg>
      {o.label}
    </button>
  {/each}
</div>

<style>
  .seg {
    display: inline-flex;
    padding: 4px;
    border-radius: 999px;
    background: var(--card-2);
    gap: 2px;
  }
  button {
    border: 0;
    background: none;
    border-radius: 999px;
    min-height: 36px;
    padding: 0 14px;
    font-size: 14px;
    font-weight: 700;
    color: var(--fg-2);
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }
  button svg {
    width: 16px;
    height: 16px;
  }
  button:hover {
    color: var(--fg);
  }
  button[aria-pressed="true"] {
    background: var(--card);
    color: var(--fg);
    box-shadow: var(--shadow);
  }
</style>
