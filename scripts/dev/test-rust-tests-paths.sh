#!/bin/sh
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# SPDX-License-Identifier: AGPL-3.0-or-later

# Contract tests for the fail-closed root Rust test path closure and the
# changed-file API boundary used by ci.yml.
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/../.." && pwd)
CLASSIFIER=$ROOT/scripts/dev/rust-tests-path.sh
CHANGED_FILES=$ROOT/scripts/dev/changed-paths.sh
PACKAGES=$ROOT/scripts/dev/rust-tests-package-paths.txt
INPUTS=$ROOT/scripts/dev/rust-tests-input-paths.txt
WORKFLOW=$ROOT/.github/workflows/ci.yml

fail() { echo "test-rust-tests-paths: $*" >&2; exit 1; }
[ -x "$CLASSIFIER" ] || fail "classifier is not executable"
[ -x "$CHANGED_FILES" ] || fail "changed-file helper is not executable"
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

expect_relevant rust/tcl-compiler/src/lib.rs
expect_relevant rust/xtask/src/main.rs
expect_relevant Cargo.lock
expect_relevant Makefile
expect_relevant .github/dependabot.yml
expect_relevant .github/workflows/ci.yml
expect_relevant .claude/skills/fetch-tcl-source/fetch_tcl_source.sh
expect_relevant specs/sdc_base.tclspec
expect_relevant docs/generated/diagnostic_codes.md
expect_relevant editors/vscode/src/extension.ts
expect_relevant scripts/dev/already-green.sh
expect_relevant scripts/dev/select-rust-tests-runner.sh
expect_relevant scripts/dev/test-already-green.sh
expect_relevant scripts/dev/test-rust-tests-runner.sh
expect_relevant rust/bigip-report-gen/templates/report.html.j2
expect_relevant rust/tcl-vm-wasm/Cargo.toml
expect_relevant rust/tcl-vm-wasm/Cargo.lock
expect_relevant rust/tcl-vm-wasm/src/lib.rs
expect_relevant rust/tcl-vm-wasm/verify.mjs
expect_relevant editors/vscode/testFixture/variableContexts.tcl
expect_relevant tests/external/backend_constraints.tcl
expect_relevant docs/design/contracts/shared-utility-contracts-rust.md
expect_relevant docs/design/compiler/semantic-aot-optimisation.md
expect_relevant docs/design/compiler/wasm-codegen.md
expect_relevant rust/bigip-report-gen/python/deploy/github-pages.yml
expect_relevant rust/bigip-report-gen/python/deploy/report-pyz.yml
expect_relevant .github/workflows/github-pages.yml
expect_relevant .github/workflows/report-pyz.yml
expect_unrelated README.md
expect_unrelated docs/design/compiler/wasm-native-lowering-plan.md
expect_relevant rust/tcl-lsp-server/src/lib.rs
expect_unrelated runtime/rust/src/lib.rs
expect_unrelated grammars/tree-sitter-tcl/grammar.js
expect_unrelated editors/vscode/src/test/runTest.ts
expect_invalid ""

# A missing or malformed committed closure must never classify a path as
# unrelated; CI converts status 2 into a broad Rust-test run.
tmp=${TMPDIR:-/tmp}/tcl-lsp-rust-tests-paths.$$
trap 'rm -rf "$tmp"' EXIT HUP INT TERM
mkdir -p "$tmp"
if TCL_LSP_RUST_TESTS_INPUT_MANIFEST="$tmp/missing" "$CLASSIFIER" README.md 2>/dev/null; then
    fail "missing manifest unexpectedly passed"
else
    status=$?
    [ "$status" -eq 2 ] || fail "missing manifest returned $status"
fi
if TCL_LSP_RUST_TESTS_PACKAGE_MANIFEST="$tmp/missing" "$CLASSIFIER" README.md 2>/dev/null; then
    fail "missing package manifest unexpectedly passed"
else
    status=$?
    [ "$status" -eq 2 ] || fail "missing package manifest returned $status"
fi
printf '%s\n' 'rust/' > "$tmp/bad"
if TCL_LSP_RUST_TESTS_INPUT_MANIFEST="$tmp/bad" "$CLASSIFIER" README.md 2>/dev/null; then
    fail "malformed manifest unexpectedly passed"
else
    status=$?
    [ "$status" -eq 2 ] || fail "malformed manifest returned $status"
fi

# Re-derive the local package closure from locked cargo metadata so a new path
# dependency cannot silently fall outside the committed classifier manifest.
meta="$tmp/metadata.json"
cargo metadata --locked --format-version 1 > "$meta" || fail "cargo metadata --locked failed"
python3 - "$ROOT" "$PACKAGES" "$meta" <<'PY'
import json
import sys
from pathlib import Path

root = Path(sys.argv[1]).resolve()
manifest = Path(sys.argv[2])
metadata = json.loads(Path(sys.argv[3]).read_text(encoding="utf-8"))
packages = {package["id"]: package for package in metadata["packages"]}
nodes = {node["id"]: node for node in metadata["resolve"]["nodes"]}
pending = list(metadata["workspace_members"])
seen = set(pending)
while pending:
    current = pending.pop()
    for dependency in nodes[current]["deps"]:
        dependency_id = dependency["pkg"]
        if packages[dependency_id].get("source") is None and dependency_id not in seen:
            seen.add(dependency_id)
            pending.append(dependency_id)

actual = set()
for package_id in seen:
    directory = Path(packages[package_id]["manifest_path"]).resolve().parent
    actual.add(directory.relative_to(root).as_posix())
expected = {line.strip() for line in manifest.read_text(encoding="utf-8").splitlines()
            if line.strip() and not line.strip().startswith("#")}
if actual != expected:
    print("cargo metadata closure drift", file=sys.stderr)
    print("missing:", sorted(actual - expected), file=sys.stderr)
    print("extra:", sorted(expected - actual), file=sys.stderr)
    raise SystemExit(1)
print(f"cargo metadata closure contract passed ({len(actual)} local packages)")
PY

# Mock the GitHub CLI to exercise the changed-file API boundary without a
# network call. A missing base, tag/unsupported event, empty result, compare
# cap, or API error all return status 2 and therefore force a broad run.
cat > "$tmp/gh" <<'EOF'
#!/bin/sh
set -eu
case "${SCENARIO:-pr-negative}:$2" in
    api-fail:*) exit 1 ;;
    pr-negative:*) printf '%s\n' '[[{"filename":"README.md","status":"modified"}]]' ;;
    pr-positive:*) printf '%s\n' '[[{"filename":"rust/tcl-compiler/src/lib.rs","status":"modified"}]]' ;;
    pr-rename:*) printf '%s\n' '[[{"filename":"docs/moved.rs","previous_filename":"rust/tcl-compiler/tests/removed.rs","status":"renamed"}]]' ;;
    pr-empty:*) printf '%s\n' '[[]]' ;;
    pr-malformed-null:*) printf '%s\n' '[[{"filename":null,"status":"modified"}]]' ;;
    pr-malformed-empty:*) printf '%s\n' '[[{"filename":"","status":"modified"}]]' ;;
    pr-malformed-status:*) printf '%s\n' '[[{"filename":"README.md","status":"moved"}]]' ;;
    pr-malformed-previous:*) printf '%s\n' '[[{"filename":"docs/moved.rs","previous_filename":null,"status":"renamed"}]]' ;;
    pr-many:*) python3 -c 'import json; print(json.dumps([[{"filename": f"docs/file-{i}.md", "status": "modified"} for i in range(3000)]]))' ;;
    push-positive:*) printf '%s\n' '{"files":[{"filename":"rust/xtask/src/main.rs","status":"modified"}]}' ;;
    push-rename:*) printf '%s\n' '{"files":[{"filename":"docs/moved.rs","previous_filename":"rust/tcl-compiler/tests/removed.rs","status":"renamed"}]}' ;;
    push-malformed-null:*) printf '%s\n' '{"files":[{"filename":null,"status":"modified"}]}' ;;
    push-malformed-status:*) printf '%s\n' '{"files":[{"filename":"README.md","status":"moved"}]}' ;;
    push-malformed-previous:*) printf '%s\n' '{"files":[{"filename":"docs/moved.rs","previous_filename":null,"status":"renamed"}]}' ;;
    push-many:*) python3 -c 'import json; print(json.dumps({"files": [{"filename": f"docs/file-{i}.md", "status": "modified"} for i in range(300)]}))' ;;
    *) printf '%s\n' '[[{"filename":"README.md","status":"modified"}]]' ;;
esac
EOF
chmod +x "$tmp/gh"

expect_files() {
    scenario=$1
    expected=$2
    actual=$(SCENARIO=$scenario GH_BIN="$tmp/gh" PATH="$tmp:$PATH" \
        "$CHANGED_FILES" "$3" owner/repo 42 "$4" "$5" "$6")
    [ "$actual" = "$expected" ] || fail "$scenario returned unexpected files: $actual"
}
expect_files pr-negative README.md pull_request '' deadbeef refs/pull/42/merge
expect_files pr-positive rust/tcl-compiler/src/lib.rs pull_request '' deadbeef refs/pull/42/merge
expect_files push-positive rust/xtask/src/main.rs push 0123456789abcdef0123456789abcdef01234567 0123456789abcdef0123456789abcdef01234567 refs/heads/rust

# A rename emits both API paths, while the cap still counts one file object.
rename_paths=$(SCENARIO=pr-rename GH_BIN="$tmp/gh" PATH="$tmp:$PATH" \
    "$CHANGED_FILES" pull_request owner/repo 42 '' deadbeef refs/pull/42/merge)
expected_rename=$(printf '%s\n%s' docs/moved.rs rust/tcl-compiler/tests/removed.rs)
[ "$rename_paths" = "$expected_rename" ] || fail "rename paths were not preserved: $rename_paths"
rename_relevant=false
while IFS= read -r path; do
    if "$CLASSIFIER" "$path"; then
        rename_relevant=true
    fi
done <<EOF
$rename_paths
EOF
[ "$rename_relevant" = true ] || fail "relevant source rename did not trigger root tests"

push_rename_paths=$(SCENARIO=push-rename GH_BIN="$tmp/gh" PATH="$tmp:$PATH" \
    "$CHANGED_FILES" push owner/repo 42 0123456789abcdef0123456789abcdef01234567 0123456789abcdef0123456789abcdef01234567 refs/heads/rust)
[ "$push_rename_paths" = "$expected_rename" ] || fail "compare rename paths were not preserved: $push_rename_paths"

expect_status_2() {
    scenario=$1
    shift
    if SCENARIO=$scenario GH_BIN="$tmp/gh" PATH="$tmp:$PATH" \
        "$CHANGED_FILES" "$@"; then
        fail "$scenario unexpectedly produced a complete file list"
    else
        status=$?
        [ "$status" -eq 2 ] || fail "$scenario returned $status, expected 2"
    fi
}
expect_status_2 api-fail pull_request owner/repo 42 '' deadbeef refs/pull/42/merge
expect_status_2 pr-empty pull_request owner/repo 42 '' deadbeef refs/pull/42/merge
expect_status_2 pr-malformed-null pull_request owner/repo 42 '' deadbeef refs/pull/42/merge
expect_status_2 pr-malformed-empty pull_request owner/repo 42 '' deadbeef refs/pull/42/merge
expect_status_2 pr-malformed-status pull_request owner/repo 42 '' deadbeef refs/pull/42/merge
expect_status_2 pr-malformed-previous pull_request owner/repo 42 '' deadbeef refs/pull/42/merge
expect_status_2 pr-many pull_request owner/repo 42 '' deadbeef refs/pull/42/merge
expect_status_2 missing-base push owner/repo 42 '' 0123456789abcdef0123456789abcdef01234567 refs/heads/rust
expect_status_2 tag-push push owner/repo 42 0123456789abcdef0123456789abcdef01234567 0123456789abcdef0123456789abcdef01234567 refs/tags/v1
expect_status_2 wrong-ref push owner/repo 42 0123456789abcdef0123456789abcdef01234567 0123456789abcdef0123456789abcdef01234567 refs/heads/feature
expect_status_2 abbreviated-before push owner/repo 42 0123456789abcdef 0123456789abcdef0123456789abcdef01234567 refs/heads/rust
expect_status_2 abbreviated-sha push owner/repo 42 0123456789abcdef0123456789abcdef01234567 0123456789abcdef refs/heads/rust
expect_status_2 uppercase-before push owner/repo 42 0123456789ABCDEF0123456789abcdef01234567 0123456789abcdef0123456789abcdef01234567 refs/heads/rust
expect_status_2 uppercase-sha push owner/repo 42 0123456789abcdef0123456789abcdef01234567 0123456789ABCDEF0123456789abcdef01234567 refs/heads/rust
expect_status_2 zero-before push owner/repo 42 0000000000000000000000000000000000000000 0123456789abcdef0123456789abcdef01234567 refs/heads/rust
expect_status_2 zero-sha push owner/repo 42 0123456789abcdef0123456789abcdef01234567 0000000000000000000000000000000000000000 refs/heads/rust
expect_status_2 unsupported-event schedule owner/repo 42 '' deadbeef refs/heads/rust
expect_status_2 push-malformed-null push owner/repo 42 0123456789abcdef0123456789abcdef01234567 0123456789abcdef0123456789abcdef01234567 refs/heads/rust
expect_status_2 push-malformed-status push owner/repo 42 0123456789abcdef0123456789abcdef01234567 0123456789abcdef0123456789abcdef01234567 refs/heads/rust
expect_status_2 push-malformed-previous push owner/repo 42 0123456789abcdef0123456789abcdef01234567 0123456789abcdef0123456789abcdef01234567 refs/heads/rust
expect_status_2 push-many push owner/repo 42 0123456789abcdef0123456789abcdef01234567 0123456789abcdef0123456789abcdef01234567 refs/heads/rust

# Verify channel wiring remains fail-closed and step-level.
grep -Fq 'rust_tests_changed: ${{ steps.paths.outputs.rust_tests_changed }}' "$WORKFLOW" \
    || fail "channel output is missing rust_tests_changed"
grep -Fq 'scripts/dev/changed-paths.sh' "$WORKFLOW" \
    || fail "channel does not use the committed changed-file helper"
grep -Fq 'scripts/dev/rust-tests-path.sh "$f"' "$WORKFLOW" \
    || fail "channel does not invoke the committed path classifier"
grep -Fq 'rust_tests_changed=true' "$WORKFLOW" \
    || fail "channel has no fail-closed Rust-test fallback"
rust_job=$(awk '
    /^  rust-tests:/ { in_job = 1 }
    in_job && /^  [A-Za-z0-9_-]+:/ && $1 != "rust-tests:" { exit }
    in_job { print }
' "$WORKFLOW")
[ -n "$rust_job" ] || fail "ci.yml must define rust-tests"
printf '%s\n' "$rust_job" | grep -Fq "if: needs.channel.outputs.rust_tests_changed == 'true'" \
    || fail "rust-tests expensive steps are not path-gated"
case "$rust_job" in
    *'if: always() && needs.channel.outputs.rust_tests_changed == '\''true'\'''*'sccache --show-stats'*) ;;
    *) fail "sccache statistics must be skipped when Rust setup is skipped" ;;
esac

echo "root Rust test path and changed-file contracts passed"
