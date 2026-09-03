#!/usr/bin/env bash
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# SPDX-License-Identifier: AGPL-3.0-or-later

# Select and prove the C compiler used by cc-rs for wasm32-unknown-unknown.
#
# Stock Apple clang has no WebAssembly backend. Rust-only crates can therefore
# cross-compile successfully until a dependency such as ring invokes cc-rs,
# where the build fails much later with an opaque "No available targets"
# diagnostic. Source this helper and call wasm_cc_prepare before Cargo so the
# exact compiler Cargo will inherit is selected and exercised up front.

WASM_CC_TARGET="wasm32-unknown-unknown"

wasm_cc_run() {
    local compiler="$1"
    shift
    local -a command

    # cc-rs permits a wrapper plus compiler in CC (for example "sccache
    # clang"). Preserve that convention for explicit overrides. Paths used by
    # the owned wasi-sdk do not contain whitespace.
    read -r -a command <<< "$compiler"
    if [ "${#command[@]}" -eq 0 ] || ! command -v "${command[0]}" >/dev/null 2>&1; then
        return 127
    fi
    "${command[@]}" "$@"
}

wasm_cc_probe() {
    local compiler="$1"
    local probe_dir status
    probe_dir="$(mktemp -d "${TMPDIR:-/tmp}/tcl-lsp-wasm-cc.XXXXXX")"
    printf '%s\n' 'int tcl_lsp_wasm_cc_probe(void) { return 0; }' > "$probe_dir/probe.c"

    if wasm_cc_run "$compiler" --target="$WASM_CC_TARGET" -x c -c \
        "$probe_dir/probe.c" -o "$probe_dir/probe.o"; then
        status=0
    else
        status=$?
    fi

    if [ "$status" -eq 0 ] && [ -s "$probe_dir/probe.o" ]; then
        rm -rf "$probe_dir"
        return 0
    fi
    rm -rf "$probe_dir"
    return 1
}

wasm_cc_export() {
    local compiler="$1"
    CC_wasm32_unknown_unknown="$compiler"
    export CC_wasm32_unknown_unknown
    printf '==> wasm32 C compiler: %s\n' "$compiler"
}

wasm_cc_prepare() {
    local compiler candidate
    local target_specific_hyphen
    target_specific_hyphen="$(printenv 'CC_wasm32-unknown-unknown' 2>/dev/null || true)"

    # cc-rs gives the hyphenated target variable precedence over the
    # shell-friendly underscore spelling. Respect either explicit choice and
    # fail on it rather than silently replacing a developer's override.
    compiler="${target_specific_hyphen:-${CC_wasm32_unknown_unknown:-}}"
    if [ -n "$compiler" ]; then
        if wasm_cc_probe "$compiler"; then
            wasm_cc_export "$compiler"
            return 0
        fi
        printf "ERROR: configured wasm32 C compiler '%s' cannot compile for %s.\n" \
            "$compiler" "$WASM_CC_TARGET" >&2
        printf '%s\n' \
            "       Run 'make ensure-rust-deps' or set CC_wasm32_unknown_unknown to a WASM-capable clang." >&2
        return 1
    fi

    # An explicit SDK root is authoritative. A typo must not quietly fall back
    # to a different host installation.
    if [ -n "${WASI_SDK_PATH:-}" ]; then
        candidate="$WASI_SDK_PATH/bin/clang"
        if [ -x "$candidate" ] && wasm_cc_probe "$candidate"; then
            wasm_cc_export "$candidate"
            return 0
        fi
        printf "ERROR: WASI_SDK_PATH '%s' has no clang capable of targeting %s.\n" \
            "$WASI_SDK_PATH" "$WASM_CC_TARGET" >&2
        return 1
    fi

    # Prefer the project-owned SDK. Homebrew LLVM is a useful existing
    # installation on macOS; PATH clang is last because it is normally the
    # incapable Apple compiler there.
    for candidate in \
        /opt/wasi-sdk/bin/clang \
        /opt/homebrew/opt/llvm/bin/clang \
        /usr/local/opt/llvm/bin/clang \
        "$(command -v clang 2>/dev/null || true)"; do
        [ -n "$candidate" ] || continue
        [ -x "$candidate" ] || continue
        if wasm_cc_probe "$candidate" >/dev/null 2>&1; then
            wasm_cc_export "$candidate"
            return 0
        fi
    done

    printf 'ERROR: no C compiler capable of targeting %s was found.\n' \
        "$WASM_CC_TARGET" >&2
    printf '%s\n' \
        "       Run 'make ensure-rust-deps'; on macOS it installs the pinned wasi-sdk." >&2
    return 1
}

if [[ "${BASH_SOURCE[0]}" == "$0" ]]; then
    wasm_cc_prepare
fi
