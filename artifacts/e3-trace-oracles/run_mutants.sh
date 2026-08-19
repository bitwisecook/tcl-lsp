#!/usr/bin/env bash
# Walk the manifest, running each mutant in turn. Filter with an id prefix.
set -u
D=/tmp/claude-0/-home-user-tcl-lsp/49cadaef-dfc0-5f80-9bf3-267da5a707d9/scratchpad/e3
M=$D/mutants
FILTER="${1:-}"
while IFS=$'\t' read -r mid path scope; do
    case "$mid" in ${FILTER}*) ;; *) continue ;; esac
    bash "$D/mutate.sh" "$mid" "$path" "$M/$mid.from" "$M/$mid.to" "$scope"
    rc=$?
    if [ $rc -eq 9 ]; then echo "STOPPING: disk below 4GB"; exit 9; fi
done < "$M/manifest.tsv"
