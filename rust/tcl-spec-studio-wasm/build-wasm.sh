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

# Build the command-registry spec studio and assemble it into ONE
# self-contained page (`dist/index.html`): the wasm (registry + compiler
# analyser + both renderers) base64-inlined, the wasm-bindgen glue, the
# stylesheet, the controller, and the project mark all embedded. No external
# fetch, no build step at load time, and `connect-src 'none'` so it cannot
# make a network request even if it wanted to.
#
# Requires: the rustup wasm32-unknown-unknown target, wasm-bindgen-cli (matching
# the wasm-bindgen crate version resolved in Cargo.lock), python3 (for the
# byte-safe asset injection), and the front-end bundle at
# `../tcl-spec-studio/web/dist/studio.js` (`cd ../tcl-spec-studio/web && npm ci
# && npm run build`; `make spec-studio-wasm` does it for you). Node, if
# present, also verifies the module.
set -euo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
web="$here/../tcl-spec-studio/web"
assets="$here/../bigip-report-gen/assets"
dist="$here/dist"
out="$(mktemp -d)"
trap 'rm -rf "$out"' EXIT
mkdir -p "$dist"

echo "==> cargo build --target wasm32-unknown-unknown --release"
( cd "$here" && cargo build --target wasm32-unknown-unknown --release )
wasm="$here/target/wasm32-unknown-unknown/release/tcl_spec_studio_wasm.wasm"

echo "==> wasm-bindgen (no-modules)"
wasm-bindgen "$wasm" --out-dir "$out" --target no-modules --no-typescript

# Make the glue's script_src probe non-fatal.
#
# wasm-bindgen emits, at the top of `let wasm_bindgen = (function(){…})()`:
#
#     script_src = new URL(document.currentScript.src, location.href).toString();
#
# The glue is inlined, so `document.currentScript.src` is "" — and
# `new URL("", base)` THROWS when `base` is a blob: URL. The probe only derives
# a default `_bg.wasm` path when the caller passes no module, and we always pass
# explicit bytes, so swallowing the failure is free. Same patch the BIG-IP
# report generator applies, for the same reason.
echo "==> patching script_src probe (blob: URLs make new URL() throw)"
python3 - "$out"/*.js <<'PY'
import re, sys
pat = re.compile(
    r"(\s*)script_src = new URL\(document\.currentScript\.src, location\.href\)\.toString\(\);"
)
patched = 0
for p in sys.argv[1:]:
    s = open(p, encoding="utf-8").read()
    if not pat.search(s):
        continue
    s = pat.sub(
        lambda m: f"{m.group(1)}try {{ script_src = new URL(document.currentScript.src, location.href).toString(); }}"
                  f" catch (_) {{ script_src = undefined; }}  // blob: base throws; see build-wasm.sh",
        s, count=1,
    )
    open(p, "w", encoding="utf-8").write(s)
    patched += 1
if not patched:
    raise SystemExit("script_src probe not found — did wasm-bindgen change its glue?")
print(f"    patched {patched} glue file(s)")
PY

# wasm-opt is intentionally NOT run. On modern rustc layouts, binaryen rebinds
# the `__wbindgen_externrefs` export from the growable externref table onto the
# fixed-size funcref table, so `Table.grow` throws at runtime ("could not grow
# the table") and the page never initialises — the same regression that broke
# the compiler explorer and the report generator. The raw wasm-bindgen output
# has the correct binding; gzipped it is within ~1% of the -Os output.
echo "==> verifying the externref table is growable"
if command -v node >/dev/null 2>&1; then
    node "$here/../../scripts/verify-wasm-externref.mjs" "$out/tcl_spec_studio_wasm_bg.wasm"
else
    echo "    note: node not found — skipping wasm growability check"
fi

if [ ! -f "$web/dist/studio.js" ]; then
    echo "error: $web/dist/studio.js is missing — build the front-end first:" >&2
    echo "       cd $web && npm ci && npm run build" >&2
    exit 1
fi

echo "==> assembling single-file dist/index.html"
python3 - \
    "$web/studio.html" "$web/src/studio.css" "$web/dist/studio.js" \
    "$out/tcl_spec_studio_wasm.js" "$out/tcl_spec_studio_wasm_bg.wasm" \
    "$dist/index.html" "$assets" <<'PY'
import base64, os, sys
tmpl_path, css_path, js_path, glue_path, wasm_path, out_path, assets_dir = sys.argv[1:8]
tmpl = open(tmpl_path, encoding="utf-8").read()
css = open(css_path, encoding="utf-8").read()
js = open(js_path, encoding="utf-8").read()
glue = open(glue_path, encoding="utf-8").read()
b64 = base64.b64encode(open(wasm_path, "rb").read()).decode("ascii")
payload = (
    '<script id="studio-wasm" type="application/octet-stream">' + b64 + "</script>\n"
    "<script>" + glue + "</script>"
)
# The project mark, inlined as <svg> — the same asset files the BIG-IP report
# builder embeds, so the two tools carry an identical logo.
LOGOS = {
    "__LOGO_TCL_LSP__": "logo-tcl-lsp.svg",
    "__LOGO_TCL_LSP_DARK__": "logo-tcl-lsp-dark.svg",
}
# Fully self-contained: forbid ALL network egress (`connect-src 'none'`) so an
# unreleased spec or an imported proprietary package can never be uploaded. The
# allowances cover only the inlined machinery — inline scripts/styles, wasm
# instantiation, blob downloads. `form-action 'none'` blocks form posts;
# opening a GitHub issue is a `window.open` top-level navigation, which no CSP
# directive here restricts and which the user sees before anything is posted.
CSP = (
    "default-src 'none'; script-src 'unsafe-inline' 'wasm-unsafe-eval'; "
    "style-src 'unsafe-inline'; img-src data: blob:; connect-src 'none'; "
    "base-uri 'none'; form-action 'none'"
)
# str.replace is a literal replace (not regex), so backslashes/ampersands in the
# payload are safe.
tmpl = tmpl.replace("__CSP__", CSP)
tmpl = tmpl.replace("__STYLES__", "<style>" + css + "</style>")
tmpl = tmpl.replace("__WASM_PAYLOAD__", payload)
tmpl = tmpl.replace("__STUDIO_JS__", "<script>" + js + "</script>")
for tok, name in LOGOS.items():
    mark = open(os.path.join(assets_dir, name), encoding="utf-8").read()
    tmpl = tmpl.replace(tok, mark)
for tok in ("__CSP__", "__STYLES__", "__WASM_PAYLOAD__", "__STUDIO_JS__", *LOGOS):
    if tok in tmpl:
        raise SystemExit(f"placeholder {tok} substitution failed")
open(out_path, "w", encoding="utf-8").write(tmpl)
print(f"    wrote {out_path} ({len(tmpl)/1024/1024:.2f} MiB, wasm {len(b64)/1024/1024:.2f} MiB b64)")
PY

echo "==> done:"
ls -lh "$dist/index.html"
