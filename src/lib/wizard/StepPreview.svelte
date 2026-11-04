<script lang="ts">
  import Icon from "../Icon.svelte";
  import Warnbox from "../Warnbox.svelte";
  import { formatNumber, formatSize } from "../format";
  import type { FileGroup, Preview } from "../types";

  let {
    scanning,
    mirror,
    preview,
    error,
    onstart,
    onstop,
    onback,
  }: {
    scanning: boolean;
    mirror: boolean;
    preview: Preview | null;
    error: string;
    onstart: () => void;
    onstop: () => void;
    onback: () => void;
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
  <div class="err-msg" role="alert">{error}</div>
  <div class="btns back"><button type="button" class="link" onclick={onback}>Indietro</button></div>
{:else if preview && total === 0 && !preview.blocked}
  <div class="empty">
    <Icon name="check" />
    <h2>B è già aggiornato</h2>
    <p>Non c’è niente da copiare: B ha già tutto quello che c’è in A.</p>
  </div>
  <div class="final">
    <div class="btns">
      <button type="button" class="btn primary" onclick={onstop}>Torna all’inizio</button>
      <button type="button" class="link" onclick={onback}>Indietro</button>
    </div>
  </div>
{:else if preview}
  {#if preview.blocked}
    <div class="guard">
      <Warnbox>
        <p>A sembra vuota o scollegata: con lo specchio verrebbe svuotato B. Controlla che il disco di A sia collegato.</p>
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
  {@render group("Da aggiornare", "File che in B sono più vecchi di quelli in A.", preview.updated)}
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
      <button type="button" class="btn big" onclick={onstop}>No, salva e basta</button>
      <button type="button" class="link" onclick={onback}>Indietro, cambio qualcosa</button>
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
  .grp.del {
    background: var(--danger-soft);
    color: var(--on-danger-soft);
  }
  .grp.del > summary b {
    background: var(--danger);
    color: var(--card);
  }
  .grp.del > summary:hover {
    background: color-mix(in srgb, var(--on-danger-soft) 8%, transparent);
  }
  .why {
    padding: 0 20px;
    color: var(--fg-2);
    font-size: 15px;
  }
  .grp.del .why,
  .grp.del .more {
    color: var(--on-danger-soft);
  }
  .guard {
    margin-bottom: 16px;
  }
  .ack {
    display: flex;
    gap: 14px;
    align-items: flex-start;
    padding: 16px 20px;
    border-radius: var(--r);
    background: var(--danger-soft);
    color: var(--on-danger-soft);
    margin: 14px 0;
    cursor: pointer;
    font-weight: 700;
  }
  .ack:hover {
    background: color-mix(in srgb, var(--on-danger-soft) 8%, var(--danger-soft));
  }
  .ack input {
    width: 28px;
    height: 28px;
    flex: none;
    accent-color: var(--danger);
    margin: 0;
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
