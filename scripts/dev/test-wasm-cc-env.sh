#!/usr/bin/env bash
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# SPDX-License-Identifier: AGPL-3.0-or-later

# Hermetic regression for macOS's wasm32 C-compiler bootstrap. The fixtures
# model stock Apple clang and the owned wasi-sdk without requiring a macOS host
# or downloading a toolchain.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
HELPER="$SCRIPT_DIR/wasm-cc-env.sh"
INSTALLER="$SCRIPT_DIR/ensure-test-deps.sh"
fixture="$(mktemp -d)"
trap 'rm -rf "$fixture"' EXIT
apple_clang="$fixture/apple-clang"
sdk_root="$fixture/wasi-sdk"
sdk_clang="$sdk_root/bin/clang"
probe_log="$fixture/probe.log"
mkdir -p "$sdk_root/bin"

{
    printf '%s\n' '#!/bin/sh'
    printf '%s\n' 'case " $* " in'
    printf '%s\n' '    *" --version "*) echo "Apple clang version 17.0.0"; exit 0 ;;'
    printf '%s\n' '    *" --target=wasm32-unknown-unknown "*)'
    printf '%s\n' "        echo \"error: unable to create target: No available targets are compatible with triple 'wasm32-unknown-unknown'\" >&2"
    printf '%s\n' '        exit 1 ;;'
    printf '%s\n' 'esac'
    printf '%s\n' 'exit 1'
} > "$apple_clang"
chmod 0755 "$apple_clang"

{
    printf '%s\n' '#!/bin/sh'
    printf '%s\n' 'case " $* " in'
    printf '%s\n' '    *" --version "*) echo "clang version 25.0.0-wasi-sdk"; exit 0 ;;'
    printf '%s\n' '    *" --target=wasm32-unknown-unknown "*) ;;'
    printf '%s\n' '    *) echo "missing wasm32 target" >&2; exit 1 ;;'
    printf '%s\n' 'esac'
    printf '%s\n' 'printf "%s\n" "$*" >> "${WASM_CC_TEST_LOG:?}"'
    printf '%s\n' 'output=' 'previous='
    printf '%s\n' 'for argument in "$@"; do'
    printf '%s\n' '    if [ "$previous" = -o ]; then output="$argument"; fi'
    printf '%s\n' '    previous="$argument"'
    printf '%s\n' 'done'
    printf '%s\n' '[ -n "$output" ] || exit 1'
    printf '%s\n' 'printf wasm-object > "$output"'
} > "$sdk_clang"
chmod 0755 "$sdk_clang"

# The exact compiler that produced #1716 must be rejected before Cargo starts,
# even when a valid SDK is available as a fallback: explicit overrides win.
set +e
bad_output="$({
    env -u 'CC_wasm32-unknown-unknown' \
        CC_wasm32_unknown_unknown="$apple_clang" \
        WASI_SDK_PATH="$sdk_root" \
        /bin/bash "$HELPER"
} 2>&1)"
bad_status=$?
set -e
if [ "$bad_status" -eq 0 ]; then
    echo "stock Apple clang unexpectedly passed the wasm32 probe" >&2
    exit 1
fi
case "$bad_output" in
    *"cannot compile for wasm32-unknown-unknown"*) ;;
    *)
        echo "invalid-compiler diagnostic did not name the target" >&2
        printf '%s\n' "$bad_output" >&2
        exit 1
        ;;
esac

# With no target-specific override, the owned SDK is selected, exported under
# cc-rs's target-specific name, and actually asked to compile a C translation
# unit for the precise Rust target.
selected="$({
    env -u 'CC_wasm32-unknown-unknown' -u CC_wasm32_unknown_unknown \
        WASI_SDK_PATH="$sdk_root" \
        WASM_CC_TEST_LOG="$probe_log" \
        /bin/bash -c '. "$1"; wasm_cc_prepare >/dev/null; printf "%s" "$CC_wasm32_unknown_unknown"' \
            wasm-cc-test "$HELPER"
})"
if [ "$selected" != "$sdk_clang" ]; then
    echo "owned wasi-sdk clang was not selected (got '$selected')" >&2
    exit 1
fi
case "$(cat "$probe_log")" in
    *"--target=wasm32-unknown-unknown"*" -c "*" -o "*) ;;
    *)
        echo "compiler probe did not compile for the exact target" >&2
        cat "$probe_log" >&2
        exit 1
        ;;
esac

run_installer_check() {
    env \
        WASI_SDK_PATH="$1" \
        WASM_CC_TEST_LOG="$probe_log" \
        SKIP_TCLSH=1 \
        SKIP_PYTHON_TK=1 \
        SKIP_NODE=1 \
        SKIP_KOTLINC=1 \
        SKIP_RUST=1 \
        SKIP_WASMTIME=1 \
        SKIP_BINARYEN=1 \
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

# Merely finding an executable named clang is not enough. --check must report
# an installed but incapable SDK compiler as missing, then accept the proven
# SDK fixture without mutating the host.
broken_sdk="$fixture/broken-sdk"
mkdir -p "$broken_sdk/bin"
cp "$apple_clang" "$broken_sdk/bin/clang"
set +e
check_output="$(run_installer_check "$broken_sdk" 2>&1)"
check_status=$?
set -e
if [ "$check_status" -ne 1 ]; then
    echo "ensure-test-deps --check accepted an incapable SDK clang" >&2
    printf '%s\n' "$check_output" >&2
    exit 1
fi
case "$check_output" in
    *"wasi-sdk 25.0 with a wasm32-unknown-unknown-capable C compiler"*) ;;
    *)
        echo "dependency audit did not report the missing wasm32 compiler" >&2
        printf '%s\n' "$check_output" >&2
        exit 1
        ;;
esac

good_output="$(run_installer_check "$sdk_root" 2>&1)"
case "$good_output" in
    *"wasi-sdk already present"*"all dependencies satisfied"*) ;;
    *)
        echo "dependency audit rejected the working SDK fixture" >&2
        printf '%s\n' "$good_output" >&2
        exit 1
        ;;
esac

echo "wasm32 C compiler bootstrap regression: ok"
