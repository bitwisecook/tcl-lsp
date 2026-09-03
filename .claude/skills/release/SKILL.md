---
name: release
description: >
  Run the full release workflow: validate the active `rust` release branch,
  benchmark the release and regenerate the release-notes performance graphs,
  write the changelog, land the notes PR, tag, then let CI build and publish —
  approving the marketplace Environments with `gh` from the release laptop.
  Asks for patch/minor/major if not specified. Driven by
  `scripts/release/rust_release.sh`.
allowed-tools: Bash, Read, Write, Edit, Glob, Grep, AskUserQuestion
---

# Release

Internal to the project. Follow the steps **in order**; stop and report on
any failure.

The release is **tag-only**: every version literal derives from the latest
annotated tag (Makefile + `git describe`). Cutting a release is pushing a
`vX.Y.Z` tag — no source bump, no commit on the release branch. The one
exception is `RELEASE_NOTES.md` with the release's benchmark result and
regenerated graphs, which land via a PR before the tag.

## The driver

`scripts/release/rust_release.sh` implements everything except the prose
changelog. Use it; it is what makes two releases come out the same way.

```bash
scripts/release/rust_release.sh next patch      # -> the next version
scripts/release/rust_release.sh preflight X.Y.Z # branch, clean tree, free tag, ordering
#   ...write the prose changelog (step 4)
scripts/release/rust_release.sh prepare X.Y.Z   # preflight + bench + graphs + notes + verify + local commit
#   ...push the branch, open + merge the notes PR, pull `rust`
scripts/release/rust_release.sh tag X.Y.Z       # re-verifies, then tag.sh
```

`prepare` stops at a local commit and prints the push / PR commands — read
the diff first.

## One branch, two channels

Every release is cut from `rust`; the tag selects the channel.
`scripts/release/prerelease.sh` is the single decider — pre-release when
major ≥ 2 and minor is odd (`v2.1.8` pre-release, `v2.2.0` stable) — and CI
reads it for the GitHub release flag and the marketplace channel; nothing is
passed by hand.

| Line | Channel | GitHub release |
|---|---|---|
| even minor | VS Code / Open VSX / JetBrains stable | latest |
| odd minor | VS Code / Open VSX pre-release, JetBrains eap | pre-release, never latest |

`legacy-py` (the Python 1.x line) is a locked archive: never branch, merge,
or tag from it.

## Workflow

### 1. Guard and pull

```bash
[ "$(git branch --show-current)" = rust ] || { echo "ERROR: release from rust"; exit 1; }
git pull origin rust
```

### 2. Validate

Every PR passed `make prep-pr` before merge and CI ran the deep suites on the
push. If you doubt the tip is green, run once before tagging:

```bash
make check-all
make test-ext test-rust runtime-rust-test test-emacs
```

### 3. Version

Take `patch` / `minor` / `major` from `$ARGUMENTS`, else ask with
`AskUserQuestion`. Then:

```bash
scripts/release/rust_release.sh next <bump>       # prints X.Y.Z
scripts/release/rust_release.sh preflight X.Y.Z   # branch, clean worktree, tag free locally and on origin, newer than latest
```

### 4. Changelog (prose only)

Generate from the **source diff** since the previous tag, not the git log:

```bash
prev_tag=$(git describe --tags --abbrev=0)
git diff "$prev_tag"..HEAD -- '*.rs' '*.ts' '*.kt' '*.toml' '*.json' '*.tcl' '*.tclspec' \
  ':!**/package-lock.json' ':!**/Cargo.lock'
```

Write `RELEASE_NOTES.md` at the root: `# vX.Y.Z`, then `## New Features`,
`## Improvements`, `## Bug Fixes`, `## Breaking Changes` (omit empty
sections). User-visible changes, grouped, UK spelling, no file lists.
**Never write the `## Performance` section** — step 5 generates it, and a
hand-edited one is how a release ships the previous release's graphs.

### 5. Benchmark, graphs, notes, commit

```bash
scripts/release/rust_release.sh prepare X.Y.Z
```

Runs preflight, a release build of `tcl-lsp-server`, the pinned-corpus
benchmark into `scripts/perf/results/X.Y.Z.json`, the `MANIFEST.toml` entry,
a re-render of `scripts/perf/graphs/` with X.Y.Z highlighted, the
`## Performance` section with its four release-asset URLs, `verify` (re-render
and diff against the committed graphs), then a local commit on
`release/vX.Y.Z`. Report rather than paper over:

- **Measurement host.** A warning that this release was measured on a
  different machine than the last means wall time and CPU are not
  comparable; the notes say so instead of quoting a delta
  (`scripts/perf/README.md`).
- **`--force`.** An existing `results/X.Y.Z.json` is kept; re-measuring needs
  `scripts/release/perf_release.sh X.Y.Z --force` and a reason.

### 6. Land the notes PR

The release branch is protected; the notes commit lands via a PR **against
`rust`** (a PR against another branch lands the notes on the wrong line):

```bash
git push -u origin release/vX.Y.Z
gh pr create --base rust --title "Release vX.Y.Z notes" --body "..."
```

One PR carries `RELEASE_NOTES.md`, `scripts/perf/results/X.Y.Z.json`, the
regenerated `scripts/perf/graphs/`, and the `MANIFEST.toml` entry — the
graphs only mean something beside the result they were rendered from. Wait
for green CI, ask the user to squash-merge, then:

```bash
git checkout rust && git pull origin rust
```

### 7. Tag

```bash
scripts/release/rust_release.sh tag X.Y.Z     # or: make release-rust-tag V=X.Y.Z
```

It re-runs `verify`, refuses if the notes PR has not merged, then hands off
to `tag.sh` (`make release-tag V=X.Y.Z` is the bare primitive: clean tree,
correct branch, no existing tag, pushes only the tag). The tag push runs
`ci.yml`: artefacts, `publish-checksums`, the GitHub release with the
channel from `prerelease.sh`.

`Permission denied (publickey)` means the SSH agent has no identities
(a locked 1Password). Override the push URL for that one command instead of
reconfiguring git — the `x-access-token@` form escapes the repo's
`insteadOf` rewrite:

```bash
TOKEN=$(gh auth token)
GIT_CONFIG_COUNT=1 \
GIT_CONFIG_KEY_0=remote.origin.pushurl \
GIT_CONFIG_VALUE_0="https://x-access-token:${TOKEN}@github.com/bitwisecook/tcl-lsp.git" \
  make release-tag V=X.Y.Z
```

### 8. Verify the published artefacts

CI aggregates every artefact into `SHA256SUMS` (cosign-signed when keyless
OIDC is on); the installer verifies against it. **Wait for CI, then verify
locally before anything is published.** A mismatch means a modified or
non-reproducible artefact — **do not proceed to step 9**.

```bash
tag="vX.Y.Z"
mkdir -p /tmp/release-verify && cd /tmp/release-verify
gh release download "$tag" --clobber
if command -v sha256sum >/dev/null 2>&1; then sha256sum -c SHA256SUMS; else shasum -a 256 -c SHA256SUMS; fi
if [ -f SHA256SUMS.cosign.bundle ] && command -v cosign >/dev/null 2>&1; then
    cosign verify-blob --bundle SHA256SUMS.cosign.bundle \
        --certificate-identity-regexp "^https://github.com/bitwisecook/tcl-lsp/\.github/workflows/.+@refs/tags/" \
        --certificate-oidc-issuer "https://token.actions.githubusercontent.com" \
        SHA256SUMS
fi
cd - && rm -rf /tmp/release-verify
```

Then smoke the installer from a clean shell:

```bash
bash scripts/release/smoke_installer.sh "$tag"
```

It pins `TCL_LSP_VERSION` to the tag (an unpinned run installs the latest
*stable* and verifies the wrong version), checks every installed artefact
against `SHA256SUMS`, `tcl --version` / `f5 --version` report the release,
`--help` exits 0, the MCP server answers `initialize` with the released
`serverInfo` (it takes no flags), and at least `MIN_SKILLS` (default 22)
Claude skills installed. Knobs: `TCL_LSP_PREFIX`, `TCL_LSP_OS`, `MIN_SKILLS`,
`KEEP_PREFIX=1`. All checks pass before step 9.

### 9. Publish

**VS Code, Open VSX, and JetBrains publish from CI.** The tag run pauses at
`publish-vsix-marketplace`, `publish-vsix-openvsx`, and
`publish-jetbrains-marketplace` on their protected Environments
(`marketplace-vscode` / `marketplace-openvsx` / `marketplace-jetbrains`);
approving is what ships. Approve all three with `gh` once step 8 passed:

```bash
tag="vX.Y.Z"
# The run for the *tag*, not the branch push with the same head commit.
run=$(gh run list --workflow ci.yml --limit 20 --json databaseId,headBranch,event \
        --jq "[.[] | select(.headBranch==\"$tag\" and .event==\"push\")][0].databaseId")
gh api "repos/bitwisecook/tcl-lsp/actions/runs/$run/pending_deployments" \
  --jq '.[] | "\(.environment.id)  \(.environment.name)  can_approve=\(.current_user_can_approve)"'
ids=$(gh api "repos/bitwisecook/tcl-lsp/actions/runs/$run/pending_deployments" --jq '.[].environment.id')
gh api "repos/bitwisecook/tcl-lsp/actions/runs/$run/pending_deployments" \
  -X POST $(for i in $ids; do printf ' -F environment_ids[]=%s' "$i"; done) \
  -f state=approved -f comment="Artefacts verified against SHA256SUMS ($tag)"
# The POST returns a list; confirm by re-reading rather than parsing it.
gh api "repos/bitwisecook/tcl-lsp/actions/runs/$run/pending_deployments" --jq 'length'  # -> 0
gh run watch "$run" --exit-status
```

`current_user_can_approve=false` means this `gh` account is not a reviewer
on that Environment — report it, do not work around it. The laptop targets
`make publish-vsix` / `publish-openvsx` / `publish-jetbrains` are fallbacks
only when a CI job itself failed (keyless `az login` for vsce; `OVSX_PAT`;
`JETBRAINS_TOKEN`, first-ever upload is manual in the web UI).

Sublime needs no step: Package Control resolves the `TclLsp.sublime-package`
asset on the release (`make publish-verify` checks it is there; registering
the channel entry is a one-time PR the user raises —
`editors/sublime-text/SUBMITTING.md`). Neovim and Helix are one-time
upstream PRs by the user.

What remains is **Zed**. Run `make publish-verify` (`[ok]` / `[warn]` /
`[fail]` per editor; non-zero only on `[fail]`), then ask with
`AskUserQuestion`:

> Which editors should be published? (All / None / zed) — default None

- **None** — skip. **All** or **zed** — `make publish-zed`: prepares a local
  checkout of `zed-industries/extensions` with the submodule advanced to the
  tag and the version bumped, then **stops** and prints the commit / push /
  `gh pr create` commands; the user reviews and raises the PR. Never run
  `make publish-all` — it re-publishes what CI already shipped.

The make targets push only to repositories the maintainer owns; every
external-repo PR is raised by the user.

### 10. Summary

```
Release vX.Y.Z complete.
  Previous version: <prev>
  New version:      X.Y.Z
  Tag:              vX.Y.Z
  Benchmarked on:   <host from results/X.Y.Z.json>
  Editors published: <list or "none">
```

Say if the benchmark host differed from the previous release's — it is why
the notes quote no delta.

$ARGUMENTS
