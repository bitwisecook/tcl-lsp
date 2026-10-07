#!/usr/bin/env python3
"""Verify portable scan receipts or replay their unchanged native C probe."""

import argparse
import concurrent.futures
import fcntl
import hashlib
import json
import shutil
import subprocess
import tempfile
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def checked_run(argv, tag, output, slots):
    while True:
        for slot in slots:
            lock = slot.open("a")
            try:
                fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
                break
            except BlockingIOError:
                lock.close()
        else:
            time.sleep(0.1)
            continue
        break
    try:
        result = subprocess.run(
            argv, capture_output=True, timeout=60, cwd=output, check=False
        )
    finally:
        fcntl.flock(lock, fcntl.LOCK_UN)
        lock.close()
    record = {
        "argv": argv,
        "status": result.returncode,
        "slot": str(slot),
        "timeout_seconds": 60,
    }
    for stream in ["stdout", "stderr"]:
        path = output / (tag + "." + stream)
        path.write_bytes(getattr(result, stream))
        record[stream] = path.name
        record[stream + "_sha256"] = sha(path)
    if result.returncode:
        raise RuntimeError(record)
    return record


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--verify-only", action="store_true")
    parser.add_argument(
        "--engine-root", action="append", default=[], metavar="VERSION=PATH"
    )
    parser.add_argument("--slot", action="append", default=[], metavar="PATH")
    parser.add_argument("--output", type=Path)
    parser.add_argument("--cc", default="cc")
    args = parser.parse_args()
    manifest = json.loads((HERE / "manifest.json").read_text())
    for name, expected in manifest["file_sha256"].items():
        if sha(HERE / name) != expected:
            raise RuntimeError("fixture hash mismatch: " + name)
    if args.verify_only:
        print(json.dumps({"fixture_files_verified": len(manifest["file_sha256"])}))
        return
    roots = {
        name: Path(path).resolve()
        for name, path in (item.split("=", 1) for item in args.engine_root)
    }
    if set(roots) != {engine["engine"] for engine in manifest["engines"]}:
        parser.error("supply both original C9 VERSION=PATH native engine roots")
    if len(args.slot) != 2:
        parser.error("supply the two global lock paths with --slot")
    slots = [Path(path).resolve() for path in args.slot]
    output = args.output or Path(tempfile.mkdtemp(prefix="native-scan-presenter-"))
    output.mkdir(parents=True, exist_ok=True)
    cc = str(Path(shutil.which(args.cc)).resolve())

    def capture(engine):
        version = engine["engine"]
        root = roots[version]
        library = root / "unix" / ("libtcl" + version[:3] + ".a")
        native = root / "unix/tclsh"
        if (
            sha(library) != engine["native_library_sha256"]
            or sha(native) != engine["native_elf_sha256"]
        ):
            raise RuntimeError("native library/engine hash mismatch: " + version)
        for table in ["headers", "native_sources"]:
            for recorded, expected in engine[table].items():
                path = root / Path(recorded).relative_to(
                    Path(recorded.split("/generic/")[0])
                )
                if sha(path) != expected:
                    raise RuntimeError(
                        "native source/header hash mismatch: " + str(path)
                    )
        binary = output / ("probe-" + version)
        argv = [
            cc,
            "-std=c11",
            "-DSTDC_HEADERS=1",
            "-DHAVE_UNISTD_H=1",
            "-I" + str(root / "generic"),
            "-I" + str(root / "unix"),
            str(HERE / "probe.c"),
            str(library),
            "-lm",
            "-ldl",
            "-lpthread",
            "-lz",
            "-o",
            str(binary),
        ]
        receipts = [
            checked_run(argv, version + "-compile", output, slots),
            checked_run([str(binary)], version + "-run", output, slots),
        ]
        for stream in ["stdout", "stderr"]:
            if (output / (version + "-run." + stream)).read_bytes() != (
                HERE / (version + "-run." + stream)
            ).read_bytes():
                raise RuntimeError(
                    "original native output mismatch: " + version + "/" + stream
                )
        return {
            "engine": version,
            "compiler_elf_sha256": sha(cc),
            "native_elf_sha256": sha(native),
            "probe_elf_sha256": sha(binary),
            "receipts": receipts,
        }

    with concurrent.futures.ThreadPoolExecutor(max_workers=2) as pool:
        captures = list(pool.map(capture, manifest["engines"]))
    (output / "replay-manifest.json").write_text(
        json.dumps({"engines": captures}, indent=2) + "\n"
    )
    print(json.dumps({"native_controls_compared": 2, "output": str(output)}))


if __name__ == "__main__":
    main()
