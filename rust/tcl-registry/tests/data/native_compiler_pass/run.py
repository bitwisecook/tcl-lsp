from pathlib import Path
import hashlib, subprocess, json, fcntl, os

p = Path("/workspace/.proofs/2286-command-compiler-pass-native")
records = []
with open("/workspace/.proofs/2286-test-slot-1.lock", "a") as lock:
    fcntl.flock(lock, fcntl.LOCK_EX)
    for v in ["8.4.20", "8.5.19", "8.6.18", "9.0.4", "9.1.0"]:
        tree = Path("/workspace/tcl-lsp/tmp") / ("tcl" + v)
        binary = p / ("probe-" + v)
        archive = tree / "unix" / ("libtcl" + v[:3] + ".a")
        argv = [
            "cc",
            "-Wl,--wrap=TclCompileArraySetCmd",
            "-I" + str(tree / "generic"),
            "-I" + str(tree / "unix"),
            str(p / "probe.c"),
            str(archive),
            "-lm",
            "-ldl",
            "-lpthread",
            "-lz",
            "-o",
            str(binary),
        ]
        c = subprocess.run(argv, capture_output=True, timeout=60)
        r = {
            "version": v,
            "compile_argv": argv,
            "compile_status": c.returncode,
            "source_sha256": hashlib.sha256((p / "probe.c").read_bytes()).hexdigest(),
            "archive_sha256": hashlib.sha256(archive.read_bytes()).hexdigest(),
        }
        if c.returncode == 0:
            a = subprocess.run(
                [str(binary)],
                env={**os.environ, "TCL_LIBRARY": str(tree / "library")},
                capture_output=True,
                timeout=60,
            )
            (p / (v + ".tsv")).write_bytes(a.stdout)
            (p / (v + ".stderr")).write_bytes(a.stderr)
            r.update(
                status=a.returncode,
                stdout_hex=a.stdout.hex(),
                stderr_hex=a.stderr.hex(),
                binary_sha256=hashlib.sha256(binary.read_bytes()).hexdigest(),
            )
            print(v, a.returncode, a.stdout.decode())
        else:
            print(v, c.stderr.decode())
        records.append(r)
(p / "manifest.json").write_text(json.dumps(records, indent=2) + "\n")
