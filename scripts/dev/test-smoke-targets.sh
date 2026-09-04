#!/bin/sh
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# SPDX-License-Identifier: AGPL-3.0-or-later

# Drift contract for the targeted Cargo smoke fallback. Adding a convention-
# named smoke test must also name the Cargo target that owns it, so a faster
# fallback can never silently narrow the assertions run before a push.

set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
REPO_ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/../.." && pwd)
MANIFEST=$SCRIPT_DIR/smoke-targets.tsv
EXPECTED=$(mktemp)
ACTUAL=$(mktemp)
trap 'rm -f "$EXPECTED" "$ACTUAL"' EXIT HUP INT TERM

cd "$REPO_ROOT"

awk -F '\t' '!/^#/ && NF { print $1 }' "$MANIFEST" | sort -u > "$EXPECTED"

{
    rg -l --glob '*.rs' \
        '^[[:space:]]*(pub(\([^)]*\))?[[:space:]]+)?(async[[:space:]]+)?fn[[:space:]]+smoke([_[:alnum:]]*)[[:space:]]*\(|^[[:space:]]*mod[[:space:]]+smoke[[:space:]]*[;{]' \
        rust || true
    rg --files rust | awk '
        /\/tests\/smoke\.rs$/ || /\/tests\/[^/]*_smoke\.rs$/ || /\/tests\/smoke\/[^/]*_smoke\.rs$/ { print }
    '
} | sort -u > "$ACTUAL"

if ! diff -u "$EXPECTED" "$ACTUAL"; then
    echo "smoke-targets.tsv does not own every convention-named smoke source" >&2
    exit 1
fi

# Cargo metadata is the authority for target names and source roots. The
# helper rejects a source associated with the wrong integration target and
# rejects ambiguous library/binary module ownership instead of guessing.
python3 "$SCRIPT_DIR/check_smoke_targets.py" --self-test
python3 "$SCRIPT_DIR/check_smoke_targets.py" "$MANIFEST"

echo "targeted Cargo smoke ownership contract passed"
