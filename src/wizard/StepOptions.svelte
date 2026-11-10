<script lang="ts">
  import Warnbox from "../shared/Warnbox.svelte";

  let { mirror = $bindable() }: { mirror: boolean } = $props();
</script>

{#if !mirror}
  <p class="callout">I file nuovi e modificati vengono copiati in B. Non cancello mai niente in B.</p>
{/if}
<details class="adv">
  <summary
    >Opzioni avanzate {#if mirror}<span class="chip mirror">Specchio attivo</span>{/if}</summary
  >
  <div class="advbody">
    <label class="sw">
      <input type="checkbox" role="switch" bind:checked={mirror} />
      <span>Rendi B identico ad A (specchio)</span>
    </label>
    {#if mirror}
      <Warnbox>
        <h3>Attenzione: questa opzione cancella dei file</h3>
        <p>Ogni file che c’è in B ma non in A verrà <b>CANCELLATO</b> da B. Non finisce nel Cestino.</p>
        <p>Usalo solo se B serve esclusivamente come copia di A.</p>
        <p>Prima di partire vedrai l’elenco dei file che verranno cancellati.</p>
      </Warnbox>
    {/if}
  </div>
</details>

<style>
  .callout {
    display: flex;
    gap: 12px;
    align-items: center;
    padding: 18px 20px;
    border-radius: var(--r);
    background: var(--ok-soft);
    color: var(--on-ok-soft);
    font-weight: 600;
    margin-bottom: 18px;
    font-size: 17.5px;
  }
  .callout::before {
    content: "\2713";
    flex: none;
    width: 36px;
    height: 36px;
    border-radius: 50%;
    display: grid;
    place-items: center;
    background: var(--ok);
    color: var(--on-ok);
    font-weight: 800;
  }
  .adv {
    border-radius: var(--r);
    background: var(--card);
    box-shadow: var(--shadow);
  }
  .adv > summary {
    display: flex;
    flex-wrap: wrap;
    gap: 8px 12px;
    align-items: center;
    min-height: 58px;
    padding: 8px 20px;
    font-weight: 700;
    cursor: pointer;
    border-radius: var(--r);
    list-style: none;
  }
  .adv > summary::-webkit-details-marker {
    display: none;
  }
  .adv > summary::before {
    content: "";
    width: 9px;
    height: 9px;
    border-right: 2.5px solid var(--fg-2);
    border-bottom: 2.5px solid var(--fg-2);
    transform: rotate(-45deg);
    margin-right: 4px;
  }
  .adv[open] > summary::before {
    transform: rotate(45deg);
  }
  .adv > summary:hover {
    background: color-mix(in srgb, var(--fg) 5%, transparent);
  }
  .advbody {
    padding: 4px 20px 20px;
  }
  .sw {
    display: flex;
    align-items: center;
    gap: 14px;
    min-height: 52px;
    cursor: pointer;
    font-weight: 700;
  }
  .sw input {
    appearance: none;
    flex: none;
    width: 58px;
    height: 34px;
    border-radius: 999px;
    background: var(--card-3);
    position: relative;
    margin: 0;
    cursor: pointer;
    border: 2px solid var(--line);
  }
  .sw input::after {
    content: "";
    position: absolute;
    top: 3px;
    left: 3px;
    width: 24px;
    height: 24px;
    border-radius: 50%;
    background: var(--knob);
    box-shadow: var(--knob-shadow);
  }
  .sw input:checked {
    background: var(--danger);
    border-color: var(--danger);
  }
  .sw input:checked::after {
    left: 27px;
  }
  @media (prefers-reduced-motion: no-preference) {
    .sw input::after,
    .adv > summary::before {
      transition:
        left 0.15s,
        transform 0.15s;
    }
  }
</style>
