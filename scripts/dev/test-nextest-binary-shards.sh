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
expect_failure --metadata-only "$tmp_dir/metadata-no-common.json" "$tmp_dir/manifest.tsv"

(cd "$REPO_ROOT" && cargo metadata --no-deps --locked --format-version 1) > "$tmp_dir/workspace-metadata.json"
run --metadata-only --partition-count 5 \
    "$tmp_dir/workspace-metadata.json" "$SCRIPT_DIR/rust-test-binary-shards.tsv" >/dev/null

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

echo "nextest binary shard verifier tests: ok (coverage, assignments, filters, metadata)"
