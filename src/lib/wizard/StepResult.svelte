<script lang="ts">
  import Icon from "../Icon.svelte";
  import { formatDuration, formatNumber, partialText } from "../format";
  import type { Run } from "../types";

  let { run, failure, onhome }: { run: Run | null; failure: string; onhome: () => void } = $props();

  const ok = $derived(run?.outcome === "Ok");
  const cancelled = $derived(run?.outcome === "Cancelled");
  const copied = $derived(run?.partial ?? 0);
  const title = $derived(
    run && ok
      ? `Fatto: ${formatNumber(run.new)} ${run.new === 1 ? "file copiato" : "file copiati"}, ${formatNumber(run.updated)} ${run.updated === 1 ? "aggiornato" : "aggiornati"} in ${formatDuration(run.durationMs)}`
      : cancelled
        ? "Annullato: ho fermato la copia."
        : "Non completato",
  );
</script>

<div class="kicker">Risultato</div>
<div class="res" class:err={!ok && !cancelled} class:ann={cancelled} role="status">
  <Icon name={ok ? "check" : cancelled ? "stop" : "cross"} />
  <div>
    <h1 id="h">{title}</h1>
    {#if ok}
      <p>B ora è aggiornato.</p>
    {:else if cancelled}
      <p>{copied === 1 ? "Il file già copiato resta" : `I ${formatNumber(copied)} file già copiati restano`} in B.</p>
    {:else}
      <p>{run?.error ?? failure}</p>
    {/if}
    {#if run?.partial !== undefined}
      <p>{partialText(run.partial)}</p>
    {/if}
  </div>
</div>
<div class="btns">
  <button type="button" class="btn primary big" onclick={onhome}>Torna all’inizio</button>
</div>

<style>
  .res {
    display: flex;
    gap: 16px;
    align-items: flex-start;
    padding: 22px;
    background: var(--success-container);
    color: var(--on-success-container);
    margin-bottom: 22px;
  }
  .res.err {
    background: var(--error-container);
    color: var(--on-error-container);
  }
  .res.ann {
    background: var(--warning-container);
    color: var(--on-warning-container);
  }
  .res :global(svg) {
    width: 52px;
    height: 52px;
    flex: none;
    color: var(--success);
  }
  .res.err :global(svg) {
    color: var(--error);
  }
  .res.ann :global(svg) {
    color: var(--warning);
  }
  .res > div {
    min-width: 0;
  }
  .res h1 {
    font-size: 26px;
  }
  .res p {
    overflow-wrap: anywhere;
  }
</style>
