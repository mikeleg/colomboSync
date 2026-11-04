# XP practices in this repo

How Extreme Programming practices translate into concrete agent behaviour here.

| Practice | What you do |
|---|---|
| **Planning game / small releases** | Work only on one slice from `docs/backlog/colombo-sync/backlog.md`. Its acceptance criteria are the scope. Nothing from "Later". |
| **Test-first (TDD)** | For each acceptance criterion and each test named in the slice: write the failing test, run it, see it fail for the right reason. Then the smallest code to green. Then refactor with tests green. |
| **Simple design** | Pass the tests, reveal intent, no duplication, fewest elements. No interfaces/traits with one implementation, no options nobody asked for. |
| **Refactoring** | Only on green. Rename to domain terms, remove duplication you introduced. Don't restructure code outside the slice. |
| **Continuous integration** | Commit at every green state (`feat(sN): …`, `test(sN): …`, `refactor(sN): …`). Before handing back: full suite green (`npm test`, `cargo test`, `npm run check`), `npm run portable` builds, and the slice branch is green on GitHub Actions for Windows, macOS and Linux (`docs/agents/github.md`). |
| **Collective ownership + coding standard** | `cargo fmt`, `cargo clippy -- -D warnings`, prettier defaults. Domain names from the glossary, in English in code, Italian only in UI strings. |
| **Pair programming** | `/code-review` runs on every slice branch before merge, no exceptions: it found real data-loss bugs in every slice (path rules incl. A inside B, links/aliases, normalization, MirrorGuard trusting the UI, overwriting unreadable files, non-atomic writes). Each finding gets a test. Write commit messages and a short report a reviewer can follow. |
| **Customer on site** | The prototype (`docs/backlog/colombo-sync/prototype.html`) and scope are the customer's voice: copy Italian wording verbatim. If a criterion is ambiguous, stop and report – don't guess. |
| **Sustainable pace** | Finish a slice cleanly rather than half of two. |

## Test pyramid

- **Rust unit tests** (most of them): pure logic – args builder, exit codes, preview parsing, path rules, error mapping, stores with temp dirs.
- **Rust integration tests** (`src-tauri/tests/`): call the real engine on temp folders (`tempfile`): `robocopy_integration.rs` (Windows only) and `rsync_integration.rs` (macOS/Linux; compiled and run only on CI from a Windows machine). Fixtures in `src-tauri/tests/fixtures/` are real captures.
- **Svelte component tests** (vitest + @testing-library/svelte): UI rules – which button shows, which text, disabled states. Mock `@tauri-apps/api/core` `invoke`.
- **Manual smoke** on the portable exe for the slice's main flow (macOS/Linux bundles come from CI and can't be opened here: say so).
- In TS tests write Windows paths with `String.raw`.

## Definition of done (per slice)

1. Every acceptance criterion covered by at least one automated test, or explicitly listed as "manual only" in the report.
2. All suites green, clippy clean, portable build OK, CI green on all three OS.
3. `/code-review` done and its findings fixed.
4. Commits on the slice branch; report lists files touched, tests added, new Italian copy not in the prototype, anything not done.
