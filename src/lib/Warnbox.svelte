<script lang="ts">
  import type { Snippet } from "svelte";
  import Icon from "./Icon.svelte";

  let { children }: { children: Snippet } = $props();
</script>

<div class="warnbox" role="alert">
  <Icon name="warn" />
  <div>{@render children()}</div>
</div>

<style>
  .warnbox {
    display: flex;
    gap: 16px;
    margin-top: 16px;
    padding: 20px;
    border-radius: var(--r);
    background: var(--danger-soft);
    color: var(--on-danger-soft);
  }
  .warnbox :global(svg) {
    width: 52px;
    height: 52px;
    padding: 12px;
    border-radius: 50%;
    background: var(--danger);
    color: var(--card);
    flex: none;
  }
  .warnbox > div {
    min-width: 0;
  }
  .warnbox > div :global(h3) {
    font-size: 20px;
    font-weight: 800;
    margin-bottom: 6px;
  }
  .warnbox > div :global(p + p) {
    margin-top: 8px;
  }
</style>
