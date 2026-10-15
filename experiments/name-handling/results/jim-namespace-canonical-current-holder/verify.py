#!/usr/bin/env python3
"""Verify the captured original inputs, raw channels and promoted fixture."""

import argparse
import hashlib
import json
from pathlib import Path


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--repository-root", type=Path, required=True)
    parser.add_argument("--current-provider", type=Path)
    args = parser.parse_args()
    capture = Path(__file__).resolve().parent
    manifest = json.loads((capture / "manifest.json").read_text())
    upstream = json.loads((capture / "upstream.json").read_text())
    provider = json.loads((capture / "provider-verification.json").read_text())
    assert manifest["provider_revision"] == upstream["sha"] == provider["local_revision"]
    assert provider["tracked_source_status"] == ""
    assert manifest["provider_drift"] == {}
    assert manifest["provider_before_sha256"] == manifest["provider_after_sha256"]
    for name, key in [("probe.c", "probe_sha256"), ("capture.py", "capture_sha256"),
                      ("upstream.json", "upstream_receipt_sha256")]:
        assert digest(capture / name) == manifest[key], name
    for section, label in [("compiler", "compiler"), ("version", "version"),
                           ("compile", "compile"), ("probe", "probe"),
                           ("original_namespace_suite", "namespace-suite")]:
        record = manifest[section]
        assert record["exit"] == 0, section
        for channel in ["stdout", "stderr"]:
            assert digest(capture / f"{label}.{channel}") == record[f"{channel}_sha256"]
    original = capture / "probe.stdout"
    fixture = args.repository_root / "rust/tcl-vm/tests/data/native_jim_namespace_canonical/current-holder-20261015.tsv"
    assert original.read_bytes() == fixture.read_bytes()
    rows = original.read_text().splitlines()
    assert len(rows) == 80
    assert {line.split("|")[1] for line in rows} == {
        f"{case}_{phase}" for case in ["CURRENT", "ABSOLUTE", "RELATIVE", "EXPLICIT"]
        for phase in ["COLD", "REUSE"]}
    if args.current_provider:
        for name, expected in manifest["provider_after_sha256"].items():
            assert digest(args.current_provider / name) == expected, name
    print("Verified original capture, raw channels and exact promoted fixture")


if __name__ == "__main__":
    main()
