<script lang="ts">
  import Icon from "../Icon.svelte";
  import { formatDuration, formatNumber } from "../format";
  import type { Run } from "../types";

  let { run, failure, onhome }: { run: Run | null; failure: string; onhome: () => void } = $props();

  const ok = $derived(run?.outcome === "Ok");
  const deletedText = $derived(
    run && run.mode === "Mirror" && run.deleted > 0
      ? `, ${formatNumber(run.deleted)} ${run.deleted === 1 ? "cancellato" : "cancellati"} da B`
      : "",
  );
  const title = $derived(
    run && ok
      ? `Fatto: ${formatNumber(run.new)} ${run.new === 1 ? "file copiato" : "file copiati"}, ${formatNumber(run.updated)} ${run.updated === 1 ? "aggiornato" : "aggiornati"}${deletedText} in ${formatDuration(run.durationMs)}`
      : "Non completato",
  );
</script>

<div class="kicker">Risultato</div>
<div class="res" class:err={!ok} role="status">
  <Icon name={ok ? "check" : "cross"} />
  <div>
    <h1 id="h">{title}</h1>
    {#if ok}
      <p>B ora è aggiornato.</p>
    {:else}
      <p>{run?.error ?? failure}</p>
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
    padding: 24px;
    border-radius: var(--r);
    background: var(--success-container);
    color: var(--on-success-container);
    margin-bottom: 22px;
  }
  .res.err {
    background: var(--error-container);
    color: var(--on-error-container);
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
