"""Verify retained original quiet global getter evidence; never execute a provider."""
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
capture = root / "capture01"
manifest = json.loads((capture / "manifest.json").read_text())
assert manifest["setup_ready"] and manifest["upstream_matches_local"]
assert manifest["provider_unchanged"] and manifest["source_unchanged"]
assert manifest["provider_before_sha256"] == manifest["provider_after_sha256"]
assert len(manifest["provider_before_sha256"]) == 247
assert manifest["rows"] == 12
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
assert (root / "probe.c").read_bytes() == (capture / "probe.c").read_bytes()
assert (root / "capture.py").read_bytes() == (capture / "capture.py").read_bytes()
assert sha(capture / "probe.c") == manifest["source_sha256"]
assert sha(capture / "capture.py") == manifest["driver_sha256"]
fixture = root.parents[3] / "runtime/rust/tests/data/native_jim_original_global_error_code_getter"
assert (fixture / "observations-20261015.tsv").read_bytes() == (capture / "execution.stdout").read_bytes()
source = (capture / "probe.c").read_text()
setups = re.search(r'const char \*setups\[\] = \{(.*?)\};', source, re.S)[1]
setups = [ast.literal_eval(value) for value in re.findall(r'"(?:\\.|[^"\\])*"', setups)]
assert (fixture / "setups.tsv").read_text() == "".join(str(i) + "|" + value + "\n" for i, value in enumerate(setups))
call = re.search(r'code = Jim_Eval\(interp, ("namespace eval ReadKey.*")\);', source)[1]
assert (fixture / "call.tcl").read_bytes() == ast.literal_eval(call).encode()
if (capture / "probe").exists():
    assert sha(capture / "probe") == manifest["executable_sha256"]
if args.current_provider:
    for relative, expected in manifest["provider_before_sha256"].items():
        assert sha(Path(manifest["sdk_dir"]) / relative) == expected, relative
    for library, expected in manifest["linked_library_sha256"].items():
        assert sha(Path(library)) == expected, library
print("PASS: 12 original quiet-getter rows, exact fixtures and provider/package hashes")
