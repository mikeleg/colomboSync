<script lang="ts">
  import { onMount } from "svelte";
  import { readRunLog } from "./api";
  import { formatWhen } from "./format";
  import type { Run } from "./types";

  let { run, onclose }: { run: Run; onclose: () => void } = $props();

  let text = $state("");
  let unavailable = $state(false);
  let closeButton: HTMLButtonElement;
  let dialog: HTMLElement;

  onMount(() => {
    const opener = document.activeElement as HTMLElement | null;
    closeButton.focus();
    readRunLog(run.logPath).then(
      (log) => (text = log),
      () => (unavailable = true),
    );
    return () => opener?.focus();
  });

  function onkeydown(e: KeyboardEvent) {
    if (e.key === "Escape") return onclose();
    if (e.key !== "Tab") return;
    // Keep Tab inside the dialog: wrap from the last control to the first and back.
    const stops = [...dialog.querySelectorAll<HTMLElement>("button, [tabindex='0']")];
    const edge = e.shiftKey ? stops[0] : stops[stops.length - 1];
    if (document.activeElement === edge || !dialog.contains(document.activeElement)) {
      e.preventDefault();
      (e.shiftKey ? stops[stops.length - 1] : stops[0]).focus();
    }
  }
</script>

<svelte:window {onkeydown} />

<div class="overlay" role="presentation" onclick={(e) => e.target === e.currentTarget && onclose()}>
  <div class="dialog" bind:this={dialog} role="dialog" aria-modal="true" aria-labelledby="log-title">
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
    background: var(--card);
    color: var(--fg);
    box-shadow: var(--shadow);
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
    background: var(--card-2);
    flex: 1;
    tab-size: 4;
  }
  .gone {
    margin: 0 22px 22px;
    color: var(--fg-2);
  }
</style>
