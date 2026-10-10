import fcntl
import hashlib
import json
import pathlib
import subprocess
import time
from contextlib import ExitStack

root = pathlib.Path("/workspace/.proofs/2286-command-mathop-identity")
checkpoint = json.loads(
    pathlib.Path(
        "/workspace/.proofs/2286-root-main71-immutable/checkpoint.json"
    ).read_text()
)
engines = [
    ("tcl8.4", "TCL_LSP_TCLSH84"),
    ("tcl8.5", "TCL_LSP_TCLSH85"),
    ("tcl8.6", "TCL_LSP_TCLSH86"),
    ("tcl9.0", "TCL_LSP_TCLSH90"),
    ("tcl9.1", "TCL_LSP_TCLSH91"),
    ("jim", "TCL_LSP_JIMSH"),
]
cases = [
    "llength [info commands ::tcl::mathop::+]",
    "rename ::tcl::mathop::+ add; add 2 3",
    "interp alias {} aliasplus {} ::tcl::mathop::+; aliasplus 2 3",
    "namespace eval n {namespace path ::tcl::mathop; + 2 3}",
    "namespace eval n {namespace import ::tcl::mathop::+; + 2 3}",
    "set seen {}; proc watch {cmd op} {lappend ::seen [lindex $cmd 0]}; trace add execution ::tcl::mathop::+ enter watch; rename ::tcl::mathop::+ renamed; set sum [renamed 2 3]; list $sum $seen",
    "set events {}; proc number {v} {lappend ::events $v; return $v}; interp alias {} aliasplus {} ::tcl::mathop::+; set sum [aliasplus [number 2] [number 3]]; list $sum $events",
    "rename ::tcl::mathop::! {odd name}; catch {{odd name}} message; set message",
    "namespace import ::tcl::mathop::+; rename ::tcl::mathop::+ other; + 2 3",
    "rename ::tcl::mathop::+ original; proc ::tcl::mathop::+ args {return OVERRIDE}; list [original 2 3] [::tcl::mathop::+ 2 3]",
]
sources = [
    "if {![llength [info commands ::tcl::mathop::+]]} {set marker UNAVAILABLE} else {"
    + case
    + "}"
    for case in cases
]
(root / "cases.json").write_text(json.dumps(sources, indent=2) + "\n")
captures = []
for engine, variable in engines:
    binary = pathlib.Path(checkpoint["strict_environment"][variable])
    for case, source in enumerate(sources):
        wrapper = (
            "set code [catch {"
            + source
            + "} result]; binary scan $result H* bytes; puts [list $code $bytes]\n"
        )
        with ExitStack() as slot_files:
            slot = None
            while slot is None:
                for number in range(2):
                    candidate = slot_files.enter_context(
                        open(
                            "/workspace/.proofs/2286-test-slot-"
                            + str(number)
                            + ".lock",
                            "a",
                        )
                    )
                    try:
                        fcntl.flock(candidate, fcntl.LOCK_EX | fcntl.LOCK_NB)
                    except BlockingIOError:
                        candidate.close()
                        continue
                    slot = candidate
                    break
                if slot is None:
                    time.sleep(0.1)
            result = subprocess.run(
                [str(binary)],
                check=False,
                input=wrapper.encode(),
                capture_output=True,
                timeout=60,
            )
        captures.append(
            {
                "engine": engine,
                "case": case,
                "source": source,
                "exit": result.returncode,
                "stdout": result.stdout.decode(),
                "stderr": result.stderr.decode(),
                "binary": str(binary),
                "binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
            }
        )
(root / "manifest.json").write_text(
    json.dumps(
        {
            "source_sha256": hashlib.sha256(
                (root / "cases.json").read_bytes()
            ).hexdigest(),
            "captures": captures,
        },
        indent=2,
    )
    + "\n"
)
print(
    json.dumps(
        {
            "rows": len(captures),
            "failures": [row for row in captures if row["exit"] or row["stderr"]],
        }
    )
)
