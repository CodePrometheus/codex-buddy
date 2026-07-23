---
name: release
description: Release codex-buddy from the current main-branch changes. Use when the user says "发版", "release", "publish a version", asks to create or push a version tag, or asks to verify a codex-buddy GitHub Release. Covers SemVer selection, local validation, scoped commits, version bumping, tag publication, GitHub Actions monitoring, and release-asset verification.
---

# Release codex-buddy

Publish the next codex-buddy version through the repository's existing cargo-dist and tray-app workflows. Treat the release as complete only after both workflows and all expected assets are verified.

## Guardrails

- Use this skill from the beginning of every codex-buddy release, including release retries and verification-only requests.
- Work only in the `CodePrometheus/codex-buddy` checkout. Verify the repository and read `CLAUDE.md` before acting.
- Treat an unqualified release request as authorization to commit the intended current changes, push `main`, and push the selected version tag. Do not include unrelated changes.
- Never rewrite published history, move an existing tag, delete a tag or release, or rerun or manually dispatch a failed workflow without explicit user approval.
- Report partial state truthfully. A pushed tag is not a completed release.
- Never print credentials or token values.

## 1. Inspect state

Run read-only checks first:

```bash
git remote -v
git status --short --branch
git diff
git fetch origin --tags --prune
git log --oneline --decorate -n 12
git tag --sort=-version:refname | head -n 10
gh auth status
gh release list --limit 10
```

Require:

- branch `main`;
- no unreviewed or unrelated local changes;
- local `main` not behind `origin/main`;
- authenticated GitHub access;
- the target version absent locally and remotely.

For a small compatible fix, select the next patch after the highest stable `vX.Y.Z` tag. Ask before choosing a minor, major, or prerelease version when intent is ambiguous.

## 2. Prepare the release

Keep implementation and version commits separate when there are uncommitted product changes:

1. Stage only the exact intended files and create a conventional `fix:`, `feat:`, `docs:`, or `chore:` commit.
2. Update only `[workspace.package].version` in `Cargo.toml`.
3. Do not add `Cargo.lock`; this repository intentionally ignores it.
4. Commit the version change as `chore: bump version to X.Y.Z`.

If the product changes are already committed, create only the version-bump commit. Do not amend or squash existing commits unless asked.

## 3. Validate locally

Run all required checks before pushing:

```bash
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --all -- --check
swift test --package-path apps/tray
git diff --check
git status --short --branch
```

Stop on any failure. Diagnose it and obtain approval before changing work outside the requested release scope.

## 4. Publish

Push `main` before creating the lightweight version tag, matching repository history:

```bash
git push origin main
git tag "vX.Y.Z"
git push origin "vX.Y.Z"
```

The tag push triggers `.github/workflows/release.yml`. Its success triggers `.github/workflows/tray-app.yml`. Never create a GitHub Release manually while the automatic workflow is running.

## 5. Monitor workflows

Resolve the Release run by exact tag and tagged commit, then watch its actual run ID:

```bash
gh run list --workflow release.yml --limit 20 \
  --json databaseId,headBranch,headSha,status,conclusion,url
gh run watch <release-run-id> --exit-status --compact
```

After it succeeds, find the `tray-app.yml` `workflow_run` whose `headSha` is the tagged commit and wait for it too:

```bash
gh run list --workflow tray-app.yml --limit 20 \
  --json databaseId,event,headSha,status,conclusion,url
gh run watch <tray-run-id> --exit-status --compact
```

Do not confuse an older successful run with the current release. If a workflow fails, inspect it with `gh run view <id> --log-failed`, report the root failure, and stop before any retry or cleanup.

## 6. Verify the GitHub Release

Inspect the final release:

```bash
gh release view "vX.Y.Z" \
  --json isDraft,isPrerelease,publishedAt,url,assets
```

For a stable release, require `isDraft: false`, `isPrerelease: false`, and these 12 assets:

- `codex-buddy-aarch64-apple-darwin.tar.xz`
- `codex-buddy-aarch64-apple-darwin.tar.xz.sha256`
- `codex-buddy-x86_64-apple-darwin.tar.xz`
- `codex-buddy-x86_64-apple-darwin.tar.xz.sha256`
- `codex-buddy-installer.sh`
- `codex-buddy.rb`
- `dist-manifest.json`
- `sha256.sum`
- `source.tar.gz`
- `source.tar.gz.sha256`
- `Codex-Buddy-arm64-macOS.zip`
- `Codex-Buddy-x86_64-macOS.zip`

Confirm the local worktree is clean and `main` matches `origin/main`.

## Final report

Lead with success or the exact partial state. Include:

- published version and release URL;
- product and version-bump commit hashes;
- Release and Tray app workflow conclusions;
- asset count and missing assets, if any;
- local branch and worktree status.
