#!/bin/sh
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# SPDX-License-Identifier: AGPL-3.0-or-later

# Cargo fallback for the local smoke tier. `cargo test --workspace smoke`
# selects the right test names, but first builds and launches every workspace
# test target. This manifest-driven runner invokes only targets that own a
# convention-named smoke test. The full deep suite still runs in CI.

set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
REPO_ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/../.." && pwd)
MANIFEST=$SCRIPT_DIR/smoke-targets.tsv
LIST_ONLY=false

case "${1:-}" in
    '') ;;
    --list) LIST_ONLY=true ;;
    *)
        echo "usage: $0 [--list]" >&2
        exit 2
        ;;
esac

cd "$REPO_ROOT"
seen='|'

# Do not consume a stale manifest when this runner is invoked directly from
# `make smoke` / `make prep-pr`. The rust-check gate also runs this contract,
# but the fallback must be safe on its own.
sh "$SCRIPT_DIR/test-smoke-targets.sh"

run_smoke() {
    if [ "$LIST_ONLY" = true ]; then
        "$@" smoke -- --list
    else
        "$@" smoke
    fi
}

run_named_target() {
    kind=$1
    package=$2
    target=$3
    shift 3

    case "$target" in
        smoke|*_smoke)
            # Nextest's binary selector includes every test in a
            # convention-named smoke target, even when an individual
            # function does not contain the word "smoke".
            echo "==> cargo test -p $package --$kind $target"
            if [ "$LIST_ONLY" = true ]; then
                cargo test -p "$package" "--$kind" "$target" -- --list
            else
                cargo test -p "$package" "--$kind" "$target"
            fi
            ;;
        *)
            echo "==> cargo test -p $package --$kind $target smoke"
            run_smoke cargo test -p "$package" "--$kind" "$target"
            ;;
    esac
}

while IFS="$(printf '\t')" read -r source package kind target; do
    case "$source" in
        ''|'#'*) continue ;;
    esac

    key="$package|$kind|$target"
    case "$seen" in
        *"|$key|"*) continue ;;
    esac
    seen="$seen$key|"

    case "$kind" in
        lib)
            echo "==> cargo test -p $package --lib smoke"
            run_smoke cargo test -p "$package" --lib
            ;;
        test|bin|example|bench)
            run_named_target "$kind" "$package" "$target"
            ;;
        *)
            echo "invalid target kind '$kind' for $source in $MANIFEST" >&2
            exit 1
            ;;
    esac
done < "$MANIFEST"
