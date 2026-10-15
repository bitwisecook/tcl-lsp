"""Verify retained original Jim namespace-info evidence without executing a provider."""

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
for number in (1, 2):
    capture = root / f"capture0{number}"
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
    assert manifest["rows"] == (17 if number == 1 else 18)
    assert b"Passed    79" in (capture / "namespace-suite.stdout").read_bytes()
    assert b"Failed     0" in (capture / "namespace-suite.stdout").read_bytes()
    fixture = root.parents[3] / f"runtime/rust/tests/data/native_jim_namespace_info_local_dictionary/observations-20261015-0{number}.tsv"
    assert fixture.read_bytes() == (capture / "execution.stdout").read_bytes()
    setup = (capture / "probe.c").read_text().split("const char *setup =", 1)[1].split("if (Jim_Eval(interp, setup)", 1)[0]
    setup = "".join(ast.literal_eval(value) for value in re.findall(r'"(?:\\.|[^"\\])*"', setup))
    assert fixture.with_name(f"setup-0{number}.tcl").read_bytes() == setup.encode()
    assert sha(capture / "probe.c") == manifest["source_sha256"]
    if number == 2:
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
print("PASS: 35 original local projection/dictionary observations, exact fixtures and provider/package hashes")
