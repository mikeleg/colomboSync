<script lang="ts">
  import { onMount, untrack } from "svelte";
  import Icon from "../shared/Icon.svelte";
  import { checkStep, previewSync } from "./api";
  import { defaultSyncName, saveSync } from "../syncs/api";
  import { onCloseRequested, runSync } from "../run/api";
  import { toExplanation } from "../shared/explanation";
  import type { SchemaModel } from "../shared/schema";
  import type { Counts, Explanation, RunReport, Sync, SyncMode } from "../shared/types";
  import type { Preview, StepId } from "./types";
  import StepDestination from "./StepDestination.svelte";
  import StepName from "./StepName.svelte";
  import StepOptions from "./StepOptions.svelte";
  import StepPreview from "./StepPreview.svelte";
  import StepResult from "../run/StepResult.svelte";
  import StepRunning from "../run/StepRunning.svelte";
  import StepSource from "./StepSource.svelte";

  let {
    onexit,
    onhistory,
    sync: syncToRerun,
  }: {
    onexit: () => void;
    /** "Vai allo storico" on the result. */
    onhistory: () => void;
    /** A saved sync to run again: the wizard opens straight on a fresh preview. */
    sync?: Sync;
  } = $props();
  // The sync to run again, or the one saved when this wizard started its first run.
  const rerun = untrack(() => syncToRerun);
  let sync = $state.raw(rerun);

  // Step order lives here: add a step by adding an entry and its branch in the markup.
  const STEPS: { id: StepId; label: string; title: string }[] = [
    { id: "source", label: "Da dove", title: "Da dove copio?" },
    { id: "destination", label: "Dove", title: "Dove copio?" },
    { id: "name", label: "Nome", title: "Come la chiamo?" },
    { id: "options", label: "Opzioni", title: "Opzioni" },
    { id: "preview", label: "Anteprima", title: "Anteprima" },
  ];
  let phase = $state<"steps" | "running" | "result">("steps");
  let step = $state(rerun ? STEPS.length - 1 : 0);
  let source = $state(rerun?.source ?? "");
  let destination = $state(rerun?.destination ?? "");
  let mirror = $state(rerun?.mode === "Mirror");
  let name = $state(rerun?.name ?? "");
  let nameTouched = $state(!!rerun);
  let error = $state<Explanation | null>(null);
  let preview = $state<Preview | null>(null);
  let scanning = $state(!!rerun);
  let previewError = $state<Explanation | null>(null);
  let run = $state<RunReport | null>(null);
  let failure = $state<Explanation | null>(null);
  let scanId = 0;
  let suggestionId = 0;
  let suggesting = $state(false);
  let saving = false; // a save (and run) is in flight: further clicks are ignored
  // Listen from the start: an X click right after "Sì, avvia" must not slip by the running view.
  let closeRequests = $state(0);

  onMount(() => {
    if (rerun) scan();
    const unlisten = onCloseRequested(() => closeRequests++);
    return () => {
      unlisten.then((stop) => stop());
    };
  });

  const mode = $derived<SyncMode>(mirror ? "Mirror" : "Copy");
  const stepId = $derived(STEPS[step].id);

  const previewCounts = $derived<Counts | null>(
    preview ? { new: preview.new.count, updated: preview.updated.count, deleted: preview.deleted.count } : null,
  );

  const runTotal = $derived(previewCounts ? previewCounts.new + previewCounts.updated + previewCounts.deleted : 0);

  const schema = $derived.by((): SchemaModel => {
    const places = { source: source.trim(), destination: destination.trim() };
    if (phase === "result") {
      const ok = run?.outcome === "Ok";
      const counts = run && ok ? { new: run.new, updated: run.updated, deleted: run.deleted } : null;
      // No Run when Rust refused to start one: there is no outcome for it to word.
      const caption = run ? run.caption : "Non completato.";
      return { ...places, counts, caption, mirror, warnCaption: false, outcome: run?.outcome };
    }
    if (phase === "running") {
      return { ...places, counts: previewCounts, caption: "Sto copiando…", running: true, mirror, warnCaption: mirror };
    }
    if (stepId === "preview" && scanning) {
      return { ...places, counts: null, caption: "Sto controllando le due cartelle…", mirror, warnCaption: mirror };
    }
    if (stepId === "preview" && preview && previewCounts) {
      // Rust's caption: why it is blocked, or what is to do.
      return {
        ...places,
        counts: previewCounts,
        caption: preview.caption,
        mirror,
        warnCaption: !!preview.blocked || mirror,
      };
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

  /** What the schema panel beside this screen draws; the app reads it (`bind:this`), so nothing is sent up. */
  export const getSchema = () => schema;

  async function next() {
    const here = step;
    const id = stepId;
    if (id === "name" && suggesting) return;
    // Opzioni has nothing to ask Rust.
    if (id === "source" || id === "destination" || id === "name") {
      for (;;) {
        const checked: [string, string, string] = [source.trim(), destination.trim(), name.trim()];
        let refusal: Explanation | null = null;
        try {
          await checkStep(id, ...checked);
        } catch (e) {
          refusal = toExplanation(e);
        }
        if (step !== here) return; // Indietro or a second click got there first: this answer is late
        // The user may have edited a path or the name while Rust was answering: check what is there now.
        if (checked[0] !== source.trim() || checked[1] !== destination.trim() || checked[2] !== name.trim()) continue;
        if (refusal) {
          error = refusal;
          return;
        }
        break;
      }
    }
    error = null;
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
    error = null;
    if (step === 0 || sync) onexit();
    else step--;
  }

  async function scan() {
    const id = ++scanId;
    scanning = true;
    preview = null;
    previewError = null;
    try {
      const result = await previewSync(source.trim(), destination.trim(), mode);
      if (id === scanId) preview = result;
    } catch (e) {
      if (id === scanId) previewError = toExplanation(e);
    } finally {
      if (id === scanId) scanning = false;
    }
  }

  /** A new sync is saved when its first run starts or on "No, salva e basta"; a saved one already is. */
  const saveIfNew = async (): Promise<Sync> =>
    (sync ??= await saveSync(name.trim(), source.trim(), destination.trim(), mode));

  async function start(consent: boolean) {
    if (!preview || saving) return;
    saving = true;
    closeRequests = 0;
    phase = "running";
    try {
      run = await runSync(await saveIfNew(), preview.previewToken, consent);
    } catch (e) {
      failure = toExplanation(e);
    }
    phase = "result";
  }

  /** "Riprova" on the result: back to a fresh preview of the same sync, never straight to a run. */
  function retry() {
    run = null;
    failure = null;
    saving = false;
    phase = "steps";
    step = STEPS.length - 1;
    scan();
  }

  async function stop() {
    if (saving) return;
    saving = true;
    try {
      await saveIfNew();
      onexit();
    } catch (e) {
      previewError = toExplanation(e);
      saving = false;
    }
  }
</script>

{#if phase === "running"}
  <StepRunning total={runTotal} {closeRequests} />
{:else if phase === "result"}
  <StepResult {run} {failure} onhome={onexit} onretry={retry} {onhistory} />
{:else}
  <ol class="steps" aria-label="Passi">
    {#each STEPS as s, i (s.id)}
      <li class:done={i < step} class:cur={i === step} aria-current={i === step ? "step" : undefined}>
        <span class="n"
          >{#if i < step}<Icon name="tick" size={14} />{:else}{i + 1}{/if}</span
        >
        <span>{s.label}</span>
      </li>
    {/each}
  </ol>
  <div class="kicker">Passo {step + 1} di {STEPS.length}</div>
  <h1 id="h">{sync && stepId === "preview" ? `Anteprima: ${sync.name}` : STEPS[step].title}</h1>
  <!-- `fill` asks the app to give the preview the whole window (see App.svelte). -->
  <div class={["content", { fill: stepId === "preview" }]}>
    {#if stepId === "source"}
      <StepSource bind:path={source} {error} onenter={next} />
    {:else if stepId === "destination"}
      <StepDestination bind:path={destination} source={source.trim()} {error} onenter={next} />
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
        onretry={scan}
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
    gap: 6px;
    list-style: none;
    padding: 0;
    margin: 0 0 28px;
    flex-wrap: wrap;
    align-items: center;
  }
  .steps li {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 14px;
    border-radius: 999px;
    background: color-mix(in srgb, var(--fg) 6%, transparent);
    color: var(--fg-2);
    font-weight: 600;
    font-size: 14px;
  }
  .n {
    display: inline-grid;
    place-items: center;
    font-weight: 700;
  }
  .n :global(svg) {
    stroke-width: 3;
  }
  .steps li.done .n {
    color: var(--accent);
  }
  .steps li.cur {
    background: var(--fg);
    color: var(--bg);
  }
  .content {
    margin-top: 18px;
  }
  .nav {
    display: flex;
    flex-wrap: wrap;
    gap: 12px;
    justify-content: space-between;
    margin-top: 32px;
  }
</style>
