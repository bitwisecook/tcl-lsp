#!/usr/bin/env bash
# Build the BIG-IP report-generator WASM and assemble a single, self-contained
# hosting page (`dist/index.html`) that runs the whole pipeline in the browser —
# upload a UCS/SCF, get a standalone HTML report, entirely client-side.
#
# Requires: the rustup wasm32-unknown-unknown target, wasm-bindgen-cli (matching
# the wasm-bindgen crate version pinned in Cargo.toml), wasm-opt (binaryen), and
# python3 (for the byte-safe asset injection).
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

echo "==> wasm-opt -Os"
wasm-opt -Os "$out/bigip_report_wasm_bg.wasm" -o "$out/opt.wasm"

echo "==> assembling single-file dist/index.html"
python3 - "$here/www/index.html" "$out/bigip_report_wasm.js" "$out/opt.wasm" "$dist/index.html" <<'PY'
import base64, sys
tmpl_path, glue_path, wasm_path, out_path = sys.argv[1:5]
tmpl = open(tmpl_path, encoding="utf-8").read()
glue = open(glue_path, encoding="utf-8").read()
b64 = base64.b64encode(open(wasm_path, "rb").read()).decode("ascii")
# Inject the wasm-bindgen glue and the base64 wasm. Use replace-with-callable so
# a stray backslash / ampersand in the payload is never treated as a regex.
tmpl = tmpl.replace("//__WASM_BINDGEN_GLUE__", glue)
tmpl = tmpl.replace("__WASM_B64__", b64)
if "__WASM_B64__" in tmpl or "//__WASM_BINDGEN_GLUE__" in tmpl:
    raise SystemExit("placeholder substitution failed")
open(out_path, "w", encoding="utf-8").write(tmpl)
print(f"    wrote {out_path} ({len(tmpl)/1024/1024:.2f} MiB, wasm {len(b64)/1024/1024:.2f} MiB b64)")
PY

echo "==> done:"
ls -lh "$dist/index.html"
