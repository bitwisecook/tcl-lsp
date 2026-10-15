"""Check original capture/fixture/provider hashes without executing a provider."""
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
assert len(manifest["provider_before_sha256"]) == 247
assert manifest["rows"] == 113
assert sha(capture / "probe.c") == manifest["source_sha256"]
assert sha(capture / "capture.py") == manifest["driver_sha256"]
assert (root / "probe.c").read_bytes() == (capture / "probe.c").read_bytes()
assert (root / "capture.py").read_bytes() == (capture / "capture.py").read_bytes()
labels = {"local_revision": "local-revision", "local_status": "local-status",
          "linked_libraries": "linked-libraries", "original_namespace_suite": "namespace-suite"}
for stage in ("upstream", "local_revision", "local_status", "compiler", "compile", "execution", "linked_libraries", "original_namespace_suite"):
    result = manifest[stage]
    assert result["exit_code"] == 0 and not result["timed_out"]
    label = labels.get(stage, stage)
    for stream in ("stdout", "stderr"):
        assert sha(capture / f"{label}.{stream}") == result[f"{stream}_sha256"]
    assert not (capture / f"{label}.stderr").read_bytes()
assert (capture / "upstream.stdout").read_bytes() == (capture / "local-revision.stdout").read_bytes().strip() + b"\tHEAD\n"
assert not (capture / "local-status.stdout").read_bytes()
assert b"Total    79   Passed    79  Skipped     0  Failed     0" in (capture / "namespace-suite.stdout").read_bytes()
fixture = root.parents[3] / "runtime/rust/tests/data/native_jim_try_leading_switches/observations-20261015.tsv"
assert fixture.read_bytes() == (capture / "execution.stdout").read_bytes()
assert len([line for line in fixture.read_text().splitlines() if line.startswith("OBS|")]) == 92
if (capture / "probe").exists():
    assert sha(capture / "probe") == manifest["executable_sha256"]
if args.current_provider:
    for relative, expected in manifest["provider_before_sha256"].items():
        assert sha(Path(manifest["sdk_dir"]) / relative) == expected, relative
    for library, expected in manifest["linked_library_sha256"].items():
        assert sha(Path(library)) == expected, library
cases = json.loads((root / "cases.json").read_text())
assert len(cases) == 23
assert (root / "cases.json").read_bytes() == (fixture.parent / "cases-20261015.json").read_bytes()
import ast, re
source = (capture / "probe.c").read_text().split('static const struct Case cases[] = {', 1)[1].split('\n    };', 1)[0]
pattern = r'CASE\(("(?:[^"\\]|\\.)*")\)|\{("(?:[^"\\]|\\.)*"),\s*(\d+),\s*(\d+),\s*(\d+),\s*(\d+)\}'
parsed = []
for match in re.finditer(pattern, source):
    value = ast.literal_eval(match.group(1) or match.group(2)).encode()
    flags = [0, 0, 0] if match.group(1) else list(map(int, match.group(4, 5, 6)))
    if not match.group(1):
        assert len(value) == int(match.group(3))
    parsed.append({"case": len(parsed), "selector_hex": value.hex(), "custom_selector": flags[0], "custom_body": flags[1], "custom_handler": flags[2]})
assert cases == parsed
assert (fixture.parent / "cases-20261015.tsv").read_text() == ''.join(f"{r['case']}|{r['selector_hex']}|{r['custom_selector']}|{r['custom_body']}|{r['custom_handler']}\n" for r in cases)
assert fixture.read_text().startswith("CONFIG|64|64\n")
print("PASS: original Try 113-row/92-cell leading-switch capture, exact inputs/fixture/provider hashes")
