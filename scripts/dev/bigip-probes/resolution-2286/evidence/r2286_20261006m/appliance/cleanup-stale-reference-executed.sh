#!/usr/bin/env bash
set -uo pipefail
umask 077

evidence=/var/tmp/r2286m-evidence
caller=/R2286_r2286m/lifetime/__tcl_lsp_2286_r2286m_caller
provider=/R2286_r2286m/lifetime/__tcl_lsp_2286_r2286m_provider
folder=/R2286_r2286m/lifetime
partition=R2286_r2286m
log="$evidence/cleanup-stale-reference-actions.txt"
: > "$log"

if tmsh load sys config merge file /var/tmp/r2286m-caller-neutral-cleanup.conf >> "$log" 2>&1; then
    printf 'neutral-caller-load ACCEPTED\n' >> "$log"
else
    printf 'neutral-caller-load REJECTED\n' >> "$log"
fi
tmsh list ltm rule "$caller" all-properties > "$evidence/caller-neutral-object.txt" 2>&1 || true

if tmsh delete ltm rule "$provider" >> "$log" 2>&1; then
    printf 'provider-delete-with-neutral-caller ACCEPTED\n' >> "$log"
else
    printf 'provider-delete-with-neutral-caller REJECTED\n' >> "$log"
fi
if tmsh delete ltm rule "$caller" >> "$log" 2>&1; then
    printf 'neutral-caller-delete ACCEPTED\n' >> "$log"
else
    printf 'neutral-caller-delete REJECTED\n' >> "$log"
fi
if tmsh list ltm rule "$provider" >/dev/null 2>&1; then
    if tmsh delete ltm rule "$provider" >> "$log" 2>&1; then
        printf 'provider-delete-after-caller ACCEPTED\n' >> "$log"
    else
        printf 'provider-delete-after-caller REJECTED\n' >> "$log"
    fi
fi
if tmsh delete sys folder "$folder" >> "$log" 2>&1; then
    printf 'folder-delete ACCEPTED\n' >> "$log"
else
    printf 'folder-delete REJECTED\n' >> "$log"
fi
if tmsh delete auth partition "$partition" >> "$log" 2>&1; then
    printf 'partition-delete ACCEPTED\n' >> "$log"
else
    printf 'partition-delete REJECTED\n' >> "$log"
fi

{
    printf 'CALLER %s\n' "$caller"
    tmsh list ltm rule "$caller" 2>&1 || true
    printf 'PROVIDER %s\n' "$provider"
    tmsh list ltm rule "$provider" 2>&1 || true
    printf 'FOLDER %s\n' "$folder"
    tmsh list sys folder "$folder" 2>&1 || true
    printf 'PARTITION %s\n' "$partition"
    tmsh list auth partition "$partition" 2>&1 || true
} > "$evidence/cleanup-stale-reference-verification.txt"

if tmsh list ltm rule "$caller" >/dev/null 2>&1 || \
   tmsh list ltm rule "$provider" >/dev/null 2>&1 || \
   tmsh list sys folder "$folder" >/dev/null 2>&1 || \
   tmsh list auth partition "$partition" >/dev/null 2>&1; then
    printf 'owned objects remain; capture retained\n' >> "$log"
    exit 6
fi

capture_pid=
read -r capture_pid < "$evidence/ltm-capture.pid"
if [[ $capture_pid =~ ^[0-9]+$ ]] && [[ -r /proc/$capture_pid/cmdline ]]; then
    tr '\0' ' ' < "/proc/$capture_pid/cmdline" > "$evidence/ltm-capture-before-final-stop.txt"
    if grep -Fq 'tail ' "$evidence/ltm-capture-before-final-stop.txt" && \
       grep -Fq '/var/log/ltm' "$evidence/ltm-capture-before-final-stop.txt"; then
        kill "$capture_pid"
        printf 'stopped %s\n' "$capture_pid" > "$evidence/ltm-capture-final-stop.txt"
    else
        printf 'PID did not match owned capture\n' > "$evidence/ltm-capture-final-stop.txt"
        exit 7
    fi
else
    printf 'invalid or absent capture PID\n' > "$evidence/ltm-capture-final-stop.txt"
    exit 7
fi
date -u +%Y-%m-%dT%H:%M:%SZ > "$evidence/capture-end-utc.txt"
