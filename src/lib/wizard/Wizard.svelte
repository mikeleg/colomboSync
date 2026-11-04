<script lang="ts">
  import { onMount, untrack } from "svelte";
  import Icon from "../Icon.svelte";
  import { checkDestination, defaultSyncName, onCloseRequested, previewSync, runSync, saveSync } from "../api";
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
  let phase = $state<"steps" | "running" | "result">("steps");
  let step = $state(sync ? STEPS.length - 1 : 0);
  let source = $state(sync?.source ?? "");
  let destination = $state(sync?.destination ?? "");
  let mirror = $state(sync?.mode === "Mirror");
  let name = $state(sync?.name ?? "");
  let nameTouched = $state(!!sync);
  let error = $state("");
  let preview = $state<Preview | null>(null);
  let scanning = $state(!!sync);
  let previewError = $state("");
  let run = $state<Run | null>(null);
  let failure = $state("");
  let scanId = 0;
  let suggestionId = 0;
  let suggesting = $state(false);
  let saving = false; // a save (and run) is in flight: further clicks are ignored
  // Listen from the start: an X click right after "Sì, avvia" must not slip by the running view.
  let closeRequests = $state(0);

  onMount(() => {
    if (sync) scan();
    const unlisten = onCloseRequested(() => closeRequests++);
    return () => {
      unlisten.then((stop) => stop());
    };
  });

  const mode = $derived<SyncMode>(mirror ? "Mirror" : "Copy");
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
      return { ...places, counts, caption, mirror, warnCaption: false };
    }
    if (phase === "running") {
      return { ...places, counts: previewCounts, caption: "Sto copiando…", running: true, mirror, warnCaption: mirror };
    }
    if (stepId === "preview" && scanning) {
      return { ...places, counts: null, caption: "Sto controllando le due cartelle…", mirror, warnCaption: mirror };
    }
    if (stepId === "preview" && preview?.blocked) {
      return { ...places, counts: previewCounts, caption: preview.blocked, mirror, warnCaption: true };
    }
    if (stepId === "preview" && preview && previewCounts) {
      const { new: n, updated: u, deleted: d } = previewCounts;
      const caption =
        n + u + d === 0
          ? "B è già aggiornato."
          : `Da fare: copiare ${formatNumber(n)} file nuovi, aggiornarne ${formatNumber(u)}${mirror ? `, cancellarne ${formatNumber(d)} da B` : ""}.`;
      return { ...places, counts: previewCounts, caption, mirror, warnCaption: mirror };
    }
    const caption = !places.source
      ? "Scegli la cartella da copiare."
      : !places.destination
        ? "Ora scegli dove mettere la copia."
        : mirror
          ? "SPECCHIO: B diventerà identico ad A. I file che ci sono solo in B verranno cancellati."
          : "I file nuovi e modificati di A verranno copiati in B. In B non cancello niente.";
    return { ...places, counts: null, caption, mirror, warnCaption: mirror };
  });

  $effect(() => onschema(schema));

  async function next() {
    if (stepId === "source" && !source.trim()) {
      error = "Scegli prima la cartella da copiare.";
      return;
    }
    if (stepId === "destination" && !destination.trim()) {
      error = "Scegli prima dove mettere la copia.";
      return;
    }
    if (stepId === "name" && suggesting) return;
    if (stepId === "name" && !name.trim()) {
      error = "Scrivi un nome per riconoscere questa sincronizzazione.";
      return;
    }
    if (stepId === "destination") {
      const here = step;
      let checked: string[];
      do {
        checked = [source.trim(), destination.trim()];
        try {
          await checkDestination(checked[0], checked[1]);
        } catch (e) {
          error = String(e);
          return;
        }
        // The user may have edited a path while Rust was answering: check what is there now.
      } while (checked[0] !== source.trim() || checked[1] !== destination.trim());
      if (step !== here) return; // a second click got there first
    }
    error = "";
    step++;
    if (STEPS[step].id === "name") suggestName();
    if (STEPS[step].id === "preview") scan();
  }

  /** Prefills "Copia di <last folder of A>" unless the user already chose a name. Avanti waits for it. */
  async function suggestName() {
    if (nameTouched) return;
    const id = ++suggestionId;
    suggesting = true;
    try {
      const suggestion = await defaultSyncName(source.trim());
      if (id === suggestionId && !nameTouched) name = suggestion;
    } catch {
      // The user can type a name.
    } finally {
      if (id === suggestionId) suggesting = false;
    }
  }

  function editName() {
    nameTouched = true;
    suggesting = false;
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
  const saveIfNew = async (): Promise<Sync> =>
    sync ?? (await saveSync(name.trim(), source.trim(), destination.trim(), mode));

  async function start() {
    if (!previewCounts || saving) return;
    saving = true;
    closeRequests = 0;
    phase = "running";
    try {
      run = await runSync(await saveIfNew(), previewCounts);
    } catch (e) {
      failure = String(e);
    }
    phase = "result";
  }

  async function stop() {
    if (saving) return;
    saving = true;
    try {
      await saveIfNew();
      onexit();
    } catch (e) {
      previewError = String(e);
      saving = false;
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
      <StepName bind:name {error} onedit={editName} onenter={next} />
    {:else if stepId === "options"}
      <StepOptions bind:mirror />
    {:else if stepId === "preview"}
      <StepPreview
        {scanning}
        {mirror}
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
        <button type="button" class="btn primary" disabled={stepId === "name" && suggesting} onclick={next}>
          Avanti
        </button>
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
