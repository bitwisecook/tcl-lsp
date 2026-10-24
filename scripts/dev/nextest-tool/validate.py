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


def verify_install():
    """Bounded post-GitHub-PATH custody check; never rebuild or rerun smokes."""
    temp = Path(os.environ["RUNNER_TEMP"])
    free = shutil.disk_usage(temp).free
    if free < 2 * 1024**3:
        raise RuntimeError(
            f"need at least 2 GiB free under RUNNER_TEMP; found {free} bytes"
        )
    work = Path(tempfile.mkdtemp(prefix="nextest-verify-", dir=temp))
    initialize(work)
    try:
        private_dir = os.environ.get("TCL_LSP_NEXTEST_PRIVATE_BIN", "")
        private_binary = os.environ.get("TCL_LSP_NEXTEST_PRIVATE_BINARY", "")
        expected_hash = os.environ.get("TCL_LSP_NEXTEST_PRIVATE_SHA256", "")
        expected_version = os.environ.get("TCL_LSP_NEXTEST_VERSION", "")
        if not all((private_dir, private_binary, expected_hash, expected_version)):
            raise RuntimeError("private Nextest custody environment is incomplete")
        if "\r" in private_dir or "\n" in private_dir:
            raise RuntimeError("private binary directory contains a line break")
        if expected_version != PINS["NEXTTEST_VERSION"]:
            raise RuntimeError("private Nextest version pin changed between steps")
        binary = Path(private_binary)
        if binary.resolve() != (Path(private_dir) / "cargo-nextest").resolve():
            raise RuntimeError(
                "private binary path does not match private bin directory"
            )
        if not binary.is_file() or not os.access(binary, os.X_OK):
            raise RuntimeError("private cargo-nextest is missing or not executable")
        path = os.environ.get("PATH", "")
        cargo_home_value = os.environ.get("CARGO_HOME")
        cargo_home = (
            Path(cargo_home_value)
            if cargo_home_value is not None
            else Path.home() / ".cargo"
        )
        cargo_home_bin = str(cargo_home / "bin")
        if not cargo_home.is_absolute():
            raise RuntimeError("Cargo home must be an absolute path")
        if cargo_home_bin not in path.split(os.pathsep):
            raise RuntimeError(
                "Cargo home bin must remain in PATH for command precedence"
            )
        resolved = shutil.which("cargo-nextest", path=path)
        if resolved is None or Path(resolved).resolve() != binary.resolve():
            raise RuntimeError("published private binary is not first in PATH")
        if "CARGO_ALIAS_NEXTEST" in os.environ:
            raise RuntimeError(
                "CARGO_ALIAS_NEXTEST cannot override the external command"
            )
        initial_hash = hashlib.sha256(binary.read_bytes()).hexdigest()
        if initial_hash != expected_hash:
            raise RuntimeError("published private binary SHA-256 custody mismatch")
        command_list = supervise(["cargo", "--color", "never", "--list", "--verbose"])
        command_rows = []
        for line in command_list.splitlines():
            fields = line.strip().split(None, 1)
            if len(fields) == 2 and fields[0] == "nextest":
                command_rows.append(fields[1])
        if (
            len(command_rows) != 1
            or command_rows[0].startswith("alias:")
            or not Path(command_rows[0]).is_absolute()
        ):
            raise RuntimeError("Cargo nextest alias/command listing is ambiguous")
        command_version = supervise(["cargo", "nextest", "--version"])
        if f"cargo-nextest {expected_version}" not in command_version:
            raise RuntimeError("Cargo selected a different Nextest version")
        actual_hash = hashlib.sha256(binary.read_bytes()).hexdigest()
        if actual_hash != expected_hash or actual_hash != initial_hash:
            raise RuntimeError("private binary changed during custody verification")
        print(
            "post-publication cargo-nextest path, hash, alias, and "
            "version custody verified",
            flush=True,
        )
    finally:
        print(f"retained Nextest custody directory: {work}", flush=True)


def main():
    modes = {"--validate-only", "--install", "--verify-install"}
    if len(sys.argv) != 2 or sys.argv[1] not in modes:
        raise SystemExit(
            "usage: validate.py --validate-only | --install | --verify-install"
        )
    if sys.argv[1] == "--verify-install":
        verify_install()
        return
    install_mode = sys.argv[1] == "--install"
    github_path = os.environ.get("GITHUB_PATH", "") if install_mode else ""
    github_env = os.environ.get("GITHUB_ENV", "") if install_mode else ""
    if install_mode and (not github_path or not github_env):
        raise RuntimeError("GITHUB_PATH and GITHUB_ENV are required for install mode")
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
        if install_mode:
            binary_dir = root / "bin"
            binary = binary_dir / "cargo-nextest"
            binary_dir_text = str(binary_dir)
            if "\r" in binary_dir_text or "\n" in binary_dir_text:
                raise RuntimeError("private binary directory contains a line break")
            current_path = os.environ.get("PATH", "")
            cargo_home_value = os.environ.get("CARGO_HOME")
            cargo_home = (
                Path(cargo_home_value)
                if cargo_home_value is not None
                else Path.home() / ".cargo"
            )
            cargo_home_bin = str(cargo_home / "bin")
            if not cargo_home.is_absolute():
                raise RuntimeError("Cargo home must be an absolute path")
            if cargo_home_bin not in current_path.split(os.pathsep):
                raise RuntimeError(
                    "Cargo home bin must already be in PATH for command precedence"
                )
            resolved = shutil.which(
                "cargo-nextest",
                path=os.pathsep.join([binary_dir_text, current_path]),
            )
            if resolved is None or Path(resolved).resolve() != binary.resolve():
                raise RuntimeError(
                    "private cargo-nextest is not first in external command resolution"
                )
            install_env = env.copy()
            install_env["PATH"] = os.pathsep.join([binary_dir_text, current_path])
            if "CARGO_ALIAS_NEXTEST" in install_env:
                raise RuntimeError(
                    "CARGO_ALIAS_NEXTEST cannot override the external command"
                )
            command_list = supervise(
                ["cargo", "--color", "never", "--list", "--verbose"],
                env=install_env,
            )
            command_rows = []
            for line in command_list.splitlines():
                fields = line.strip().split(None, 1)
                if len(fields) == 2 and fields[0] == "nextest":
                    command_rows.append(fields[1])
            if (
                len(command_rows) != 1
                or command_rows[0].startswith("alias:")
                or not Path(command_rows[0]).is_absolute()
            ):
                raise RuntimeError("Cargo nextest alias/command listing is ambiguous")
            command_version = supervise(
                ["cargo", "nextest", "--version"], env=install_env
            )
            if f"cargo-nextest {PINS['NEXTTEST_VERSION']}" not in command_version:
                raise RuntimeError(
                    "cargo subcommand selected wrong Nextest: "
                    f"{command_version.strip()}"
                )
            binary_sha256 = hashlib.sha256(binary.read_bytes()).hexdigest()
            with Path(github_path).open("a") as path_file:
                path_file.write(binary_dir_text + "\n")
            with Path(github_env).open("a") as env_file:
                env_file.write(f"TCL_LSP_NEXTEST_PRIVATE_BIN={binary_dir_text}\n")
                env_file.write(f"TCL_LSP_NEXTEST_PRIVATE_BINARY={binary}\n")
                env_file.write(f"TCL_LSP_NEXTEST_PRIVATE_SHA256={binary_sha256}\n")
                env_file.write(f"TCL_LSP_NEXTEST_VERSION={PINS['NEXTTEST_VERSION']}\n")
        print(
            "pinned Nextest build and plus/non-plus mixed/all-ignored reporter "
            "smoke passed; optimized checker rejected empty and malformed reports"
        )
        if install_mode:
            print(
                "private cargo-nextest passed exact Cargo PATH resolution and was "
                "published with binary custody",
                flush=True,
            )
    finally:
        # The hosted runner teardown owns fresh executables and build products.
        print(f"retained Nextest validation directory: {work}", flush=True)


if __name__ == "__main__":
    main()
