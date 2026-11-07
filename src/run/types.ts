/** The `run-progress` event while a Run is going. */
export interface RunProgress {
  syncName: string;
  done: number;
  total: number;
  elapsedMs: number;
  currentFile: string;
}
