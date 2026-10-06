import hashlib
import json
import subprocess
from pathlib import Path

base = Path("/workspace/.proofs/2286-packages-dialects/package-native-completion")
repo = Path("/workspace/tcl-lsp")
index = base / "pkgIndex.tcl"
leaf = base / "leaf.tcl"
skip = base / "skip.tcl"
nested = base / "nested.tcl"
leaf.write_text("set ::leafSeen yes\n")
skip.write_text("set ::skipSeen yes\n")
nested.write_text("package provide child 1\n")
index.write_text(
    "package ifneeded parent 1 {source "
    + str(leaf)
    + "; source -nopkg "
    + str(skip)
    + "; package require child; source "
    + str(leaf)
    + "; package provide parent 1}\npackage ifneeded child 1 {source "
    + str(nested)
    + "}\n"
)
cases = [
    ("roster", "catch {package bad} m; set m"),
    ("help", "package -commands"),
    ("forget", "package provide probe 1; package forget probe; package provide probe"),
    ("preference", "list [package prefer] [package prefer l] [package prefer s]"),
    (
        "versions",
        "package ifneeded probe 2 SECOND; package ifneeded probe 1 FIRST; package ifneeded probe 1.00 REPLACED; list [package versions probe] [package ifneeded probe 1.0]",
    ),
    (
        "conflict",
        "package provide probe 1; catch {package provide probe 2} m; list $m $::errorCode",
    ),
    ("invalid", "catch {package vcompare 1 bad} m; list $m $::errorCode"),
    (
        "require",
        "package ifneeded probe 1 {package provide probe 1}; package require probe",
    ),
    (
        "stable",
        "package ifneeded probe 1 {package provide probe 1}; package ifneeded probe 2b1 {package provide probe 2b1}; package require probe",
    ),
    (
        "unknown",
        "proc discover args {set ::seen $args}; package unknown discover; catch {package require -exact missing 1}; set ::seen",
    ),
    (
        "files",
        "source "
        + str(index)
        + "; package require parent; list [package files parent] [package files child] [package files absent]",
    ),
    (
        "files-forget",
        "source "
        + str(index)
        + "; package require parent; set old [package files parent]; package forget parent; list $old [package files parent]",
    ),
    (
        "files-failure",
        "package ifneeded broken 1 {source "
        + str(leaf)
        + "; error STOP}; catch {package require broken}; package files broken",
    ),
    ("files-plain", "package provide plain 1; package files plain"),
]
engines = [
    ("8.4.20", "8.4"),
    ("8.5.19", "8.5"),
    ("8.6.18", "8.6"),
    ("9.0.4", "9.0"),
    ("9.1.0", "9.1"),
    ("Jim", "Jim"),
]
rows = []
for version, short in engines:
    exe = (
        Path("/tmp/2286-oracles/jimtcl/jimsh")
        if short == "Jim"
        else Path("/tmp/2286-oracles/bin/tclsh" + short)
    )
    lib = (
        Path("/tmp/2286-oracles/jimtcl/libjim.a")
        if short == "Jim"
        else repo / "tmp" / ("tcl" + version) / "unix" / ("libtcl" + short + ".a")
    )
    header = (
        Path("/tmp/2286-oracles/jimtcl/jim.h")
        if short == "Jim"
        else repo / "tmp" / ("tcl" + version) / "generic/tcl.h"
    )
    out = []
    for name, script in cases:
        # Captures result first. No GetVar observer is inserted after a completion.
        wrapper = (
            "set status [catch {" + script + "} result]; puts [list $status $result]\n"
        )
        proc = subprocess.run(
            [str(exe)],
            check=False,
            input=wrapper,
            text=True,
            capture_output=True,
            timeout=10,
        )
        out.append(
            {
                "case": name,
                "script": script,
                "exit": proc.returncode,
                "stdout": proc.stdout,
                "stderr": proc.stderr,
            }
        )
    rows.append(
        {
            "engine": version,
            "binary_sha256": hashlib.sha256(exe.read_bytes()).hexdigest(),
            "header_sha256": hashlib.sha256(header.read_bytes()).hexdigest(),
            "library_sha256": hashlib.sha256(lib.read_bytes()).hexdigest(),
            "rows": out,
        }
    )
(base / "manifest.json").write_text(
    json.dumps(
        {
            "engines": rows,
            "source_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
        },
        indent=2,
    )
    + "\n"
)
for e in rows:
    print(e["engine"])
    for r in e["rows"]:
        print(r["case"], r["stdout"].strip(), r["stderr"].strip())
