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

"""Capture upstream naming suites with real init.tcl and tcltest startup."""

import argparse
import concurrent.futures
import hashlib
import json
import os
import re
import subprocess
import tempfile
import time
from pathlib import Path

STEMS = (
    "namespace",
    "namespace-old",
    "var",
    "set",
    "set-old",
    "upvar",
    "uplevel",
    "proc",
    "proc-old",
    "interp",
    "rename",
    "trace",
    "info",
    "cmdInfo",
    "resolver",
    "unknown",
    "oo",
    "ooNext2",
    "ooProp",
    "ooUtil",
    "apply",
)
SUMMARY = re.compile(
    r"Total\s+(\d+)\s+Passed\s+(\d+)\s+Skipped\s+(\d+)\s+Failed\s+(\d+)"
)
FAILED = re.compile(r"^====\s+(\S+).*\sFAILED\s*$", re.MULTILINE)
STARTED = re.compile(r"^----\s+(\S+)\s+start\s*$", re.MULTILINE)


def tcl_word(value):
    return (
        '"'
        + str(value)
        .replace("\\", "\\\\")
        .replace("$", "\\$")
        .replace("[", "\\[")
        .replace('"', '\\"')
        .replace("\n", "\\n")
        .replace("\r", "\\r")
        + '"'
    )


def sha256(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--tcl-root", type=Path, required=True)
    parser.add_argument("--tclsh", type=Path, required=True)
    parser.add_argument("--vm", type=Path)
    parser.add_argument("--runtime", type=Path)
    parser.add_argument("--version", default="9.0")
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--stem", action="append")
    parser.add_argument("--match")
    parser.add_argument("--timeout", type=int, default=90)
    parser.add_argument("--workers", type=int, default=3)
    parser.add_argument(
        "--no-overlay",
        action="store_true",
        help="omit the Tcl 8.5+ backend overlay for Tcl 8.4",
    )
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[2]
    source = args.tcl_root.resolve()
    output = args.output.resolve()
    if output.exists() and any(output.iterdir()):
        parser.error("output must be new or empty to preserve existing measurements")
    output.mkdir(parents=True, exist_ok=True)
    overlay = root / "tests/external/backend_constraints.tcl"
    library = source / "library"
    assert (library / "init.tcl").is_file()
    assert (library / "tcltest/tcltest.tcl").is_file()
    if args.vm and args.version != "9.0":
        parser.error("the existing VM run_test runner is fixed to Tcl 9.0")
    engines = {"c": [str(args.tclsh.resolve())]}
    if args.vm:
        engines["vm"] = [str(args.vm.resolve())]
    if args.runtime:
        engines["runtime"] = [
            str(args.runtime.resolve()),
            "--init",
            "--quiet",
            "--tcl-version",
            args.version,
        ]
    provenance = {
        "subject_commit": subprocess.check_output(
            ["git", "rev-parse", "HEAD"], cwd=root, text=True
        ).strip(),
        "source_root": str(source),
        "version": args.version,
        "init_sha256": sha256(library / "init.tcl"),
        "tcltest_sha256": sha256(library / "tcltest/tcltest.tcl"),
        "overlay_sha256": sha256(overlay),
        "runner_sha256": sha256(Path(__file__)),
        "method": "C/runtime stdin configure+source; VM run_test original file",
        "engines": engines,
        "binary_sha256": {k: sha256(Path(v[0])) for k, v in engines.items()},
        "timeout_seconds": args.timeout,
        "match": args.match,
        "backend_constraints_applied": not args.no_overlay,
        "runner_source_snapshot": "runner-source.txt",
        "started_utc": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
    }
    (output / "runner-source.txt").write_bytes(Path(__file__).read_bytes())
    (output / "provenance.json").write_text(json.dumps(provenance, indent=2) + "\n")

    def run_one(engine, stem):
        testfile = source / "tests" / (stem + ".test")
        destination = output / engine / stem
        destination.mkdir(parents=True, exist_ok=True)
        if not testfile.is_file():
            return {"engine": engine, "stem": stem, "status": "missing"}
        wrapper = destination / "input.tcl"
        match = (
            ""
            if args.match is None
            else "::tcltest::configure -match " + tcl_word(args.match) + "\n"
        )
        wrapper.write_text(
            "if {[catch {interp debug {} -frame 1} frame_debug]} {puts [list NH_FRAME_DEBUG unavailable $frame_debug]} else {puts [list NH_FRAME_DEBUG enabled]}\n"
            + "package require tcltest\nnamespace import -force ::tcltest::*\n"
            + "puts [list NH_HARNESS library [info library] tcl [package present Tcl] tcltest [package present tcltest] loader [package ifneeded tcltest [package present tcltest]]]\n"
            + "if {[file normalize [info library]] ne [file normalize "
            + tcl_word(library)
            + "]} {error {wrong upstream library}}\n"
            + "::tcltest::configure -verbose {body error start skip}\n"
            + match
            + ("" if args.no_overlay else "source " + tcl_word(overlay) + "\n")
            + "source "
            + tcl_word(testfile)
            + "\n"
        )
        command = ["timeout", "--kill-after=5", str(args.timeout), *engines[engine]]
        stdin = wrapper.read_bytes()
        if engine == "vm":
            command.append(str(testfile))
            if args.match:
                command.extend(["--match", args.match])
            stdin = None
        env = os.environ.copy()
        env["TCL_LIBRARY"] = str(library)
        env.pop("TCL_BACKEND_CONSTRAINTS", None)
        if engine == "vm":
            if not args.no_overlay:
                env["TCL_BACKEND_CONSTRAINTS"] = str(overlay)
            env["TCL_TEST_VERBOSE"] = "1"
        start = time.monotonic()
        with tempfile.TemporaryDirectory(prefix="name-suite-") as cwd:
            result = subprocess.run(
                command, input=stdin, env=env, cwd=cwd, capture_output=True, check=False
            )
        elapsed = round(time.monotonic() - start, 3)
        (destination / "stdout.txt").write_bytes(result.stdout)
        (destination / "stderr.txt").write_bytes(result.stderr)
        stdout = result.stdout.decode("utf-8", errors="replace")
        summary = SUMMARY.findall(stdout)
        record = {
            "engine": engine,
            "stem": stem,
            "exit_code": result.returncode,
            "seconds": elapsed,
            "command": command,
            "input_sha256": sha256(wrapper),
            "test_sha256": sha256(testfile),
            "bootstrap_seen": "NH_HARNESS" in stdout,
            "bootstrap_record": "separate VM startup probe"
            if engine == "vm"
            else "stdout NH_HARNESS",
            "failed_ids": sorted(set(FAILED.findall(stdout))),
            "last_started_id": (STARTED.findall(stdout) or [None])[-1],
        }
        if result.returncode in (124, 137):
            record["status"] = "timeout"
        elif result.returncode != 0:
            record["status"] = "process-error"
        elif not summary:
            record["status"] = "no-summary"
        else:
            total, passed, skipped, failed = map(int, summary[-1])
            record.update(total=total, passed=passed, skipped=skipped, failed=failed)
            record["status"] = (
                "fail" if failed else ("pass" if passed else "no-executed-tests")
            )
        if summary and "total" not in record:
            record["partial_summary"] = list(map(int, summary[-1]))
        (destination / "record.json").write_text(json.dumps(record, indent=2) + "\n")
        print(
            json.dumps(
                {
                    k: record[k]
                    for k in (
                        "engine",
                        "stem",
                        "status",
                        "seconds",
                        "failed_ids",
                        "last_started_id",
                    )
                }
            ),
            flush=True,
        )
        return record

    jobs = [(engine, stem) for stem in (args.stem or STEMS) for engine in engines]
    records = []
    with concurrent.futures.ThreadPoolExecutor(max_workers=args.workers) as executor:
        futures = [executor.submit(run_one, *job) for job in jobs]
        for future in concurrent.futures.as_completed(futures):
            records.append(future.result())
            (output / "records.json").write_text(
                json.dumps(
                    sorted(records, key=lambda x: (x["stem"], x["engine"])), indent=2
                )
                + "\n"
            )
    print(
        "Captured",
        len(records),
        "records; inspect statuses and counts, not the capture process exit.",
    )


if __name__ == "__main__":
    main()
