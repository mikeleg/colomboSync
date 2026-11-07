<script lang="ts">
  import FolderPicker from "./FolderPicker.svelte";
  import type { Explanation } from "../shared/types";

  let {
    path = $bindable(),
    source,
    error,
    onenter,
  }: { path: string; source: string; error: Explanation | null; onenter: () => void } = $props();
</script>

<FolderPicker
  id="path-b"
  label="Clicca sulla cartella dove mettere la copia (B)"
  placeholder="Per esempio: E:\Backup"
  bind:value={path}
  {error}
  {onenter}
  canCreate
  {source}
/>
