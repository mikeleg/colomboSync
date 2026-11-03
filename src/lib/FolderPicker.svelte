<script lang="ts">
  import { chooseFolder } from "./api";

  let {
    id,
    label,
    placeholder,
    value = $bindable(),
    error,
    onenter,
  }: {
    id: string;
    label: string;
    placeholder: string;
    value: string;
    error: string;
    onenter: () => void;
  } = $props();

  async function browse() {
    const chosen = await chooseFolder();
    if (chosen) value = chosen;
  }
</script>

<label class="f" for={id}>{label}</label>
<div class="row">
  <input
    type="text"
    {id}
    bind:value
    {placeholder}
    autocomplete="off"
    spellcheck="false"
    aria-invalid={error ? "true" : "false"}
    onkeydown={(e) => e.key === "Enter" && onenter()}
  />
  <button type="button" class="btn" onclick={browse}>Sfoglia…</button>
</div>
<p class="help">Scrivi il percorso, oppure scegli la cartella con il pulsante.</p>
{#if error}<div class="err-msg" role="alert">{error}</div>{/if}

<style>
  .row {
    display: flex;
    gap: 12px;
  }
  .row .btn {
    flex: none;
    min-height: 56px;
  }
</style>
