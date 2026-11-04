<script lang="ts">
  import { formatNumber, formatWhen, modeChip, outcomeChip } from "./format";
  import type { Run } from "./types";

  /** Newest first, as returned by `list_runs`. */
  let { runs }: { runs: Run[] } = $props();

  const succeeded = $derived(runs.filter((r) => r.outcome === "Ok"));
  const lastSuccess = $derived(succeeded[0]);
</script>

<h2>Come leggere lo storico</h2>
<div class="counts">
  <div class="cnt"><b>{formatNumber(runs.length)}</b><span>Copie fatte</span></div>
  <div class="cnt"><b>{formatNumber(succeeded.length)}</b><span>Riuscite</span></div>
</div>
<div class="legend">
  <p><span class="chip {outcomeChip.Ok.cls}">{outcomeChip.Ok.label}</span> La copia è andata a buon fine.</p>
  <p>
    <span class="chip {outcomeChip.Error.cls}">{outcomeChip.Error.label}</span> Interrotta per un problema (di solito un disco o la rete). I file già copiati restano.
  </p>
  <p><span class="chip {outcomeChip.Cancelled.cls}">{outcomeChip.Cancelled.label}</span> Fermata a mano.</p>
  <p><span class="chip {modeChip.Mirror.cls}">{modeChip.Mirror.label}</span> In quella copia erano stati cancellati da B i file in più.</p>
</div>
{#if lastSuccess}
  <p class="cap">Ultima copia riuscita: {formatWhen(lastSuccess.startedAt)}.</p>
{/if}

<style>
  .counts {
    display: grid;
    grid-auto-flow: column;
    grid-auto-columns: 1fr;
    gap: 8px;
    margin: 0 0 14px;
  }
  .cnt {
    text-align: center;
    padding: 12px 4px;
    background: var(--surface-container-lowest);
    min-width: 0;
  }
  .cnt b {
    display: block;
    font-size: 28px;
    line-height: 1.1;
    font-weight: 800;
  }
  .cnt span {
    display: block;
    font-size: 14px;
    color: var(--on-surface-variant);
    font-weight: 600;
    margin-top: 2px;
  }
  .legend {
    display: grid;
    gap: 10px;
    font-size: 15px;
    color: var(--on-surface-variant);
  }
  .legend p {
    display: flex;
    gap: 10px;
    align-items: flex-start;
    line-height: 1.4;
  }
  .legend .chip {
    flex: none;
    margin-top: 1px;
  }
  .cap {
    margin-top: 14px;
    font-size: 15.5px;
    text-align: center;
    color: var(--on-surface-variant);
    font-weight: 600;
  }
</style>
