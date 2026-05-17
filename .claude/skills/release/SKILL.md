---
name: release
description: >
  Run the full release workflow: validate branch, test, bump version,
  generate changelog, tag, push, and publish to all editor marketplaces.
  Asks for patch/minor/major if not specified.
allowed-tools: Bash, Read, Write, Edit, Glob, Grep, AskUserQuestion
---

# Release

Orchestrates a full release of tcl-lsp. This skill is internal to the project.

## Workflow

Follow these steps **in order**. Stop and report on failure at any step.

### 1. Branch guard

```bash
branch=$(git branch --show-current)
if [ "$branch" != "main" ]; then
  echo "ERROR: Must be on 'main' branch to release (currently on '$branch')"
  exit 1
fi
```

Fail immediately if not on `main`. Do not offer to switch branches.

### 2. Pull latest

```bash
git pull origin main
```

### 3. Pre-release validation

Run the fast gate first, then slow tests:

```bash
make prep-pr
```

If `prep-pr` applies formatting changes, commit them:

```bash
git add -A && git commit -m "Pre-release formatting fixes"
```

Then run slow tests:

```bash
make test-slow
```

If any test fails, investigate and fix the issue. After fixing, re-run the
failing target. Once all tests pass, commit fixes and push:

```bash
git push origin main
```

### 4. Determine version bump

Check `$ARGUMENTS` for `patch`, `minor`, or `major`. If not provided, ask the
user with `AskUserQuestion`:

> What type of version bump? (patch / minor / major)

Compute the new version from the latest git tag:

```bash
prev_tag=$(git describe --tags --abbrev=0)
prev_version=${prev_tag#v}
```

Split `prev_version` into MAJOR.MINOR.PATCH and apply the bump:

- `patch`: increment PATCH
- `minor`: increment MINOR, reset PATCH to 0
- `major`: increment MAJOR, reset MINOR and PATCH to 0

### 5. Generate changelog

Generate a changelog from the **source diff** between the previous tag and
HEAD, not from the git log:

```bash
git diff "$prev_tag"..HEAD -- '*.py' '*.ts' '*.rs' '*.toml' '*.json' '*.tcl' \
  ':!**/package-lock.json' ':!**/Cargo.lock'
```

Analyse the diff and write a concise `RELEASE_NOTES.md` at the repository root
with these sections (omit empty sections):

```markdown
# vX.Y.Z

## New Features
- ...

## Improvements
- ...

## Bug Fixes
- ...

## Breaking Changes
- ...
```

Focus on user-visible changes. Group related changes. Use UK spelling. Do not
list every file touched; summarise the meaningful changes. Commit the file:

```bash
git add RELEASE_NOTES.md
git commit -m "Add release notes for vX.Y.Z"
```

### 6. Bump version, tag, and push

```bash
make release-tag V=X.Y.Z
```

This runs `scripts/release.sh` which bumps all version files, commits, creates
an annotated tag `vX.Y.Z`, and pushes with `--follow-tags`.

### 6.5. Verify published artefacts

After `make release-tag` pushes the tag, CI builds and uploads every
artefact, then the `publish-checksums` job aggregates them into a
`SHA256SUMS` file (cosign-signed when keyless OIDC is enabled). The
installer (`scripts/install.sh`) verifies downloads against this file.

**Wait for CI to finish, then verify locally** before publishing to any
editor marketplace. A SUMS mismatch means an artefact was modified
after upload or the build was non-reproducible — either way, **do not
proceed to step 7**.

```bash
tag="vX.Y.Z"
mkdir -p /tmp/release-verify
cd /tmp/release-verify

# Pull SUMS and every artefact
gh release download "$tag" --clobber

# Verify all hashes
if command -v sha256sum >/dev/null 2>&1; then
    sha256sum -c SHA256SUMS
else
    shasum -a 256 -c SHA256SUMS
fi

# (Optional) verify the cosign keyless signature
if [ -f SHA256SUMS.cosign.bundle ] && command -v cosign >/dev/null 2>&1; then
    cosign verify-blob \
        --bundle SHA256SUMS.cosign.bundle \
        --certificate-identity-regexp "^https://github.com/bitwisecook/tcl-lsp/\.github/workflows/.+@refs/tags/" \
        --certificate-oidc-issuer "https://token.actions.githubusercontent.com" \
        SHA256SUMS
fi

cd - && rm -rf /tmp/release-verify
```

Also smoke-test the installer one-liner from a clean shell:

```bash
curl -fsSL "https://github.com/bitwisecook/tcl-lsp/releases/download/$tag/install.sh" \
  | TCL_LSP_PREFIX=/tmp/verify-bin TCL_LSP_ASSUME_NO=1 sh
ls -la /tmp/verify-bin/
rm -rf /tmp/verify-bin/
```

The installer aborts by default when `SHA256SUMS` is missing — this is
the safety net for a CI regression that drops the `publish-checksums`
job. If the smoke-test succeeds, integrity is intact.

### 7. Editor publishing

Ask the user which editors to publish to using `AskUserQuestion`:

> Which editors should be published? (All / None / comma-separated list of: vscode, jetbrains, sublime, zed)
> Default: None

Based on the response:

- **None** (default): Skip publishing entirely.
- **All**: Run `make publish-all`.
- **Specific editors**: Run the corresponding `make publish-<editor>` targets.

Available targets:
- `make publish-vsix` — VS Code Marketplace. Runs `vsce publish`. Requires
  a valid PAT for `bitwisecook` (interactive login otherwise).
- `make publish-jetbrains` — JetBrains Marketplace. Runs
  `./gradlew publishPlugin`. Requires `JETBRAINS_TOKEN` env var. The
  first-ever publish must be done interactively via the JetBrains web
  UI; `publishPlugin` only updates an already-listed plugin.
- `make publish-sublime` — Sublime Text / Package Control. Verifies the
  GitHub Release carries the `.sublime-package` asset. After the one-time
  channel submission, Package Control follows tags automatically — no
  per-release marketplace API call.
- `make publish-zed` — Zed extensions registry. Prepares a local
  checkout of `zed-industries/extensions` with the tcl submodule advanced
  to the new tag and the version bumped in `extensions.toml`, then
  **stops** and prints the suggested commit / push / `gh pr create`
  commands. The script never pushes to a fork or opens a PR — the user
  reviews the diff first and raises the PR themselves.

Neovim (`nvim-lspconfig`) and Helix integration are one-time upstream
PRs that the user raises by hand; there is no per-release publish step
or `make publish-*` target for them.

The make targets in this repository never push to or open PRs against
external repositories — any external-repo PR (JetBrains first-time
upload, Package Control channel submission, Zed extensions registry,
nvim-lspconfig, Helix) is raised by the user.

### 8. Summary

Print a summary of what was done:

```
Release vX.Y.Z complete.
  Previous version: <prev>
  New version:      X.Y.Z
  Tag:              vX.Y.Z
  Editors published: <list or "none">
```

$ARGUMENTS
