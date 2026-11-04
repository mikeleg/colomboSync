<script lang="ts">
  import { onMount } from "svelte";
  import Icon from "../Icon.svelte";
  import { cancelRun, onRunProgress } from "../api";
  import { etaText, formatNumber } from "../format";
  import type { RunProgress } from "../types";

  /**
   * `total` comes from the preview, so "0 di N file" shows before the first event.
   * `closeRequests` counts the clicks on the window's X during the run; the Wizard listens from
   * its mount on, so none is lost while this view is still starting.
   */
  let { total, closeRequests = 0 }: { total: number; closeRequests?: number } = $props();

  let progress = $state<RunProgress | null>(null);
  let confirming = $state(false);
  let closingWindow = $state(false);
  let stopping = $state(false);
  let announcement = $state("");
  let announcedPercent = 0;

  const done = $derived(progress?.done ?? 0);
  const percent = $derived(total > 0 ? Math.min(100, Math.floor((done / total) * 100)) : 0);

  // Screen readers hear the progress every 10%, not every file.
  $effect(() => {
    if (percent >= announcedPercent + 10) {
      announcedPercent = percent - (percent % 10);
      announcement = `Copia al ${announcedPercent}%`;
    }
  });

  $effect(() => {
    if (closeRequests > 0) {
      confirming = true;
      closingWindow = true;
    }
  });

  onMount(() => {
    const unlisten = onRunProgress((p) => (progress = p));
    return () => {
      unlisten.then((stop) => stop());
    };
  });

  function keepCopying() {
    confirming = false;
    closingWindow = false;
  }

  async function stop() {
    stopping = true;
    let stopped = false;
    try {
      stopped = await cancelRun(closingWindow);
    } catch {
      // Stopping failed: the copy goes on, so the buttons come back.
    }
    // Nothing left to stop means the run just ended: its result replaces this view.
    if (!stopped) {
      stopping = false;
      keepCopying();
    }
  }

  const focus = (node: HTMLElement) => node.focus();
</script>

<div class="kicker">In corso</div>
<h1 id="h">Sto copiando{#if progress}{" "}<span class="hn">{progress.syncName}</span>{/if}…</h1>
<p class="lead">
  Non spegnere il computer e non scollegare i dischi finché non ho finito. Puoi fermarti quando vuoi: i file già
  copiati restano in B.
</p>
<div class="runprog">
  <div class="rp-top"><b>{percent}%</b><span>{formatNumber(done)} di {formatNumber(total)} file</span></div>
  <div
    class="bar"
    role="progressbar"
    aria-label="Avanzamento della copia"
    aria-valuemin="0"
    aria-valuemax="100"
    aria-valuenow={percent}
  >
    <i style:width="{percent}%"></i>
  </div>
  <p class="rp-eta">{etaText(done, total, progress?.elapsedMs ?? 0)}</p>
  <p class="sr" aria-live="polite">{announcement}</p>
</div>
<div class="curfile" aria-hidden="true">{progress?.currentFile ? `Sto copiando: ${progress.currentFile}` : ""}</div>
{#if confirming}
  <div class="confirm" role="alertdialog" aria-labelledby="cq">
    <p id="cq">Vuoi fermare la copia? I file già copiati restano in B.</p>
    <div class="btns">
      <button type="button" class="btn danger" disabled={stopping} onclick={stop}>Sì, ferma</button>
      <button type="button" class="btn" disabled={stopping} onclick={keepCopying} use:focus>No, continua</button>
    </div>
  </div>
{:else}
  <button type="button" class="btn" onclick={() => (confirming = true)} use:focus><Icon name="stop" />Ferma la copia</button>
{/if}

<style>
  .runprog {
    padding: 20px 22px;
    background: var(--card-2);
    margin-bottom: 14px;
  }
  .rp-top {
    display: flex;
    align-items: baseline;
    gap: 14px;
    flex-wrap: wrap;
  }
  .rp-top b {
    font-size: 44px;
    line-height: 1;
    font-weight: 600;
    color: var(--accent);
  }
  .rp-top span {
    font-size: 17px;
    font-weight: 600;
    color: var(--fg-2);
  }
  .bar {
    height: 18px;
    margin-top: 14px;
    background: var(--card-3);
    overflow: hidden;
  }
  .bar i {
    display: block;
    height: 100%;
    background: var(--accent);
    transition: width 0.1s linear;
  }
  .rp-eta {
    margin-top: 10px;
    color: var(--fg-2);
    font-weight: 600;
  }
  .sr {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
    white-space: nowrap;
  }
  .curfile {
    font-family: ui-monospace, Consolas, monospace;
    font-size: 15px;
    color: var(--fg-2);
    overflow-wrap: anywhere;
    margin: 0 0 22px;
    min-height: 24px;
    padding: 12px 16px;
    background: var(--card-2);
  }
  .confirm {
    padding: 20px 22px;
    background: var(--warn-soft);
    color: var(--on-warn-soft);
  }
  .confirm p {
    font-weight: 700;
    margin-bottom: 12px;
  }
  .btn.danger {
    background: var(--danger);
    color: var(--card);
  }
  .btn.danger:not(:disabled):hover {
    background: color-mix(in srgb, var(--card) 10%, var(--danger));
  }
  @media (prefers-reduced-motion: reduce) {
    .bar i {
      transition: none;
    }
  }
</style>
