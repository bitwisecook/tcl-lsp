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
build="rust/tcl-lsp-server-wasi/build-wasi.sh"

grep -Fq "CARGO_PROFILE_RELEASE_LTO: \${{ startsWith(github.ref, 'refs/tags/v') && 'true' || 'thin' }}" "$workflow" \
    || { echo "functional WASI step must select thin LTO only off tags" >&2; exit 1; }
grep -Fq 'CARGO_PROFILE_RELEASE_LTO: true' "$workflow" \
    || { echo "release WASI step must select fat LTO" >&2; exit 1; }

awk '
  /name: Build and exercise the WASI LSP server/ { functional=1 }
  functional && /CARGO_PROFILE_RELEASE_LTO: \$\{\{ startsWith\(github.ref/ { found=1 }
  /name: Build the WASI LSP server \(release artefact\)/ { release=1 }
  release && /CARGO_PROFILE_RELEASE_LTO: true/ { found_release=1 }
  END { exit !(found && found_release) }
' "$workflow" \
    || { echo "LTO selections are not scoped to the intended workflow steps" >&2; exit 1; }

grep -Fq 'CARGO_PROFILE_RELEASE_LTO="$lto_mode"' "$build" \
    || { echo "build script must pass its selected LTO mode to Cargo" >&2; exit 1; }
grep -Fq 'wasm-opt -Os' "$build" \
    || { echo "WASI build must retain wasm-opt -Os" >&2; exit 1; }
grep -Fq 'wasm-opt elapsed:' "$build" \
    || { echo "build script must report wasm-opt timing" >&2; exit 1; }
grep -Fq 'run: make lsp-server-wasi-test' "$workflow" \
    || { echo "functional WASI step must retain the full e2e harness" >&2; exit 1; }

echo "WASI LTO workflow contract passed"
