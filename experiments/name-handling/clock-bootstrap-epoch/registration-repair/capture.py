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
"""Replay clock evidence into a required fresh destination."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[4]
HERE = Path(__file__).resolve().parent
TIMEOUT_SECONDS = 30


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def write_json(path, value):
    path.write_text(json.dumps(value, indent=2) + "\n")


def library_inventory(output, label, directory):
    files = sorted(path for path in directory.rglob("*") if path.is_file())
    receipt = output / (label + ".library-inventory.json")
    write_json(receipt, {str(path.relative_to(directory)): sha(path) for path in files})
    return dict(directory=str(directory), inventory=receipt.name,
                inventory_sha256=sha(receipt), file_count=len(files))


def retain(output, label, argv, environment, input_path=None, **context):
    before = sha(Path(argv[0]))
    try:
        result = subprocess.run(argv, cwd=ROOT, env=environment, capture_output=True,
                                timeout=TIMEOUT_SECONDS)
        code, timed_out = result.returncode, False
        stdout, stderr = result.stdout, result.stderr
    except subprocess.TimeoutExpired as result:
        code, timed_out = None, True
        stdout, stderr = result.stdout or b"", result.stderr or b""
    stdout_path = output / (label + ".stdout.txt")
    stderr_path = output / (label + ".stderr.txt")
    stdout_path.write_bytes(stdout)
    stderr_path.write_bytes(stderr)
    return dict(case=label, argv=argv, cwd=str(ROOT), exit_code=code,
                timed_out=timed_out, timeout_seconds=TIMEOUT_SECONDS,
                binary_sha256_before=before, binary_sha256_after=sha(Path(argv[0])),
                input_path=str(input_path) if input_path else None,
                input_sha256=sha(input_path) if input_path else None,
                stdout=stdout_path.name, stdout_sha256=sha(stdout_path),
                stderr=stderr_path.name, stderr_sha256=sha(stderr_path),
                environment={name: environment.get(name) for name in
                             ["TCL_LIBRARY", "TCLLIBPATH", "JIMLIB"]}, **context)


def registration(output, runner):
    tree = ROOT / "tmp/tcl8.6.18"
    environment = dict(os.environ, TCL_LIBRARY=str(tree / "library"))
    oracle = output / "probe-c86"
    argv = ["cc", "-DHAVE_UNISTD_H=1", "-I" + str(tree / "unix"),
            "-I" + str(tree / "generic"), str(HERE / "probe.c"),
            str(tree / "unix/libtcl8.6.a"), "-lm", "-ldl", "-lpthread", "-lz",
            "-o", str(oracle)]
    built = subprocess.run(argv, cwd=ROOT, capture_output=True, timeout=TIMEOUT_SECONDS)
    (output / "compile.stdout.txt").write_bytes(built.stdout)
    (output / "compile.stderr.txt").write_bytes(built.stderr)
    write_json(output / "compile-record.json", dict(
        argv=argv, exit_code=built.returncode, timeout_seconds=TIMEOUT_SECONDS,
        probe_source=str(HERE / "probe.c"), probe_source_sha256=sha(HERE / "probe.c"),
        static_library_sha256=sha(tree / "unix/libtcl8.6.a")))
    if built.returncode:
        return [], built.returncode
    library = library_inventory(output, "c86", tree / "library")
    rows = []
    for member in ["fresh", "tcl::clock::add", "tcl::clock::format", "tcl::clock::scan",
                   "tcl::clock::seconds", "tcl::clock::getenv", "tcl::mathfunc::min", "tcl::mathfunc::max"]:
        rows.append(retain(output, "c-" + member.replace("::", "-"),
                           [str(oracle), "::" + member], environment, library=library,
                           startup="fresh Tcl_CreateInterp; ordinary scalar mutation"))
    rows.append(retain(output, "c-init", [str(oracle), "init"], environment,
                       library=library, startup="fresh Tcl_CreateInterp followed by real Tcl_Init"))
    for name, path, init in [
        ("replacement", HERE.parent / "replacement-input.tcl", False),
        ("fresh", HERE.parent / "fresh-input.tcl", False),
        ("inventory", HERE / "inventory-input.tcl", False),
        ("map-dispatch", HERE / "map-dispatch-input.tcl", False),
        ("reading", HERE / "reading-input.tcl", False),
        ("mathfunc-min-replacement", HERE / "mathfunc-min-replacement-input.tcl", False),
        ("startup", HERE.parent / "startup-input.tcl", True),
        ("startup-inventory", HERE / "inventory-input.tcl", True),
        ("calendar", HERE / "calendar-input.tcl", False),
    ]:
        argv = [str(runner), "--quiet", "--tcl-version", "8.6"]
        if init:
            argv.append("--init")
        rows.append(retain(output, "runtime-" + name, argv + [str(path)], environment,
                           path, library=library, startup="matching real init" if init else "native core only"))
    return rows, 0


def compiled_foreach(output):
    input_path = HERE / "compiled-foreach-input.tcl"
    rows = []
    for release in ["8.4.20", "8.5.19", "8.6.18", "9.0.4", "9.1.0"]:
        tree = ROOT / ("tmp/tcl" + release)
        shell = tree / "unix/tclsh"
        environment = dict(os.environ, TCL_LIBRARY=str(tree / "library"))
        library = library_inventory(output, "c" + release, tree / "library")
        rows.append(retain(output, "c" + release, [str(shell), str(input_path)],
                           environment, input_path, release=release, library=library,
                           startup="stock shell; real matching Tcl_Init before unchanged script",
                           limits="One fresh process; script mode does not print the final result; no appended query or output marker."))
    tree = ROOT / "tmp/jimtcl-oracle"
    manifest = ROOT / "rust/tcl-test-support/jim-reference.txt"
    pin = dict(line.split("=", 1) for line in manifest.read_text().splitlines()
               if line and not line.startswith("#"))
    revision = (tree / ".git/HEAD").read_text().strip()
    if revision != pin["revision"]:
        raise RuntimeError("Jim checkout HEAD does not match the maintained pinned oracle")
    environment = dict(os.environ)
    environment.pop("TCL_LIBRARY", None)
    startup_sources = [tree / name for name in
                       ["jimsh.c", "initjimsh.tcl", "jim-config.h", "jimautoconf.h", "Makefile"]]
    receipt = output / "jim-startup-source-hashes.json"
    write_json(receipt, {str(path): sha(path) for path in startup_sources})
    rows.append(retain(output, "jim-" + pin["revision"][:7],
                       [str(tree / "jimsh"), str(input_path)], environment, input_path,
                       release=pin["patchlevel"], revision=pin["revision"],
                       maintained_pin=str(manifest), maintained_pin_sha256=sha(manifest),
                       library=dict(startup_source_hashes=receipt.name,
                                    startup_source_hashes_sha256=sha(receipt),
                                    configured_tree=str(tree)),
                       startup="stock Jim shell; embedded initjimsh and configured static extensions; no C Tcl init substitution",
                       limits="One fresh process executing the unchanged script; selected build-source hashes do not independently authenticate embedded bytes."))
    return rows


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output-dir", type=Path, required=True,
                        help="Required new destination; an existing path is refused.")
    parser.add_argument("--question", choices=["registration", "compiled-foreach", "all"],
                        default="registration")
    parser.add_argument("--runner", type=Path,
                        default=Path("/tmp/name-clock-registration-repair-runner-final"),
                        help="Existing native runner for registration controls; no Cargo build is performed.")
    args = parser.parse_args()
    output = args.output_dir.expanduser().resolve()
    if output.exists():
        parser.error("--output-dir must name a nonexistent fresh destination")
    runner = args.runner.expanduser().resolve()
    if args.question != "compiled-foreach" and not runner.is_file():
        parser.error("--runner must name an existing native runner")
    output.mkdir(parents=True)
    (output / "capture-source.py").write_bytes(Path(__file__).read_bytes())
    rows, status = [], 0
    if args.question in ["registration", "all"]:
        rows, status = registration(output, runner)
    if args.question in ["compiled-foreach", "all"]:
        rows.extend(compiled_foreach(output))
    write_json(output / "records.json", dict(
        question=args.question, capture_source=str(Path(__file__).resolve()),
        capture_source_sha256=sha(Path(__file__).resolve()), output_directory=str(output),
        capture_source_snapshot="capture-source.py",
        process_policy="Each scenario starts a separate fresh process; input files are never modified.",
        F5="Unmeasured; no C or Jim outcome supplies an F5 answer.", runs=rows))
    for row in rows:
        print(row["case"], row["exit_code"], row["timed_out"])
    return status


if __name__ == "__main__":
    raise SystemExit(main())
