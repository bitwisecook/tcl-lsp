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
RUNNABLE_MANIFEST=$(mktemp)
trap 'rm -f "$RUNNABLE_MANIFEST"' EXIT HUP INT TERM
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
python3 "$SCRIPT_DIR/check_smoke_targets.py" --runnable-manifest "$MANIFEST" \
    > "$RUNNABLE_MANIFEST"

run_smoke() {
    if [ "$LIST_ONLY" = true ]; then
        "$@" smoke -- --list
    else
        "$@" smoke
    fi
}

run_all() {
    if [ "$LIST_ONLY" = true ]; then
        "$@" -- --list
    else
        "$@"
    fi
}

run_named_target() {
    kind=$1
    package=$2
    target=$3
    features=$4

    if [ "$features" = - ]; then
        set -- cargo test -p "$package" "--$kind" "$target"
    else
        set -- cargo test -p "$package" --features "$features" "--$kind" "$target"
    fi

    case "$target" in
        smoke|*_smoke)
            # Nextest's binary selector includes every test in a
            # convention-named smoke target, even when an individual
            # function does not contain the word "smoke".
            echo "==> cargo test -p $package --$kind $target"
            run_all "$@"
            ;;
        *)
            echo "==> cargo test -p $package --$kind $target smoke"
            run_smoke "$@"
            ;;
    esac
}

while IFS="$(printf '\t')" read -r source package kind target features; do
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
            if [ "$features" = - ]; then
                set -- cargo test -p "$package" --lib
            else
                set -- cargo test -p "$package" --features "$features" --lib
            fi
            case "$target" in
                smoke|*_smoke)
                    echo "==> cargo test -p $package --lib"
                    run_all "$@"
                    ;;
                *)
                    echo "==> cargo test -p $package --lib smoke"
                    run_smoke "$@"
                    ;;
            esac
            ;;
        test|bin|example|bench)
            run_named_target "$kind" "$package" "$target" "$features"
            ;;
        *)
            echo "invalid target kind '$kind' for $source in $MANIFEST" >&2
            exit 1
            ;;
    esac
done < "$RUNNABLE_MANIFEST"
