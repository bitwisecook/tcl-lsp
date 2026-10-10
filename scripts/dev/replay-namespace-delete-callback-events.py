#!/usr/bin/env python3
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Verify or replay the exact namespace deletion callback event control."""

import argparse
import hashlib
import json
import subprocess
from pathlib import Path

VERSIONS = ("8.4.20", "8.5.19", "8.6.18", "9.0.4", "9.1.0")
CORPUS = Path("rust/tcl-syntax/tests/data/native_namespace_delete_callback_events")


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def verify(corpus, version):
    folder = corpus / version
    record = json.loads((folder / "receipt.json").read_text())
    if record["provider"] != version:
        raise ValueError("selected provider association differs")
    if digest(corpus / "probe.c") != record["probe_sha256"]:
        raise ValueError("original probe bytes changed")
    for filename, field in [("stdout.tsv", "stdout_sha256"), ("stderr", "stderr_sha256")]:
        if digest(folder / filename) != record[field]:
            raise ValueError("original stream bytes changed: " + filename)
    if record["compile_exit"] != 0 or record["process_exit"] != 0:
        raise ValueError("original compile/process did not complete")
    events = [json.loads(line) for line in (folder / "stderr").read_text().splitlines()]
    if len(events) != 10 or [event["ordinal"] for event in events] != list(range(1, 11)):
        raise ValueError("original event order differs")
    return record


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--corpus", type=Path, default=Path(__file__).resolve().parents[2] / CORPUS)
    parser.add_argument("--verify-only", action="store_true")
    parser.add_argument("--provider", choices=VERSIONS)
    parser.add_argument("--tcl-root", type=Path, help="Selected configured C source/build root containing generic/ and unix/.")
    parser.add_argument("--compiler", default="cc")
    parser.add_argument("--output", type=Path, help="New output directory; existing paths are refused.")
    args = parser.parse_args()
    corpus = args.corpus.resolve()
    if args.verify_only:
        for version in VERSIONS:
            verify(corpus, version)
        print(json.dumps({"verified_original_provider_associations": len(VERSIONS), "native_launches": 0}))
        return 0
    if not args.provider or not args.tcl_root or not args.output:
        parser.error("replay requires --provider, --tcl-root and a new --output")
    expected = verify(corpus, args.provider)
    source = args.tcl_root.resolve()
    library = source / "unix" / ("libtcl" + ".".join(args.provider.split(".")[:2]) + ".a")
    required = [(source / "generic/tcl.h", expected["header_sha256"]),
                (source / "unix/Makefile", expected["makefile_sha256"]),
                (library, expected["library_sha256"])]
    required += [(source / "generic" / name, checksum)
                 for name, checksum in expected["private_headers_sha256"].items()]
    for path, checksum in required:
        if digest(path) != checksum:
            raise ValueError("selected native build differs: " + str(path))
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    executable = output / "probe"
    command = [args.compiler, "-std=c99", "-D_GNU_SOURCE", *expected["configured_flags"],
               "-I" + str(source / "generic"), "-I" + str(source / "unix"),
               str(corpus / "probe.c"), str(library), "-lm", "-ldl", "-lpthread", "-lz",
               "-o", str(executable)]
    compiled = subprocess.run(command, capture_output=True, check=False, timeout=60)
    (output / "compile.stdout").write_bytes(compiled.stdout)
    (output / "compile.stderr").write_bytes(compiled.stderr)
    receipt = dict(provider=args.provider, probe_sha256=digest(corpus / "probe.c"),
                   compile_command=command, compile_exit=compiled.returncode,
                   required_build=[dict(path=str(path), sha256=checksum) for path, checksum in required])
    if compiled.returncode == 0:
        result = subprocess.run([str(executable)], capture_output=True, check=False, timeout=30)
        (output / "stdout.tsv").write_bytes(result.stdout)
        (output / "stderr").write_bytes(result.stderr)
        receipt.update(executable_sha256=digest(executable), process_exit=result.returncode,
                       stdout_sha256=hashlib.sha256(result.stdout).hexdigest(),
                       stderr_sha256=hashlib.sha256(result.stderr).hexdigest(),
                       matches_original=result.returncode == expected["process_exit"]
                       and result.stdout == (corpus / args.provider / "stdout.tsv").read_bytes()
                       and result.stderr == (corpus / args.provider / "stderr").read_bytes())
    (output / "receipt.json").write_text(json.dumps(receipt, indent=2) + "\n")
    print(json.dumps(receipt))
    return int(not receipt.get("matches_original", False))


if __name__ == "__main__":
    raise SystemExit(main())
