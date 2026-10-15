"""Verify original C catch evidence and provider pairing without execution."""
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
versions = ["8.4.20", "8.5.19", "8.6.18", "9.0.4", "9.1.0"]
for number in [1, 2]:
    capture = root / f"capture0{number}"
    manifest = json.loads((capture / "manifest.json").read_text())
    assert list(manifest["versions"]) == versions
    assert sha(capture / "probe.c") == manifest["source_sha256"]
    if number == 2:
        assert (root / "probe.c").read_bytes() == (capture / "probe.c").read_bytes()
        assert (root / "capture.py").read_bytes() == (capture / "capture.py").read_bytes()
        assert sha(capture / "capture.py") == manifest["driver_sha256"]
    for version, row in manifest["versions"].items():
        assert row["provider_unchanged"]
        assert row["provider_before_sha256"] == row["provider_after_sha256"]
        for stage in ["compile", "execution"]:
            result = row[stage]
            assert result["exit_code"] == 0 and not result.get("timed_out", False)
            for channel in ["stdout", "stderr"]:
                path = capture / (version + "-" + stage + "." + channel)
                assert sha(path) == result[channel + "_sha256"]
            assert not (capture / (version + "-" + stage + ".stderr")).read_bytes()
        output = (capture / (version + "-execution.stdout")).read_bytes()
        assert len(output.splitlines()) == 13
        assert output.startswith(("HEADER|" + version + "\n").encode())
        assert all(("VERSION|" + str(mode) + "|" + version).encode() in output.splitlines() for mode in range(3))
        if number == 2:
            assert output == (root / "capture01" / (version + "-execution.stdout")).read_bytes()
            fixture = root.parents[3] / "runtime/rust/tests/data/native_c_catch_original_output_names" / (version + ".tsv")
            assert output == fixture.read_bytes()
        executable = capture / ("probe-" + version)
        if executable.exists(): assert sha(executable) == row["executable_sha256"]
        if args.current_provider:
            for relative, expected in row["provider_before_sha256"].items():
                assert sha(Path(row["tree"]) / relative) == expected, relative
            for library, expected in row["linked_library_sha256"].items():
                assert sha(Path(library)) == expected, library
print("PASS: five original C providers, 13 rows each, paired captures/fixtures/source/library hashes")
