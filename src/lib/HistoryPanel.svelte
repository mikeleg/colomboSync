<script lang="ts">
  import Icon from "./Icon.svelte";
  import { formatNumber, formatWhen, modeChip, outcomeChip } from "./format";
  import type { HistorySummary } from "./types";

  /** From `history_summary`; null while it loads or when it failed: then only the legend shows. */
  let { summary }: { summary: HistorySummary | null } = $props();

  const numbers = $derived(
    summary && [
      ["Copie fatte", summary.runs],
      ["Riuscite", summary.ok],
      ["Con errore", summary.failed],
      ["Annullate", summary.cancelled],
      ["File copiati", summary.filesCopied],
      ["File cancellati in B", summary.filesDeleted],
    ] as const,
  );
</script>

<div class="panel">
  {#if summary && numbers}
    {#if summary.runs === 0}
      <p class="empty">
        Non hai ancora fatto nessuna copia. Quando ne farai una, qui vedrai i numeri e lo stato di ogni sincronizzazione.
      </p>
    {:else}
      <section>
        <h2>Riepilogo</h2>
        <div class="counts">
          {#each numbers as [label, value]}
            <div class="cnt"><b>{formatNumber(value)}</b><span>{label}</span></div>
          {/each}
        </div>
      </section>
      {#if summary.syncs.length > 0}
        <section>
          <h2>Per sincronizzazione</h2>
          <ul class="syncs">
            {#each summary.syncs as sync (sync.id)}
              <li>
                <strong>{sync.name}</strong>
                <span class="last">
                  {sync.lastSuccess ? `Ultima copia riuscita: ${formatWhen(sync.lastSuccess)}` : "Nessuna copia riuscita finora."}
                </span>
                {#if sync.warning}
                  <span class="warn"><Icon name="warn" />{sync.warning}</span>
                {/if}
              </li>
            {/each}
          </ul>
        </section>
      {/if}
    {/if}
  {/if}
  <section class="legend">
    <h2>Cosa vogliono dire le etichette</h2>
    <p><span class="chip {outcomeChip.Ok.cls}">{outcomeChip.Ok.label}</span> La copia è andata a buon fine.</p>
    <p>
      <span class="chip {outcomeChip.Error.cls}">{outcomeChip.Error.label}</span> Interrotta per un problema (di solito un disco o la rete). I file già copiati restano.
    </p>
    <p><span class="chip {outcomeChip.Cancelled.cls}">{outcomeChip.Cancelled.label}</span> Fermata a mano.</p>
    <p><span class="chip {modeChip.Mirror.cls}">{modeChip.Mirror.label}</span> In quella copia erano stati cancellati da B i file in più.</p>
  </section>
</div>

<style>
  /* A long list of syncs scrolls inside the panel instead of pushing it off the window. */
  .panel {
    display: grid;
    gap: 24px;
    max-height: calc(100vh - 120px);
    overflow-y: auto;
  }
  h2 {
    font-size: 13px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--fg-2);
    margin-bottom: 12px;
  }
  .empty {
    font-size: 15px;
    line-height: 1.4;
    color: var(--fg-2);
  }
  .counts {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 8px;
  }
  .cnt {
    text-align: center;
    padding: 10px 6px;
    border-radius: 16px;
    background: var(--card-2);
    min-width: 0;
  }
  .cnt b {
    display: block;
    font-size: 26px;
    line-height: 1.1;
    font-weight: 700;
    letter-spacing: -0.03em;
  }
  .cnt span {
    display: block;
    font-size: 13px;
    color: var(--fg-2);
    margin-top: 2px;
  }
  .syncs {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 8px;
  }
  .syncs li {
    display: grid;
    gap: 2px;
    padding: 10px 14px;
    border-radius: 16px;
    background: var(--card-2);
    min-width: 0;
  }
  .syncs strong {
    overflow-wrap: anywhere;
  }
  .last {
    font-size: 14px;
    color: var(--fg-2);
  }
  .warn {
    display: flex;
    gap: 8px;
    align-items: flex-start;
    margin-top: 6px;
    padding: 8px 10px;
    border-radius: 12px;
    background: var(--danger-soft);
    color: var(--on-danger-soft);
    font-size: 14px;
    font-weight: 600;
    line-height: 1.35;
  }
  .warn :global(svg) {
    width: 18px;
    height: 18px;
    flex: none;
  }
  .legend {
    display: grid;
    gap: 8px;
    padding-top: 18px;
    border-top: 1px solid var(--line);
    font-size: 13px;
    color: var(--fg-2);
  }
  .legend h2 {
    margin-bottom: 4px;
  }
  .legend p {
    display: flex;
    gap: 10px;
    align-items: flex-start;
    line-height: 1.4;
  }
  .legend .chip {
    flex: none;
    font-size: 12px;
    padding: 2px 10px;
  }
</style>
