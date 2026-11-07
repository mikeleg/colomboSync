<script lang="ts">
  import { onMount } from "svelte";
  import { readRunLog } from "./api";
  import { formatWhen } from "./format";
  import RunChips from "./RunChips.svelte";
  import RunStats from "./RunStats.svelte";
  import type { RunLog, RunReport } from "./types";

  let { run, onclose }: { run: RunReport; onclose: () => void } = $props();

  let log = $state<RunLog>({ text: "", errorLines: [] });
  let unavailable = $state(false);
  let closeButton: HTMLButtonElement;
  let dialog: HTMLElement;

  onMount(() => {
    const opener = document.activeElement as HTMLElement | null;
    closeButton.focus();
    readRunLog(run.logPath).then(
      (read) => (log = read),
      () => (unavailable = true),
    );
    return () => opener?.focus();
  });

  /** The log as runs of plain lines, with each error line on its own (Rust says which lines are errors). */
  const segments = $derived.by(() => {
    const errors = new Set(log.errorLines);
    const lines = log.text.split("\n");
    const segments: { text: string; error: boolean }[] = [];
    lines.forEach((line, number) => {
      const text = number < lines.length - 1 ? `${line}\n` : line;
      const error = errors.has(number);
      const last = segments.at(-1);
      if (last && !error && !last.error) last.text += text;
      else segments.push({ text, error });
    });
    return segments;
  });

  const goToFirstError = () => dialog.querySelector<HTMLElement>(".error-line")?.focus();

  function onkeydown(e: KeyboardEvent) {
    if (e.key === "Escape") return onclose();
    if (e.key !== "Tab") return;
    // Keep Tab inside the dialog: wrap from the last control to the first and back.
    const stops = [...dialog.querySelectorAll<HTMLElement>("button, [tabindex='0']")];
    const edge = e.shiftKey ? stops[0] : stops[stops.length - 1];
    // A line inside the log counts as the log: Tab from it leaves the log, not the dialog.
    if (edge.contains(document.activeElement) || !dialog.contains(document.activeElement)) {
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
    <section class="summary" aria-label="Riepilogo">
      <div class="chips">
        <RunChips {run} />
      </div>
      <RunStats {run} />
      {#if run.explanation}
        <p class="why">
          <strong>{run.explanation.title}</strong>
          {#if run.explanation.advice}<span>{run.explanation.advice}</span>{/if}
        </p>
      {/if}
    </section>
    {#if unavailable}
      <p class="gone">Il registro di questa copia non è più disponibile.</p>
    {:else}
      {#if log.errorLines.length > 0}
        <button type="button" class="link jump" onclick={goToFirstError}>Vai al primo errore</button>
      {/if}
      <!-- svelte-ignore a11y_no_noninteractive_tabindex
        The <pre> is focusable so keyboard users can scroll the log; the error spans get tabindex="-1"
        only to receive focus from "Vai al primo errore". -->
      <pre tabindex="0">{#each segments as { text, error }, i (i)}{#if error}<span class="error-line" tabindex="-1"
              >{text}</span
            >{:else}{text}{/if}{/each}</pre>
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
    box-shadow: var(--shadow-win);
    border-radius: var(--r);
    overflow: hidden;
  }
  header {
    display: flex;
    gap: 12px;
    align-items: center;
    justify-content: space-between;
    padding: 22px 26px;
  }
  h2 {
    min-width: 0;
    overflow-wrap: anywhere;
    font-size: 20px;
  }
  .summary {
    display: grid;
    gap: 10px;
    padding: 0 26px 18px;
  }
  .chips {
    display: flex;
    gap: 8px;
  }
  .why {
    margin: 0;
  }
  .why strong {
    display: block;
  }
  .jump {
    align-self: flex-start;
    margin: 0 20px 8px;
  }
  .error-line {
    color: var(--on-danger-soft);
    background: var(--danger-soft);
    font-weight: 700;
  }
  pre {
    margin: 0 20px 20px;
    padding: 16px 18px;
    overflow: auto;
    font:
      13.5px/1.45 Consolas,
      "Cascadia Mono",
      monospace;
    background: var(--card-2);
    border-radius: var(--r-s);
    flex: 1;
    tab-size: 4;
  }
  .gone {
    margin: 0 26px 26px;
    color: var(--fg-2);
  }
</style>
