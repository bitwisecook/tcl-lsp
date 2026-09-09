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
VERIFIER=$SCRIPT_DIR/verify-nextest-binary-shards.py
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
            "targets": [{"name": "smoke", "kind": ["test"], "test": True}],
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

manifest = """# binary-aware shard manifest
@partitions\t2
@exclude\tskip
1\tcore\tlib\tcore\tcore
1\tcli::bin/runner\tbin\tcli\trunner
1\tintegration::smoke\ttest\tintegration\tsmoke
"""
root.joinpath("manifest.tsv").write_text(manifest, encoding="utf-8")
manifest_shard_two = manifest.replace("1\tintegration::smoke", "2\tintegration::smoke")
root.joinpath("manifest-shard-two.tsv").write_text(manifest_shard_two, encoding="utf-8")
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
    }
    if include_integration:
        suites["integration::smoke"] = suite(
            "integration::smoke", "integration", "test", "smoke", "integration_case"
        )
    return {"test-count": sum(len(s["testcases"]) for s in suites.values()), "rust-suites": suites}

one = listing(True)
two = listing(False)
root.joinpath("one.json").write_text(json.dumps(one), encoding="utf-8")
root.joinpath("two.json").write_text(json.dumps(two), encoding="utf-8")

missing = copy.deepcopy(one)
del missing["rust-suites"]["integration::smoke"]
root.joinpath("missing-suite.json").write_text(json.dumps(missing), encoding="utf-8")

missing_test = copy.deepcopy(one)
del missing_test["rust-suites"]["integration::smoke"]["testcases"]["integration_case"]
root.joinpath("missing-test.json").write_text(json.dumps(missing_test), encoding="utf-8")

ignored = copy.deepcopy(one)
ignored["rust-suites"]["integration::smoke"]["testcases"]["ignored_case"]["filter-match"]["status"] = "matches"
root.joinpath("ignored-selected.json").write_text(json.dumps(ignored), encoding="utf-8")

malformed = copy.deepcopy(one)
malformed["rust-suites"]["core"]["kind"] = "not-a-kind"
root.joinpath("malformed-metadata.json").write_text(json.dumps(malformed), encoding="utf-8")

bad_status = copy.deepcopy(one)
bad_status["rust-suites"]["core"]["testcases"]["unit_case"]["filter-match"]["status"] = "unexpected"
root.joinpath("malformed-status.json").write_text(json.dumps(bad_status), encoding="utf-8")

wrong_one = listing(False)
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

expect_failure() {
    if run "$@" >/dev/null 2>&1; then
        echo "expected verifier failure for $*" >&2
        exit 1
    fi
}

expect_failure "$tmp_dir/metadata.json" "$tmp_dir/manifest-omitted.tsv" "$tmp_dir/one.json" "$tmp_dir/two.json"
expect_failure "$tmp_dir/metadata.json" "$tmp_dir/manifest-duplicate.tsv" "$tmp_dir/one.json" "$tmp_dir/two.json"
expect_failure "$tmp_dir/metadata.json" "$tmp_dir/manifest-shard-two.tsv" "$tmp_dir/selected-wrong-shard.json" "$tmp_dir/two.json"
expect_failure "$tmp_dir/metadata.json" "$tmp_dir/manifest.tsv" "$tmp_dir/missing-suite.json" "$tmp_dir/two.json"
expect_failure "$tmp_dir/metadata.json" "$tmp_dir/manifest.tsv" "$tmp_dir/missing-test.json" "$tmp_dir/two.json"
expect_failure "$tmp_dir/metadata.json" "$tmp_dir/manifest.tsv" "$tmp_dir/ignored-selected.json" "$tmp_dir/two.json"
expect_failure "$tmp_dir/metadata.json" "$tmp_dir/manifest.tsv" "$tmp_dir/malformed-metadata.json" "$tmp_dir/two.json"
expect_failure "$tmp_dir/metadata.json" "$tmp_dir/manifest.tsv" "$tmp_dir/malformed-status.json" "$tmp_dir/two.json"
expect_failure "$tmp_dir/metadata.json" "$tmp_dir/manifest.tsv" "$tmp_dir/empty.json" "$tmp_dir/two.json"

echo "nextest binary shard verifier tests: ok (coverage, assignments, filters, metadata)"
