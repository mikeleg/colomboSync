<script lang="ts">
  import Icon from "./Icon.svelte";
  import { formatNumber, isNetworkPath } from "./format";
  import type { SchemaModel } from "./schema";

  let { model }: { model: SchemaModel } = $props();

  const place = (path: string) =>
    isNetworkPath(path)
      ? "Rete · " + path.slice(2).split("\\")[0]
      : /^[A-Za-z]:/.test(path)
        ? "Disco " + path.slice(0, 2).toUpperCase()
        : "Cartella";
  const value = (key: "new" | "updated" | "deleted") =>
    model.counts ? formatNumber(model.counts[key]) : "—";
</script>

{#snippet node(tag: string, path: string, mir = false)}
  {#if path}
    <div class="node" class:mir>
      <div class="tag">{tag}</div>
      <div class="nb">
        <Icon name={isNetworkPath(path) ? "net" : "disk"} />
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
<div class="mid" class:mir={model.mirror} class:run={model.running}>
  <div class="arr">
    <svg viewBox="0 0 44 64" fill="none" stroke="currentColor" stroke-width="4" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
      <path class="flow" d="M22 4v44" /><path d="M10 40l12 16 12-16" />
    </svg>
    <span class="chip" class:copy={!model.mirror} class:mirror={model.mirror}>{model.mirror ? "Specchio" : "Copia"}</span>
  </div>
  <div class="counts">
    <div class="cnt"><b>{value("new")}</b><span>Nuovi</span></div>
    <div class="cnt"><b>{value("updated")}</b><span>Aggiornati</span></div>
    {#if model.mirror}
      <div class="cnt d"><b>{value("deleted")}</b><span>Eliminati da B</span></div>
    {/if}
  </div>
</div>
{@render node("B · Copia", model.destination, model.mirror)}
{#if !model.mirror && model.source && model.destination}
  <p class="safe"><Icon name="tick" />Nessun file viene cancellato in B</p>
{/if}
<p class="cap" class:mir={model.warnCaption}>{model.caption}</p>

<style>
  h2 {
    font-size: 15px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--fg-2);
    margin-bottom: 12px;
  }
  .node {
    background: var(--card);
    color: var(--fg);
    padding: 14px 16px;
    border-radius: var(--r);
    box-shadow: var(--shadow);
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
    border-radius: 14px;
    flex: none;
    background: var(--accent-soft);
    color: var(--on-accent-soft);
  }
  .nb > div {
    min-width: 0;
  }
  .dl {
    font-size: 15px;
    color: var(--fg-2);
  }
  .pth {
    font-weight: 600;
    overflow-wrap: anywhere;
    line-height: 1.3;
  }
  .node.mir {
    background: var(--danger-soft);
    color: var(--on-danger-soft);
  }
  .node.mir .dl,
  .node.mir .tag {
    color: var(--on-danger-soft);
  }
  .node.mir .nb > :global(svg) {
    background: var(--danger);
    color: var(--card);
  }
  .empty-n {
    background: var(--card-3);
    box-shadow: none;
    border: 2px dashed var(--line);
  }
  .empty-n .pth {
    color: var(--fg-2);
    font-weight: 500;
    font-style: italic;
  }
  .empty-n .nb > :global(svg) {
    background: var(--card-3);
    color: var(--fg-2);
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
    color: var(--accent);
  }
  .arr svg {
    width: 44px;
    height: 64px;
  }
  .mid.mir .arr {
    color: var(--danger);
  }
  .mid.mir .chip {
    background: var(--danger);
    color: var(--card);
  }
  .flow {
    stroke-dasharray: 7 7;
  }
  @media (prefers-reduced-motion: no-preference) {
    .mid.run .flow {
      animation: flow 0.7s linear infinite;
    }
  }
  @keyframes flow {
    to {
      stroke-dashoffset: -14;
    }
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
    border-radius: 16px;
    background: var(--card);
    color: var(--fg);
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
    color: var(--fg-2);
    display: block;
    line-height: 1.2;
  }
  .cnt.d {
    background: var(--danger-soft);
  }
  .cnt.d b,
  .cnt.d span {
    color: var(--on-danger-soft);
  }
  .safe {
    display: flex;
    gap: 8px;
    align-items: center;
    justify-content: center;
    margin-top: 12px;
    padding: 10px 14px;
    border-radius: 999px;
    background: var(--ok-soft);
    color: var(--on-ok-soft);
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
    color: var(--fg-2);
  }
  .cap.mir {
    background: var(--danger-soft);
    color: var(--on-danger-soft);
    font-weight: 700;
    text-align: left;
    padding: 12px 16px;
    border-radius: var(--r-s);
  }
</style>
