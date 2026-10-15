"""Verify original catch/try output keys and global-frame observations."""
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
for number, rows in [(1, 18), (2, 27), (3, 33)]:
    capture = root / f"capture0{number}"
    manifest = json.loads((capture / "manifest.json").read_text())
    assert manifest["setup_ready"] and manifest["upstream_matches_local"]
    assert manifest["provider_unchanged"] and manifest["source_unchanged"]
    assert manifest["provider_before_sha256"] == manifest["provider_after_sha256"]
    assert len(manifest["provider_before_sha256"]) == 247
    for stage, label in [("compile", "compile"), ("execution", "execution"), ("original_namespace_suite", "namespace-suite")]:
        result = manifest[stage]
        assert result["exit_code"] == 0 and not result["timed_out"]
        assert sha(capture / (label + ".stdout")) == result["stdout_sha256"]
        assert sha(capture / (label + ".stderr")) == result["stderr_sha256"]
        assert not (capture / (label + ".stderr")).read_bytes()
    assert (capture / "upstream.stdout").read_bytes() == (capture / "local-revision.stdout").read_bytes().strip() + b"\tHEAD\n"
    assert not (capture / "local-status.stdout").read_bytes()
    assert manifest["rows"] == rows
    assert b"Passed    79" in (capture / "namespace-suite.stdout").read_bytes()
    assert b"Failed     0" in (capture / "namespace-suite.stdout").read_bytes()
    assert sha(capture / "probe.c") == manifest["source_sha256"]
    assert sha(capture / "capture.py") == manifest["driver_sha256"]
    if (capture / "probe").exists():
        assert sha(capture / "probe") == manifest["executable_sha256"]
    if args.current_provider:
        for relative, expected in manifest["provider_before_sha256"].items():
            assert sha(Path(manifest["sdk_dir"]) / relative) == expected, relative
        for library, expected in manifest["linked_library_sha256"].items():
            assert sha(Path(library)) == expected, library
capture = root / "capture03"
assert (root / "probe.c").read_bytes() == (capture / "probe.c").read_bytes()
assert (root / "capture.py").read_bytes() == (capture / "capture.py").read_bytes()
for previous in ["capture01", "capture02"]:
    assert set((root / previous / "execution.stdout").read_bytes().splitlines()).issubset(
        set((capture / "execution.stdout").read_bytes().splitlines())
    )
fixture = root.parents[3] / "runtime/rust/tests/data/native_jim_catch_try_original_output_keys"
assert (fixture / "observations-20261015.tsv").read_bytes() == (capture / "execution.stdout").read_bytes()
source = (capture / "probe.c").read_text()
setups = re.search(r'const char \*setups\[\] = \{(.*?)\};', source, re.S)[1]
setups = [ast.literal_eval(value) for value in re.findall(r'"(?:\\.|[^"\\])*"', setups)]
assert (fixture / "global-setups.tsv").read_text() == "".join(str(i) + "|" + value + "\n" for i, value in enumerate(setups))
call = re.search(r'code = Jim_Eval\(interp, ("namespace eval CatchKey.*")\);', source)[1]
assert (fixture / "global-call.tcl").read_bytes() == ast.literal_eval(call).encode()
assert sha(root / "incorrect-array-control-f462.rs") == "c0dd481dc370d60835491bf5f1eeeedd4a9e893aa62c43c78f30fac5f6e7ae15"
print("PASS: 18+27+33 original rows, exact fixtures, incorrect setup beforeimage, provider/package hashes")
