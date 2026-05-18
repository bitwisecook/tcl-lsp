---
name: release
description: >
  Run the full release workflow: validate branch, test, generate changelog,
  tag, push, and publish to all editor marketplaces. Asks for
  patch/minor/major if not specified. Tag-only — no source-file edits,
  no commit on main (main is protected).
allowed-tools: Bash, Read, Write, Edit, Glob, Grep, AskUserQuestion
---

# Release

Orchestrates a full release of tcl-lsp. This skill is internal to the project.

The release is **tag-only**: every version literal in the tree is derived
from the latest annotated tag (via `hatch-vcs` for the Python wheel and via
the Makefile + `git describe` for every editor build). To cut a release we
push a `vX.Y.Z` tag — there is no source-file bump and no commit on `main`.
RELEASE_NOTES.md is the one exception, and it lands via a PR before tagging.

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

If any test fails, investigate and fix the issue. **Main is protected — push
fixes via a PR.** Create a fix branch, push, open the PR with `gh pr create`,
wait for CI green, and ask the user to merge it. Then `git checkout main &&
git pull origin main` before continuing.

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
list every file touched; summarise the meaningful changes.

### 6. Land RELEASE_NOTES.md, then tag

Main is protected, so the release-notes commit lands via a PR:

```bash
git checkout -b release/vX.Y.Z
git add RELEASE_NOTES.md
git commit -m "Add release notes for vX.Y.Z"
git push -u origin release/vX.Y.Z
gh pr create --title "Release vX.Y.Z notes" --body "..."
```

Wait for CI on the PR to go green, then ask the user to merge it (squash).
Once merged, switch back and pull:

```bash
git checkout main
git pull origin main
```

Now create + push the annotated tag. `make release-tag` handles validation
(clean tree, correct branch, no existing tag) and pushes only the tag —
no source-file edits, no commit on main:

```bash
make release-tag V=X.Y.Z
```

The tag push triggers `.github/workflows/ci.yml` to build artefacts, run
`publish-checksums`, and publish the GitHub release.

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

Also smoke-test the installer one-liner from a clean shell and
verify the installed payload — `scripts/smoke_installer.sh`
wraps the full check matrix:

1. installer exits 0;
2. every installed `.pyz` matches its SHA256SUMS entry;
3. `tcl --version` / `f5 --version` include the released version;
4. `tcl --help` / `f5 --help` exit 0 with non-empty output;
5. MCP zipapp launches and its `--help` banner mentions the version
   (`--version` is intentionally empty there);
6. the Claude-skills directory has at least `MIN_SKILLS` entries
   (default 22).

```bash
bash scripts/smoke_installer.sh "$tag"
```

Env knobs the script honours: `TCL_LSP_PREFIX` (install destination,
default `/tmp/verify-bin`), `TCL_LSP_OS` (forced detection when
`/etc/os-release` isn't readable in this environment), `MIN_SKILLS`,
`KEEP_PREFIX=1` (skip the cleanup so you can poke at a failure).

The installer aborts by default when `SHA256SUMS` is missing — that's
the safety net for a CI regression that drops the `publish-checksums`
job. The post-install checks above catch the next failure modes after
that: a successfully-completed installer that nevertheless landed
stale or version-skewed binaries (e.g. cached artefact, partial
upload, dev build leaking through). All checks must pass before
step 7.

### 7. Editor publishing

Before asking which editors to publish, run a readiness check so any
missing token or unclaimed namespace surfaces *before* the user picks
targets:

```bash
make publish-verify
```

`publish-verify` prints one of `[ok] / [warn] / [fail]` per editor; it
exits non-zero only on `[fail]` (tool missing or remote unreachable).
`[warn]` lines (e.g. `JETBRAINS_TOKEN not set`) are recoverable — note
them back to the user and let them decide whether to set the token now
or skip that target.

Then ask which editors to publish to using `AskUserQuestion`:

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
- `make publish-sublime` — Sublime Text / Package Control. Pushes the
  built `build/sublime-stage/` tree (the same contents that go into the
  `.sublime-package`) to the dedicated mirror repo
  `bitwisecook/tcl-lsp-sublime-text` at the current tag. Package Control
  scrapes the mirror's tags and serves the package source archive
  directly — no marketplace API call, no per-release channel PR. The
  mirror exists because Package Control needs the package contents at
  the root of a git tag, which our monorepo can't satisfy directly.
  One-time setup: the empty mirror repo must exist on GitHub
  (`gh repo create bitwisecook/tcl-lsp-sublime-text --public ...`).
  Override the mirror destination with `TCL_LSP_SUBLIME_MIRROR_REPO`
  and `TCL_LSP_SUBLIME_MIRROR_DIR`; set `TCL_LSP_SUBLIME_DRY_RUN=1` to
  stage the commit + tag locally without pushing.
- `make publish-zed` — Zed extensions registry. Prepares a local
  checkout of `zed-industries/extensions` with the tcl submodule advanced
  to the new tag and the version bumped in `extensions.toml`, then
  **stops** and prints the suggested commit / push / `gh pr create`
  commands. The script never pushes to a fork or opens a PR — the user
  reviews the diff first and raises the PR themselves.

Neovim (`nvim-lspconfig`) and Helix integration are one-time upstream
PRs that the user raises by hand; there is no per-release publish step
or `make publish-*` target for them.

The make targets in this repository **only push to repositories owned
by the maintainer** (the canonical repo, the
`tcl-lsp-sublime-text` mirror). They never push to or open PRs against
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
