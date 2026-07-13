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
vendor="$here/../bigip-report-gen/assets"
out="$(mktemp -d)"
trap 'rm -rf "$out"' EXIT

echo "==> cargo build --target wasm32-unknown-unknown --release"
( cd "$here" && cargo build --target wasm32-unknown-unknown --release )
wasm="$here/target/wasm32-unknown-unknown/release/bigip_query_wasm.wasm"

echo "==> wasm-bindgen (no-modules)"
wasm-bindgen "$wasm" --out-dir "$out" --target no-modules --no-typescript

# ---------------------------------------------------------------------------
# Make the glue's script_src probe non-fatal.
#
# wasm-bindgen emits, at the very top of `let wasm_bindgen = (function(){…})()`:
#
#     script_src = new URL(document.currentScript.src, location.href).toString();
#
# We inline the glue into the report, so `document.currentScript.src` is "" — and
# `new URL("", base)` THROWS when `base` is a blob: URL, which is exactly what
# `location.href` is when the in-browser generator opens the report as a Blob
# (`blob:https://bitwisecook.github.io/…`). It is fine from https:// and file://,
# which is why this only bites the hosted app.
#
# When that throws, the `let` initialiser never completes and `wasm_bindgen` is
# trapped in the temporal dead zone for the whole page. Every later `typeof
# wasm_bindgen` guard then throws ReferenceError instead of returning "undefined",
# killing print.js, console.js and irule-format.js — no Print button, no Ctrl/Cmd-P,
# no query console.
#
# `script_src` is only used to derive a default `_bg.wasm` path when the caller
# passes no module. The report always passes explicit bytes, so it is dead weight
# here: swallowing the failure costs nothing and leaves `script_src` undefined.
echo "==> patching script_src probe (blob: URLs make new URL() throw)"
python3 - "$out/bigip_query_wasm.js" <<'PY'
import re, sys
p = sys.argv[1]
s = open(p, encoding="utf-8").read()
pat = re.compile(
    r"(\s*)script_src = new URL\(document\.currentScript\.src, location\.href\)\.toString\(\);"
)
if not pat.search(s):
    raise SystemExit("script_src probe not found — did wasm-bindgen change its glue?")
s = pat.sub(
    lambda m: f"{m.group(1)}try {{ script_src = new URL(document.currentScript.src, location.href).toString(); }}"
              f" catch (_) {{ script_src = undefined; }}  // blob: base throws; see build-wasm.sh",
    s, count=1,
)
open(p, "w", encoding="utf-8").write(s)
print("    patched")
PY

echo "==> wasm-opt -Os"
wasm-opt -Os "$out/bigip_query_wasm_bg.wasm" -o "$vendor/f5query_wasm_bg.wasm"
cp "$out/bigip_query_wasm.js" "$vendor/f5query_wasm.js"

echo "==> vendored:"
ls -la "$vendor"/f5query_wasm*
