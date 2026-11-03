<script lang="ts">
  import Icon from "../Icon.svelte";
  import { formatNumber, formatSize } from "../format";
  import type { FileGroup, Preview } from "../types";

  let {
    scanning,
    preview,
    error,
    onstart,
    onstop,
  }: {
    scanning: boolean;
    preview: Preview | null;
    error: string;
    onstart: () => void;
    onstop: () => void;
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
{:else if preview && total === 0}
  <div class="empty">
    <Icon name="check" />
    <h2>B è già aggiornato</h2>
    <p>Non c’è niente da copiare: B ha già tutto quello che c’è in A.</p>
  </div>
  <div class="final">
    <div class="btns"><button type="button" class="btn primary" onclick={onstop}>Torna all’inizio</button></div>
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
      <button type="button" class="btn big" onclick={onstop}>No, torna all’inizio</button>
    </div>
  </section>
{/if}

<style>
  .scan {
    display: flex;
    gap: 14px;
    align-items: center;
    padding: 22px;
    background: var(--surface-container-low);
    font-size: 18px;
  }
  .spin {
    width: 28px;
    height: 28px;
    border: 4px solid var(--surface-container-highest);
    border-top-color: var(--primary);
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
    margin-bottom: 14px;
  }
  .grp {
    margin-bottom: 12px;
    background: var(--surface-container-low);
  }
  .grp > summary {
    display: flex;
    gap: 12px;
    align-items: center;
    justify-content: space-between;
    min-height: 56px;
    padding: 8px 20px;
    font-weight: 700;
    cursor: pointer;
  }
  .grp > summary:hover {
    background: color-mix(in srgb, var(--on-surface) 6%, transparent);
  }
  .grp > summary b {
    font-size: 20px;
  }
  .why {
    padding: 0 20px;
    color: var(--on-surface-variant);
    font-size: 15px;
  }
  .files {
    margin: 8px 0 0;
    padding: 0 20px 14px 40px;
    font-family: Consolas, "Cascadia Mono", monospace;
    font-size: 14.5px;
    overflow-wrap: anywhere;
  }
  .more {
    padding: 0 20px 14px;
    color: var(--on-surface-variant);
    font-size: 15px;
  }
  .empty {
    text-align: center;
    padding: 32px 16px;
    background: var(--success-container);
    color: var(--on-success-container);
    margin-bottom: 12px;
  }
  .empty :global(svg) {
    width: 56px;
    height: 56px;
    color: var(--success);
  }
  .empty h2 {
    font-size: 24px;
    margin: 6px 0;
  }
  .final {
    margin-top: 22px;
    padding: 20px;
    background: var(--surface-container-low);
  }
  .final h2 {
    font-size: 24px;
    margin-bottom: 12px;
  }
</style>
