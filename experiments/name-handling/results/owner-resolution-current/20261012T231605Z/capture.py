#!/usr/bin/env python3
# Copyright (C) 2026 James Deucker (bitwisecook)
# SPDX-License-Identifier: AGPL-3.0-or-later
import datetime
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import time

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[4]
COMMANDS = {
    "controls": ["cargo", "test", "--offline", "-j2", "-p", "xtask", "owner_resolution::tests", "--", "--nocapture"],
    "gate": ["cargo", "run", "--offline", "-j2", "-p", "xtask", "--", "owner-resolution"],
}
stage = sys.argv[1]
command = COMMANDS[stage]
started = datetime.datetime.now(datetime.timezone.utc).isoformat()
before = time.monotonic()
receipt = {"stage": stage, "argv": command, "cwd": str(ROOT), "started_utc": started, "timeout_seconds": 900, "cache": os.environ["CARGO_TARGET_DIR"]}
(HERE / (stage + "-command.json")).write_text(json.dumps(receipt, indent=2) + "\n")
print(stage, "started", started, flush=True)
with (HERE / (stage + "-stdout.log")).open("wb") as stdout, (HERE / (stage + "-stderr.log")).open("wb") as stderr:
    try:
        process = subprocess.run(command, cwd=ROOT, stdout=stdout, stderr=stderr, timeout=900, check=False)
        receipt.update(exit_code=process.returncode, timed_out=False)
    except subprocess.TimeoutExpired:
        receipt.update(exit_code=None, timed_out=True)
receipt.update(completed_utc=datetime.datetime.now(datetime.timezone.utc).isoformat(), elapsed_seconds=time.monotonic() - before)
for kind in ["stdout", "stderr"]:
    receipt[kind + "_sha256"] = hashlib.sha256((HERE / (stage + "-" + kind + ".log")).read_bytes()).hexdigest()
(HERE / (stage + "-process.json")).write_text(json.dumps(receipt, indent=2) + "\n")
print(json.dumps(receipt), flush=True)
