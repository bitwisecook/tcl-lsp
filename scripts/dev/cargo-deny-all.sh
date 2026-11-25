#!/usr/bin/env bash
# Audit every committed Cargo lockfile with the repository supply-chain policy.

set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"

lockfiles=()
# Match workspace lockfiles named exactly `Cargo.lock`. The scratchpad mirror
# stores historical lockfile evidence under docs/, not independent workspaces.
while IFS= read -r -d '' lockfile; do
    lockfiles+=("$lockfile")
done < <(git ls-files -z -- ':(glob)**/Cargo.lock' \
    ':(exclude)docs/work/name-handling/scratchpad-mirror/**')

if ((${#lockfiles[@]} == 0)); then
    echo "ERROR: no tracked Cargo.lock files found" >&2
    exit 1
fi

status=0
for lockfile in "${lockfiles[@]}"; do
    manifest="${lockfile%Cargo.lock}Cargo.toml"
    if [[ ! -f "$manifest" ]]; then
        echo "ERROR: tracked $lockfile has no adjacent $manifest" >&2
        status=1
        continue
    fi

    echo "==> cargo deny: $manifest ($lockfile)"
    if ! cargo deny --locked --all-features --manifest-path "$manifest" check; then
        status=1
    fi
done
exit "$status"
