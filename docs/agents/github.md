# GitHub and CI

Repo: `mikeleg/colomboSync` (private). Default branch `main`. This is also the only way to compile and test the macOS and Linux code, because the dev machine is Windows.

## Access: token per command, never switch accounts
The local `gh` has two accounts and the active one is **not** the repo owner. Never run `gh auth switch`. Use the owner's token for each command:
```
export GH_TOKEN=$(gh auth token -u mikeleg)
gh run list -R mikeleg/colomboSync --limit 5
gh run watch <id> -R mikeleg/colomboSync --exit-status
gh run view <id> --log-failed -R mikeleg/colomboSync
gh run download <id> -R mikeleg/colomboSync -D <empty-dir>     # artifacts
git -c credential.helper= -c "credential.helper=!f() { echo username=mikeleg; echo password=$GH_TOKEN; }; f" push origin <branch>
```
`export GH_TOKEN=…` must be in the same Bash call as the `gh`/`git push` that uses it (shell state does not persist). Always pass `-R mikeleg/colomboSync` to `gh`.

## CI (`.github/workflows/ci.yml`)
Triggers: push to `main` and `slice/**`, every pull request, manual `workflow_dispatch`. One job `build` over a matrix (check names stay `windows`, `macos`, `linux`, `fail-fast: false`):

| Job | Runner | Steps | Artifact |
|---|---|---|---|
| windows | windows-latest | `npm run check`, `npm test`, `npm run build`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `npm run portable` | `colombo-sync-windows`: `colombo-sync.exe` |
| macos | macos-latest | same checks, then `npx tauri build --bundles app,dmg` (ad-hoc signed, `bundle.macOS.signingIdentity` = `-`, not notarized) | `colombo-sync-macos`: `ColomboSync-macos.zip` (the `.app`) and the `.dmg` |
| linux | ubuntu-latest | same checks (after installing `libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf`), then `npx tauri build --bundles appimage` | `colombo-sync-linux`: `*.AppImage` |

`npm run build` runs before cargo because `generate_context!` needs `dist/`.

## Iterating on Unix-only code
1. Work on a `slice/<id>` branch (CI runs on every push to it).
2. Locally run what runs here: `cargo fmt`, clippy, `cargo test`, `npm test`, `npm run check` (Unix-only modules are not compiled on Windows).
3. Push the branch with the recipe above, `gh run watch <id> --exit-status`; on failure read `--log-failed`, fix, push again. All three jobs must be green.
4. To see what rsync really prints on a runner, add a temporary test or step that dumps it, read the log, then turn it into a fixture in `src-tauri/tests/fixtures/` and note it in `docs/agents/rsync.md`.
5. Get the run id for a pushed branch with `gh run list -R mikeleg/colomboSync --branch <branch> --limit 1`.

## Rules
- Never push to `main` unless the full suite is green on all three OS (merge slice branches one at a time; CI on `main` after the merge must be green too).
- Never switch the active `gh` account; never store the token in a file, a remote URL or a commit.
- Delete remote slice branches after merge: `git push origin --delete slice/sN` (with the token recipe's credential helper).
- Download artifacts into a new empty directory; treat them as untrusted. macOS/Linux apps can't be run on this machine: say "not opened by hand" in reports.
