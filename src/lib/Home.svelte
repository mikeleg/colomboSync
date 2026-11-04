<script lang="ts">
  import { onMount } from "svelte";
  import Icon from "./Icon.svelte";
  import { listSyncs } from "./api";
  import { formatWhen, isNetworkPath, modeChip, outcomeChip } from "./format";
  import { emptySchema, type SchemaModel } from "./schema";
  import type { Sync, SyncSummary } from "./types";

  let {
    onnew,
    onrun,
    onschema,
  }: { onnew: () => void; onrun: (sync: Sync) => void; onschema: (model: SchemaModel) => void } = $props();

  const CAPTION = 'Premi "Controlla e avvia": prima ti mostro cosa cambierà, poi decidi tu.';

  let syncs = $state<SyncSummary[]>([]);
  let selectedId = $state<number | null>(null);

  /** Set when syncs.json cannot be read: no list, and no new sync that would have to be saved over it. */
  let loadError = $state("");

  onMount(async () => {
    try {
      syncs = await listSyncs();
    } catch (e) {
      loadError = String(e);
    }
  });

  const selected = $derived(syncs.find((s) => s.id === selectedId) ?? syncs[0]);

  $effect(() => {
    onschema(
      selected
        ? {
            source: selected.source,
            destination: selected.destination,
            counts: null,
            caption: CAPTION,
            mirror: selected.mode === "Mirror",
            warnCaption: false,
          }
        : emptySchema,
    );
  });
</script>

{#snippet place(path: string)}
  <Icon name={isNetworkPath(path) ? "net" : "disk"} /><span>{path}</span>
{/snippet}

<h1 id="h">Le tue sincronizzazioni</h1>
<p class="lead">
  Ogni sincronizzazione copia i file da una cartella (A) in un’altra (B). Prima di copiare ti mostro sempre cosa cambierà.
</p>
{#if syncs.length > 0}
  <ul class="list">
    {#each syncs as sync (sync.id)}
      {@const retry = sync.lastRun?.outcome === "Error"}
      {@const outcome = sync.lastRun && outcomeChip[sync.lastRun.outcome]}
      <!-- svelte-ignore a11y_no_noninteractive_element_interactions, a11y_click_events_have_key_events -->
      <li
        class="card"
        class:sel={sync === selected}
        onclick={() => (selectedId = sync.id)}
        onfocusin={() => (selectedId = sync.id)}
      >
        <div class="head">
          <h2>{sync.name}</h2>
          <span class="chip {modeChip[sync.mode].cls}">{modeChip[sync.mode].label}</span>
        </div>
        <p class="route">
          {@render place(sync.source)}<span class="ar">&rarr;</span>{@render place(sync.destination)}
        </p>
        <div class="foot">
          <div class="last">
            {#if sync.lastRun && outcome}
              <span>Ultima volta: <span>{formatWhen(sync.lastRun.startedAt)}</span></span>
              <span class="chip {outcome.cls}">{outcome.label}</span>
            {:else}
              <span>Mai avviata</span>
            {/if}
          </div>
          <button
            type="button"
            class="btn primary"
            aria-label="{retry ? 'Riprova' : 'Controlla e avvia'} {sync.name}"
            onclick={() => onrun(sync)}
          >
            <Icon name="play" />{retry ? "Riprova" : "Controlla e avvia"}
          </button>
        </div>
      </li>
    {/each}
  </ul>
{/if}
{#if loadError}<div class="err-msg" role="alert">{loadError}</div>{/if}
<button type="button" class="newcard" disabled={!!loadError} onclick={onnew}><Icon name="plus" />Nuova sincronizzazione</button>

<style>
  .list {
    display: grid;
    gap: 14px;
    margin: 0 0 18px;
    padding: 0;
    list-style: none;
  }
  .card {
    border-radius: var(--r);
    padding: 20px 22px;
    background: var(--card-2);
    color: var(--fg);
    cursor: pointer;
    min-width: 0;
    border: 2px solid transparent;
  }
  .card:hover {
    background: color-mix(in srgb, var(--fg) 4%, var(--card-2));
  }
  .card.sel {
    border-color: var(--accent);
    background: color-mix(in srgb, var(--accent) 7%, var(--card-2));
  }
  .head {
    display: flex;
    flex-wrap: wrap;
    gap: 8px 12px;
    align-items: center;
    margin-bottom: 10px;
  }
  .head h2 {
    min-width: 0;
    overflow-wrap: anywhere;
    flex: 1;
  }
  .route {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px 10px;
    color: var(--fg-2);
    overflow-wrap: anywhere;
    margin-bottom: 16px;
    font-size: 15.5px;
    font-weight: 600;
  }
  .route :global(svg) {
    width: 18px;
    height: 18px;
    color: var(--accent);
    flex: none;
  }
  .ar {
    color: var(--fg-2);
    opacity: 0.6;
  }
  .foot {
    display: flex;
    flex-wrap: wrap;
    gap: 10px 16px;
    align-items: center;
    justify-content: space-between;
  }
  .last {
    display: flex;
    flex-wrap: wrap;
    gap: 6px 10px;
    align-items: center;
    min-width: 0;
    font-size: 15.5px;
    color: var(--fg-2);
  }
  .foot .btn :global(svg) {
    width: 18px;
    height: 18px;
  }
  .newcard {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 10px;
    width: 100%;
    min-height: 68px;
    border: 2px dashed color-mix(in srgb, var(--accent) 50%, transparent);
    border-radius: var(--r);
    background: none;
    color: var(--accent);
    font-size: 18px;
    font-weight: 800;
    cursor: pointer;
  }
  .newcard:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  .newcard:hover:not(:disabled) {
    background: color-mix(in srgb, var(--accent) 7%, transparent);
    border-style: solid;
  }
  .newcard :global(svg) {
    width: 22px;
    height: 22px;
  }
</style>
