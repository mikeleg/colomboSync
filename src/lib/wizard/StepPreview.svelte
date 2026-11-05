<script lang="ts">
  import Icon from "../Icon.svelte";
  import Warnbox from "../Warnbox.svelte";
  import { formatNumber, formatSize } from "../format";
  import type { Explanation, FileGroup, Preview } from "../types";

  let {
    scanning,
    mirror,
    preview,
    error,
    saved = false,
    onstart,
    onstop,
    onback,
    onretry,
  }: {
    scanning: boolean;
    mirror: boolean;
    preview: Preview | null;
    /** Why the preview could not be made, in Rust's words. */
    error: Explanation | null;
    /** Re-running a saved sync: nothing to save, nothing to change, "No" just returns to the list. */
    saved?: boolean;
    onstart: () => void;
    onstop: () => void;
    onback: () => void;
    onretry: () => void;
  } = $props();

  const total = $derived(preview ? preview.new.count + preview.updated.count + preview.deleted.count : 0);
  // The consent is per run: this component is built again every time the preview is shown, so it starts unchecked.
  const needsConsent = $derived(mirror && (preview?.deleted.count ?? 0) > 0);
  let consent = $state(false);
</script>

{#snippet group(title: string, why: string, g: FileGroup, del = false)}
  {#if g.count > 0}
    <details class="grp" class:del open>
      <summary><span>{title}</span><b>{formatNumber(g.count)}</b></summary>
      <p class="why">{why}</p>
      <ul class="files">
        {#each g.files as file (file)}<li>{file}</li>{/each}
      </ul>
      {#if g.count > g.files.length}
        <p class="more">… e altri {formatNumber(g.count - g.files.length)} file</p>
      {/if}
    </details>
  {/if}
{/snippet}

{#if scanning}
  <div class="scan" role="status">
    <span class="spin"></span>
    <span>Sto controllando le due cartelle… Non copio ancora niente.</span>
  </div>
{:else if error}
  <div class="err-msg" role="alert">
    <p>{error.title}</p>
    {#if error.advice}<p>{error.advice}</p>{/if}
  </div>
  <div class="btns back">
    <button type="button" class="btn primary" onclick={onretry}>Riprova</button>
    <button type="button" class="link" onclick={onback}>Indietro</button>
  </div>
{:else if preview && total === 0 && !preview.blocked}
  <div class="empty">
    <Icon name="check" />
    <h2>B è già aggiornato</h2>
    <p>Non c’è niente da copiare: B ha già tutto quello che c’è in A.</p>
  </div>
  <div class="final">
    <div class="btns">
      <button type="button" class="btn primary" onclick={onstop}>{saved ? "Torna all’elenco" : "Salva e chiudi"}</button>
      {#if !saved}<button type="button" class="link" onclick={onback}>Indietro</button>{/if}
    </div>
  </div>
{:else if preview}
  {#if preview.blocked}
    <div class="guard">
      <Warnbox>
        <p>{preview.blocked}</p>
      </Warnbox>
    </div>
  {:else}
    <p class="sum">
      {mirror ? "Ecco cosa succederà in B" : "Ecco cosa verrà fatto"}: <b>{formatNumber(preview.new.count)}</b> file nuovi,
      <b>{formatNumber(preview.updated.count)}</b> da aggiornare{#if mirror}, <b>{formatNumber(preview.deleted.count)}</b> da cancellare{/if}.
      Peso totale da copiare: <b>{formatSize(preview.bytes)}</b>.
    </p>
  {/if}
  {@render group("Da copiare (nuovi)", "File che ci sono in A e non ci sono ancora in B.", preview.new)}
  {@render group("Da aggiornare", "File che ci sono sia in A sia in B ma sono diversi: verranno sostituiti con quelli di A.", preview.updated)}
  {#if mirror}
    {@render group(
      "Da cancellare in B",
      "File che ci sono solo in B. Verranno cancellati e non finiscono nel Cestino.",
      preview.deleted,
      true,
    )}
    {#if preview.deleted.count === 0 && !preview.blocked}
      <p class="sum">In B non ci sono file in più: non verrà cancellato niente.</p>
    {/if}
  {/if}
  {#if needsConsent && !preview.blocked}
    <label class="ack">
      <input type="checkbox" bind:checked={consent} />
      <span>Ho capito: {formatNumber(preview.deleted.count)} file verranno cancellati da B</span>
    </label>
  {/if}
  <section class="final" aria-labelledby={preview.blocked ? undefined : "fq"}>
    {#if !preview.blocked}<h2 id="fq">Avviare adesso?</h2>{/if}
    <div class="btns">
      {#if !preview.blocked}
        <button type="button" class="btn primary big" disabled={needsConsent && !consent} onclick={onstart}>Sì, avvia</button>
      {/if}
      <button type="button" class="btn big" onclick={onstop}>{saved ? "No, torna all’elenco" : "No, salva e basta"}</button>
      {#if !saved}<button type="button" class="link" onclick={onback}>Indietro, cambio qualcosa</button>{/if}
    </div>
  </section>
{/if}

<style>
  .scan {
    display: flex;
    gap: 16px;
    align-items: center;
    padding: 26px;
    border-radius: var(--r);
    background: var(--card);
    box-shadow: var(--shadow);
    font-size: 18px;
    font-weight: 600;
  }
  .spin {
    width: 30px;
    height: 30px;
    border: 4px solid var(--card-3);
    border-top-color: var(--accent);
    border-radius: 50%;
    flex: none;
  }
  @media (prefers-reduced-motion: no-preference) {
    .spin {
      animation: rot 1s linear infinite;
    }
  }
  @keyframes rot {
    to {
      transform: rotate(360deg);
    }
  }
  .sum {
    font-size: 19px;
    margin-bottom: 20px;
    line-height: 1.5;
    color: var(--fg-2);
  }
  .sum b {
    color: var(--fg);
  }
  /* Each group's count is the big number of its tile; the one that deletes is red and glows. */
  .grp {
    border-radius: 24px;
    margin-bottom: 14px;
    background: var(--card);
    box-shadow: var(--shadow);
  }
  .grp > summary {
    display: flex;
    gap: 12px;
    align-items: center;
    justify-content: space-between;
    min-height: 84px;
    padding: 12px 24px;
    font-size: 17px;
    font-weight: 700;
    cursor: pointer;
    border-radius: 24px;
    list-style: none;
  }
  .grp > summary::-webkit-details-marker {
    display: none;
  }
  .grp > summary:hover {
    background: color-mix(in srgb, var(--fg) 4%, transparent);
  }
  .grp > summary b {
    font-size: 56px;
    line-height: 1;
    font-weight: 700;
    letter-spacing: -0.05em;
  }
  .grp.del {
    background: var(--danger-soft);
    color: var(--on-danger-soft);
    box-shadow:
      inset 0 0 0 1.5px color-mix(in srgb, var(--danger) 60%, transparent),
      0 20px 50px -25px var(--danger);
  }
  .grp.del > summary b {
    color: var(--danger);
  }
  .grp.del > summary:hover {
    background: color-mix(in srgb, var(--on-danger-soft) 8%, transparent);
  }
  .why {
    padding: 0 24px;
    color: var(--fg-2);
    font-size: 15px;
  }
  .grp.del .why,
  .grp.del .more,
  .grp.del .files {
    color: var(--on-danger-soft);
  }
  .guard {
    margin-bottom: 16px;
  }
  .ack {
    display: flex;
    gap: 14px;
    align-items: center;
    padding: 18px 24px;
    border-radius: 20px;
    background: color-mix(in srgb, var(--danger) 20%, transparent);
    color: var(--on-danger-soft);
    margin: 16px 0;
    cursor: pointer;
    font-size: 17px;
    font-weight: 700;
  }
  .ack:hover {
    background: color-mix(in srgb, var(--danger) 26%, transparent);
  }
  .ack input {
    width: 26px;
    height: 26px;
    flex: none;
    accent-color: var(--danger);
    margin: 0;
  }
  .files {
    margin: 10px 0 0;
    padding: 0 24px 16px 44px;
    font-family: var(--mono);
    font-size: 14px;
    color: color-mix(in srgb, var(--fg) 85%, transparent);
    overflow-wrap: anywhere;
  }
  .more {
    padding: 0 24px 14px;
    color: var(--fg-2);
    font-size: 14px;
  }
  .empty {
    text-align: center;
    padding: 36px 16px;
    background: var(--ok-soft);
    color: var(--on-ok-soft);
    border-radius: var(--r);
    margin-bottom: 12px;
  }
  .empty :global(svg) {
    width: 60px;
    height: 60px;
    color: var(--ok);
  }
  .empty h2 {
    font-size: 26px;
    margin: 8px 0 4px;
  }
  .final {
    margin-top: 28px;
    padding: 26px;
    border-radius: var(--r);
    background: var(--card);
    box-shadow: var(--shadow);
  }
  .final h2 {
    margin-bottom: 16px;
  }
  .back {
    margin-top: 12px;
  }
</style>
