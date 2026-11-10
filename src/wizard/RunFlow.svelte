<script lang="ts">
  import { onMount } from "svelte";
  import { toExplanation } from "../shared/explanation";
  import type { Explanation, RunReport, Sync } from "../shared/types";
  import { runSync } from "./api";
  import StepResult from "./StepResult.svelte";
  import StepRunning from "./StepRunning.svelte";

  /**
   * One run, from "Sì, avvia" to its result: it starts as soon as this is shown, shows the progress while it goes and the
   * result when it ends. `run` or `failure` (never both) is set when it has ended: the wizard reads them for its schema.
   */
  let {
    save,
    previewToken,
    consent,
    total,
    closeRequests,
    run = $bindable(null),
    failure = $bindable(null),
    onexit,
    onretry,
    onhistory,
  }: {
    /** The sync to run: a new one is saved first, a saved one is only handed over. */
    save: () => Promise<Sync>;
    previewToken: string | null;
    /** The box "Ho capito" was ticked. */
    consent: boolean;
    /** Files in the preview, for "0 di N file" before the first event. */
    total: number;
    /** Clicks on the window's X so far; the wizard counts them from its mount on, so none is lost while this starts. */
    closeRequests: number;
    run?: RunReport | null;
    /** Why no Run exists (Rust refused it, or the sync could not be saved). */
    failure?: Explanation | null;
    onexit: () => void;
    /** "Riprova" on the result. */
    onretry: () => void;
    /** "Vai allo storico" on the result. */
    onhistory: () => void;
  } = $props();

  onMount(async () => {
    try {
      run = await runSync(await save(), previewToken, consent);
    } catch (e) {
      failure = toExplanation(e);
    }
  });
</script>

{#if run || failure}
  <StepResult {run} {failure} onhome={onexit} {onretry} {onhistory} />
{:else}
  <StepRunning {total} {closeRequests} />
{/if}
