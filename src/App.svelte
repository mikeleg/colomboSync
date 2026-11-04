<script lang="ts">
  import History from "./lib/History.svelte";
  import Home from "./lib/Home.svelte";
  import { emptySchema, type SchemaModel } from "./lib/schema";
  import SchemaView from "./lib/SchemaView.svelte";
  import ThemeSwitch from "./lib/ThemeSwitch.svelte";
  import Wizard from "./lib/wizard/Wizard.svelte";

  let view = $state<"home" | "history" | "wizard">("home");
  let schema = $state<SchemaModel>(emptySchema);

  function leaveWizard() {
    view = "home";
    schema = emptySchema;
  }
</script>

<div class="app">
  <header class="topbar">
    <div class="brand">
      <span class="logo" aria-hidden="true">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M4 8h12l-3-3M20 16H8l3 3" />
        </svg>
      </span>
      <span><span class="t">ColomboSync</span><span class="s">Copie di sicurezza senza sorprese</span></span>
    </div>
    <ThemeSwitch />
  </header>
  <div class="layout">
    <main class="screen">
      {#if view !== "wizard"}
        <nav class="tabs" aria-label="Sezioni">
          <button type="button" aria-current={view === "home" ? "page" : undefined} onclick={() => (view = "home")}>
            Sincronizzazioni
          </button>
          <button type="button" aria-current={view === "history" ? "page" : undefined} onclick={() => (view = "history")}>
            Storico
          </button>
        </nav>
      {/if}
      {#if view === "home"}
        <Home onnew={() => (view = "wizard")} />
      {:else if view === "history"}
        <History />
      {:else}
        <Wizard onschema={(model) => (schema = model)} onexit={leaveWizard} />
      {/if}
    </main>
    <aside class="schema" aria-label="Schema di cosa succede">
      <SchemaView model={schema} />
    </aside>
  </div>
</div>

<style>
  .app {
    min-height: 100vh;
    background: var(--surface);
  }
  .topbar {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 16px 22px 14px 26px;
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
    width: 40px;
    height: 40px;
    border-radius: 14px;
    background: var(--accent);
    color: var(--on-accent);
    display: grid;
    place-items: center;
    flex: none;
  }
  .logo svg {
    width: 22px;
    height: 22px;
  }
  .t {
    font-size: 19px;
    font-weight: 800;
    letter-spacing: -0.01em;
  }
  .s {
    font-size: 14px;
    color: var(--fg-2);
    display: block;
    line-height: 1.2;
  }
  .layout {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 360px;
    gap: 18px;
    padding: 6px 18px 18px;
  }
  .screen {
    padding: 22px 26px 28px;
    min-width: 0;
    background: var(--card);
    border-radius: var(--r);
    box-shadow: var(--shadow);
  }
  .schema {
    background: var(--card-2);
    border-radius: var(--r);
    padding: 20px;
    min-width: 0;
    align-self: start;
    position: sticky;
    top: 16px;
  }
  .tabs {
    display: inline-flex;
    gap: 4px;
    margin-bottom: 22px;
    padding: 4px;
    border-radius: 999px;
    background: var(--card-2);
  }
  .tabs button {
    background: none;
    border: 0;
    border-radius: 999px;
    padding: 8px 20px;
    min-height: 44px;
    font-weight: 700;
    color: var(--fg-2);
    cursor: pointer;
  }
  .tabs button:hover {
    color: var(--fg);
  }
  .tabs button[aria-current="page"] {
    background: var(--card);
    color: var(--fg);
    box-shadow: var(--shadow);
  }
  @media (max-width: 900px) {
    .layout {
      grid-template-columns: minmax(0, 1fr);
    }
    .schema {
      order: -1;
      position: static;
    }
  }
</style>
