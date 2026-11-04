<script lang="ts">
  import { listRuns } from "./lib/api";
  import History from "./lib/History.svelte";
  import HistoryLegend from "./lib/HistoryLegend.svelte";
  import Home from "./lib/Home.svelte";
  import { emptySchema, type SchemaModel } from "./lib/schema";
  import SchemaView from "./lib/SchemaView.svelte";
  import type { Run } from "./lib/types";
  import Wizard from "./lib/wizard/Wizard.svelte";

  let view = $state<"home" | "history" | "wizard">("home");
  let schema = $state<SchemaModel>(emptySchema);
  let runs = $state<Run[]>([]);
  let historyStatus = $state<"loading" | "ready" | "error">("ready");
  let historyLoads = 0;

  async function openHistory() {
    const load = ++historyLoads;
    view = "history";
    runs = [];
    historyStatus = "loading";
    try {
      const result = await listRuns();
      if (load !== historyLoads) return; // a newer request owns the screen
      runs = result;
      historyStatus = "ready";
    } catch {
      if (load === historyLoads) historyStatus = "error";
    }
  }

  function leaveWizard() {
    view = "home";
    schema = emptySchema;
  }
</script>

<div class="layout">
  <main class="screen">
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
    {#if view === "home"}
      <Home onnew={() => (view = "wizard")} />
    {:else if view === "history"}
      <History {runs} status={historyStatus} />
    {:else}
      <Wizard onschema={(model) => (schema = model)} onexit={leaveWizard} />
    {/if}
  </main>
  <aside class="schema" aria-label={view === "history" ? "Come leggere lo storico" : "Schema di cosa succede"}>
    {#if view === "history"}
      <HistoryLegend {runs} />
    {:else}
      <SchemaView model={schema} />
    {/if}
  </aside>
</div>

<style>
  .layout {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 356px;
    min-height: 100vh;
  }
  .screen {
    padding: 28px 32px 32px;
    min-width: 0;
  }
  .schema {
    background: var(--surface-container);
    margin: 16px 16px 16px 0;
    padding: 20px;
    min-width: 0;
    align-self: start;
    position: sticky;
    top: 16px;
  }
  .tabs {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    margin-bottom: 22px;
  }
  .tabs button {
    background: none;
    border: 0;
    border-radius: 0;
    padding: 10px 20px;
    min-height: 48px;
    font-weight: 600;
    color: var(--on-surface-variant);
    cursor: pointer;
  }
  .tabs button:hover {
    background: color-mix(in srgb, var(--on-surface) 8%, transparent);
  }
  .tabs button[aria-current="page"] {
    background: var(--secondary-container);
    color: var(--on-secondary-container);
  }
</style>
