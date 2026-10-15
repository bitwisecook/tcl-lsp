#!/usr/bin/env python3
"""Capture original SDK helper source providers without modifying their files."""
import argparse
import datetime
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def execute(command, cwd, output, label):
    start = datetime.datetime.now(datetime.timezone.utc).isoformat()
    try:
        completed = subprocess.run(command, cwd=cwd, capture_output=True, timeout=30, check=False)
        stdout, stderr = completed.stdout, completed.stderr
        status = {"exit": completed.returncode, "completion": "process completed"}
    except subprocess.TimeoutExpired as error:
        stdout, stderr = error.stdout or b"", error.stderr or b""
        status = {"exit": None, "completion": "TIMEOUT after unchanged 30-second process budget"}
    (output / f"{label}.stdout").write_bytes(stdout)
    (output / f"{label}.stderr").write_bytes(stderr)
    return {"command": command, "cwd": str(cwd), "started_utc": start, **status,
            "stdout_sha256": sha(output / f"{label}.stdout"),
            "stderr_sha256": sha(output / f"{label}.stderr")}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--provider", required=True, type=Path)
    args = parser.parse_args()
    provider = args.provider.resolve()
    output = Path(__file__).resolve().parent
    if (output / "manifest.json").exists():
        raise SystemExit("Capture already exists; copy exact inputs into a fresh capture directory")
    upstream = json.loads((output / "upstream.json").read_text())
    paths = sorted({p for pattern in ["*.c", "*.h", "*.tcl"] for p in provider.glob(pattern)})
    paths += [provider / name for name in ["libjim.a", "jimsh", "Makefile", "config.log",
              "configure", "auto.def", "tests/testing.tcl", "tests/namespace.test"]]
    before = {str(path.relative_to(provider)): sha(path) for path in paths}
    manifest = {"question": "Which exact names, formals, body bytes, filename and first line "
                "does original Jim load for its nine namespace helpers through static "
                "initialization versus explicit original nshelper.tcl source?",
                "provider_revision": upstream["commit"]["sha"], "provider": str(provider),
                "fresh_upstream_receipt_sha256": sha(output / "upstream.json"),
                "failed_metadata_setup_sha256": sha(output / "failed-metadata-setup.json"),
                "inputs": {name: sha(output / name) for name in ["probe.c", "capture.py"]},
                "provider_before_sha256": before,
                "harness": "Exact retained SDK C observer calls original core/namespace/static "
                "initializers, original Jim_EvalFile, Jim_GetCommand and Jim_GetSourceInfo. "
                "Independent original jimsh tests/namespace.test uses its original testing.tcl "
                "and tcltest.tcl; no tcltest overlay or replacement implementation.",
                "environment": {name: os.environ.get(name) for name in ["CC", "CFLAGS", "LDFLAGS", "JIMLIB"]}}
    manifest["local_revision"] = execute(["git", "rev-parse", "HEAD"], provider, output, "local-revision")
    manifest["tracked_provider_status"] = execute(["git", "status", "--porcelain", "--untracked-files=no"], provider, output, "provider-status")
    manifest["compiler"] = execute(["cc", "--version"], provider, output, "compiler")
    manifest["version"] = execute([str(provider / "jimsh"), "-e", "puts [info version]"], provider, output, "version")
    with tempfile.TemporaryDirectory(prefix="jim-helper-provider-") as scratch:
        executable = str(Path(scratch) / "probe")
        manifest["compile"] = execute(["cc", "-std=c99", f"-I{provider}", str(output / "probe.c"),
            str(provider / "libjim.a"), "-lm", "-lssl", "-lcrypto", "-lz", "-ldl", "-o", executable], provider, output, "compile")
        if manifest["compile"]["exit"] == 0:
            manifest["executable_sha256"] = sha(Path(executable))
            manifest["probe"] = execute([executable, str(provider / "nshelper.tcl")], provider, output, "probe")
    manifest["original_namespace_suite"] = execute([str(provider / "jimsh"), "tests/namespace.test"], provider, output, "namespace-suite")
    manifest["provider_after_sha256"] = {str(path.relative_to(provider)): sha(path) for path in paths}
    manifest["provider_drift"] = {name: digest for name, digest in manifest["provider_after_sha256"].items() if digest != before[name]}
    manifest["limitations"] = [
        "Original pinned build and exact observer cases only; no Runtime or VM execution acceptance.",
        "Source metadata exposes filename and first line, not a native source-buffer byte-offset receipt.",
        "Raw source file loading and static embedded-source loading are distinct provider purposes.",
        "Selected provider/build/archive/executable hashes are not a complete system toolchain or dynamic-library closure.",
        "No statement about C Tcl/F5, universal static extension sets, helper behavior, tracing or final-free lifecycle.",
    ]
    (output / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")


if __name__ == "__main__":
    main()
