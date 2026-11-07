<script lang="ts">
  import { tick } from "svelte";
  import ErrorText from "../shared/ErrorText.svelte";
  import { toExplanation } from "../shared/explanation";
  import Icon from "../shared/Icon.svelte";
  import { browseFolders, createFolder } from "./api";
  import type { Explanation } from "../shared/types";
  import type { FolderListing } from "./types";

  let {
    id,
    label,
    placeholder,
    value = $bindable(),
    error,
    onenter,
    canCreate = false,
    source = "",
  }: {
    id: string;
    label: string;
    placeholder: string;
    value: string;
    error: Explanation | null;
    onenter: () => void;
    /** Offers "Nuova cartella" inside the folder being browsed. */
    canCreate?: boolean;
    /** The folder A: Rust refuses to create a new folder that would not be a valid B next to it. */
    source?: string;
  } = $props();

  let listing = $state<FolderListing>({ parent: "", folders: [], error: null });
  let manualOpen = $state(false);
  let head: HTMLElement;
  let input: HTMLInputElement;
  let lastLoad = 0;
  /** The folder the name box was opened in: the box belongs to it and goes away with any other choice. */
  let creatingIn = $state<string | null>(null);
  let newName = $state("");
  let createError = $state<Explanation | null>(null);
  let nameInput = $state<HTMLInputElement>();
  let createBusy = false;

  const chosen = $derived(value.trim());
  const creating = $derived(creatingIn === chosen);

  async function load(path: string) {
    const seq = ++lastLoad;
    let result: FolderListing;
    try {
      result = await browseFolders(path);
    } catch (e) {
      result = { parent: "", folders: [], error: toExplanation(e).title };
    }
    if (seq === lastLoad) listing = result;
  }

  function pick(path: string) {
    stopCreating();
    value = path;
    head?.focus();
  }

  async function startCreating() {
    creatingIn = chosen;
    await tick();
    nameInput?.focus();
  }

  function stopCreating() {
    creatingIn = null;
    newName = "";
    createError = null;
  }

  async function create() {
    if (createBusy) return;
    createBusy = true;
    try {
      pick(await createFolder(source, chosen, newName));
    } catch (e) {
      createError = toExplanation(e);
    } finally {
      createBusy = false;
    }
  }

  // The rows and the parent always follow the chosen path, however it got there (click or typing).
  $effect(() => {
    load(chosen);
  });

  $effect(() => {
    if (error) {
      manualOpen = true;
      tick().then(() => input.focus());
    }
  });
</script>

<p class="f">{label}</p>
<p class="help intro">Ogni clic entra nella cartella e la sceglie. Quando sei in quella giusta, premi Avanti.</p>
<div class="pick">
  <div class="pickhead" class:chosen={chosen && !listing.error} bind:this={head} tabindex="-1">
    <span>
      {#if !chosen}
        Dischi del computer: scegline uno
      {:else if listing.error}
        <b>{chosen}</b>
      {:else}
        <Icon name="tick" />Cartella scelta: <b>{chosen}</b>
      {/if}
    </span>
    {#if chosen}
      <div class="acts">
        {#if canCreate && !listing.error && !creating}
          <button type="button" class="btn small" onclick={startCreating}><Icon name="plus" /> Nuova cartella</button>
        {/if}
        <button type="button" class="btn small" onclick={() => pick(listing.parent)}
          ><Icon name="up" /> Livello superiore</button
        >
      </div>
    {/if}
  </div>
  {#if creating}
    <div class="newfolder">
      <label class="f" for="{id}-new">Nome della nuova cartella</label>
      <div class="newrow">
        <input
          type="text"
          id="{id}-new"
          bind:value={newName}
          bind:this={nameInput}
          placeholder="Per esempio: Backup"
          autocomplete="off"
          spellcheck="false"
          aria-invalid={createError ? "true" : "false"}
          onkeydown={(e) => {
            if (e.key === "Enter") create();
            else if (e.key === "Escape") stopCreating();
          }}
        />
        <button type="button" class="btn primary" onclick={create}>Crea</button>
        <button type="button" class="btn" onclick={stopCreating}>Annulla</button>
      </div>
      {#if createError}<div class="err-msg" role="alert"><ErrorText explanation={createError} /></div>{/if}
    </div>
  {/if}
  <div class="rows">
    {#each listing.folders as folder (folder.path)}
      <button type="button" class="row" onclick={() => pick(folder.path)}>
        <Icon name={chosen ? "folder" : "disk"} /><span>{folder.name}</span>
      </button>
    {:else}
      {#if listing.error}
        <div class="err-msg" role="alert">{listing.error}</div>
      {:else if chosen}
        <p class="muted">Qui dentro non ci sono altre cartelle.</p>
      {/if}
    {/each}
  </div>
</div>
{#if error}<div class="err-msg" role="alert"><ErrorText explanation={error} /></div>{/if}
<details class="manual" bind:open={manualOpen}>
  <summary>Preferisci scrivere il percorso a mano?</summary>
  <div class="manualbody">
    <label class="f" for={id}>Percorso</label>
    <input
      type="text"
      {id}
      bind:value
      bind:this={input}
      {placeholder}
      autocomplete="off"
      spellcheck="false"
      aria-invalid={error ? "true" : "false"}
      onkeydown={(e) => e.key === "Enter" && onenter()}
    />
    <p class="help">Per esempio D:\Foto oppure \\NAS\Condivisa. Premi Invio per andare avanti.</p>
  </div>
</details>

<style>
  .intro {
    margin: 0 0 10px;
  }
  .pick {
    margin-top: 18px;
    border-radius: var(--r);
    overflow: hidden;
    background: var(--card);
    box-shadow: var(--shadow);
  }
  .pickhead {
    display: flex;
    flex-wrap: wrap;
    gap: 8px 12px;
    align-items: center;
    justify-content: space-between;
    padding: 12px 18px;
    overflow-wrap: anywhere;
    font-size: 15px;
    color: var(--fg-2);
  }
  .pickhead b {
    color: var(--fg);
  }
  .pickhead > span {
    min-width: 0;
  }
  .pickhead.chosen {
    background: var(--ok-soft);
    color: var(--on-ok-soft);
    font-weight: 700;
  }
  .pickhead.chosen b {
    color: inherit;
  }
  .pickhead.chosen > span {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
  }
  .pickhead.chosen > span :global(svg) {
    width: 20px;
    height: 20px;
    flex: none;
    color: var(--ok);
  }
  .pickhead :global(.btn svg) {
    width: 20px;
    height: 20px;
  }
  .acts {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }
  .newfolder {
    padding: 4px 18px 16px;
  }
  .newrow {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    align-items: center;
  }
  .newrow input {
    flex: 1 1 200px;
    min-width: 0;
  }
  .rows {
    display: grid;
    gap: 6px;
    padding: 0 10px 10px;
    max-height: 300px;
    overflow: auto;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 12px;
    min-height: 54px;
    padding: 6px 14px;
    border: 0;
    border-radius: var(--r-s);
    background: var(--card-2);
    text-align: left;
    cursor: pointer;
    font-size: 17px;
    font-weight: 600;
  }
  .row:hover {
    background: color-mix(in srgb, var(--accent) 10%, var(--card-2));
  }
  .row :global(svg) {
    width: 26px;
    height: 26px;
    flex: none;
    color: var(--accent);
  }
  .row span {
    min-width: 0;
    overflow-wrap: anywhere;
  }
  .rows p {
    padding: 14px;
  }
  .manual {
    margin-top: 16px;
    border-radius: var(--r);
    background: var(--card);
    box-shadow: var(--shadow);
    color: var(--fg-2);
  }
  .manual > summary {
    cursor: pointer;
    min-height: 50px;
    display: flex;
    align-items: center;
    padding: 4px 20px;
    font-weight: 700;
    font-size: 15.5px;
    list-style: none;
    border-radius: var(--r);
  }
  .manual > summary::-webkit-details-marker {
    display: none;
  }
  .manual > summary:hover {
    background: color-mix(in srgb, var(--fg) 5%, transparent);
  }
  .manualbody {
    padding: 4px 20px 18px;
    color: var(--fg);
  }
</style>
