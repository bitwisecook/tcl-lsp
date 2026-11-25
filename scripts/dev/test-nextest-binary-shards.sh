#!/bin/sh
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

set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
REPO_ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/../.." && pwd)
VERIFIER=$SCRIPT_DIR/verify-nextest-binary-shards.py
BINARY_SHARD=$SCRIPT_DIR/rust-test-binary-shard.sh
tmp_dir=$(mktemp -d "${TMPDIR:-/tmp}/nextest-binary-shards.XXXXXX")
trap 'rm -rf "$tmp_dir"' EXIT HUP INT TERM

python3 - "$tmp_dir" <<'PY'
import copy
import json
import pathlib
import sys

root = pathlib.Path(sys.argv[1])
metadata = {
    "packages": [
        {
            "id": "path+file:///core#0.1.0",
            "name": "core",
            "targets": [{"name": "core", "kind": ["lib"], "test": True}],
        },
        {
            "id": "path+file:///cli#0.1.0",
            "name": "cli",
            "targets": [{"name": "runner", "kind": ["bin"], "test": True}],
        },
        {
            "id": "path+file:///integration#0.1.0",
            "name": "integration",
            "targets": [
                {"name": "integration", "kind": ["lib"], "test": True},
                {"name": "smoke", "kind": ["test"], "test": True},
            ],
        },
        {
            "id": "path+file:///skip#0.1.0",
            "name": "skip",
            "targets": [{"name": "skip", "kind": ["lib"], "test": True}],
        },
    ],
    "workspace_members": [
        "path+file:///core#0.1.0",
        "path+file:///cli#0.1.0",
        "path+file:///integration#0.1.0",
        "path+file:///skip#0.1.0",
    ],
}
root.joinpath("metadata.json").write_text(json.dumps(metadata), encoding="utf-8")
metadata_without_common_harness = copy.deepcopy(metadata)
metadata_without_common_harness["packages"][2]["targets"] = [
    {"name": "smoke", "kind": ["test"], "test": True}
]
root.joinpath("metadata-no-common.json").write_text(
    json.dumps(metadata_without_common_harness), encoding="utf-8"
)

manifest = """# binary-aware shard manifest
@partitions\t2
@exclude\tskip
1\tcore\tlib\tcore\tcore
1\tcli::bin/runner\tbin\tcli\trunner
2\tintegration\tlib\tintegration\tintegration
2\tintegration::smoke\ttest\tintegration\tsmoke
"""
root.joinpath("manifest.tsv").write_text(manifest, encoding="utf-8")
manifest_all_one = manifest.replace("2\tintegration\tlib", "1\tintegration\tlib").replace(
    "2\tintegration::smoke", "1\tintegration::smoke"
)
root.joinpath("manifest-empty-shard.tsv").write_text(manifest_all_one, encoding="utf-8")
root.joinpath("manifest-omitted.tsv").write_text(
    manifest.replace("1\tcli::bin/runner\tbin\tcli\trunner\n", ""), encoding="utf-8"
)
root.joinpath("manifest-duplicate.tsv").write_text(
    manifest + "1\tcore\tlib\tcore\tcore\n", encoding="utf-8"
)

def testcase(ignored=False, status=None):
    if status is None:
        status = "mismatch" if ignored else "matches"
    return {"ignored": ignored, "filter-match": {"status": status}}

def suite(binary_id, package, kind, binary_name, test_name):
    return {
        "package-name": package,
        "binary-id": binary_id,
        "binary-name": binary_name,
        "kind": kind,
        "status": "listed",
        "testcases": {
            test_name: testcase(),
            "ignored_case": testcase(ignored=True),
        },
    }

def listing(include_integration):
    suites = {
        "core": suite("core", "core", "lib", "core", "unit_case"),
        "cli::bin/runner": suite("cli::bin/runner", "cli", "bin", "runner", "bin_case"),
        "integration": suite(
            "integration", "integration", "lib", "integration", "integration_unit_case"
        ),
    }
    if include_integration:
        suites["integration::smoke"] = suite(
            "integration::smoke", "integration", "test", "smoke", "integration_case"
        )
    return {"test-count": sum(len(s["testcases"]) for s in suites.values()), "rust-suites": suites}

def deselect_nonignored(document):
    for suite_data in document["rust-suites"].values():
        for test_data in suite_data["testcases"].values():
            if not test_data["ignored"]:
                test_data["filter-match"]["status"] = "mismatch"

one = listing(False)
deselect_nonignored(one)
one["rust-suites"]["core"]["testcases"]["unit_case"]["filter-match"]["status"] = "matches"
one["rust-suites"]["cli::bin/runner"]["testcases"]["bin_case"]["filter-match"]["status"] = "matches"
two = listing(True)
deselect_nonignored(two)
two["rust-suites"]["integration"]["testcases"]["integration_unit_case"]["filter-match"]["status"] = "matches"
two["rust-suites"]["integration::smoke"]["testcases"]["integration_case"]["filter-match"]["status"] = "matches"
root.joinpath("one.json").write_text(json.dumps(one), encoding="utf-8")
root.joinpath("two.json").write_text(json.dumps(two), encoding="utf-8")

missing = copy.deepcopy(two)
del missing["rust-suites"]["integration::smoke"]
root.joinpath("missing-suite.json").write_text(json.dumps(missing), encoding="utf-8")

missing_test = copy.deepcopy(two)
del missing_test["rust-suites"]["integration::smoke"]["testcases"]["integration_case"]
root.joinpath("missing-test.json").write_text(json.dumps(missing_test), encoding="utf-8")

ignored = copy.deepcopy(two)
ignored["rust-suites"]["integration::smoke"]["testcases"]["ignored_case"]["filter-match"]["status"] = "matches"
root.joinpath("ignored-selected.json").write_text(json.dumps(ignored), encoding="utf-8")

malformed = copy.deepcopy(one)
malformed["rust-suites"]["core"]["kind"] = "not-a-kind"
root.joinpath("malformed-metadata.json").write_text(json.dumps(malformed), encoding="utf-8")

bad_status = copy.deepcopy(one)
bad_status["rust-suites"]["core"]["testcases"]["unit_case"]["filter-match"]["status"] = "unexpected"
root.joinpath("malformed-status.json").write_text(json.dumps(bad_status), encoding="utf-8")

wrong_one = copy.deepcopy(one)
wrong_one["rust-suites"]["integration::smoke"] = suite(
    "integration::smoke", "integration", "test", "smoke", "integration_case"
)
root.joinpath("selected-wrong-shard.json").write_text(json.dumps(wrong_one), encoding="utf-8")
root.joinpath("empty.json").write_text(json.dumps({"test-count": 0, "rust-suites": {}}), encoding="utf-8")

# Light tier: the heavy-listed integration case is left out of shard two.
root.joinpath("heavy.tsv").write_text(
    "# heavy tests\nintegration::smoke\tintegration_case\ncore\tgone_case\n", encoding="utf-8"
)
light_two = copy.deepcopy(two)
light_two["rust-suites"]["integration::smoke"]["testcases"]["integration_case"]["filter-match"]["status"] = "mismatch"
root.joinpath("light-two.json").write_text(json.dumps(light_two), encoding="utf-8")
root.joinpath("heavy-whole.tsv").write_text("integration::smoke\t*\n", encoding="utf-8")
root.joinpath("heavy-malformed.tsv").write_text("integration::smoke\n", encoding="utf-8")
PY

run() {
    python3 "$VERIFIER" "$@"
}

run --partition-count 2 "$tmp_dir/metadata.json" "$tmp_dir/manifest.tsv" "$tmp_dir/one.json" "$tmp_dir/two.json" >/dev/null
run --metadata-only --partition-count 2 "$tmp_dir/metadata.json" "$tmp_dir/manifest.tsv" >/dev/null

expect_failure() {
    if run "$@" >/dev/null 2>&1; then
        echo "expected verifier failure for $*" >&2
        exit 1
    fi
}

expect_failure "$tmp_dir/metadata.json" "$tmp_dir/manifest-omitted.tsv" "$tmp_dir/one.json" "$tmp_dir/two.json"
expect_failure "$tmp_dir/metadata.json" "$tmp_dir/manifest-duplicate.tsv" "$tmp_dir/one.json" "$tmp_dir/two.json"
expect_failure "$tmp_dir/metadata.json" "$tmp_dir/manifest.tsv" "$tmp_dir/selected-wrong-shard.json" "$tmp_dir/two.json"
expect_failure "$tmp_dir/metadata.json" "$tmp_dir/manifest-empty-shard.tsv" "$tmp_dir/one.json" "$tmp_dir/two.json"
expect_failure "$tmp_dir/metadata.json" "$tmp_dir/manifest.tsv" "$tmp_dir/one.json" "$tmp_dir/missing-suite.json"
expect_failure "$tmp_dir/metadata.json" "$tmp_dir/manifest.tsv" "$tmp_dir/one.json" "$tmp_dir/missing-test.json"
expect_failure "$tmp_dir/metadata.json" "$tmp_dir/manifest.tsv" "$tmp_dir/one.json" "$tmp_dir/ignored-selected.json"
expect_failure "$tmp_dir/metadata.json" "$tmp_dir/manifest.tsv" "$tmp_dir/malformed-metadata.json" "$tmp_dir/two.json"
expect_failure "$tmp_dir/metadata.json" "$tmp_dir/manifest.tsv" "$tmp_dir/malformed-status.json" "$tmp_dir/two.json"
expect_failure "$tmp_dir/metadata.json" "$tmp_dir/manifest.tsv" "$tmp_dir/empty.json" "$tmp_dir/two.json"
expect_failure --metadata-only "$tmp_dir/metadata.json" "$tmp_dir/manifest.tsv" "$tmp_dir/one.json"

# Light tier: a heavy test must be left out, everything else stays exact-once,
# and a stale heavy row only warns.
light_out=$(run --heavy-list "$tmp_dir/heavy.tsv" "$tmp_dir/metadata.json" "$tmp_dir/manifest.tsv" "$tmp_dir/one.json" "$tmp_dir/light-two.json")
case "$light_out" in
    *'light: heavy-excluded=1'*'heavy test list entry matches no assigned test'*core*gone_case* | \
        *'heavy test list entry matches no assigned test'*core*gone_case*'light: heavy-excluded=1'*) ;;
    *)
        echo "light-tier proof must count the excluded heavy test and warn on stale rows: $light_out" >&2
        exit 1
        ;;
esac
run --heavy-list "$tmp_dir/heavy-whole.tsv" "$tmp_dir/metadata.json" "$tmp_dir/manifest.tsv" "$tmp_dir/one.json" "$tmp_dir/light-two.json" >/dev/null
expect_failure --heavy-list "$tmp_dir/heavy.tsv" "$tmp_dir/metadata.json" "$tmp_dir/manifest.tsv" "$tmp_dir/one.json" "$tmp_dir/two.json"
expect_failure "$tmp_dir/metadata.json" "$tmp_dir/manifest.tsv" "$tmp_dir/one.json" "$tmp_dir/light-two.json"
expect_failure --heavy-list "$tmp_dir/heavy-malformed.tsv" "$tmp_dir/metadata.json" "$tmp_dir/manifest.tsv" "$tmp_dir/one.json" "$tmp_dir/light-two.json"
expect_failure --metadata-only "$tmp_dir/metadata-no-common.json" "$tmp_dir/manifest.tsv"

(cd "$REPO_ROOT" && cargo metadata --no-deps --locked --format-version 1) > "$tmp_dir/workspace-metadata.json"
run --metadata-only --partition-count 5 \
    "$tmp_dir/workspace-metadata.json" "$SCRIPT_DIR/rust-test-binary-shards.tsv" >/dev/null

# Dedicated targets belong to one checked workflow, never an ordinary shard.
python3 - "$VERIFIER" "$tmp_dir" "$REPO_ROOT" <<'PY_DEDICATED'
import copy
import importlib.util
import json
import pathlib
import sys

spec = importlib.util.spec_from_file_location("shard_verifier", sys.argv[1])
verifier = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = verifier
spec.loader.exec_module(verifier)
root = pathlib.Path(sys.argv[2])
repository = pathlib.Path(sys.argv[3])
directive = "@dedicated\ttcl-lsp-core::conformance\ttest\ttcl-lsp-core\tconformance\t.github/workflows/command-definitions.yml\tscripts/dev/run-conformance.sh\tconformance\n"
original = root.joinpath("manifest.tsv").read_text()
runner_manifest = original.replace("@exclude\tskip\n", "@exclude\tskip\n" + directive)
root.joinpath("manifest-dedicated-runner.tsv").write_text(runner_manifest)
manifest = runner_manifest + "1\ttcl-lsp-core\tlib\ttcl-lsp-core\ttcl_lsp_core\n"
manifest_path = root / "manifest-dedicated.tsv"
manifest_path.write_text(manifest)
metadata = json.loads(root.joinpath("metadata.json").read_text())
metadata["packages"].append({
    "id": "path+file:///tcl-lsp-core#0.1.0",
    "name": "tcl-lsp-core",
    "targets": [
        {"name": "tcl_lsp_core", "kind": ["lib"], "test": True},
        {"name": "conformance", "kind": ["test"], "test": True},
    ],
})
metadata["workspace_members"].append("path+file:///tcl-lsp-core#0.1.0")
metadata_path = root / "metadata-dedicated.json"
metadata_path.write_text(json.dumps(metadata))
listings = []
for index, name in enumerate(["one.json", "two.json"], 1):
    document = json.loads(root.joinpath(name).read_text())
    suite = copy.deepcopy(document["rust-suites"]["core"])
    suite.update({"package-name": "tcl-lsp-core", "binary-id": "tcl-lsp-core", "binary-name": "tcl_lsp_core"})
    suite["testcases"]["unit_case"]["filter-match"]["status"] = "matches" if index == 1 else "mismatch"
    document["rust-suites"]["tcl-lsp-core"] = suite
    document["test-count"] += len(suite["testcases"])
    path = root / f"dedicated-{index}.json"
    path.write_text(json.dumps(document))
    listings.append(path)
verifier.verify(metadata_path, manifest_path, listings)

def fails(action, reason):
    try:
        action()
    except verifier.VerificationError as error:
        assert reason in str(error), str(error)
    else:
        raise AssertionError(f"expected failure containing {reason!r}")

# Ownership must be total and disjoint, and metadata remains authoritative.
for text, reason in [
    (manifest.replace(directive, ""), "missing (1)"),
    (manifest + "2\ttcl-lsp-core::conformance\ttest\ttcl-lsp-core\tconformance\n", "both shard and dedicated"),
    (manifest.replace(directive, directive + directive), "duplicate dedicated"),
    (manifest.replace("@dedicated", "@unknown"), "unknown directive"),
    (manifest.replace("\ttcl-lsp-core::conformance\ttest\ttcl-lsp-core\tconformance", "\ttcl-lsp-core::absent\ttest\ttcl-lsp-core\tabsent"), "declared cargo test target"),
]:
    bad = root / "bad-dedicated.tsv"
    bad.write_text(text)
    fails(lambda: verifier.verify(metadata_path, bad, listings), reason)

# Even an unselected dedicated suite cannot hide inside an ordinary listing.
document = json.loads(listings[1].read_text())
suite = copy.deepcopy(document["rust-suites"]["tcl-lsp-core"])
suite.update({"binary-id": "tcl-lsp-core::conformance", "binary-name": "conformance", "kind": "test"})
document["rust-suites"]["tcl-lsp-core::conformance"] = suite
document["test-count"] += len(suite["testcases"])
bad_listing = root / "dedicated-selected.json"
bad_listing.write_text(json.dumps(document))
fails(lambda: verifier.verify(metadata_path, manifest_path, [listings[0], bad_listing]), "dedicated binary appears")

# Verify each real invocation edge; comments and missing release dependencies fail.
owner_root = root / "owner-source"
paths = [".github/workflows/ci.yml", ".github/workflows/command-definitions.yml", "scripts/dev/run-conformance.sh", "rust/xtask/src/main.rs"]
for relative in paths:
    destination = owner_root / relative
    destination.parent.mkdir(parents=True, exist_ok=True)
    destination.write_text((repository / relative).read_text())
parsed = verifier._manifest(manifest_path)
verifier._verify_dedicated(parsed, owner_root)
mutations = [
    (paths[0], "uses: ./.github/workflows/command-definitions.yml", "uses: ./.github/workflows/absent.yml", "exactly one CI job"),
    (paths[0], "python, web-frontends, command-definitions]", "python, web-frontends]", "required by release"),
    (paths[1], "run: bash scripts/dev/run-conformance.sh", "# run: bash scripts/dev/run-conformance.sh", "does not invoke its script"),
    (paths[2], 'cargo xtask conformance "$@"', '# cargo xtask conformance "$@"', "does not invoke its xtask"),
    (paths[3], '            "conformance",', '            "absent",', "declared cargo test target"),
    (paths[3], 'Command::Conformance { update, harness } => conformance(update, &harness)', 'Command::Conformance { update, harness } => absent(update, &harness)', "not dispatched"),
]
for relative, before, after, reason in mutations:
    path = owner_root / relative
    original_text = path.read_text()
    assert before in original_text, before
    path.write_text(original_text.replace(before, after))
    fails(lambda: verifier._verify_dedicated(parsed, owner_root), reason)
    path.write_text(original_text)
# The called workflow must run the target unconditionally and propagate its
# failure, regardless of boolean/string spelling. The actual workflow already
# has a conditional upload step and a tag-guarded CI caller; both remain valid.
workflow_path = owner_root / paths[1]
workflow_text = workflow_path.read_text()
for key, value in [
    ("if", "false"), ("if", "'false'"), ("if", '"false"'),
    ("continue-on-error", "true"), ("continue-on-error", "'true'"),
    ("continue-on-error", '"true"'),
]:
    for before, after, reason in [
        (
            "  command-definition-oracles:\n",
            f"  command-definition-oracles:\n    {key}: {value}\n",
            "owning job must be unconditional and propagate failure",
        ),
        (
            "      - name: Check the probe conformance rows\n",
            f"      - name: Check the probe conformance rows\n        {key}: {value}\n",
            "owning step must be unconditional and propagate failure",
        ),
    ]:
        assert before in workflow_text, before
        workflow_path.write_text(workflow_text.replace(before, after))
        fails(lambda: verifier._verify_dedicated(parsed, owner_root), reason)
        workflow_path.write_text(workflow_text)
verifier._verify_dedicated(parsed, owner_root)
missing = owner_root / paths[2]
missing.unlink()
fails(lambda: verifier._verify_dedicated(parsed, owner_root), "missing or invalid owner path")
print("dedicated binary ownership tests: ok (coverage, disjointness, invocation chain, release dependency)")
PY_DEDICATED

mkdir "$tmp_dir/bin"
cat > "$tmp_dir/bin/cargo" <<'EOF'
#!/bin/sh
printf '%s\n' "$@" > "$CARGO_ARGS_FILE"
EOF
chmod +x "$tmp_dir/bin/cargo"
CARGO_ARGS_FILE="$tmp_dir/shard-one.args" \
    TCL_LSP_RUST_TEST_BINARY_SHARDS="$tmp_dir/manifest.tsv" \
    PATH="$tmp_dir/bin:$PATH" \
    "$BINARY_SHARD" list 1/2 --message-format json
CARGO_ARGS_FILE="$tmp_dir/shard-two.args" \
    TCL_LSP_RUST_TEST_BINARY_SHARDS="$tmp_dir/manifest.tsv" \
    PATH="$tmp_dir/bin:$PATH" \
    "$BINARY_SHARD" run 2/2 --no-fail-fast

for required in nextest --workspace --all-features --lib --bins; do
    grep -Fxq -- "$required" "$tmp_dir/shard-one.args"
    grep -Fxq -- "$required" "$tmp_dir/shard-two.args"
done
grep -Fxq -- list "$tmp_dir/shard-one.args"
grep -Fxq -- run "$tmp_dir/shard-two.args"
grep -Fxq -- skip "$tmp_dir/shard-one.args"
grep -Fxq -- smoke "$tmp_dir/shard-two.args"
grep -Fxq -- '(package(=core) & binary(=core)) | (package(=cli) & binary(=runner))' \
    "$tmp_dir/shard-one.args"
grep -Fxq -- '(package(=integration) & binary(=integration)) | (package(=integration) & binary(=smoke))' \
    "$tmp_dir/shard-two.args"
if grep -Fq -- '--partition' "$tmp_dir/shard-one.args" "$tmp_dir/shard-two.args"; then
    echo "binary-aware runner must not add a hash partition" >&2
    exit 1
fi
if grep -Fxq -- '--profile' "$tmp_dir/shard-one.args"; then
    echo "the heavy tier (the default) must not select the ci-light profile" >&2
    exit 1
fi

# Light tier: the same selection minus the heavy rows, under ci-light.
CARGO_ARGS_FILE="$tmp_dir/shard-two-light.args" \
    TCL_LSP_RUST_TEST_BINARY_SHARDS="$tmp_dir/manifest.tsv" \
    TCL_LSP_RUST_HEAVY_TESTS="$tmp_dir/heavy.tsv" \
    TCL_LSP_RUST_TEST_TIER=light \
    PATH="$tmp_dir/bin:$PATH" \
    "$BINARY_SHARD" run 2/2 --no-fail-fast
grep -Fxq -- '--profile' "$tmp_dir/shard-two-light.args"
grep -Fxq -- ci-light "$tmp_dir/shard-two-light.args"
grep -Fxq -- '((package(=integration) & binary(=integration)) | (package(=integration) & binary(=smoke))) & not ((binary_id(=integration::smoke) & (test(=integration_case))))' \
    "$tmp_dir/shard-two-light.args"
if CARGO_ARGS_FILE="$tmp_dir/unused.args" TCL_LSP_RUST_TEST_BINARY_SHARDS="$tmp_dir/manifest.tsv" \
    TCL_LSP_RUST_TEST_TIER=medium PATH="$tmp_dir/bin:$PATH" "$BINARY_SHARD" list 1/2 2>/dev/null; then
    echo "an unknown test tier must be refused" >&2
    exit 1
fi
if CARGO_ARGS_FILE="$tmp_dir/unused.args" TCL_LSP_RUST_TEST_BINARY_SHARDS="$tmp_dir/manifest.tsv" \
    TCL_LSP_RUST_HEAVY_TESTS="$tmp_dir/heavy-malformed.tsv" TCL_LSP_RUST_TEST_TIER=light \
    PATH="$tmp_dir/bin:$PATH" "$BINARY_SHARD" list 1/2 2>/dev/null; then
    echo "a malformed heavy row must be refused" >&2
    exit 1
fi

# Dedicated ownership stays outside ordinary selection in both tiers.
for tier in heavy light; do
    CARGO_ARGS_FILE="$tmp_dir/dedicated-$tier.args" \
        TCL_LSP_RUST_TEST_BINARY_SHARDS="$tmp_dir/manifest-dedicated-runner.tsv" \
        TCL_LSP_RUST_HEAVY_TESTS="$tmp_dir/heavy.tsv" TCL_LSP_RUST_TEST_TIER="$tier" \
        PATH="$tmp_dir/bin:$PATH" "$BINARY_SHARD" run 2/2
    if grep -Fq -- conformance "$tmp_dir/dedicated-$tier.args"; then
        echo "dedicated target must not be built or selected by ordinary $tier shards" >&2
        exit 1
    fi
done
if CARGO_ARGS_FILE="$tmp_dir/unused.args" \
    TCL_LSP_RUST_TEST_BINARY_SHARDS="$tmp_dir/bad-dedicated.tsv" \
    PATH="$tmp_dir/bin:$PATH" "$BINARY_SHARD" run 2/2 2>/dev/null; then
    echo "the runner must reject an unverified dedicated owner" >&2
    exit 1
fi

# Legitimate Rust raw-identifier segments are test identity data, not filters.
python3 - "$SCRIPT_DIR/nextest-terminal-capture.py" "$tmp_dir" <<'PY_CAPTURE'
import copy
import hashlib
import importlib.util
import json
import pathlib
import sys

spec = importlib.util.spec_from_file_location("nextest_capture", sys.argv[1])
capture = importlib.util.module_from_spec(spec)
spec.loader.exec_module(capture)
root = pathlib.Path(sys.argv[2])
# Exact selected names from the native Rust shard that capture previously refused.
names = [
    "model::r#gen::dispatch::header_quote_tests::quoted_identifier_with_space_is_not_truncated",
    "model::r#gen::dispatch::header_quote_tests::unquoted_identifier_still_parses",
]
suite = {
    "package-name": "tcl-bigip",
    "binary-id": "tcl-bigip",
    "binary-name": "tcl_bigip",
    "kind": "lib",
    "status": "listed",
    "testcases": {
        name: {"filter-match": {"status": "matches"}, "ignored": False}
        for name in names
    },
}
roster = {"rust-suites": {"tcl-bigip": suite}}
path = root / "raw-identifier-selected.json"
raw = json.dumps(roster).encode()
path.write_bytes(raw)
digest, _, selected, ignored = capture.roster(path)
assert digest == hashlib.sha256(raw).hexdigest()
assert set(selected) == {("tcl-bigip", name) for name in names}
assert not ignored
assert path.read_bytes() == raw
assert all(not capture.SAFE_NAME.fullmatch(name) for name in names)
assert capture.safe_testcase_name("plain::case")
assert capture.safe_testcase_name("r#async::r#type::case")
for name in [
    "model::gen#bad::case", "model::xr#gen::case", "model::r#gen#bad::case",
    "model::r#::case", "model::r#9gen::case", "model::r#gen-suffix::case",
    "model::r#_::case", "model::r#self::case", "model::r#super::case",
    "model::r#crate::case", "model::r#Self::case", "", None, 1,
]:
    assert not capture.safe_testcase_name(name), repr(name)
    invalid = copy.deepcopy(roster)
    if isinstance(name, str):
        invalid["rust-suites"]["tcl-bigip"]["testcases"] = {
            name: {"filter-match": {"status": "matches"}, "ignored": False}
        }
        path.write_text(json.dumps(invalid))
        try:
            capture.roster(path)
        except capture.CaptureError as error:
            assert str(error) == "unsafe testcase name"
        else:
            raise AssertionError(name)
for unsafe in ['"', "\\", "$", "@"] + [chr(code) for code in range(32)]:
    for name in ["plain::" + unsafe + "case", "model::r#gen::" + unsafe + "case"]:
        assert not capture.safe_testcase_name(name), repr(name)
# Existing names without # retain exactly the old acceptance predicate.
for code in range(128):
    character = chr(code)
    if character == "#":
        continue
    for name in [character + "case", "mod::" + character + "case", "case" + character]:
        assert capture.safe_testcase_name(name) == bool(capture.SAFE_NAME.fullmatch(name))
# Raw-identifier accommodation must not broaden any suite identity validation.
invalid = copy.deepcopy(roster)
invalid["rust-suites"]["tcl-bigip"]["package-name"] = "r#package"
path.write_text(json.dumps(invalid))
try:
    capture.roster(path)
except capture.CaptureError as error:
    assert str(error) == "unsafe suite identity"
else:
    raise AssertionError("suite validation changed")
PY_CAPTURE

echo "nextest binary shard verifier tests: ok (coverage, assignments, filters, metadata)"
