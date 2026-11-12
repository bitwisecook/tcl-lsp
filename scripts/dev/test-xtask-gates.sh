#!/usr/bin/env bash
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# SPDX-License-Identifier: AGPL-3.0-or-later

# Every `xtask-*` gate target must run under `make xtask-check`, either as a
# prerequisite make actually parsed (text after `##` is help, not a
# prerequisite) or as an entry of the batched XTASK_GATES list. Targets that
# are deliberately outside the gate are listed in OUTSIDE_GATE with a reason.

set -euo pipefail

cd "$(dirname "$0")/../.."
makefile="${1:-Makefile}"

declare -A OUTSIDE_GATE=(
    [xtask-check]="the aggregate itself"
    [xtask-gates]="the batch runner"
    [xtask-gen-equivalence-packs]="advisory SpecTcl renderings, not in xtask-check"
    [xtask-audit-option-dialects]="on-demand regeneration from built tclsh trees"
    [xtask-registry-oracle]="needs a local BIG-IP extract"
)

database="$(make -pRrq -f "$makefile" : 2>/dev/null || true)"
prerequisites=" $(awk -F': ' '$1 == "xtask-check" { print $2; exit }' <<<"$database") "
gates="$(awk -F' := ' '$1 == "XTASK_GATES" { print $2; exit }' <<<"$database")"

if [[ "$prerequisites" == "  " || -z "$gates" ]]; then
    echo "FAIL: could not read xtask-check or XTASK_GATES from $makefile" >&2
    exit 1
fi

failures=0
while IFS= read -r target; do
    [[ -n "${OUTSIDE_GATE[$target]:-}" ]] && continue
    [[ "$prerequisites" == *" $target "* ]] && continue
    command="$(awk -v t="$target:" '
        $1 == t { found = 1; next }
        found && /^\t/ && /cargo xtask / { sub(/.*cargo xtask /, ""); print; exit }
        found && !/^\t/ { exit }
    ' "$makefile")"
    if [[ -n "$command" && "$gates" == *"'$command'"* ]]; then
        continue
    fi
    echo "FAIL: $target is not reachable from xtask-check (not a prerequisite, not in XTASK_GATES${command:+ as '$command'})" >&2
    failures=$((failures + 1))
done < <(grep -oE '^xtask-[a-z0-9-]+:' "$makefile" | tr -d ':' | sort -u)

if ((failures > 0)); then
    echo "$failures xtask gate target(s) would never run under make xtask-check" >&2
    exit 1
fi
echo "ok: every xtask gate target runs under xtask-check"
