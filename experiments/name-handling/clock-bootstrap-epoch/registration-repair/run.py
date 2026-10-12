#!/usr/bin/env python3
"""A discrete C8.6 registration repair question; never overwrites its parent."""
import hashlib
import json
import os
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[4]
HERE = Path(__file__).resolve().parent
TREE = ROOT / "tmp/tcl8.6.18"
ORACLE = Path("/tmp/name-clock-registration-repair-c86")
RUNNER = Path(os.environ.get("CLOCK_REPAIR_RUNNER", "/tmp/name-clock-registration-repair-runner-final"))
ENV = dict(os.environ, TCL_LIBRARY=str(TREE / "library"))


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def retain(name, argv, input_path=None):
    try:
        result = subprocess.run(argv, env=ENV, capture_output=True, timeout=30)
        code, timed_out, stdout, stderr = result.returncode, False, result.stdout, result.stderr
    except subprocess.TimeoutExpired as result:
        code, timed_out, stdout, stderr = None, True, result.stdout or b"", result.stderr or b""
    (HERE / (name + ".stdout.txt")).write_bytes(stdout)
    (HERE / (name + ".stderr.txt")).write_bytes(stderr)
    return dict(case=name, argv=argv, exit_code=code, timed_out=timed_out,
                binary_sha256=sha(Path(argv[0])),
                input_path=str(input_path) if input_path else None,
                input_sha256=sha(input_path) if input_path else None,
                TCL_LIBRARY=ENV["TCL_LIBRARY"])


compile_argv = ["cc", "-DHAVE_UNISTD_H=1", "-I" + str(TREE / "unix"),
                "-I" + str(TREE / "generic"), str(HERE / "probe.c"),
                str(TREE / "unix/libtcl8.6.a"), "-lm", "-ldl", "-lpthread", "-lz",
                "-o", str(ORACLE)]
built = subprocess.run(compile_argv, capture_output=True, timeout=30)
(HERE / "compile.stdout.txt").write_bytes(built.stdout)
(HERE / "compile.stderr.txt").write_bytes(built.stderr)
if built.returncode:
    raise SystemExit(built.returncode)
runs = []
for member in ["fresh", "tcl::clock::add", "tcl::clock::format", "tcl::clock::scan",
               "tcl::clock::seconds", "tcl::clock::getenv", "tcl::mathfunc::min", "tcl::mathfunc::max"]:
    label = member.replace("::", "-")
    runs.append(retain("c-" + label, [str(ORACLE), "::" + member]))
runs.append(retain("c-init", [str(ORACLE), "init"]))
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
    argv = [str(RUNNER), "--quiet", "--tcl-version", "8.6"]
    if init:
        argv.append("--init")
    runs.append(retain("runtime-" + name, argv + [str(path)], path))
sources = [HERE / "probe.c", HERE / "run.py", TREE / "generic/tclInt.h",
           TREE / "generic/tclClock.c", TREE / "generic/tclEnsemble.c",
           TREE / "generic/tclBasic.c", TREE / "library/init.tcl", TREE / "unix/libtcl8.6.a"]
(HERE / "records.json").write_text(json.dumps(dict(
    question="Does C8.6 clock registration repair preserve authentic deferred maps and known compiler currency; what still prevents matching real init?",
    compile_argv=compile_argv, compile_exit_code=built.returncode,
    source_sha256={str(path): sha(path) for path in sources},
    runtime_selected_source_snapshots="build-selected-sources.json",
    runs=runs), indent=2) + "\n")
for row in runs:
    print(row["case"], row["exit_code"], row["timed_out"])
