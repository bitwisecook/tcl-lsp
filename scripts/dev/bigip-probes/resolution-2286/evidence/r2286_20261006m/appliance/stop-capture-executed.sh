#!/usr/bin/env bash
set -uo pipefail
evidence=/var/tmp/r2286m-evidence
capture_pid=15577
comm=
read -r comm < "/proc/$capture_pid/comm"
if [[ $comm == tail ]] && grep -aq '/var/log/ltm' "/proc/$capture_pid/cmdline"; then
    od -An -tx1 -v "/proc/$capture_pid/cmdline" > "$evidence/ltm-capture-command-hex.txt"
    kill "$capture_pid"
    printf 'stopped %s\n' "$capture_pid" > "$evidence/ltm-capture-final-stop.txt"
    date -u +%Y-%m-%dT%H:%M:%SZ > "$evidence/capture-end-utc.txt"
else
    printf 'capture identity mismatch\n' > "$evidence/ltm-capture-final-stop.txt"
    exit 7
fi
