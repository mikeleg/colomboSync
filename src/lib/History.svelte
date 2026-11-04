<script lang="ts">
  import { formatDuration, formatNumber, formatWhen } from "./format";
  import LogViewer from "./LogViewer.svelte";
  import type { Outcome, Run } from "./types";

  /** Newest first, as returned by `list_runs`. */
  let { runs }: { runs: Run[] } = $props();

  let viewing = $state<Run | null>(null);

  const outcomeChip: Record<Outcome, { cls: string; label: string }> = {
    Ok: { cls: "ok", label: "OK" },
    Error: { cls: "err", label: "Errore" },
    Cancelled: { cls: "ann", label: "Annullato" },
  };
</script>

<h1 id="h">Storico</h1>
<p class="lead">Tutte le copie fatte, dalla più recente. Ogni riga ha il suo registro completo.</p>
<ul class="runs">
  {#each runs as run}
    <li class="run">
      <div class="top">
        <strong>{run.syncName}</strong>
        <span class="when">{formatWhen(run.startedAt)}</span>
        <span class="chips">
          <span class="chip {run.mode === 'Mirror' ? 'mirror' : 'copy'}">{run.mode === "Mirror" ? "Specchio" : "Copia"}</span>
          <span class="chip {outcomeChip[run.outcome].cls}">{outcomeChip[run.outcome].label}</span>
        </span>
      </div>
      <dl class="stats">
        <div><dt>Nuovi</dt><dd>{formatNumber(run.new)}</dd></div>
        <div><dt>Aggiornati</dt><dd>{formatNumber(run.updated)}</dd></div>
        <div class:d={run.deleted > 0}><dt>Cancellati</dt><dd>{formatNumber(run.deleted)}</dd></div>
        <div><dt>Durata</dt><dd>{formatDuration(run.durationMs)}</dd></div>
        <div class="lg"><button type="button" class="link" onclick={() => (viewing = run)}>Registro</button></div>
      </dl>
    </li>
  {/each}
</ul>
{#if viewing}
  <LogViewer run={viewing} onclose={() => (viewing = null)} />
{/if}

<style>
  .runs {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 12px;
  }
  .run {
    background: var(--surface-container-low);
    padding: 18px 22px;
    min-width: 0;
  }
  .top {
    display: flex;
    flex-wrap: wrap;
    gap: 6px 12px;
    align-items: center;
    margin-bottom: 10px;
  }
  .top strong {
    min-width: 0;
    overflow-wrap: anywhere;
    font-weight: 800;
    font-size: 17.5px;
  }
  .when {
    color: var(--on-surface-variant);
    font-size: 15px;
  }
  .chips {
    display: flex;
    gap: 8px;
    margin-left: auto;
  }
  .stats {
    display: flex;
    flex-wrap: wrap;
    gap: 6px 22px;
    margin: 0;
    align-items: center;
  }
  .stats div {
    min-width: 0;
  }
  dt {
    display: inline;
    color: var(--on-surface-variant);
    font-size: 15px;
  }
  dt::after {
    content: " ";
  }
  dd {
    display: inline;
    margin: 0;
    font-weight: 800;
  }
  .d dd {
    color: var(--error);
  }
  .lg {
    margin-left: auto;
  }
</style>
