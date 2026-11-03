# XP practices in this repo

How Extreme Programming practices translate into concrete agent behaviour here.

| Practice | What you do |
|---|---|
| **Planning game / small releases** | Work only on one slice from `docs/backlog/colombo-sync/backlog.md`. Its acceptance criteria are the scope. Nothing from "Later". |
| **Test-first (TDD)** | For each acceptance criterion and each test named in the slice: write the failing test, run it, see it fail for the right reason. Then the smallest code to green. Then refactor with tests green. |
| **Simple design** | Pass the tests, reveal intent, no duplication, fewest elements. No interfaces/traits with one implementation, no options nobody asked for. |
| **Refactoring** | Only on green. Rename to domain terms, remove duplication you introduced. Don't restructure code outside the slice. |
| **Continuous integration** | Commit at every green state (`feat(sN): …`, `test(sN): …`, `refactor(sN): …`). Before handing back: full suite green (`npm test`, `cargo test`, `npm run check`) and `npm run portable` builds. |
| **Collective ownership + coding standard** | `cargo fmt`, `cargo clippy -- -D warnings`, prettier defaults. Domain names from the glossary, in English in code, Italian only in UI strings. |
| **Pair programming** | The orchestrator (or `/code-review`) reviews every slice branch before merge. Write commit messages and a short report a reviewer can follow. |
| **Customer on site** | The prototype (`docs/backlog/colombo-sync/prototype.html`) and scope are the customer's voice: copy Italian wording verbatim. If a criterion is ambiguous, stop and report – don't guess. |
| **Sustainable pace** | Finish a slice cleanly rather than half of two. |

## Test pyramid

- **Rust unit tests** (most of them): pure logic – args builder, exit codes, preview parsing, path rules, error mapping, stores with temp dirs.
- **Rust integration tests** (`src-tauri/tests/`): call the real `robocopy.exe` on temp folders (`tempfile`). Windows only, that's fine.
- **Svelte component tests** (vitest + @testing-library/svelte): UI rules – which button shows, which text, disabled states. Mock `@tauri-apps/api/core` `invoke`.
- **Manual smoke** on the portable exe for the slice's main flow.

## Definition of done (per slice)

1. Every acceptance criterion covered by at least one automated test, or explicitly listed as "manual only" in the report.
2. All suites green, clippy clean, portable build OK.
3. Commits on the slice branch; report lists files touched, tests added, anything not done.
