#!/usr/bin/env python3
"""Replay separately without modifying retained original evidence."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--out", required=True, type=Path)
    args = parser.parse_args()
    base = Path(__file__).resolve().parent
    manifest = json.loads((base / "manifest.json").read_text())
    if sha(base / "probe.tcl") != manifest["probe_sha256"]:
        raise SystemExit("probe hash mismatch")
    args.out.mkdir(parents=True, exist_ok=False)
    rows = []
    for original in manifest["rows"]:
        expected = original["original_files_before"]
        before = {path: sha(path) for path in expected}
        if before != expected:
            raise SystemExit("provider/init hash mismatch")
        env = os.environ.copy()
        library = original.get("selected_original_library")
        if library:
            env["TCL_LIBRARY"] = library
        argv = [original["argv"][0], str(base / "probe.tcl")]
        process = subprocess.run(argv, env=env, capture_output=True, timeout=30)
        after = {path: sha(path) for path in expected}
        if before != after:
            raise SystemExit("provider/init changed")
        (args.out / original["stdout_file"]).write_bytes(process.stdout)
        (args.out / original["stderr_file"]).write_bytes(process.stderr)
        rows.append({"provider_label": original["provider_label"], "argv": argv,
                     "exit_code": process.returncode, "before": before, "after": after,
                     "stdout_sha256": hashlib.sha256(process.stdout).hexdigest(),
                     "stderr_sha256": hashlib.sha256(process.stderr).hexdigest()})
    (args.out / "replay-manifest.json").write_text(json.dumps(rows, indent=2) + "\n")
    print(json.dumps({"count": len(rows), "exit_codes": [row["exit_code"] for row in rows]}))


if __name__ == "__main__":
    main()
