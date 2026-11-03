<script lang="ts">
  import FolderPicker from "../FolderPicker.svelte";

  let { path = $bindable(), error, onenter }: { path: string; error: string; onenter: () => void } = $props();
</script>

<FolderPicker
  id="path-b"
  label="Cartella dove metto la copia (B)"
  placeholder="Per esempio: E:\Backup"
  bind:value={path}
  {error}
  {onenter}
/>
