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
#
# Build and run the DYNAMIC side-module C-extension spike.
#
#   runtime/  (Rust cdylib) --rustc--> tcl_ext_spike_dyn_runtime.wasm
#                                       (exports memory + table + Tcl C API)
#   ../ext/pkga.c (unmodified) --clang/-fPIC--> pkga_pic.o
#                              --wasm-ld -shared--> build/pkga.side.wasm
#   loader.py: instantiate runtime, load side module at allocated bases, run it.
#
# Exit 0 on SPIKE PASS.
set -euo pipefail
cd "$(dirname "$0")"
export RUSTUP_TOOLCHAIN=stable
mkdir -p build

echo ">> build runtime cdylib (wasm32-unknown-unknown, exported + growable table)"
# --export-table exposes __indirect_function_table; --growable-table drops its
# fixed max so the loader can append the side module's functions.
RUSTFLAGS="-C link-arg=--export-table -C link-arg=--growable-table" \
    cargo build --quiet --manifest-path runtime/Cargo.toml --target wasm32-unknown-unknown
RT=runtime/target/wasm32-unknown-unknown/debug/tcl_ext_spike_dyn_runtime.wasm

echo ">> build side module from UNMODIFIED ../ext/pkga.c (zig cc -fPIC, wasm-ld -shared)"
# zig cc bundles wasi-libc so tcl.h's libc includes resolve; pkga.c uses no
# libc, so the side module stays free of extra imports.
zig cc --target=wasm32-wasi -fPIC -fvisibility=default -O2 \
    -c -I ../include ../ext/pkga.c -o build/pkga_pic.o
wasm-ld --experimental-pic -shared --no-entry --import-memory --import-table \
    build/pkga_pic.o -o build/pkga.side.wasm

echo ">> run loader"
echo
exec uv run --with wasmtime python loader.py "$RT" build/pkga.side.wasm
