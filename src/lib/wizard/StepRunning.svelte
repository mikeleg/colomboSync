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
  h1 {
    font-size: clamp(32px, 4vw, 44px);
    line-height: 1.05;
    letter-spacing: -0.035em;
  }
  .hn {
    color: var(--accent);
  }
  .runprog {
    margin: 28px 0 22px;
  }
  .rp-top {
    display: flex;
    align-items: baseline;
    gap: 8px 24px;
    flex-wrap: wrap;
  }
  .rp-top b {
    font-size: clamp(96px, 15vw, 180px);
    line-height: 0.9;
    font-weight: 700;
    letter-spacing: -0.06em;
    background: var(--grad-text);
    -webkit-background-clip: text;
    background-clip: text;
    color: transparent;
  }
  .rp-top span {
    font-size: 24px;
    font-weight: 600;
    color: var(--fg-2);
  }
  .bar {
    height: 8px;
    margin-top: 22px;
    border-radius: 999px;
    background: color-mix(in srgb, var(--fg) 10%, transparent);
    overflow: hidden;
  }
  .bar i {
    display: block;
    height: 100%;
    border-radius: 999px;
    background: var(--grad);
    box-shadow: 0 0 24px color-mix(in srgb, var(--accent) 70%, transparent);
    transition: width 0.1s linear;
  }
  .rp-eta {
    margin-top: 14px;
    color: var(--fg);
    font-size: 17px;
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
    font-size: 14px;
    color: var(--fg-2);
    overflow-wrap: anywhere;
    margin: 0 0 22px;
    min-height: 24px;
    padding: 12px 16px;
    border-radius: 14px;
    background: color-mix(in srgb, var(--fg) 5%, transparent);
  }
  .confirm {
    padding: 22px 24px;
    border-radius: var(--r-s);
    background: var(--warn-soft);
    box-shadow: inset 0 0 0 1.5px color-mix(in srgb, var(--warn) 45%, transparent);
    color: var(--on-warn-soft);
  }
  .confirm p {
    font-weight: 700;
    margin-bottom: 14px;
  }
  .btn :global(svg) {
    width: 18px;
    height: 18px;
  }
  .btn.danger {
    background: var(--danger);
    color: var(--on-danger);
  }
  .btn.danger:not(:disabled):hover {
    background: color-mix(in srgb, var(--on-danger) 12%, var(--danger));
  }
  @media (prefers-reduced-motion: reduce) {
    .bar i {
      transition: none;
    }
  }
</style>
