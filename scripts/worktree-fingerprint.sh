#!/usr/bin/env bash
# Emit a stable fingerprint of the current git worktree (HEAD + index + unstaged
# changes). Used by `make check-all` / `make test-slow` to stamp a successful
# run, and by the pre-push hook to verify the worktree hasn't drifted since
# the stamp was written.
#
# Untracked files are intentionally ignored — they don't affect tests run from
# tracked sources, and including them would invalidate the stamp on every
# stray scratch file.
set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

# sha256 wrapper: GNU coreutils ships sha256sum, macOS / BusyBox ship shasum.
sha256() {
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum | cut -d' ' -f1
    else
        shasum -a 256 | cut -d' ' -f1
    fi
}

head=$(git rev-parse HEAD 2>/dev/null || echo "no-head")
# `git diff --cached` (no HEAD) is robust on empty repos; `git diff` (no HEAD)
# captures only unstaged changes, so we don't double-count the index.
staged=$(git diff --cached --no-color --no-ext-diff 2>/dev/null | sha256)
unstaged=$(git diff --no-color --no-ext-diff 2>/dev/null | sha256)

printf '%s\n%s\n%s\n' "$head" "$staged" "$unstaged" | sha256
