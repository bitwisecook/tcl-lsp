"""Verify retained original captured error-code options evidence; never execute a provider."""
import argparse
import ast
import hashlib
import json
import re
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
for number in [1, 2]:
    capture = root / f"capture0{number}"
    manifest = json.loads((capture / "manifest.json").read_text())
    assert manifest["setup_ready"] and manifest["upstream_matches_local"]
    assert manifest["provider_unchanged"] and manifest["source_unchanged"]
    assert manifest["provider_before_sha256"] == manifest["provider_after_sha256"]
    assert len(manifest["provider_before_sha256"]) == 247
    assert manifest["rows"] == (6 if number == 1 else 12)
    for stage, label in [("compile", "compile"), ("execution", "execution"), ("original_namespace_suite", "namespace-suite")]:
        result = manifest[stage]
        assert result["exit_code"] == 0 and not result["timed_out"]
        assert sha(capture / (label + ".stdout")) == result["stdout_sha256"]
        assert sha(capture / (label + ".stderr")) == result["stderr_sha256"]
        assert not (capture / (label + ".stderr")).read_bytes()
    assert (capture / "upstream.stdout").read_bytes() == (capture / "local-revision.stdout").read_bytes().strip() + b"\tHEAD\n"
    assert not (capture / "local-status.stdout").read_bytes()
    assert b"Passed    79" in (capture / "namespace-suite.stdout").read_bytes()
    assert b"Failed     0" in (capture / "namespace-suite.stdout").read_bytes()
    if number == 2:
        assert (root / "probe.c").read_bytes() == (capture / "probe.c").read_bytes()
    assert (root / "capture.py").read_bytes() == (capture / "capture.py").read_bytes()
    assert sha(capture / "probe.c") == manifest["source_sha256"]
    assert sha(capture / "capture.py") == manifest["driver_sha256"]
    fixture = root.parents[3] / "runtime/rust/tests/data/native_jim_captured_error_code_options"
    if number == 2:
        assert (fixture / "observations-20261015.tsv").read_bytes() == (capture / "execution.stdout").read_bytes()
        assert (capture / "execution.stdout").read_bytes().startswith((root / "capture01/execution.stdout").read_bytes())
    source = (capture / "probe.c").read_text()
    body = re.search(r'const char \*body = (".*");', source)[1]
    assert (fixture / "body.tcl").read_bytes() == ast.literal_eval(body).encode()
    if number == 2:
        ok_body = re.search(r'const char \*ok_body = (".*");', source)[1]
        assert (fixture / "ok-body.tcl").read_bytes() == ast.literal_eval(ok_body).encode()
    if (capture / "probe").exists():
        assert sha(capture / "probe") == manifest["executable_sha256"]
    if args.current_provider:
        for relative, expected in manifest["provider_before_sha256"].items():
            assert sha(Path(manifest["sdk_dir"]) / relative) == expected, relative
        for library, expected in manifest["linked_library_sha256"].items():
            assert sha(Path(library)) == expected, library
print("PASS: 6+12 original captured-options rows, exact fixtures and provider/package hashes")
