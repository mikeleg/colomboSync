<script lang="ts">
  import { formatDuration, formatNumber, formatWhen, modeChip, outcomeChip, partialText } from "../shared/format";
  import LogViewer from "./LogViewer.svelte";
  import type { Run } from "../shared/types";

  /** Newest first, as returned by `list_runs`. */
  let { runs, status }: { runs: Run[]; status: "loading" | "ready" | "error" } = $props();

  let viewing = $state<Run | null>(null);

  // "" = no filter. Kept here, so they survive the Registro modal; not stored anywhere.
  let syncFilter = $state("");
  let outcomeFilter = $state("");
  let modeFilter = $state("");
  let periodDays = $state("");

  /** Id of each name used by a run that has one (newest wins), so old lines without an id join their sync. */
  const idByName = $derived(
    new Map([...runs].reverse().flatMap((r): [string, number][] => (r.syncId != null ? [[r.syncName, r.syncId]] : []))),
  );

  /** A run belongs to its saved sync by id; old lines without one, by the id of a run with the same name, else by name. */
  const syncKey = (run: Run) => {
    const id = run.syncId ?? idByName.get(run.syncName);
    return id != null ? `id:${id}` : `name:${run.syncName}`;
  };

  const syncOptions = $derived(
    // runs are newest first: a renamed sync shows under its latest name.
    [...new Map([...runs].reverse().map((r): [string, string] => [syncKey(r), r.syncName]))].sort((a, b) =>
      a[1].localeCompare(b[1], "it"),
    ),
  );
  const filtered = $derived.by(() => {
    const since = Date.now() - Number(periodDays) * 86_400_000;
    return (
      runs
        // The history only grows at the front: counting from the oldest run keeps a run's key when new ones arrive.
        .map((run, i) => ({ run, key: runs.length - i }))
        .filter(
          ({ run: r }) =>
            (!syncFilter || syncKey(r) === syncFilter) &&
            (!outcomeFilter || r.outcome === outcomeFilter) &&
            (!modeFilter || r.mode === modeFilter) &&
            (!periodDays || Date.parse(r.startedAt) >= since),
        )
    );
  });
  const isFiltered = $derived(!!(syncFilter || outcomeFilter || modeFilter || periodDays));

  function clearFilters() {
    syncFilter = outcomeFilter = modeFilter = periodDays = "";
  }
</script>

<h1 id="h">Storico</h1>
<p class="lead">Tutte le copie fatte, dalla più recente. Ogni riga ha il suo registro completo.</p>
{#if status === "loading"}
  <p class="muted" role="status">Sto leggendo lo storico…</p>
{:else if status === "error"}
  <p class="err-msg" role="alert">Non riesco a leggere lo storico.</p>
{/if}
{#if runs.length > 0}
  <div class="filters">
    <label>
      Sincronizzazione
      <select bind:value={syncFilter}>
        <option value="">Tutte</option>
        {#each syncOptions as [key, name] (key)}
          <option value={key}>{name}</option>
        {/each}
      </select>
    </label>
    <label>
      Esito
      <select bind:value={outcomeFilter}>
        <option value="">Tutti</option>
        <option value="Ok">{outcomeChip.Ok.label}</option>
        <option value="Error">{outcomeChip.Error.label}</option>
        <option value="Cancelled">{outcomeChip.Cancelled.label}</option>
      </select>
    </label>
    <label>
      Modalità
      <select bind:value={modeFilter}>
        <option value="">Tutte</option>
        <option value="Copy">{modeChip.Copy.label}</option>
        <option value="Mirror">{modeChip.Mirror.label}</option>
      </select>
    </label>
    <label>
      Periodo
      <select bind:value={periodDays}>
        <option value="">Sempre</option>
        <option value="7">Ultimi 7 giorni</option>
        <option value="30">Ultimi 30 giorni</option>
      </select>
    </label>
  </div>
  {#if isFiltered}
    <p class="found">
      <span role="status">{filtered.length} {filtered.length === 1 ? "copia" : "copie"} su {runs.length}</span>
      <button type="button" class="link" onclick={clearFilters}>Togli i filtri</button>
    </p>
    {#if filtered.length === 0}
      <p class="muted">Nessuna copia con questi filtri.</p>
    {/if}
  {/if}
{/if}
<ul class="runs">
  {#each filtered as { run, key } (key)}
    <li class="run">
      <div class="top">
        <strong>{run.syncName}</strong>
        <span class="when">{formatWhen(run.startedAt)}</span>
        <span class="chips">
          <span class="chip {modeChip[run.mode].cls}">{modeChip[run.mode].label}</span>
          <span class="chip {outcomeChip[run.outcome].cls}">{outcomeChip[run.outcome].label}</span>
        </span>
      </div>
      <dl class="stats">
        {#if run.partial != null}
          <div><dd>{partialText(run.partial)}</dd></div>
        {:else}
          <div>
            <dt>Nuovi</dt>
            <dd>{formatNumber(run.new)}</dd>
          </div>
          <div>
            <dt>Aggiornati</dt>
            <dd>{formatNumber(run.updated)}</dd>
          </div>
        {/if}
        <div class:d={run.deleted > 0}>
          <dt>Cancellati</dt>
          <dd>{formatNumber(run.deleted)}</dd>
        </div>
        <div>
          <dt>Durata</dt>
          <dd>{formatDuration(run.durationMs)}</dd>
        </div>
        <div class="lg"><button type="button" class="link" onclick={() => (viewing = run)}>Registro</button></div>
      </dl>
    </li>
  {/each}
</ul>
{#if viewing}
  <LogViewer run={viewing} onclose={() => (viewing = null)} />
{/if}

<style>
  .filters {
    display: flex;
    flex-wrap: wrap;
    gap: 12px 18px;
    margin-bottom: 18px;
  }
  .filters label {
    display: grid;
    gap: 4px;
    min-width: 0;
    font-size: 14px;
    color: var(--fg-2);
  }
  select {
    min-height: 44px;
    max-width: 100%;
    padding: 6px 14px;
    border: 0;
    border-radius: 999px;
    background: var(--card);
    box-shadow: var(--shadow);
    font-size: 16px;
    color: var(--fg);
  }
  .found {
    display: flex;
    flex-wrap: wrap;
    gap: 6px 12px;
    align-items: center;
    margin-bottom: 14px;
    color: var(--fg-2);
  }
  .runs {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 14px;
  }
  .run {
    background: var(--card);
    box-shadow: var(--shadow);
    border-radius: var(--r);
    padding: 22px 26px;
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
    font-weight: 700;
    font-size: 20px;
    letter-spacing: -0.02em;
  }
  .when {
    color: var(--fg-2);
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
    color: var(--fg-2);
    font-size: 15px;
  }
  dt::after {
    content: " ";
  }
  dd {
    display: inline;
    margin: 0;
    font-weight: 700;
  }
  .d dd {
    color: var(--danger);
  }
  .lg {
    margin-left: auto;
  }
</style>
