#!/usr/bin/env bash
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.
#
# SPDX-License-Identifier: AGPL-3.0-or-later

# rust_release.sh — the 2.1.x pre-release process, as a program rather than a
# procedure someone follows.
#
# Usage:  scripts/release/rust_release.sh <command> [args]
#         make release-prepare V=X.Y.Z
#
#   next [patch|minor|major]  print the version that bump would produce
#   preflight X.Y.Z           check the branch, tree and tag are releasable
#   perf X.Y.Z                measure X.Y.Z and regenerate the graphs
#   notes X.Y.Z               write the performance section of RELEASE_NOTES.md
#   verify X.Y.Z              prove the committed artefacts are self-consistent
#   prepare X.Y.Z             preflight + perf + notes + verify + commit
#   tag X.Y.Z                 check the notes landed, then create + push the tag
#
# Why this exists
# ---------------
# The tag half of releasing was already deterministic: `tag.sh` derives the
# channel from the version and refuses the wrong branch, so `v2.1.20` can only
# mean one thing.  The half before it was not.  The performance figures that go
# into the notes were produced by hand — build a server, remember the right
# bench.py incantation, remember to add the tag to the manifest, remember to
# re-render the graphs, then hand-edit four asset URLs into RELEASE_NOTES.md.
# Every one of those is silent when skipped, and the failure lands in published
# release notes: last release's graphs under this release's heading.
#
# So the whole sequence is one program, each step idempotent and separately
# runnable, with `verify` as the gate that says the committed bytes actually
# agree with each other.
#
# What it deliberately does not do
# --------------------------------
# * Write the prose changelog. That is a judgement about what changed, and a
#   script that emitted it would emit something nobody would want to read.
# * Push the notes branch or open the PR. `prepare` stops at a local commit and
#   prints the two commands, matching `publish_zed.sh` — the maintainer reads
#   the diff before it leaves the laptop. `--push` opts in.
# * Publish anything. Tagging is the trigger; CI does the rest (see
#   docs/design/contracts/release-and-publish.md).
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
HERE="$ROOT/scripts/release"
PERF="$ROOT/scripts/perf"
RELEASE_BRANCH=rust

# Everything a release preparation is allowed to touch — the prose changelog
# plus the three perf artefacts `perf` regenerates. This is both what `prepare`
# stages and what `preflight` tolerates in a modified worktree: the changelog is
# written *before* `prepare` runs (it is a judgement, not something a script can
# derive), so a flat clean-tree requirement would reject the one edit the
# process depends on. Anything outside this list still fails, because a stray
# source change swept into the release commit is exactly what that check is for.
RELEASE_PATHS=(
    RELEASE_NOTES.md
    scripts/perf/results
    scripts/perf/graphs
    scripts/perf/MANIFEST.toml
    rust/tcl-sslictcl/data
)

die() { echo "error: $*" >&2; exit 1; }
step() { echo; echo "==> $*"; }

# Scratch directory for `verify`'s re-render. Cleaned on exit rather than on
# function return so it also goes away when `die` aborts mid-check.
SCRATCH=""
cleanup() { [ -n "$SCRATCH" ] && rm -rf "$SCRATCH"; return 0; }
trap cleanup EXIT

usage() {
    cat <<'EOF'
Usage: scripts/release/rust_release.sh <command> [args]

  next [patch|minor|major]  print the version that bump would produce
  preflight X.Y.Z           check the branch, tree and tag are releasable
  perf X.Y.Z [opts]         measure X.Y.Z and regenerate the graphs
  notes X.Y.Z               write the performance section of RELEASE_NOTES.md
  verify X.Y.Z              prove the committed artefacts are self-consistent
  prepare X.Y.Z [--push]    preflight + perf + notes + verify + commit
  tag X.Y.Z                 check the notes landed, then create + push the tag

The 2.1.x pre-release line only; stable releases are also cut from rust by tag.sh.
See the header of this file, and docs/design/contracts/release-and-publish.md.
EOF
}

# Validate and normalise a version argument.
need_version() {
    local v="${1:-}"
    [ -n "$v" ] || die "missing version (expected X.Y.Z)"
    v="${v#v}"
    [[ "$v" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || die "'$v' is not a version (expected X.Y.Z)"
    printf '%s' "$v"
}

# The 2.1.x line only. An even-minor or 1.x version is a stable release cut
# which this script knows nothing about — and quietly doing the
# rust-line thing to it would put pre-release graphs in stable notes.
require_prerelease_line() {
    local v="$1"
    [ "$(bash "$HERE/prerelease.sh" "$v")" = "true" ] ||
        die "v$v is a stable release (cut from rust via tag.sh). This script drives the
       $RELEASE_BRANCH pre-release line only — see scripts/release/tag.sh."
}

latest_tag() {
    git -C "$ROOT" describe --tags --abbrev=0 2>/dev/null || true
}

# --------------------------------------------------------------------------
# next — compute the version a bump produces
# --------------------------------------------------------------------------

cmd_next() {
    local bump="${1:-patch}"
    local prev
    prev="$(latest_tag)"
    [ -n "$prev" ] || die "no tags found — run: git fetch --tags origin"
    prev="${prev#v}"
    IFS='.' read -r major minor patch <<EOF
$prev
EOF
    case "$bump" in
        patch) patch=$((patch + 1)) ;;
        minor) minor=$((minor + 1)); patch=0 ;;
        major) major=$((major + 1)); minor=0; patch=0 ;;
        *) die "unknown bump '$bump' (expected patch, minor or major)" ;;
    esac
    echo "$major.$minor.$patch"
}

# --------------------------------------------------------------------------
# preflight — is this tree releasable as X.Y.Z at all?
# --------------------------------------------------------------------------

cmd_preflight() {
    local v; v="$(need_version "${1:-}")"
    require_prerelease_line "$v"

    step "Preflight for v$v"

    # `release/vX.Y.Z` is allowed as well as the release branch: `prepare`
    # creates it partway through, so requiring `rust` would make re-running
    # `prepare` — the whole point of the steps being idempotent — fail on the
    # branch its own previous run left you on.
    local branch; branch="$(git -C "$ROOT" branch --show-current)"
    if [ "$branch" != "$RELEASE_BRANCH" ] && [ "$branch" != "release/v$v" ] &&
       [ "${ALLOW_NON_RUST_RELEASE:-0}" != "1" ]; then
        die "on branch '$branch'; 2.x pre-releases are cut from '$RELEASE_BRANCH'
       (or its 'release/v$v' preparation branch).
       Set ALLOW_NON_RUST_RELEASE=1 to override."
    fi
    echo "    branch:   $branch"

    # Modifications are confined to RELEASE_PATHS. Anything else has no business
    # in a release commit, and at tag time `tag.sh` still demands a wholly clean
    # tree — `hatch-vcs` and `git describe` derive every version literal from the
    # tag, so a dirty tree there embeds a `+dev` suffix in artefacts that claim
    # to be the release.
    local excludes=() p
    for p in "${RELEASE_PATHS[@]}"; do excludes+=(":(exclude)$p"); done
    local dirty
    dirty="$(git -C "$ROOT" status --porcelain -uno -- "${excludes[@]}")"
    [ -z "$dirty" ] || die "worktree has changes outside the release artefacts:
$(printf '%s\n' "$dirty" | sed 's/^/         /')
       Commit or stash them. Only these may be modified going into a release:
         ${RELEASE_PATHS[*]}"
    echo "    worktree: clean outside the release artefacts"

    if git -C "$ROOT" rev-parse "v$v" >/dev/null 2>&1; then
        die "tag v$v already exists locally"
    fi
    if git -C "$ROOT" ls-remote --exit-code --tags origin "refs/tags/v$v" >/dev/null 2>&1; then
        die "tag v$v already exists on origin"
    fi
    echo "    tag v$v:  free"

    local prev; prev="$(latest_tag)"
    if [ -n "$prev" ]; then
        # Sort-based rather than arithmetic so 2.1.9 < 2.1.10 comes out right.
        local newest
        newest="$(printf '%s\n%s\n' "${prev#v}" "$v" | sort -V | tail -1)"
        [ "$newest" = "$v" ] || die "v$v is not newer than the latest tag $prev"
        echo "    previous: $prev"
    else
        echo "    previous: (no tags fetched — cannot check ordering)"
    fi

    echo "    channel:  pre-release (GitHub --prerelease, VS Code --pre-release, JetBrains eap)"

    if [ -n "${SSLICTCL_SOURCE_DATA_WAIVER:-}" ]; then
        echo "    SslicTcl source data: freshness waived — $SSLICTCL_SOURCE_DATA_WAIVER"
    else
        step "Checking embedded SslicTcl source data"
        make --no-print-directory check-source-data SOURCE_DATA_MAX_AGE_DAYS=180
    fi
}

# --------------------------------------------------------------------------
# perf / notes — the two artefact-producing steps
# --------------------------------------------------------------------------

cmd_perf() {
    local v; v="$(need_version "${1:-}")"; shift || true
    step "Benchmarking v$v and regenerating the release graphs"
    bash "$HERE/perf_release.sh" "$v" "$@"
}

cmd_notes() {
    local v; v="$(need_version "${1:-}")"
    step "Writing the performance section of RELEASE_NOTES.md for v$v"
    python3 "$HERE/perf_notes.py" "$v"
}

# --------------------------------------------------------------------------
# verify — do the committed artefacts agree with each other?
# --------------------------------------------------------------------------

cmd_verify() {
    local v; v="$(need_version "${1:-}")"
    step "Verifying the release artefacts for v$v"

    [ -f "$PERF/results/$v.json" ] ||
        die "no benchmark result at scripts/perf/results/$v.json — run: $0 perf $v"
    echo "    result:   scripts/perf/results/$v.json"

    grep -q "\"v$v\"" "$PERF/MANIFEST.toml" ||
        die "scripts/perf/MANIFEST.toml does not list v$v — run: $0 perf $v"
    echo "    manifest: lists v$v"

    # The graphs are committed and are quoted as the release record, so they
    # have to be exactly what the committed results render to. report.py is
    # byte-deterministic by design; re-rendering into a scratch directory and
    # diffing is what turns that design into a check.
    SCRATCH="$(mktemp -d)"
    (cd "$PERF" && python3 report.py --results results --out "$SCRATCH" --highlight "$v" >/dev/null)
    diff -r "$PERF/graphs" "$SCRATCH" >/dev/null ||
        die "scripts/perf/graphs/ is not what scripts/perf/results/ renders to with
       v$v highlighted. Re-render it:
         cd scripts/perf && python3 report.py --highlight $v"
    echo "    graphs:   reproduce byte-for-byte, highlighting $v"

    python3 "$HERE/perf_notes.py" "$v" --check
}

# --------------------------------------------------------------------------
# prepare — everything that happens before the tag
# --------------------------------------------------------------------------

cmd_prepare() {
    local v; v="$(need_version "${1:-}")"; shift || true
    local push=0 branch="release/v$v"
    while [ $# -gt 0 ]; do
        case "$1" in
            --push) push=1; shift ;;
            *) die "unknown option for prepare: $1" ;;
        esac
    done

    cmd_preflight "$v"

    if [ -f "$PERF/results/$v.json" ]; then
        echo
        echo "    scripts/perf/results/$v.json already exists — keeping it."
        echo "    (re-measure deliberately with: $0 perf $v --force)"
    else
        cmd_perf "$v"
    fi
    cmd_notes "$v"
    cmd_verify "$v"

    step "Committing the release preparation on $branch"
    if [ "$(git -C "$ROOT" branch --show-current)" != "$branch" ]; then
        git -C "$ROOT" checkout -b "$branch"
    fi
    git -C "$ROOT" add -- "${RELEASE_PATHS[@]}"
    if git -C "$ROOT" diff --cached --quiet; then
        echo "    nothing to commit — already prepared"
    else
        git -C "$ROOT" commit -m "release: prepare v$v performance notes"
        echo "    committed"
    fi

    if [ "$push" -eq 1 ]; then
        step "Pushing $branch"
        git -C "$ROOT" push -u origin "$branch"
    fi

    echo
    echo "Prepared v$v. Remaining steps:"
    echo
    echo "  review the diff:   git show"
    [ "$push" -eq 1 ] || echo "  push the branch:   git push -u origin $branch"
    echo "  open the PR:       gh pr create --base $RELEASE_BRANCH \\"
    echo "                       --title 'Release v$v notes' \\"
    echo "                       --body 'Performance graphs and release notes for v$v'"
    echo "  merge it (squash), then:"
    echo "                     git checkout $RELEASE_BRANCH && git pull origin $RELEASE_BRANCH"
    echo "                     scripts/release/rust_release.sh tag $v"
}

# --------------------------------------------------------------------------
# tag — the point of no return
# --------------------------------------------------------------------------

cmd_tag() {
    local v; v="$(need_version "${1:-}")"
    require_prerelease_line "$v"

    step "Tagging v$v"
    # The notes and graphs must be on the release branch *before* the tag, not
    # after: CI reads RELEASE_NOTES.md from the tagged commit, and perf.yml
    # attaches the graphs to the release the tag creates.
    local branch; branch="$(git -C "$ROOT" branch --show-current)"
    [ "$branch" = "$RELEASE_BRANCH" ] || [ "${ALLOW_NON_RUST_RELEASE:-0}" = "1" ] ||
        die "on '$branch' — tag from '$RELEASE_BRANCH' so the tag carries the merged notes"

    head -1 "$ROOT/RELEASE_NOTES.md" | grep -qx "# v$v" ||
        die "RELEASE_NOTES.md is not headed '# v$v'. The notes PR has not merged
       into $RELEASE_BRANCH yet — merge it, pull, and retry."
    cmd_verify "$v"

    bash "$HERE/tag.sh" "$v"
}

# --------------------------------------------------------------------------

case "${1:-}" in
    next)      shift; cmd_next "$@" ;;
    preflight) shift; cmd_preflight "$@" ;;
    perf)      shift; cmd_perf "$@" ;;
    notes)     shift; cmd_notes "$@" ;;
    verify)    shift; cmd_verify "$@" ;;
    prepare)   shift; cmd_prepare "$@" ;;
    tag)       shift; cmd_tag "$@" ;;
    -h|--help|help|"") usage ;;
    *) echo "error: unknown command '$1'" >&2; echo >&2; usage >&2; exit 2 ;;
esac
