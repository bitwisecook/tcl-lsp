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
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program. If not, see <https://www.gnu.org/licenses/>.
#
# SPDX-License-Identifier: AGPL-3.0-or-later

"""Retain real C Tcl results and the bytes of its loaded startup libraries."""

import argparse
import hashlib
import json
import re
import subprocess
from pathlib import Path


def digest(data):
    return hashlib.sha256(data).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--bin-dir", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    if args.output.exists() and any(args.output.iterdir()):
        parser.error("output must be new or empty")
    args.output.mkdir(parents=True, exist_ok=True)
    root = Path(__file__).resolve().parents[3]
    probes = Path(__file__).with_name("wiki-probes.tcl")
    manifest = root / "rust/tcl-dialect/data/reference-toolchains.tsv"
    startup = """
package require tcltest
puts [info patchlevel]
foreach path [list [file join [info library] init.tcl] [lindex [package ifneeded tcltest [package present tcltest]] end]] {
    set f [open $path r]
    fconfigure $f -translation binary
    set data [read $f]
    close $f
    binary scan $data H* bytes
    puts $path
    puts $bytes
}
"""
    records = []
    for line in manifest.read_text().splitlines():
        if not line or line.startswith("#"):
            continue
        release, patchlevel, tag = line.split("\t")
        executable = (args.bin_dir / ("tclsh" + release)).resolve()
        metadata = subprocess.run(
            [str(executable)],
            input=startup,
            capture_output=True,
            text=True,
            timeout=30,
            check=False,
        )
        lines = metadata.stdout.splitlines()
        if (
            metadata.returncode
            or metadata.stderr
            or len(lines) != 5
            or lines[0] != patchlevel
        ):
            raise RuntimeError(f"invalid startup for {release}: {metadata}")
        result = subprocess.run(
            [str(executable), str(probes)], capture_output=True, timeout=60, check=False
        )
        stem = "tcl" + patchlevel
        (args.output / (stem + ".stdout")).write_bytes(result.stdout)
        (args.output / (stem + ".stderr")).write_bytes(result.stderr)
        binary = executable
        wrapper = executable.read_bytes()
        if wrapper.startswith(b"#!/bin/sh"):
            target = re.search(rb'exec "([^"]+)"', wrapper)
            if target:
                binary = Path(target.group(1).decode())
        summary = re.search(
            rb"Total\s+(\d+)\s+Passed\s+(\d+)\s+Skipped\s+(\d+)\s+Failed\s+(\d+)",
            result.stdout,
        )
        records.append(
            {
                "release": release,
                "patchlevel": patchlevel,
                "upstream_tag": tag,
                "invocation": [str(executable), str(probes)],
                "binary": str(binary),
                "binary_sha256": digest(binary.read_bytes()),
                "startup_libraries": [
                    {"path": lines[i], "sha256": digest(bytes.fromhex(lines[i + 1]))}
                    for i in (1, 3)
                ],
                "exit_code": result.returncode,
                "stdout_sha256": digest(result.stdout),
                "stderr_sha256": digest(result.stderr),
                "counts": dict(
                    zip(
                        ("total", "passed", "skipped", "failed"),
                        map(int, summary.groups()),
                    )
                )
                if summary
                else None,
            }
        )
    provenance = {
        "retrieved_date": "2026-10-12",
        "manifest": str(manifest.relative_to(root)),
        "manifest_sha256": digest(manifest.read_bytes()),
        "probes": str(probes.relative_to(root)),
        "probes_sha256": digest(probes.read_bytes()),
        "runner_sha256": digest(Path(__file__).read_bytes()),
        "records": records,
    }
    (args.output / "provenance.json").write_text(
        json.dumps(provenance, indent=2) + "\n"
    )
    for record in records:
        print(record["patchlevel"], record["counts"])
    if any(r["exit_code"] or not r["counts"] or r["counts"]["failed"] for r in records):
        raise SystemExit(1)


if __name__ == "__main__":
    main()
