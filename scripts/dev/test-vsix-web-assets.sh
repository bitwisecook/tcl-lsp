#!/usr/bin/env bash
# Verify that target-specific VSIX packages cannot reuse assets after inputs move.
set -euo pipefail

repo_root="$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)"
fixture="$(mktemp -d)"
trap 'rm -rf "$fixture"' EXIT

mkdir -p \
  "$fixture/.cargo" \
  "$fixture/editors/vscode/src" \
  "$fixture/editors/vscode/out" \
  "$fixture/ai/claude/skills" \
  "$fixture/rust/tcl-dialect/src" \
  "$fixture/rust/tcl-lsp-server-wasm/dist" \
  "$fixture/rust/tcl-registry/src" \
  "$fixture/rust/tcl-spec-studio-wasm/dist" \
  "$fixture/rust/tcl-syntax/src" \
  "$fixture/rust/xtask/src" \
  "$fixture/specs" \
  "$fixture/scripts/dev"

ln -s "$repo_root/scripts/dev/check-vsix-web-assets-parity.sh" \
  "$fixture/scripts/dev/check-vsix-web-assets-parity.sh"

printf 'source\n' > "$fixture/rust/tcl-spec-studio-wasm/lib.rs"
printf 'studio\n' > "$fixture/rust/tcl-spec-studio-wasm/dist/index.html"
printf 'worker\n' > "$fixture/rust/tcl-lsp-server-wasm/dist/worker.js"
printf 'glue\n' > "$fixture/rust/tcl-lsp-server-wasm/dist/tcl_lsp_server_wasm.js"
printf 'wasm\n' > "$fixture/rust/tcl-lsp-server-wasm/dist/tcl_lsp_server_wasm_bg.wasm"
printf 'browser\n' > "$fixture/editors/vscode/out/extension.browser.js"
printf 'spec\n' > "$fixture/specs/fixture.tclspec"
touch "$fixture/Makefile" "$fixture/Cargo.toml" "$fixture/Cargo.lock" "$fixture/rust-toolchain.toml"

make_cmd="${MAKE:-make}"
common=(
  -f "$repo_root/Makefile"
  -o compile
  -o spec-studio-wasm
  -o "$fixture/editors/vscode/out/extension.js"
  "ROOT=$fixture/"
)

if "$make_cmd" "${common[@]}" _check-vsix-web-assets; then
  echo "ERROR: private VSIX web assets unexpectedly worked without state" >&2
  exit 1
fi

mkdir -p "$fixture/build/stamps/vsix-package.lock"
printf 'pid=999999\nhost=fixture\nstarted_utc=1970-01-01T00:00:00Z\n' \
  > "$fixture/build/stamps/vsix-package.lock/owner"
if "$make_cmd" -f "$repo_root/Makefile" "ROOT=$fixture/" MAKE=/bin/true package-vsix; then
  echo "ERROR: a recorded VSIX package lock unexpectedly allowed packaging" >&2
  exit 1
fi
rm -f "$fixture/build/stamps/vsix-package.lock/owner"
rmdir "$fixture/build/stamps/vsix-package.lock"

mkdir -p "$fixture/build/stamps"
state="$(mktemp -d "$fixture/build/stamps/vsix-web-assets.XXXXXX")"
token="$(mktemp "$state/token.XXXXXX")"
printf '%s\n' "$state" > "$token"
private=(__VSIX_WEB_ASSETS_STATE="$state" __VSIX_WEB_ASSETS_TOKEN="$token")
"$make_cmd" "${common[@]}" _prepare-vsix-web-assets "${private[@]}"
"$make_cmd" "${common[@]}" _check-vsix-web-assets "${private[@]}"

printf 'changed\n' >> "$fixture/rust/tcl-spec-studio-wasm/lib.rs"
if "$make_cmd" "${common[@]}" _check-vsix-web-assets "${private[@]}"; then
  echo "ERROR: stale VSIX web-asset source unexpectedly passed" >&2
  exit 1
fi

"$make_cmd" "${common[@]}" _prepare-vsix-web-assets "${private[@]}"
printf 'changed\n' >> "$fixture/rust/tcl-lsp-server-wasm/dist/worker.js"
if "$make_cmd" "${common[@]}" _check-vsix-web-assets "${private[@]}"; then
  echo "ERROR: stale VSIX web-asset output unexpectedly passed" >&2
  exit 1
fi

"$make_cmd" "${common[@]}" _prepare-vsix-web-assets "${private[@]}"
printf 'changed\n' >> "$fixture/editors/vscode/out/extension.browser.js"
if "$make_cmd" "${common[@]}" _check-vsix-web-assets "${private[@]}"; then
  echo "ERROR: stale browser extension output unexpectedly passed" >&2
  exit 1
fi

mkdir -p "$fixture/parity"
for name in first second; do
  payload="$fixture/parity/$name/extension"
  mkdir -p "$payload/spec-studio" "$payload/out" "$payload/dist/web/specs"
  printf 'studio\n' > "$payload/spec-studio/index.html"
  printf 'browser\n' > "$payload/out/extension.browser.js"
  printf 'worker\n' > "$payload/dist/web/worker.js"
  printf 'glue\n' > "$payload/dist/web/tcl_lsp_server_wasm.js"
  printf 'wasm\n' > "$payload/dist/web/tcl_lsp_server_wasm_bg.wasm"
  printf '["fixture.tclspec"]\n' > "$payload/dist/web/specs/index.json"
  printf 'spec\n' > "$payload/dist/web/specs/fixture.tclspec"
  (cd "$fixture/parity/$name" && zip -qr "$fixture/parity/$name.vsix" extension)
done
"$make_cmd" -f "$repo_root/Makefile" "ROOT=$fixture/" \
  _check-vsix-web-assets-parity \
  __VSIX_ARCHIVES="$fixture/parity/first.vsix $fixture/parity/second.vsix"
printf 'changed browser\n' > "$fixture/parity/second/extension/out/extension.browser.js"
rm -f "$fixture/parity/second.vsix"
(cd "$fixture/parity/second" && zip -qr "$fixture/parity/second.vsix" extension)
if "$make_cmd" -f "$repo_root/Makefile" "ROOT=$fixture/" \
  _check-vsix-web-assets-parity \
  __VSIX_ARCHIVES="$fixture/parity/first.vsix $fixture/parity/second.vsix"; then
  echo "ERROR: mismatched browser payload unexpectedly passed parity" >&2
  exit 1
fi

echo "==> VSIX web-asset stale-source contract: passed"
