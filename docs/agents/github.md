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
| macos | macos-14 | same checks, then `npx tauri build --bundles app,dmg` (ad-hoc signed, `bundle.macOS.signingIdentity` = `-`, not notarized) | `colombo-sync-macos`: `ColomboSync-macos.zip` (the `.app`) and the `.dmg` |
| linux | ubuntu-22.04 | same checks (after installing `libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf`), then `npx tauri build --bundles appimage` | `colombo-sync-linux`: `*.AppImage` |

`npm run build` runs before cargo because `generate_context!` needs `dist/`.

All three CI jobs run on `windows-latest`, `macos-14` (pinned: `macos-latest` jobs stayed queued with no runner) and `ubuntu-22.04` (the AppImage baseline). `ci.yml` is also `workflow_call`-able: the release workflow uses it as its test gate.

## Releasing (`.github/workflows/release.yml`, spec in `docs/release-plan.md`)
A tag `v*` runs: version check → CI gate on three OS → builds → **draft** release with `ColomboSync-<ver>-windows-x64.exe`, `-macos-universal.dmg`, `-linux-x86_64.AppImage` and `SHA256SUMS.txt`. Notes = `feat`/`fix` commits since the previous `v*` tag + `.github/release-notes-it.md` (edit that file for the Italian install notes). Pre-release flag is set when the version starts with `0.` or contains `-`.

Version rule: `package.json`, `src-tauri/tauri.conf.json` and `src-tauri/Cargo.toml` must be identical. The tag without the `v` must equal that version, or that version plus a pre-release suffix (`v0.1.0-rc.1` is accepted while the files say `0.1.0`; `v0.1.1` is not). The suffix lets you test the pipeline without bumping files; it ends up in the file names.

Steps:
1. Bump the three files to the new version (`npm version X.Y.Z --no-git-tag-version` for `package.json` + lock, edit `tauri.conf.json` and `Cargo.toml` by hand, `cd src-tauri && cargo check` to refresh `Cargo.lock`). Commit `chore(release): X.Y.Z` on `main`, push (CI green).
2. Tag and push the tag: `git tag vX.Y.Z && git push origin vX.Y.Z` (with the token recipe's credential helper).
3. `gh run watch <id> -R mikeleg/colomboSync --exit-status` (run id: `gh run list -R mikeleg/colomboSync --workflow release.yml --limit 1`). Any red job → no release; fix, bump patch, new tag.
4. Smoke checklist from `docs/release-plan.md` on a clean machine per OS with the **release files**, not local builds. Note the result as a comment on the draft.
5. Publish the draft by hand (GitHub → Releases → Edit → Publish; keep "pre-release" for 0.x).
6. The owner downloads the files and `SHA256SUMS.txt` and shares them privately with the Italian install notes (repo stays private).

Dry run without a tag: `gh workflow run release.yml -R mikeleg/colomboSync --ref <branch>`; builds and uploads workflow artifacts, creates no release. GitHub only accepts `workflow_dispatch` for a workflow file that is on `main`, so until it is merged add a temporary `push: branches: [<branch>]` trigger to test a branch (a non-tag run takes the same dry-run path). macOS runner queues can last long or time out ("job was not acquired"): `gh run rerun <id> --failed`. A bad throwaway tag/draft is removed with `gh release delete <tag> --cleanup-tag -R mikeleg/colomboSync` (and `git tag -d <tag>`).

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
