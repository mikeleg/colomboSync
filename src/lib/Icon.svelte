<script lang="ts">
  // Paths copied from the prototype's icon set.
  const PATHS = {
    disk: '<rect x="3" y="6" width="18" height="12" rx="2"/><path d="M7 12h6M17 12h.01"/>',
    net: '<rect x="3" y="4" width="18" height="7" rx="1.5"/><rect x="3" y="13" width="18" height="7" rx="1.5"/><path d="M7 7.5h.01M7 16.5h.01"/>',
    folder:
      '<path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"/>',
    check: '<circle cx="12" cy="12" r="10"/><path d="M7.5 12.5l3 3 6-6.5"/>',
    tick: '<path d="M5 12.5l4.5 4.5L19 7.5"/>',
    cross: '<circle cx="12" cy="12" r="10"/><path d="M8 8l8 8M16 8l-8 8"/>',
  };

  let { name }: { name: keyof typeof PATHS } = $props();
</script>

<svg
  viewBox="0 0 24 24"
  fill="none"
  stroke="currentColor"
  stroke-width="2"
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"
>
  {@html PATHS[name]}
</svg>
