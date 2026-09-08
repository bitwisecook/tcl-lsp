#!/usr/bin/env bash
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# SPDX-License-Identifier: AGPL-3.0-or-later

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

make_database="$($make_cmd -f "$repo_root/Makefile" -qp 2>/dev/null || :)"
rule_for() {
  awk -v target="$1" '$1 == target ":" { print; exit }' <<<"$make_database"
}
require_prerequisite() {
  target="$1"
  prerequisite="$2"
  rule="$(rule_for "$target")"
  case " $rule " in
    *" $prerequisite "*) ;;
    *)
      echo "ERROR: $target must depend on $prerequisite" >&2
      echo "found: $rule" >&2
      exit 1
      ;;
  esac
}
reject_prerequisite() {
  target="$1"
  prerequisite="$2"
  rule="$(rule_for "$target")"
  case " $rule " in
    *" $prerequisite "*)
      echo "ERROR: $target must not depend on $prerequisite" >&2
      echo "found: $rule" >&2
      exit 1
      ;;
  esac
}

require_prerequisite package-vsix-targets package-vsix-all
reject_prerequisite package-vsix-targets package-vsix
require_prerequisite package-vsix-all compile
require_prerequisite package-vsix-all spec-studio-wasm
require_prerequisite build-editors build-editor-vsix-targets
reject_prerequisite build-editors build-editor-vsix
for target in publish-vsix publish-vsix-targets publish-openvsx publish-openvsx-targets; do
  case "$target" in
    *-targets) require_prerequisite "$target" package-vsix-targets ;;
    *) require_prerequisite "$target" package-vsix-all ;;
  esac
done

# Recursive Make invocations are opaque to the parent dependency graph. The
# public all-variants target must expose the shared builders, then suppress
# only their recursive duplicates. A dry run catches either build returning.
dry_run="$($make_cmd -f "$repo_root/Makefile" -n package-vsix-all 2>/dev/null)"
for marker in \
  "$repo_root/rust/tcl-lsp-server-wasm/build-wasm.sh" \
  "$repo_root/rust/tcl-spec-studio-wasm/build-wasm.sh" \
  'Building the spec studio front-end'; do
  count="$(grep -F -c "$marker" <<<"$dry_run" || :)"
  if [ "$count" -ne 1 ]; then
    echo "ERROR: package-vsix-all dry run contains $count copies of: $marker" >&2
    exit 1
  fi
done

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
