#!/bin/bash
# One full check + clippy, then per attempt: fetch, rebase, cargo check if upstream Rust moved, push, verify.
S=/tmp/claude-0/-home-user-tcl-lsp/8d622143-3532-5414-9b9f-0ba860598aa9/scratchpad
source $S/env.sh
git fetch -q --depth=300 origin work && git rebase origin/work >/dev/null 2>&1 || { echo "rebase conflict"; git rebase --abort; exit 1; }
cargo check --workspace 2>&1 | grep -E "^error" && { echo "check failed"; exit 1; }
CARGO_PROFILE_TEST_INCREMENTAL=false cargo clippy -p tcl-compiler --all-targets -- -D warnings -A clippy::assert_is_empty 2>&1 | grep -E "^(error|warning)" && { echo "clippy failed"; exit 1; }
checked=$(git rev-parse origin/work)
for i in $(seq 1 20); do
  git fetch -q --depth=300 origin work
  git rebase origin/work >/dev/null 2>&1 || { echo "rebase conflict"; git rebase --abort; exit 1; }
  if git diff --name-only "$checked" origin/work | grep -qE '\.rs$|Cargo\.(toml|lock)$'; then
    cargo check --workspace 2>&1 | grep -E "^error" && { echo "check failed after rebase"; exit 1; }
    checked=$(git rev-parse origin/work)
  fi
  if git push origin HEAD:work >$S/push.log 2>&1; then
    git fetch -q --depth=300 origin work
    if git merge-base --is-ancestor HEAD origin/work; then echo "pushed $(git rev-parse --short HEAD) on attempt $i (verified)"; exit 0; fi
  fi
  echo "attempt $i: $(grep -E 'rejected|error' $S/push.log | head -1)"
done
echo "gave up"; exit 1
