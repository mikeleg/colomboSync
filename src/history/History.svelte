<script lang="ts">
  import ErrorText from "../shared/ErrorText.svelte";
  import { modeChip, outcomeChip } from "../shared/format";
  import LogViewer from "../shared/LogViewer.svelte";
  import RunChips from "../shared/RunChips.svelte";
  import RunStats from "../shared/RunStats.svelte";
  import type { RunReport } from "../shared/types";
  import type { HistoryState } from "./types";

  /** `list_runs` as the app holds it: runs newest first, `summary.syncs` the saved syncs; `failed` says why it failed. */
  let { history }: { history: HistoryState } = $props();

  const status = $derived(history.status === "loading" ? "loading" : "ready");
  const error = $derived(history.status === "failed" ? history.error : null);
  const runs = $derived(history.status === "ready" ? history.runs : []);
  const syncs = $derived(history.status === "ready" ? history.summary.syncs : []);

  /** How many runs show at first, and how many more each "Mostra altre" adds. */
  const PAGE = 200;
  // "" = no filter. Kept here, so they survive the Registro modal; not stored anywhere.
  const NO_FILTERS = { sync: "", outcome: "", mode: "", period: "" };

  let viewing = $state.raw<RunReport | null>(null);
  let filters = $state({ ...NO_FILTERS });
  // Other runs or another filter start the list again from the first page; "Mostra altre" then adds to it.
  let limit = $derived.by(() => {
    void [runs, ...Object.values(filters)];
    return PAGE;
  });

  /** Id of each name used by a run that has one (newest wins), so old lines without an id join their sync. */
  const idByName = $derived(
    new Map([...runs].reverse().flatMap((r): [string, number][] => (r.syncId != null ? [[r.syncName, r.syncId]] : []))),
  );

  /** A run belongs to its saved sync by id; old lines without one, by the id of a run with the same name, else by name. */
  const syncKey = (run: RunReport) => {
    const id = run.syncId ?? idByName.get(run.syncName);
    return id != null ? `id:${id}` : `name:${run.syncName}`;
  };

  const syncOptions = $derived.by(() => {
    const options = new Map(syncs.map((s): [string, string] => [`id:${s.id}`, s.name]));
    for (const run of runs) {
      const key = syncKey(run);
      if (key.startsWith("name:")) options.set(key, run.syncName);
    }
    return [...options].sort((a, b) => a[1].localeCompare(b[1], "it"));
  });
  const filtered = $derived.by(() => {
    const { sync, outcome, mode, period } = filters;
    const since = period ? Date.now() - Number(period) * 86_400_000 : 0;
    return (
      runs
        // The history only grows at the front: counting from the oldest run keeps a run's key when new ones arrive.
        .map((run, i) => ({ run, key: runs.length - i }))
        .filter(
          ({ run: r }) =>
            (!sync || syncKey(r) === sync) &&
            (!outcome || r.outcome === outcome) &&
            (!mode || r.mode === mode) &&
            (!period || Date.parse(r.startedAt) >= since),
        )
    );
  });
  const shown = $derived(filtered.slice(0, limit));
  const isFiltered = $derived(Object.values(filters).some(Boolean));

  function clearFilters() {
    filters = { ...NO_FILTERS };
  }
</script>

<h1 id="h">Storico</h1>
<p class="lead">Tutte le copie fatte, dalla più recente. Ogni riga ha il suo registro completo.</p>
{#if status === "loading"}
  <p class="muted" role="status">Sto leggendo lo storico…</p>
{:else if error}
  <p class="err-msg" role="alert"><ErrorText explanation={error} /></p>
{/if}
{#if runs.length > 0}
  <div class="filters">
    <label>
      Sincronizzazione
      <select bind:value={filters.sync}>
        <option value="">Tutte</option>
        {#each syncOptions as [key, name] (key)}
          <option value={key}>{name}</option>
        {/each}
      </select>
    </label>
    <label>
      Esito
      <select bind:value={filters.outcome}>
        <option value="">Tutti</option>
        {#each Object.entries(outcomeChip) as [outcome, chip] (outcome)}
          <option value={outcome}>{chip.label}</option>
        {/each}
      </select>
    </label>
    <label>
      Modalità
      <select bind:value={filters.mode}>
        <option value="">Tutte</option>
        {#each Object.entries(modeChip) as [mode, chip] (mode)}
          <option value={mode}>{chip.label}</option>
        {/each}
      </select>
    </label>
    <label>
      Periodo
      <select bind:value={filters.period}>
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
  {#each shown as { run, key } (key)}
    <li class="run">
      <div class="top">
        <strong>{run.syncName}</strong>
        <span class="when">{run.when}</span>
        <span class="chips">
          <RunChips {run} />
        </span>
      </div>
      <RunStats {run}>
        {#snippet extra()}
          <div class="lg"><button type="button" class="link" onclick={() => (viewing = run)}>Registro</button></div>
        {/snippet}
      </RunStats>
    </li>
  {/each}
</ul>
{#if filtered.length > limit}
  <div class="btns more">
    <button type="button" class="btn" onclick={() => (limit += PAGE)}>Mostra altre</button>
  </div>
{/if}
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
  .more {
    margin-top: 18px;
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
  .lg {
    min-width: 0;
    margin-left: auto;
  }
</style>
