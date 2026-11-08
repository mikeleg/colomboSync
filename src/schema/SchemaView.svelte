<script lang="ts">
  import { untrack } from "svelte";
  import Icon from "../shared/Icon.svelte";
  import { formatNumber } from "../shared/format";
  import { describePlace } from "./api";
  import type { SchemaModel } from "../shared/schema";
  import type { Counts, Place } from "../shared/types";

  let { model }: { model: SchemaModel } = $props();

  // Without motion (reduced-motion preference, or no matchMedia) every number is shown at once.
  const motionOK = () =>
    typeof globalThis.matchMedia === "function" &&
    globalThis.matchMedia("(prefers-reduced-motion: no-preference)").matches;

  // The counters shown on screen: they count up to model.counts and always end on it exactly.
  let shown = $state<Counts | null>(untrack(() => (model.counts ? { ...model.counts } : null)));
  // One ring pulse on B, cleared when its CSS animation ends.
  let pulse = $state<"" | "preview" | "Ok" | "Error">("");

  $effect(() => {
    const to = model.counts;
    const from = untrack(() => (shown ? { ...shown } : null));
    if (!to || !motionOK()) {
      shown = to ? { ...to } : null;
      return;
    }
    if (!from && !untrack(() => model.running || model.outcome)) pulse = "preview";
    const start = from ?? { new: 0, updated: 0, deleted: 0 };
    if (start.new === to.new && start.updated === to.updated && start.deleted === to.deleted) return;
    const duration = from ? 350 : 600;
    let t0: number | undefined;
    let raf = requestAnimationFrame(function tick(now) {
      t0 ??= now;
      const p = Math.min(1, (now - t0) / duration);
      const ease = 1 - (1 - p) ** 3;
      const at = (k: keyof Counts) => (p === 1 ? to[k] : Math.round(start[k] + (to[k] - start[k]) * ease));
      shown = { new: at("new"), updated: at("updated"), deleted: at("deleted") };
      if (p < 1) raf = requestAnimationFrame(tick);
    });
    return () => cancelAnimationFrame(raf);
  });

  let lastOutcome: SchemaModel["outcome"];
  $effect(() => {
    const outcome = model.outcome;
    if ((outcome === "Ok" || outcome === "Error") && outcome !== lastOutcome && motionOK()) pulse = outcome;
    lastOutcome = outcome;
  });

  // What Rust says about the paths the model has no place for, by path. A path is asked once, after it
  // has stopped changing (typing must not make one call per key); until answered it has no label.
  let places = $state<Record<string, Place>>({});
  const asked = new Set<string>();
  // Strings, so that the effect below only runs when a path itself changes, not on every new model.
  const sourceToAsk = $derived(model.sourcePlace ? "" : model.source);
  const destinationToAsk = $derived(model.destinationPlace ? "" : model.destination);
  $effect(() => {
    const paths = [sourceToAsk, destinationToAsk];
    const timer = setTimeout(() => {
      for (const path of paths) {
        if (!path || asked.has(path)) continue;
        asked.add(path);
        describePlace(path).then(
          (place) => (places[path] = place),
          () => {
            // Without an answer the path is shown with no label.
          },
        );
      }
    }, 150);
    return () => clearTimeout(timer);
  });
  const value = (key: keyof Counts) => (shown ? formatNumber(shown[key]) : "—");
</script>

{#snippet node(tag: string, path: string, known: Place | undefined, mir = false, isB = false)}
  {#if path}
    <div
      class="node"
      class:mir
      class:run={model.running}
      class:b={isB}
      data-pulse={isB && pulse ? pulse : undefined}
      onanimationend={(e) => {
        if (!e.pseudoElement) pulse = "";
      }}
    >
      {#if isB && mir}
        <svg class="gone" viewBox="0 0 120 36" aria-hidden="true">
          <rect class="g g1" x="6" y="14" width="14" height="10" rx="3" />
          <rect class="g g2" x="40" y="6" width="14" height="10" rx="3" />
          <rect class="g g3" x="74" y="18" width="14" height="10" rx="3" />
          <rect class="g g4" x="102" y="8" width="14" height="10" rx="3" />
        </svg>
      {/if}
      <div class="tag">{tag}</div>
      <div class="nb">
        <Icon name={(known ?? places[path])?.isNetwork ? "net" : "disk"} />
        <div>
          <div class="dl">{(known ?? places[path])?.label ?? ""}</div>
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

<h2 class="section-title">Cosa succede</h2>
{@render node("A · Da copiare", model.source, model.sourcePlace)}
<div class="mid" class:mir={model.mirror} class:run={model.running}>
  <div class="arr">
    <svg
      viewBox="0 0 44 64"
      fill="none"
      stroke="currentColor"
      stroke-width="4"
      stroke-linecap="round"
      stroke-linejoin="round"
      aria-hidden="true"
    >
      <path class="flow" d="M22 4v44" /><path d="M10 40l12 16 12-16" />
      <g class="dots"
        ><circle class="dot d1" cx="22" cy="4" r="3" /><circle class="dot d2" cx="22" cy="4" r="3" /><circle
          class="dot d3"
          cx="22"
          cy="4"
          r="3"
        /></g
      >
    </svg>
    <span class="chip" class:copy={!model.mirror} class:mirror={model.mirror}
      >{model.mirror ? "Specchio" : "Copia"}</span
    >
  </div>
  <div class="counts">
    <div class="cnt"><b>{value("new")}</b><span>Nuovi</span></div>
    <div class="cnt"><b>{value("updated")}</b><span>Aggiornati</span></div>
    {#if model.mirror}
      <div class="cnt d"><b>{value("deleted")}</b><span>Eliminati da B</span></div>
    {/if}
  </div>
</div>
{@render node("B · Copia", model.destination, model.destinationPlace, model.mirror, true)}
{#if !model.mirror && model.source && model.destination}
  <p class="safe"><Icon name="tick" />Nessun file viene cancellato in B</p>
{/if}
<p class={["cap", { warn: model.warnCaption }]}>{model.caption}</p>

<style>
  .node {
    background: var(--card-2);
    color: var(--fg);
    padding: 16px 18px;
    border-radius: 20px;
    position: relative;
    overflow: hidden; /* clips the travelling highlight; the ring pulse is the node's own box-shadow, so it still shows */
    --base: inset 0 0 0 0 transparent;
    box-shadow: var(--base);
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
    --base: inset 0 0 0 1.5px color-mix(in srgb, var(--danger) 60%, transparent);
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
  .dot {
    fill: var(--fg);
    opacity: 0;
  }
  .dots {
    filter: drop-shadow(0 0 3px currentColor);
  }
  .g {
    fill: var(--danger);
    opacity: 0;
    transform-box: fill-box;
    transform-origin: center;
  }
  .gone {
    position: absolute;
    top: 10px;
    right: 14px;
    width: 96px;
    height: 29px;
    pointer-events: none;
  }

  /* Motion: all of it lives here, so with reduced motion the schema is the plain still picture. */
  @media (prefers-reduced-motion: no-preference) {
    .arr {
      --dur: 4.8s; /* one particle trip; running makes it faster */
    }
    .mid.run .arr {
      --dur: 1.2s;
    }
    /* idle: the arrow breathes and small "files" drift from A to B */
    .arr svg {
      animation: breathe 3.2s ease-in-out infinite alternate;
    }
    .mid.mir .arr svg {
      animation-duration: 2.2s; /* red: a firmer pulse, not a playful one */
    }
    .mid.run .arr svg {
      animation-duration: 1.2s;
    }
    .dot {
      animation: drift var(--dur) linear infinite;
    }
    .d2 {
      animation-delay: calc(var(--dur) / -3);
    }
    .d3 {
      animation-delay: calc(var(--dur) * -2 / 3);
    }
    .mid.run .flow {
      animation: flow 0.7s linear infinite;
    }

    /* running: a highlight travels across A, then B */
    .node.run::after {
      content: "";
      position: absolute;
      inset: 0 auto 0 0;
      width: 45%;
      pointer-events: none;
      background: linear-gradient(100deg, transparent, color-mix(in srgb, var(--accent) 16%, transparent), transparent);
      transform: translateX(-100%);
      animation: sweep 2.6s ease-in-out infinite;
    }
    .node.run.b::after {
      animation-delay: 1.3s;
    }
    .node.mir.run::after {
      background: linear-gradient(100deg, transparent, color-mix(in srgb, var(--danger) 20%, transparent), transparent);
    }

    /* one ring pulse on B: preview ready, run ok, run failed */
    .node[data-pulse] {
      --ring: var(--accent);
      --spread: 10px;
      animation: ring 0.8s ease-out 1;
    }
    .node.mir[data-pulse="preview"],
    .node[data-pulse="Error"] {
      --ring: var(--danger);
    }
    .node[data-pulse="Ok"],
    .node[data-pulse="Error"] {
      --spread: 22px;
      animation-duration: 1.4s;
    }

    /* mirror: files dissolve inside B */
    .g {
      animation: dissolve 3s ease-in infinite;
    }
    .g2 {
      animation-delay: 0.7s;
    }
    .g3 {
      animation-delay: 1.4s;
    }
    .g4 {
      animation-delay: 2.1s;
    }
    .cnt.d {
      animation: cnt-pulse 0.9s ease-out 1;
    }
  }
  @keyframes flow {
    to {
      stroke-dashoffset: -14;
    }
  }
  @keyframes breathe {
    from {
      opacity: 0.85;
      filter: drop-shadow(0 0 1px color-mix(in srgb, currentColor 20%, transparent));
    }
    to {
      opacity: 1;
      filter: drop-shadow(0 0 9px color-mix(in srgb, currentColor 75%, transparent));
    }
  }
  @keyframes drift {
    0% {
      transform: translateY(0);
      opacity: 0;
    }
    15%,
    85% {
      opacity: 0.9;
    }
    100% {
      transform: translateY(46px);
      opacity: 0;
    }
  }
  @keyframes sweep {
    0%,
    15% {
      transform: translateX(-100%);
    }
    85%,
    100% {
      transform: translateX(320%);
    }
  }
  @keyframes ring {
    from {
      box-shadow:
        var(--base),
        0 0 0 0 color-mix(in srgb, var(--ring) 65%, transparent);
    }
    to {
      box-shadow:
        var(--base),
        0 0 0 var(--spread) color-mix(in srgb, var(--ring) 0%, transparent);
    }
  }
  @keyframes dissolve {
    0% {
      opacity: 0;
      transform: scale(1);
    }
    12% {
      opacity: 0.85;
    }
    60% {
      opacity: 0.5;
      transform: translateY(-3px) scale(0.8);
    }
    100% {
      opacity: 0;
      transform: translateY(-8px) scale(0.3);
    }
  }
  @keyframes cnt-pulse {
    0% {
      transform: scale(0.96);
      box-shadow: 0 0 0 0 color-mix(in srgb, var(--danger) 60%, transparent);
    }
    40% {
      transform: scale(1.05);
    }
    100% {
      transform: scale(1);
      box-shadow: 0 0 0 14px color-mix(in srgb, var(--danger) 0%, transparent);
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
  }
  .cap:not(.warn) {
    font-size: 15px;
    text-align: center;
    color: var(--fg-2);
  }
</style>
