#!/usr/bin/env python3
import fcntl
import hashlib
import json
import os
import pathlib
import signal
import subprocess
import time
from contextlib import ExitStack

ROOT = pathlib.Path("/workspace/tcl-lsp")
OUT = pathlib.Path("/workspace/.proofs/2286-native-proc-recompile-roles-v3")
VERSIONS = ["8.4.20", "8.5.19", "8.6.18", "9.0.4", "9.1.0"]


def sha(path):
    with open(path, "rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def run_native(command, budget):
    with ExitStack() as lock_resources:
        slot = None
        while slot is None:
            for number in range(2):
                candidate = lock_resources.enter_context(
                    open(
                        "/workspace/.proofs/2286-test-slot-" + str(number) + ".lock",
                        "a",
                    )
                )
                try:
                    fcntl.flock(candidate, fcntl.LOCK_EX | fcntl.LOCK_NB)
                except BlockingIOError:
                    lock_resources.close()
                    continue
                slot = candidate
                break
            if slot is None:
                time.sleep(0.1)
        start = time.monotonic()
        process = subprocess.Popen(
            command,
            cwd=ROOT,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            start_new_session=True,
        )
        timed_out = False
        try:
            stdout, stderr = process.communicate(timeout=budget)
        except subprocess.TimeoutExpired:
            timed_out = True
            os.killpg(process.pid, signal.SIGKILL)
            stdout, stderr = process.communicate()
        finally:
            lock_resources.close()
    return (
        {
            "exit": process.returncode,
            "timeout": timed_out,
            "seconds": time.monotonic() - start,
        },
        stdout,
        stderr,
    )


manifest = {
    "source_sha256": sha(OUT / "probe.c"),
    "rows": [],
    "rust_comparisons": 0,
    "snapshot_window": "All identity/count/type/residency fields captured directly before result/string observers.",
    "modes": {
        "0": "sole declaration invalidation",
        "1": "real TclProcBody owner invalidation",
        "2": "active frame callback redefinition and recursive recompile",
        "3": "original dynamic-local name object owner invalidation",
    },
}
for version in VERSIONS:
    source = ROOT / "tmp" / ("tcl" + version)
    library = source / "unix" / ("libtcl" + ".".join(version.split(".")[:2]) + ".a")
    binary = OUT / ("probe-" + version)
    command = [
        "cc",
        "-O0",
        "-g",
        "-I",
        str(source / "generic"),
        "-I",
        str(source / "unix"),
        str(OUT / "probe.c"),
        str(library),
        "-lm",
        "-ldl",
        "-lpthread",
        "-lz",
        "-o",
        str(binary),
    ]
    build = subprocess.run(
        command, cwd=ROOT, capture_output=True, timeout=60, check=False
    )
    (OUT / (version + "-build.stdout")).write_bytes(build.stdout)
    (OUT / (version + "-build.stderr")).write_bytes(build.stderr)
    if build.returncode:
        raise RuntimeError(
            (version, build.returncode, build.stderr.decode(errors="replace"))
        )
    for mode in range(4):
        path = OUT / (version + "-" + str(mode))
        assert not pathlib.Path(str(path) + ".json").exists(), str(path)
        result, stdout, stderr = run_native([str(binary), str(mode)], 60)
        # Append extensions rather than replace the dotted engine version.
        pathlib.Path(str(path) + ".txt").write_bytes(stdout)
        pathlib.Path(str(path) + ".stderr").write_bytes(stderr)
        result.update(
            version=version,
            mode=mode,
            original_budget=60,
            snapshots=sum(line.startswith(b"S|") for line in stdout.splitlines()),
            completions=sum(line.startswith(b"R|") for line in stdout.splitlines()),
            command=[str(binary), str(mode)],
            source=str(source),
            library_sha256=sha(library),
            binary_sha256=sha(binary),
            proc_source_sha256=sha(source / "generic" / "tclProc.c"),
            variable_source_sha256=sha(source / "generic" / "tclVar.c"),
            stdout_sha256=hashlib.sha256(stdout).hexdigest(),
            stderr=stderr.decode(errors="replace"),
        )
        pathlib.Path(str(path) + ".json").write_text(
            json.dumps(result, indent=2) + "\n"
        )
        manifest["rows"].append(result)
        (OUT / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
        print(
            json.dumps(
                {
                    key: result[key]
                    for key in [
                        "version",
                        "mode",
                        "exit",
                        "timeout",
                        "snapshots",
                        "completions",
                        "stderr",
                    ]
                }
            ),
            flush=True,
        )
        if result["exit"] != 0 or result["timeout"] or stderr:
            raise RuntimeError((version, mode, result))
