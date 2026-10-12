#!/usr/bin/env python3
"""Verify the returned-row adaptation against each retained real oracle."""

import hashlib
import json
import os
from pathlib import Path
import subprocess

HERE = Path(__file__).resolve().parent


def main():
    destination = HERE / "regression-vectors"
    destination.mkdir(exist_ok=True)
    records = []
    for capture in sorted((HERE / "captures").glob("*.json")):
        oracle = json.loads(capture.read_text())
        if oracle["subject"].startswith("frozen-"):
            continue
        rows = []
        env = os.environ.copy()
        if oracle["TCL_LIBRARY"]:
            env["TCL_LIBRARY"] = oracle["TCL_LIBRARY"]
        for case in oracle["cases"]:
            source = (HERE / case["input"]).read_text()
            adapted = "set ::__observations {}\n" + source.replace(
                "puts [list", "lappend ::__observations [list")
            adapted += 'puts [join $::__observations "\\n"]\n'
            actual = subprocess.run(case["argv"], input=adapted.encode(),
                                    capture_output=True, env=env, timeout=15, check=False)
            assert actual.returncode == case["exit_code"], (capture, case["input"])
            assert actual.stdout == case["stdout"].encode(), (capture, case["input"], actual.stdout)
            assert actual.stderr == case["stderr"].encode(), (capture, case["input"], actual.stderr)
            assert "\\n" not in case["stdout"]
            rows.append(case["input"] + "\t" + case["stdout"].replace("\n", "\\n"))
            records.append({"subject": oracle["subject"], "input": case["input"],
                            "argv": case["argv"], "binary_sha256": oracle["binary_sha256"],
                            "adapted_input_sha256": hashlib.sha256(adapted.encode()).hexdigest(),
                            "stdout_sha256": hashlib.sha256(actual.stdout).hexdigest(),
                            "stderr_sha256": hashlib.sha256(actual.stderr).hexdigest(),
                            "exit_code": actual.returncode})
        (destination / (oracle["subject"] + ".tsv")).write_text("\n".join(rows) + "\n")
        print(oracle["subject"], len(rows), "adapted rows match original raw capture")
    (destination / "adaptation-records.json").write_text(json.dumps(records, indent=2) + "\n")


if __name__ == "__main__":
    main()
