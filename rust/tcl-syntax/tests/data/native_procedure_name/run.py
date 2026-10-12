import fcntl
import hashlib
import json
import pathlib
import subprocess
import time
from contextlib import ExitStack

root = pathlib.Path("/workspace/.proofs/2286-command-procedure-name")
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
    "namespace eval : {proc p {} {return [namespace current]}; p}",
    "namespace eval : {set code [catch {proc : {} {return colon}} result]; list $code $result}",
    r"namespace eval : {set code [catch {proc : \{ {return colon}} result]; list $code $result}",
]
sources = cases
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
