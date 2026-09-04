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

while IFS="$(printf '\t')" read -r source package kind target; do
    case "$source" in
        ''|'#'*) continue ;;
    esac

    test -f "$source" || {
        echo "missing smoke source: $source" >&2
        exit 1
    }

    case "$source" in
        */src/*) crate_root=${source%%/src/*} ;;
        */tests/*) crate_root=${source%%/tests/*} ;;
        *)
            echo "cannot resolve crate root for smoke source: $source" >&2
            exit 1
            ;;
    esac

    test -f "$crate_root/Cargo.toml" || {
        echo "missing Cargo.toml for smoke source: $source" >&2
        exit 1
    }

    declared_package=$(awk -F '"' '
        /^\[package\]$/ { in_package = 1; next }
        in_package && /^\[/ { exit }
        in_package && /^name = "/ { print $2; exit }
    ' "$crate_root/Cargo.toml")
    test "$package" = "$declared_package" || {
        echo "smoke source $source belongs to '$declared_package', not '$package'" >&2
        exit 1
    }

    case "$kind" in
        lib)
            test "$target" = '-' || {
                echo "library smoke row must use '-' target: $source" >&2
                exit 1
            }
            ;;
        test)
            test -f "$crate_root/tests/$target.rs" || {
                echo "missing integration target '$target' for $source" >&2
                exit 1
            }
            ;;
        *)
            echo "invalid smoke target kind '$kind' for $source" >&2
            exit 1
            ;;
    esac
done < "$MANIFEST"

echo "targeted Cargo smoke ownership contract passed"
