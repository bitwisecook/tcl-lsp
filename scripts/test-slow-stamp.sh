#!/usr/bin/env bash
# test-slow-stamp.sh — write / verify the committed proof that
# ``make test-slow`` passed against the exact code under test.
#
# Unlike the local-only ``tmp/test-slow.stamp`` (gitignored, consumed by
# the pre-push hook), this stamp is **committed** so it travels with a
# PR and a GitHub CI job can gate on it: if the tree changed since the
# last green ``test-slow`` run, the recomputed fingerprint won't match
# and the check fails loudly.
#
# The fingerprint is a content hash of every tracked file *except the
# stamp itself*.  Two consequences make it the right primitive:
#   * Content-addressed, so it is independent of file mtimes — which
#     ``actions/checkout`` resets to "now" on every CI run, making a
#     timestamp comparison useless there.
#   * Excludes the stamp, so committing the freshly-written stamp does
#     not change the fingerprint.  (A ``git describe`` / HEAD comparison
#     can't do this — the commit that adds the stamp changes both — so
#     describe/head are recorded for humans but never gated on.)
#
# Usage:
#   scripts/test-slow-stamp.sh write        # refresh .test-slow.stamp
#   scripts/test-slow-stamp.sh check        # verify; exit 1 on drift
#   scripts/test-slow-stamp.sh fingerprint  # print the bare fingerprint
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

STAMP_REL=".test-slow.stamp"
STAMP_PATH="${REPO_ROOT}/${STAMP_REL}"

sha256() {
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum | cut -d' ' -f1
    else
        shasum -a 256 | cut -d' ' -f1
    fi
}

# Content fingerprint of all tracked files except the stamp.  Hashes
# working-tree blob content (so uncommitted edits to tracked files are
# reflected too) bound to the sorted path list (so a rename changes it).
fingerprint() {
    local paths
    paths="$(git -c core.quotepath=false ls-files -- ":(exclude,top)${STAMP_REL}" | LC_ALL=C sort)"
    {
        printf '%s\n' "$paths"
        # --stdin-paths preserves input order, so hash[i] ↔ path[i].
        # Aborts if a listed path is missing on disk (a deleted-but-
        # tracked file) — that's genuine drift, so let it fail loudly.
        printf '%s\n' "$paths" | git hash-object --stdin-paths
    } | sha256
}

write_stamp() {
    local fp describe head now
    fp="$(fingerprint)"
    describe="$(git describe --always --dirty --long --tags 2>/dev/null || echo unknown)"
    head="$(git rev-parse HEAD 2>/dev/null || echo no-head)"
    now="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
    cat > "$STAMP_PATH" <<EOF
# tcl-lsp test-slow stamp — DO NOT EDIT BY HAND.
# Written by 'make test-slow' on success; verified in CI via
# 'scripts/test-slow-stamp.sh check'.  Only 'fingerprint' is gated on;
# describe/head/date are informational (they change when this stamp is
# committed, so they cannot be compared).
describe=${describe}
head=${head}
date=${now}
fingerprint=${fp}
EOF
    echo "test-slow-stamp: wrote ${STAMP_REL} (fingerprint=${fp})"
}

read_field() {
    # read_field <name> — echo the value of "<name>=..." from the stamp.
    sed -n "s/^$1=//p" "$STAMP_PATH" | head -1
}

check_stamp() {
    if [ ! -f "$STAMP_PATH" ]; then
        cat >&2 <<EOF
ERROR: ${STAMP_REL} is missing.

  'make test-slow' has never recorded a passing run for this branch.
  Run 'make test-slow' locally, commit the updated ${STAMP_REL}, and push.
EOF
        exit 1
    fi

    local recorded current
    recorded="$(read_field fingerprint)"
    current="$(fingerprint)"

    if [ -z "$recorded" ]; then
        echo "ERROR: ${STAMP_REL} has no 'fingerprint=' line (corrupt stamp)." >&2
        exit 1
    fi

    if [ "$recorded" != "$current" ]; then
        cat >&2 <<EOF
ERROR: test-slow stamp is STALE — the tree changed since the last green run.

  recorded fingerprint: ${recorded}
  current  fingerprint: ${current}
  stamp recorded at:    $(read_field describe) ($(read_field date))

  The committed ${STAMP_REL} does not match the current code, so 'make
  test-slow' has NOT been run against what you're trying to merge.

  Fix it locally:
    make test-slow            # runs everything; rewrites ${STAMP_REL} on success
    git add ${STAMP_REL}
    git commit && git push
EOF
        exit 1
    fi

    echo "test-slow-stamp: OK — ${STAMP_REL} matches the current tree (fingerprint=${current})."
}

# drop_if_stale — remove the stamp ONLY when it no longer matches the
# tree.  A still-valid stamp is left untouched (so e.g. `make clean
# publish-vsix` with a green tree publishes), while a stale one is
# cleared so the gate reports "missing → run test-slow".  Best-effort:
# never fails the caller (it runs from a Makefile $(shell) hook).
drop_if_stale() {
    [ -f "$STAMP_PATH" ] || return 0
    local recorded current
    recorded="$(read_field fingerprint 2>/dev/null || true)"
    current="$(fingerprint 2>/dev/null || true)"
    if [ -z "$current" ] || [ "$recorded" != "$current" ]; then
        rm -f "$STAMP_PATH"
        echo "test-slow-stamp: dropped stale ${STAMP_REL} (run 'make test-slow' to refresh)" >&2
    fi
    return 0
}

case "${1:-}" in
    write)        write_stamp ;;
    check)        check_stamp ;;
    fingerprint)  fingerprint ;;
    drop-if-stale) drop_if_stale ;;
    *)
        echo "usage: $0 {write|check|fingerprint|drop-if-stale}" >&2
        exit 2
        ;;
esac
