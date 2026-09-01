#!/usr/bin/env bash
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# SPDX-License-Identifier: AGPL-3.0-or-later

# Verify the version reported at runtime by every native release binary.
#
# Usage:
#   scripts/verify-native-versions.sh VERSION BINDIR [RUNNER [ARG...]]
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

for binary in tcl-lsp-server tcl-mcp tcl f5-query; do
	if [[ ! -x "$bindir/$binary" ]]; then
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

lsp_request='{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"processId":null,"rootUri":null,"capabilities":{}}}'
lsp_output="$(frame "$lsp_request" | run_binary "$bindir/tcl-lsp-server" 2>/dev/null || true)"
require_version tcl-lsp-server "$lsp_output"

mcp_request='{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"release-check","version":"0"}}}'
mcp_output="$(printf '%s\n' "$mcp_request" | run_binary "$bindir/tcl-mcp" 2>/dev/null | head -1 || true)"
require_version tcl-mcp "$mcp_output"

for binary in tcl f5-query; do
	output="$(run_binary "$bindir/$binary" --version 2>&1 || true)"
	require_version "$binary" "$output"
done
