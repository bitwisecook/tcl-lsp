#!/usr/bin/env bash
# Emit a stable fingerprint of the current git worktree (HEAD + index + unstaged
# changes). Used by `make test-slow` to stamp a successful run, and by the
# pre-push hook to verify the worktree hasn't drifted since the last test-slow.
#
# Untracked files are intentionally ignored — they don't affect tests run from
# tracked sources, and including them would invalidate the stamp on every
# stray scratch file.
set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

head=$(git rev-parse HEAD 2>/dev/null || echo "no-head")
staged=$(git diff --cached HEAD 2>/dev/null | sha256sum | cut -d' ' -f1)
unstaged=$(git diff HEAD 2>/dev/null | sha256sum | cut -d' ' -f1)

printf '%s\n%s\n%s\n' "$head" "$staged" "$unstaged" | sha256sum | cut -d' ' -f1
