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

SMOKE_DECL_RE='^[[:space:]]*(pub(\([^)]*\))?[[:space:]]+)?((async[[:space:]]+)?fn[[:space:]]+smoke([_[:alnum:]]*)[[:space:]]*\(|mod[[:space:]]+smoke[[:space:]]*[;{])'

# Keep the declaration scanner broad enough for Rust's visibility-qualified
# module forms. A missed declaration would let the fallback consume a
# manifest that silently omits the owning Cargo target.
for declaration in \
    'mod smoke {' \
    'pub mod smoke;' \
    'pub(crate) mod smoke {' \
    'pub(super) mod smoke;' \
    'pub(in crate::tests) mod smoke {'
do
    if ! printf '%s\n' "$declaration" | grep -E -q "$SMOKE_DECL_RE"; then
        echo "smoke declaration scanner missed: $declaration" >&2
        exit 1
    fi
done

cd "$REPO_ROOT"

awk -F '\t' '!/^#/ && NF { print $1 }' "$MANIFEST" | sort -u > "$EXPECTED"

{
    git grep -l -E "$SMOKE_DECL_RE" \
        -- ':(glob)rust/**/*.rs' || true
    git ls-files ':(glob)rust/**/*.rs' | awk '
        /\/tests\/smoke\.rs$/ || /\/tests\/[^/]*_smoke\.rs$/ || /\/tests\/smoke\/[^/]*_smoke\.rs$/ { print }
    '
    # Nextest's binary-name filter selects the entire target even when its
    # unit-test functions have ordinary names, so Cargo's source roots belong
    # in the same discovered path set as declaration-named smoke tests.
    python3 "$SCRIPT_DIR/check_smoke_targets.py" --smoke-bin-sources
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
