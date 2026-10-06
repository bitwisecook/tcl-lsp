import fcntl
import hashlib
import json
import os
import pathlib
import signal
import subprocess
import time
from contextlib import contextmanager

root = pathlib.Path("/workspace/tcl-lsp")
out = pathlib.Path(__file__).parent
sha = lambda p: hashlib.sha256(pathlib.Path(p).read_bytes()).hexdigest()


@contextmanager
def native_slot():
    while True:
        for number in range(2):
            with open(
                "/workspace/.proofs/2286-test-slot-" + str(number) + ".lock", "a"
            ) as lock:
                try:
                    fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
                except BlockingIOError:
                    continue
                yield
                return
        time.sleep(0.1)


def execute(command, native_env=None):
    with native_slot():
        start = time.monotonic()
        env = os.environ.copy()
        env.update(native_env or {})
        p = subprocess.Popen(
            command,
            cwd=root,
            env=env,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            start_new_session=True,
        )
        try:
            stdout, stderr = p.communicate(timeout=60)
            code = p.returncode
            timeout = False
        except subprocess.TimeoutExpired:
            os.killpg(p.pid, signal.SIGKILL)
            stdout, stderr = p.communicate()
            code = p.returncode
            timeout = True
        return {
            "command": command,
            "exit": code,
            "timeout": timeout,
            "seconds": time.monotonic() - start,
            "stdout": stdout.decode(errors="backslashreplace"),
            "stderr": stderr.decode(errors="backslashreplace"),
            "original_budget": 60,
            "global_slots": 2,
            "explicit_environment": native_env or {},
        }


base = root / "tmp/tcl9.1.0"
source = out / "probe.c"
binary = out / "probe"
library = base / "unix/libtcl9.1.a"
inputs = [
    source,
    library,
    base / "generic/tclOOProp.c",
    base / "generic/tclOODefineCmds.c",
    base / "generic/tclOOMethod.c",
    base / "generic/tclOOInt.h",
    base / "generic/tclInt.h",
    base / "generic/tcl.h",
]
compiled = execute(
    [
        "cc",
        "-I" + str(base / "generic"),
        "-I" + str(base / "unix"),
        str(source),
        str(library),
        "-lm",
        "-ldl",
        "-lpthread",
        "-lz",
        "-o",
        str(binary),
    ]
)
entry = {"compile": compiled, "sha256": {str(p): sha(p) for p in inputs}}
if compiled["exit"] == 0:
    entry["sha256"][str(binary)] = sha(binary)
    entry["run"] = execute([str(binary)], {"TCL_LIBRARY": str(base / "library")})
    (out / "native.tsv").write_text(entry["run"]["stdout"])
(out / "manifest.json").write_text(json.dumps(entry, indent=2) + "\n")
print(json.dumps(entry.get("run", compiled), indent=2))
