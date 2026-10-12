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

"""Capture original opaque namespace-origin diagnostics from five pinned engines."""

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
    source = (output / "source.tcl").read_bytes()
    observer = (
        b"set observed ["
        + source.strip()
        + b"]\nbinary scan $observed H* hex\nputs $hex\n"
    )
    rows = []
    manifest = []
    for short, version in [
        ("8.4", "8.4.20"),
        ("8.5", "8.5.19"),
        ("8.6", "8.6.18"),
        ("9.0", "9.0.4"),
        ("9.1", "9.1.0"),
    ]:
        executable = Path(f"/tmp/2286-oracles/bin/tclsh{short}")
        with native_slot():
            run = subprocess.run(
                [str(executable)],
                input=observer,
                capture_output=True,
                timeout=60,
                check=False,
            )
        assert run.returncode == 0 and not run.stderr, (version, run)
        rows.append(version + "\t" + run.stdout.decode().strip())
        manifest.append(
            {
                "version": version,
                "command": [str(executable)],
                "binary_sha256": digest(executable.read_bytes()),
                "source_sha256": digest(source),
                "observer_sha256": digest(observer),
                "exit": run.returncode,
                "stdout_hex": run.stdout.hex(),
                "stderr_hex": run.stderr.hex(),
            }
        )
    (output / "controls.tsv").write_text("\n".join(rows) + "\n")
    (output / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")


if __name__ == "__main__":
    capture()
