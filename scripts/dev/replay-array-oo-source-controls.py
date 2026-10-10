#!/usr/bin/env python3
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Reconfirm exact array/OO source controls against explicitly supplied builds."""

import argparse
import fcntl
import hashlib
import json
import os
import subprocess
from contextlib import contextmanager
from pathlib import Path

REPOSITORY = Path(__file__).resolve().parents[2]
FIXTURE = (
    REPOSITORY / "rust/tcl-registry/tests/data/native_array_and_oo_source_controls"
)
VERSIONS = ["8.4.20", "8.5.19", "8.6.18", "9.0.4", "9.1.0", "jim"]


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


@contextmanager
def native_slot(directory):
    """Share the two provider slots used by the other maintained probes."""
    directory.mkdir(parents=True, exist_ok=True)
    while True:
        for number in range(2):
            with (directory / f"2286-test-slot-{number}.lock").open("a") as lock:
                try:
                    fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
                except BlockingIOError:
                    continue
                yield
                return
        # A blocking shared lock lets another process run without busy polling.
        with (directory / "2286-test-slot-0.lock").open("a") as lock:
            fcntl.flock(lock, fcntl.LOCK_EX)
            yield
            return


def retained_inputs():
    rows = json.loads((FIXTURE / "receipt.json").read_text())
    if len(rows) != 6:
        raise ValueError("the retained corpus must include all six provider receipts")
    for version, row in zip(VERSIONS, rows):
        directory = FIXTURE / version
        if json.loads((directory / "receipt.json").read_text()) != row:
            raise ValueError(f"provider receipt differs from aggregate: {version}")
        for field, leaf in [("probe_sha256", "probe.c")]:
            if digest(FIXTURE / leaf) != row[field]:
                raise ValueError(f"retained input bytes changed: {leaf}")
        for field, leaf in [
            ("stdout_sha256", "stdout.tsv"),
            ("stderr_sha256", "stderr"),
        ]:
            if digest(directory / leaf) != row[field]:
                raise ValueError(f"retained stream bytes changed: {version}/{leaf}")
        if (directory / "stdout.tsv").read_text().splitlines() != row["rows"]:
            raise ValueError(f"embedded rows differ from original stream: {version}")
    return rows


def replay(args, originals):
    destination = args.output.resolve()
    if destination == FIXTURE or destination.is_relative_to(FIXTURE):
        raise ValueError("new captures must not replace the retained fixture")
    destination.mkdir(parents=True, exist_ok=False)
    captures = []
    for version in args.providers:
        original = originals[VERSIONS.index(version)]
        if version == "jim":
            source = args.jim_root.resolve()
            header, library = source / "jim.h", source / "libjim.a"
            flags = ["-DJIM_PROBE=1", "-I" + str(source)]
            libraries = ["-lm", "-lssl", "-lcrypto", "-lz", "-ldl"]
        else:
            source = args.tcl_root.resolve() / ("tcl" + version)
            header = source / "generic/tcl.h"
            library = (
                source / "unix" / ("libtcl" + ".".join(version.split(".")[:2]) + ".a")
            )
            flags = ["-I" + str(header.parent), "-I" + str(source / "unix")]
            libraries = ["-lm", "-ldl", "-lpthread", "-lz"]
        if (
            digest(header) != original["header_sha256"]
            or digest(library) != original["library_sha256"]
        ):
            raise ValueError(
                f"selected build differs from the captured header/library: {version}"
            )
        directory = destination / version
        directory.mkdir()
        executable = directory / "probe"
        command = [
            args.compiler,
            "-std=c99",
            *flags,
            str(FIXTURE / "probe.c"),
            str(library),
            *libraries,
            "-o",
            str(executable),
        ]
        with native_slot(args.lock_root):
            compiled = subprocess.run(
                command, capture_output=True, timeout=60, check=False
            )
        (directory / "compile.stdout").write_bytes(compiled.stdout)
        (directory / "compile.stderr").write_bytes(compiled.stderr)
        if compiled.returncode:
            raise ValueError(
                f"compile failed; exact separate streams retained: {directory}"
            )
        environment = os.environ.copy()
        if version != "jim":
            environment["TCL_LIBRARY"] = str(source / "library")
        with native_slot(args.lock_root):
            result = subprocess.run(
                [str(executable)],
                env=environment,
                capture_output=True,
                timeout=60,
                check=False,
            )
        (directory / "stdout.tsv").write_bytes(result.stdout)
        (directory / "stderr").write_bytes(result.stderr)
        row = dict(
            provider=original["provider"],
            probe_sha256=digest(FIXTURE / "probe.c"),
            header_sha256=digest(header),
            library_sha256=digest(library),
            compile_command=command,
            compile_exit=compiled.returncode,
            executable_sha256=digest(executable),
            process_exit=result.returncode,
            stdout_sha256=digest(directory / "stdout.tsv"),
            stderr_sha256=digest(directory / "stderr"),
            rows=result.stdout.decode().splitlines(),
        )
        row["matches_original"] = (
            result.returncode == original["process_exit"]
            and row["stdout_sha256"] == original["stdout_sha256"]
            and row["stderr_sha256"] == original["stderr_sha256"]
        )
        (directory / "receipt.json").write_text(json.dumps(row, indent=2) + "\n")
        captures.append(row)
        (destination / "receipt.json").write_text(json.dumps(captures, indent=2) + "\n")
        if not row["matches_original"]:
            raise ValueError(
                f"new process/streams differ from the retained capture: {version}"
            )
        print(f"{version}: exact {len(row['rows'])} source/result rows match")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--verify-only",
        action="store_true",
        help="Inspect retained digests without launching a compiler or provider.",
    )
    parser.add_argument(
        "--tcl-root",
        type=Path,
        help="Parent of the five explicit tcl<release> source/build directories.",
    )
    parser.add_argument(
        "--jim-root", type=Path, help="Explicit captured Jim source/build directory."
    )
    parser.add_argument("--compiler", default="cc")
    parser.add_argument("--providers", nargs="+", choices=VERSIONS, default=VERSIONS)
    parser.add_argument(
        "--output", type=Path, help="New, nonexisting capture directory."
    )
    parser.add_argument("--lock-root", type=Path, default=Path("/workspace/.proofs"))
    args = parser.parse_args()
    try:
        original = retained_inputs()
        if args.verify_only:
            print(
                "Six retained input/receipt/stream associations verified; no native execution."
            )
            return
        if (
            args.output is None
            or any(v != "jim" for v in args.providers)
            and args.tcl_root is None
            or "jim" in args.providers
            and args.jim_root is None
        ):
            parser.error(
                "replay requires --output and the explicit source/build root for each selected provider"
            )
        replay(args, original)
    except (ValueError, OSError, subprocess.TimeoutExpired) as error:
        parser.exit(1, str(error) + "\n")


if __name__ == "__main__":
    main()
