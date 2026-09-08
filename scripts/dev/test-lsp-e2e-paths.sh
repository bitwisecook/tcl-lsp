#!/bin/sh
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# SPDX-License-Identifier: AGPL-3.0-or-later

# Contract tests for the fail-closed native lsp-e2e archive closure. This is
# separate from root Rust tests because the archive executes only
# tcl-lsp-server's test package, while it compiles that package's exact local
# dependency closure with all features.
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/../.." && pwd)
CLASSIFIER=$ROOT/scripts/dev/lsp-e2e-path.sh
PACKAGES=$ROOT/scripts/dev/lsp-e2e-package-paths.txt
INPUTS=$ROOT/scripts/dev/lsp-e2e-input-paths.txt
WORKFLOW=$ROOT/.github/workflows/ci.yml

fail() { echo "test-lsp-e2e-paths: $*" >&2; exit 1; }
[ -x "$CLASSIFIER" ] || fail "classifier is not executable"
[ -r "$PACKAGES" ] || fail "package manifest is missing"
[ -r "$INPUTS" ] || fail "input manifest is missing"

expect_relevant() {
    "$CLASSIFIER" "$1" || fail "expected relevant path: $1"
}
expect_unrelated() {
    if "$CLASSIFIER" "$1"; then
        fail "expected unrelated path: $1"
    else
        status=$?
        [ "$status" -eq 1 ] || fail "unrelated path returned $status: $1"
    fi
}
expect_invalid() {
    if "$CLASSIFIER" "$1" 2>/dev/null; then
        fail "invalid path unexpectedly passed: $1"
    else
        status=$?
        [ "$status" -eq 2 ] || fail "invalid path returned $status: $1"
    fi
}

expect_relevant rust/tcl-lsp-server/src/lib.rs
expect_relevant rust/tcl-lsp-server/tests/e2e/completion.rs
expect_relevant rust/tcl-compiler/src/lib.rs
expect_relevant rust/tcl-registry/src/spec.rs
expect_relevant rust/tcl-vm/src/lib.rs
expect_relevant Cargo.toml
expect_relevant Cargo.lock
expect_relevant .cargo/config.toml
expect_relevant .config/nextest.toml
expect_relevant rust-toolchain.toml
expect_relevant .github/workflows/ci.yml
expect_relevant scripts/dev/changed-paths.sh
expect_relevant scripts/dev/lsp-e2e-path.sh
expect_relevant scripts/dev/lsp-e2e-input-paths.txt
expect_relevant scripts/dev/lsp-e2e-package-paths.txt
expect_relevant scripts/dev/test-lsp-e2e-paths.sh
expect_relevant scripts/dev/test-lsp-e2e-partitions.sh
expect_relevant editors/vscode/testFixture/variableContexts.tcl
expect_relevant samples/sslictcl/example.sslictcl
expect_relevant specs/sdc_base.tclspec
expect_unrelated README.md
expect_unrelated docs/design/compiler/wasm-native-lowering-plan.md
expect_unrelated editors/vscode/src/test/runTest.ts
expect_unrelated rust/tcl-lsp-server-wasm/src/lib.rs
expect_unrelated rust/tcl-irule-test/src/lib.rs
expect_unrelated runtime/rust/src/lib.rs
expect_invalid ""

# A missing or malformed committed closure must never classify a path as
# unrelated; CI converts status 2 into archive and partition work.
tmp=${TMPDIR:-/tmp}/tcl-lsp-e2e-paths.$$
trap 'rm -rf "$tmp"' EXIT HUP INT TERM
mkdir -p "$tmp"
if TCL_LSP_E2E_INPUT_MANIFEST="$tmp/missing" "$CLASSIFIER" README.md 2>/dev/null; then
    fail "missing input manifest unexpectedly passed"
else
    status=$?
    [ "$status" -eq 2 ] || fail "missing input manifest returned $status"
fi
if TCL_LSP_E2E_PACKAGE_MANIFEST="$tmp/missing" "$CLASSIFIER" README.md 2>/dev/null; then
    fail "missing package manifest unexpectedly passed"
else
    status=$?
    [ "$status" -eq 2 ] || fail "missing package manifest returned $status"
fi
printf '%s\n' 'rust/' > "$tmp/bad"
if TCL_LSP_E2E_INPUT_MANIFEST="$tmp/bad" "$CLASSIFIER" README.md 2>/dev/null; then
    fail "malformed manifest unexpectedly passed"
else
    status=$?
    [ "$status" -eq 2 ] || fail "malformed manifest returned $status"
fi
for malformed in . .. rust/./src rust/../src 'rust\\src' rust//src; do
    printf '%s\n' "$malformed" > "$tmp/bad"
    if TCL_LSP_E2E_INPUT_MANIFEST="$tmp/bad" "$CLASSIFIER" README.md 2>/dev/null; then
        fail "malformed manifest unexpectedly passed: $malformed"
    else
        status=$?
        [ "$status" -eq 2 ] || fail "malformed manifest returned $status: $malformed"
    fi
done

# Re-derive the local dependency closure from locked metadata with all
# features. `nextest archive` builds this exact package configuration.
meta="$tmp/metadata.json"
cargo metadata --locked --all-features --format-version 1 > "$meta" || fail "cargo metadata --locked --all-features failed"
python3 - "$ROOT" "$PACKAGES" "$meta" <<'PY'
import json
import sys
from pathlib import Path

root = Path(sys.argv[1]).resolve()
manifest = Path(sys.argv[2])
metadata = json.loads(Path(sys.argv[3]).read_text(encoding="utf-8"))
packages = {package["id"]: package for package in metadata["packages"]}
nodes = {node["id"]: node for node in metadata["resolve"]["nodes"]}
start = next(
    package_id
    for package_id, package in packages.items()
    if package["name"] == "tcl-lsp-server"
    and package["manifest_path"].endswith("rust/tcl-lsp-server/Cargo.toml")
)
pending = [start]
seen = {start}
while pending:
    current = pending.pop()
    for dependency in nodes[current]["deps"]:
        dependency_id = dependency["pkg"]
        if packages[dependency_id].get("source") is None and dependency_id not in seen:
            seen.add(dependency_id)
            pending.append(dependency_id)

actual = {
    Path(packages[package_id]["manifest_path"]).resolve().parent.relative_to(root).as_posix()
    for package_id in seen
}
expected = {
    line.strip()
    for line in manifest.read_text(encoding="utf-8").splitlines()
    if line.strip() and not line.strip().startswith("#")
}
if actual != expected:
    print("lsp-e2e cargo metadata closure drift", file=sys.stderr)
    print("missing:", sorted(actual - expected), file=sys.stderr)
    print("extra:", sorted(expected - actual), file=sys.stderr)
    raise SystemExit(1)
print(f"lsp-e2e cargo metadata closure contract passed ({len(actual)} local packages)")
PY

# The channel evaluates the base-commit classifier for PRs. A malformed
# trusted copy and every classifier error force lsp_e2e_changed=true.
grep -Fq 'lsp_e2e_changed: ${{ steps.paths.outputs.lsp_e2e_changed }}' "$WORKFLOW" || fail "channel output is missing lsp_e2e_changed"
grep -Fq 'scripts/dev/lsp-e2e-path.sh' "$WORKFLOW" || fail "PR classification does not load the lsp-e2e classifier"
grep -Fq 'TCL_LSP_E2E_PACKAGE_MANIFEST="$trusted/lsp-e2e-package-paths.txt"' "$WORKFLOW" || fail "trusted package closure is not wired into PR classification"
grep -Fq 'TCL_LSP_E2E_INPUT_MANIFEST="$trusted/lsp-e2e-input-paths.txt"' "$WORKFLOW" || fail "trusted input closure is not wired into PR classification"
grep -Fq '"$lsp_e2e_path" "$f"' "$WORKFLOW" || fail "channel does not invoke the lsp-e2e path classifier"
grep -Fq 'lsp_e2e_changed=true' "$WORKFLOW" || fail "channel has no fail-closed lsp-e2e fallback"

echo "lsp-e2e path closure contract passed"
