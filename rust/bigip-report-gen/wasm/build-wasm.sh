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

# Build the BIG-IP report-generator WASM and assemble a single, self-contained
# hosting page (`dist/index.html`) that runs the whole pipeline in the browser —
# upload a UCS/SCF, get a standalone HTML report, entirely client-side.
#
# Requires: the rustup wasm32-unknown-unknown target, wasm-bindgen-cli (matching
# the wasm-bindgen crate version pinned in Cargo.toml), and python3 (for the
# byte-safe asset injection). Node, if present, is used to verify the module.
set -euo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
dist="$here/dist"
out="$(mktemp -d)"
trap 'rm -rf "$out"' EXIT
mkdir -p "$dist"

echo "==> cargo build --target wasm32-unknown-unknown --release"
( cd "$here" && cargo build --target wasm32-unknown-unknown --release )
wasm="$here/target/wasm32-unknown-unknown/release/bigip_report_wasm.wasm"

echo "==> wasm-bindgen (no-modules)"
wasm-bindgen "$wasm" --out-dir "$out" --target no-modules --no-typescript

# wasm-opt is intentionally NOT run. On modern rustc layouts, binaryen rebinds
# the `__wbindgen_externrefs` export from the growable externref table onto the
# fixed-size funcref table, so `Table.grow` throws at runtime ("could not grow
# the table") and the page never initialises — the same regression that broke
# the compiler explorer. The raw wasm-bindgen output has the correct binding;
# gzipped it is within ~1% of the -Os output.
echo "==> verifying the externref table is growable"
if command -v node >/dev/null 2>&1; then
    node "$here/../../../scripts/verify-wasm-externref.mjs" "$out/bigip_report_wasm_bg.wasm"
else
    echo "    note: node not found — skipping wasm growability check"
fi

echo "==> assembling single-file dist/index.html from the shared builder page"
# The builder page (styles + controller) is the shared front-end; this app just
# inlines the wasm generator behind it. `shared/dist/input.js` must be built
# first (`cd ../frontend && npm run build`); CI builds it before this.
shared="$here/../frontend"
# input.html is the shared builder-page template (canonical copy under the
# report-gen `templates/` dir — the same one build.mjs reads via `../templates`);
# input.css/input.js come from the front-end (src + built dist/).
python3 - \
    "$here/../templates/input.html" "$shared/src/styles/input.css" \
    "$shared/dist/input.js" "$out/bigip_report_wasm.js" \
    "$out/bigip_report_wasm_bg.wasm" "$dist/index.html" "$here/../assets" <<'PY'
import base64, os, sys
tmpl_path, css_path, js_path, glue_path, wasm_path, out_path, assets_dir = sys.argv[1:8]
tmpl = open(tmpl_path, encoding="utf-8").read()
css = open(css_path, encoding="utf-8").read()
js = open(js_path, encoding="utf-8").read()
glue = open(glue_path, encoding="utf-8").read()
b64 = base64.b64encode(open(wasm_path, "rb").read()).decode("ascii")
payload = (
    '<script id="report-wasm" type="application/octet-stream">' + b64 + "</script>\n"
    "<script>" + glue + "</script>"
)
# The project marks, inlined as <svg> — the same asset files the report embeds.
LOGOS = {
    "__LOGO_F5Q__": "logo-f5q.svg",
    "__LOGO_TCL_LSP__": "logo-tcl-lsp.svg",
    "__LOGO_TCL_LSP_DARK__": "logo-tcl-lsp-dark.svg",
}
# str.replace is a literal replace (not regex), so backslashes/ampersands in the
# payload are safe.
tmpl = tmpl.replace("__STYLES__", "<style>" + css + "</style>")
tmpl = tmpl.replace("__WASM_PAYLOAD__", payload)
tmpl = tmpl.replace("__INPUT_JS__", "<script>" + js + "</script>")
for tok, name in LOGOS.items():
    mark = open(os.path.join(assets_dir, name), encoding="utf-8").read()
    tmpl = tmpl.replace(tok, mark)
for tok in ("__STYLES__", "__WASM_PAYLOAD__", "__INPUT_JS__", *LOGOS):
    if tok in tmpl:
        raise SystemExit(f"placeholder {tok} substitution failed")
open(out_path, "w", encoding="utf-8").write(tmpl)
print(f"    wrote {out_path} ({len(tmpl)/1024/1024:.2f} MiB, wasm {len(b64)/1024/1024:.2f} MiB b64)")
PY

echo "==> done:"
ls -lh "$dist/index.html"
