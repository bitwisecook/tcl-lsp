#!/usr/bin/env python3
"""Replay controlled observations separately against exact original providers."""
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
    parser.add_argument("--repo", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()
    base = Path(__file__).resolve().parent / "oracles" / "controlled"
    manifest = json.loads((base / "manifest.json").read_text())
    if sha(base / "probe.tcl") != manifest["probe_sha256"]:
        raise SystemExit("original probe mismatch")
    def mapped(path):
        prefix = "/workspace/tcl-lsp/"
        if not path.startswith(prefix):
            raise SystemExit("unrepresented provider path")
        return args.repo / path[len(prefix):]
    args.out.mkdir(parents=True, exist_ok=False)
    rows = []
    for original in manifest["rows"]:
        expected = original["original_files_before"]
        before = {path: sha(mapped(path)) for path in expected}
        if before != expected:
            raise SystemExit("original provider/init hash mismatch")
        env = os.environ.copy()
        env.pop("TCL_LIBRARY", None)
        library = original.get("selected_original_library")
        if library:
            env["TCL_LIBRARY"] = str(mapped(library))
        argv = [str(mapped(original["argv"][0])), str(base / "probe.tcl")]
        process = subprocess.run(argv, env=env, capture_output=True, timeout=30)
        after = {path: sha(mapped(path)) for path in expected}
        if before != after:
            raise SystemExit("provider/init changed")
        row = {"provider_label": original["provider_label"], "argv": argv,
               "exit_code": process.returncode, "before": before, "after": after}
        for channel, data in [("stdout", process.stdout), ("stderr", process.stderr)]:
            name = original[channel + "_file"]
            (args.out / name).write_bytes(data)
            row[channel + "_sha256"] = hashlib.sha256(data).hexdigest()
        rows.append(row)
    (args.out / "replay-manifest.json").write_text(json.dumps(rows, indent=2) + "\n")


if __name__ == "__main__":
    main()
