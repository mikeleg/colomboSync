/** The `run-progress` event while a Run is going. */
export interface RunProgress {
  syncName: string;
  done: number;
  currentFile: string;
  /** Time left, in words, as Rust says it. */
  eta: string;
}
