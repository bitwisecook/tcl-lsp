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

# Build the WASI stdio transport for the Tcl language server into `dist/`:
# one `.wasm` module, and nothing else. No bindgen glue and no worker — a WASI
# module is a program, so a host runs it with `wasmtime run` (or
# `@vscode/wasm-wasi`) and speaks Content-Length-framed LSP to its stdio.
#
# Requires: the rustup wasm32-wasip1 target. `wasm-opt` (binaryen) is used when
# present and skipped with a note when it is not.
set -euo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
dist="$here/dist"
mkdir -p "$dist"

# Every analysis stack's budget — the wasip1 twin of `main.rs`'s
# WORKER_STACK_SIZE and of the browser build's STACK_SIZE, load-bearing for the
# same reason (issue #996).
#
# The analyser's `analyse_body` recursion and the CFG builder's `lower_script`
# recursion each cap their nesting depth, but a cap on the *number* of frames
# says nothing about how much stack those frames need. The native server gives
# its Tokio workers 64 MiB because 2 MiB overflows around nesting depth
# 130-140, well inside the cap. rust-lld's default wasm stack is 1 MiB —
# smaller still — and a wasm stack overflow is not a clean panic: it silently
# corrupts the shadow stack or traps with `unreachable`, so the failure mode is
# worse than native's. Match the native budget. Mirrored as `STACK_SIZE` in
# src/main.rs; keep the two in step.
STACK_SIZE=$((64 * 1024 * 1024))

echo "==> cargo build --target wasm32-wasip1 --release (stack ${STACK_SIZE} bytes)"
(
    cd "$here"
    # This crate is its own workspace with its own lockfile, so it gets its own
    # target dir too. Pinning it here rather than inheriting the environment is
    # what keeps the output path below correct when the caller has exported a
    # CARGO_TARGET_DIR for the main workspace (scripts/dev/agent-build-env.sh).
    CARGO_TARGET_DIR="$here/target" \
    RUSTFLAGS="${RUSTFLAGS:-} -C link-arg=-zstack-size=${STACK_SIZE}" \
        cargo build --target wasm32-wasip1 --release
)
built="$here/target/wasm32-wasip1/release/tcl-lsp-server-wasi.wasm"
out="$dist/tcl-lsp-server-wasi.wasm"

# Unlike the browser build, `wasm-opt` is SAFE here and is run. The browser
# crate skips it because binaryen rebinds wasm-bindgen's `__wbindgen_externrefs`
# export from the growable externref table onto the fixed-size funcref table,
# which breaks `Table.grow` at run time. This module has no wasm-bindgen glue,
# no externref table, and no JS to rebind against — it is a plain WASI command
# module — so none of that applies.
if command -v wasm-opt >/dev/null 2>&1; then
    echo "==> wasm-opt -Os"
    wasm-opt -Os "$built" -o "$out"
else
    echo "    note: wasm-opt not found — shipping the unoptimised link"
    cp "$built" "$out"
fi

echo "==> done:"
ls -lh "$dist"
raw=$(wc -c <"$out")
gz=$(gzip -9 -c "$out" | wc -c)
printf '    wasm %s bytes raw, %s bytes gzipped\n' "$raw" "$gz"
