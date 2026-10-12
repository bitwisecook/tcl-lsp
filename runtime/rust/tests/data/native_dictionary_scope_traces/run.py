#!/usr/bin/env python3
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.
#
# SPDX-License-Identifier: AGPL-3.0-or-later

"""Capture original dictionary scope read callbacks from the six pinned engines."""

import fcntl
import hashlib
import json
import subprocess
import time
from contextlib import contextmanager
from pathlib import Path


@contextmanager
def native_slot():
    while True:
        for number in range(2):
            with Path(f"/workspace/.proofs/2286-test-slot-{number}.lock").open(
                "a"
            ) as lock:
                try:
                    fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
                except BlockingIOError:
                    continue
                yield
                return
        time.sleep(0.1)


def digest(data):
    return hashlib.sha256(data).hexdigest()


def capture():
    output = Path(__file__).resolve().parent
    cases = json.loads((output / "cases.json").read_text())
    engines = [
        (version, Path(f"/tmp/2286-oracles/bin/tclsh{short}"))
        for short, version in [
            ("8.4", "8.4.20"),
            ("8.5", "8.5.19"),
            ("8.6", "8.6.18"),
            ("9.0", "9.0.4"),
            ("9.1", "9.1.0"),
        ]
    ] + [("jim", Path("/tmp/2286-oracles/jimtcl/jimsh"))]
    manifest = []
    rows = []
    for version, executable in engines:
        for case in cases:
            source = case["source"].encode()
            observed = (
                b"set code [catch {"
                + source
                + b'} result]\nbinary scan $result H* hex\nputs "$code\\t$hex"\n'
            )
            with native_slot():
                started = time.monotonic()
                run = subprocess.run(
                    [str(executable)],
                    input=observed,
                    capture_output=True,
                    timeout=60,
                    check=False,
                )
            assert run.returncode == 0 and not run.stderr, (version, case, run)
            code, result = run.stdout.decode().strip().split("\t")
            rows.append("\t".join([version, case["name"], source.hex(), code, result]))
            manifest.append(
                {
                    "version": version,
                    "case": case["name"],
                    "command": [str(executable)],
                    "binary_sha256": digest(executable.read_bytes()),
                    "source_sha256": digest(source),
                    "observer_source_sha256": digest(observed),
                    "exit": run.returncode,
                    "stderr_hex": run.stderr.hex(),
                    "stdout_hex": run.stdout.hex(),
                    "seconds": time.monotonic() - started,
                    "budget_seconds": 60,
                }
            )
        print(version, len(cases), flush=True)
    (output / "controls.tsv").write_text("\n".join(rows) + "\n")
    (output / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")


if __name__ == "__main__":
    capture()
