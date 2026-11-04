<script lang="ts">
  import { onMount } from "svelte";
  import { readRunLog } from "./api";
  import { formatWhen } from "./format";
  import type { Run } from "./types";

  let { run, onclose }: { run: Run; onclose: () => void } = $props();

  let text = $state("");
  let unavailable = $state(false);
  let closeButton: HTMLButtonElement;

  onMount(() => {
    closeButton.focus();
    readRunLog(run.logPath).then(
      (log) => (text = log),
      () => (unavailable = true),
    );
  });
</script>

<svelte:window onkeydown={(e) => e.key === "Escape" && onclose()} />

<div class="overlay" role="presentation" onclick={(e) => e.target === e.currentTarget && onclose()}>
  <div class="dialog" role="dialog" aria-modal="true" aria-labelledby="log-title">
    <header>
      <h2 id="log-title">Registro – {run.syncName} ({formatWhen(run.startedAt)})</h2>
      <button type="button" class="btn" bind:this={closeButton} onclick={onclose}>Chiudi</button>
    </header>
    {#if unavailable}
      <p class="gone">Il registro di questa copia non è più disponibile.</p>
    {:else}
      <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
      <pre tabindex="0">{text}</pre>
    {/if}
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: var(--scrim);
    display: grid;
    place-items: center;
    padding: 16px;
    z-index: 10;
  }
  .dialog {
    width: min(760px, 100%);
    max-height: 90vh;
    display: flex;
    flex-direction: column;
    background: var(--surface-container-lowest);
    color: var(--on-surface);
    box-shadow: var(--shadow-2);
    overflow: hidden;
  }
  header {
    display: flex;
    gap: 12px;
    align-items: center;
    justify-content: space-between;
    padding: 18px 22px;
  }
  h2 {
    min-width: 0;
    overflow-wrap: anywhere;
    font-size: 19px;
  }
  pre {
    margin: 0 16px 16px;
    padding: 16px 18px;
    overflow: auto;
    font: 13.5px/1.45 Consolas, "Cascadia Mono", monospace;
    background: var(--surface-container-low);
    flex: 1;
    tab-size: 4;
  }
  .gone {
    margin: 0 22px 22px;
    color: var(--on-surface-variant);
  }
</style>
