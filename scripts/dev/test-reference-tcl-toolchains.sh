#!/usr/bin/env bash
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# SPDX-License-Identifier: AGPL-3.0-or-later

# Hermetic regression for ensure-test-deps' Tcl reference matrix. Fake
# interpreters and source roots prove a stale-but-present tclsh9.0 cannot take
# the all-ready path; --check guarantees no host installation or network use.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
INSTALLER="$SCRIPT_DIR/ensure-test-deps.sh"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
TOOLCHAIN_READER="$SCRIPT_DIR/tcl-reference-toolchains.sh"
# shellcheck source=tcl-reference-toolchains.sh
. "$TOOLCHAIN_READER"
tcl_reference_load_toolchains "$REPO_ROOT"

fixture="$(mktemp -d)"
trap 'rm -rf "$fixture"' EXIT
fake_bin="$fixture/bin"
source_parent="$fixture/sources"
install_dir="$fixture/install"
fake_home="$fixture/home"
tcl90_patchlevel="$(tcl_reference_patchlevel 9.0)"
mkdir -p "$fake_bin" "$source_parent/tcl${tcl90_patchlevel}/unix" "$fake_home"

# The adapter itself must run under a POSIX shell. That is a stricter syntax
# check than macOS Bash 3.2 and catches accidental `declare -A`/`declare -g`
# regressions before ensure-test-deps reaches a host.
adapter_output="$({
    # shellcheck disable=SC2016 # Expansions intentionally happen in the child shell.
    env -i PATH=/usr/bin:/bin REPO_ROOT="$REPO_ROOT" TOOLCHAIN_READER="$TOOLCHAIN_READER" \
        /bin/sh -c '. "$TOOLCHAIN_READER"; tcl_reference_load_toolchains "$REPO_ROOT"; tcl_reference_releases; tcl_reference_patchlevel 9.0; tcl_reference_source_tag 9.0'
} 2>&1)"
expected_adapter_output="$(printf '8.4\n8.5\n8.6\n9.0\n9.1\n%s\n%s' \
    "$tcl90_patchlevel" "$(tcl_reference_source_tag 9.0)")"
if [ "$adapter_output" != "$expected_adapter_output" ]; then
    echo "reference toolchain adapter did not run under /bin/sh" >&2
    printf '%s\n' "$adapter_output" >&2
    exit 1
fi

write_fake_tclsh() {
    local name="$1" patchlevel="$2"
    {
        printf '#!/bin/sh\n'
        printf 'while IFS= read -r _line; do :; done\n'
        printf "printf '%%s\\n' '%s'\n" "$patchlevel"
    } > "$fake_bin/$name"
    chmod 0755 "$fake_bin/$name"
}

write_exact_matrix() {
    local release
    while IFS= read -r release; do
        write_fake_tclsh "tclsh${release}" "$(tcl_reference_patchlevel "$release")"
    done < <(tcl_reference_releases)
    write_fake_tclsh tclsh "$tcl90_patchlevel"
}

run_check() {
    env \
        HOME="$fake_home" \
        PATH="$fake_bin:/usr/bin:/bin" \
        TCL_LSP_TCL_SOURCE_PARENT="$source_parent" \
        TCL_LSP_TCL_BIN_DIR="$install_dir" \
        SKIP_PYTHON_TK=1 \
        SKIP_NODE=1 \
        SKIP_KOTLINC=1 \
        SKIP_RUST=1 \
        SKIP_WASMTIME=1 \
        SKIP_BINARYEN=1 \
        SKIP_WASI_SDK=1 \
        SKIP_EMACS=1 \
        SKIP_XVFB=1 \
        SKIP_TSHARK=1 \
        SKIP_OPENSSL=1 \
        SKIP_PING=1 \
        SKIP_RGXG=1 \
        SKIP_TCLLIB=1 \
        SKIP_UV=1 \
        bash "$INSTALLER" --check
}

write_exact_matrix
write_fake_tclsh tclsh9.0 9.0.3

set +e
stale_output="$(run_check 2>&1)"
stale_status=$?
set -e
if [ "$stale_status" -ne 1 ]; then
    echo "expected stale tclsh9.0 check to fail, got $stale_status" >&2
    printf '%s\n' "$stale_output" >&2
    exit 1
fi
case "$stale_output" in
    *"tclsh9.0 $tcl90_patchlevel (found 9.0.3"*"would build from $source_parent/tcl${tcl90_patchlevel}"*) ;;
    *)
        echo "stale tclsh9.0 diagnostic did not name actual, expected, and pinned source" >&2
        printf '%s\n' "$stale_output" >&2
        exit 1
        ;;
esac
if [ -e "$install_dir" ]; then
    echo "--check mutated the fake installation prefix" >&2
    exit 1
fi

write_fake_tclsh tclsh9.0 "$tcl90_patchlevel"
exact_output="$(run_check 2>&1)"
case "$exact_output" in
    *"requested tclsh exact reference patchlevels already on PATH"*) ;;
    *)
        echo "exact reference matrix did not take the all-ready path" >&2
        printf '%s\n' "$exact_output" >&2
        exit 1
        ;;
esac
if [ -e "$install_dir" ]; then
    echo "exact --check mutated the fake installation prefix" >&2
    exit 1
fi

echo "reference Tcl toolchain shell regression: ok"
