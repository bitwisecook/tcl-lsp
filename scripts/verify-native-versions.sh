#!/usr/bin/env bash
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# SPDX-License-Identifier: AGPL-3.0-or-later

# Verify the version reported at runtime by native release binaries.
#
# Usage:
#   scripts/verify-native-versions.sh VERSION BINDIR [RUNNER [ARG...]]
#
# By default all four shipping binaries are required. A feature-isolated
# matrix leg sets TCL_LSP_RELEASE_BINARIES to one or more space-separated
# names from that same set.
#
# RUNNER is optional. For example, a RISC-V build can be checked with:
#   scripts/verify-native-versions.sh 2.2.1+g12345678 DIR \
#     qemu-riscv64 -L /usr/riscv64-linux-gnu
#
# Runtime checks are intentional. Release LTO may materialise a short string as
# immediate machine-code operands, so `strings` is not a reliable way to prove
# what an optimised binary reports.

set -euo pipefail

if [[ $# -lt 2 ]]; then
	printf 'usage: %s VERSION BINDIR [RUNNER [ARG...]]\n' "$0" >&2
	exit 2
fi

expected="$1"
bindir="$2"
shift 2
runner=("$@")

read -r -a binaries <<<"${TCL_LSP_RELEASE_BINARIES-tcl-lsp-server tcl-mcp tcl f5-query}"
if [[ ${#binaries[@]} -eq 0 ]]; then
	printf 'error: TCL_LSP_RELEASE_BINARIES selected no binaries\n' >&2
	exit 2
fi

native_executable() {
	local binary="$1"
	if [[ -x "$bindir/$binary" ]]; then
		printf '%s\n' "$bindir/$binary"
	elif [[ -x "$bindir/$binary.exe" ]]; then
		printf '%s\n' "$bindir/$binary.exe"
	else
		return 1
	fi
}

for binary in "${binaries[@]}"; do
	case "$binary" in
		tcl-lsp-server | tcl-mcp | tcl | f5-query) ;;
		*)
			printf 'error: unsupported native release binary: %s\n' "$binary" >&2
			exit 2
			;;
	esac
	if ! native_executable "$binary" >/dev/null; then
		printf 'error: executable release binary not found: %s\n' "$bindir/$binary" >&2
		exit 2
	fi
done

run_binary() {
	if command -v timeout >/dev/null 2>&1; then
		timeout 30 "${runner[@]}" "$@"
	else
		"${runner[@]}" "$@"
	fi
}

require_version() {
	local label="$1"
	local output="$2"
	if [[ "$output" != *"$expected"* ]]; then
		printf 'error: %s did not report %s\n' "$label" "$expected" >&2
		printf 'output: %.240s\n' "$output" >&2
		exit 1
	fi
	printf 'PASS  %-14s %s\n' "$label" "$expected"
}

frame() {
	local body="$1"
	printf 'Content-Length: %d\r\n\r\n%s' "${#body}" "$body"
}

for binary in "${binaries[@]}"; do
	executable="$(native_executable "$binary")"
	case "$binary" in
		tcl-lsp-server)
			request='{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"processId":null,"rootUri":null,"capabilities":{}}}'
			output="$(frame "$request" | run_binary "$executable" 2>/dev/null || true)"
			;;
		tcl-mcp)
			request='{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"release-check","version":"0"}}}'
			output="$(printf '%s\n' "$request" | run_binary "$executable" 2>/dev/null | head -1 || true)"
			;;
		tcl | f5-query)
			output="$(run_binary "$executable" --version 2>&1 || true)"
			;;
	esac
	require_version "$binary" "$output"
done
