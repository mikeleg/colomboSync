<script lang="ts">
  import { modeChip, outcomeChip } from "./format";
  import type { Run } from "./types";

  /** The mode and outcome chips of a run, as the Storico rows and the log window show them (the parent lays them out). */
  let { run }: { run: Run } = $props();
</script>

<span class="chip {modeChip[run.mode].cls}">{modeChip[run.mode].label}</span>
<span class="chip {outcomeChip[run.outcome].cls}">{outcomeChip[run.outcome].label}</span>
