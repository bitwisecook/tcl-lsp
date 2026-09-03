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

# Build the browser transport for the Tcl language server into `dist/`:
# the wasm module, the wasm-bindgen no-modules glue, and the Web Worker that
# ties the two to `postMessage`. Three plain files, no bundler — a page loads
# `worker.js` with `new Worker(...)` and speaks raw LSP JSON-RPC to it.
#
# Requires: the rustup wasm32-unknown-unknown target, a WASM-capable C compiler
# (`make ensure-rust-deps` provisions one), and wasm-bindgen-cli (matching the
# wasm-bindgen crate version resolved in Cargo.lock). Node, if present, also
# verifies the module.
set -euo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
# shellcheck source=../../scripts/dev/wasm-cc-env.sh
. "$here/../../scripts/dev/wasm-cc-env.sh"
dist="$here/dist"
mkdir -p "$dist"

# Every analysis thread's stack budget — the wasm equivalent of `main.rs`'s
# WORKER_STACK_SIZE, and load-bearing for the same reason (issue #996).
#
# The analyser's `analyse_body` recursion and the CFG builder's `lower_script`
# recursion each cap their nesting depth, but a cap on the *number* of frames
# says nothing about how much stack those frames need. The native server gives
# its Tokio workers 64 MiB because 2 MiB overflows around nesting depth
# 130-140, well inside the cap. rust-lld's default wasm stack is 1 MiB —
# smaller still — and a wasm stack overflow is not a clean panic: it silently
# corrupts the shadow stack or traps with `unreachable`, so the failure mode is
# worse than native's. Match the native budget.
STACK_SIZE=$((64 * 1024 * 1024))

wasm_cc_prepare
echo "==> cargo build --target wasm32-unknown-unknown --release (stack ${STACK_SIZE} bytes)"
(
    cd "$here"
    # This crate is its own workspace with its own lockfile, so it gets its own
    # target dir too. Pinning it here rather than inheriting the environment is
    # what keeps the output path below correct when the caller has exported a
    # CARGO_TARGET_DIR for the main workspace (scripts/dev/agent-build-env.sh).
    CARGO_TARGET_DIR="$here/target" \
    RUSTFLAGS="${RUSTFLAGS:-} -C link-arg=-zstack-size=${STACK_SIZE}" \
        cargo build --target wasm32-unknown-unknown --release
)
wasm="$here/target/wasm32-unknown-unknown/release/tcl_lsp_server_wasm.wasm"

echo "==> wasm-bindgen (no-modules)"
command -v wasm-bindgen >/dev/null 2>&1 || {
    echo "wasm-bindgen not found — 'cargo install wasm-bindgen-cli'" >&2
    exit 1
}
wasm-bindgen "$wasm" --out-dir "$dist" --target no-modules --no-typescript

# wasm-opt is intentionally NOT run (also disabled in Cargo.toml's
# wasm-pack metadata). On modern rustc layouts, binaryen rebinds the
# `__wbindgen_externrefs` export from the growable externref table onto the
# fixed-size funcref table, so `Table.grow` throws at runtime ("could not grow
# the table") and the worker never initialises — the same regression that broke
# the compiler explorer, the report generator, and the spec studio. The raw
# wasm-bindgen output has the correct binding; gzipped it is within ~1% of the
# -Os output.
echo "==> verifying the externref table is growable"
if command -v node >/dev/null 2>&1; then
    node "$here/../../scripts/verify-wasm-externref.mjs" "$dist/tcl_lsp_server_wasm_bg.wasm"
else
    echo "    note: node not found — skipping wasm growability check"
fi

cp "$here/worker.js" "$dist/worker.js"

echo "==> done:"
ls -lh "$dist"
raw=$(wc -c <"$dist/tcl_lsp_server_wasm_bg.wasm")
gz=$(gzip -9 -c "$dist/tcl_lsp_server_wasm_bg.wasm" | wc -c)
printf '    wasm %s bytes raw, %s bytes gzipped\n' "$raw" "$gz"
