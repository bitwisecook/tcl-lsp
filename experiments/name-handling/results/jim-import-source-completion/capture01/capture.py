# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
# SPDX-License-Identifier: AGPL-3.0-or-later

"""Capture original Jim namespace Import Source completion and alias reports."""

import argparse
import datetime
import hashlib
import json
import pathlib
import subprocess


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def now():
    return datetime.datetime.now(datetime.timezone.utc).isoformat()


parser = argparse.ArgumentParser()
parser.add_argument("--sdk-dir", required=True, type=pathlib.Path)
parser.add_argument("--out", required=True, type=pathlib.Path)
args = parser.parse_args()
sdk = args.sdk_dir.resolve()
out = args.out.resolve()
out.mkdir(parents=True, exist_ok=False)
probe = pathlib.Path(__file__).resolve().with_name("probe.c")
(out / "probe.c").write_bytes(probe.read_bytes())
(out / "capture.py").write_bytes(pathlib.Path(__file__).read_bytes())


def command(argv, label, timeout=30):
    started = now()
    try:
        result = subprocess.run(argv, capture_output=True, timeout=timeout)
        stdout, stderr = result.stdout, result.stderr
        code, timed_out = result.returncode, False
    except subprocess.TimeoutExpired as error:
        stdout, stderr = error.stdout or b"", error.stderr or b""
        code, timed_out = None, True
    (out / (label + ".stdout")).write_bytes(stdout)
    (out / (label + ".stderr")).write_bytes(stderr)
    return {
        "argv": argv,
        "started_utc": started,
        "finished_utc": now(),
        "exit_code": code,
        "timed_out": timed_out,
        "stdout_sha256": sha(out / (label + ".stdout")),
        "stderr_sha256": sha(out / (label + ".stderr")),
    }


def original_suite():
    started = now()
    result = subprocess.run([str(sdk / "jimsh"), "tests/namespace.test"], cwd=sdk, capture_output=True, timeout=30)
    (out / "namespace-suite.stdout").write_bytes(result.stdout)
    (out / "namespace-suite.stderr").write_bytes(result.stderr)
    return {
        "argv": [str(sdk / "jimsh"), "tests/namespace.test"],
        "cwd": str(sdk), "started_utc": started, "finished_utc": now(),
        "exit_code": result.returncode, "timed_out": False,
        "stdout_sha256": sha(out / "namespace-suite.stdout"),
        "stderr_sha256": sha(out / "namespace-suite.stderr"),
        "harness": "Original tests/namespace.test with its original tests/testing.tcl and tests/tcltest.tcl; no replacement harness.",
    }


def provider():
    selected = {
        path.relative_to(sdk).as_posix(): sha(path)
        for path in sdk.rglob("*")
        if path.is_file()
        and ".git" not in path.relative_to(sdk).parts
        and (
            path.suffix in {".c", ".h", ".tcl", ".test", ".tm", ".a", ".so", ".sh"}
            or path.name in {"jimsh", "Makefile", "configure", "config.log"}
        )
    }
    for required in [
        "jimsh", "libjim.a", "jim.h", "jim.c", "jim-namespace.c", "jim-subcmd.c",
        "jim-subcmd.h", "nshelper.tcl", "jim-config.h", "Makefile",
    ]:
        if required not in selected:
            raise ValueError("Missing original provider input: " + required)
    return selected


manifest = {
    "started_utc": now(),
    "scope": "Original C API Import source completion and alias-prefix reports; no Runtime or VM execution.",
    "source": str(probe),
    "source_sha256": sha(probe),
    "driver_sha256": sha(pathlib.Path(__file__)),
    "sdk_dir": str(sdk),
    "provider_before_sha256": provider(),
}
manifest["upstream"] = command(
    ["git", "ls-remote", "https://github.com/msteveb/jimtcl.git", "HEAD"], "upstream"
)
manifest["local_revision"] = command(
    ["git", "-C", str(sdk), "rev-parse", "HEAD"], "local-revision"
)
manifest["local_status"] = command(
    ["git", "-C", str(sdk), "status", "--porcelain", "--untracked-files=no"], "local-status"
)
manifest["compiler"] = command(["cc", "--version"], "compiler")
checks = [manifest[key] for key in ["upstream", "local_revision", "local_status", "compiler"]]
manifest["setup_ready"] = all(row["exit_code"] == 0 and not row["timed_out"] for row in checks)
manifest["upstream_matches_local"] = (
    (out / "upstream.stdout").read_bytes().split()[0:1]
    == (out / "local-revision.stdout").read_bytes().split()[0:1]
)
if manifest["setup_ready"] and manifest["upstream_matches_local"]:
    executable = out / "probe"
    manifest["compile"] = command(
        ["cc", "-std=c99", "-I" + str(sdk), str(probe), str(sdk / "libjim.a"),
         "-lm", "-lssl", "-lcrypto", "-lz", "-ldl", "-o", str(executable)],
        "compile",
    )
    if manifest["compile"]["exit_code"] == 0:
        manifest["executable_sha256"] = sha(executable)
        manifest["execution"] = command([str(executable)], "execution", timeout=10)
        manifest["rows"] = len((out / "execution.stdout").read_bytes().splitlines())
        manifest["linked_libraries"] = command(["ldd", str(executable)], "linked-libraries")
        manifest["linked_library_sha256"] = {
            str(path.resolve()): sha(path.resolve())
            for word in (out / "linked-libraries.stdout").read_text().split()
            if word.startswith("/") and (path := pathlib.Path(word)).is_file()
        }
        manifest["original_namespace_suite"] = original_suite()
manifest["provider_after_sha256"] = provider()
manifest["provider_unchanged"] = manifest["provider_before_sha256"] == manifest["provider_after_sha256"]
manifest["source_unchanged"] = manifest["source_sha256"] == sha(probe)
manifest["finished_utc"] = now()
(out / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
print(json.dumps({key: manifest.get(key) for key in [
    "setup_ready", "upstream_matches_local", "compile", "execution", "rows",
    "provider_unchanged", "source_unchanged",
]}))
