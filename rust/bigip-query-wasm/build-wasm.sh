#!/usr/bin/env bash
# tcl-lsp — a language server and toolchain for Tcl
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
# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.
#
# SPDX-License-Identifier: AGPL-3.0-or-later

# Build the wasm32 query engine and vendor the artifacts into the f5report
# Python package, so `f5report` can embed a live in-browser query console
# without needing the wasm toolchain at report-generation time.
#
# Requires: rustup wasm32-unknown-unknown target, wasm-bindgen-cli (matching the
# wasm-bindgen crate version pinned in Cargo.toml), and wasm-opt (binaryen).
set -euo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
vendor="$here/../bigip-query-py/python/f5report/vendor"
out="$(mktemp -d)"
trap 'rm -rf "$out"' EXIT

echo "==> cargo build --target wasm32-unknown-unknown --release"
( cd "$here" && cargo build --target wasm32-unknown-unknown --release )
wasm="$here/target/wasm32-unknown-unknown/release/bigip_query_wasm.wasm"

echo "==> wasm-bindgen (no-modules)"
wasm-bindgen "$wasm" --out-dir "$out" --target no-modules --no-typescript

echo "==> wasm-opt -Os"
wasm-opt -Os "$out/bigip_query_wasm_bg.wasm" -o "$vendor/f5query_wasm_bg.wasm"
cp "$out/bigip_query_wasm.js" "$vendor/f5query_wasm.js"

echo "==> vendored:"
ls -la "$vendor"/f5query_wasm*
