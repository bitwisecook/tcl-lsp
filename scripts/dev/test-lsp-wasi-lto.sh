#!/usr/bin/env bash
# tcl-lsp — a Tcl language server and toolchain
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
#
# SPDX-License-Identifier: AGPL-3.0-or-later

set -euo pipefail

workflow="${1:-.github/workflows/ci.yml}"
makefile="${2:-Makefile}"
build="rust/tcl-lsp-server-wasi/build-wasi.sh"

job_block="$({
    awk '
      /^  lsp-server-wasi:$/ { capture=1 }
      capture && /^  [A-Za-z0-9_-]+:$/ && $0 != "  lsp-server-wasi:" { exit }
      capture { print }
    ' "$workflow"
})"
[[ -n "$job_block" ]] \
    || { echo "lsp-server-wasi job is missing" >&2; exit 1; }

step_block() {
    local name="$1"
    awk -v wanted="      - name: ${name}" '
      $0 == wanted { capture=1 }
      capture && /^      - name: / && $0 != wanted { exit }
      capture { print }
    ' <<<"$job_block"
}

functional_block="$(step_block 'Build and exercise the WASI LSP server')"
release_block="$(step_block 'Build the WASI LSP server (release artefact)')"
[[ -n "$functional_block" && -n "$release_block" ]] \
    || { echo "WASI build steps are missing from the lsp-server-wasi job" >&2; exit 1; }

[[ "$(grep -Fc 'CARGO_PROFILE_RELEASE_LTO:' <<<"$job_block")" -eq 2 ]] \
    || { echo "lsp-server-wasi must own exactly two explicit LTO selections" >&2; exit 1; }
grep -Fqx "          CARGO_PROFILE_RELEASE_LTO: \${{ startsWith(github.ref, 'refs/tags/v') && 'true' || 'thin' }}" <<<"$functional_block" \
    || { echo "functional WASI step must select thin LTO only off tags" >&2; exit 1; }
grep -Fqx '        run: make lsp-server-wasi-test' <<<"$functional_block" \
    || { echo "functional WASI step must retain the full test harness" >&2; exit 1; }
grep -Fqx '          CARGO_PROFILE_RELEASE_LTO: true' <<<"$release_block" \
    || { echo "release WASI step must select fat LTO" >&2; exit 1; }
grep -Fqx '        run: make lsp-server-wasi' <<<"$release_block" \
    || { echo "release WASI step must invoke the release-only build" >&2; exit 1; }

grep -Fq 'CARGO_PROFILE_RELEASE_LTO="$lto_mode"' "$build" \
    || { echo "build script must pass its selected LTO mode to Cargo" >&2; exit 1; }
grep -Fq 'wasm-opt -Os' "$build" \
    || { echo "WASI build must retain wasm-opt -Os" >&2; exit 1; }
grep -Fq 'wasm-opt elapsed:' "$build" \
    || { echo "build script must report wasm-opt timing" >&2; exit 1; }
wasi_test_block="$({
    awk '
      /^lsp-server-wasi-test:/ { capture=1 }
      capture && /^[^[:space:]#][^=]*:/ && $0 !~ /^lsp-server-wasi-test:/ { exit }
      capture { print }
    ' "$makefile"
})"
[[ -n "$wasi_test_block" ]] \
    || { echo "lsp-server-wasi-test target is missing" >&2; exit 1; }
grep -Fq '@command -v wasmtime >/dev/null 2>&1 || {' <<<"$wasi_test_block" \
    || { echo "WASI runtime coverage must fail closed without wasmtime" >&2; exit 1; }
grep -Fq 'cargo test --target wasm32-wasip1' <<<"$wasi_test_block" \
    || { echo "WASI framing unit tests are missing" >&2; exit 1; }
grep -Fq 'node $(LSP_SERVER_WASI_DIR)/test/e2e.mjs' <<<"$wasi_test_block" \
    || { echo "WASI end-to-end tests are missing" >&2; exit 1; }

echo "WASI LTO workflow contract passed"
