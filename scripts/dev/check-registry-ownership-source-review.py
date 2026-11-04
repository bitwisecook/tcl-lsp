#!/usr/bin/env python3
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Authenticate one registry source delta for two authored proof bindings.

Original bytes remain in memory. This source comparison does not execute Rust
assertions, renew native evidence, or replace the mandatory global proof gate.
"""

import argparse
import copy
import hashlib
import importlib.util
import json
import subprocess
import sys
from pathlib import Path

PROOF_DIRECTORY = "docs/design/analysis/name-resolution-proofs"
SOURCE = "rust/tcl-registry/src/model/namespace_ownership.rs"
ORIGINAL = "ac64cdc140d737a2cdf2639f64305ecccfe9eb0a205f7bdf7969604203ec3da3"
CURRENT = "b193bdb23698cb5408c2b0f5a45fc8d43b2bb0f1ccd8ac59cf91b0a671eed8a5"
COMMIT = "c90d3e35cbd21adf31739be003bf3c29239d2e6b"
BLOB = "c8f9881271306b8dcdd48e07bfb4a382a2f0f3a4"
HELPER = "scripts/dev/check-authored-fixture-source-review.py"
HELPER_SHA256 = "4f553f7a5ec3d731bf9cc1db7a64c797c4a4f570d8ed5929673bc31b5484df4f"
BINDINGS = {
    (
        "naming.analysis.constructed-key-consumer-projections",
        "receiver-selected-source598-745f40e488371f68",
    ),
    (
        "naming.core.original-package-source-action-context",
        "receiver-selected-source598-7c61bded9baa6d7a",
    ),
}


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def claim_pin(record, evidence_id):
    reviewed = copy.deepcopy(record)
    matching = [item for item in reviewed["evidence"] if item["id"] == evidence_id]
    if len(matching) != 1:
        raise ValueError("reviewed evidence ID is absent or ambiguous")
    matching[0]["sha256"] = ORIGINAL
    reviewed["evidence"] = matching
    return sha(json.dumps(reviewed, sort_keys=True, ensure_ascii=False).encode())


def compare(old, new):
    if (
        sha(old) != ORIGINAL
        or len(old) != 9310
        or sha(new) != CURRENT
        or len(new) != 9314
    ):
        raise ValueError("reviewed whole-source snapshots changed")
    prefix = b"    fn namespace_package_consensus_keeps_unknown_nested_case_and_literal_colons() {\n"
    marker = b"        let specs = [\n"
    if old.count(prefix) != 1:
        raise ValueError("reviewed test declaration is absent or ambiguous")
    before, body = old.split(prefix)
    if body.count(marker) != 1:
        raise ValueError("reviewed test fixture declaration is absent or ambiguous")
    expected = before + prefix + body.replace(marker, b"        let specs = vec![\n", 1)
    if new != expected:
        raise ValueError("source delta exceeds the exact test collection change")


def validate(root, contract_path, fetch=False):
    root = root.resolve()
    contract_raw = contract_path.read_bytes()
    contract = json.loads(contract_raw)
    if (
        contract["schema_version"] != 1
        or contract["kind"] != "registry-ownership-authored-source-snapshot"
        or contract["source"] != SOURCE
        or contract["original_commit"] != COMMIT
        or contract["original_blob"] != BLOB
        or contract["original_sha256"] != ORIGINAL
        or contract["current_sha256"] != CURRENT
        or contract["original_reader_sha256"] != HELPER_SHA256
    ):
        raise ValueError("reviewed source identity contract differs")
    reviews = {
        (row["proof_id"], row["evidence_id"]): row for row in contract["bindings"]
    }
    if len(contract["bindings"]) != 2 or set(reviews) != BINDINGS:
        raise ValueError("reviewed two-binding roster differs")
    helper_path = root / HELPER
    helper_raw = helper_path.read_bytes()
    if sha(helper_raw) != HELPER_SHA256:
        raise ValueError("authenticated original reader changed")
    spec = importlib.util.spec_from_file_location("authored_source_reader", helper_path)
    helper = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(helper)
    old, availability = helper.original(
        root,
        {"path": SOURCE, "original_commit": COMMIT, "original_blob": BLOB},
        fetch,
    )
    if (
        hashlib.sha1(b"blob " + str(len(old)).encode() + b"\0" + old).hexdigest()
        != BLOB
    ):
        raise ValueError("original Git blob identity differs")
    source_path = root / SOURCE
    new = source_path.read_bytes()
    compare(old, new)
    manifest_path = root / PROOF_DIRECTORY / "manifest.json"
    census_path = root / PROOF_DIRECTORY / "corpus-inventory.json"
    manifest_raw, census_raw = manifest_path.read_bytes(), census_path.read_bytes()
    manifest = json.loads(manifest_raw)
    observed = set()
    for record in manifest["proofs"]:
        for item in record["evidence"]:
            if item["path"] != SOURCE:
                continue
            pair = (record["id"], item["id"])
            if (
                pair not in reviews
                or pair in observed
                or record["kind"] != "implementation-contract"
                or item["role"] != "implementation"
                or item["sha256"] not in {ORIGINAL, CURRENT}
                or item.keys() & {"contains", "lines", "json_pointer"}
                or claim_pin(record, item["id"]) != reviews[pair]["claim_sha256"]
            ):
                raise ValueError("exact reviewed authored proof binding differs")
            observed.add(pair)
    if observed != BINDINGS:
        raise ValueError("reviewed proof binding is absent")
    if (
        source_path.read_bytes() != new
        or helper_path.read_bytes() != helper_raw
        or contract_path.read_bytes() != contract_raw
        or manifest_path.read_bytes() != manifest_raw
        or census_path.read_bytes() != census_raw
    ):
        raise ValueError("source review input changed during inspection")
    return {
        "schema_version": 1,
        "scope": "Authenticated exact registry ownership source snapshot for two authored implementation bindings. Only the fixed cfg(test) fixture collection changes from array to Vec; all other source bytes match. No Rust/native execution, provider answer, original capture renewal or global gate acceptance.",
        "verifier_sha256": sha(Path(__file__).read_bytes()),
        "contract_sha256": sha(contract_raw),
        "original_reader_sha256": sha(helper_raw),
        "input_manifest_sha256": sha(manifest_raw),
        "input_census_sha256": sha(census_raw),
        "source": SOURCE,
        "original_commit": COMMIT,
        "original_blob": BLOB,
        "original_sha256": sha(old),
        "current_sha256": sha(new),
        "original_availability": availability,
        "exact_reviewed_test_collection_delta": True,
        "all_other_source_bytes_identical": True,
        "bindings": contract["bindings"],
        "native_or_rust_execution": "not run",
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--root", type=Path, default=Path(__file__).resolve().parents[2]
    )
    parser.add_argument("--contract", type=Path, required=True)
    parser.add_argument("--fetch-origin", action="store_true")
    parser.add_argument("--receipt", type=Path)
    args = parser.parse_args()
    try:
        result = validate(args.root, args.contract, args.fetch_origin)
        if args.receipt:
            args.receipt.write_text(json.dumps(result, indent=2) + "\n")
    except (
        ValueError,
        OSError,
        KeyError,
        IndexError,
        TypeError,
        subprocess.TimeoutExpired,
    ):
        print(
            "Registry ownership source review failed; required originals, pins or exact reviewed transformation are unavailable.",
            file=sys.stderr,
        )
        return 1
    print(
        json.dumps(
            {
                "source_files": 1,
                "authored_bindings": 2,
                "native_or_rust_execution": "not run",
            },
            sort_keys=True,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
