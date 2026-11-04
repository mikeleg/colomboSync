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
    font-size: 13px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--fg-2);
    margin-bottom: 14px;
  }
  .node {
    background: var(--card-2);
    color: var(--fg);
    padding: 16px 18px;
    border-radius: 20px;
  }
  .tag {
    font-weight: 700;
    font-size: 12px;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--fg-2);
    margin-bottom: 10px;
  }
  .nb {
    display: flex;
    gap: 12px;
    align-items: center;
  }
  .nb > :global(svg) {
    width: 44px;
    height: 44px;
    padding: 10px;
    border-radius: 14px;
    flex: none;
    background: var(--accent-soft);
    color: var(--accent);
  }
  .nb > div {
    min-width: 0;
  }
  .dl {
    font-size: 14px;
    color: var(--fg-2);
  }
  .pth {
    font-size: 18px;
    font-weight: 700;
    overflow-wrap: anywhere;
    line-height: 1.3;
  }
  .node.mir {
    background: var(--danger-soft);
    box-shadow: inset 0 0 0 1.5px color-mix(in srgb, var(--danger) 60%, transparent);
    color: var(--on-danger-soft);
  }
  .node.mir .dl,
  .node.mir .tag {
    color: var(--on-danger-soft);
  }
  .node.mir .nb > :global(svg) {
    background: var(--danger);
    color: var(--on-danger);
  }
  .empty-n {
    background: none;
    border: 1.5px dashed var(--line);
  }
  .empty-n .pth {
    color: var(--fg-2);
    font-size: 16px;
    font-weight: 500;
    font-style: italic;
  }
  .empty-n .nb > :global(svg) {
    background: var(--card-2);
    color: var(--fg-2);
  }
  .mid {
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 12px 0;
  }
  .arr {
    display: flex;
    align-items: center;
    gap: 12px;
    color: var(--accent);
  }
  .arr svg {
    width: 34px;
    height: 58px;
  }
  .mid.mir .arr {
    color: var(--danger);
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
    margin-top: 10px;
  }
  .cnt {
    text-align: center;
    padding: 12px 6px;
    border-radius: 16px;
    background: var(--card-2);
    color: var(--fg);
    min-width: 0;
  }
  .cnt b {
    display: block;
    font-size: 30px;
    line-height: 1.1;
    font-weight: 700;
    letter-spacing: -0.03em;
  }
  .cnt span {
    font-size: 13px;
    color: var(--fg-2);
    display: block;
    line-height: 1.2;
  }
  .cnt.d {
    background: color-mix(in srgb, var(--danger) 20%, transparent);
  }
  .cnt.d b {
    color: var(--danger);
  }
  .cnt.d span {
    color: var(--on-danger-soft);
    font-weight: 700;
  }
  .safe {
    display: flex;
    gap: 8px;
    align-items: center;
    justify-content: center;
    margin-top: 14px;
    color: var(--ok);
    font-size: 15px;
    font-weight: 700;
    text-align: center;
  }
  .safe :global(svg) {
    width: 20px;
    height: 20px;
    flex: none;
  }
  .cap {
    margin-top: 14px;
    font-size: 15px;
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
