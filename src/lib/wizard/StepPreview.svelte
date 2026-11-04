<script lang="ts">
  import Icon from "../Icon.svelte";
  import { formatNumber, formatSize } from "../format";
  import type { FileGroup, Preview } from "../types";

  let {
    scanning,
    preview,
    error,
    saved = false,
    onstart,
    onstop,
    onback,
  }: {
    scanning: boolean;
    preview: Preview | null;
    error: string;
    /** Re-running a saved sync: nothing to save, nothing to change, "No" just returns to the list. */
    saved?: boolean;
    onstart: () => void;
    onstop: () => void;
    onback: () => void;
  } = $props();

  const total = $derived(preview ? preview.new.count + preview.updated.count + preview.deleted.count : 0);
</script>

{#snippet group(title: string, why: string, g: FileGroup)}
  {#if g.count > 0}
    <details class="grp" open>
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
  <div class="err-msg" role="alert">{error}</div>
  <div class="btns back"><button type="button" class="link" onclick={onback}>Indietro</button></div>
{:else if preview && total === 0}
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
  <p class="sum">
    Ecco cosa verrà fatto: <b>{formatNumber(preview.new.count)}</b> file nuovi,
    <b>{formatNumber(preview.updated.count)}</b> da aggiornare. Peso totale da copiare:
    <b>{formatSize(preview.bytes)}</b>.
  </p>
  {@render group("Da copiare (nuovi)", "File che ci sono in A e non ci sono ancora in B.", preview.new)}
  {@render group("Da aggiornare", "File che in B sono più vecchi di quelli in A.", preview.updated)}
  <section class="final" aria-labelledby="fq">
    <h2 id="fq">Avviare adesso?</h2>
    <div class="btns">
      <button type="button" class="btn primary big" onclick={onstart}>Sì, avvia</button>
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
    background: var(--card-2);
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
    font-size: 18px;
    margin-bottom: 16px;
    line-height: 1.5;
  }
  .sum b {
    color: var(--accent);
  }
  .grp {
    border-radius: var(--r);
    margin-bottom: 12px;
    background: var(--card-2);
  }
  .grp > summary {
    display: flex;
    gap: 12px;
    align-items: center;
    justify-content: space-between;
    min-height: 58px;
    padding: 8px 20px;
    font-weight: 700;
    cursor: pointer;
    border-radius: var(--r);
    list-style: none;
  }
  .grp > summary::-webkit-details-marker {
    display: none;
  }
  .grp > summary:hover {
    background: color-mix(in srgb, var(--fg) 5%, transparent);
  }
  .grp > summary b {
    font-size: 22px;
    font-weight: 800;
    padding: 0 12px;
    border-radius: 999px;
    background: var(--card);
  }
  .why {
    padding: 0 20px;
    color: var(--fg-2);
    font-size: 15px;
  }
  .files {
    margin: 10px 0 0;
    padding: 0 20px 16px 40px;
    font-family: var(--mono);
    font-size: 14.5px;
    overflow-wrap: anywhere;
  }
  .more {
    padding: 0 20px 14px;
    color: var(--fg-2);
    font-size: 15px;
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
    font-size: 24px;
    margin: 8px 0 4px;
  }
  .final {
    margin-top: 24px;
    padding: 22px;
    border-radius: var(--r);
    background: var(--card-2);
  }
  .final h2 {
    font-size: 24px;
    margin-bottom: 14px;
  }
  .back {
    margin-top: 12px;
  }
</style>
