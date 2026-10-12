#!/usr/bin/env python3
"""Capture actual warm Bytecode retention after enabling a native command limit."""

import fcntl
import hashlib
import json
import subprocess
import tempfile
from pathlib import Path

FIXTURE = Path(__file__).resolve().parent
REPOSITORY = FIXTURE.parents[5]
SOURCE = FIXTURE / "probe.c"
VERSIONS = ("8.6.18", "9.0.4", "9.1.0")


def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def capture(version: str, binaries: Path) -> dict:
    tree = REPOSITORY / "tmp" / f"tcl{version}"
    release = ".".join(version.split(".")[:2])
    archive = tree / "unix" / f"libtcl{release}.a"
    binary = binaries / f"probe-{version}"
    compiler = [
        "cc",
        "-std=c99",
        "-Wl,--wrap=TclCompileArraySetCmd",
        f"-I{tree / 'generic'}",
        f"-I{tree / 'unix'}",
        str(SOURCE),
        str(archive),
        "-lm",
        "-ldl",
        "-lpthread",
        "-lz",
        "-o",
        str(binary),
    ]
    compilation = subprocess.run(compiler, capture_output=True, check=False, timeout=60)
    record = {
        "version": version,
        "compile_argv": compiler,
        "compile_status": compilation.returncode,
        "compile_stdout_hex": compilation.stdout.hex(),
        "compile_stderr_hex": compilation.stderr.hex(),
        "source_sha256": digest(SOURCE),
        "archive_sha256": digest(archive),
        "header_sha256": digest(tree / "generic" / "tclInt.h"),
    }
    if compilation.returncode == 0:
        result = subprocess.run(
            [str(binary)], capture_output=True, check=False, timeout=60
        )
        record.update(
            {
                "argv": [str(binary)],
                "status": result.returncode,
                "stdout_hex": result.stdout.hex(),
                "stderr_hex": result.stderr.hex(),
                "binary_sha256": digest(binary),
            }
        )
        (FIXTURE / f"{version}.tsv").write_bytes(result.stdout)
    return record


def main() -> None:
    with (
        tempfile.TemporaryDirectory(prefix="2286-native-warm-limit-") as binaries,
        Path("/workspace/.proofs/2286-test-slot-1.lock").open("a") as lock,
    ):
        fcntl.flock(lock, fcntl.LOCK_EX)
        records = [capture(version, Path(binaries)) for version in VERSIONS]
    (FIXTURE / "manifest.json").write_text(json.dumps(records, indent=2) + "\n")
    for record in records:
        print(
            record["version"],
            record.get("status", record["compile_status"]),
            bytes.fromhex(record.get("stdout_hex", "")).decode().strip(),
        )


if __name__ == "__main__":
    main()
