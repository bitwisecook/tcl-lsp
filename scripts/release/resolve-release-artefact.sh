#!/usr/bin/env bash
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# SPDX-License-Identifier: AGPL-3.0-or-later

# Resolve the maintained release glob to exactly one path before provenance,
# SBOM generation, or upload. This keeps every stage bound to the same subject
# and turns accidental zero/multiple matches into a loud release failure.

set -euo pipefail

if [[ $# -ne 1 || -z $1 ]]; then
    echo "usage: scripts/release/resolve-release-artefact.sh ARTEFACT_GLOB" >&2
    exit 2
fi

pattern=$1
matches=()
while IFS= read -r match; do
    matches+=("$match")
done < <(compgen -G "$pattern" || true)

if [[ ${#matches[@]} -ne 1 ]]; then
    printf 'release artefact glob %q matched %d files; expected exactly one\n' \
        "$pattern" "${#matches[@]}" >&2
    if [[ ${#matches[@]} -gt 0 ]]; then
        printf '  %s\n' "${matches[@]}" >&2
    fi
    exit 1
fi

printf '%s\n' "${matches[0]}"
