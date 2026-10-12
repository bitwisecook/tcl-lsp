#!/usr/bin/env bash
set -uo pipefail
umask 077

evidence=/var/tmp/r2286m-evidence
cleanup_log="$evidence/cleanup-actions.txt"
verification="$evidence/cleanup-verification.txt"
partition=R2286_r2286m
folder=/R2286_r2286m/lifetime
virtual=/R2286_r2286m/__tcl_lsp_2286_r2286m_vs
pool=/R2286_r2286m/__tcl_lsp_2286_r2286m_pool
node=/Common/__tcl_lsp_2286_r2286m_node
caller=/R2286_r2286m/lifetime/__tcl_lsp_2286_r2286m_caller
provider=/R2286_r2286m/lifetime/__tcl_lsp_2286_r2286m_provider
fresh_caller=/R2286_r2286m/lifetime/__tcl_lsp_2286_r2286m_caller_fresh
fresh_provider=/R2286_r2286m/lifetime/__tcl_lsp_2286_r2286m_provider_fresh

rules=(
    /Common/__tcl_lsp_2286_r2286m_backend
    /Common/__tcl_lsp_2286_r2286m_nul_counted_last
    /Common/__tcl_lsp_2286_r2286m_nul_plain_last
    /Common/__tcl_lsp_2286_r2286m_nul_plain_unset_first
    /Common/__tcl_lsp_2286_r2286m_nul_array_root
    /Common/__tcl_lsp_2286_r2286m_nul_array_index
    /Common/__tcl_lsp_2286_r2286m_unicode_format_forward
    /Common/__tcl_lsp_2286_r2286m_unicode_format_reverse
    /Common/__tcl_lsp_2286_r2286m_unicode_bytes_forward
    /Common/__tcl_lsp_2286_r2286m_unicode_bytes_reverse
    /Common/__tcl_lsp_2286_r2286m_unicode_cross_producer
    /Common/__tcl_lsp_2286_r2286m_lexical_format_unbraced
    /Common/__tcl_lsp_2286_r2286m_lexical_format_braced
    /Common/__tcl_lsp_2286_r2286m_lexical_bytes_unbraced
    /Common/__tcl_lsp_2286_r2286m_lexical_bytes_braced
    /Common/__tcl_lsp_2286_r2286m_commands_format
    /Common/__tcl_lsp_2286_r2286m_commands_bytes
    /Common/__tcl_lsp_2286_r2286m_commands_cross_producer
    /Common/__tcl_lsp_2286_r2286m_expressions
)

{
    tmsh show sys version
    tmsh show sys failover
    tmsh show sys tmm-info
    tmsh list auth partition "$partition"
    tmsh list sys folder "$folder"
    tmsh list ltm virtual "$virtual" all-properties
    tmsh list ltm pool "$pool" all-properties
    tmsh list ltm node "$node" all-properties
    for rule in "${rules[@]}" "$caller" "$provider" "$fresh_caller" "$fresh_provider"; do
        tmsh list ltm rule "$rule" all-properties 2>&1 || true
    done
} > "$evidence/config-before-cleanup.txt" 2>&1

delete_object() {
    printf 'DELETE %s\n' "$*" >> "$cleanup_log"
    tmsh delete "$@" >> "$cleanup_log" 2>&1
    rc=$?
    printf 'RC %s\n' "$rc" >> "$cleanup_log"
    return "$rc"
}

: > "$cleanup_log"
delete_object ltm virtual "$virtual"
delete_object ltm pool "$pool"
delete_object ltm rule "$fresh_caller"
delete_object ltm rule "$caller"
delete_object ltm rule "$fresh_provider"
delete_object ltm rule "$provider"
for rule in "${rules[@]}"; do
    delete_object ltm rule "$rule"
done
delete_object ltm node "$node"
delete_object sys folder "$folder"
delete_object auth partition "$partition"

{
    printf 'VIRTUAL %s\n' "$virtual"
    tmsh list ltm virtual "$virtual" 2>&1 || true
    printf 'POOL %s\n' "$pool"
    tmsh list ltm pool "$pool" 2>&1 || true
    printf 'NODE %s\n' "$node"
    tmsh list ltm node "$node" 2>&1 || true
    for rule in "${rules[@]}" "$caller" "$provider" "$fresh_caller" "$fresh_provider"; do
        printf 'RULE %s\n' "$rule"
        tmsh list ltm rule "$rule" 2>&1 || true
    done
    printf 'FOLDER %s\n' "$folder"
    tmsh list sys folder "$folder" 2>&1 || true
    printf 'PARTITION %s\n' "$partition"
    tmsh list auth partition "$partition" 2>&1 || true
} > "$verification"

capture_pid=
read -r capture_pid < "$evidence/ltm-capture.pid"
if [[ $capture_pid =~ ^[0-9]+$ ]]; then
    ps -p "$capture_pid" -o pid=,args= > "$evidence/ltm-capture-before-stop.txt"
    if grep -Fq 'tail -n 0 -F /var/log/ltm' "$evidence/ltm-capture-before-stop.txt"; then
        kill "$capture_pid"
        wait "$capture_pid" 2>/dev/null || true
        printf 'stopped %s\n' "$capture_pid" > "$evidence/ltm-capture-stop.txt"
    else
        printf 'PID did not match owned capture\n' > "$evidence/ltm-capture-stop.txt"
        exit 5
    fi
else
    printf 'invalid capture PID\n' > "$evidence/ltm-capture-stop.txt"
    exit 5
fi
date -u +%Y-%m-%dT%H:%M:%SZ > "$evidence/capture-end-utc.txt"
