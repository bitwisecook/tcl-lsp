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

workflow="${1:-.github/workflows/wasi-lto-experiment.yml}"
test -f "$workflow"
grep -Fq 'pull_request:' "$workflow"
! grep -Eq 'startsWith\(github.ref|upload-artifact|gh release|release upload' "$workflow"
grep -Fq '[[ "$GITHUB_REF" != refs/tags/* ]]' "$workflow"
grep -Fq 'include:' "$workflow"
! grep -Fq 'matrix.lto' "$workflow"
test "$(grep -Fc 'repetition:' "$workflow")" -eq 3
test "$(grep -Fc 'order:' "$workflow")" -eq 3
grep -Fq 'repetition: 1' "$workflow"
grep -Fq 'repetition: 2' "$workflow"
grep -Fq 'repetition: 3' "$workflow"
grep -Fq 'order: "true thin"' "$workflow"
grep -Fq 'order: "thin true"' "$workflow"
grep -Fq 'for mode in ${{ matrix.order }}' "$workflow"
grep -Fq 'do' "$workflow"
grep -Fq 'export CARGO_PROFILE_RELEASE_LTO="$mode"' "$workflow"
grep -Fq 'rm -rf rust/tcl-lsp-server-wasi/target rust/tcl-lsp-server-wasi/dist' "$workflow"
grep -Fq '26/26 checks passed' "$workflow"
grep -Fq 'test result: ok. 22 passed' "$workflow"
grep -Fq '\"unit_tests\":22' "$workflow"
grep -Fq '\"e2e_checks\":26' "$workflow"
grep -Fq 'sha256sum' "$workflow"
grep -Fq 'GITHUB_STEP_SUMMARY' "$workflow"
echo "temporary WASI LTO experiment contract passed"
