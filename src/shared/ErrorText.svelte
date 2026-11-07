<script lang="ts">
  import type { Explanation } from "./types";

  /** The sentence of an `Explanation` as running text: title, then the advice when there is one. */
  let { explanation }: { explanation: Explanation } = $props();
</script>

{explanation.title}{#if explanation.advice}{" "}{explanation.advice}{/if}
