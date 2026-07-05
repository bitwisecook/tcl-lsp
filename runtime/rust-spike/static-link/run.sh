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

# ============================================================================
# SPIKE -- throwaway proof-of-concept. NOT the final design.
# ============================================================================
# Build and run the Rust-runtime C-extension spike under wasmtime.
#
#   clang  ext/pkga.c (unmodified)  --target=wasm32-wasi -c   ->  pkga.o
#   rustc  src/main.rs (Rust runtime) --target wasm32-wasip1  +  pkga.o
#          ---- linked by wasm-ld into ----                       tcl_ext_spike.wasm
#   wasmtime run tcl_ext_spike.wasm
#
# Exit 0 on SPIKE PASS, non-zero otherwise.
set -euo pipefail
cd "$(dirname "$0")"

TARGET=wasm32-wasip1
echo ">> cargo build --target $TARGET"
cargo build --quiet --target "$TARGET"

WASM="target/$TARGET/debug/tcl_ext_spike.wasm"
echo ">> wasmtime run $WASM"
echo
exec wasmtime run "$WASM"
