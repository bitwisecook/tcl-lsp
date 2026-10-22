#!/bin/bash
# Run preserved review inputs and freshly generated scratch outputs on six engines.
set -u
cd "$(git rev-parse --show-toplevel)"
E="$PWD/docs/work/name-handling/reviews/triage-20261022"
tag="${1:?provide the output run tag}"
log_tag="${2:-$tag}"
log="$E/$log_tag-oracles.log"
{
    date -u
    git rev-parse HEAD
    for f in "$E"/tcl/*.tcl "$E/out/$tag"/*.tcl; do
        test -f "$f" || continue
        printf '\nFILE %s\n' "${f#"$PWD/"}"
        sha256sum "$f"
        bash "$E/oracle.sh" "$f"
    done
} > "$log" 2>&1
printf 'Oracle replay saved to %s\n' "$log"
