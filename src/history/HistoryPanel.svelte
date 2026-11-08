<script lang="ts">
  import Icon from "../shared/Icon.svelte";
  import { formatNumber, modeChip, outcomeChip } from "../shared/format";
  import type { HistorySummary } from "./types";

  /** From `list_runs`; null while it loads or when the history could not be read: then only the legend shows. */
  let { summary }: { summary: HistorySummary | null } = $props();
</script>

{#snippet count(label: string, value: number)}
  <div class="cnt"><b>{formatNumber(value)}</b><span>{label}</span></div>
{/snippet}

<div class="panel">
  {#if summary}
    {#if summary.runs === 0}
      <p class="empty">
        Non hai ancora fatto nessuna copia. Quando ne farai una, qui vedrai i numeri e lo stato di ogni
        sincronizzazione.
      </p>
    {:else}
      <section>
        <h2 class="section-title">Riepilogo</h2>
        <div class="counts">
          {@render count("Copie fatte", summary.runs)}
          {@render count("Riuscite", summary.ok)}
          {@render count("Con errore", summary.failed)}
          {@render count("Annullate", summary.cancelled)}
          {@render count("File copiati", summary.filesCopied)}
          {@render count("File cancellati in B", summary.filesDeleted)}
        </div>
      </section>
      {#if summary.syncs.length > 0}
        <section>
          <h2 class="section-title">Per sincronizzazione</h2>
          <ul class="syncs">
            {#each summary.syncs as sync (sync.id)}
              <li>
                <strong>{sync.name}</strong>
                <span class="last">{sync.lastSuccessText}</span>
                {#if sync.warning}
                  <span class="danger-note"><Icon name="warn" />{sync.warning}</span>
                {/if}
              </li>
            {/each}
          </ul>
        </section>
      {/if}
    {/if}
  {/if}
  <section class="legend">
    <h2 class="section-title">Cosa vogliono dire le etichette</h2>
    <p><span class="chip {outcomeChip.Ok.cls}">{outcomeChip.Ok.label}</span> La copia è andata a buon fine.</p>
    <p>
      <span class="chip {outcomeChip.Error.cls}">{outcomeChip.Error.label}</span> Interrotta per un problema (di solito un
      disco o la rete). I file già copiati restano.
    </p>
    <p><span class="chip {outcomeChip.Cancelled.cls}">{outcomeChip.Cancelled.label}</span> Fermata a mano.</p>
    <p>
      <span class="chip {modeChip.Mirror.cls}">{modeChip.Mirror.label}</span> In quella copia erano stati cancellati da B
      i file in più.
    </p>
  </section>
</div>

<style>
  .panel {
    display: grid;
    gap: 24px;
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
  .syncs .danger-note {
    margin-top: 6px;
  }
  .legend {
    display: grid;
    gap: 8px;
    padding-top: 18px;
    border-top: 1px solid var(--line);
    font-size: 13px;
    color: var(--fg-2);
  }
  .legend .section-title {
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
