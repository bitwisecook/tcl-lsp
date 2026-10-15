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
    assert manifest["rows"] == 18
    assert b"Passed    79" in (capture / "namespace-suite.stdout").read_bytes()
    assert b"Failed     0" in (capture / "namespace-suite.stdout").read_bytes()
    fixture = root.parents[3] / "runtime/rust/tests/data/native_jim_import_source_completion/observations-20261015.tsv"
    assert fixture.read_bytes() == (capture / "execution.stdout").read_bytes()
    assert sha(capture / "probe.c") == manifest["source_sha256"]
    assert sha(capture / "capture.py") == manifest["driver_sha256"]
    if number == 2:
        assert (root / "probe.c").read_bytes() == (capture / "probe.c").read_bytes()
        assert (root / "capture.py").read_bytes() == (capture / "capture.py").read_bytes()
        setup = (capture / "probe.c").read_text().split("const char *setup=", 1)[1].split(";\n    if(Jim_Eval", 1)[0]
        setup = "".join(ast.literal_eval(value) for value in re.findall(r'"(?:\\.|[^"\\])*"', setup))
        assert fixture.with_name("setup.tcl").read_bytes() == setup.encode()
        source = (capture / "probe.c").read_text()
        report = source.split("const char *report=", 1)[1].split(";\n", 1)[0]
        assert fixture.with_name("commands-report.tcl").read_bytes() == ast.literal_eval(report).encode()
        report = re.search(r'code=Jim_Eval\(interp,("if.*")\);', source).group(1)
        assert fixture.with_name("alias-report.tcl").read_bytes() == ast.literal_eval(report).encode()
        labels = [ast.literal_eval(value) for value in re.findall(r'"(?:\\.|[^"\\])*"', source.split('const char *labels[]={', 1)[1].split('};', 1)[0])]
        inputs = [None if value == "NULL" else ast.literal_eval(value) for value in re.findall(r'"(?:\\.|[^"\\])*"|NULL', source.split('const char *inputs[]={', 1)[1].split('};', 1)[0])]
        cases = "".join(label + "|" + (value or "namespace import") + "\n" for label, value in zip(labels, inputs))
        assert fixture.with_name("cases.tsv").read_text() == cases
        cycle_source = source.split('static int cycle(void)', 1)[1]
        cycle_inputs = re.findall(r'(?:int )?code=Jim_Eval\(interp,(".*")\);', cycle_source)
        assert len(cycle_inputs) == 2
        for name, value in zip(["cycle-setup.tcl", "cycle-input.tcl"], cycle_inputs):
            assert fixture.with_name(name).read_bytes() == ast.literal_eval(value).encode()
        cycle = manifest["cycle"]
        assert cycle["timed_out"] and cycle["exit_code"] is None
        assert (capture / "cycle.stdout").read_bytes() == fixture.with_name("cycle-observation-20261015.tsv").read_bytes()
        assert sha(capture / "cycle.stdout") == cycle["stdout_sha256"]
        assert sha(capture / "cycle.stderr") == cycle["stderr_sha256"]
        assert not (capture / "cycle.stderr").read_bytes()
    if (capture / "probe").exists():
        assert sha(capture / "probe") == manifest["executable_sha256"]
    if args.current_provider:
        sdk = Path(manifest["sdk_dir"])
        for relative, expected in manifest["provider_before_sha256"].items():
            assert sha(sdk / relative) == expected, relative
        for library, expected in manifest["linked_library_sha256"].items():
            assert sha(Path(library)) == expected, library
print("PASS: original Import completion reports, separate bounded cycle noncompletion, exact fixtures and provider/package hashes")
