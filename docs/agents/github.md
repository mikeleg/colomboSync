# GitHub and CI

Repo: `mikeleg/colomboSync` (public as of 2026-11-07). Default branch `main`. This is also the only way to compile and test the macOS and Linux code, because the dev machine is Windows.

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
Triggers: push to `main` and `slice/**`, every pull request, manual `workflow_dispatch`. A push to `main` runs the checks only (no package builds, no artifacts); artifacts come from `slice/**` pushes, pull requests and manual runs. One job `build` over a matrix (check names stay `windows`, `macos`, `linux`, `fail-fast: false`):

| Job | Runner | Steps | Artifact |
|---|---|---|---|
| windows | windows-latest | `npm run check`, `npm run format:check`, `npm test`, `npm run build`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `npm run portable` | `colombo-sync-windows`: `colombo-sync.exe` |
| macos | macos-15 | same checks (incl. `npm run format:check`), then `npx tauri build --bundles app,dmg` (ad-hoc signed, `bundle.macOS.signingIdentity` = `-`, not notarized) | `colombo-sync-macos`: `ColomboSync-macos.zip` (the `.app`) and the `.dmg` |
| linux | ubuntu-latest | same checks (incl. `npm run format:check`, after installing `libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf`), then `npx tauri build --bundles appimage` | `colombo-sync-linux`: `*.AppImage` |

On a push to `main` the package steps (`npm run portable`, `npx tauri build`) and the artifact uploads are skipped; the checks still run. Release mode always builds.

`npm run build` runs before cargo because `generate_context!` needs `dist/`.

All three CI jobs run on `windows-latest`, `macos-15` (explicit label: `macos-latest` jobs stayed queued with no runner on 2026-11-04; `macos-14` ships samba rsync 2.6.9, not openrsync) and `ubuntu-latest` (GNU rsync 3.4.1; the release AppImage is built on `ubuntu-22.04`). The "Which rsync" step prints the rsync each runner has.

`ci.yml` is also `workflow_call`-able with inputs `release` (boolean, default false) and `version`. `release.yml` calls it with `release: true`: the same jobs and checks, but after them each OS builds the **release file** instead of the CI artifact (Windows renamed exe; macOS universal dmg with a retry and a universal-binary assertion; Linux AppImage, on `ubuntu-22.04` instead of `ubuntu-latest` for the glibc baseline) and uploads it as `release-<windows|macos|linux>`. There is one build per release, not a CI build plus a second release build. The macOS runners have no Rosetta (`arch -x86_64` fails), so the Intel half of the universal dmg is never executed; in release mode the macOS job runs `cargo clippy --target x86_64-apple-darwin --all-targets -- -D warnings` so it at least compiles clean.

## Releasing (`.github/workflows/release.yml`, spec in `docs/release-plan.md`)
A tag `v*` runs: version check → `ci.yml` in release mode (checks and builds on three OS) → **draft** release with `ColomboSync-<ver>-windows-x64.exe`, `-macos-universal.dmg`, `-linux-x86_64.AppImage` and `SHA256SUMS.txt`. Notes = `feat`/`fix` commits (a final release, tag without `-`, lists those since the previous final tag, skipping rc tags; an rc lists those since the previous tag) + `.github/release-notes-it.md` (edit that file for the Italian install notes). Pre-release flag is set when the version starts with `0.` or contains `-`.

Version rule: `package.json`, `src-tauri/tauri.conf.json` and `src-tauri/Cargo.toml` must be identical. The tag without the `v` must equal that version, or that version plus a pre-release suffix (`v0.1.0-rc.1` is accepted while the files say `0.1.0`; `v0.1.1` is not). The suffix lets you test the pipeline without bumping files; it ends up in the file names.

Steps:
1. Bump the three files to the new version (`npm version X.Y.Z --no-git-tag-version` for `package.json` + lock, edit `tauri.conf.json` and `Cargo.toml` by hand, `cd src-tauri && cargo check` to refresh `Cargo.lock`). Commit `chore(release): X.Y.Z` on a branch (e.g. `release/X.Y.Z`), open a PR to `main` and merge it with CI green.
2. Tag **the commit on `main`** and push the tag: `git tag vX.Y.Z && git push origin vX.Y.Z` (with the token recipe's credential helper). The `version` job fails a tag whose commit is not an ancestor of `origin/main`: tags never go on slice branches. It checks only that the commit is on main, not that main's CI passed on it; the release-mode CI run (step 3) is the test gate. To try the pipeline on a branch, use a dry run (below), not a tag.
3. `gh run watch <id> -R mikeleg/colomboSync --exit-status` (run id: `gh run list -R mikeleg/colomboSync --workflow release.yml --limit 1`). Any red job → no release; fix, bump patch, new tag.
4. Smoke checklist from `docs/release-plan.md` on a clean machine per OS with the **release files**, not local builds. Note the result as a comment on the draft.
5. Publish the draft by hand (GitHub → Releases → Edit → Publish; keep "pre-release" for 0.x).
6. The owner downloads the files and `SHA256SUMS.txt` and shares them privately with the Italian install notes (repo stays private).

Dry run without a tag: `gh workflow run release.yml -R mikeleg/colomboSync --ref <branch>`; builds and uploads workflow artifacts, creates no release (a non-tag run takes this path; `release.yml` is on `main`, so GitHub accepts the dispatch on any branch). macOS runner queues can last long or time out ("job was not acquired"): `gh run rerun <id> --failed`. A bad throwaway tag/draft is removed with `gh release delete <tag> --cleanup-tag -R mikeleg/colomboSync` (and `git tag -d <tag>`).

## Iterating on Unix-only code
1. Work on a `slice/<id>` branch (CI runs on every push to it).
2. Locally run what runs here: `cargo fmt`, clippy, `cargo test`, `npm test`, `npm run check` (Unix-only modules are not compiled on Windows).
3. Push the branch with the recipe above, `gh run watch <id> --exit-status`; on failure read `--log-failed`, fix, push again. All three jobs must be green.
4. To see what rsync really prints on a runner, add a temporary test or step that dumps it, read the log, then turn it into a fixture in `src-tauri/tests/fixtures/` and note it in `docs/agents/rsync.md`.
5. Get the run id for a pushed branch with `gh run list -R mikeleg/colomboSync --branch <branch> --limit 1`.

## Rules
- `main` changes only through a pull request, for everyone including the owner: a ruleset ("main: solo via pull request") on the default branch requires a PR (0 approvals, since the owner can't approve their own), blocks force-push and deletion, and has no bypass. A direct `git push origin main` is rejected. Merge a PR only with the full suite green on all three OS (one at a time; CI on `main` after the merge must be green too): `gh pr create -R mikeleg/colomboSync --base main --head <branch> --fill`, then `gh pr merge <n> -R mikeleg/colomboSync --merge --delete-branch`. Tags are not covered by the ruleset: release tags are still pushed directly.
- Never switch the active `gh` account; never store the token in a file, a remote URL or a commit.
- Delete remote slice branches after merge: `git push origin --delete slice/sN` (with the token recipe's credential helper).
- Download artifacts into a new empty directory; treat them as untrusted. macOS/Linux apps can't be run on this machine: say "not opened by hand" in reports.
