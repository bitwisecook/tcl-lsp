#!/usr/bin/env python3
"""Compile and run the native private-header compiler epoch probe."""
import argparse
import hashlib
import json
import pathlib
import subprocess

parser = argparse.ArgumentParser()
parser.add_argument("--tcl-root", type=pathlib.Path, required=True)
parser.add_argument("--output", type=pathlib.Path, required=True)
args = parser.parse_args()
args.output.mkdir(parents=True, exist_ok=False)
source = pathlib.Path(__file__).with_name("probe.c")
sha = lambda path: hashlib.sha256(path.read_bytes()).hexdigest()
runs = []
for version in ["8.4.20", "8.5.19", "8.6.18", "9.0.4", "9.1.0"]:
    tree = args.tcl_root / ("tcl" + version)
    library = tree / "unix" / ("libtcl" + ".".join(version.split(".")[:2]) + ".a")
    binary = args.output / version
    command = ["cc", "-I" + str(tree / "unix"), "-I" + str(tree / "generic"),
               str(source), str(library), "-lm", "-ldl", "-lpthread", "-lz", "-o", str(binary)]
    compiled = subprocess.run(command, capture_output=True, timeout=60)
    (args.output / (version + "-compile.log")).write_bytes(compiled.stdout + compiled.stderr)
    row = dict(version=version, compile_command=command, compile_exit=compiled.returncode,
               library_sha256=sha(library), header_sha256=sha(tree / "generic" / "tclInt.h"))
    if compiled.returncode == 0:
        executed = subprocess.run([str(binary)], capture_output=True, timeout=60)
        (args.output / (version + ".tsv")).write_bytes(executed.stdout)
        (args.output / (version + "-stderr.log")).write_bytes(executed.stderr)
        row.update(exit=executed.returncode, binary_sha256=sha(binary),
                   observations=executed.stdout.decode().splitlines(), stderr=executed.stderr.decode())
    runs.append(row)
(args.output / "manifest.json").write_text(json.dumps(dict(source_sha256=sha(source), runs=runs), indent=2) + "\n")
raise SystemExit(any(row["compile_exit"] or row.get("exit", 1) or row.get("stderr") for row in runs))
