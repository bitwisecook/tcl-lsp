#!/usr/bin/env python3
"""Capture discrete array-name questions without authored expected results."""

import datetime
import hashlib
import json
import os
from pathlib import Path
import subprocess

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def run(argv, source, env):
    completed = subprocess.run(argv, input=source.read_bytes(), capture_output=True,
                               env=env, timeout=15, check=False)
    return {"argv": [str(part) for part in argv], "input": str(source.relative_to(HERE)),
            "input_sha256": digest(source), "exit_code": completed.returncode,
            "stdout_sha256": hashlib.sha256(completed.stdout).hexdigest(),
            "stderr_sha256": hashlib.sha256(completed.stderr).hexdigest(),
            "stdout": completed.stdout.decode("utf-8", "surrogateescape"),
            "stderr": completed.stderr.decode("utf-8", "surrogateescape")}


def main():
    output = HERE / "captures"
    output.mkdir(exist_ok=True)
    source_paths = [ROOT / f"tmp/tcl{version}/generic/tclVar.c"
                    for version in ["8.4.20", "8.5.19", "8.6.18", "9.0.4", "9.1.0"]]
    source_paths += [ROOT / "tmp/jimtcl-oracle/jim-array.c",
                     ROOT / "rust/tcl-cmd-core/src/array.rs"]
    provenance = {str(path.relative_to(ROOT)): digest(path) for path in source_paths}
    cases = sorted((HERE / "cases").glob("*.tcl"))
    cases += sorted((HERE / "traces").glob("*.tcl"))
    subjects = []
    for version in ["8.4.20", "8.5.19", "8.6.18", "9.0.4", "9.1.0"]:
        binary = ROOT / f"tmp/tcl{version}/unix/tclsh"
        library = ROOT / f"tmp/tcl{version}/library"
        subjects.append((f"c-{version}", [binary], library, "real C, matching init"))
    subjects.append(("jim-5bac7c9", [ROOT / "tmp/jimtcl-oracle/jimsh"], None,
                     "latest independently built upstream Jim in this session"))
    frozen = Path("/tmp/name-native-c84-declared-cell-runner")
    for version in ["8.4", "8.5", "8.6", "9.0", "9.1"]:
        subjects.append((f"frozen-runtime-{version}",
                         [frozen, "--quiet", "--tcl-version", version], None,
                         "frozen native runner, no init; not current-tree build evidence"))
    for label, argv, library, channel in subjects:
        env = os.environ.copy()
        if library is not None:
            env["TCL_LIBRARY"] = str(library)
        records = [run(argv, case, env) for case in cases]
        identity = run(argv, HERE / "identity.tcl", env)
        record = {"captured_utc": datetime.datetime.now(datetime.timezone.utc).isoformat(),
                  "subject": label, "channel": channel,
                  "binary_sha256": digest(Path(argv[0])),
                  "TCL_LIBRARY": str(library) if library else None,
                  "source_sha256": provenance, "identity": identity, "cases": records}
        if label.startswith("c-"):
            record["init_sha256"] = digest(library / "init.tcl")
        if label.startswith("jim-"):
            record["upstream_revision"] = subprocess.check_output(
                ["git", "-C", str(ROOT / "tmp/jimtcl-oracle"), "rev-parse", "HEAD"],
                text=True).strip()
        (output / f"{label}.json").write_text(json.dumps(record, indent=2) + "\n")
        print(label, len(records), "cases", "process-errors", sum(r["exit_code"] != 0 for r in records))


if __name__ == "__main__":
    main()
