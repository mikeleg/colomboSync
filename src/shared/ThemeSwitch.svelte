<script lang="ts">
  import Icon from "./Icon.svelte";
  import { loadTheme, setTheme, type Theme } from "./theme";

  const OPTIONS: { value: Theme; label: string }[] = [
    { value: "auto", label: "Auto" },
    { value: "light", label: "Chiaro" },
    { value: "dark", label: "Scuro" },
  ];

  let theme = $state(loadTheme());

  function choose(value: Theme) {
    theme = value;
    setTheme(value);
  }
</script>

<div class="seg" role="group" aria-label="Tema">
  {#each OPTIONS as o (o.value)}
    <button type="button" aria-pressed={theme === o.value} onclick={() => choose(o.value)}>
      <Icon name={o.value} size={16} />
      {o.label}
    </button>
  {/each}
</div>

<style>
  .seg {
    display: inline-flex;
    padding: 4px;
    border-radius: 999px;
    background: color-mix(in srgb, var(--fg) 8%, transparent);
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
  button:hover {
    color: var(--fg);
  }
  button[aria-pressed="true"] {
    background: var(--fg);
    color: var(--bg);
  }
</style>
