#!/usr/bin/env bash
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# SPDX-License-Identifier: AGPL-3.0-or-later

# Bind a packaged WASI module to its source inventory and build settings.
set -euo pipefail

action=${1:?expected snapshot, seal, or check}
root=${2:?expected repository root}
root=$(cd "$root" && pwd)
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

snapshot() {
    (
        cd "$root"
        printf 'wasi-assets-v1\n'
        # Same content-inventory approach as the shared VSIX web payload.
        # Include complete trees, not just .rs: data, generated includes and
        # embedded scripts are compiler inputs too. Output trees are excluded.
        find rust runtime specs scripts .cargo grammars samples .github \
            Makefile Cargo.toml Cargo.lock rust-toolchain.toml \
            \( -type d \( -name target -o -name dist -o -name node_modules \
                -o -name out -o -name build -o -name __pycache__ \
                -o -name .vscode-test -o -name .vscode-test-web \) -prune \) \
            -o \( -type f -o -type l \) -print0 | xargs -0 shasum -a 256 | LC_ALL=C sort
        # Cargo also reads ancestor and per-user configuration outside the
        # checkout. Record contents without embedding producer-specific paths.
        config_parent=$(dirname "$root")
        config_depth=1
        while :; do
            for config_name in config config.toml; do
                config="$config_parent/.cargo/$config_name"
                if [ -e "$config" ]; then
                    printf 'ancestor-config-%s-%s ' "$config_depth" "$config_name"
                    shasum -a 256 < "$config"
                fi
            done
            [ "$config_parent" != / ] || break
            config_parent=$(dirname "$config_parent")
            config_depth=$((config_depth + 1))
        done
        for config_name in config config.toml; do
            config="${CARGO_HOME:-$HOME/.cargo}/$config_name"
            if [ -e "$config" ]; then
                printf 'cargo-home-%s ' "$config_name"
                shasum -a 256 < "$config"
            fi
        done
        # Version stamping reads Git as well as the source files. Hash these
        # values so paths and arbitrary environment text cannot split records.
        {
            git rev-parse --verify HEAD
            git describe --tags --always --dirty
            printf '%s\0' "${TCL_LSP_VERSION-}" "${GIT_HASH-}" \
                "${CARGO_PROFILE_RELEASE_LTO:-true}"
            # Enumerate names separately: env's newline-delimited records
            # cannot preserve multiline flag values. Indirection works on Bash 3.
            compgen -e | LC_ALL=C sort | while IFS= read -r variable; do
                case "$variable" in
                    CARGO_TARGET_DIR | CARGO_HOME | CARGO_MAKEFLAGS | RUSTUP_HOME) ;;
                    CARGO_PROFILE_RELEASE_LTO) ;;
                    CARGO_* | RUST* | TCL_* | CC* | AR* | CFLAGS* | CXX* | LDFLAGS*)
                        printf '%s\0%s\0' "$variable" "${!variable}" ;;
                esac
            done
        } | shasum -a 256
    )
}

module_digest() {
    test -f "$1"
    # Keep the receipt independent of the producer's absolute checkout path.
    printf 'module '
    shasum -a 256 < "$1"
}

case "$action" in
    snapshot)
        snapshot > "$work/source"
        cp "$work/source" "${3:?expected snapshot output}"
        ;;
    seal)
        module=${3:?expected module}
        before=${4:?expected pre-build snapshot}
        receipt=${5:?expected receipt output}
        snapshot > "$work/source"
        cmp -s "$before" "$work/source" || {
            echo 'ERROR: WASI source inputs changed during the build' >&2
            exit 1
        }
        module_digest "$module" >> "$work/source"
        # Replace only after the complete source/output join succeeds.
        cp "$work/source" "$receipt.tmp"
        mv "$receipt.tmp" "$receipt"
        ;;
    check)
        module=${3:?expected module}
        receipt=${4:?expected receipt}
        test -s "$receipt" && test -s "$module" || exit 1
        snapshot > "$work/source"
        module_digest "$module" >> "$work/source"
        cmp -s "$receipt" "$work/source"
        ;;
    *)
        echo "unknown WASI asset action: $action" >&2
        exit 2
        ;;
esac
