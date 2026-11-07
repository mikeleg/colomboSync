<script lang="ts">
  import type { Snippet } from "svelte";
  import { formatDuration, formatNumber, partialText } from "./format";
  import type { Run } from "./types";

  /** The figures of a run, as the Storico rows and the log window show them; `extra` adds one more item at the end. */
  let { run, extra }: { run: Run; extra?: Snippet } = $props();
</script>

<dl class="stats">
  {#if run.partial != null}
    <div><dd>{partialText(run.partial)}</dd></div>
  {:else}
    <div>
      <dt>Nuovi</dt>
      <dd>{formatNumber(run.new)}</dd>
    </div>
    <div>
      <dt>Aggiornati</dt>
      <dd>{formatNumber(run.updated)}</dd>
    </div>
  {/if}
  <div class:d={run.deleted > 0}>
    <dt>Cancellati</dt>
    <dd>{formatNumber(run.deleted)}</dd>
  </div>
  <div>
    <dt>Durata</dt>
    <dd>{formatDuration(run.durationMs)}</dd>
  </div>
  {@render extra?.()}
</dl>

<style>
  .stats {
    display: flex;
    flex-wrap: wrap;
    gap: 6px 22px;
    margin: 0;
    align-items: center;
  }
  .stats div {
    min-width: 0;
  }
  dt {
    display: inline;
    color: var(--fg-2);
    font-size: 15px;
  }
  dt::after {
    content: " ";
  }
  dd {
    display: inline;
    margin: 0;
    font-weight: 700;
  }
  .d dd {
    color: var(--danger);
  }
</style>
