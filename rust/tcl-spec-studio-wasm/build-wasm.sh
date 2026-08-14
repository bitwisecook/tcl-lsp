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

# Build the command-registry spec studio and assemble its `dist/`.
#
# The dist is a DIRECTORY, deployed whole:
#
#   index.html                 the page. The studio wasm (registry + compiler
#                              analyser + both renderers) is base64-inlined
#                              here along with its glue, the stylesheet, the
#                              controller, and the project mark — so the core
#                              of the app is still one self-contained file.
#   assets/monaco-host.js      the code editor + the language client, an ES
#   assets/monaco-host.css     module fetched only when an editor tab is opened.
#   lsp/worker.js              the REAL Tcl language server: the same
#   lsp/tcl_lsp_server_wasm.js `LspService` the native binary runs, compiled to
#   lsp/…_bg.wasm              wasm and driven over postMessage from a Worker.
#
# Why not one file any more: a 21 MB language server and a 2.7 MB editor cannot
# be base64-inlined into a page every visitor loads, and a Web Worker needs a
# URL of its own. Splitting them out is also what makes them *lazy* — someone
# who only browses the registry downloads neither. The core keeps its old
# property: `index.html` alone still boots, browses, edits, renders and imports,
# and says so in its status line when the editor chunk is missing.
#
# The CSP still forbids everything except what the page demonstrably needs:
# same-origin scripts/workers/fetches for the files above, and exactly two
# remote origins for the opt-in GitHub release fetcher (which is the one
# feature on the page that uses the network, and says so beside itself).
#
# Requires: the rustup wasm32-unknown-unknown target, wasm-bindgen-cli (matching
# the wasm-bindgen crate version resolved in Cargo.lock), python3 (for the
# byte-safe asset injection), the front-end bundles under
# `../tcl-spec-studio/web/dist/` (`cd ../tcl-spec-studio/web && npm ci && npm run
# build`), and the language server worker's `dist/` from
# `../tcl-lsp-server-wasm` (`make lsp-server-wasm`). `make spec-studio-wasm`
# does all of it for you. Node, if present, also verifies the module.
set -euo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
web="$here/../tcl-spec-studio/web"
assets="$here/../bigip-report-gen/assets"
lspdist="$here/../tcl-lsp-server-wasm/dist"
dist="$here/dist"
out="$(mktemp -d)"
trap 'rm -rf "$out"' EXIT
# Rebuilt from scratch each time: a stale `assets/` or `lsp/` left behind by an
# older layout would be deployed alongside the new one and served to somebody.
rm -rf "$dist"
mkdir -p "$dist/assets" "$dist/lsp"

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

for required in dist/studio.js dist/assets/monaco-host.js dist/assets/monaco-host.css; do
    if [ ! -f "$web/$required" ]; then
        echo "error: $web/$required is missing — build the front-end first:" >&2
        echo "       cd $web && npm ci && npm run build" >&2
        exit 1
    fi
done

# The language server worker, copied in whole. Its three files must stay
# together and keep their names: `worker.js` derives the other two from its own
# location (`new URL(".", self.location.href)`), so renaming or splitting them
# breaks the boot with no diagnostic beyond a failed fetch.
echo "==> copying the language server worker into dist/lsp/"
if [ ! -f "$lspdist/worker.js" ] || [ ! -f "$lspdist/tcl_lsp_server_wasm_bg.wasm" ]; then
    echo "error: $lspdist is missing or incomplete — build the server worker first:" >&2
    echo "       make lsp-server-wasm" >&2
    exit 1
fi
cp "$lspdist/worker.js" "$lspdist/tcl_lsp_server_wasm.js" \
   "$lspdist/tcl_lsp_server_wasm_bg.wasm" "$dist/lsp/"

echo "==> copying the editor chunk into dist/assets/"
cp "$web/dist/assets/monaco-host.js" "$web/dist/assets/monaco-host.css" "$dist/assets/"

echo "==> assembling dist/index.html"
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
# The narrowest policy the page can actually run under, enumerated rather than
# relaxed wholesale. Everything is `'self'` — the editor chunk, its stylesheet,
# the worker, and the fetches that pull the server's wasm — with exactly two
# remote origins on `connect-src`, and those two exist for one clearly-labelled
# opt-in panel. Nothing else on the page can reach the network, so an unreleased
# spec or an imported proprietary package still cannot be uploaded by anything
# the user did not ask for by name.
#
#   script-src   'self' for assets/monaco-host.js and the worker's glue;
#                'unsafe-inline' for the inlined controller + wasm glue;
#                'wasm-unsafe-eval' to instantiate either wasm module.
#   worker-src   'self' for lsp/worker.js. `blob:` is NOT allowed: the worker
#                ships as a real file, and permitting blob workers would open a
#                script channel the page has no use for.
#   connect-src  'self' for the worker fetching its .wasm, plus the GitHub
#                release fetcher's two hosts.
#   style-src    'self' for the editor's stylesheet, 'unsafe-inline' for the
#                inlined studio stylesheet and Monaco's own inline styles.
#   font-src     data: — Monaco's codicon font is embedded in its CSS.
#   form-action  'none' blocks form posts; opening a GitHub issue is a
#                `window.open` top-level navigation, which no directive here
#                restricts and which the user sees before anything is posted.
CSP = (
    "default-src 'none'; "
    "script-src 'self' 'unsafe-inline' 'wasm-unsafe-eval'; "
    "worker-src 'self'; "
    "style-src 'self' 'unsafe-inline'; "
    "font-src data:; "
    "img-src data: blob:; "
    "connect-src 'self' https://api.github.com https://codeload.github.com; "
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

# A relative `src`/`href` anywhere in the page would resolve against the
# deployment directory and silently 404 on GitHub Pages, where the studio lives
# under /spec-studio/. Every path the page uses is built at run time from
# `document.baseURI` or `import.meta.url` instead, so assert that no static one
# crept back in.
echo "==> checking dist/index.html references no external file"
python3 - "$dist/index.html" <<'PY'
import re, sys
html = open(sys.argv[1], encoding="utf-8").read()
# Only look at the page's own markup, not the inlined bundles (which contain
# plenty of `src=` inside strings).
head = html.split("<script id=\"studio-wasm\"", 1)[0]
bad = [
    m.group(0)
    for m in re.finditer(r"""<(?:script|link|img)\b[^>]*\b(?:src|href)=["'](?!https?:|#|data:)[^"']+["'][^>]*>""", head)
]
if bad:
    raise SystemExit("index.html links a file relatively: " + "; ".join(bad))
print("    ok — the page loads its extra files by computed URL only")
PY

echo "==> done: dist/ is deployed whole"
( cd "$dist" && find . -type f | sort | while read -r f; do
    printf '    %8s  %s\n' "$(du -h "$f" | cut -f1)" "${f#./}"
done )
printf '    %8s  total (uncompressed)\n' "$(du -sh "$dist" | cut -f1)"
