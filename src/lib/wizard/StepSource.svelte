<script lang="ts">
  import FolderPicker from "../FolderPicker.svelte";

  let { path = $bindable(), error, onenter }: { path: string; error: string; onenter: () => void } = $props();
</script>

<FolderPicker
  id="path-a"
  label="Cartella da copiare (A)"
  placeholder="Per esempio: D:\Foto"
  bind:value={path}
  {error}
  {onenter}
/>
