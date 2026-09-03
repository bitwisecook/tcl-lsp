#!/usr/bin/env bash
# Audit every committed Cargo lockfile with the repository supply-chain policy.

set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"

lockfiles=()
while IFS= read -r -d '' lockfile; do
    lockfiles+=("$lockfile")
done < <(git ls-files -z -- '*Cargo.lock')

if ((${#lockfiles[@]} == 0)); then
    echo "ERROR: no tracked Cargo.lock files found" >&2
    exit 1
fi

for lockfile in "${lockfiles[@]}"; do
    manifest="${lockfile%Cargo.lock}Cargo.toml"
    if [[ ! -f "$manifest" ]]; then
        echo "ERROR: tracked $lockfile has no adjacent $manifest" >&2
        exit 1
    fi

    echo "==> cargo deny: $manifest ($lockfile)"
    cargo deny --locked --all-features --manifest-path "$manifest" check
done
