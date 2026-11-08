// Runs before every test file (vite.config.ts `test.setupFiles`): the Tauri bridge is always a mock,
// and every test starts with `invoke` forgotten, so it must say what Rust answers.
import { invoke } from "@tauri-apps/api/core";
import { beforeEach, vi } from "vitest";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn(() => Promise.resolve(() => {})) }));

beforeEach(() => {
  vi.mocked(invoke).mockReset();
});
