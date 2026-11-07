<script lang="ts">
  import { onMount } from "svelte";
  import Icon from "../shared/Icon.svelte";
  import { listSyncs } from "./api";
  import { formatWhen, isNetworkPath, modeChip, outcomeChip } from "../shared/format";
  import { emptySchema, type SchemaModel } from "../schema/schema";
  import type { Sync } from "../shared/types";
  import type { SyncSummary } from "./types";

  let {
    onnew,
    onrun,
    onschema,
  }: { onnew: () => void; onrun: (sync: Sync) => void; onschema: (model: SchemaModel) => void } = $props();

  const CAPTION = 'Premi "Controlla e avvia": prima ti mostro cosa cambierà, poi decidi tu.';

  let syncs = $state<SyncSummary[]>([]);
  let selectedId = $state<number | null>(null);

  /** Set when syncs.json cannot be read nor set aside: no list, and no new sync that would have to be saved over it. */
  let loadError = $state("");
  /** A damaged syncs.json was renamed aside: the list is empty and the user is told. */
  let setAside = $state(false);

  onMount(async () => {
    try {
      const listed = await listSyncs();
      syncs = listed.syncs;
      setAside = listed.setAside;
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
  Ogni sincronizzazione copia i file da una cartella (A) in un’altra (B). Prima di copiare ti mostro sempre cosa
  cambierà.
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
        class:bad={retry}
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
        {#if sync.lastRun?.note}<p class="note" class:err={retry}>{sync.lastRun.note}</p>{/if}
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
            class="btn"
            class:primary={!retry}
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
{#if setAside}<p class="note aside">
    Le sincronizzazioni salvate non erano leggibili: ne ho messo da parte una copia.
  </p>{/if}
{#if loadError}<div class="err-msg" role="alert">{loadError}</div>{/if}
<button type="button" class="newcard" disabled={!!loadError} onclick={onnew}
  ><Icon name="plus" />Nuova sincronizzazione</button
>

<style>
  .list {
    display: grid;
    gap: 20px;
    margin: 0 0 20px;
    padding: 0;
    list-style: none;
  }
  .card {
    border-radius: var(--r);
    padding: 28px;
    background: var(--card);
    box-shadow: var(--shadow);
    color: var(--fg);
    cursor: pointer;
    min-width: 0;
  }
  .card:hover {
    background: color-mix(in srgb, var(--fg) 3%, var(--card));
  }
  .card.bad {
    box-shadow: inset 0 0 0 1.5px color-mix(in srgb, var(--danger) 45%, transparent);
  }
  .card.sel {
    box-shadow: var(--shadow-sel);
  }
  .card.bad.sel {
    box-shadow:
      inset 0 0 0 1.5px var(--danger),
      0 30px 60px -40px var(--danger);
  }
  .head {
    display: flex;
    flex-wrap: wrap;
    gap: 8px 12px;
    align-items: center;
    margin-bottom: 14px;
  }
  .head h2 {
    min-width: 0;
    overflow-wrap: anywhere;
    flex: 1;
    font-size: 28px;
    letter-spacing: -0.025em;
  }
  .route {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px 10px;
    color: var(--fg-2);
    overflow-wrap: anywhere;
    margin-bottom: 18px;
    font-size: 16px;
  }
  .route span {
    color: var(--fg);
    font-weight: 600;
  }
  .route :global(svg) {
    width: 18px;
    height: 18px;
    color: var(--accent);
    flex: none;
  }
  .route .ar {
    color: var(--fg-2);
    font-weight: 400;
  }
  .note {
    margin: -4px 0 18px;
    padding: 12px 16px;
    border-radius: var(--r-s);
    background: var(--card-2);
    color: var(--fg-2);
    font-size: 15px;
    font-weight: 600;
  }
  .note.aside {
    margin: 0 0 14px;
  }
  .note.err {
    background: var(--danger-soft);
    color: var(--on-danger-soft);
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
    font-size: 15px;
    color: var(--fg-2);
  }
  .newcard {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 10px;
    width: 100%;
    min-height: 76px;
    border: 1.5px dashed color-mix(in srgb, var(--accent) 45%, transparent);
    border-radius: var(--r);
    background: none;
    color: var(--accent);
    font-size: 19px;
    font-weight: 700;
    cursor: pointer;
  }
  .newcard:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  .newcard:hover:not(:disabled) {
    background: color-mix(in srgb, var(--accent) 8%, transparent);
    border-style: solid;
  }
  .newcard :global(svg) {
    width: 22px;
    height: 22px;
  }
</style>
