#!/usr/bin/env python3
"""Capture original Jim SDK namespace canonical ownership and original suite."""

import argparse
import datetime
import hashlib
import json
from pathlib import Path
import subprocess
import tempfile


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def execute(command, cwd, output, label):
    started = datetime.datetime.now(datetime.timezone.utc).isoformat()
    result = subprocess.run(command, cwd=cwd, capture_output=True, timeout=30, check=False)
    (output / f"{label}.stdout").write_bytes(result.stdout)
    (output / f"{label}.stderr").write_bytes(result.stderr)
    return {"command": command, "cwd": str(cwd), "started_utc": started,
            "exit": result.returncode, "completion": "process completed",
            "stdout_sha256": digest(output / f"{label}.stdout"),
            "stderr_sha256": digest(output / f"{label}.stderr")}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--provider", type=Path, required=True)
    args = parser.parse_args()
    provider = args.provider.resolve()
    output = Path(__file__).resolve().parent
    upstream = json.loads((output / "upstream.json").read_text())
    sources = ["jim.h", "jim.c", "jim-namespace.c", "jim-config.h", "jimautoconf.h",
               "Makefile", "config.log", "configure", "auto.def", "nshelper.tcl",
               "tcltest.tcl", "tests/testing.tcl", "tests/namespace.test"]
    artifacts = ["libjim.a", "jimsh"]
    paths = sources + artifacts
    before = {name: digest(provider / name) for name in paths}
    manifest = {"question": "Does original no-argument namespace canonical return the actual "
                "current namespace object, and does a reused same-object lookup leave its "
                "String absent? Do absolute and explicit-prefix controls avoid that getter?",
                "provider_revision": upstream["sha"], "upstream_receipt_sha256": digest(output / "upstream.json"),
                "provider": str(provider), "provider_before_sha256": before,
                "probe_sha256": digest(output / "probe.c"), "capture_sha256": digest(Path(__file__)),
                "harness": "SDK Jim_EvalNamespace with original libjim.a; independently original "
                "jimsh tests/namespace.test -> tests/testing.tcl -> tcltest.tcl, no overlays"}
    manifest["compiler"] = execute(["cc", "--version"], provider, output, "compiler")
    manifest["version"] = execute([str(provider / "jimsh"), "-e", "puts [info version]"], provider, output, "version")
    with tempfile.TemporaryDirectory(prefix="jim-canonical-owner-") as scratch:
        executable = str(Path(scratch) / "probe")
        command = ["cc", "-std=c99", f"-I{provider}", str(output / "probe.c"),
                   str(provider / "libjim.a"), "-lm", "-lssl", "-lcrypto", "-lz", "-ldl",
                   "-o", executable]
        manifest["compile"] = execute(command, provider, output, "compile")
        if manifest["compile"]["exit"] == 0:
            manifest["executable_sha256"] = digest(Path(executable))
            manifest["probe"] = execute([executable], provider, output, "probe")
    manifest["original_namespace_suite"] = execute(
        [str(provider / "jimsh"), "tests/namespace.test"], provider, output, "namespace-suite")
    manifest["provider_after_sha256"] = {name: digest(provider / name) for name in paths}
    manifest["provider_drift"] = {name: actual for name, actual in manifest["provider_after_sha256"].items()
                                  if actual != before[name]}
    manifest["limitations"] = "Exact SDK and original namespace-suite observations only. Selected "
    manifest["limitations"] += "source/configuration/library hashes are not a complete system toolchain closure. "
    manifest["limitations"] += "No C Tcl/F5, Runtime/VM result, universal cache hit, namespace existence or lifecycle claim."
    (output / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")


if __name__ == "__main__":
    main()
