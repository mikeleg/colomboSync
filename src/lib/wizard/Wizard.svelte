<script lang="ts">
  import { onMount, untrack } from "svelte";
  import Icon from "../Icon.svelte";
  import { defaultSyncName, onCloseRequested, previewSync, runSync, saveSync } from "../api";
  import { formatNumber } from "../format";
  import type { SchemaModel } from "../schema";
  import type { Counts, Preview, Run, Sync, SyncMode } from "../types";
  import StepDestination from "./StepDestination.svelte";
  import StepName from "./StepName.svelte";
  import StepOptions from "./StepOptions.svelte";
  import StepPreview from "./StepPreview.svelte";
  import StepResult from "./StepResult.svelte";
  import StepRunning from "./StepRunning.svelte";
  import StepSource from "./StepSource.svelte";

  let {
    onschema,
    onexit,
    sync: syncToRerun,
  }: {
    onschema: (model: SchemaModel) => void;
    onexit: () => void;
    /** A saved sync to run again: the wizard opens straight on a fresh preview. */
    sync?: Sync;
  } = $props();
  // Fixed for the life of the wizard: the app mounts a new one per sync.
  const sync = untrack(() => syncToRerun);

  // Step order lives here: add a step by adding an entry and its branch in the markup.
  const STEPS = [
    { id: "source", label: "Da dove", title: "Da dove copio?" },
    { id: "destination", label: "Dove", title: "Dove copio?" },
    { id: "name", label: "Nome", title: "Come la chiamo?" },
    { id: "options", label: "Opzioni", title: "Opzioni" },
    { id: "preview", label: "Anteprima", title: "Anteprima" },
  ];
  const mode: SyncMode = sync?.mode ?? "Copy";

  let phase = $state<"steps" | "running" | "result">("steps");
  let step = $state(sync ? STEPS.length - 1 : 0);
  let source = $state(sync?.source ?? "");
  let destination = $state(sync?.destination ?? "");
  let name = $state(sync?.name ?? "");
  let nameTouched = $state(!!sync);
  let error = $state("");
  let preview = $state<Preview | null>(null);
  let scanning = $state(!!sync);
  let previewError = $state("");
  let run = $state<Run | null>(null);
  let failure = $state("");
  let scanId = 0;
  // Listen from the start: an X click right after "Sì, avvia" must not slip by the running view.
  let closeRequests = $state(0);

  onMount(() => {
    if (sync) scan();
    const unlisten = onCloseRequested(() => closeRequests++);
    return () => {
      unlisten.then((stop) => stop());
    };
  });

  const stepId = $derived(STEPS[step].id);

  const previewCounts = $derived<Counts | null>(
    preview
      ? { new: preview.new.count, updated: preview.updated.count, deleted: preview.deleted.count }
      : null,
  );

  const runTotal = $derived(previewCounts ? previewCounts.new + previewCounts.updated + previewCounts.deleted : 0);

  const schema = $derived.by((): SchemaModel => {
    const places = { source: source.trim(), destination: destination.trim() };
    if (phase === "result") {
      const ok = run?.outcome === "Ok";
      const counts = run && ok ? { new: run.new, updated: run.updated, deleted: run.deleted } : null;
      const caption = ok ? "Fatto." : run?.outcome === "Cancelled" ? "Annullato." : "Non completato.";
      return { ...places, counts, caption };
    }
    if (phase === "running") return { ...places, counts: previewCounts, caption: "Sto copiando…", running: true };
    if (stepId === "preview" && scanning) {
      return { ...places, counts: null, caption: "Sto controllando le due cartelle…" };
    }
    if (stepId === "preview" && preview && previewCounts) {
      const { new: n, updated: u, deleted: d } = previewCounts;
      const caption =
        n + u + d === 0
          ? "B è già aggiornato."
          : `Da fare: copiare ${formatNumber(n)} file nuovi, aggiornarne ${formatNumber(u)}.`;
      return { ...places, counts: previewCounts, caption };
    }
    const caption = !places.source
      ? "Scegli la cartella da copiare."
      : !places.destination
        ? "Ora scegli dove mettere la copia."
        : "I file nuovi e modificati di A verranno copiati in B. In B non cancello niente.";
    return { ...places, counts: null, caption };
  });

  $effect(() => onschema(schema));

  function next() {
    if (stepId === "source" && !source.trim()) {
      error = "Scegli prima la cartella da copiare.";
      return;
    }
    if (stepId === "destination" && !destination.trim()) {
      error = "Scegli prima dove mettere la copia.";
      return;
    }
    if (stepId === "name" && !name.trim()) {
      error = "Scrivi un nome per riconoscere questa sincronizzazione.";
      return;
    }
    error = "";
    step++;
    if (STEPS[step].id === "name") suggestName();
    if (STEPS[step].id === "preview") scan();
  }

  /** Prefills "Copia di <last folder of A>" unless the user already chose a name. */
  async function suggestName() {
    if (nameTouched) return;
    try {
      const suggestion = await defaultSyncName(source.trim());
      if (!nameTouched) name = suggestion;
    } catch {
      // The user can type a name.
    }
  }

  function back() {
    scanId++; // ignore a preview that is still running
    scanning = false;
    error = "";
    if (step === 0 || sync) onexit();
    else step--;
  }

  async function scan() {
    const id = ++scanId;
    scanning = true;
    preview = null;
    previewError = "";
    try {
      const result = await previewSync(source.trim(), destination.trim(), mode);
      if (id === scanId) preview = result;
    } catch (e) {
      if (id === scanId) previewError = String(e);
    } finally {
      if (id === scanId) scanning = false;
    }
  }

  /** A new sync is saved when its first run starts or on "No, salva e basta"; a saved one already is. */
  async function saveIfNew() {
    if (!sync) await saveSync(name.trim(), source.trim(), destination.trim(), mode);
  }

  async function start() {
    if (!previewCounts) return;
    closeRequests = 0;
    phase = "running";
    try {
      await saveIfNew();
      run = await runSync(source.trim(), destination.trim(), mode, previewCounts, sync?.name ?? name.trim());
    } catch (e) {
      failure = String(e);
    }
    phase = "result";
  }

  async function stop() {
    try {
      await saveIfNew();
      onexit();
    } catch (e) {
      previewError = String(e);
    }
  }
</script>

{#if phase === "running"}
  <StepRunning total={runTotal} {closeRequests} />
{:else if phase === "result"}
  <StepResult {run} {failure} onhome={onexit} />
{:else}
  <ol class="steps" aria-label="Passi">
    {#each STEPS as s, i (s.id)}
      <li class:done={i < step} class:cur={i === step} aria-current={i === step ? "step" : undefined}>
        <span class="n">{#if i < step}<Icon name="tick" />{:else}{i + 1}{/if}</span>
        <span>{s.label}</span>
      </li>
    {/each}
  </ol>
  <div class="kicker">Passo {step + 1} di {STEPS.length}</div>
  <h1 id="h">{sync && stepId === "preview" ? `Anteprima: ${sync.name}` : STEPS[step].title}</h1>
  <div class="content">
    {#if stepId === "source"}
      <StepSource bind:path={source} {error} onenter={next} />
    {:else if stepId === "destination"}
      <StepDestination bind:path={destination} {error} onenter={next} />
    {:else if stepId === "name"}
      <StepName bind:name {error} onedit={() => (nameTouched = true)} onenter={next} />
    {:else if stepId === "options"}
      <StepOptions />
    {:else if stepId === "preview"}
      <StepPreview
        {scanning}
        {preview}
        error={previewError}
        saved={!!sync}
        onstart={start}
        onstop={stop}
        onback={back}
      />
    {/if}
  </div>
  <!-- After the scan the preview has its own choices (see StepPreview). -->
  {#if stepId !== "preview" || scanning}
    <div class="nav">
      <button type="button" class="btn" onclick={back}>{step === 0 || sync ? "Annulla" : "Indietro"}</button>
      {#if step < STEPS.length - 1}
        <button type="button" class="btn primary" onclick={next}>Avanti</button>
      {/if}
    </div>
  {/if}
{/if}

<style>
  .steps {
    display: flex;
    gap: 4px;
    list-style: none;
    padding: 0;
    margin: 0 0 24px;
    flex-wrap: wrap;
    align-items: center;
  }
  .steps li {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 4px 14px 4px 4px;
    border-radius: 999px;
    color: var(--fg-2);
    font-weight: 700;
    font-size: 14.5px;
  }
  .n {
    width: 30px;
    height: 30px;
    border-radius: 50%;
    display: grid;
    place-items: center;
    background: var(--card-3);
    color: var(--fg-2);
    font-size: 14px;
    font-weight: 800;
  }
  .n :global(svg) {
    width: 16px;
    height: 16px;
  }
  .steps li.done {
    color: var(--fg);
  }
  .steps li.done .n {
    background: var(--ok-soft);
    color: var(--on-ok-soft);
  }
  .steps li.cur {
    background: var(--accent-soft);
    color: var(--on-accent-soft);
  }
  .steps li.cur .n {
    background: var(--accent);
    color: var(--on-accent);
  }
  .content {
    margin-top: 14px;
  }
  .nav {
    display: flex;
    flex-wrap: wrap;
    gap: 12px;
    justify-content: space-between;
    margin-top: 30px;
  }
</style>
