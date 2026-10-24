# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook)
# SPDX-License-Identifier: AGPL-3.0-or-later

"""Hosted-only pinned Nextest source build and reporter smoke validation."""

import hashlib
import os
import shutil
import sys
import tempfile
from pathlib import Path

from process_owner import initialize, supervise

ROOT = Path(__file__).resolve().parents[3]
TOOL = ROOT / "scripts/dev/nextest-tool"
PINS = {}
for line in (TOOL / "pins.env").read_text().splitlines():
    if line and not line.startswith("#"):
        key, value = line.split("=", 1)
        PINS[key] = value


def main():
    if len(sys.argv) != 2 or sys.argv[1] != "--validate-only":
        raise SystemExit("usage: validate.py --validate-only")
    temp = Path(os.environ["RUNNER_TEMP"])
    free = shutil.disk_usage(temp).free
    if free < 2 * 1024**3:
        raise RuntimeError(
            f"need at least 2 GiB free under RUNNER_TEMP; found {free} bytes"
        )
    work = Path(tempfile.mkdtemp(prefix="nextest-validate-", dir=temp))
    initialize(work)
    print(f"private Nextest validation artifacts retained at {work}", flush=True)
    try:
        source = work / "source"
        source.mkdir()
        supervise(["git", "init", "-q", str(source)])
        supervise(
            [
                "git",
                "-C",
                str(source),
                "remote",
                "add",
                "origin",
                "https://github.com/nextest-rs/nextest.git",
            ]
        )
        supervise(
            [
                "git",
                "-C",
                str(source),
                "fetch",
                "--depth=1",
                "origin",
                f"refs/tags/{PINS['NEXTTEST_TAG']}:refs/tags/{PINS['NEXTTEST_TAG']}",
            ]
        )
        supervise(
            [
                "git",
                "-C",
                str(source),
                "checkout",
                "-q",
                "--detach",
                f"refs/tags/{PINS['NEXTTEST_TAG']}",
            ]
        )
        commit = supervise(["git", "-C", str(source), "rev-parse", "HEAD"]).strip()
        if commit != PINS["NEXTTEST_COMMIT"]:
            raise RuntimeError(f"unexpected upstream commit: {commit}")
        manifest = (source / "cargo-nextest/Cargo.toml").read_text()
        if f'version = "{PINS["NEXTTEST_VERSION"]}"' not in manifest:
            raise RuntimeError("Nextest package version mismatch")
        lock = hashlib.sha256((source / "Cargo.lock").read_bytes()).hexdigest()
        if lock != PINS["NEXTTEST_LOCK_SHA256"]:
            raise RuntimeError(f"Cargo.lock mismatch: {lock}")
        blob = supervise(
            [
                "git",
                "-C",
                str(source),
                "rev-parse",
                "HEAD:nextest-runner/src/reporter/structured/libtest.rs",
            ]
        ).strip()
        if blob != PINS["NEXTTEST_REPORTER_BLOB_SHA1"]:
            raise RuntimeError(f"reporter source blob mismatch: {blob}")
        patch = TOOL / "patches/libtest-ignore-lifecycle.patch"
        patch_hash = hashlib.sha256(patch.read_bytes()).hexdigest()
        if patch_hash != PINS["NEXTTEST_PATCH_SHA256"]:
            raise RuntimeError(f"patch hash mismatch: {patch_hash}")
        supervise(["git", "-C", str(source), "apply", "--check", str(patch)])
        supervise(["git", "-C", str(source), "apply", str(patch)])
        root = work / "install-root"
        target = work / "target"
        env = os.environ.copy()
        env["CARGO_TARGET_DIR"] = str(target)
        env["RUSTC_WRAPPER"] = ""
        env["RUSTC_WORKSPACE_WRAPPER"] = ""
        supervise(
            [
                "cargo",
                "install",
                "--locked",
                "--path",
                str(source / "cargo-nextest"),
                "--root",
                str(root),
                "--force",
                "--no-default-features",
                "--features",
                "default-no-update",
            ],
            env=env,
            seconds=900,
        )
        binary = root / "bin/cargo-nextest"
        version = supervise([str(binary), "nextest", "--version"])
        if f"cargo-nextest {PINS['NEXTTEST_VERSION']}" not in version:
            raise RuntimeError(f"installed version mismatch: {version.strip()}")
        env["NEXTEST_EXPERIMENTAL_LIBTEST_JSON"] = "1"
        reports = []
        for message_format in ("libtest-json-plus", "libtest-json"):
            for fixture, name in (("mixed", "mixed"), ("all-ignored", "all-ignored")):
                report = work / f"{message_format}-{name}.jsonl"
                no_tests = ["--no-tests=pass"] if fixture == "all-ignored" else []
                supervise(
                    [
                        str(binary),
                        "nextest",
                        "run",
                        *no_tests,
                        "--manifest-path",
                        str(TOOL / "fixtures" / fixture / "Cargo.toml"),
                        "--message-format",
                        message_format,
                        "--message-format-version",
                        "0.1",
                    ],
                    env=env,
                    report=report,
                    seconds=180,
                )
                reports.append(report)
        checker = TOOL / "check-ignored-report.py"
        supervise([sys.executable, str(checker), *[str(report) for report in reports]])
        empty = work / "invalid-empty.jsonl"
        empty.write_text("")
        malformed = work / "invalid-malformed.jsonl"
        malformed.write_text("{not-json\n")
        optimized_env = env.copy()
        optimized_env["PYTHONOPTIMIZE"] = "1"
        empty_log = supervise(
            [
                sys.executable,
                str(checker),
                str(empty),
                *[str(report) for report in reports[1:]],
            ],
            env=optimized_env,
            expect_nonzero=True,
        )
        if "expected one suite start and finish" not in empty_log:
            raise RuntimeError(
                "optimized checker failed without the expected empty-report rejection"
            )
        malformed_log = supervise(
            [
                sys.executable,
                str(checker),
                str(malformed),
                *[str(report) for report in reports[1:]],
            ],
            env=optimized_env,
            expect_nonzero=True,
        )
        if "invalid JSON" not in malformed_log:
            raise RuntimeError(
                "optimized checker failed without the expected malformed-JSON rejection"
            )
        print(
            "pinned Nextest build and plus/non-plus mixed/all-ignored reporter smoke passed; optimized checker rejected empty and malformed reports"
        )
    finally:
        # No deletion of fresh executables/build products; hosted runner teardown owns them.
        print(f"retained Nextest validation directory: {work}", flush=True)


if __name__ == "__main__":
    main()
