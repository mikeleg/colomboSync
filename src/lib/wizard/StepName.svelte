<script lang="ts">
  import { tick } from "svelte";

  let {
    name = $bindable(),
    error,
    onedit,
    onenter,
  }: { name: string; error: string; onedit: () => void; onenter: () => void } = $props();

  let input: HTMLInputElement;

  $effect(() => {
    if (error) tick().then(() => input.focus());
  });
</script>

<label class="f" for="name">Nome della sincronizzazione</label>
<input
  type="text"
  id="name"
  bind:value={name}
  bind:this={input}
  autocomplete="off"
  aria-invalid={error ? "true" : "false"}
  oninput={onedit}
  onkeydown={(e) => e.key === "Enter" && onenter()}
/>
<p class="help">Serve solo a riconoscerla nell’elenco. Puoi tenere questo nome o cambiarlo.</p>
{#if error}<div class="err-msg" role="alert">{error}</div>{/if}
