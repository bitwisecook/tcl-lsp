"""Verify original C try evidence and provider pairing without execution."""
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
assert list(manifest["versions"]) == ["8.6.18", "9.0.4", "9.1.0"]
for name, key in [("probe.c", "source_sha256"), ("capture.py", "driver_sha256")]:
    assert (root / name).read_bytes() == (capture / name).read_bytes()
    assert sha(capture / name) == manifest[key]
for version, row in manifest["versions"].items():
    assert row["provider_unchanged"]
    assert row["provider_before_sha256"] == row["provider_after_sha256"]
    for stage in ["compile", "execution"]:
        result = row[stage]
        assert result["exit_code"] == 0 and not result["timed_out"]
        for channel in ["stdout", "stderr"]:
            path = capture / (version + "-" + stage + "." + channel)
            assert sha(path) == result[channel + "_sha256"]
        assert not (capture / (version + "-" + stage + ".stderr")).read_bytes()
    output = (capture / (version + "-execution.stdout")).read_bytes()
    assert len(output.splitlines()) == 31
    assert output.startswith(("HEADER|" + version + "\n").encode())
    assert all(("VERSION|" + str(mode) + "|" + version).encode() in output.splitlines() for mode in range(5))
    fixture = root.parents[3] / "runtime/rust/tests/data/native_c_try_original_output_names" / (version + ".tsv")
    assert output == fixture.read_bytes()
    executable = capture / ("probe-" + version)
    if executable.exists(): assert sha(executable) == row["executable_sha256"]
    if args.current_provider:
        for relative, expected in row["provider_before_sha256"].items():
            assert sha(Path(row["tree"]) / relative) == expected, relative
        for library, expected in row["linked_library_sha256"].items():
            assert sha(Path(library)) == expected, library
print("PASS: three original C providers, 31 rows each, exact fixtures/source/library hashes")
