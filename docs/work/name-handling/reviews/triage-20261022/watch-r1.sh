#!/bin/bash
# Bounded read-only monitor. It fetches refs but never changes the checkout.
set -u
cd "$(git rev-parse --show-toplevel)"
log="${1:-/tmp/review-triage-watch.log}"
cycles="${2:-4}"
{
    date -u
    printf 'checkout=%s\n' "$(git rev-parse HEAD)"
    for ((cycle=1; cycle<=cycles; cycle++)); do
        printf 'FETCH cycle=%s %s\n' "$cycle" "$(date -u)"
        if ! git fetch origin work; then
            echo 'status=fetch_error'
            exit 1
        fi
        printf 'remote=%s\n' "$(git rev-parse origin/work)"
        changed="$(git diff --name-only HEAD origin/work -- rust/tcl-compiler rust/tcl-registry rust/tcl-lsp-core/src specs)"
        if [ -n "$changed" ]; then
            printf '%s\nstatus=source_change\n' "$changed"
            exit 0
        fi
        echo 'status=no_source_change'
        if [ "$cycle" -lt "$cycles" ]; then
            # Short sleeps keep the orchestration interruptible.
            for tick in 1 2 3 4; do sleep 45; done
        fi
    done
    printf 'status=bounded_watch_complete %s\n' "$(date -u)"
} > "$log" 2>&1
