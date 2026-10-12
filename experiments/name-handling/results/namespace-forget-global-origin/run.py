#!/usr/bin/env python3
# Copyright (C) 2026 James Deucker (bitwisecook)
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Capture original #2300 native operations; unavailable APIs are not adapted."""
import concurrent.futures
import datetime
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile
import time

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[3]
PATTERNS = ROOT / "experiments/name-handling/patterns"


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def capture(provider):
    label, binary, library, expected_hash = provider
    dest = HERE / "results" / label
    dest.mkdir(parents=True, exist_ok=False)
    assert digest(binary) == expected_hash
    command = [str(binary), str(HERE / "probe.tcl")]
    environment = dict(os.environ, LC_ALL="C")
    if library is not None:
        environment["TCL_LIBRARY"] = str(library)
    started = datetime.datetime.now(datetime.timezone.utc).isoformat()
    before = time.monotonic()
    with tempfile.TemporaryDirectory(prefix="forget-origin-oracle-") as work:
        try:
            process = subprocess.run(command, cwd=work, env=environment,
                                     capture_output=True, timeout=30, check=False)
            stdout, stderr, code, timeout = process.stdout, process.stderr, process.returncode, False
        except subprocess.TimeoutExpired as exc:
            stdout, stderr, code, timeout = exc.stdout or b"", exc.stderr or b"", None, True
    (dest / "stdout.txt").write_bytes(stdout)
    (dest / "stderr.txt").write_bytes(stderr)
    fields = {}
    for line in stdout.decode("utf-8", errors="replace").splitlines():
        if line.startswith("FIELD "):
            _, key, encoded = line.split(" ", 2)
            fields[key] = bytes.fromhex(encoded).decode("utf-8", errors="replace")
    outcome = "captured-original-native-control"
    if timeout or code != 0 or "setup_code" not in fields:
        outcome = "capture-failure"
    elif fields["setup_code"] != "0":
        outcome = "original-setup-refusal"
    elif fields.get("positive_code") != "0":
        outcome = "original-operation-refusal"
    row = {"provider": label, "outcome": outcome, "argv": command,
           "binary_sha256": digest(binary), "input_sha256": digest(HERE / "probe.tcl"),
           "started_utc": started, "completed_utc": datetime.datetime.now(datetime.timezone.utc).isoformat(),
           "elapsed_seconds": time.monotonic() - before, "timeout_seconds": 30,
           "process_exit_code": code, "timed_out": timeout, "fields": fields,
           "stdout_sha256": digest(dest / "stdout.txt"), "stderr_sha256": digest(dest / "stderr.txt")}
    (dest / "result.json").write_text(json.dumps(row, indent=2) + "\n")
    return row


references = json.loads((PATTERNS / "results/provenance.json").read_text())["references"]
providers = []
for reference in references:
    tree = ROOT / "tmp" / ("tcl" + reference["patchlevel"])
    providers.append(("tcl" + reference["patchlevel"], tree / "unix/tclsh",
                      tree / "library", reference["binary_sha256"]))
jim = json.loads((PATTERNS / "jim-results-5bac7c9/provenance.json").read_text())
providers.append(("jim-5bac7c9", ROOT / "tmp/jimtcl-oracle/jimsh", None, jim["binary_sha256"]))
with concurrent.futures.ThreadPoolExecutor(max_workers=6) as executor:
    rows = list(executor.map(capture, providers))
result = {"question": "Exact original #2300 setup/operation plus fixed adverse controls",
          "issue_url": "https://github.com/bitwisecook/tcl-lsp/issues/2300",
          "input_sha256": digest(HERE / "probe.tcl"), "runner_sha256": digest(Path(__file__)),
          "original_witness_sha256": digest(HERE / "issue-witness.tcl"),
          "jim_revision": jim["revision"], "providers": rows,
          "Rust_consumer_execution": "not measured", "F5": "pending; not executed"}
(HERE / "result.json").write_text(json.dumps(result, indent=2) + "\n")
for row in rows:
    print(row["provider"], row["outcome"], row["fields"])
