<script lang="ts">
  let { onnew }: { onnew: () => void } = $props();
</script>

<h1 id="h">Le tue sincronizzazioni</h1>
<p class="lead">Una sincronizzazione copia i file da una cartella (A) in un’altra (B).</p>
<button type="button" class="btn primary big" onclick={onnew}>+ Nuova sincronizzazione</button>
