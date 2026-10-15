#!/usr/bin/env python3
"""Verify retained original C84 evidence without provider execution."""
import hashlib
import json
from pathlib import Path

root = Path(__file__).resolve().parent
capture = root / "capture01"
manifest = json.loads((capture / "manifest.json").read_text())


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def streams(record, label):
    assert record["exit_code"] == 0 and not record["timed_out"]
    for stream in ("stdout", "stderr"):
        assert digest(capture / (label + "." + stream)) == record[stream + "_sha256"]


assert manifest["setup_ready"] and not manifest["original_library_test_mismatches"]
assert manifest["reference_source_tag"] == "core-8-4-20"
assert manifest["upstream_commit"] == "9ccfe9d1b35741ff7323837f6485ffe48b06fad9"
assert manifest["provider_unchanged"] and manifest["source_unchanged"]
assert manifest["provider_before_sha256"] == manifest["provider_after_sha256"]
assert len(manifest["provider_before_sha256"]) == 432
for name, field in (("probe.c", "source_sha256"), ("capture.py", "driver_sha256")):
    assert digest(capture / name) == manifest[field] == digest(root / name)
assert digest(capture / "probe") == manifest["executable_sha256"]
assert digest(capture / "reference-toolchains.tsv") == manifest["reference_manifest_sha256"]
for key, label in (("compiler", "compiler"), ("compile", "compile"),
                   ("execution", "execution"), ("linked_libraries", "linked-libraries"),
                   ("original_result_suite", "result-suite")):
    streams(manifest[key], label)
for name, record in manifest["upstream_sources"].items():
    streams(record, "upstream-" + Path(name).name)
    assert record["matches_sdk"]
    assert record["stdout_sha256"] == record["sdk_sha256"] == manifest["provider_before_sha256"][name]
original = json.loads((capture / "retained-upstream-inputs.json").read_text())
assert set(original) - set(manifest["provider_before_sha256"]) == {"library/tclIndex", "tests/README", "tests/httpd"}
assert all(manifest["provider_before_sha256"][name] == value
           for name, value in original.items() if name in manifest["provider_before_sha256"])
# These three original support entries were checked at setup by the retained
# driver; its before/after extension inventory does not include their suffixes.
assert "Total\t21\tPassed\t1\tSkipped\t20\tFailed\t0" in (capture / "result-suite.stdout").read_text()
rows = (capture / "execution.stdout").read_text().splitlines()
assert len(rows) == 16
observed = [row.split("|") for row in rows if row.startswith("OBS|")]
assert len(observed) == 6
for mode, fields in enumerate(observed):
    assert fields[1] == str(mode)
    assert fields[3:8] == ["0", "1", "1", "1", "1"]
    assert fields[9:11] == ["int", "0"]
    assert fields[11] == (b"TRACE_RESULT" if mode == 5 else b"MARKER").hex()
    assert fields[8] == ("0" if mode in (3, 4) else "1")
    assert fields[12] == ("-" if mode == 3 else (b"REPLACED" if mode == 4 else b"777").hex())
traces = [row.split("|") for row in rows if row.startswith("TRACE|")]
assert [row[1] for row in traces] == ["1", "2", "4", "5"]
assert all(row[2:] == ["1", b"errorCode".hex(), "-", "1", "1", "1", "0"] for row in traces)
assert [row for row in rows if row.startswith("CALL|")] == [
    f"CALL|{mode}|0|{b'LOCAL'.hex()}|{b'NAMESPACE'.hex()}" for mode in range(6)
]
print("PASS: retained original C84 evidence, 6 cases/16 rows; no provider execution")
