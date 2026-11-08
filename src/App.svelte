<script lang="ts">
  import { onMount } from "svelte";
  import { isElevated } from "./shared/api";
  import { toExplanation } from "./shared/explanation";
  import { listRuns } from "./history/api";
  import History from "./history/History.svelte";
  import HistoryPanel from "./history/HistoryPanel.svelte";
  import Home from "./syncs/Home.svelte";
  import Icon from "./shared/Icon.svelte";
  import { emptySchema, type SchemaModel } from "./shared/schema";
  import SchemaView from "./schema/SchemaView.svelte";
  import ThemeSwitch from "./shared/ThemeSwitch.svelte";
  import type { Explanation, RunReport, Sync } from "./shared/types";
  import type { HistorySummary } from "./history/types";
  import Wizard from "./wizard/Wizard.svelte";

  /** Started as administrator: mapped network drives are not visible then. */
  let elevated = $state(false);
  onMount(() => {
    isElevated().then(
      (yes) => (elevated = yes),
      () => {}, // only a hint: no answer, no banner
    );
  });

  let view = $state<"home" | "history" | "wizard">("home");
  /** The screen on show (home or wizard): it knows what the schema panel draws. */
  let screen = $state.raw<{ getSchema: () => SchemaModel }>();
  const schema = $derived(screen?.getSchema() ?? emptySchema);
  /** The saved sync the wizard re-runs from its preview; undefined when it creates a new one. */
  let rerun = $state<Sync>();

  function openWizard(sync?: Sync) {
    rerun = sync;
    view = "wizard";
  }

  let runs = $state<RunReport[]>([]);
  let summary = $state<HistorySummary | null>(null);
  let historyStatus = $state<"loading" | "ready">("ready");
  let historyError = $state<Explanation | null>(null);
  let historyLoads = 0;

  async function openHistory() {
    const load = ++historyLoads;
    view = "history";
    runs = [];
    summary = null;
    historyError = null;
    historyStatus = "loading";
    try {
      const result = await listRuns();
      if (load !== historyLoads) return; // a newer request owns the screen
      runs = result.runs;
      summary = result.summary;
      historyStatus = "ready";
    } catch (e) {
      if (load !== historyLoads) return;
      historyError = toExplanation(e);
      historyStatus = "ready";
    }
  }

  function leaveWizard() {
    view = "home";
  }
</script>

<div class="app" class:mirror={view === "wizard" && schema.mirror}>
  <header class="topbar">
    <div class="brand">
      <!-- "Eclissi": A (teal) partly covered by B (the page colour, with a faint rim). -->
      <svg class="logo" viewBox="0 0 100 100" aria-hidden="true">
        <defs>
          <linearGradient id="colombo-eclissi" x1="0" y1="0" x2="1" y2="1">
            <stop offset="0" stop-color="#6ff0d6" />
            <stop offset="0.55" stop-color="#1fc7a8" />
            <stop offset="1" stop-color="#0a7d6e" />
          </linearGradient>
        </defs>
        <circle cx="58" cy="50" r="30" fill="url(#colombo-eclissi)" />
        <circle class="cover" cx="45" cy="50" r="30" />
      </svg>
      <span>
        <span class="t">Colombo<span class="sync">Sync</span></span>
        <span class="s">Copie di sicurezza senza sorprese</span>
      </span>
    </div>
    {#if view !== "wizard"}
      <nav class="tabs" aria-label="Sezioni">
        <button type="button" aria-current={view === "home" ? "page" : undefined} onclick={() => (view = "home")}>
          Sincronizzazioni
        </button>
        <button type="button" aria-current={view === "history" ? "page" : undefined} onclick={openHistory}>
          Storico
        </button>
      </nav>
    {/if}
    <ThemeSwitch />
  </header>
  {#if elevated}
    <p class="elevated" role="alert">
      <Icon name="warn" />Le unità di rete potrebbero non essere visibili: apri ColomboSync senza 'Esegui come
      amministratore'.
    </p>
  {/if}
  <div class="layout">
    <main class="screen">
      {#if view === "home"}
        <Home bind:this={screen} onnew={() => openWizard()} onrun={openWizard} />
      {:else if view === "history"}
        <History {runs} status={historyStatus} error={historyError} />
      {:else}
        <Wizard
          bind:this={screen}
          sync={rerun}
          onexit={leaveWizard}
          onhistory={() => {
            leaveWizard();
            openHistory();
          }}
        />
      {/if}
    </main>
    <aside class="schema" aria-label={view === "history" ? "Riepilogo dello storico" : "Schema di cosa succede"}>
      {#if view === "history"}
        <HistoryPanel {summary} />
      {:else}
        <SchemaView model={schema} />
      {/if}
    </aside>
  </div>
</div>

<style>
  .app {
    min-height: 100vh;
    background: radial-gradient(70% 55% at 85% 0%, var(--glow) 0%, transparent 70%), var(--surface);
  }
  /* Mirror is the one risky mode: the whole page takes a red glow instead of the teal one. */
  .app.mirror {
    background:
      radial-gradient(60% 45% at 90% 30%, color-mix(in srgb, var(--danger) 18%, transparent) 0%, transparent 70%),
      var(--surface);
  }
  .topbar {
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 24px 40px 0;
    flex-wrap: wrap;
  }
  .brand {
    display: flex;
    align-items: center;
    gap: 12px;
    min-width: 0;
    flex: 1;
  }
  .logo {
    width: 34px;
    height: 34px;
    flex: none;
  }
  .logo .cover {
    fill: var(--surface);
    stroke: var(--fg);
    stroke-opacity: 0.2;
    stroke-width: 2;
  }
  .t {
    display: block;
    font-size: 20px;
    font-weight: 700;
    letter-spacing: -0.02em;
    line-height: 1.2;
  }
  .sync {
    font-weight: 300;
    color: var(--fg-2);
  }
  .s {
    font-size: 13px;
    color: var(--fg-2);
    display: block;
    line-height: 1.2;
  }
  .elevated {
    display: flex;
    gap: 12px;
    align-items: center;
    margin: 16px 40px 0;
    padding: 12px 18px;
    border-radius: var(--r-s);
    background: var(--warn-soft);
    color: var(--on-warn-soft);
    font-weight: 700;
  }
  .elevated :global(svg) {
    width: 24px;
    height: 24px;
    flex: none;
  }
  .layout {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 360px;
    gap: 32px;
    padding: 32px 40px 48px;
    align-items: start;
  }
  .screen {
    min-width: 0;
  }
  /* Only the preview step renders `.content.fill` (see Wizard.svelte): the other steps keep the page scrolling. */
  .screen:has(:global(.content.fill)) {
    display: flex;
    flex-direction: column;
  }
  /* Wide and tall window: the wizard fills the window, so only its groups scroll and the start box stays at the bottom.
     Narrow or short window (below 760px high, where the header takes most of the height): the page scrolls and the
     start box sticks instead. */
  @media (min-width: 901px) and (min-height: 760px) {
    .app:has(:global(.content.fill)) :global(.groups) {
      min-height: 240px;
    }
    .app:has(:global(.content.fill)) {
      height: 100vh;
      display: flex;
      flex-direction: column;
    }
    .app:has(:global(.content.fill)) .layout {
      flex: 1;
      min-height: 0;
      grid-template-rows: minmax(0, 1fr);
      align-items: stretch;
    }
    .app:has(:global(.content.fill)) .screen {
      min-height: 0;
    }
  }
  .schema {
    background: var(--panel);
    box-shadow: var(--shadow);
    border-radius: 32px;
    padding: 28px;
    min-width: 0;
    align-self: start;
    position: sticky;
    top: 16px;
    /* A tall panel (many syncs in the Storico) scrolls inside itself instead of running off the window. */
    max-height: calc(100vh - 32px);
    overflow-y: auto;
  }
  .tabs {
    display: inline-flex;
    gap: 4px;
    padding: 4px;
    border-radius: 999px;
    background: color-mix(in srgb, var(--fg) 8%, transparent);
  }
  .tabs button {
    background: none;
    border: 0;
    border-radius: 999px;
    padding: 8px 18px;
    min-height: 40px;
    font-size: 15px;
    font-weight: 600;
    color: var(--fg-2);
    cursor: pointer;
  }
  .tabs button:hover {
    color: var(--fg);
  }
  .tabs button[aria-current="page"] {
    background: var(--fg);
    color: var(--bg);
  }
  @media (max-width: 900px) {
    .topbar {
      padding-inline: 24px;
    }
    .layout {
      grid-template-columns: minmax(0, 1fr);
      padding-inline: 24px;
    }
    .schema {
      order: -1;
      position: static;
      max-height: none;
    }
  }
</style>
