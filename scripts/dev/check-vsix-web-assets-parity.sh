#!/usr/bin/env bash
# Compare the shared web payload carried by every VSIX built in one invocation.
set -euo pipefail

if [ "$#" -lt 2 ]; then
  echo "usage: check-vsix-web-assets-parity.sh <vsix> <vsix> [...]" >&2
  exit 2
fi

work_dir="$(mktemp -d)"
trap 'rm -rf "$work_dir"' EXIT
reference="$work_dir/reference.sha256"
first=1

for archive in "$@"; do
  [ -f "$archive" ] || { echo "ERROR: missing VSIX for parity check: $archive" >&2; exit 1; }
  unpacked="$work_dir/$(basename "$archive").d"
  unzip -qq "$archive" -d "$unpacked"
  payload="$unpacked/extension"
  digest="$unpacked/payload.sha256"
  {
    cd "$payload"
    {
      find spec-studio -type f -print
      printf '%s\n' \
        out/extension.browser.js \
        dist/web/worker.js \
        dist/web/tcl_lsp_server_wasm.js \
        dist/web/tcl_lsp_server_wasm_bg.wasm
      find dist/web/specs -type f -print
    } | LC_ALL=C sort | while IFS= read -r file; do
      test -f "$file" || { echo "ERROR: $archive misses shared asset $file" >&2; exit 1; }
      shasum -a 256 "$file"
    done
  } >"$digest"
  if [ "$first" = 1 ]; then
    cp "$digest" "$reference"
    first=0
  elif ! cmp -s "$reference" "$digest"; then
    echo "ERROR: shared web payload differs in $archive" >&2
    exit 1
  fi
done

echo "==> Shared VSIX web payload parity: passed ($# archives)"
