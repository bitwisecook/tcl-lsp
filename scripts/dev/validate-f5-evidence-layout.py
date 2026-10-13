#!/usr/bin/env python3
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Compare compact F5 views with their retained original observations."""

import collections
import gzip
import importlib.util
import json
from pathlib import Path

from oracle_store import catalogue, retained_file

ROOT = Path(__file__).resolve().parents[2]
HERE = ROOT / "experiments/oracles/f5"


def module(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    result = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(result)
    return result


proofs = module("proofs", ROOT / "scripts/dev/check-name-resolution-proofs.py")
views = module("views", ROOT / "scripts/dev/rebuild-f5-evidence.py")


def load(path):
    return json.loads(retained_file(ROOT, path).read_bytes())


def main():
    files = catalogue(ROOT)["files"]
    manifest = load("docs/design/analysis/name-resolution-proofs/manifest.json")
    descriptors = 0
    documents = 0
    for proof in manifest["proofs"]:
        for item in proof["evidence"]:
            if item["path"] not in files:
                continue
            assert files[item["path"]]["sha256"] == item["sha256"], item["path"]
            if "json_pointer" in item:
                proofs.pointer(
                    proofs.selected_json(ROOT, item["path"], item["sha256"]),
                    item["json_pointer"],
                )
            descriptors += 1
        if not any(
            p["id"] == "bigip" and p["status"] != "not-tested"
            for p in proof["providers"]
        ):
            continue
        text = proofs.normalized((ROOT / proof["document"]).read_text())
        for field in ["question", "conclusion", "scope", "problem_statement"]:
            assert proofs.normalized(proof[field]) in text, proof["id"]
        for provider in proof["providers"]:
            assert proofs.normalized(provider["answer"]) in text, proof["id"]
        documents += 1
    observations = 0
    for run in sorted((HERE / "runs").glob("*/run.json")):
        metadata = json.loads(run.read_text())
        original = load(metadata["historical_records"])
        shared = json.loads(
            gzip.decompress((run.parent / "shared.json.gz").read_bytes())
        )
        rows = [
            json.loads(line)
            for line in gzip.decompress(
                (run.parent / "observations.jsonl.gz").read_bytes()
            ).splitlines()
        ]
        assert len(rows) == len(original) == metadata["observations"], run
        for selected, actual in zip(rows, original, strict=True):
            for key, value in selected.items():
                if key == "historical_record":
                    continue
                if key.endswith("_ref"):
                    assert shared[value] == actual[key[:-4]], (run, key)
                elif key == "other_stages" and isinstance(actual[key], dict):
                    assert value == {
                        k: views.stage(v) for k, v in actual[key].items()
                    }, (run, key)
                elif key in ["setup", "lookup", "load"]:
                    assert value == views.stage(actual[key]), (run, key)
                else:
                    assert value == actual[key], (run, key)
            observations += 1
    bundle = load("experiments/name-handling/patterns/f5/questions.json")
    contexts = 0
    coverage = collections.Counter()
    for question in bundle["questions"]:
        selected = json.loads(
            (HERE / "questions" / (question["id"] + ".json")).read_text()
        )
        assert selected["question"] == question["question"], question["id"]
        for context, original in question["contexts"].items():
            row = selected["contexts"][context]
            for key, value in original.items():
                assert row[key] == value, (question["id"], context, key)
            assert row["answer_coverage"] == views.coverage(original), question["id"]
            coverage[row["answer_coverage"]] += 1
            contexts += 1
    print(
        json.dumps(
            {
                "retained_proof_descriptors": descriptors,
                "canonical_f5_documents": documents,
                "run_observations": observations,
                "requested_question_contexts": contexts,
                "coverage": dict(coverage),
                "measured_appliance_execution": False,
            },
            indent=2,
        )
    )


if __name__ == "__main__":
    main()
