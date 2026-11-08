<script lang="ts">
  import Icon from "../shared/Icon.svelte";
  import LogViewer from "../shared/LogViewer.svelte";
  import type { Explanation, RunReport } from "../shared/types";

  let {
    run,
    failure,
    onhome,
    onretry,
    onhistory,
  }: {
    run: RunReport | null;
    /** Why no Run exists (Rust refused it, or the sync could not be saved). */
    failure: Explanation | null;
    onhome: () => void;
    onretry: () => void;
    onhistory: () => void;
  } = $props();

  let viewingLog = $state(false);

  const ok = $derived(run?.outcome === "Ok");
  const cancelled = $derived(run?.outcome === "Cancelled");
  // What Rust says about the Run, or about the command that failed before any Run existed.
  const title = $derived(run?.title ?? failure?.title);
  const advice = $derived(run?.explanation?.advice ?? failure?.advice);
</script>

<div class="kicker">Risultato</div>
<div class="res" class:err={!ok && !cancelled} class:ann={cancelled} role="status">
  <Icon name={ok ? "check" : cancelled ? "stop" : "cross"} />
  <div>
    <h1 id="h">{title}</h1>
    {#if ok}
      <p>B ora è aggiornato.</p>
    {:else if !cancelled && advice}
      <p>{advice}</p>
    {/if}
    {#if !ok && run?.resultNote}<p>{run.resultNote}</p>{/if}
  </div>
</div>
<div class="btns">
  {#if !ok}<button type="button" class="btn primary big" onclick={onretry}>Riprova</button>{/if}
  <button type="button" class="btn big" class:primary={ok} onclick={onhome}>Torna all’elenco</button>
  {#if run}<button type="button" class="link" onclick={() => (viewingLog = true)}>Apri il registro</button>{/if}
  <button type="button" class="link" onclick={onhistory}>Vai allo storico</button>
</div>
{#if run && viewingLog}
  <LogViewer {run} onclose={() => (viewingLog = false)} />
{/if}

<style>
  .res {
    display: flex;
    gap: 16px;
    align-items: flex-start;
    padding: 24px;
    border-radius: var(--r);
    background: var(--ok-soft);
    color: var(--on-ok-soft);
    margin-bottom: 22px;
  }
  .res.err {
    background: var(--danger-soft);
    color: var(--on-danger-soft);
  }
  .res.ann {
    background: var(--warn-soft);
    color: var(--on-warn-soft);
  }
  .res :global(svg) {
    width: 52px;
    height: 52px;
    flex: none;
    color: var(--ok);
  }
  .res.err :global(svg) {
    color: var(--danger);
  }
  .res.ann :global(svg) {
    color: var(--warn);
  }
  .res > div {
    min-width: 0;
  }
  .res h1 {
    font-size: 34px;
  }
  .res p {
    overflow-wrap: anywhere;
  }
</style>
