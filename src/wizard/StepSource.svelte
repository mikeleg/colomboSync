<script lang="ts">
  import FolderPicker from "./FolderPicker.svelte";
  import type { Explanation } from "../shared/types";

  let {
    path = $bindable(),
    error,
    onenter,
  }: { path: string; error: Explanation | null; onenter: () => void } = $props();
</script>

<FolderPicker
  id="path-a"
  label="Clicca sulla cartella da copiare (A)"
  placeholder="Per esempio: D:\Foto"
  bind:value={path}
  {error}
  {onenter}
/>
