<script lang="ts">
  import Icon from "./Icon.svelte";
  import { formatNumber } from "./format";
  import type { SchemaModel } from "./schema";

  let { model }: { model: SchemaModel } = $props();

  const isNetwork = (path: string) => path.startsWith("\\\\");
  const place = (path: string) =>
    isNetwork(path)
      ? "Rete · " + path.slice(2).split("\\")[0]
      : /^[A-Za-z]:/.test(path)
        ? "Disco " + path.slice(0, 2).toUpperCase()
        : "Cartella";
  const value = (key: "new" | "updated") =>
    model.counts ? formatNumber(model.counts[key]) : "—";
</script>

{#snippet node(tag: string, path: string)}
  {#if path}
    <div class="node">
      <div class="tag">{tag}</div>
      <div class="nb">
        <Icon name={isNetwork(path) ? "net" : "disk"} />
        <div>
          <div class="dl">{place(path)}</div>
          <div class="pth">{path}</div>
        </div>
      </div>
    </div>
  {:else}
    <div class="node empty-n">
      <div class="tag">{tag}</div>
      <div class="nb">
        <Icon name="folder" />
        <div><div class="pth">Scegli la cartella</div></div>
      </div>
    </div>
  {/if}
{/snippet}

<h2>Cosa succede</h2>
{@render node("A · Da copiare", model.source)}
<div class="mid">
  <div class="arr">
    <svg viewBox="0 0 44 64" fill="none" stroke="currentColor" stroke-width="4" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
      <path class="flow" d="M22 4v44" /><path d="M10 40l12 16 12-16" />
    </svg>
    <span class="chip copy">Copia</span>
  </div>
  <div class="counts">
    <div class="cnt"><b>{value("new")}</b><span>Nuovi</span></div>
    <div class="cnt"><b>{value("updated")}</b><span>Aggiornati</span></div>
  </div>
</div>
{@render node("B · Copia", model.destination)}
{#if model.source && model.destination}
  <p class="safe"><Icon name="tick" />Nessun file viene cancellato in B</p>
{/if}
<p class="cap">{model.caption}</p>

<style>
  h2 {
    font-size: 15px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--on-surface-variant);
    margin-bottom: 12px;
  }
  .node {
    background: var(--surface-container-lowest);
    color: var(--on-surface);
    padding: 14px 16px;
  }
  .tag {
    font-weight: 700;
    font-size: 16px;
    margin-bottom: 8px;
  }
  .nb {
    display: flex;
    gap: 12px;
    align-items: center;
  }
  .nb > :global(svg) {
    width: 48px;
    height: 48px;
    padding: 10px;
    flex: none;
    background: var(--primary-container);
    color: var(--on-primary-container);
  }
  .nb > div {
    min-width: 0;
  }
  .dl {
    font-size: 15px;
    color: var(--on-surface-variant);
  }
  .pth {
    font-weight: 600;
    overflow-wrap: anywhere;
    line-height: 1.3;
  }
  .empty-n {
    background: var(--surface-container-high);
  }
  .empty-n .pth {
    color: var(--on-surface-variant);
    font-weight: 500;
    font-style: italic;
  }
  .empty-n .nb > :global(svg) {
    background: var(--surface-container-highest);
    color: var(--on-surface-variant);
  }
  .mid {
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 6px 0;
  }
  .arr {
    display: flex;
    align-items: center;
    gap: 12px;
    color: var(--primary);
  }
  .arr svg {
    width: 44px;
    height: 64px;
  }
  .flow {
    stroke-dasharray: 7 7;
  }
  .counts {
    display: grid;
    grid-auto-flow: column;
    grid-auto-columns: 1fr;
    gap: 8px;
    width: 100%;
    margin-top: 6px;
  }
  .cnt {
    text-align: center;
    padding: 10px 4px;
    background: var(--surface-container-lowest);
    color: var(--on-surface);
    min-width: 0;
  }
  .cnt b {
    display: block;
    font-size: 26px;
    line-height: 1.1;
    font-weight: 600;
  }
  .cnt span {
    font-size: 14px;
    color: var(--on-surface-variant);
    display: block;
    line-height: 1.2;
  }
  .safe {
    display: flex;
    gap: 8px;
    align-items: center;
    justify-content: center;
    margin-top: 12px;
    padding: 10px 14px;
    background: var(--success-container);
    color: var(--on-success-container);
    font-size: 16px;
    font-weight: 600;
    text-align: left;
  }
  .safe :global(svg) {
    width: 20px;
    height: 20px;
    flex: none;
  }
  .cap {
    margin-top: 14px;
    font-size: 16px;
    text-align: center;
    color: var(--on-surface-variant);
  }
</style>
