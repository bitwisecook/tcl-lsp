"""Verify retained original Jim inventory evidence without executing a provider."""

import argparse
import hashlib
import json
from pathlib import Path


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


parser = argparse.ArgumentParser()
parser.add_argument("--current-provider", action="store_true")
args = parser.parse_args()
root = Path(__file__).resolve().parent
for line in (root / "SHA256SUMS").read_text().splitlines():
    expected, relative = line.split("  ", 1)
    assert sha(root / relative) == expected, relative
capture = root / "capture01"
manifest = json.loads((capture / "manifest.json").read_text())
assert manifest["setup_ready"] and manifest["upstream_matches_local"]
assert manifest["provider_unchanged"] and manifest["source_unchanged"]
assert manifest["provider_before_sha256"] == manifest["provider_after_sha256"]
assert len(manifest["provider_before_sha256"]) >= 200
for stage, label in [("compile", "compile"), ("execution", "execution"), ("original_namespace_suite", "namespace-suite")]:
    result = manifest[stage]
    assert result["exit_code"] == 0 and not result["timed_out"]
    assert sha(capture / (label + ".stdout")) == result["stdout_sha256"]
    assert sha(capture / (label + ".stderr")) == result["stderr_sha256"]
    assert not (capture / (label + ".stderr")).read_bytes()
assert (capture / "upstream.stdout").read_bytes() == (capture / "local-revision.stdout").read_bytes().strip() + b"\tHEAD\n"
assert manifest["rows"] == 24
assert b"Passed    79" in (capture / "namespace-suite.stdout").read_bytes()
assert b"Failed     0" in (capture / "namespace-suite.stdout").read_bytes()
fixture = root.parents[3] / "runtime/rust/tests/data/native_jim_command_inventory/original-key-objects-20261015.tsv"
assert fixture.read_bytes() == (capture / "execution.stdout").read_bytes()
assert (root / "probe.c").read_bytes() == (capture / "probe.c").read_bytes()
assert (root / "capture.py").read_bytes() == (capture / "capture.py").read_bytes()
if (capture / "probe").exists():
    assert sha(capture / "probe") == manifest["executable_sha256"]
if args.current_provider:
    sdk = Path(manifest["sdk_dir"])
    for relative, expected in manifest["provider_before_sha256"].items():
        assert sha(sdk / relative) == expected, relative
    for library, expected in manifest["linked_library_sha256"].items():
        assert sha(Path(library)) == expected, library
print("PASS: 24 original key/header observations, exact fixture, provider/package hashes")
