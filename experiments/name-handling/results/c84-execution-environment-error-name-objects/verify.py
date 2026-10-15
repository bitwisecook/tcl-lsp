#!/usr/bin/env python3
"""Check retained original ExecEnv evidence without provider execution."""
import hashlib
import json
from pathlib import Path

root = Path(__file__).resolve().parent


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def streams(capture, record, label):
    for channel in ("stdout", "stderr"):
        assert sha(capture / (label + "." + channel)) == record[channel + "_sha256"]


for capture in sorted(root.glob("capture[0-9][0-9]")):
    manifest = json.loads((capture / "manifest.json").read_text())
    for name, key in (("probe.c", "source_sha256"), ("capture.py", "driver_sha256")):
        assert sha(capture / name) == manifest[key] == sha(root / name)
    assert manifest["upstream_commit"] == "9ccfe9d1b35741ff7323837f6485ffe48b06fad9"
    assert manifest["reference_source_tag"] == "core-8-4-20"
    assert sha(capture / "reference-toolchains.tsv") == manifest["reference_manifest_sha256"]
    assert manifest["source_unchanged"] and manifest["provider_unchanged"]
    assert manifest["provider_before_sha256"] == manifest["provider_after_sha256"]
    assert len(manifest["provider_before_sha256"]) == 432
    assert not manifest["original_library_test_mismatches"]
    original = json.loads((capture / "retained-upstream-inputs.json").read_text())
    assert set(original) - set(manifest["provider_before_sha256"]) == {"library/tclIndex", "tests/README", "tests/httpd"}
    assert all(manifest["provider_before_sha256"][name] == value
               for name, value in original.items() if name in manifest["provider_before_sha256"])
    for name, record in manifest["upstream_sources"].items():
        streams(capture, record, "upstream-" + Path(name).name)
    streams(capture, manifest["compiler"], "compiler")
    if capture.name == "capture01":
        assert not manifest["setup_ready"]
        assert "compile" not in manifest and "execution" not in manifest
        assert all(record["exit_code"] == 7 for record in manifest["upstream_sources"].values())
        continue
    assert manifest["setup_ready"]
    for name, record in manifest["upstream_sources"].items():
        assert record["exit_code"] == 0 and record["matches_sdk"]
        assert record["stdout_sha256"] == record["sdk_sha256"] == manifest["provider_before_sha256"][name]
    assert sha(capture / "probe") == manifest["executable_sha256"]
    for key, label in (("compile", "compile"), ("execution", "execution"),
                       ("linked_libraries", "linked-libraries"), ("original_result_suite", "result-suite")):
        record = manifest[key]
        assert record["exit_code"] == 0 and not record["timed_out"]
        streams(capture, record, label)
    assert "Total\t21\tPassed\t1\tSkipped\t20\tFailed\t0" in (capture / "result-suite.stdout").read_text()

rows = (root / "capture02/execution.stdout").read_text().splitlines()
assert len(rows) == 13
phases = ["birth-before-init", "after-init", "after-seed-default-append", "after-second-append",
          "after-reset", "after-nested-error", "independent-birth", "with-observation-roles"]
environments = [row.split("|") for row in rows if row.startswith("ENV|")]
assert [fields[1] for fields in environments] == phases
for fields in environments:
    assert fields[2:6] == ["1"] * 4
    assert fields[6] == fields[10] == ("2" if fields[1] == "with-observation-roles" else "1")
    expected_type = "none" if fields[1] in phases[:2] + ["independent-birth"] else "parsedVarName"
    assert fields[7] == fields[11] == expected_type
    assert fields[8] == fields[12] == "1"
    assert fields[9] == b"::errorInfo".hex() and fields[13] == b"::errorCode".hex()
    assert fields[14] == "-1"
assert "DISTINCT|same-interpreter|1" in rows
assert "DISTINCT|independent-interpreters|1|1|1|1" in rows
assert "SCRIPT|nested-error|1" in rows
assert "DELETE|independent-interpreter|completed" in rows
assert rows[-1] == "AFTER_DELETE|observer-held|1|1|1|parsedVarName|1|" + b"::errorInfo".hex() + "|1|parsedVarName|1|" + b"::errorCode".hex()
print("PASS: retained original ExecEnv capture02 13 rows; capture01 setup failure only; no provider execution")
