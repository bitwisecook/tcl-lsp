#!/usr/bin/env python3
"""Verify immutable evidence, extracted original bytes and the completed package seal."""
import hashlib
import json
from pathlib import Path


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    root = Path(__file__).resolve().parent
    manifest = json.loads((root / "manifest.json").read_text())
    assert manifest["provider_drift"] == {}
    assert manifest["provider_before_sha256"] == manifest["provider_after_sha256"]
    assert len(manifest["provider_before_sha256"]) == 89
    assert sha(root / "upstream.json") == manifest["fresh_upstream_receipt_sha256"]
    assert sha(root / "failed-metadata-setup.json") == manifest["failed_metadata_setup_sha256"]
    assert json.loads((root / "upstream.json").read_text())["commit"]["sha"] == manifest["provider_revision"]
    for original, retained in [("nshelper.tcl", "raw-nshelper-source.tcl"), ("_nshelper.c", "original-_nshelper.c"), ("make-c-ext.tcl", "original-make-c-ext.tcl")]:
        assert sha(root / retained) == manifest["provider_before_sha256"][original]
    for name, digest in manifest["inputs"].items():
        assert sha(root / name) == digest, name
    for key in ["local_revision", "tracked_provider_status", "compiler", "version", "compile", "probe", "original_namespace_suite"]:
        stage = manifest[key]
        assert stage["exit"] == 0 and stage["completion"] == "process completed", key
        label = {"local_revision": "local-revision", "tracked_provider_status": "provider-status", "original_namespace_suite": "namespace-suite"}.get(key, key)
        for stream in ["stdout", "stderr"]:
            assert sha(root / f"{label}.{stream}") == stage[f"{stream}_sha256"], (key, stream)
    assert (root / "probe.stderr").read_bytes() == b""
    assert (root / "provider-status.stdout").read_bytes() == b""
    assert (root / "local-revision.stdout").read_text().strip() == manifest["provider_revision"]
    assert "Passed    79  Skipped     0  Failed     0" in (root / "namespace-suite.stdout").read_text()
    rows = [line.split("|") for line in (root / "probe.stdout").read_text().splitlines()]
    assert len(rows) == 211
    assert all(row[-1] == "0" for row in rows if row[0] == "SETUP")
    for purpose in ["CORE", "NAMESPACE_PRIMITIVE"]:
        presence = [row for row in rows if row[:2] == ["HELPER", purpose]]
        assert len(presence) == 9 and all(row[4] == "0" for row in presence)
    observations = json.loads((root / "observations.json").read_text())
    for purpose, helpers in observations.items():
        assert len(helpers) == 9
        for helper in helpers:
            ordinal = str(helper["ordinal"])
            assert helper["present"]
            for role in ["formals", "body"]:
                record = helper[role]
                value = (root / record["path"]).read_bytes()
                original = next(row[4] for row in rows if row[:4] == ["BYTES", purpose, ordinal, role.upper()])
                assert value == bytes.fromhex(original)
                assert len(value) == record["bytes"] and sha(root / record["path"]) == record["sha256"]
            source = helper["derived_source_slice"]
            whole = (root / source["source"]).read_bytes()
            assert sha(root / source["source"]) == source["source_sha256"]
            assert whole[source["body_start"]:source["body_end_exclusive"]] == (root / helper["body"]["path"]).read_bytes()
            assert whole[:source["body_start"]].count(b"\n") + 1 == helper["body"]["first_line"]
    differing = []
    for static, raw, again in zip(observations["STATIC"], observations["RAW_ONLY"], observations["RAW_AFTER_STATIC"]):
        for role in ["formals", "body"]:
            assert raw[role]["sha256"] == again[role]["sha256"]
        if static["body"]["sha256"] != raw["body"]["sha256"]:
            differing.append(static["name"])
    assert differing == ["namespace code", "namespace import", "namespace info", "namespace ensemble"]
    sealed = set()
    for line in (root / "SHA256SUMS").read_text().splitlines():
        digest, name = line.split("  ", 1)
        sealed.add(name)
        assert sha(root / name) == digest, name
    assert sealed == {str(path.relative_to(root)) for path in root.rglob("*") if path.is_file() and path.name != "SHA256SUMS"}
    print("PASS: 211 original SDK rows; 27 source slices; all provider and package hashes retained")


if __name__ == "__main__":
    main()
