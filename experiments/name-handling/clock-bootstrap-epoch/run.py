#!/usr/bin/env python3
"""Retain the selected C8.6 registration/epoch question and frozen runtime controls."""
import hashlib
import json
import os
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[3]
HERE = Path(__file__).resolve().parent
TREE = ROOT / "tmp/tcl8.6.18"
RUNNER = Path("/tmp/name-native-c84-declared-cell-runner")
ORACLE = Path("/tmp/name-clock-bootstrap-epoch-c86")
ENV = dict(os.environ, TCL_LIBRARY=str(TREE / "library"))

def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

compile_argv = ["cc", "-DHAVE_UNISTD_H=1", "-I" + str(TREE / "unix"),
    "-I" + str(TREE / "generic"), str(HERE / "probe.c"),
    str(TREE / "unix/libtcl8.6.a"), "-lm", "-ldl", "-lpthread", "-lz",
    "-o", str(ORACLE)]
built = subprocess.run(compile_argv, capture_output=True, timeout=30)
(HERE / "compile.stdout.txt").write_bytes(built.stdout)
(HERE / "compile.stderr.txt").write_bytes(built.stderr)
if built.returncode:
    raise SystemExit(built.returncode)
records = []

def retain(name, argv, input_path=None):
    reached = subprocess.run(argv, env=ENV, capture_output=True, timeout=30)
    (HERE / (name + ".stdout.txt")).write_bytes(reached.stdout)
    (HERE / (name + ".stderr.txt")).write_bytes(reached.stderr)
    records.append(dict(case=name, argv=argv, exit_code=reached.returncode,
        timed_out=False, binary_sha256=sha(Path(argv[0])),
        input_path=str(input_path) if input_path else None,
        input_sha256=sha(input_path) if input_path else None,
        TCL_LIBRARY=ENV["TCL_LIBRARY"]))

for member in ["fresh", "tcl::clock::add", "tcl::clock::format", "tcl::clock::scan",
               "tcl::clock::seconds", "tcl::clock::getenv"]:
    retain("c-" + member.split("::")[-1], [str(ORACLE), "::" + member])
retain("c-init", [str(ORACLE), "init"])
for name, init in [("replacement", False), ("fresh", False), ("inventory", False), ("startup", True)]:
    input_path = HERE / (name + "-input.tcl")
    argv = [str(RUNNER), "--quiet", "--tcl-version", "8.6"]
    if init:
        argv.append("--init")
    retain("runtime-" + name, argv + [str(input_path)], input_path)
source_paths = [HERE / "probe.c", HERE / "run.py", TREE / "generic/tclInt.h",
    TREE / "generic/tclClock.c", TREE / "generic/tclEnsemble.c", TREE / "generic/tclBasic.c",
    TREE / "library/init.tcl", TREE / "unix/libtcl8.6.a",
    ROOT / "runtime/rust/src/cmd_clock.rs", ROOT / "runtime/rust/src/interp/native_compilation.rs",
    ROOT / "runtime/rust/src/interp/native_body_artifact.rs", ROOT / "runtime/rust/src/namespace.rs",
    ROOT / "rust/tcl-registry/src/commands/tcl/clock_.rs", ROOT / "rust/tcl-registry/src/native_procedure.rs",
    ROOT / "rust/tcl-registry/src/registry.rs"]
(HERE / "records.json").write_text(json.dumps(dict(
    scope="C Tcl 8.6.18 pre-init clock registrations and independent mutation epochs; frozen native runtime controls",
    compile_argv=compile_argv, compile_exit_code=built.returncode,
    source_sha256={str(path):sha(path) for path in source_paths}, runs=records), indent=2) + "\n")
for row in records:
    print(row["case"], row["exit_code"])
