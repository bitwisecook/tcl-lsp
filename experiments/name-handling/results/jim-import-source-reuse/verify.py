"""Verify original Import reuse/break evidence without executing a provider."""
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
failed = json.loads((root / "capture01/manifest.json").read_text())
assert not failed["setup_ready"] and "compile" not in failed
assert failed["provider_unchanged"]
assert (root / "capture01/upstream.stderr").read_bytes()
capture = root / "capture02"
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
assert b"Passed    79" in (capture / "namespace-suite.stdout").read_bytes()
assert b"Failed     0" in (capture / "namespace-suite.stdout").read_bytes()
assert manifest["rows"] == 15
assert (root / "probe.c").read_bytes() == (capture / "probe.c").read_bytes()
assert sha(capture / "probe.c") == manifest["source_sha256"]
assert (root / "capture.py").read_bytes() == (capture / "capture.py").read_bytes()
assert sha(capture / "capture.py") == manifest["driver_sha256"]
fixture = root.parents[3] / "runtime/rust/tests/data/native_jim_import_source_reuse"
assert (fixture / "observations-20261015.tsv").read_bytes() == (capture / "execution.stdout").read_bytes()
source = (capture / "probe.c").read_text()
reused = ast.literal_eval(re.search(r'const char \*import = (".*");', source)[1])
scripts = []
for match in re.finditer(r'report\(interp, ("(?:\\.|[^"\\])*"), ("(?:\\.|[^"\\])*"|import)\)', source):
    label = ast.literal_eval(match[1])
    script = reused if match[2] == "import" else ast.literal_eval(match[2])
    scripts.append(label + "|" + script + "\n")
assert (fixture / "scripts.tsv").read_text() == "".join(scripts)
if (capture / "probe").exists():
    assert sha(capture / "probe") == manifest["executable_sha256"]
if args.current_provider:
    sdk = Path(manifest["sdk_dir"])
    for relative, expected in manifest["provider_before_sha256"].items():
        assert sha(sdk / relative) == expected, relative
    for library, expected in manifest["linked_library_sha256"].items():
        assert sha(Path(library)) == expected, library
print("PASS: 15 original reuse/break rows, exact scripts/fixture, failed setup and provider closure")
