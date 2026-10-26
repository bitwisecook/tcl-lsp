# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program. If not, see <https://www.gnu.org/licenses/>.
#
# SPDX-License-Identifier: AGPL-3.0-or-later

"""WORK-only original58 and separate Native35/SH21/ReadFlow/catch diagnostics."""

import argparse
import datetime
import hashlib
import importlib.util
import json
import os
import re
import resource
import shutil
import subprocess
import sys
import time
from pathlib import Path
from typing import Any

CAP = 20 * 1024**2
FLOOR = 2 * 1024**3
EXECUTOR_SHA = "88125bd5267ac897dda4b5f665872c5ae0ab9984b0f42a41edeadf718873d811"
BASE = Path(__file__).resolve().parent


def require(value, message):
    if not value:
        raise RuntimeError(message)


def sha(path):
    with Path(path).open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def save(path, value):
    Path(path).write_text(json.dumps(value, indent=2, sort_keys=True) + "\n")


def git(*argv):
    return subprocess.check_output(["git", *argv], text=True).strip()


def files(paths):
    result = {}
    for root in paths:
        require(root.exists(), "required provider absent: " + str(root))
        names = [root] if root.is_file() else sorted(root.rglob("*"))
        for path in names:
            if path.is_file():
                result[str(path)] = {
                    "sha256": sha(path),
                    "symlink": os.readlink(path) if path.is_symlink() else None,
                }
    return result


def source():
    require(git("rev-parse", "HEAD") == args.sha, "source commit changed")
    require(
        not git("status", "--porcelain=v1", "--untracked-files=all"), "dirty source"
    )
    paths = [
        root / os.fsdecode(name)
        for name in subprocess.check_output(["git", "ls-files", "-z"]).split(b"\0")
        if name
    ]
    return {
        "head": args.sha,
        "tree": git("rev-parse", "HEAD^{tree}"),
        "files": files(paths),
    }


def lifecycle(receipt):
    return (
        all(
            receipt.get(key) is True
            for key in [
                "complete",
                "group_empty_before_parent_reap",
                "timer_armed_before_spawn",
                "artifact_unchanged",
            ]
        )
        and receipt.get("stop_reason") is None
    )


def original_result(stdout, exit_code):
    terminal = re.findall(
        r"^test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out;",
        stdout,
        re.MULTILINE,
    )
    require(len(terminal) == 1, "original test terminal missing/ambiguous")
    label = terminal[0][0]
    passed, failed, ignored, measured, _filtered = map(int, terminal[0][1:])
    require(
        passed + failed == 1
        and ignored == measured == 0
        and "running 1 test" in stdout,
        "original selected/terminal counts mismatch",
    )
    require(
        (label, passed, failed, exit_code) in [("ok", 1, 0, 0), ("FAILED", 0, 1, 101)],
        "original exit/terminal disagreement",
    )
    return "PASS" if passed == 1 else "FAIL"


def execute(argv, prefix, artifact, seconds, environment, budget):
    require(budget > 0, "aggregate log budget exhausted")
    available = deadline - time.time()
    require(available > 0, "absolute diagnostic deadline reached")
    if prefix.startswith(
        (
            "original58-",
            "supplemental-native35-",
            "supplemental-sh21-",
            "supplemental-readflow1-",
            "diagnostic-catch1-",
            "diagnostic-taint2-",
            "diagnostic-closed-consumer-",
            "diagnostic-inlining-readonly10-",
            "diagnostic-original-body-owner3-",
            "diagnostic-source-read-ancestry3-",
            "diagnostic-logical-declared-read3-",
            "diagnostic-logical-source-body-entry5-",
            "diagnostic-compact-codec1-",
            "diagnostic-original-child-all10-",
            "diagnostic-restored-child-owner1-",
            "diagnostic-native-string-effects3-",
            "diagnostic-native-formal-layout2-",
            "diagnostic-analyser-layout3-",
            "diagnostic-own-jim-member3-",
            "diagnostic-advice-catalogue3-",
            "diagnostic-source-point-memo8-",
            "diagnostic-source-point-corpus2-",
            "diagnostic-source-point-report-firstguard1-",
            "diagnostic-own-jim-alias-firstguard1-",
            "diagnostic-analyser-layout-firstguard3-",
            "diagnostic-native-eligibility2-",
            "diagnostic-jim-source-layout11-",
            "diagnostic-layout-own-jim-",
            "diagnostic-inherited-point-report3-",
            "list-",
            "select-",
        )
    ):
        require(
            available >= seconds,
            "insufficient overall time for original full test/list budget",
        )
    if any(
        label in prefix
        for label in (
            "diagnostic-closed-consumer-",
            "diagnostic-inlining-readonly10-",
            "diagnostic-original-body-owner3-",
            "diagnostic-source-read-ancestry3-",
            "diagnostic-logical-declared-read3-",
            "diagnostic-logical-source-body-entry5-",
            "diagnostic-compact-codec1-",
            "diagnostic-original-child-all10-",
            "diagnostic-restored-child-owner1-",
            "diagnostic-native-string-effects3-",
            "diagnostic-native-formal-layout2-",
            "diagnostic-analyser-layout3-",
            "diagnostic-own-jim-member3-",
            "diagnostic-advice-catalogue3-",
            "diagnostic-source-point-memo8-",
            "diagnostic-source-point-corpus2-",
            "diagnostic-source-point-report-firstguard1-",
            "diagnostic-own-jim-alias-firstguard1-",
            "diagnostic-analyser-layout-firstguard3-",
            "diagnostic-native-eligibility2-",
            "diagnostic-jim-source-layout11-",
            "diagnostic-layout-own-jim-",
            "diagnostic-inherited-point-report3-",
        )
    ):
        require(available >= seconds + 20, "insufficient time for complete closed control and cleanup")
    seconds = min(seconds, available)
    if prefix.startswith(
        (
            "original58-",
            "supplemental-native35-",
            "supplemental-sh21-",
            "supplemental-readflow1-",
            "diagnostic-catch1-",
            "diagnostic-taint2-",
            "diagnostic-closed-consumer-",
            "diagnostic-inlining-readonly10-",
            "diagnostic-original-body-owner3-",
            "diagnostic-source-read-ancestry3-",
            "diagnostic-logical-declared-read3-",
            "diagnostic-logical-source-body-entry5-",
            "diagnostic-compact-codec1-",
            "diagnostic-original-child-all10-",
            "diagnostic-restored-child-owner1-",
            "diagnostic-native-string-effects3-",
            "diagnostic-native-formal-layout2-",
            "diagnostic-analyser-layout3-",
            "diagnostic-own-jim-member3-",
            "diagnostic-advice-catalogue3-",
            "diagnostic-source-point-memo8-",
            "diagnostic-source-point-corpus2-",
            "diagnostic-source-point-report-firstguard1-",
            "diagnostic-own-jim-alias-firstguard1-",
            "diagnostic-analyser-layout-firstguard3-",
            "diagnostic-native-eligibility2-",
            "diagnostic-jim-source-layout11-",
            "diagnostic-layout-own-jim-",
            "diagnostic-inherited-point-report3-",
        )
    ):
        require(
            argv and Path(argv[0]).resolve() == Path(artifact).resolve(),
            "transport child must be the explicit selected test artifact",
        )
        argv = [
            str(tools["python"]),
            str(tools["table_filter"]),
            "--summary-path",
            str(out / (prefix + ".transport.json")),
            "--",
            *argv,
        ]
        artifact = tools["python"]
    return owner.run(
        argv,
        cwd=root,
        env=environment,
        stdout_path=out / (prefix + ".stdout"),
        stderr_path=out / (prefix + ".stderr"),
        receipt_path=out / (prefix + ".process.json"),
        artifact_path=artifact,
        deadline_seconds=seconds,
        cap_bytes=budget,
        disk_floor_bytes=FLOOR,
        disk_paths=(root, out, target, Path(os.environ["RUNNER_TEMP"])),
    )


def run_readonly_body_controls() -> None:
    global remaining
    require(
        all(
            row["state"] in ["PASS", "FAIL"]
            for row in original65_rows + taint_rows + closed_rows
        ),
        "readonly diagnostics cannot precede original83 closure",
    )
    require(
        files(dependency_roots) == dependencies_before
        and source() == before
        and files(provider_roots) == providers_before
        and files(list(tools.values())) == tool_pins,
        "pre-readonly source/provider/tool/dependency drift",
    )
    target_spec = readonly_validation["build_only_target"]
    package = target_spec["package"]
    target_name = target_spec["name"]
    target_kind = target_spec["kind"]
    selected_rows = [
        row
        for row in execution_rows
        if row["package"] == package
        and row["kind"] == target_kind
        and row["target"] == target_name
    ]
    selectors = [row["selector"] for row in selected_rows]
    require(remaining > 512 * 1024, "insufficient log budget before selected build")
    print("BUILD original target " + target_name, flush=True)
    argv = [
        str(tools["cargo"]),
        "test",
        "--locked",
        "-p",
        package,
        *(["--lib"] if target_kind == ["lib"] else ["--test", target_name]),
        "--no-run",
        "--message-format=json",
        "-j",
        "2",
    ]
    built = execute(argv, "build-" + target_name, tools["cargo"], 2400, env, remaining)
    processes.append(built)
    remaining -= built["retained_bytes"]
    require(
        lifecycle(built) and built["exit_code"] == 0,
        "selected target build failed or unclosed: " + target_name,
    )
    messages = [
        json.loads(line)
        for line in (out / ("build-" + target_name + ".stdout"))
        .read_text()
        .splitlines()
        if line.startswith("{")
    ]
    require(
        not any(
            row.get("reason") == "compiler-message"
            and row.get("message", {}).get("level") in ["warning", "error"]
            for row in messages
        ),
        "selected build emitted Rust warning/error",
    )
    require(
        sum(
            row.get("reason") == "build-finished" and row.get("success") is True
            for row in messages
        )
        == 1,
        "successful Cargo terminal missing",
    )
    selected = [
        row
        for row in messages
        if row.get("reason") == "compiler-artifact"
        and row.get("target", {}).get("name") == target_name
        and row.get("target", {}).get("kind") == target_kind
        and row.get("manifest_path") == str(root / "rust" / package / "Cargo.toml")
        and row.get("profile", {}).get("test") is True
        and row.get("executable")
    ]
    require(
        len(selected) == 1
        and selected[0].get("fresh") is False
        and selected[0].get("profile", {}).get("debug_assertions") is True,
        "fresh current diagnostic test artifact missing/ambiguous",
    )
    executable = Path(selected[0]["executable"]).resolve()
    require(
        executable.is_file() and executable.is_relative_to(target),
        "test artifact outside fresh owned target",
    )
    artifact = {
        "target": target_name,
        "package": package,
        "kind": target_kind,
        "path": str(executable),
        "sha256": sha(executable),
        "cargo_event": selected[0],
        "strict_Rust_0w": True,
        "admission": "selected target compilation only",
    }
    artifacts.append(artifact)
    save(out / "selected-artifacts.json", artifacts)
    require(
        files(dependency_roots) == dependencies_before
        and source() == before
        and files(provider_roots) == providers_before
        and files(list(tools.values())) == tool_pins,
        "pre-runtime source/provider/tool/dependency drift",
    )
    list_argv = [str(executable), "--list", "--format", "terse"]
    listing = execute(list_argv, "list-" + target_name, executable, 180, env, remaining)
    processes.append(listing)
    remaining -= listing["retained_bytes"]
    require(
        lifecycle(listing) and listing["exit_code"] == 0,
        "original roster listing failed or unclosed",
    )
    listed = []
    for line in (out / ("list-" + target_name + ".stdout")).read_text().splitlines():
        if line.endswith(": test"):
            listed.append(line.removesuffix(": test"))
        else:
            require(
                not line or re.fullmatch(r"\d+ tests?, \d+ benchmarks?", line),
                "unexpected listing data: " + line[:256],
            )
    require(
        len(listed) == len(set(listed))
        and set(selectors).issubset(listed)
        and len(selectors)
        == target_spec["expected_selected"]
        + sum(
            row["target"] == target_name
            and row["package"] == package
            and row["kind"] == target_kind
            for row in supplemental_rows + sh21_rows + readflow_rows + catch_rows
        ),
        "original actual roster mismatch: " + target_name,
    )
    save(
        out / ("roster-" + target_name + ".json"),
        {
            "target": target_name,
            "selected": listed,
            "listing_sha256": sha(out / ("list-" + target_name + ".stdout")),
            "executable_sha256": artifact["sha256"],
        },
    )
    require(
        len(listed) == 77
        and set(listed) == set(readonly_validation["full_target_inventory"])
        and {row["selector"] for row in readonly_rows[:10]}.issubset(listed),
        "readonly fresh complete77 inventory differs",
    )
    original_lib_roster = json.loads((out / "roster-tcl_compiler.json").read_text())
    require(
        {row["selector"] for row in readonly_rows[10:]}.issubset(
            original_lib_roster["selected"]
        ),
        "readonly owner controls absent from actual original lib inventory",
    )
    for index, row in enumerate(readonly_rows):
        package, target_kind, target_name = row["package"], row["kind"], row["target"]
        selected_artifacts = [
            value
            for value in artifacts
            if value["package"] == package
            and value["kind"] == target_kind
            and value["target"] == target_name
        ]
        require(
            len(selected_artifacts) == 1,
            "taint diagnostic current built artifact absent/ambiguous",
        )
        diagnostic_artifact: dict[str, Any] = selected_artifacts[0]
        require(
            diagnostic_artifact["strict_Rust_0w"] is True
            and diagnostic_artifact["cargo_event"]["profile"]["debug_assertions"]
            is True,
            "taint diagnostic requires existing fresh strict debug-assertion test profile",
        )
        executable = Path(diagnostic_artifact["path"])
        require(
            sha(executable) == diagnostic_artifact["sha256"],
            "taint diagnostic artifact changed before listing",
        )
        selector = row["selector"]
        require(remaining > 512 * 1024, "insufficient log budget before original test")
        prefix = row["group"] + "-" + target_name + "-" + str(index).zfill(2)
        group_label = (
            "readonly-inlining"
            if row["group"] == "diagnostic-inlining-readonly10"
            else "original-body-owner"
        )
        print("SELECT " + group_label + " " + selector, flush=True)
        runtime_env = env.copy()
        selected_env = row["env"]
        if not isinstance(selected_env, dict):
            raise TypeError("original per-selector environment is not a mapping")
        for name, value in selected_env.items():
            if not isinstance(name, str) or not isinstance(value, str):
                raise TypeError("original per-selector environment is not text")
            runtime_env[name] = value
        selected_list = execute(
            [str(executable), selector, "--exact", "--list", "--format", "terse"],
            "select-" + prefix,
            executable,
            180,
            runtime_env,
            remaining,
        )
        processes.append(selected_list)
        remaining -= selected_list["retained_bytes"]
        require(
            lifecycle(selected_list) and selected_list["exit_code"] == 0,
            "original exact selector list failed or unclosed",
        )
        selected_lines = (
            (out / ("select-" + prefix + ".stdout")).read_text().splitlines()
        )
        require(
            [line for line in selected_lines if line.endswith(": test")]
            == [selector + ": test"],
            "original exact selector list mismatch",
        )
        require(
            all(
                line == selector + ": test"
                or not line
                or re.fullmatch(r"1 tests?, 0 benchmarks?", line)
                for line in selected_lines
            ),
            "unexpected original exact selector list data",
        )
        require(
            remaining > 512 * 1024,
            "insufficient log budget after original exact selector list",
        )
        child_before = sha(executable)
        require(
            child_before == diagnostic_artifact["sha256"],
            "selected Rust test artifact changed before transport spawn",
        )
        row["selected_test_artifact"] = {
            "path": str(executable),
            "expected_sha256": diagnostic_artifact["sha256"],
            "before_sha256": child_before,
            "after_sha256": None,
            "unchanged": False,
        }
        row["state"] = "RUNNING"
        save(out / "outcomes.json", rows)
        print("RUN " + group_label + " " + selector, flush=True)
        receipt = execute(
            [
                str(executable),
                selector,
                "--exact",
                "--nocapture",
                "--test-threads=1",
            ],
            prefix,
            executable,
            180,
            runtime_env,
            remaining,
        )
        processes.append(receipt)
        remaining -= receipt["retained_bytes"]
        child_after = sha(executable)
        row["selected_test_artifact"]["after_sha256"] = child_after
        row["selected_test_artifact"]["unchanged"] = (
            child_before == child_after == diagnostic_artifact["sha256"]
        )
        save(out / "outcomes.json", rows)
        require(
            row["selected_test_artifact"]["unchanged"],
            "selected Rust test artifact changed during transport",
        )
        transport_path = out / (prefix + ".transport.json")
        if lifecycle(receipt):
            require(
                transport_path.is_file(), "completed test transport summary missing"
            )
            transport = json.loads(transport_path.read_text())
            encoded_stderr = (out / (prefix + ".stderr")).read_bytes()
            repeat_stage = transport["repeat_stage"]
            table_stage = transport["table_stage"]
            require(
                transport["child_exit_code"] == receipt["exit_code"]
                and transport["ordinary_stderr_verbatim"] is False
                and transport["stderr_losslessly_reconstructible_after_table_filter"]
                is True
                and table_stage["ordinary_stderr_verbatim"] is True
                and table_stage["stdout_verbatim"] is True
                and transport["stdout_verbatim"] is True
                and transport["retained_stderr_bytes"]
                == repeat_stage["output_bytes"]
                == len(encoded_stderr)
                and repeat_stage["format"]
                == "two-family-lossless-line-dictionary-rle-v1"
                and repeat_stage["input_bytes"] == table_stage["retained_stderr_bytes"]
                and repeat_stage["output_sha256"]
                == hashlib.sha256(encoded_stderr).hexdigest(),
                "transport child status or layered byte census mismatch",
            )
            reconstructed = transport_filter.decode_diagnostic_repeats(
                encoded_stderr, lambda _data: None, repeat_stage["input_bytes"]
            )
            require(
                reconstructed["bytes"] == repeat_stage["input_bytes"]
                and reconstructed["sha256"] == repeat_stage["input_sha256"],
                "transport diagnostic reconstruction differs from original stream",
            )
            row["transport_reconstruction"] = reconstructed
        row.update(
            exit_code=receipt["exit_code"],
            receipt=prefix + ".process.json",
            stdout=prefix + ".stdout",
            stderr=prefix + ".stderr",
        )
        require(lifecycle(receipt), "original test lifecycle/capture stop: " + selector)
        stdout = (out / (prefix + ".stdout")).read_text(errors="replace")
        try:
            row["state"] = original_result(stdout, receipt["exit_code"])
        except RuntimeError:
            row["state"] = "SIGNAL" if receipt["exit_code"] < 0 else "PROTOCOL_FAILURE"
            save(out / "outcomes.json", rows)
            raise
        save(out / "outcomes.json", rows)
        require(
            sha(executable) == diagnostic_artifact["sha256"],
            "selected test artifact changed",
        )


def run_source_read_ancestry_controls() -> None:
    global remaining
    require(
        all(
            row["state"] in ["PASS", "FAIL"]
            for row in original65_rows + taint_rows + closed_rows + readonly_rows
        ),
        "ancestry controls cannot precede original96 closure",
    )
    require(
        files(dependency_roots) == dependencies_before
        and source() == before
        and files(provider_roots) == providers_before
        and files(list(tools.values())) == tool_pins,
        "pre-ancestry source/provider/tool/dependency drift",
    )
    original_lib_roster = json.loads((out / "roster-tcl_compiler.json").read_text())
    require(
        {row["selector"] for row in ancestry_rows}.issubset(
            original_lib_roster["selected"]
        ),
        "ancestry controls absent from actual fresh compiler lib inventory",
    )
    for index, row in enumerate(ancestry_rows):
        package, target_kind, target_name = row["package"], row["kind"], row["target"]
        selected_artifacts = [
            value
            for value in artifacts
            if value["package"] == package
            and value["kind"] == target_kind
            and value["target"] == target_name
        ]
        require(
            len(selected_artifacts) == 1,
            "taint diagnostic current built artifact absent/ambiguous",
        )
        diagnostic_artifact: dict[str, Any] = selected_artifacts[0]
        require(
            diagnostic_artifact["strict_Rust_0w"] is True
            and diagnostic_artifact["cargo_event"]["profile"]["debug_assertions"]
            is True,
            "taint diagnostic requires existing fresh strict debug-assertion test profile",
        )
        executable = Path(diagnostic_artifact["path"])
        require(
            sha(executable) == diagnostic_artifact["sha256"],
            "taint diagnostic artifact changed before listing",
        )
        selector = row["selector"]
        require(remaining > 512 * 1024, "insufficient log budget before original test")
        prefix = row["group"] + "-" + target_name + "-" + str(index).zfill(2)
        group_label = "source-read-ancestry"
        print("SELECT " + group_label + " " + selector, flush=True)
        runtime_env = env.copy()
        selected_env = row["env"]
        if not isinstance(selected_env, dict):
            raise TypeError("original per-selector environment is not a mapping")
        for name, value in selected_env.items():
            if not isinstance(name, str) or not isinstance(value, str):
                raise TypeError("original per-selector environment is not text")
            runtime_env[name] = value
        selected_list = execute(
            [str(executable), selector, "--exact", "--list", "--format", "terse"],
            "select-" + prefix,
            executable,
            180,
            runtime_env,
            remaining,
        )
        processes.append(selected_list)
        remaining -= selected_list["retained_bytes"]
        require(
            lifecycle(selected_list) and selected_list["exit_code"] == 0,
            "original exact selector list failed or unclosed",
        )
        selected_lines = (
            (out / ("select-" + prefix + ".stdout")).read_text().splitlines()
        )
        require(
            [line for line in selected_lines if line.endswith(": test")]
            == [selector + ": test"],
            "original exact selector list mismatch",
        )
        require(
            all(
                line == selector + ": test"
                or not line
                or re.fullmatch(r"1 tests?, 0 benchmarks?", line)
                for line in selected_lines
            ),
            "unexpected original exact selector list data",
        )
        require(
            remaining > 512 * 1024,
            "insufficient log budget after original exact selector list",
        )
        child_before = sha(executable)
        require(
            child_before == diagnostic_artifact["sha256"],
            "selected Rust test artifact changed before transport spawn",
        )
        row["selected_test_artifact"] = {
            "path": str(executable),
            "expected_sha256": diagnostic_artifact["sha256"],
            "before_sha256": child_before,
            "after_sha256": None,
            "unchanged": False,
        }
        row["state"] = "RUNNING"
        save(out / "outcomes.json", rows)
        print("RUN " + group_label + " " + selector, flush=True)
        receipt = execute(
            [
                str(executable),
                selector,
                "--exact",
                "--nocapture",
                "--test-threads=1",
            ],
            prefix,
            executable,
            180,
            runtime_env,
            remaining,
        )
        processes.append(receipt)
        remaining -= receipt["retained_bytes"]
        child_after = sha(executable)
        row["selected_test_artifact"]["after_sha256"] = child_after
        row["selected_test_artifact"]["unchanged"] = (
            child_before == child_after == diagnostic_artifact["sha256"]
        )
        save(out / "outcomes.json", rows)
        require(
            row["selected_test_artifact"]["unchanged"],
            "selected Rust test artifact changed during transport",
        )
        transport_path = out / (prefix + ".transport.json")
        if lifecycle(receipt):
            require(
                transport_path.is_file(), "completed test transport summary missing"
            )
            transport = json.loads(transport_path.read_text())
            encoded_stderr = (out / (prefix + ".stderr")).read_bytes()
            repeat_stage = transport["repeat_stage"]
            table_stage = transport["table_stage"]
            require(
                transport["child_exit_code"] == receipt["exit_code"]
                and transport["ordinary_stderr_verbatim"] is False
                and transport["stderr_losslessly_reconstructible_after_table_filter"]
                is True
                and table_stage["ordinary_stderr_verbatim"] is True
                and table_stage["stdout_verbatim"] is True
                and transport["stdout_verbatim"] is True
                and transport["retained_stderr_bytes"]
                == repeat_stage["output_bytes"]
                == len(encoded_stderr)
                and repeat_stage["format"]
                == "two-family-lossless-line-dictionary-rle-v1"
                and repeat_stage["input_bytes"] == table_stage["retained_stderr_bytes"]
                and repeat_stage["output_sha256"]
                == hashlib.sha256(encoded_stderr).hexdigest(),
                "transport child status or layered byte census mismatch",
            )
            reconstructed = transport_filter.decode_diagnostic_repeats(
                encoded_stderr, lambda _data: None, repeat_stage["input_bytes"]
            )
            require(
                reconstructed["bytes"] == repeat_stage["input_bytes"]
                and reconstructed["sha256"] == repeat_stage["input_sha256"],
                "transport diagnostic reconstruction differs from original stream",
            )
            row["transport_reconstruction"] = reconstructed
        row.update(
            exit_code=receipt["exit_code"],
            receipt=prefix + ".process.json",
            stdout=prefix + ".stdout",
            stderr=prefix + ".stderr",
        )
        require(lifecycle(receipt), "original test lifecycle/capture stop: " + selector)
        stdout = (out / (prefix + ".stdout")).read_text(errors="replace")
        try:
            row["state"] = original_result(stdout, receipt["exit_code"])
        except RuntimeError:
            row["state"] = "SIGNAL" if receipt["exit_code"] < 0 else "PROTOCOL_FAILURE"
            save(out / "outcomes.json", rows)
            raise
        save(out / "outcomes.json", rows)
        require(
            sha(executable) == diagnostic_artifact["sha256"],
            "selected test artifact changed",
        )


def run_logical_declared_read_controls() -> None:
    global remaining
    require(
        all(
            row["state"] in ["PASS", "FAIL"]
            for row in original65_rows + taint_rows + closed_rows + readonly_rows + ancestry_rows
        ),
        "logical controls cannot precede original99 closure",
    )
    require(
        files(dependency_roots) == dependencies_before
        and source() == before
        and files(provider_roots) == providers_before
        and files(list(tools.values())) == tool_pins,
        "pre-logical source/provider/tool/dependency drift",
    )
    original_lib_roster = json.loads((out / "roster-tcl_compiler.json").read_text())
    require(
        {row["selector"] for row in logical_rows}.issubset(
            original_lib_roster["selected"]
        ),
        "logical controls absent from actual fresh compiler lib inventory",
    )
    for index, row in enumerate(logical_rows):
        package, target_kind, target_name = row["package"], row["kind"], row["target"]
        selected_artifacts = [
            value
            for value in artifacts
            if value["package"] == package
            and value["kind"] == target_kind
            and value["target"] == target_name
        ]
        require(
            len(selected_artifacts) == 1,
            "taint diagnostic current built artifact absent/ambiguous",
        )
        diagnostic_artifact: dict[str, Any] = selected_artifacts[0]
        require(
            diagnostic_artifact["strict_Rust_0w"] is True
            and diagnostic_artifact["cargo_event"]["profile"]["debug_assertions"]
            is True,
            "taint diagnostic requires existing fresh strict debug-assertion test profile",
        )
        executable = Path(diagnostic_artifact["path"])
        require(
            sha(executable) == diagnostic_artifact["sha256"],
            "taint diagnostic artifact changed before listing",
        )
        selector = row["selector"]
        require(remaining > 512 * 1024, "insufficient log budget before original test")
        prefix = row["group"] + "-" + target_name + "-" + str(index).zfill(2)
        group_label = "logical-declared-read"
        print("SELECT " + group_label + " " + selector, flush=True)
        runtime_env = env.copy()
        selected_env = row["env"]
        if not isinstance(selected_env, dict):
            raise TypeError("original per-selector environment is not a mapping")
        for name, value in selected_env.items():
            if not isinstance(name, str) or not isinstance(value, str):
                raise TypeError("original per-selector environment is not text")
            runtime_env[name] = value
        selected_list = execute(
            [str(executable), selector, "--exact", "--list", "--format", "terse"],
            "select-" + prefix,
            executable,
            180,
            runtime_env,
            remaining,
        )
        processes.append(selected_list)
        remaining -= selected_list["retained_bytes"]
        require(
            lifecycle(selected_list) and selected_list["exit_code"] == 0,
            "original exact selector list failed or unclosed",
        )
        selected_lines = (
            (out / ("select-" + prefix + ".stdout")).read_text().splitlines()
        )
        require(
            [line for line in selected_lines if line.endswith(": test")]
            == [selector + ": test"],
            "original exact selector list mismatch",
        )
        require(
            all(
                line == selector + ": test"
                or not line
                or re.fullmatch(r"1 tests?, 0 benchmarks?", line)
                for line in selected_lines
            ),
            "unexpected original exact selector list data",
        )
        require(
            remaining > 512 * 1024,
            "insufficient log budget after original exact selector list",
        )
        child_before = sha(executable)
        require(
            child_before == diagnostic_artifact["sha256"],
            "selected Rust test artifact changed before transport spawn",
        )
        row["selected_test_artifact"] = {
            "path": str(executable),
            "expected_sha256": diagnostic_artifact["sha256"],
            "before_sha256": child_before,
            "after_sha256": None,
            "unchanged": False,
        }
        row["state"] = "RUNNING"
        save(out / "outcomes.json", rows)
        print("RUN " + group_label + " " + selector, flush=True)
        receipt = execute(
            [
                str(executable),
                selector,
                "--exact",
                "--nocapture",
                "--test-threads=1",
            ],
            prefix,
            executable,
            180,
            runtime_env,
            remaining,
        )
        processes.append(receipt)
        remaining -= receipt["retained_bytes"]
        child_after = sha(executable)
        row["selected_test_artifact"]["after_sha256"] = child_after
        row["selected_test_artifact"]["unchanged"] = (
            child_before == child_after == diagnostic_artifact["sha256"]
        )
        save(out / "outcomes.json", rows)
        require(
            row["selected_test_artifact"]["unchanged"],
            "selected Rust test artifact changed during transport",
        )
        transport_path = out / (prefix + ".transport.json")
        if lifecycle(receipt):
            require(
                transport_path.is_file(), "completed test transport summary missing"
            )
            transport = json.loads(transport_path.read_text())
            encoded_stderr = (out / (prefix + ".stderr")).read_bytes()
            repeat_stage = transport["repeat_stage"]
            table_stage = transport["table_stage"]
            require(
                transport["child_exit_code"] == receipt["exit_code"]
                and transport["ordinary_stderr_verbatim"] is False
                and transport["stderr_losslessly_reconstructible_after_table_filter"]
                is True
                and table_stage["ordinary_stderr_verbatim"] is True
                and table_stage["stdout_verbatim"] is True
                and transport["stdout_verbatim"] is True
                and transport["retained_stderr_bytes"]
                == repeat_stage["output_bytes"]
                == len(encoded_stderr)
                and repeat_stage["format"]
                == "two-family-lossless-line-dictionary-rle-v1"
                and repeat_stage["input_bytes"] == table_stage["retained_stderr_bytes"]
                and repeat_stage["output_sha256"]
                == hashlib.sha256(encoded_stderr).hexdigest(),
                "transport child status or layered byte census mismatch",
            )
            reconstructed = transport_filter.decode_diagnostic_repeats(
                encoded_stderr, lambda _data: None, repeat_stage["input_bytes"]
            )
            require(
                reconstructed["bytes"] == repeat_stage["input_bytes"]
                and reconstructed["sha256"] == repeat_stage["input_sha256"],
                "transport diagnostic reconstruction differs from original stream",
            )
            row["transport_reconstruction"] = reconstructed
        row.update(
            exit_code=receipt["exit_code"],
            receipt=prefix + ".process.json",
            stdout=prefix + ".stdout",
            stderr=prefix + ".stderr",
        )
        require(lifecycle(receipt), "original test lifecycle/capture stop: " + selector)
        stdout = (out / (prefix + ".stdout")).read_text(errors="replace")
        try:
            row["state"] = original_result(stdout, receipt["exit_code"])
        except RuntimeError:
            row["state"] = "SIGNAL" if receipt["exit_code"] < 0 else "PROTOCOL_FAILURE"
            save(out / "outcomes.json", rows)
            raise
        save(out / "outcomes.json", rows)
        require(
            sha(executable) == diagnostic_artifact["sha256"],
            "selected test artifact changed",
        )


def run_logical_body_entry_controls() -> None:
    global remaining
    require(
        all(
            row["state"] in ["PASS", "FAIL"]
            for row in rows
        ),
        "logical body-entry controls cannot precede original102 closure",
    )
    require(
        files(dependency_roots) == dependencies_before
        and source() == before
        and files(provider_roots) == providers_before
        and files(list(tools.values())) == tool_pins,
        "pre-logical-body-entry source/provider/tool/dependency drift",
    )
    original_lib_roster = json.loads((out / "roster-tcl_compiler.json").read_text())
    require(
        {row["selector"] for row in logical_body_rows}.issubset(
            original_lib_roster["selected"]
        ),
        "logical body-entry controls absent from actual fresh compiler lib inventory",
    )
    for index, row in enumerate(logical_body_rows):
        package, target_kind, target_name = row["package"], row["kind"], row["target"]
        selected_artifacts = [
            value
            for value in artifacts
            if value["package"] == package
            and value["kind"] == target_kind
            and value["target"] == target_name
        ]
        require(
            len(selected_artifacts) == 1,
            "logical body-entry current built artifact absent/ambiguous",
        )
        diagnostic_artifact: dict[str, Any] = selected_artifacts[0]
        require(
            diagnostic_artifact["strict_Rust_0w"] is True
            and diagnostic_artifact["cargo_event"]["profile"]["debug_assertions"]
            is True,
            "logical body-entry requires existing fresh strict debug-assertion test profile",
        )
        executable = Path(diagnostic_artifact["path"])
        require(
            sha(executable) == diagnostic_artifact["sha256"],
            "logical body-entry artifact changed before listing",
        )
        selector = row["selector"]
        require(remaining > 512 * 1024, "insufficient log budget before original test")
        prefix = row["group"] + "-" + target_name + "-" + str(index).zfill(2)
        group_label = "logical-source-body-entry"
        print("SELECT " + group_label + " " + selector, flush=True)
        runtime_env = env.copy()
        selected_env = row["env"]
        if not isinstance(selected_env, dict):
            raise TypeError("original per-selector environment is not a mapping")
        for name, value in selected_env.items():
            if not isinstance(name, str) or not isinstance(value, str):
                raise TypeError("original per-selector environment is not text")
            runtime_env[name] = value
        selected_list = execute(
            [str(executable), selector, "--exact", "--list", "--format", "terse"],
            "select-" + prefix,
            executable,
            180,
            runtime_env,
            remaining,
        )
        processes.append(selected_list)
        remaining -= selected_list["retained_bytes"]
        require(
            lifecycle(selected_list) and selected_list["exit_code"] == 0,
            "original exact selector list failed or unclosed",
        )
        selected_lines = (
            (out / ("select-" + prefix + ".stdout")).read_text().splitlines()
        )
        require(
            [line for line in selected_lines if line.endswith(": test")]
            == [selector + ": test"],
            "original exact selector list mismatch",
        )
        require(
            all(
                line == selector + ": test"
                or not line
                or re.fullmatch(r"1 tests?, 0 benchmarks?", line)
                for line in selected_lines
            ),
            "unexpected original exact selector list data",
        )
        require(
            remaining > 512 * 1024,
            "insufficient log budget after original exact selector list",
        )
        child_before = sha(executable)
        require(
            child_before == diagnostic_artifact["sha256"],
            "selected Rust test artifact changed before transport spawn",
        )
        row["selected_test_artifact"] = {
            "path": str(executable),
            "expected_sha256": diagnostic_artifact["sha256"],
            "before_sha256": child_before,
            "after_sha256": None,
            "unchanged": False,
        }
        row["state"] = "RUNNING"
        save(out / "logical-body-entry-outcomes.json", logical_body_rows)
        print("RUN " + group_label + " " + selector, flush=True)
        receipt = execute(
            [
                str(executable),
                selector,
                "--exact",
                "--nocapture",
                "--test-threads=1",
            ],
            prefix,
            executable,
            180,
            runtime_env,
            remaining,
        )
        processes.append(receipt)
        remaining -= receipt["retained_bytes"]
        child_after = sha(executable)
        row["selected_test_artifact"]["after_sha256"] = child_after
        row["selected_test_artifact"]["unchanged"] = (
            child_before == child_after == diagnostic_artifact["sha256"]
        )
        save(out / "logical-body-entry-outcomes.json", logical_body_rows)
        require(
            row["selected_test_artifact"]["unchanged"],
            "selected Rust test artifact changed during transport",
        )
        transport_path = out / (prefix + ".transport.json")
        if lifecycle(receipt):
            require(
                transport_path.is_file(), "completed test transport summary missing"
            )
            transport = json.loads(transport_path.read_text())
            encoded_stderr = (out / (prefix + ".stderr")).read_bytes()
            repeat_stage = transport["repeat_stage"]
            table_stage = transport["table_stage"]
            require(
                transport["child_exit_code"] == receipt["exit_code"]
                and transport["ordinary_stderr_verbatim"] is False
                and transport["stderr_losslessly_reconstructible_after_table_filter"]
                is True
                and table_stage["ordinary_stderr_verbatim"] is True
                and table_stage["stdout_verbatim"] is True
                and transport["stdout_verbatim"] is True
                and transport["retained_stderr_bytes"]
                == repeat_stage["output_bytes"]
                == len(encoded_stderr)
                and repeat_stage["format"]
                == "two-family-lossless-line-dictionary-rle-v1"
                and repeat_stage["input_bytes"] == table_stage["retained_stderr_bytes"]
                and repeat_stage["output_sha256"]
                == hashlib.sha256(encoded_stderr).hexdigest(),
                "transport child status or layered byte census mismatch",
            )
            reconstructed = transport_filter.decode_diagnostic_repeats(
                encoded_stderr, lambda _data: None, repeat_stage["input_bytes"]
            )
            require(
                reconstructed["bytes"] == repeat_stage["input_bytes"]
                and reconstructed["sha256"] == repeat_stage["input_sha256"],
                "transport diagnostic reconstruction differs from original stream",
            )
            row["transport_reconstruction"] = reconstructed
        row.update(
            exit_code=receipt["exit_code"],
            receipt=prefix + ".process.json",
            stdout=prefix + ".stdout",
            stderr=prefix + ".stderr",
        )
        require(lifecycle(receipt), "original test lifecycle/capture stop: " + selector)
        stdout = (out / (prefix + ".stdout")).read_text(errors="replace")
        try:
            row["state"] = original_result(stdout, receipt["exit_code"])
        except RuntimeError:
            row["state"] = "SIGNAL" if receipt["exit_code"] < 0 else "PROTOCOL_FAILURE"
            save(out / "logical-body-entry-outcomes.json", logical_body_rows)
            raise
        save(out / "logical-body-entry-outcomes.json", logical_body_rows)
        require(
            sha(executable) == diagnostic_artifact["sha256"],
            "selected test artifact changed",
        )


def run_compact_codec_control() -> None:
    global remaining
    require(
        all(
            row["state"] in ["PASS", "FAIL"]
            for row in rows + logical_body_rows
        ),
        "compact codec controls cannot precede original107 closure",
    )
    require(
        files(dependency_roots) == dependencies_before
        and source() == before
        and files(provider_roots) == providers_before
        and files(list(tools.values())) == tool_pins,
        "pre-logical-body-entry source/provider/tool/dependency drift",
    )
    original_lib_roster = json.loads((out / "roster-tcl_compiler.json").read_text())
    require(
        {row["selector"] for row in codec_rows}.issubset(
            original_lib_roster["selected"]
        ),
        "compact codec controls absent from actual fresh compiler lib inventory",
    )
    for index, row in enumerate(codec_rows):
        package, target_kind, target_name = row["package"], row["kind"], row["target"]
        selected_artifacts = [
            value
            for value in artifacts
            if value["package"] == package
            and value["kind"] == target_kind
            and value["target"] == target_name
        ]
        require(
            len(selected_artifacts) == 1,
            "compact codec current built artifact absent/ambiguous",
        )
        diagnostic_artifact: dict[str, Any] = selected_artifacts[0]
        require(
            diagnostic_artifact["strict_Rust_0w"] is True
            and diagnostic_artifact["cargo_event"]["profile"]["debug_assertions"]
            is True,
            "compact codec requires existing fresh strict debug-assertion test profile",
        )
        executable = Path(diagnostic_artifact["path"])
        require(
            sha(executable) == diagnostic_artifact["sha256"],
            "compact codec artifact changed before listing",
        )
        selector = row["selector"]
        require(remaining > 512 * 1024, "insufficient log budget before original test")
        prefix = row["group"] + "-" + target_name + "-" + str(index).zfill(2)
        group_label = "compact-codec"
        print("SELECT " + group_label + " " + selector, flush=True)
        runtime_env = env.copy()
        selected_env = row["env"]
        if not isinstance(selected_env, dict):
            raise TypeError("original per-selector environment is not a mapping")
        for name, value in selected_env.items():
            if not isinstance(name, str) or not isinstance(value, str):
                raise TypeError("original per-selector environment is not text")
            runtime_env[name] = value
        selected_list = execute(
            [str(executable), selector, "--exact", "--list", "--format", "terse"],
            "select-" + prefix,
            executable,
            180,
            runtime_env,
            remaining,
        )
        processes.append(selected_list)
        remaining -= selected_list["retained_bytes"]
        require(
            lifecycle(selected_list) and selected_list["exit_code"] == 0,
            "original exact selector list failed or unclosed",
        )
        selected_lines = (
            (out / ("select-" + prefix + ".stdout")).read_text().splitlines()
        )
        require(
            [line for line in selected_lines if line.endswith(": test")]
            == [selector + ": test"],
            "original exact selector list mismatch",
        )
        require(
            all(
                line == selector + ": test"
                or not line
                or re.fullmatch(r"1 tests?, 0 benchmarks?", line)
                for line in selected_lines
            ),
            "unexpected original exact selector list data",
        )
        require(
            remaining > 512 * 1024,
            "insufficient log budget after original exact selector list",
        )
        child_before = sha(executable)
        require(
            child_before == diagnostic_artifact["sha256"],
            "selected Rust test artifact changed before transport spawn",
        )
        row["selected_test_artifact"] = {
            "path": str(executable),
            "expected_sha256": diagnostic_artifact["sha256"],
            "before_sha256": child_before,
            "after_sha256": None,
            "unchanged": False,
        }
        row["state"] = "RUNNING"
        save(out / "compact-codec-outcomes.json", codec_rows)
        print("RUN " + group_label + " " + selector, flush=True)
        receipt = execute(
            [
                str(executable),
                selector,
                "--exact",
                "--nocapture",
                "--test-threads=1",
            ],
            prefix,
            executable,
            180,
            runtime_env,
            remaining,
        )
        processes.append(receipt)
        remaining -= receipt["retained_bytes"]
        child_after = sha(executable)
        row["selected_test_artifact"]["after_sha256"] = child_after
        row["selected_test_artifact"]["unchanged"] = (
            child_before == child_after == diagnostic_artifact["sha256"]
        )
        save(out / "compact-codec-outcomes.json", codec_rows)
        require(
            row["selected_test_artifact"]["unchanged"],
            "selected Rust test artifact changed during transport",
        )
        transport_path = out / (prefix + ".transport.json")
        if lifecycle(receipt):
            require(
                transport_path.is_file(), "completed test transport summary missing"
            )
            transport = json.loads(transport_path.read_text())
            encoded_stderr = (out / (prefix + ".stderr")).read_bytes()
            repeat_stage = transport["repeat_stage"]
            table_stage = transport["table_stage"]
            require(
                transport["child_exit_code"] == receipt["exit_code"]
                and transport["ordinary_stderr_verbatim"] is False
                and transport["stderr_losslessly_reconstructible_after_table_filter"]
                is True
                and table_stage["ordinary_stderr_verbatim"] is True
                and table_stage["stdout_verbatim"] is True
                and transport["stdout_verbatim"] is True
                and transport["retained_stderr_bytes"]
                == repeat_stage["output_bytes"]
                == len(encoded_stderr)
                and repeat_stage["format"]
                == "two-family-lossless-line-dictionary-rle-v1"
                and repeat_stage["input_bytes"] == table_stage["retained_stderr_bytes"]
                and repeat_stage["output_sha256"]
                == hashlib.sha256(encoded_stderr).hexdigest(),
                "transport child status or layered byte census mismatch",
            )
            reconstructed = transport_filter.decode_diagnostic_repeats(
                encoded_stderr, lambda _data: None, repeat_stage["input_bytes"]
            )
            require(
                reconstructed["bytes"] == repeat_stage["input_bytes"]
                and reconstructed["sha256"] == repeat_stage["input_sha256"],
                "transport diagnostic reconstruction differs from original stream",
            )
            row["transport_reconstruction"] = reconstructed
        row.update(
            exit_code=receipt["exit_code"],
            receipt=prefix + ".process.json",
            stdout=prefix + ".stdout",
            stderr=prefix + ".stderr",
        )
        require(lifecycle(receipt), "original test lifecycle/capture stop: " + selector)
        stdout = (out / (prefix + ".stdout")).read_text(errors="replace")
        try:
            row["state"] = original_result(stdout, receipt["exit_code"])
        except RuntimeError:
            row["state"] = "SIGNAL" if receipt["exit_code"] < 0 else "PROTOCOL_FAILURE"
            save(out / "compact-codec-outcomes.json", codec_rows)
            raise
        save(out / "compact-codec-outcomes.json", codec_rows)
        require(
            sha(executable) == diagnostic_artifact["sha256"],
            "selected test artifact changed",
        )


def run_original_child_controls() -> None:
    global remaining
    require(
        all(
            row["state"] in ["PASS", "FAIL"]
            for row in rows + logical_body_rows + codec_rows
        ),
        "original child controls cannot precede original108 closure",
    )
    require(
        files(dependency_roots) == dependencies_before
        and source() == before
        and files(provider_roots) == providers_before
        and files(list(tools.values())) == tool_pins,
        "pre-logical-body-entry source/provider/tool/dependency drift",
    )
    original_lib_roster = json.loads((out / "roster-tcl_compiler.json").read_text())
    require(
        {row["selector"] for row in named_child_rows}.issubset(
            original_lib_roster["selected"]
        ),
        "original child controls absent from actual fresh compiler lib inventory",
    )
    for index, row in enumerate(named_child_rows):
        package, target_kind, target_name = row["package"], row["kind"], row["target"]
        selected_artifacts = [
            value
            for value in artifacts
            if value["package"] == package
            and value["kind"] == target_kind
            and value["target"] == target_name
        ]
        require(
            len(selected_artifacts) == 1,
            "original child current built artifact absent/ambiguous",
        )
        diagnostic_artifact: dict[str, Any] = selected_artifacts[0]
        require(
            diagnostic_artifact["strict_Rust_0w"] is True
            and diagnostic_artifact["cargo_event"]["profile"]["debug_assertions"]
            is True,
            "original child requires existing fresh strict debug-assertion test profile",
        )
        executable = Path(diagnostic_artifact["path"])
        require(
            sha(executable) == diagnostic_artifact["sha256"],
            "original child artifact changed before listing",
        )
        selector = row["selector"]
        require(remaining > 512 * 1024, "insufficient log budget before original test")
        prefix = row["group"] + "-" + target_name + "-" + str(index).zfill(2)
        group_label = "original-child-all10"
        print("SELECT " + group_label + " " + selector, flush=True)
        runtime_env = env.copy()
        selected_env = row["env"]
        if not isinstance(selected_env, dict):
            raise TypeError("original per-selector environment is not a mapping")
        for name, value in selected_env.items():
            if not isinstance(name, str) or not isinstance(value, str):
                raise TypeError("original per-selector environment is not text")
            runtime_env[name] = value
        selected_list = execute(
            [str(executable), selector, "--exact", "--list", "--format", "terse"],
            "select-" + prefix,
            executable,
            180,
            runtime_env,
            remaining,
        )
        processes.append(selected_list)
        remaining -= selected_list["retained_bytes"]
        require(
            lifecycle(selected_list) and selected_list["exit_code"] == 0,
            "original exact selector list failed or unclosed",
        )
        selected_lines = (
            (out / ("select-" + prefix + ".stdout")).read_text().splitlines()
        )
        require(
            [line for line in selected_lines if line.endswith(": test")]
            == [selector + ": test"],
            "original exact selector list mismatch",
        )
        require(
            all(
                line == selector + ": test"
                or not line
                or re.fullmatch(r"1 tests?, 0 benchmarks?", line)
                for line in selected_lines
            ),
            "unexpected original exact selector list data",
        )
        require(
            remaining > 512 * 1024,
            "insufficient log budget after original exact selector list",
        )
        child_before = sha(executable)
        require(
            child_before == diagnostic_artifact["sha256"],
            "selected Rust test artifact changed before transport spawn",
        )
        row["selected_test_artifact"] = {
            "path": str(executable),
            "expected_sha256": diagnostic_artifact["sha256"],
            "before_sha256": child_before,
            "after_sha256": None,
            "unchanged": False,
        }
        row["state"] = "RUNNING"
        save(out / "original-child-outcomes.json", named_child_rows)
        print("RUN " + group_label + " " + selector, flush=True)
        receipt = execute(
            [
                str(executable),
                selector,
                "--exact",
                "--nocapture",
                "--test-threads=1",
            ],
            prefix,
            executable,
            180,
            runtime_env,
            remaining,
        )
        processes.append(receipt)
        remaining -= receipt["retained_bytes"]
        child_after = sha(executable)
        row["selected_test_artifact"]["after_sha256"] = child_after
        row["selected_test_artifact"]["unchanged"] = (
            child_before == child_after == diagnostic_artifact["sha256"]
        )
        save(out / "original-child-outcomes.json", named_child_rows)
        require(
            row["selected_test_artifact"]["unchanged"],
            "selected Rust test artifact changed during transport",
        )
        transport_path = out / (prefix + ".transport.json")
        if lifecycle(receipt):
            require(
                transport_path.is_file(), "completed test transport summary missing"
            )
            transport = json.loads(transport_path.read_text())
            encoded_stderr = (out / (prefix + ".stderr")).read_bytes()
            repeat_stage = transport["repeat_stage"]
            table_stage = transport["table_stage"]
            require(
                transport["child_exit_code"] == receipt["exit_code"]
                and transport["ordinary_stderr_verbatim"] is False
                and transport["stderr_losslessly_reconstructible_after_table_filter"]
                is True
                and table_stage["ordinary_stderr_verbatim"] is True
                and table_stage["stdout_verbatim"] is True
                and transport["stdout_verbatim"] is True
                and transport["retained_stderr_bytes"]
                == repeat_stage["output_bytes"]
                == len(encoded_stderr)
                and repeat_stage["format"]
                == "two-family-lossless-line-dictionary-rle-v1"
                and repeat_stage["input_bytes"] == table_stage["retained_stderr_bytes"]
                and repeat_stage["output_sha256"]
                == hashlib.sha256(encoded_stderr).hexdigest(),
                "transport child status or layered byte census mismatch",
            )
            reconstructed = transport_filter.decode_diagnostic_repeats(
                encoded_stderr, lambda _data: None, repeat_stage["input_bytes"]
            )
            require(
                reconstructed["bytes"] == repeat_stage["input_bytes"]
                and reconstructed["sha256"] == repeat_stage["input_sha256"],
                "transport diagnostic reconstruction differs from original stream",
            )
            row["transport_reconstruction"] = reconstructed
        row.update(
            exit_code=receipt["exit_code"],
            receipt=prefix + ".process.json",
            stdout=prefix + ".stdout",
            stderr=prefix + ".stderr",
        )
        require(lifecycle(receipt), "original test lifecycle/capture stop: " + selector)
        stdout = (out / (prefix + ".stdout")).read_text(errors="replace")
        try:
            row["state"] = original_result(stdout, receipt["exit_code"])
        except RuntimeError:
            row["state"] = "SIGNAL" if receipt["exit_code"] < 0 else "PROTOCOL_FAILURE"
            save(out / "original-child-outcomes.json", named_child_rows)
            raise
        save(out / "original-child-outcomes.json", named_child_rows)
        require(
            sha(executable) == diagnostic_artifact["sha256"],
            "selected test artifact changed",
        )


def run_restored_child_owner_control() -> None:
    global remaining
    require(
        all(
            row["state"] in ["PASS", "FAIL"]
            for row in rows + logical_body_rows + codec_rows + named_child_rows
        ),
        "restored child owner controls cannot precede original118 closure",
    )
    require(
        files(dependency_roots) == dependencies_before
        and source() == before
        and files(provider_roots) == providers_before
        and files(list(tools.values())) == tool_pins,
        "pre-logical-body-entry source/provider/tool/dependency drift",
    )
    original_lib_roster = json.loads((out / "roster-tcl_compiler.json").read_text())
    require(
        {row["selector"] for row in restored_child_rows}.issubset(
            original_lib_roster["selected"]
        ),
        "restored child owner controls absent from actual fresh compiler lib inventory",
    )
    for index, row in enumerate(restored_child_rows):
        package, target_kind, target_name = row["package"], row["kind"], row["target"]
        selected_artifacts = [
            value
            for value in artifacts
            if value["package"] == package
            and value["kind"] == target_kind
            and value["target"] == target_name
        ]
        require(
            len(selected_artifacts) == 1,
            "restored child owner current built artifact absent/ambiguous",
        )
        diagnostic_artifact: dict[str, Any] = selected_artifacts[0]
        require(
            diagnostic_artifact["strict_Rust_0w"] is True
            and diagnostic_artifact["cargo_event"]["profile"]["debug_assertions"]
            is True,
            "restored child owner requires existing fresh strict debug-assertion test profile",
        )
        executable = Path(diagnostic_artifact["path"])
        require(
            sha(executable) == diagnostic_artifact["sha256"],
            "restored child owner artifact changed before listing",
        )
        selector = row["selector"]
        require(remaining > 512 * 1024, "insufficient log budget before original test")
        prefix = row["group"] + "-" + target_name + "-" + str(index).zfill(2)
        group_label = "restored-child-owner1"
        print("SELECT " + group_label + " " + selector, flush=True)
        runtime_env = env.copy()
        selected_env = row["env"]
        if not isinstance(selected_env, dict):
            raise TypeError("original per-selector environment is not a mapping")
        for name, value in selected_env.items():
            if not isinstance(name, str) or not isinstance(value, str):
                raise TypeError("original per-selector environment is not text")
            runtime_env[name] = value
        selected_list = execute(
            [str(executable), selector, "--exact", "--list", "--format", "terse"],
            "select-" + prefix,
            executable,
            180,
            runtime_env,
            remaining,
        )
        processes.append(selected_list)
        remaining -= selected_list["retained_bytes"]
        require(
            lifecycle(selected_list) and selected_list["exit_code"] == 0,
            "original exact selector list failed or unclosed",
        )
        selected_lines = (
            (out / ("select-" + prefix + ".stdout")).read_text().splitlines()
        )
        require(
            [line for line in selected_lines if line.endswith(": test")]
            == [selector + ": test"],
            "original exact selector list mismatch",
        )
        require(
            all(
                line == selector + ": test"
                or not line
                or re.fullmatch(r"1 tests?, 0 benchmarks?", line)
                for line in selected_lines
            ),
            "unexpected original exact selector list data",
        )
        require(
            remaining > 512 * 1024,
            "insufficient log budget after original exact selector list",
        )
        child_before = sha(executable)
        require(
            child_before == diagnostic_artifact["sha256"],
            "selected Rust test artifact changed before transport spawn",
        )
        row["selected_test_artifact"] = {
            "path": str(executable),
            "expected_sha256": diagnostic_artifact["sha256"],
            "before_sha256": child_before,
            "after_sha256": None,
            "unchanged": False,
        }
        row["state"] = "RUNNING"
        save(out / "restored-child-owner-outcomes.json", restored_child_rows)
        print("RUN " + group_label + " " + selector, flush=True)
        receipt = execute(
            [
                str(executable),
                selector,
                "--exact",
                "--nocapture",
                "--test-threads=1",
            ],
            prefix,
            executable,
            180,
            runtime_env,
            remaining,
        )
        processes.append(receipt)
        remaining -= receipt["retained_bytes"]
        child_after = sha(executable)
        row["selected_test_artifact"]["after_sha256"] = child_after
        row["selected_test_artifact"]["unchanged"] = (
            child_before == child_after == diagnostic_artifact["sha256"]
        )
        save(out / "restored-child-owner-outcomes.json", restored_child_rows)
        require(
            row["selected_test_artifact"]["unchanged"],
            "selected Rust test artifact changed during transport",
        )
        transport_path = out / (prefix + ".transport.json")
        if lifecycle(receipt):
            require(
                transport_path.is_file(), "completed test transport summary missing"
            )
            transport = json.loads(transport_path.read_text())
            encoded_stderr = (out / (prefix + ".stderr")).read_bytes()
            repeat_stage = transport["repeat_stage"]
            table_stage = transport["table_stage"]
            require(
                transport["child_exit_code"] == receipt["exit_code"]
                and transport["ordinary_stderr_verbatim"] is False
                and transport["stderr_losslessly_reconstructible_after_table_filter"]
                is True
                and table_stage["ordinary_stderr_verbatim"] is True
                and table_stage["stdout_verbatim"] is True
                and transport["stdout_verbatim"] is True
                and transport["retained_stderr_bytes"]
                == repeat_stage["output_bytes"]
                == len(encoded_stderr)
                and repeat_stage["format"]
                == "two-family-lossless-line-dictionary-rle-v1"
                and repeat_stage["input_bytes"] == table_stage["retained_stderr_bytes"]
                and repeat_stage["output_sha256"]
                == hashlib.sha256(encoded_stderr).hexdigest(),
                "transport child status or layered byte census mismatch",
            )
            reconstructed = transport_filter.decode_diagnostic_repeats(
                encoded_stderr, lambda _data: None, repeat_stage["input_bytes"]
            )
            require(
                reconstructed["bytes"] == repeat_stage["input_bytes"]
                and reconstructed["sha256"] == repeat_stage["input_sha256"],
                "transport diagnostic reconstruction differs from original stream",
            )
            row["transport_reconstruction"] = reconstructed
        row.update(
            exit_code=receipt["exit_code"],
            receipt=prefix + ".process.json",
            stdout=prefix + ".stdout",
            stderr=prefix + ".stderr",
        )
        require(lifecycle(receipt), "original test lifecycle/capture stop: " + selector)
        stdout = (out / (prefix + ".stdout")).read_text(errors="replace")
        try:
            row["state"] = original_result(stdout, receipt["exit_code"])
        except RuntimeError:
            row["state"] = "SIGNAL" if receipt["exit_code"] < 0 else "PROTOCOL_FAILURE"
            save(out / "restored-child-owner-outcomes.json", restored_child_rows)
            raise
        save(out / "restored-child-owner-outcomes.json", restored_child_rows)
        require(
            sha(executable) == diagnostic_artifact["sha256"],
            "selected test artifact changed",
        )


def run_native_string_effect_controls() -> None:
    global remaining
    require(
        all(
            row["state"] in ["PASS", "FAIL"]
            for row in rows + logical_body_rows + codec_rows + named_child_rows + restored_child_rows
        ),
        "native string effects controls cannot precede original119 closure",
    )
    require(
        files(dependency_roots) == dependencies_before
        and source() == before
        and files(provider_roots) == providers_before
        and files(list(tools.values())) == tool_pins,
        "pre-logical-body-entry source/provider/tool/dependency drift",
    )
    original_lib_roster = json.loads((out / "roster-tcl_compiler.json").read_text())
    require(
        {row["selector"] for row in native_string_rows}.issubset(
            original_lib_roster["selected"]
        ),
        "native string effects controls absent from actual fresh compiler lib inventory",
    )
    for index, row in enumerate(native_string_rows):
        package, target_kind, target_name = row["package"], row["kind"], row["target"]
        selected_artifacts = [
            value
            for value in artifacts
            if value["package"] == package
            and value["kind"] == target_kind
            and value["target"] == target_name
        ]
        require(
            len(selected_artifacts) == 1,
            "native string effects current built artifact absent/ambiguous",
        )
        diagnostic_artifact: dict[str, Any] = selected_artifacts[0]
        require(
            diagnostic_artifact["strict_Rust_0w"] is True
            and diagnostic_artifact["cargo_event"]["profile"]["debug_assertions"]
            is True,
            "native string effects requires existing fresh strict debug-assertion test profile",
        )
        executable = Path(diagnostic_artifact["path"])
        require(
            sha(executable) == diagnostic_artifact["sha256"],
            "native string effects artifact changed before listing",
        )
        selector = row["selector"]
        require(remaining > 512 * 1024, "insufficient log budget before original test")
        prefix = row["group"] + "-" + target_name + "-" + str(index).zfill(2)
        group_label = "native-string-effects3"
        print("SELECT " + group_label + " " + selector, flush=True)
        runtime_env = env.copy()
        selected_env = row["env"]
        if not isinstance(selected_env, dict):
            raise TypeError("original per-selector environment is not a mapping")
        for name, value in selected_env.items():
            if not isinstance(name, str) or not isinstance(value, str):
                raise TypeError("original per-selector environment is not text")
            runtime_env[name] = value
        selected_list = execute(
            [str(executable), selector, "--exact", "--list", "--format", "terse"],
            "select-" + prefix,
            executable,
            180,
            runtime_env,
            remaining,
        )
        processes.append(selected_list)
        remaining -= selected_list["retained_bytes"]
        require(
            lifecycle(selected_list) and selected_list["exit_code"] == 0,
            "original exact selector list failed or unclosed",
        )
        selected_lines = (
            (out / ("select-" + prefix + ".stdout")).read_text().splitlines()
        )
        require(
            [line for line in selected_lines if line.endswith(": test")]
            == [selector + ": test"],
            "original exact selector list mismatch",
        )
        require(
            all(
                line == selector + ": test"
                or not line
                or re.fullmatch(r"1 tests?, 0 benchmarks?", line)
                for line in selected_lines
            ),
            "unexpected original exact selector list data",
        )
        require(
            remaining > 512 * 1024,
            "insufficient log budget after original exact selector list",
        )
        child_before = sha(executable)
        require(
            child_before == diagnostic_artifact["sha256"],
            "selected Rust test artifact changed before transport spawn",
        )
        row["selected_test_artifact"] = {
            "path": str(executable),
            "expected_sha256": diagnostic_artifact["sha256"],
            "before_sha256": child_before,
            "after_sha256": None,
            "unchanged": False,
        }
        row["state"] = "RUNNING"
        save(out / "native-string-effects-outcomes.json", native_string_rows)
        print("RUN " + group_label + " " + selector, flush=True)
        receipt = execute(
            [
                str(executable),
                selector,
                "--exact",
                "--nocapture",
                "--test-threads=1",
            ],
            prefix,
            executable,
            180,
            runtime_env,
            remaining,
        )
        processes.append(receipt)
        remaining -= receipt["retained_bytes"]
        child_after = sha(executable)
        row["selected_test_artifact"]["after_sha256"] = child_after
        row["selected_test_artifact"]["unchanged"] = (
            child_before == child_after == diagnostic_artifact["sha256"]
        )
        save(out / "native-string-effects-outcomes.json", native_string_rows)
        require(
            row["selected_test_artifact"]["unchanged"],
            "selected Rust test artifact changed during transport",
        )
        transport_path = out / (prefix + ".transport.json")
        if lifecycle(receipt):
            require(
                transport_path.is_file(), "completed test transport summary missing"
            )
            transport = json.loads(transport_path.read_text())
            encoded_stderr = (out / (prefix + ".stderr")).read_bytes()
            repeat_stage = transport["repeat_stage"]
            table_stage = transport["table_stage"]
            require(
                transport["child_exit_code"] == receipt["exit_code"]
                and transport["ordinary_stderr_verbatim"] is False
                and transport["stderr_losslessly_reconstructible_after_table_filter"]
                is True
                and table_stage["ordinary_stderr_verbatim"] is True
                and table_stage["stdout_verbatim"] is True
                and transport["stdout_verbatim"] is True
                and transport["retained_stderr_bytes"]
                == repeat_stage["output_bytes"]
                == len(encoded_stderr)
                and repeat_stage["format"]
                == "two-family-lossless-line-dictionary-rle-v1"
                and repeat_stage["input_bytes"] == table_stage["retained_stderr_bytes"]
                and repeat_stage["output_sha256"]
                == hashlib.sha256(encoded_stderr).hexdigest(),
                "transport child status or layered byte census mismatch",
            )
            reconstructed = transport_filter.decode_diagnostic_repeats(
                encoded_stderr, lambda _data: None, repeat_stage["input_bytes"]
            )
            require(
                reconstructed["bytes"] == repeat_stage["input_bytes"]
                and reconstructed["sha256"] == repeat_stage["input_sha256"],
                "transport diagnostic reconstruction differs from original stream",
            )
            row["transport_reconstruction"] = reconstructed
        row.update(
            exit_code=receipt["exit_code"],
            receipt=prefix + ".process.json",
            stdout=prefix + ".stdout",
            stderr=prefix + ".stderr",
        )
        require(lifecycle(receipt), "original test lifecycle/capture stop: " + selector)
        stdout = (out / (prefix + ".stdout")).read_text(errors="replace")
        try:
            row["state"] = original_result(stdout, receipt["exit_code"])
        except RuntimeError:
            row["state"] = "SIGNAL" if receipt["exit_code"] < 0 else "PROTOCOL_FAILURE"
            save(out / "native-string-effects-outcomes.json", native_string_rows)
            raise
        save(out / "native-string-effects-outcomes.json", native_string_rows)
        require(
            sha(executable) == diagnostic_artifact["sha256"],
            "selected test artifact changed",
        )


def run_native_formal_layout_controls() -> None:
    global remaining
    require(
        all(
            row["state"] in ["PASS", "FAIL"]
            for row in rows + logical_body_rows + codec_rows + named_child_rows + restored_child_rows + native_string_rows
        ),
        "native formal layout controls cannot precede original122 closure",
    )
    require(
        files(dependency_roots) == dependencies_before
        and source() == before
        and files(provider_roots) == providers_before
        and files(list(tools.values())) == tool_pins,
        "pre-logical-body-entry source/provider/tool/dependency drift",
    )
    original_lib_roster = json.loads((out / "roster-tcl_compiler.json").read_text())
    require(
        {row["selector"] for row in native_formal_rows}.issubset(
            original_lib_roster["selected"]
        ),
        "native formal layout controls absent from actual fresh compiler lib inventory",
    )
    for index, row in enumerate(native_formal_rows):
        package, target_kind, target_name = row["package"], row["kind"], row["target"]
        selected_artifacts = [
            value
            for value in artifacts
            if value["package"] == package
            and value["kind"] == target_kind
            and value["target"] == target_name
        ]
        require(
            len(selected_artifacts) == 1,
            "native formal layout current built artifact absent/ambiguous",
        )
        diagnostic_artifact: dict[str, Any] = selected_artifacts[0]
        require(
            diagnostic_artifact["strict_Rust_0w"] is True
            and diagnostic_artifact["cargo_event"]["profile"]["debug_assertions"]
            is True,
            "native formal layout requires existing fresh strict debug-assertion test profile",
        )
        executable = Path(diagnostic_artifact["path"])
        require(
            sha(executable) == diagnostic_artifact["sha256"],
            "native formal layout artifact changed before listing",
        )
        selector = row["selector"]
        require(remaining > 512 * 1024, "insufficient log budget before original test")
        prefix = row["group"] + "-" + target_name + "-" + str(index).zfill(2)
        group_label = "native-formal-layout2"
        print("SELECT " + group_label + " " + selector, flush=True)
        runtime_env = env.copy()
        selected_env = row["env"]
        if not isinstance(selected_env, dict):
            raise TypeError("original per-selector environment is not a mapping")
        for name, value in selected_env.items():
            if not isinstance(name, str) or not isinstance(value, str):
                raise TypeError("original per-selector environment is not text")
            runtime_env[name] = value
        selected_list = execute(
            [str(executable), selector, "--exact", "--list", "--format", "terse"],
            "select-" + prefix,
            executable,
            180,
            runtime_env,
            remaining,
        )
        processes.append(selected_list)
        remaining -= selected_list["retained_bytes"]
        require(
            lifecycle(selected_list) and selected_list["exit_code"] == 0,
            "original exact selector list failed or unclosed",
        )
        selected_lines = (
            (out / ("select-" + prefix + ".stdout")).read_text().splitlines()
        )
        require(
            [line for line in selected_lines if line.endswith(": test")]
            == [selector + ": test"],
            "original exact selector list mismatch",
        )
        require(
            all(
                line == selector + ": test"
                or not line
                or re.fullmatch(r"1 tests?, 0 benchmarks?", line)
                for line in selected_lines
            ),
            "unexpected original exact selector list data",
        )
        require(
            remaining > 512 * 1024,
            "insufficient log budget after original exact selector list",
        )
        child_before = sha(executable)
        require(
            child_before == diagnostic_artifact["sha256"],
            "selected Rust test artifact changed before transport spawn",
        )
        row["selected_test_artifact"] = {
            "path": str(executable),
            "expected_sha256": diagnostic_artifact["sha256"],
            "before_sha256": child_before,
            "after_sha256": None,
            "unchanged": False,
        }
        row["state"] = "RUNNING"
        save(out / "native-formal-layout-outcomes.json", native_formal_rows)
        print("RUN " + group_label + " " + selector, flush=True)
        receipt = execute(
            [
                str(executable),
                selector,
                "--exact",
                "--nocapture",
                "--test-threads=1",
            ],
            prefix,
            executable,
            180,
            runtime_env,
            remaining,
        )
        processes.append(receipt)
        remaining -= receipt["retained_bytes"]
        child_after = sha(executable)
        row["selected_test_artifact"]["after_sha256"] = child_after
        row["selected_test_artifact"]["unchanged"] = (
            child_before == child_after == diagnostic_artifact["sha256"]
        )
        save(out / "native-formal-layout-outcomes.json", native_formal_rows)
        require(
            row["selected_test_artifact"]["unchanged"],
            "selected Rust test artifact changed during transport",
        )
        transport_path = out / (prefix + ".transport.json")
        if lifecycle(receipt):
            require(
                transport_path.is_file(), "completed test transport summary missing"
            )
            transport = json.loads(transport_path.read_text())
            encoded_stderr = (out / (prefix + ".stderr")).read_bytes()
            repeat_stage = transport["repeat_stage"]
            table_stage = transport["table_stage"]
            require(
                transport["child_exit_code"] == receipt["exit_code"]
                and transport["ordinary_stderr_verbatim"] is False
                and transport["stderr_losslessly_reconstructible_after_table_filter"]
                is True
                and table_stage["ordinary_stderr_verbatim"] is True
                and table_stage["stdout_verbatim"] is True
                and transport["stdout_verbatim"] is True
                and transport["retained_stderr_bytes"]
                == repeat_stage["output_bytes"]
                == len(encoded_stderr)
                and repeat_stage["format"]
                == "two-family-lossless-line-dictionary-rle-v1"
                and repeat_stage["input_bytes"] == table_stage["retained_stderr_bytes"]
                and repeat_stage["output_sha256"]
                == hashlib.sha256(encoded_stderr).hexdigest(),
                "transport child status or layered byte census mismatch",
            )
            reconstructed = transport_filter.decode_diagnostic_repeats(
                encoded_stderr, lambda _data: None, repeat_stage["input_bytes"]
            )
            require(
                reconstructed["bytes"] == repeat_stage["input_bytes"]
                and reconstructed["sha256"] == repeat_stage["input_sha256"],
                "transport diagnostic reconstruction differs from original stream",
            )
            row["transport_reconstruction"] = reconstructed
        row.update(
            exit_code=receipt["exit_code"],
            receipt=prefix + ".process.json",
            stdout=prefix + ".stdout",
            stderr=prefix + ".stderr",
        )
        require(lifecycle(receipt), "original test lifecycle/capture stop: " + selector)
        stdout = (out / (prefix + ".stdout")).read_text(errors="replace")
        try:
            row["state"] = original_result(stdout, receipt["exit_code"])
        except RuntimeError:
            row["state"] = "SIGNAL" if receipt["exit_code"] < 0 else "PROTOCOL_FAILURE"
            save(out / "native-formal-layout-outcomes.json", native_formal_rows)
            raise
        save(out / "native-formal-layout-outcomes.json", native_formal_rows)
        require(
            sha(executable) == diagnostic_artifact["sha256"],
            "selected test artifact changed",
        )


def run_analyser_layout_controls() -> None:
    global remaining
    require(
        all(
            row["state"] in ["PASS", "FAIL"]
            for row in rows + logical_body_rows + codec_rows + named_child_rows + restored_child_rows + native_string_rows + native_formal_rows
        ),
        "analyser layout controls cannot precede original124 closure",
    )
    require(
        files(dependency_roots) == dependencies_before
        and source() == before
        and files(provider_roots) == providers_before
        and files(list(tools.values())) == tool_pins,
        "pre-logical-body-entry source/provider/tool/dependency drift",
    )
    original_lib_roster = json.loads((out / "roster-tcl_compiler.json").read_text())
    require(
        {row["selector"] for row in analyser_layout_rows}.issubset(
            original_lib_roster["selected"]
        ),
        "analyser layout controls absent from actual fresh compiler lib inventory",
    )
    for index, row in enumerate(analyser_layout_rows):
        package, target_kind, target_name = row["package"], row["kind"], row["target"]
        selected_artifacts = [
            value
            for value in artifacts
            if value["package"] == package
            and value["kind"] == target_kind
            and value["target"] == target_name
        ]
        require(
            len(selected_artifacts) == 1,
            "analyser layout current built artifact absent/ambiguous",
        )
        diagnostic_artifact: dict[str, Any] = selected_artifacts[0]
        require(
            diagnostic_artifact["strict_Rust_0w"] is True
            and diagnostic_artifact["cargo_event"]["profile"]["debug_assertions"]
            is True,
            "analyser layout requires existing fresh strict debug-assertion test profile",
        )
        executable = Path(diagnostic_artifact["path"])
        require(
            sha(executable) == diagnostic_artifact["sha256"],
            "analyser layout artifact changed before listing",
        )
        selector = row["selector"]
        require(remaining > 512 * 1024, "insufficient log budget before original test")
        prefix = row["group"] + "-" + target_name + "-" + str(index).zfill(2)
        group_label = "analyser-layout3"
        print("SELECT " + group_label + " " + selector, flush=True)
        runtime_env = env.copy()
        selected_env = row["env"]
        if not isinstance(selected_env, dict):
            raise TypeError("original per-selector environment is not a mapping")
        for name, value in selected_env.items():
            if not isinstance(name, str) or not isinstance(value, str):
                raise TypeError("original per-selector environment is not text")
            runtime_env[name] = value
        selected_list = execute(
            [str(executable), selector, "--exact", "--list", "--format", "terse"],
            "select-" + prefix,
            executable,
            180,
            runtime_env,
            remaining,
        )
        processes.append(selected_list)
        remaining -= selected_list["retained_bytes"]
        require(
            lifecycle(selected_list) and selected_list["exit_code"] == 0,
            "original exact selector list failed or unclosed",
        )
        selected_lines = (
            (out / ("select-" + prefix + ".stdout")).read_text().splitlines()
        )
        require(
            [line for line in selected_lines if line.endswith(": test")]
            == [selector + ": test"],
            "original exact selector list mismatch",
        )
        require(
            all(
                line == selector + ": test"
                or not line
                or re.fullmatch(r"1 tests?, 0 benchmarks?", line)
                for line in selected_lines
            ),
            "unexpected original exact selector list data",
        )
        require(
            remaining > 512 * 1024,
            "insufficient log budget after original exact selector list",
        )
        child_before = sha(executable)
        require(
            child_before == diagnostic_artifact["sha256"],
            "selected Rust test artifact changed before transport spawn",
        )
        row["selected_test_artifact"] = {
            "path": str(executable),
            "expected_sha256": diagnostic_artifact["sha256"],
            "before_sha256": child_before,
            "after_sha256": None,
            "unchanged": False,
        }
        row["state"] = "RUNNING"
        save(out / "analyser-layout-outcomes.json", analyser_layout_rows)
        print("RUN " + group_label + " " + selector, flush=True)
        receipt = execute(
            [
                str(executable),
                selector,
                "--exact",
                "--nocapture",
                "--test-threads=1",
            ],
            prefix,
            executable,
            180,
            runtime_env,
            remaining,
        )
        processes.append(receipt)
        remaining -= receipt["retained_bytes"]
        child_after = sha(executable)
        row["selected_test_artifact"]["after_sha256"] = child_after
        row["selected_test_artifact"]["unchanged"] = (
            child_before == child_after == diagnostic_artifact["sha256"]
        )
        save(out / "analyser-layout-outcomes.json", analyser_layout_rows)
        require(
            row["selected_test_artifact"]["unchanged"],
            "selected Rust test artifact changed during transport",
        )
        transport_path = out / (prefix + ".transport.json")
        if lifecycle(receipt):
            require(
                transport_path.is_file(), "completed test transport summary missing"
            )
            transport = json.loads(transport_path.read_text())
            encoded_stderr = (out / (prefix + ".stderr")).read_bytes()
            repeat_stage = transport["repeat_stage"]
            table_stage = transport["table_stage"]
            require(
                transport["child_exit_code"] == receipt["exit_code"]
                and transport["ordinary_stderr_verbatim"] is False
                and transport["stderr_losslessly_reconstructible_after_table_filter"]
                is True
                and table_stage["ordinary_stderr_verbatim"] is True
                and table_stage["stdout_verbatim"] is True
                and transport["stdout_verbatim"] is True
                and transport["retained_stderr_bytes"]
                == repeat_stage["output_bytes"]
                == len(encoded_stderr)
                and repeat_stage["format"]
                == "two-family-lossless-line-dictionary-rle-v1"
                and repeat_stage["input_bytes"] == table_stage["retained_stderr_bytes"]
                and repeat_stage["output_sha256"]
                == hashlib.sha256(encoded_stderr).hexdigest(),
                "transport child status or layered byte census mismatch",
            )
            reconstructed = transport_filter.decode_diagnostic_repeats(
                encoded_stderr, lambda _data: None, repeat_stage["input_bytes"]
            )
            require(
                reconstructed["bytes"] == repeat_stage["input_bytes"]
                and reconstructed["sha256"] == repeat_stage["input_sha256"],
                "transport diagnostic reconstruction differs from original stream",
            )
            row["transport_reconstruction"] = reconstructed
        row.update(
            exit_code=receipt["exit_code"],
            receipt=prefix + ".process.json",
            stdout=prefix + ".stdout",
            stderr=prefix + ".stderr",
        )
        require(lifecycle(receipt), "original test lifecycle/capture stop: " + selector)
        stdout = (out / (prefix + ".stdout")).read_text(errors="replace")
        try:
            row["state"] = original_result(stdout, receipt["exit_code"])
        except RuntimeError:
            row["state"] = "SIGNAL" if receipt["exit_code"] < 0 else "PROTOCOL_FAILURE"
            save(out / "analyser-layout-outcomes.json", analyser_layout_rows)
            raise
        save(out / "analyser-layout-outcomes.json", analyser_layout_rows)
        require(
            sha(executable) == diagnostic_artifact["sha256"],
            "selected test artifact changed",
        )


def run_own_jim_member_controls() -> None:
    global remaining
    require(
        all(
            row["state"] in ["PASS", "FAIL"]
            for row in rows + logical_body_rows + codec_rows + named_child_rows + restored_child_rows + native_string_rows + native_formal_rows + analyser_layout_rows
        ),
        "own Jim member controls cannot precede original127 closure",
    )
    require(
        files(dependency_roots) == dependencies_before
        and source() == before
        and files(provider_roots) == providers_before
        and files(list(tools.values())) == tool_pins,
        "pre-logical-body-entry source/provider/tool/dependency drift",
    )
    original_lib_roster = json.loads((out / "roster-tcl_compiler.json").read_text())
    require(
        {row["selector"] for row in own_jim_member_rows}.issubset(
            original_lib_roster["selected"]
        ),
        "own Jim member controls absent from actual fresh compiler lib inventory",
    )
    for index, row in enumerate(own_jim_member_rows):
        package, target_kind, target_name = row["package"], row["kind"], row["target"]
        selected_artifacts = [
            value
            for value in artifacts
            if value["package"] == package
            and value["kind"] == target_kind
            and value["target"] == target_name
        ]
        require(
            len(selected_artifacts) == 1,
            "own Jim member current built artifact absent/ambiguous",
        )
        diagnostic_artifact: dict[str, Any] = selected_artifacts[0]
        require(
            diagnostic_artifact["strict_Rust_0w"] is True
            and diagnostic_artifact["cargo_event"]["profile"]["debug_assertions"]
            is True,
            "own Jim member requires existing fresh strict debug-assertion test profile",
        )
        executable = Path(diagnostic_artifact["path"])
        require(
            sha(executable) == diagnostic_artifact["sha256"],
            "own Jim member artifact changed before listing",
        )
        selector = row["selector"]
        require(remaining > 512 * 1024, "insufficient log budget before original test")
        prefix = row["group"] + "-" + target_name + "-" + str(index).zfill(2)
        group_label = "own-jim-member3"
        print("SELECT " + group_label + " " + selector, flush=True)
        runtime_env = env.copy()
        selected_env = row["env"]
        if not isinstance(selected_env, dict):
            raise TypeError("original per-selector environment is not a mapping")
        for name, value in selected_env.items():
            if not isinstance(name, str) or not isinstance(value, str):
                raise TypeError("original per-selector environment is not text")
            runtime_env[name] = value
        selected_list = execute(
            [str(executable), selector, "--exact", "--list", "--format", "terse"],
            "select-" + prefix,
            executable,
            180,
            runtime_env,
            remaining,
        )
        processes.append(selected_list)
        remaining -= selected_list["retained_bytes"]
        require(
            lifecycle(selected_list) and selected_list["exit_code"] == 0,
            "original exact selector list failed or unclosed",
        )
        selected_lines = (
            (out / ("select-" + prefix + ".stdout")).read_text().splitlines()
        )
        require(
            [line for line in selected_lines if line.endswith(": test")]
            == [selector + ": test"],
            "original exact selector list mismatch",
        )
        require(
            all(
                line == selector + ": test"
                or not line
                or re.fullmatch(r"1 tests?, 0 benchmarks?", line)
                for line in selected_lines
            ),
            "unexpected original exact selector list data",
        )
        require(
            remaining > 512 * 1024,
            "insufficient log budget after original exact selector list",
        )
        child_before = sha(executable)
        require(
            child_before == diagnostic_artifact["sha256"],
            "selected Rust test artifact changed before transport spawn",
        )
        row["selected_test_artifact"] = {
            "path": str(executable),
            "expected_sha256": diagnostic_artifact["sha256"],
            "before_sha256": child_before,
            "after_sha256": None,
            "unchanged": False,
        }
        row["state"] = "RUNNING"
        save(out / "own-jim-member-outcomes.json", own_jim_member_rows)
        print("RUN " + group_label + " " + selector, flush=True)
        receipt = execute(
            [
                str(executable),
                selector,
                "--exact",
                "--nocapture",
                "--test-threads=1",
            ],
            prefix,
            executable,
            180,
            runtime_env,
            remaining,
        )
        processes.append(receipt)
        remaining -= receipt["retained_bytes"]
        child_after = sha(executable)
        row["selected_test_artifact"]["after_sha256"] = child_after
        row["selected_test_artifact"]["unchanged"] = (
            child_before == child_after == diagnostic_artifact["sha256"]
        )
        save(out / "own-jim-member-outcomes.json", own_jim_member_rows)
        require(
            row["selected_test_artifact"]["unchanged"],
            "selected Rust test artifact changed during transport",
        )
        transport_path = out / (prefix + ".transport.json")
        if lifecycle(receipt):
            require(
                transport_path.is_file(), "completed test transport summary missing"
            )
            transport = json.loads(transport_path.read_text())
            encoded_stderr = (out / (prefix + ".stderr")).read_bytes()
            repeat_stage = transport["repeat_stage"]
            table_stage = transport["table_stage"]
            require(
                transport["child_exit_code"] == receipt["exit_code"]
                and transport["ordinary_stderr_verbatim"] is False
                and transport["stderr_losslessly_reconstructible_after_table_filter"]
                is True
                and table_stage["ordinary_stderr_verbatim"] is True
                and table_stage["stdout_verbatim"] is True
                and transport["stdout_verbatim"] is True
                and transport["retained_stderr_bytes"]
                == repeat_stage["output_bytes"]
                == len(encoded_stderr)
                and repeat_stage["format"]
                == "two-family-lossless-line-dictionary-rle-v1"
                and repeat_stage["input_bytes"] == table_stage["retained_stderr_bytes"]
                and repeat_stage["output_sha256"]
                == hashlib.sha256(encoded_stderr).hexdigest(),
                "transport child status or layered byte census mismatch",
            )
            reconstructed = transport_filter.decode_diagnostic_repeats(
                encoded_stderr, lambda _data: None, repeat_stage["input_bytes"]
            )
            require(
                reconstructed["bytes"] == repeat_stage["input_bytes"]
                and reconstructed["sha256"] == repeat_stage["input_sha256"],
                "transport diagnostic reconstruction differs from original stream",
            )
            row["transport_reconstruction"] = reconstructed
        row.update(
            exit_code=receipt["exit_code"],
            receipt=prefix + ".process.json",
            stdout=prefix + ".stdout",
            stderr=prefix + ".stderr",
        )
        require(lifecycle(receipt), "original test lifecycle/capture stop: " + selector)
        stdout = (out / (prefix + ".stdout")).read_text(errors="replace")
        try:
            row["state"] = original_result(stdout, receipt["exit_code"])
        except RuntimeError:
            row["state"] = "SIGNAL" if receipt["exit_code"] < 0 else "PROTOCOL_FAILURE"
            save(out / "own-jim-member-outcomes.json", own_jim_member_rows)
            raise
        save(out / "own-jim-member-outcomes.json", own_jim_member_rows)
        require(
            sha(executable) == diagnostic_artifact["sha256"],
            "selected test artifact changed",
        )


def run_advice_catalogue_controls() -> None:
    global remaining
    require(
        all(
            row["state"] in ["PASS", "FAIL"]
            for row in rows + logical_body_rows + codec_rows + named_child_rows + restored_child_rows + native_string_rows + native_formal_rows + analyser_layout_rows + own_jim_member_rows
        ),
        "advice catalogue controls cannot precede original130 closure",
    )
    require(
        files(dependency_roots) == dependencies_before
        and source() == before
        and files(provider_roots) == providers_before
        and files(list(tools.values())) == tool_pins,
        "pre-logical-body-entry source/provider/tool/dependency drift",
    )
    original_lib_roster = json.loads((out / "roster-tcl_compiler.json").read_text())
    require(
        {row["selector"] for row in advice_catalogue_rows}.issubset(
            original_lib_roster["selected"]
        ),
        "advice catalogue controls absent from actual fresh compiler lib inventory",
    )
    for index, row in enumerate(advice_catalogue_rows):
        package, target_kind, target_name = row["package"], row["kind"], row["target"]
        selected_artifacts = [
            value
            for value in artifacts
            if value["package"] == package
            and value["kind"] == target_kind
            and value["target"] == target_name
        ]
        require(
            len(selected_artifacts) == 1,
            "advice catalogue current built artifact absent/ambiguous",
        )
        diagnostic_artifact: dict[str, Any] = selected_artifacts[0]
        require(
            diagnostic_artifact["strict_Rust_0w"] is True
            and diagnostic_artifact["cargo_event"]["profile"]["debug_assertions"]
            is True,
            "advice catalogue requires existing fresh strict debug-assertion test profile",
        )
        executable = Path(diagnostic_artifact["path"])
        require(
            sha(executable) == diagnostic_artifact["sha256"],
            "advice catalogue artifact changed before listing",
        )
        selector = row["selector"]
        require(remaining > 512 * 1024, "insufficient log budget before original test")
        prefix = row["group"] + "-" + target_name + "-" + str(index).zfill(2)
        group_label = "advice-catalogue3"
        print("SELECT " + group_label + " " + selector, flush=True)
        runtime_env = env.copy()
        selected_env = row["env"]
        if not isinstance(selected_env, dict):
            raise TypeError("original per-selector environment is not a mapping")
        for name, value in selected_env.items():
            if not isinstance(name, str) or not isinstance(value, str):
                raise TypeError("original per-selector environment is not text")
            runtime_env[name] = value
        selected_list = execute(
            [str(executable), selector, "--exact", "--list", "--format", "terse"],
            "select-" + prefix,
            executable,
            180,
            runtime_env,
            remaining,
        )
        processes.append(selected_list)
        remaining -= selected_list["retained_bytes"]
        require(
            lifecycle(selected_list) and selected_list["exit_code"] == 0,
            "original exact selector list failed or unclosed",
        )
        selected_lines = (
            (out / ("select-" + prefix + ".stdout")).read_text().splitlines()
        )
        require(
            [line for line in selected_lines if line.endswith(": test")]
            == [selector + ": test"],
            "original exact selector list mismatch",
        )
        require(
            all(
                line == selector + ": test"
                or not line
                or re.fullmatch(r"1 tests?, 0 benchmarks?", line)
                for line in selected_lines
            ),
            "unexpected original exact selector list data",
        )
        require(
            remaining > 512 * 1024,
            "insufficient log budget after original exact selector list",
        )
        child_before = sha(executable)
        require(
            child_before == diagnostic_artifact["sha256"],
            "selected Rust test artifact changed before transport spawn",
        )
        row["selected_test_artifact"] = {
            "path": str(executable),
            "expected_sha256": diagnostic_artifact["sha256"],
            "before_sha256": child_before,
            "after_sha256": None,
            "unchanged": False,
        }
        row["state"] = "RUNNING"
        save(out / "advice-catalogue-outcomes.json", advice_catalogue_rows)
        print("RUN " + group_label + " " + selector, flush=True)
        receipt = execute(
            [
                str(executable),
                selector,
                "--exact",
                "--nocapture",
                "--test-threads=1",
            ],
            prefix,
            executable,
            180,
            runtime_env,
            remaining,
        )
        processes.append(receipt)
        remaining -= receipt["retained_bytes"]
        child_after = sha(executable)
        row["selected_test_artifact"]["after_sha256"] = child_after
        row["selected_test_artifact"]["unchanged"] = (
            child_before == child_after == diagnostic_artifact["sha256"]
        )
        save(out / "advice-catalogue-outcomes.json", advice_catalogue_rows)
        require(
            row["selected_test_artifact"]["unchanged"],
            "selected Rust test artifact changed during transport",
        )
        transport_path = out / (prefix + ".transport.json")
        if lifecycle(receipt):
            require(
                transport_path.is_file(), "completed test transport summary missing"
            )
            transport = json.loads(transport_path.read_text())
            encoded_stderr = (out / (prefix + ".stderr")).read_bytes()
            repeat_stage = transport["repeat_stage"]
            table_stage = transport["table_stage"]
            require(
                transport["child_exit_code"] == receipt["exit_code"]
                and transport["ordinary_stderr_verbatim"] is False
                and transport["stderr_losslessly_reconstructible_after_table_filter"]
                is True
                and table_stage["ordinary_stderr_verbatim"] is True
                and table_stage["stdout_verbatim"] is True
                and transport["stdout_verbatim"] is True
                and transport["retained_stderr_bytes"]
                == repeat_stage["output_bytes"]
                == len(encoded_stderr)
                and repeat_stage["format"]
                == "two-family-lossless-line-dictionary-rle-v1"
                and repeat_stage["input_bytes"] == table_stage["retained_stderr_bytes"]
                and repeat_stage["output_sha256"]
                == hashlib.sha256(encoded_stderr).hexdigest(),
                "transport child status or layered byte census mismatch",
            )
            reconstructed = transport_filter.decode_diagnostic_repeats(
                encoded_stderr, lambda _data: None, repeat_stage["input_bytes"]
            )
            require(
                reconstructed["bytes"] == repeat_stage["input_bytes"]
                and reconstructed["sha256"] == repeat_stage["input_sha256"],
                "transport diagnostic reconstruction differs from original stream",
            )
            row["transport_reconstruction"] = reconstructed
        row.update(
            exit_code=receipt["exit_code"],
            receipt=prefix + ".process.json",
            stdout=prefix + ".stdout",
            stderr=prefix + ".stderr",
        )
        require(lifecycle(receipt), "original test lifecycle/capture stop: " + selector)
        stdout = (out / (prefix + ".stdout")).read_text(errors="replace")
        try:
            row["state"] = original_result(stdout, receipt["exit_code"])
        except RuntimeError:
            row["state"] = "SIGNAL" if receipt["exit_code"] < 0 else "PROTOCOL_FAILURE"
            save(out / "advice-catalogue-outcomes.json", advice_catalogue_rows)
            raise
        save(out / "advice-catalogue-outcomes.json", advice_catalogue_rows)
        require(
            sha(executable) == diagnostic_artifact["sha256"],
            "selected test artifact changed",
        )


def run_inherited_point_report_controls() -> None:
    global remaining
    require(
        all(
            row["state"] in ["PASS", "FAIL"]
            for row in rows + logical_body_rows + codec_rows + named_child_rows + restored_child_rows + native_string_rows + native_formal_rows + analyser_layout_rows + own_jim_member_rows + advice_catalogue_rows + native_eligibility_rows + own_jim_alias_rows + source_point_memo_rows + analyser_layout_diagnostic_rows + source_point_report_rows
        ),
        "inherited point report custody controls cannot precede existing report-repeat closure",
    )
    require(
        files(dependency_roots) == dependencies_before
        and source() == before
        and files(provider_roots) == providers_before
        and files(list(tools.values())) == tool_pins,
        "pre-logical-body-entry source/provider/tool/dependency drift",
    )
    original_lib_roster = json.loads((out / "roster-tcl_compiler.json").read_text())
    require(
        {row["selector"] for row in inherited_report_rows}.issubset(
            original_lib_roster["selected"]
        ),
        "inherited point report custody controls absent from actual fresh compiler lib inventory",
    )
    for pinned_path in inherited_report_validation["source_keys"]:
        require(sha(root / pinned_path) == logical_validation["source_pins"][pinned_path],
                "inherited point report custody source drift: " + pinned_path)
    for index, row in enumerate(inherited_report_rows):
        require(deadline - time.time() >= 180 + 180 + 40,
                "insufficient full inherited-report list/test/cleanup budget")
        package, target_kind, target_name = row["package"], row["kind"], row["target"]
        selected_artifacts = [
            value
            for value in artifacts
            if value["package"] == package
            and value["kind"] == target_kind
            and value["target"] == target_name
        ]
        require(
            len(selected_artifacts) == 1,
            "inherited point report custody current built artifact absent/ambiguous",
        )
        diagnostic_artifact: dict[str, Any] = selected_artifacts[0]
        require(
            diagnostic_artifact["strict_Rust_0w"] is True
            and diagnostic_artifact["cargo_event"]["profile"]["debug_assertions"]
            is True,
            "inherited point report custody requires existing fresh strict debug-assertion test profile",
        )
        executable = Path(diagnostic_artifact["path"])
        require(
            sha(executable) == diagnostic_artifact["sha256"],
            "inherited point report custody artifact changed before listing",
        )
        selector = row["selector"]
        require(remaining > 512 * 1024, "insufficient log budget before original test")
        prefix = row["group"] + "-" + target_name + "-" + str(index).zfill(2)
        group_label = "inherited-point-report3"
        print("SELECT " + group_label + " " + selector, flush=True)
        runtime_env = env.copy()
        selected_env = row["env"]
        if not isinstance(selected_env, dict):
            raise TypeError("original per-selector environment is not a mapping")
        for name, value in selected_env.items():
            if not isinstance(name, str) or not isinstance(value, str):
                raise TypeError("original per-selector environment is not text")
            runtime_env[name] = value
        selected_list = execute(
            [str(executable), selector, "--exact", "--list", "--format", "terse"],
            "select-" + prefix,
            executable,
            180,
            runtime_env,
            remaining,
        )
        processes.append(selected_list)
        remaining -= selected_list["retained_bytes"]
        require(
            lifecycle(selected_list) and selected_list["exit_code"] == 0,
            "original exact selector list failed or unclosed",
        )
        selected_lines = (
            (out / ("select-" + prefix + ".stdout")).read_text().splitlines()
        )
        require(
            [line for line in selected_lines if line.endswith(": test")]
            == [selector + ": test"],
            "original exact selector list mismatch",
        )
        require(
            all(
                line == selector + ": test"
                or not line
                or re.fullmatch(r"1 tests?, 0 benchmarks?", line)
                for line in selected_lines
            ),
            "unexpected original exact selector list data",
        )
        require(
            remaining > 512 * 1024,
            "insufficient log budget after original exact selector list",
        )
        child_before = sha(executable)
        require(
            child_before == diagnostic_artifact["sha256"],
            "selected Rust test artifact changed before transport spawn",
        )
        row["selected_test_artifact"] = {
            "path": str(executable),
            "expected_sha256": diagnostic_artifact["sha256"],
            "before_sha256": child_before,
            "after_sha256": None,
            "unchanged": False,
        }
        row["state"] = "RUNNING"
        save(out / "inherited-point-report-outcomes.json", inherited_report_rows)
        print("RUN " + group_label + " " + selector, flush=True)
        receipt = execute(
            [
                str(executable),
                selector,
                "--exact",
                "--nocapture",
                "--test-threads=1",
            ],
            prefix,
            executable,
            180,
            runtime_env,
            remaining,
        )
        processes.append(receipt)
        remaining -= receipt["retained_bytes"]
        child_after = sha(executable)
        row["selected_test_artifact"]["after_sha256"] = child_after
        row["selected_test_artifact"]["unchanged"] = (
            child_before == child_after == diagnostic_artifact["sha256"]
        )
        save(out / "inherited-point-report-outcomes.json", inherited_report_rows)
        require(
            row["selected_test_artifact"]["unchanged"],
            "selected Rust test artifact changed during transport",
        )
        transport_path = out / (prefix + ".transport.json")
        if lifecycle(receipt):
            require(
                transport_path.is_file(), "completed test transport summary missing"
            )
            transport = json.loads(transport_path.read_text())
            encoded_stderr = (out / (prefix + ".stderr")).read_bytes()
            repeat_stage = transport["repeat_stage"]
            table_stage = transport["table_stage"]
            require(
                transport["child_exit_code"] == receipt["exit_code"]
                and transport["ordinary_stderr_verbatim"] is False
                and transport["stderr_losslessly_reconstructible_after_table_filter"]
                is True
                and table_stage["ordinary_stderr_verbatim"] is True
                and table_stage["stdout_verbatim"] is True
                and transport["stdout_verbatim"] is True
                and transport["retained_stderr_bytes"]
                == repeat_stage["output_bytes"]
                == len(encoded_stderr)
                and repeat_stage["format"]
                == "two-family-lossless-line-dictionary-rle-v1"
                and repeat_stage["input_bytes"] == table_stage["retained_stderr_bytes"]
                and repeat_stage["output_sha256"]
                == hashlib.sha256(encoded_stderr).hexdigest(),
                "transport child status or layered byte census mismatch",
            )
            reconstructed = transport_filter.decode_diagnostic_repeats(
                encoded_stderr, lambda _data: None, repeat_stage["input_bytes"]
            )
            require(
                reconstructed["bytes"] == repeat_stage["input_bytes"]
                and reconstructed["sha256"] == repeat_stage["input_sha256"],
                "transport diagnostic reconstruction differs from original stream",
            )
            row["transport_reconstruction"] = reconstructed
        row.update(
            exit_code=receipt["exit_code"],
            receipt=prefix + ".process.json",
            stdout=prefix + ".stdout",
            stderr=prefix + ".stderr",
        )
        require(lifecycle(receipt), "original test lifecycle/capture stop: " + selector)
        stdout = (out / (prefix + ".stdout")).read_text(errors="replace")
        try:
            row["state"] = original_result(stdout, receipt["exit_code"])
        except RuntimeError:
            row["state"] = "SIGNAL" if receipt["exit_code"] < 0 else "PROTOCOL_FAILURE"
            save(out / "inherited-point-report-outcomes.json", inherited_report_rows)
            raise
        save(out / "inherited-point-report-outcomes.json", inherited_report_rows)
        require(
            sha(executable) == diagnostic_artifact["sha256"],
            "selected test artifact changed",
        )


def run_own_jim_alias_diagnostic() -> None:
    global remaining
    require(
        all(
            row["state"] in ["PASS", "FAIL"]
            for row in rows + logical_body_rows + codec_rows + named_child_rows + restored_child_rows + native_string_rows + native_formal_rows + analyser_layout_rows + own_jim_member_rows + advice_catalogue_rows + native_eligibility_rows
        ),
        "own Jim alias diagnostic controls cannot precede original135 closure",
    )
    require(
        files(dependency_roots) == dependencies_before
        and source() == before
        and files(provider_roots) == providers_before
        and files(list(tools.values())) == tool_pins,
        "pre-logical-body-entry source/provider/tool/dependency drift",
    )
    original_lib_roster = json.loads((out / "roster-tcl_compiler.json").read_text())
    require(
        {row["selector"] for row in own_jim_alias_rows}.issubset(
            original_lib_roster["selected"]
        ),
        "own Jim alias diagnostic controls absent from actual fresh compiler lib inventory",
    )
    for index, row in enumerate(own_jim_alias_rows):
        package, target_kind, target_name = row["package"], row["kind"], row["target"]
        selected_artifacts = [
            value
            for value in artifacts
            if value["package"] == package
            and value["kind"] == target_kind
            and value["target"] == target_name
        ]
        require(
            len(selected_artifacts) == 1,
            "own Jim alias diagnostic current built artifact absent/ambiguous",
        )
        diagnostic_artifact: dict[str, Any] = selected_artifacts[0]
        require(
            diagnostic_artifact["strict_Rust_0w"] is True
            and diagnostic_artifact["cargo_event"]["profile"]["debug_assertions"]
            is True,
            "own Jim alias diagnostic requires existing fresh strict debug-assertion test profile",
        )
        executable = Path(diagnostic_artifact["path"])
        require(
            sha(executable) == diagnostic_artifact["sha256"],
            "own Jim alias diagnostic artifact changed before listing",
        )
        selector = row["selector"]
        require(remaining > 512 * 1024, "insufficient log budget before original test")
        prefix = row["group"] + "-" + target_name + "-" + str(index).zfill(2)
        group_label = "own-jim-alias-firstguard1"
        print("SELECT " + group_label + " " + selector, flush=True)
        runtime_env = env.copy()
        selected_env = row["env"]
        if not isinstance(selected_env, dict):
            raise TypeError("original per-selector environment is not a mapping")
        for name, value in selected_env.items():
            if not isinstance(name, str) or not isinstance(value, str):
                raise TypeError("original per-selector environment is not text")
            runtime_env[name] = value
        selected_list = execute(
            [str(executable), selector, "--exact", "--list", "--format", "terse"],
            "select-" + prefix,
            executable,
            180,
            runtime_env,
            remaining,
        )
        processes.append(selected_list)
        remaining -= selected_list["retained_bytes"]
        require(
            lifecycle(selected_list) and selected_list["exit_code"] == 0,
            "original exact selector list failed or unclosed",
        )
        selected_lines = (
            (out / ("select-" + prefix + ".stdout")).read_text().splitlines()
        )
        require(
            [line for line in selected_lines if line.endswith(": test")]
            == [selector + ": test"],
            "original exact selector list mismatch",
        )
        require(
            all(
                line == selector + ": test"
                or not line
                or re.fullmatch(r"1 tests?, 0 benchmarks?", line)
                for line in selected_lines
            ),
            "unexpected original exact selector list data",
        )
        require(
            remaining > 512 * 1024,
            "insufficient log budget after original exact selector list",
        )
        child_before = sha(executable)
        require(
            child_before == diagnostic_artifact["sha256"],
            "selected Rust test artifact changed before transport spawn",
        )
        row["selected_test_artifact"] = {
            "path": str(executable),
            "expected_sha256": diagnostic_artifact["sha256"],
            "before_sha256": child_before,
            "after_sha256": None,
            "unchanged": False,
        }
        row["state"] = "RUNNING"
        save(out / "own-jim-alias-firstguard-outcomes.json", own_jim_alias_rows)
        print("RUN " + group_label + " " + selector, flush=True)
        receipt = execute(
            [
                str(executable),
                selector,
                "--exact",
                "--nocapture",
                "--test-threads=1",
            ],
            prefix,
            executable,
            180,
            runtime_env,
            remaining,
        )
        processes.append(receipt)
        remaining -= receipt["retained_bytes"]
        child_after = sha(executable)
        row["selected_test_artifact"]["after_sha256"] = child_after
        row["selected_test_artifact"]["unchanged"] = (
            child_before == child_after == diagnostic_artifact["sha256"]
        )
        save(out / "own-jim-alias-firstguard-outcomes.json", own_jim_alias_rows)
        require(
            row["selected_test_artifact"]["unchanged"],
            "selected Rust test artifact changed during transport",
        )
        transport_path = out / (prefix + ".transport.json")
        if lifecycle(receipt):
            require(
                transport_path.is_file(), "completed test transport summary missing"
            )
            transport = json.loads(transport_path.read_text())
            encoded_stderr = (out / (prefix + ".stderr")).read_bytes()
            repeat_stage = transport["repeat_stage"]
            table_stage = transport["table_stage"]
            require(
                transport["child_exit_code"] == receipt["exit_code"]
                and transport["ordinary_stderr_verbatim"] is False
                and transport["stderr_losslessly_reconstructible_after_table_filter"]
                is True
                and table_stage["ordinary_stderr_verbatim"] is True
                and table_stage["stdout_verbatim"] is True
                and transport["stdout_verbatim"] is True
                and transport["retained_stderr_bytes"]
                == repeat_stage["output_bytes"]
                == len(encoded_stderr)
                and repeat_stage["format"]
                == "two-family-lossless-line-dictionary-rle-v1"
                and repeat_stage["input_bytes"] == table_stage["retained_stderr_bytes"]
                and repeat_stage["output_sha256"]
                == hashlib.sha256(encoded_stderr).hexdigest(),
                "transport child status or layered byte census mismatch",
            )
            trace = {"seen": False, "quota": False, "tail": b"", "discard": False}
            alias_stages = {
                b"receipt", b"applied", b"capture.argument", b"capture.input",
                b"capture.subject", b"source_interpreter", b"source_current",
                b"alias_input", b"publication", b"namespace", b"target_interpreter",
                b"target_current", b"prefix", b"prefix_input", b"prefix_word", b"target_input",
            }
            def observe_alias_trace(data: bytes) -> None:
                pieces = data.split(b"\n")
                for index, piece in enumerate(pieces):
                    terminated = index < len(pieces) - 1
                    if trace["discard"]:
                        if terminated:
                            trace["discard"] = False
                        continue
                    if len(trace["tail"]) + len(piece) > 256:
                        trace["tail"] = b""
                        trace["discard"] = not terminated
                        continue
                    line = trace["tail"] + piece
                    trace["tail"] = b"" if terminated else line
                    if not terminated:
                        continue
                    match = re.fullmatch(rb"JIM_SOURCE_OWNER stage=jim\.alias\.([a-z_.]+) present=(?:true|false)", line)
                    if match and match[1] in alias_stages:
                        trace["seen"] = True
                    if line == b"JIM_SOURCE_OWNER stage=option_trace_quota state=UNOBSERVED limit=1024":
                        trace["quota"] = True
            reconstructed = transport_filter.decode_diagnostic_repeats(
                encoded_stderr, observe_alias_trace, repeat_stage["input_bytes"]
            )
            if trace["tail"]:
                observe_alias_trace(b"\n")
            row["alias_trace_observation"] = "OBSERVED" if trace["seen"] and not trace["quota"] else "UNOBSERVED"
            row["alias_trace_quota_reached"] = trace["quota"]
            require(
                reconstructed["bytes"] == repeat_stage["input_bytes"]
                and reconstructed["sha256"] == repeat_stage["input_sha256"],
                "transport diagnostic reconstruction differs from original stream",
            )
            row["transport_reconstruction"] = reconstructed
        row.update(
            exit_code=receipt["exit_code"],
            receipt=prefix + ".process.json",
            stdout=prefix + ".stdout",
            stderr=prefix + ".stderr",
        )
        require(lifecycle(receipt), "original test lifecycle/capture stop: " + selector)
        stdout = (out / (prefix + ".stdout")).read_text(errors="replace")
        try:
            row["state"] = original_result(stdout, receipt["exit_code"])
        except RuntimeError:
            row["state"] = "SIGNAL" if receipt["exit_code"] < 0 else "PROTOCOL_FAILURE"
            save(out / "own-jim-alias-firstguard-outcomes.json", own_jim_alias_rows)
            raise
        save(out / "own-jim-alias-firstguard-outcomes.json", own_jim_alias_rows)
        require(
            sha(executable) == diagnostic_artifact["sha256"],
            "selected test artifact changed",
        )


def run_native_eligibility_controls() -> None:
    global remaining
    require(
        all(
            row["state"] in ["PASS", "FAIL"]
            for row in rows + logical_body_rows + codec_rows + named_child_rows + restored_child_rows + native_string_rows + native_formal_rows + analyser_layout_rows + own_jim_member_rows + advice_catalogue_rows
        ),
        "Native eligibility controls cannot precede original133 closure",
    )
    require(
        files(dependency_roots) == dependencies_before
        and source() == before
        and files(provider_roots) == providers_before
        and files(list(tools.values())) == tool_pins,
        "pre-logical-body-entry source/provider/tool/dependency drift",
    )
    original_lib_roster = json.loads((out / "roster-inlining.json").read_text())
    require(
        {row["selector"] for row in native_eligibility_rows}.issubset(
            original_lib_roster["selected"]
        ),
        "Native eligibility controls absent from actual fresh inlining inventory",
    )
    for index, row in enumerate(native_eligibility_rows):
        package, target_kind, target_name = row["package"], row["kind"], row["target"]
        selected_artifacts = [
            value
            for value in artifacts
            if value["package"] == package
            and value["kind"] == target_kind
            and value["target"] == target_name
        ]
        require(
            len(selected_artifacts) == 1,
            "Native eligibility current built artifact absent/ambiguous",
        )
        diagnostic_artifact: dict[str, Any] = selected_artifacts[0]
        require(
            diagnostic_artifact["strict_Rust_0w"] is True
            and diagnostic_artifact["cargo_event"]["profile"]["debug_assertions"]
            is True,
            "Native eligibility requires existing fresh strict debug-assertion test profile",
        )
        executable = Path(diagnostic_artifact["path"])
        require(
            sha(executable) == diagnostic_artifact["sha256"],
            "Native eligibility artifact changed before listing",
        )
        selector = row["selector"]
        require(remaining > 512 * 1024, "insufficient log budget before original test")
        prefix = row["group"] + "-" + target_name + "-" + str(index).zfill(2)
        group_label = "native-eligibility2"
        print("SELECT " + group_label + " " + selector, flush=True)
        runtime_env = env.copy()
        selected_env = row["env"]
        if not isinstance(selected_env, dict):
            raise TypeError("original per-selector environment is not a mapping")
        for name, value in selected_env.items():
            if not isinstance(name, str) or not isinstance(value, str):
                raise TypeError("original per-selector environment is not text")
            runtime_env[name] = value
        selected_list = execute(
            [str(executable), selector, "--exact", "--list", "--format", "terse"],
            "select-" + prefix,
            executable,
            180,
            runtime_env,
            remaining,
        )
        processes.append(selected_list)
        remaining -= selected_list["retained_bytes"]
        require(
            lifecycle(selected_list) and selected_list["exit_code"] == 0,
            "original exact selector list failed or unclosed",
        )
        selected_lines = (
            (out / ("select-" + prefix + ".stdout")).read_text().splitlines()
        )
        require(
            [line for line in selected_lines if line.endswith(": test")]
            == [selector + ": test"],
            "original exact selector list mismatch",
        )
        require(
            all(
                line == selector + ": test"
                or not line
                or re.fullmatch(r"1 tests?, 0 benchmarks?", line)
                for line in selected_lines
            ),
            "unexpected original exact selector list data",
        )
        require(
            remaining > 512 * 1024,
            "insufficient log budget after original exact selector list",
        )
        child_before = sha(executable)
        require(
            child_before == diagnostic_artifact["sha256"],
            "selected Rust test artifact changed before transport spawn",
        )
        row["selected_test_artifact"] = {
            "path": str(executable),
            "expected_sha256": diagnostic_artifact["sha256"],
            "before_sha256": child_before,
            "after_sha256": None,
            "unchanged": False,
        }
        row["state"] = "RUNNING"
        save(out / "native-eligibility-outcomes.json", native_eligibility_rows)
        print("RUN " + group_label + " " + selector, flush=True)
        receipt = execute(
            [
                str(executable),
                selector,
                "--exact",
                "--nocapture",
                "--test-threads=1",
            ],
            prefix,
            executable,
            180,
            runtime_env,
            remaining,
        )
        processes.append(receipt)
        remaining -= receipt["retained_bytes"]
        child_after = sha(executable)
        row["selected_test_artifact"]["after_sha256"] = child_after
        row["selected_test_artifact"]["unchanged"] = (
            child_before == child_after == diagnostic_artifact["sha256"]
        )
        save(out / "native-eligibility-outcomes.json", native_eligibility_rows)
        require(
            row["selected_test_artifact"]["unchanged"],
            "selected Rust test artifact changed during transport",
        )
        transport_path = out / (prefix + ".transport.json")
        if lifecycle(receipt):
            require(
                transport_path.is_file(), "completed test transport summary missing"
            )
            transport = json.loads(transport_path.read_text())
            encoded_stderr = (out / (prefix + ".stderr")).read_bytes()
            repeat_stage = transport["repeat_stage"]
            table_stage = transport["table_stage"]
            require(
                transport["child_exit_code"] == receipt["exit_code"]
                and transport["ordinary_stderr_verbatim"] is False
                and transport["stderr_losslessly_reconstructible_after_table_filter"]
                is True
                and table_stage["ordinary_stderr_verbatim"] is True
                and table_stage["stdout_verbatim"] is True
                and transport["stdout_verbatim"] is True
                and transport["retained_stderr_bytes"]
                == repeat_stage["output_bytes"]
                == len(encoded_stderr)
                and repeat_stage["format"]
                == "two-family-lossless-line-dictionary-rle-v1"
                and repeat_stage["input_bytes"] == table_stage["retained_stderr_bytes"]
                and repeat_stage["output_sha256"]
                == hashlib.sha256(encoded_stderr).hexdigest(),
                "transport child status or layered byte census mismatch",
            )
            reconstructed = transport_filter.decode_diagnostic_repeats(
                encoded_stderr, lambda _data: None, repeat_stage["input_bytes"]
            )
            require(
                reconstructed["bytes"] == repeat_stage["input_bytes"]
                and reconstructed["sha256"] == repeat_stage["input_sha256"],
                "transport diagnostic reconstruction differs from original stream",
            )
            row["transport_reconstruction"] = reconstructed
        row.update(
            exit_code=receipt["exit_code"],
            receipt=prefix + ".process.json",
            stdout=prefix + ".stdout",
            stderr=prefix + ".stderr",
        )
        require(lifecycle(receipt), "original test lifecycle/capture stop: " + selector)
        stdout = (out / (prefix + ".stdout")).read_text(errors="replace")
        try:
            row["state"] = original_result(stdout, receipt["exit_code"])
        except RuntimeError:
            row["state"] = "SIGNAL" if receipt["exit_code"] < 0 else "PROTOCOL_FAILURE"
            save(out / "native-eligibility-outcomes.json", native_eligibility_rows)
            raise
        save(out / "native-eligibility-outcomes.json", native_eligibility_rows)
        require(
            sha(executable) == diagnostic_artifact["sha256"],
            "selected test artifact changed",
        )


def run_source_point_memo_controls() -> None:
    global remaining
    require("TCL_LSP_DIAGNOSTIC_SOURCE_POINT_MEMO_BYPASS" not in env, "memo8 requires diagnostic bypass absent; old environments are never mutated")
    require(
        all(
            row["state"] in ["PASS", "FAIL"]
            for row in rows + logical_body_rows + codec_rows + named_child_rows + restored_child_rows + native_string_rows + native_formal_rows + analyser_layout_rows + own_jim_member_rows + advice_catalogue_rows + native_eligibility_rows + own_jim_alias_rows
        ),
        "source-point memo controls cannot precede original136 closure",
    )
    require(
        files(dependency_roots) == dependencies_before
        and source() == before
        and files(provider_roots) == providers_before
        and files(list(tools.values())) == tool_pins,
        "pre-logical-body-entry source/provider/tool/dependency drift",
    )
    original_lib_roster = json.loads((out / "roster-tcl_compiler.json").read_text())
    require(
        {row["selector"] for row in source_point_memo_rows}.issubset(
            original_lib_roster["selected"]
        ),
        "source-point memo controls absent from actual fresh compiler lib inventory",
    )
    for index, row in enumerate(source_point_memo_rows):
        package, target_kind, target_name = row["package"], row["kind"], row["target"]
        selected_artifacts = [
            value
            for value in artifacts
            if value["package"] == package
            and value["kind"] == target_kind
            and value["target"] == target_name
        ]
        require(
            len(selected_artifacts) == 1,
            "source-point memo current built artifact absent/ambiguous",
        )
        diagnostic_artifact: dict[str, Any] = selected_artifacts[0]
        require(
            diagnostic_artifact["strict_Rust_0w"] is True
            and diagnostic_artifact["cargo_event"]["profile"]["debug_assertions"]
            is True,
            "source-point memo requires existing fresh strict debug-assertion test profile",
        )
        executable = Path(diagnostic_artifact["path"])
        require(
            sha(executable) == diagnostic_artifact["sha256"],
            "source-point memo artifact changed before listing",
        )
        selector = row["selector"]
        require(remaining > 512 * 1024, "insufficient log budget before original test")
        prefix = row["group"] + "-" + target_name + "-" + str(index).zfill(2)
        group_label = "source-point-memo8"
        print("SELECT " + group_label + " " + selector, flush=True)
        runtime_env = env.copy()
        selected_env = row["env"]
        if not isinstance(selected_env, dict):
            raise TypeError("original per-selector environment is not a mapping")
        for name, value in selected_env.items():
            if not isinstance(name, str) or not isinstance(value, str):
                raise TypeError("original per-selector environment is not text")
            runtime_env[name] = value
        selected_list = execute(
            [str(executable), selector, "--exact", "--list", "--format", "terse"],
            "select-" + prefix,
            executable,
            180,
            runtime_env,
            remaining,
        )
        processes.append(selected_list)
        remaining -= selected_list["retained_bytes"]
        require(
            lifecycle(selected_list) and selected_list["exit_code"] == 0,
            "original exact selector list failed or unclosed",
        )
        selected_lines = (
            (out / ("select-" + prefix + ".stdout")).read_text().splitlines()
        )
        require(
            [line for line in selected_lines if line.endswith(": test")]
            == [selector + ": test"],
            "original exact selector list mismatch",
        )
        require(
            all(
                line == selector + ": test"
                or not line
                or re.fullmatch(r"1 tests?, 0 benchmarks?", line)
                for line in selected_lines
            ),
            "unexpected original exact selector list data",
        )
        require(
            remaining > 512 * 1024,
            "insufficient log budget after original exact selector list",
        )
        child_before = sha(executable)
        require(
            child_before == diagnostic_artifact["sha256"],
            "selected Rust test artifact changed before transport spawn",
        )
        row["selected_test_artifact"] = {
            "path": str(executable),
            "expected_sha256": diagnostic_artifact["sha256"],
            "before_sha256": child_before,
            "after_sha256": None,
            "unchanged": False,
        }
        row["state"] = "RUNNING"
        save(out / "source-point-memo-outcomes.json", source_point_memo_rows)
        print("RUN " + group_label + " " + selector, flush=True)
        receipt = execute(
            [
                str(executable),
                selector,
                "--exact",
                "--nocapture",
                "--test-threads=1",
            ],
            prefix,
            executable,
            180,
            runtime_env,
            remaining,
        )
        processes.append(receipt)
        remaining -= receipt["retained_bytes"]
        child_after = sha(executable)
        row["selected_test_artifact"]["after_sha256"] = child_after
        row["selected_test_artifact"]["unchanged"] = (
            child_before == child_after == diagnostic_artifact["sha256"]
        )
        save(out / "source-point-memo-outcomes.json", source_point_memo_rows)
        require(
            row["selected_test_artifact"]["unchanged"],
            "selected Rust test artifact changed during transport",
        )
        transport_path = out / (prefix + ".transport.json")
        if lifecycle(receipt):
            require(
                transport_path.is_file(), "completed test transport summary missing"
            )
            transport = json.loads(transport_path.read_text())
            encoded_stderr = (out / (prefix + ".stderr")).read_bytes()
            repeat_stage = transport["repeat_stage"]
            table_stage = transport["table_stage"]
            require(
                transport["child_exit_code"] == receipt["exit_code"]
                and transport["ordinary_stderr_verbatim"] is False
                and transport["stderr_losslessly_reconstructible_after_table_filter"]
                is True
                and table_stage["ordinary_stderr_verbatim"] is True
                and table_stage["stdout_verbatim"] is True
                and transport["stdout_verbatim"] is True
                and transport["retained_stderr_bytes"]
                == repeat_stage["output_bytes"]
                == len(encoded_stderr)
                and repeat_stage["format"]
                == "two-family-lossless-line-dictionary-rle-v1"
                and repeat_stage["input_bytes"] == table_stage["retained_stderr_bytes"]
                and repeat_stage["output_sha256"]
                == hashlib.sha256(encoded_stderr).hexdigest(),
                "transport child status or layered byte census mismatch",
            )
            reconstructed = transport_filter.decode_diagnostic_repeats(
                encoded_stderr, lambda _data: None, repeat_stage["input_bytes"]
            )
            require(
                reconstructed["bytes"] == repeat_stage["input_bytes"]
                and reconstructed["sha256"] == repeat_stage["input_sha256"],
                "transport diagnostic reconstruction differs from original stream",
            )
            row["transport_reconstruction"] = reconstructed
        row.update(
            exit_code=receipt["exit_code"],
            receipt=prefix + ".process.json",
            stdout=prefix + ".stdout",
            stderr=prefix + ".stderr",
        )
        require(lifecycle(receipt), "original test lifecycle/capture stop: " + selector)
        stdout = (out / (prefix + ".stdout")).read_text(errors="replace")
        try:
            row["state"] = original_result(stdout, receipt["exit_code"])
        except RuntimeError:
            row["state"] = "SIGNAL" if receipt["exit_code"] < 0 else "PROTOCOL_FAILURE"
            save(out / "source-point-memo-outcomes.json", source_point_memo_rows)
            raise
        save(out / "source-point-memo-outcomes.json", source_point_memo_rows)
        require(
            sha(executable) == diagnostic_artifact["sha256"],
            "selected test artifact changed",
        )


def run_source_point_report_diagnostic() -> None:
    global remaining
    require("TCL_LSP_DIAGNOSTIC_SOURCE_POINT_MEMO_BYPASS" not in env, "report repeat requires diagnostic bypass absent; old environments are never mutated")
    require(
        all(
            row["state"] in ["PASS", "FAIL"]
            for row in rows + logical_body_rows + codec_rows + named_child_rows + restored_child_rows + native_string_rows + native_formal_rows + analyser_layout_rows + own_jim_member_rows + advice_catalogue_rows + native_eligibility_rows + own_jim_alias_rows + source_point_memo_rows + analyser_layout_diagnostic_rows
        ),
        "source-point report controls cannot precede original147 closure",
    )
    require(
        files(dependency_roots) == dependencies_before
        and source() == before
        and files(provider_roots) == providers_before
        and files(list(tools.values())) == tool_pins,
        "pre-logical-body-entry source/provider/tool/dependency drift",
    )
    original_lib_roster = json.loads((out / "roster-tcl_compiler.json").read_text())
    require(
        {row["selector"] for row in source_point_report_rows}.issubset(
            original_lib_roster["selected"]
        ),
        "source-point report controls absent from actual fresh compiler lib inventory",
    )
    for path, expected in source_point_report_source_pins.items():
        require(sha(root / path) == expected, "report repeat current source pin changed: " + path)
    for index, row in enumerate(source_point_report_rows):
        package, target_kind, target_name = row["package"], row["kind"], row["target"]
        selected_artifacts = [
            value
            for value in artifacts
            if value["package"] == package
            and value["kind"] == target_kind
            and value["target"] == target_name
        ]
        require(
            len(selected_artifacts) == 1,
            "source-point report current built artifact absent/ambiguous",
        )
        diagnostic_artifact: dict[str, Any] = selected_artifacts[0]
        require(
            diagnostic_artifact["strict_Rust_0w"] is True
            and diagnostic_artifact["cargo_event"]["profile"]["debug_assertions"]
            is True,
            "source-point report requires existing fresh strict debug-assertion test profile",
        )
        executable = Path(diagnostic_artifact["path"])
        require(
            sha(executable) == diagnostic_artifact["sha256"],
            "source-point report artifact changed before listing",
        )
        selector = row["selector"]
        require(remaining > 512 * 1024, "insufficient log budget before original test")
        prefix = row["group"] + "-" + target_name + "-" + str(index).zfill(2)
        group_label = "source-point-report-firstguard1"
        print("SELECT " + group_label + " " + selector, flush=True)
        runtime_env = env.copy()
        selected_env = row["env"]
        if not isinstance(selected_env, dict):
            raise TypeError("original per-selector environment is not a mapping")
        for name, value in selected_env.items():
            if not isinstance(name, str) or not isinstance(value, str):
                raise TypeError("original per-selector environment is not text")
            runtime_env[name] = value
        selected_list = execute(
            [str(executable), selector, "--exact", "--list", "--format", "terse"],
            "select-" + prefix,
            executable,
            180,
            runtime_env,
            remaining,
        )
        processes.append(selected_list)
        remaining -= selected_list["retained_bytes"]
        require(
            lifecycle(selected_list) and selected_list["exit_code"] == 0,
            "original exact selector list failed or unclosed",
        )
        selected_lines = (
            (out / ("select-" + prefix + ".stdout")).read_text().splitlines()
        )
        require(
            [line for line in selected_lines if line.endswith(": test")]
            == [selector + ": test"],
            "original exact selector list mismatch",
        )
        require(
            all(
                line == selector + ": test"
                or not line
                or re.fullmatch(r"1 tests?, 0 benchmarks?", line)
                for line in selected_lines
            ),
            "unexpected original exact selector list data",
        )
        require(
            remaining > 512 * 1024,
            "insufficient log budget after original exact selector list",
        )
        child_before = sha(executable)
        require(
            child_before == diagnostic_artifact["sha256"],
            "selected Rust test artifact changed before transport spawn",
        )
        row["selected_test_artifact"] = {
            "path": str(executable),
            "expected_sha256": diagnostic_artifact["sha256"],
            "before_sha256": child_before,
            "after_sha256": None,
            "unchanged": False,
        }
        row["state"] = "RUNNING"
        save(out / "source-point-report-firstguard-outcomes.json", source_point_report_rows)
        print("RUN " + group_label + " " + selector, flush=True)
        receipt = execute(
            [
                str(executable),
                selector,
                "--exact",
                "--nocapture",
                "--test-threads=1",
            ],
            prefix,
            executable,
            180,
            runtime_env,
            remaining,
        )
        processes.append(receipt)
        remaining -= receipt["retained_bytes"]
        child_after = sha(executable)
        row["selected_test_artifact"]["after_sha256"] = child_after
        row["selected_test_artifact"]["unchanged"] = (
            child_before == child_after == diagnostic_artifact["sha256"]
        )
        save(out / "source-point-report-firstguard-outcomes.json", source_point_report_rows)
        require(
            row["selected_test_artifact"]["unchanged"],
            "selected Rust test artifact changed during transport",
        )
        transport_path = out / (prefix + ".transport.json")
        if lifecycle(receipt):
            require(
                transport_path.is_file(), "completed test transport summary missing"
            )
            transport = json.loads(transport_path.read_text())
            encoded_stderr = (out / (prefix + ".stderr")).read_bytes()
            repeat_stage = transport["repeat_stage"]
            table_stage = transport["table_stage"]
            require(
                transport["child_exit_code"] == receipt["exit_code"]
                and transport["ordinary_stderr_verbatim"] is False
                and transport["stderr_losslessly_reconstructible_after_table_filter"]
                is True
                and table_stage["ordinary_stderr_verbatim"] is True
                and table_stage["stdout_verbatim"] is True
                and transport["stdout_verbatim"] is True
                and transport["retained_stderr_bytes"]
                == repeat_stage["output_bytes"]
                == len(encoded_stderr)
                and repeat_stage["format"]
                == "two-family-lossless-line-dictionary-rle-v1"
                and repeat_stage["input_bytes"] == table_stage["retained_stderr_bytes"]
                and repeat_stage["output_sha256"]
                == hashlib.sha256(encoded_stderr).hexdigest(),
                "transport child status or layered byte census mismatch",
            )
            trace_tail = bytearray()
            capture_rows = []
            def report_trace_sink(data):
                for piece in data.splitlines(keepends=True):
                    trace_tail.extend(piece)
                    require(len(trace_tail) <= 8192, "report trace line bound")
                    if trace_tail.endswith(b"\n"):
                        line = bytes(trace_tail).rstrip(b"\r\n")
                        if line.startswith(b"REPORT_CAPTURE stage="):
                            require(len(line) <= 512, "report trace width bound")
                            capture_rows.append(line.decode("ascii"))
                            require(len(capture_rows) <= 1024, "report trace row bound")
                        trace_tail.clear()
            reconstructed = transport_filter.decode_diagnostic_repeats(
                encoded_stderr, report_trace_sink, repeat_stage["input_bytes"]
            )
            require(not trace_tail, "report trace final line incomplete")
            row["capture_rows"] = capture_rows
            row["capture_observation"] = "OBSERVED" if capture_rows else "UNOBSERVED"
            row["first_logged_refusal"] = next((line for line in capture_rows
                if line.endswith("available=false") or line.endswith("refuses=true")), None)
            require(
                reconstructed["bytes"] == repeat_stage["input_bytes"]
                and reconstructed["sha256"] == repeat_stage["input_sha256"],
                "transport diagnostic reconstruction differs from original stream",
            )
            row["transport_reconstruction"] = reconstructed
        row.update(
            exit_code=receipt["exit_code"],
            receipt=prefix + ".process.json",
            stdout=prefix + ".stdout",
            stderr=prefix + ".stderr",
        )
        require(lifecycle(receipt), "original test lifecycle/capture stop: " + selector)
        stdout = (out / (prefix + ".stdout")).read_text(errors="replace")
        try:
            row["state"] = original_result(stdout, receipt["exit_code"])
        except RuntimeError:
            row["state"] = "SIGNAL" if receipt["exit_code"] < 0 else "PROTOCOL_FAILURE"
            save(out / "source-point-report-firstguard-outcomes.json", source_point_report_rows)
            raise
        save(out / "source-point-report-firstguard-outcomes.json", source_point_report_rows)
        require(
            sha(executable) == diagnostic_artifact["sha256"],
            "selected test artifact changed",
        )


def corpus_report_trace_observer(input_labels):
    # Reporting evidence only; Rust terminals/typed corpus markers stay authoritative.
    labels = set(input_labels)
    state = {"state": "UNOBSERVED", "rows": 0, "first_rows": [],
             "first_negative_by_stage": {}, "first_compare_refusal": None}
    context = None

    def observe(line):
        nonlocal context
        if line.startswith(b"SOURCE_POINT_CORPUS_BEGIN input="):
            require(len(line) <= 512, "corpus trace input label width bound")
            text = line[len(b"SOURCE_POINT_CORPUS_BEGIN input="):].decode("ascii")
            label, end = json.JSONDecoder().raw_decode(text)
            require(label in labels, "corpus trace input label outside exact roster")
            tail = text[end:]
            require(re.fullmatch(r" dialect=[a-zA-Z0-9_.-]+ original=(true|false)", tail)
                    is not None, "corpus trace input context shape")
            context = {"input": label, "source_context": tail.strip()}
        if not line.startswith(b"REPORT_CAPTURE stage="):
            return
        require(len(line) <= 512, "corpus report trace row width bound")
        text = line.decode("ascii")
        stage = text[len("REPORT_CAPTURE stage="):].split(" ", 1)[0]
        require(len(stage) <= 128 and re.fullmatch(r"[a-zA-Z0-9_.-]+", stage)
                is not None, "corpus report trace stage bound")
        state["state"] = "OBSERVED"
        state["rows"] += 1
        entry = {"context": context, "row": text}
        if len(state["first_rows"]) < 16:
            state["first_rows"].append(entry)
        negative = (re.search(r"(?:available|current-owner)=false(?:$| )|refuses=true(?:$| )|result=Err\(", text)
                    is not None)
        if negative and stage not in state["first_negative_by_stage"]:
            require(len(state["first_negative_by_stage"]) < 256,
                    "corpus report negative stage bound")
            state["first_negative_by_stage"][stage] = entry
        if negative and stage.startswith("compare.") and state["first_compare_refusal"] is None:
            state["first_compare_refusal"] = entry

    def consume(line):
        if state.get("error") is not None:
            return
        try:
            observe(line)
        except (RuntimeError, ValueError, TypeError, UnicodeError):
            # Keep the actual Rust PASS/FAIL row; reject trace admission later.
            state["state"] = "UNOBSERVED"
            state["error"] = "bounded trace protocol failure"

    return state, consume


def run_source_point_corpus_controls() -> None:
    global remaining
    require("TCL_LSP_DIAGNOSTIC_SOURCE_POINT_MEMO_BYPASS" not in env, "corpus2 requires diagnostic bypass absent; old environments are never mutated")
    if not args.source_point_corpus_only:
        require(
            all(
                row["state"] in ["PASS", "FAIL"]
                for row in rows + logical_body_rows + codec_rows + named_child_rows + restored_child_rows + native_string_rows + native_formal_rows + analyser_layout_rows + own_jim_member_rows + advice_catalogue_rows + native_eligibility_rows + own_jim_alias_rows + source_point_memo_rows + analyser_layout_diagnostic_rows
            ),
            "source-point corpus controls cannot precede original147 closure",
        )
    require(
        files(dependency_roots) == dependencies_before
        and source() == before
        and files(provider_roots) == providers_before
        and files(list(tools.values())) == tool_pins,
        "pre-logical-body-entry source/provider/tool/dependency drift",
    )
    original_lib_roster = json.loads((out / "roster-tcl_compiler.json").read_text())
    require(
        {row["selector"] for row in source_point_corpus_rows}.issubset(
            original_lib_roster["selected"]
        ),
        "source-point corpus controls absent from actual fresh compiler lib inventory",
    )
    for path, expected in source_point_corpus_validation["source_pins"].items():
        require(sha(root / path) == expected, "corpus input/source pin drift: " + path)
    for index, row in enumerate(source_point_corpus_rows):
        require(deadline - time.time() >= 180 + row["budget_seconds"] + 40,
                "insufficient shared time for full separate corpus list/test/cleanup budgets")
        package, target_kind, target_name = row["package"], row["kind"], row["target"]
        selected_artifacts = [
            value
            for value in artifacts
            if value["package"] == package
            and value["kind"] == target_kind
            and value["target"] == target_name
        ]
        require(
            len(selected_artifacts) == 1,
            "source-point corpus current built artifact absent/ambiguous",
        )
        diagnostic_artifact: dict[str, Any] = selected_artifacts[0]
        require(
            diagnostic_artifact["strict_Rust_0w"] is True
            and diagnostic_artifact["cargo_event"]["profile"]["debug_assertions"]
            is True,
            "source-point corpus requires existing fresh strict debug-assertion test profile",
        )
        executable = Path(diagnostic_artifact["path"])
        require(
            sha(executable) == diagnostic_artifact["sha256"],
            "source-point corpus artifact changed before listing",
        )
        selector = row["selector"]
        require(remaining > 512 * 1024, "insufficient log budget before original test")
        prefix = row["group"] + "-" + target_name + "-" + str(index).zfill(2)
        group_label = "source-point-corpus2"
        print("SELECT " + group_label + " " + selector, flush=True)
        runtime_env = env.copy()
        selected_env = row["env"]
        if not isinstance(selected_env, dict):
            raise TypeError("original per-selector environment is not a mapping")
        for name, value in selected_env.items():
            if not isinstance(name, str) or not isinstance(value, str):
                raise TypeError("original per-selector environment is not text")
            runtime_env[name] = value
        selected_list = execute(
            [str(executable), selector, "--exact", "--list", "--format", "terse"],
            "select-" + prefix,
            executable,
            180,
            runtime_env,
            remaining,
        )
        processes.append(selected_list)
        remaining -= selected_list["retained_bytes"]
        require(
            lifecycle(selected_list) and selected_list["exit_code"] == 0,
            "original exact selector list failed or unclosed",
        )
        selected_lines = (
            (out / ("select-" + prefix + ".stdout")).read_text().splitlines()
        )
        require(
            [line for line in selected_lines if line.endswith(": test")]
            == [selector + ": test"],
            "original exact selector list mismatch",
        )
        require(
            all(
                line == selector + ": test"
                or not line
                or re.fullmatch(r"1 tests?, 0 benchmarks?", line)
                for line in selected_lines
            ),
            "unexpected original exact selector list data",
        )
        require(
            remaining > 512 * 1024,
            "insufficient log budget after original exact selector list",
        )
        child_before = sha(executable)
        require(
            child_before == diagnostic_artifact["sha256"],
            "selected Rust test artifact changed before transport spawn",
        )
        row["selected_test_artifact"] = {
            "path": str(executable),
            "expected_sha256": diagnostic_artifact["sha256"],
            "before_sha256": child_before,
            "after_sha256": None,
            "unchanged": False,
        }
        corpus_test_env = runtime_env
        if args.source_point_corpus_only:
            corpus_test_env = runtime_env.copy()
            corpus_test_env["TCL_LSP_TRACE_AUTHENTICATED_REPORT"] = "1"
            row["report_trace_child_env"] = {"TCL_LSP_TRACE_AUTHENTICATED_REPORT": "1"}
        row["state"] = "RUNNING"
        save(out / "source-point-corpus-outcomes.json", source_point_corpus_rows)
        print("RUN " + group_label + " " + selector, flush=True)
        receipt = execute(
            [
                str(executable),
                selector,
                "--exact",
                "--nocapture",
                "--test-threads=1",
            ],
            prefix,
            executable,
            row["budget_seconds"],
            corpus_test_env,
            remaining,
        )
        processes.append(receipt)
        remaining -= receipt["retained_bytes"]
        child_after = sha(executable)
        row["selected_test_artifact"]["after_sha256"] = child_after
        row["selected_test_artifact"]["unchanged"] = (
            child_before == child_after == diagnostic_artifact["sha256"]
        )
        save(out / "source-point-corpus-outcomes.json", source_point_corpus_rows)
        require(
            row["selected_test_artifact"]["unchanged"],
            "selected Rust test artifact changed during transport",
        )
        transport_path = out / (prefix + ".transport.json")
        if lifecycle(receipt):
            require(
                transport_path.is_file(), "completed test transport summary missing"
            )
            transport = json.loads(transport_path.read_text())
            encoded_stderr = (out / (prefix + ".stderr")).read_bytes()
            repeat_stage = transport["repeat_stage"]
            table_stage = transport["table_stage"]
            require(
                transport["child_exit_code"] == receipt["exit_code"]
                and transport["ordinary_stderr_verbatim"] is False
                and transport["stderr_losslessly_reconstructible_after_table_filter"]
                is True
                and table_stage["ordinary_stderr_verbatim"] is True
                and table_stage["stdout_verbatim"] is True
                and transport["stdout_verbatim"] is True
                and transport["retained_stderr_bytes"]
                == repeat_stage["output_bytes"]
                == len(encoded_stderr)
                and repeat_stage["format"]
                == "two-family-lossless-line-dictionary-rle-v1"
                and repeat_stage["input_bytes"] == table_stage["retained_stderr_bytes"]
                and repeat_stage["output_sha256"]
                == hashlib.sha256(encoded_stderr).hexdigest(),
                "transport child status or layered byte census mismatch",
            )
            report_trace, report_trace_sink = (corpus_report_trace_observer(row["input_labels"])
                                                if args.source_point_corpus_only else (None, None))
            marker_tail = bytearray()
            markers = []
            def corpus_marker_sink(data):
                for piece in data.splitlines(keepends=True):
                    marker_tail.extend(piece)
                    require(len(marker_tail) <= 8192, "corpus marker parser line bound")
                    if marker_tail.endswith(b"\n"):
                        line = bytes(marker_tail).rstrip(b"\r\n")
                        if report_trace_sink is not None:
                            report_trace_sink(line)
                        if line.startswith(b"SOURCE_POINT_CORPUS input="):
                            require(len(line) <= 512, "corpus marker width bound")
                            markers.append(line.decode("ascii"))
                            require(len(markers) <= 234, "corpus marker row bound")
                        marker_tail.clear()
            reconstructed = transport_filter.decode_diagnostic_repeats(
                encoded_stderr, corpus_marker_sink, repeat_stage["input_bytes"]
            )
            require(not marker_tail, "corpus final stderr line incomplete")
            row["complete_equal_markers"] = markers
            require(
                reconstructed["bytes"] == repeat_stage["input_bytes"]
                and reconstructed["sha256"] == repeat_stage["input_sha256"],
                "transport diagnostic reconstruction differs from original stream",
            )
            row["transport_reconstruction"] = reconstructed
            if report_trace is not None:
                row["report_trace_observation"] = report_trace
        row.update(
            exit_code=receipt["exit_code"],
            receipt=prefix + ".process.json",
            stdout=prefix + ".stdout",
            stderr=prefix + ".stderr",
        )
        require(lifecycle(receipt), "original test lifecycle/capture stop: " + selector)
        stdout = (out / (prefix + ".stdout")).read_text(errors="replace")
        try:
            row["state"] = original_result(stdout, receipt["exit_code"])
        except RuntimeError:
            row["state"] = "SIGNAL" if receipt["exit_code"] < 0 else "PROTOCOL_FAILURE"
            save(out / "source-point-corpus-outcomes.json", source_point_corpus_rows)
            raise
        if row["state"] == "PASS":
            expected_markers = {
                "SOURCE_POINT_CORPUS input=" + json.dumps(label) + " mixed=" + mixed + " pairs=3 equal=3"
                for label in row["input_labels"] for mixed in ["false", "true"]
            }
            actual_markers = row.get("complete_equal_markers", [])
            require(len(actual_markers) == len(expected_markers) and set(actual_markers) == expected_markers,
                    "corpus completed input/mode/typed-Equal marker census mismatch")
            row["authenticated_complete_inputs"] = len(row["input_labels"])
            row["authenticated_threeway_pairs"] = len(actual_markers) * 3
        save(out / "source-point-corpus-outcomes.json", source_point_corpus_rows)
        require(
            sha(executable) == diagnostic_artifact["sha256"],
            "selected test artifact changed",
        )


def run_analyser_layout_diagnostics() -> None:
    global remaining
    require(
        all(
            row["state"] in ["PASS", "FAIL"]
            for row in rows + logical_body_rows + codec_rows + named_child_rows + restored_child_rows + native_string_rows + native_formal_rows + analyser_layout_rows + own_jim_member_rows + advice_catalogue_rows + native_eligibility_rows + own_jim_alias_rows + source_point_memo_rows
        ),
        "analyser layout diagnostic controls cannot precede original144 closure",
    )
    require(
        files(dependency_roots) == dependencies_before
        and source() == before
        and files(provider_roots) == providers_before
        and files(list(tools.values())) == tool_pins,
        "pre-logical-body-entry source/provider/tool/dependency drift",
    )
    original_lib_roster = json.loads((out / "roster-tcl_compiler.json").read_text())
    require(
        {row["selector"] for row in analyser_layout_diagnostic_rows}.issubset(
            original_lib_roster["selected"]
        ),
        "analyser layout diagnostic controls absent from actual fresh compiler lib inventory",
    )
    for index, row in enumerate(analyser_layout_diagnostic_rows):
        package, target_kind, target_name = row["package"], row["kind"], row["target"]
        selected_artifacts = [
            value
            for value in artifacts
            if value["package"] == package
            and value["kind"] == target_kind
            and value["target"] == target_name
        ]
        require(
            len(selected_artifacts) == 1,
            "analyser layout diagnostic current built artifact absent/ambiguous",
        )
        diagnostic_artifact: dict[str, Any] = selected_artifacts[0]
        require(
            diagnostic_artifact["strict_Rust_0w"] is True
            and diagnostic_artifact["cargo_event"]["profile"]["debug_assertions"]
            is True,
            "analyser layout diagnostic requires existing fresh strict debug-assertion test profile",
        )
        executable = Path(diagnostic_artifact["path"])
        require(
            sha(executable) == diagnostic_artifact["sha256"],
            "analyser layout diagnostic artifact changed before listing",
        )
        selector = row["selector"]
        require(remaining > 512 * 1024, "insufficient log budget before original test")
        prefix = row["group"] + "-" + target_name + "-" + str(index).zfill(2)
        group_label = "analyser-layout-firstguard3"
        print("SELECT " + group_label + " " + selector, flush=True)
        runtime_env = env.copy()
        selected_env = row["env"]
        if not isinstance(selected_env, dict):
            raise TypeError("original per-selector environment is not a mapping")
        for name, value in selected_env.items():
            if not isinstance(name, str) or not isinstance(value, str):
                raise TypeError("original per-selector environment is not text")
            runtime_env[name] = value
        selected_list = execute(
            [str(executable), selector, "--exact", "--list", "--format", "terse"],
            "select-" + prefix,
            executable,
            180,
            runtime_env,
            remaining,
        )
        processes.append(selected_list)
        remaining -= selected_list["retained_bytes"]
        require(
            lifecycle(selected_list) and selected_list["exit_code"] == 0,
            "original exact selector list failed or unclosed",
        )
        selected_lines = (
            (out / ("select-" + prefix + ".stdout")).read_text().splitlines()
        )
        require(
            [line for line in selected_lines if line.endswith(": test")]
            == [selector + ": test"],
            "original exact selector list mismatch",
        )
        require(
            all(
                line == selector + ": test"
                or not line
                or re.fullmatch(r"1 tests?, 0 benchmarks?", line)
                for line in selected_lines
            ),
            "unexpected original exact selector list data",
        )
        require(
            remaining > 512 * 1024,
            "insufficient log budget after original exact selector list",
        )
        child_before = sha(executable)
        require(
            child_before == diagnostic_artifact["sha256"],
            "selected Rust test artifact changed before transport spawn",
        )
        row["selected_test_artifact"] = {
            "path": str(executable),
            "expected_sha256": diagnostic_artifact["sha256"],
            "before_sha256": child_before,
            "after_sha256": None,
            "unchanged": False,
        }
        row["state"] = "RUNNING"
        save(out / "analyser-layout-firstguard-outcomes.json", analyser_layout_diagnostic_rows)
        print("RUN " + group_label + " " + selector, flush=True)
        receipt = execute(
            [
                str(executable),
                selector,
                "--exact",
                "--nocapture",
                "--test-threads=1",
            ],
            prefix,
            executable,
            180,
            runtime_env,
            remaining,
        )
        processes.append(receipt)
        remaining -= receipt["retained_bytes"]
        child_after = sha(executable)
        row["selected_test_artifact"]["after_sha256"] = child_after
        row["selected_test_artifact"]["unchanged"] = (
            child_before == child_after == diagnostic_artifact["sha256"]
        )
        save(out / "analyser-layout-firstguard-outcomes.json", analyser_layout_diagnostic_rows)
        require(
            row["selected_test_artifact"]["unchanged"],
            "selected Rust test artifact changed during transport",
        )
        transport_path = out / (prefix + ".transport.json")
        if lifecycle(receipt):
            require(
                transport_path.is_file(), "completed test transport summary missing"
            )
            transport = json.loads(transport_path.read_text())
            encoded_stderr = (out / (prefix + ".stderr")).read_bytes()
            repeat_stage = transport["repeat_stage"]
            table_stage = transport["table_stage"]
            require(
                transport["child_exit_code"] == receipt["exit_code"]
                and transport["ordinary_stderr_verbatim"] is False
                and transport["stderr_losslessly_reconstructible_after_table_filter"]
                is True
                and table_stage["ordinary_stderr_verbatim"] is True
                and table_stage["stdout_verbatim"] is True
                and transport["stdout_verbatim"] is True
                and transport["retained_stderr_bytes"]
                == repeat_stage["output_bytes"]
                == len(encoded_stderr)
                and repeat_stage["format"]
                == "two-family-lossless-line-dictionary-rle-v1"
                and repeat_stage["input_bytes"] == table_stage["retained_stderr_bytes"]
                and repeat_stage["output_sha256"]
                == hashlib.sha256(encoded_stderr).hexdigest(),
                "transport child status or layered byte census mismatch",
            )
            trace = {"seen": False, "quota": False, "tail": b"", "discard": False}
            alias_stages = {b"words", b"argument_count", b"origin", b"word", b"expansion", b"text", b"projection", b"argv", b"value", b"tokens", b"schema", b"hook", b"shape", b"roles", b"statics", b"positions", b"complete"}
            def observe_alias_trace(data: bytes) -> None:
                pieces = data.split(b"\n")
                for index, piece in enumerate(pieces):
                    terminated = index < len(pieces) - 1
                    if trace["discard"]:
                        if terminated:
                            trace["discard"] = False
                        continue
                    if len(trace["tail"]) + len(piece) > 256:
                        trace["tail"] = b""
                        trace["discard"] = not terminated
                        continue
                    line = trace["tail"] + piece
                    trace["tail"] = b"" if terminated else line
                    if not terminated:
                        continue
                    match = re.fullmatch(rb"JIM_SOURCE_OWNER stage=analyser\.proc\.([a-z_.]+) present=(?:true|false)", line)
                    if match and match[1] in alias_stages:
                        trace["seen"] = True
                    if line == b"JIM_SOURCE_OWNER stage=option_trace_quota state=UNOBSERVED limit=1024":
                        trace["quota"] = True
            reconstructed = transport_filter.decode_diagnostic_repeats(
                encoded_stderr, observe_alias_trace, repeat_stage["input_bytes"]
            )
            if trace["tail"]:
                observe_alias_trace(b"\n")
            row["layout_trace_observation"] = "OBSERVED" if trace["seen"] and not trace["quota"] else "UNOBSERVED"
            row["layout_trace_quota_reached"] = trace["quota"]
            require(
                reconstructed["bytes"] == repeat_stage["input_bytes"]
                and reconstructed["sha256"] == repeat_stage["input_sha256"],
                "transport diagnostic reconstruction differs from original stream",
            )
            row["transport_reconstruction"] = reconstructed
        row.update(
            exit_code=receipt["exit_code"],
            receipt=prefix + ".process.json",
            stdout=prefix + ".stdout",
            stderr=prefix + ".stderr",
        )
        require(lifecycle(receipt), "original test lifecycle/capture stop: " + selector)
        stdout = (out / (prefix + ".stdout")).read_text(errors="replace")
        try:
            row["state"] = original_result(stdout, receipt["exit_code"])
        except RuntimeError:
            row["state"] = "SIGNAL" if receipt["exit_code"] < 0 else "PROTOCOL_FAILURE"
            save(out / "analyser-layout-firstguard-outcomes.json", analyser_layout_diagnostic_rows)
            raise
        save(out / "analyser-layout-firstguard-outcomes.json", analyser_layout_diagnostic_rows)
        require(
            sha(executable) == diagnostic_artifact["sha256"],
            "selected test artifact changed",
        )


def run_jim_source_layout_controls() -> None:
    global remaining
    require(
        files(dependency_roots) == dependencies_before
        and source() == before
        and files(provider_roots) == providers_before
        and files(list(tools.values())) == tool_pins,
        "pre-logical-body-entry source/provider/tool/dependency drift",
    )
    require(len(jim_layout_rows) == 11, "exact Layout11 count required")
    for pinned_path, pinned_digest in jim_layout_validation["source_pins"].items():
        require(sha(root / pinned_path) == pinned_digest,
                "Layout11 direct source pin drift: " + pinned_path)
    for spec in jim_layout_validation["build_targets"]:
        inventory = json.loads((out / ("roster-" + spec["name"] + ".json")).read_text())
        selectors = [row["selector"] for row in jim_layout_rows
                     if row["package"] == spec["package"] and row["target"] == spec["name"]]
        require(len(selectors) == spec["expected_selected"]
                and set(selectors).issubset(inventory["selected"]),
                "Layout11 controls absent from matching fresh target inventory")
    require(deadline - time.time() >= 11 * (180 + 180 + 40),
            "insufficient shared time for ALL Layout11 list/test/cleanup budgets")
    for index, row in enumerate(jim_layout_rows):
        require(deadline - time.time() >= 180 + 180 + 40,
                "insufficient shared time for full Layout11 list/test/cleanup budgets")
        package, target_kind, target_name = row["package"], row["kind"], row["target"]
        selected_artifacts = [
            value
            for value in artifacts
            if value["package"] == package
            and value["kind"] == target_kind
            and value["target"] == target_name
        ]
        require(
            len(selected_artifacts) == 1,
            "Jim source layout11 current built artifact absent/ambiguous",
        )
        diagnostic_artifact: dict[str, Any] = selected_artifacts[0]
        require(
            diagnostic_artifact["strict_Rust_0w"] is True
            and diagnostic_artifact["cargo_event"]["profile"]["debug_assertions"]
            is True,
            "Jim source layout11 requires existing fresh strict debug-assertion test profile",
        )
        executable = Path(diagnostic_artifact["path"])
        require(
            sha(executable) == diagnostic_artifact["sha256"],
            "Jim source layout11 artifact changed before listing",
        )
        selector = row["selector"]
        require(remaining > 512 * 1024, "insufficient log budget before original test")
        prefix = row["group"] + "-" + target_name + "-" + str(index).zfill(2)
        group_label = "jim-source-layout11"
        print("SELECT " + group_label + " " + selector, flush=True)
        runtime_env = env.copy()
        selected_env = row["env"]
        if not isinstance(selected_env, dict):
            raise TypeError("original per-selector environment is not a mapping")
        for name, value in selected_env.items():
            if not isinstance(name, str) or not isinstance(value, str):
                raise TypeError("original per-selector environment is not text")
            runtime_env[name] = value
        selected_list = execute(
            [str(executable), selector, "--exact", "--list", "--format", "terse"],
            "select-" + prefix,
            executable,
            180,
            runtime_env,
            remaining,
        )
        processes.append(selected_list)
        remaining -= selected_list["retained_bytes"]
        require(
            lifecycle(selected_list) and selected_list["exit_code"] == 0,
            "original exact selector list failed or unclosed",
        )
        selected_lines = (
            (out / ("select-" + prefix + ".stdout")).read_text().splitlines()
        )
        require(
            [line for line in selected_lines if line.endswith(": test")]
            == [selector + ": test"],
            "original exact selector list mismatch",
        )
        require(
            all(
                line == selector + ": test"
                or not line
                or re.fullmatch(r"1 tests?, 0 benchmarks?", line)
                for line in selected_lines
            ),
            "unexpected original exact selector list data",
        )
        require(
            remaining > 512 * 1024,
            "insufficient log budget after original exact selector list",
        )
        child_before = sha(executable)
        require(
            child_before == diagnostic_artifact["sha256"],
            "selected Rust test artifact changed before transport spawn",
        )
        row["selected_test_artifact"] = {
            "path": str(executable),
            "expected_sha256": diagnostic_artifact["sha256"],
            "before_sha256": child_before,
            "after_sha256": None,
            "unchanged": False,
        }
        row["state"] = "RUNNING"
        save(out / "jim-source-layout-outcomes.json", jim_layout_rows)
        print("RUN " + group_label + " " + selector, flush=True)
        receipt = execute(
            [
                str(executable),
                selector,
                "--exact",
                "--nocapture",
                "--test-threads=1",
            ],
            prefix,
            executable,
            180,
            runtime_env,
            remaining,
        )
        processes.append(receipt)
        remaining -= receipt["retained_bytes"]
        child_after = sha(executable)
        row["selected_test_artifact"]["after_sha256"] = child_after
        row["selected_test_artifact"]["unchanged"] = (
            child_before == child_after == diagnostic_artifact["sha256"]
        )
        save(out / "jim-source-layout-outcomes.json", jim_layout_rows)
        require(
            row["selected_test_artifact"]["unchanged"],
            "selected Rust test artifact changed during transport",
        )
        transport_path = out / (prefix + ".transport.json")
        if lifecycle(receipt):
            require(
                transport_path.is_file(), "completed test transport summary missing"
            )
            transport = json.loads(transport_path.read_text())
            encoded_stderr = (out / (prefix + ".stderr")).read_bytes()
            repeat_stage = transport["repeat_stage"]
            table_stage = transport["table_stage"]
            require(
                transport["child_exit_code"] == receipt["exit_code"]
                and transport["ordinary_stderr_verbatim"] is False
                and transport["stderr_losslessly_reconstructible_after_table_filter"]
                is True
                and table_stage["ordinary_stderr_verbatim"] is True
                and table_stage["stdout_verbatim"] is True
                and transport["stdout_verbatim"] is True
                and transport["retained_stderr_bytes"]
                == repeat_stage["output_bytes"]
                == len(encoded_stderr)
                and repeat_stage["format"]
                == "two-family-lossless-line-dictionary-rle-v1"
                and repeat_stage["input_bytes"] == table_stage["retained_stderr_bytes"]
                and repeat_stage["output_sha256"]
                == hashlib.sha256(encoded_stderr).hexdigest(),
                "transport child status or layered byte census mismatch",
            )
            reconstructed = transport_filter.decode_diagnostic_repeats(
                encoded_stderr, lambda _data: None, repeat_stage["input_bytes"]
            )
            require(
                reconstructed["bytes"] == repeat_stage["input_bytes"]
                and reconstructed["sha256"] == repeat_stage["input_sha256"],
                "transport diagnostic reconstruction differs from original stream",
            )
            row["transport_reconstruction"] = reconstructed
        row.update(
            exit_code=receipt["exit_code"],
            receipt=prefix + ".process.json",
            stdout=prefix + ".stdout",
            stderr=prefix + ".stderr",
        )
        require(lifecycle(receipt), "original test lifecycle/capture stop: " + selector)
        stdout = (out / (prefix + ".stdout")).read_text(errors="replace")
        try:
            row["state"] = original_result(stdout, receipt["exit_code"])
        except RuntimeError:
            row["state"] = "SIGNAL" if receipt["exit_code"] < 0 else "PROTOCOL_FAILURE"
            save(out / "jim-source-layout-outcomes.json", jim_layout_rows)
            raise
        save(out / "jim-source-layout-outcomes.json", jim_layout_rows)
        require(
            sha(executable) == diagnostic_artifact["sha256"],
            "selected test artifact changed",
        )



def observe_layout_jim_trace(encoded, repeat_stage, row):
    trace = {"tail": bytearray(), "line": 0, "discard": False,
             "quota": False, "unclassified_oversized_line": False, "rows": []}
    known = set(row["trace_stages"])
    def line_observation(line):
        trace["line"] += 1
        text = line.decode("utf-8", errors="replace").rstrip("\r\n")
        marker = text.find("JIM_SOURCE_OWNER stage=")
        if marker < 0:
            return
        text = text[marker:]
        if text == "JIM_SOURCE_OWNER stage=option_trace_quota state=UNOBSERVED limit=1024":
            trace["quota"] = True
        match = re.fullmatch(r"JIM_SOURCE_OWNER stage=([A-Za-z0-9_.]+) present=(true|false)", text)
        if match and match[1] in known:
            trace["rows"].append({"raw_line": trace["line"], "stage": match[1],
                                  "present": match[2] == "true"})
            require(len(trace["rows"]) <= 1024, "Jim diagnostic trace exceeds unchanged quota")
    def sink(data):
        for piece in data.splitlines(keepends=True):
            if not trace["discard"]:
                trace["tail"].extend(piece)
                if len(trace["tail"]) > 8192:
                    trace["unclassified_oversized_line"] = True
                    trace["tail"].clear()
                    trace["discard"] = True
            if piece.endswith(b"\n"):
                if trace["discard"]:
                    trace["line"] += 1
                else:
                    line_observation(bytes(trace["tail"]))
                trace["tail"].clear()
                trace["discard"] = False
    reconstructed = transport_filter.decode_diagnostic_repeats(
        encoded, sink, repeat_stage["input_bytes"]
    )
    if trace["tail"]:
        line_observation(bytes(trace["tail"]))
    observation = "OBSERVED" if (trace["rows"] and not trace["quota"]
        and not trace["unclassified_oversized_line"]) else "UNOBSERVED"
    first_false = next((value for value in trace["rows"] if not value["present"]), None)
    row.update(jim_trace_rows=trace["rows"], jim_trace_observation=observation,
               jim_trace_quota_reached=trace["quota"],
               jim_trace_unclassified_oversized_line=trace["unclassified_oversized_line"],
               first_observed_false=first_false,
               qualified_first_refusal=first_false if observation == "OBSERVED" else None,
               first_refusal_observation="OBSERVED" if observation == "OBSERVED" and first_false else "UNOBSERVED",
               trace_qualification="Observed stage only; no fixture/owner identity supplied. Join original failure phase before causal attribution.")
    return reconstructed


def run_layout_own_jim_diagnostic(row) -> None:
    global remaining
    if deadline - time.time() < 180 + 180 + 40 or remaining <= 512 * 1024:
        row["admission"] = "UNRUN_INSUFFICIENT_FULL_STAGE_BUDGET"
        return
    row["admission"] = "ADMITTED_FULL_LIST_TEST_CLEANUP"
    phase_rows = [row]
    for path, digest in jim_layout_validation["source_pins"].items():
        require(sha(root / path) == digest, "Jim diagnostic direct source drift: " + path)
    require(
        files(dependency_roots) == dependencies_before
        and source() == before
        and files(provider_roots) == providers_before
        and files(list(tools.values())) == tool_pins,
        "pre-logical-body-entry source/provider/tool/dependency drift",
    )
    original_lib_roster = json.loads((out / "roster-tcl_compiler.json").read_text())
    require(
        {row["selector"] for row in phase_rows}.issubset(
            original_lib_roster["selected"]
        ),
        "layout own Jim diagnostic controls absent from actual fresh compiler lib inventory",
    )
    for index, row in enumerate(phase_rows):
        package, target_kind, target_name = row["package"], row["kind"], row["target"]
        selected_artifacts = [
            value
            for value in artifacts
            if value["package"] == package
            and value["kind"] == target_kind
            and value["target"] == target_name
        ]
        require(
            len(selected_artifacts) == 1,
            "layout own Jim diagnostic current built artifact absent/ambiguous",
        )
        diagnostic_artifact: dict[str, Any] = selected_artifacts[0]
        require(
            diagnostic_artifact["strict_Rust_0w"] is True
            and diagnostic_artifact["cargo_event"]["profile"]["debug_assertions"]
            is True,
            "layout own Jim diagnostic requires existing fresh strict debug-assertion test profile",
        )
        executable = Path(diagnostic_artifact["path"])
        require(
            sha(executable) == diagnostic_artifact["sha256"],
            "layout own Jim diagnostic artifact changed before listing",
        )
        selector = row["selector"]
        require(remaining > 512 * 1024, "insufficient log budget before original test")
        prefix = row["group"] + "-" + target_name + "-" + str(index).zfill(2)
        group_label = "layout-own-jim"
        print("SELECT " + group_label + " " + selector, flush=True)
        runtime_env = env.copy()
        selected_env = row["env"]
        if not isinstance(selected_env, dict):
            raise TypeError("original per-selector environment is not a mapping")
        for name, value in selected_env.items():
            if not isinstance(name, str) or not isinstance(value, str):
                raise TypeError("original per-selector environment is not text")
            runtime_env[name] = value
        selected_list = execute(
            [str(executable), selector, "--exact", "--list", "--format", "terse"],
            "select-" + prefix,
            executable,
            180,
            runtime_env,
            remaining,
        )
        processes.append(selected_list)
        remaining -= selected_list["retained_bytes"]
        require(
            lifecycle(selected_list) and selected_list["exit_code"] == 0,
            "original exact selector list failed or unclosed",
        )
        selected_lines = (
            (out / ("select-" + prefix + ".stdout")).read_text().splitlines()
        )
        require(
            [line for line in selected_lines if line.endswith(": test")]
            == [selector + ": test"],
            "original exact selector list mismatch",
        )
        require(
            all(
                line == selector + ": test"
                or not line
                or re.fullmatch(r"1 tests?, 0 benchmarks?", line)
                for line in selected_lines
            ),
            "unexpected original exact selector list data",
        )
        require(
            remaining > 512 * 1024,
            "insufficient log budget after original exact selector list",
        )
        child_before = sha(executable)
        require(
            child_before == diagnostic_artifact["sha256"],
            "selected Rust test artifact changed before transport spawn",
        )
        row["selected_test_artifact"] = {
            "path": str(executable),
            "expected_sha256": diagnostic_artifact["sha256"],
            "before_sha256": child_before,
            "after_sha256": None,
            "unchanged": False,
        }
        row["state"] = "RUNNING"
        save(out / "layout-own-jim-outcomes.json", phase_rows)
        print("RUN " + group_label + " " + selector, flush=True)
        receipt = execute(
            [
                str(executable),
                selector,
                "--exact",
                "--nocapture",
                "--test-threads=1",
            ],
            prefix,
            executable,
            180,
            runtime_env,
            remaining,
        )
        processes.append(receipt)
        remaining -= receipt["retained_bytes"]
        child_after = sha(executable)
        row["selected_test_artifact"]["after_sha256"] = child_after
        row["selected_test_artifact"]["unchanged"] = (
            child_before == child_after == diagnostic_artifact["sha256"]
        )
        save(out / "layout-own-jim-outcomes.json", phase_rows)
        require(
            row["selected_test_artifact"]["unchanged"],
            "selected Rust test artifact changed during transport",
        )
        transport_path = out / (prefix + ".transport.json")
        if lifecycle(receipt):
            require(
                transport_path.is_file(), "completed test transport summary missing"
            )
            transport = json.loads(transport_path.read_text())
            encoded_stderr = (out / (prefix + ".stderr")).read_bytes()
            repeat_stage = transport["repeat_stage"]
            table_stage = transport["table_stage"]
            require(
                transport["child_exit_code"] == receipt["exit_code"]
                and transport["ordinary_stderr_verbatim"] is False
                and transport["stderr_losslessly_reconstructible_after_table_filter"]
                is True
                and table_stage["ordinary_stderr_verbatim"] is True
                and table_stage["stdout_verbatim"] is True
                and transport["stdout_verbatim"] is True
                and transport["retained_stderr_bytes"]
                == repeat_stage["output_bytes"]
                == len(encoded_stderr)
                and repeat_stage["format"]
                == "two-family-lossless-line-dictionary-rle-v1"
                and repeat_stage["input_bytes"] == table_stage["retained_stderr_bytes"]
                and repeat_stage["output_sha256"]
                == hashlib.sha256(encoded_stderr).hexdigest(),
                "transport child status or layered byte census mismatch",
            )
            reconstructed = observe_layout_jim_trace(encoded_stderr, repeat_stage, row)
            require(
                reconstructed["bytes"] == repeat_stage["input_bytes"]
                and reconstructed["sha256"] == repeat_stage["input_sha256"],
                "transport diagnostic reconstruction differs from original stream",
            )
            row["transport_reconstruction"] = reconstructed
        row.update(
            exit_code=receipt["exit_code"],
            receipt=prefix + ".process.json",
            stdout=prefix + ".stdout",
            stderr=prefix + ".stderr",
        )
        require(lifecycle(receipt), "original test lifecycle/capture stop: " + selector)
        stdout = (out / (prefix + ".stdout")).read_text(errors="replace")
        try:
            row["state"] = original_result(stdout, receipt["exit_code"])
        except RuntimeError:
            row["state"] = "SIGNAL" if receipt["exit_code"] < 0 else "PROTOCOL_FAILURE"
            save(out / "layout-own-jim-outcomes.json", phase_rows)
            raise
        save(out / "layout-own-jim-outcomes.json", phase_rows)
        require(
            sha(executable) == diagnostic_artifact["sha256"],
            "selected test artifact changed",
        )



class _DedicatedLayoutComplete(Exception):
    """Private successful Layout phase boundary; final seals still execute."""


class _DedicatedCorpusComplete(Exception):
    """Private successful phase boundary; final seals still execute."""


parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--sha", required=True)
parser.add_argument("--out", type=Path, required=True)
parser.add_argument("--source-point-corpus-only", action="store_true")
parser.add_argument("--jim-source-layout-only", action="store_true")
args = parser.parse_args()
require(not (args.source_point_corpus_only and args.jim_source_layout_only),
        "dedicated modes are mutually exclusive")
require(args.jim_source_layout_only
        == (os.environ.get("GITHUB_JOB") == "jim-source-layout11-diagnostics"),
        "fixed Layout11 mode/job mismatch")
require(
    args.source_point_corpus_only
    == (os.environ.get("GITHUB_JOB") == "source-point-corpus2-diagnostics"),
    "fixed dedicated corpus mode/job mismatch",
)
require(re.fullmatch("[0-9a-f]{40}", args.sha), "full exact SHA required")
require(os.environ.get("GITHUB_SHA") == args.sha, "checkout/workflow SHA mismatch")
require(os.environ.get("GITHUB_REF") == "refs/heads/work", "WORK only")
root = Path.cwd().resolve()
out = args.out.resolve()
target = Path(os.environ["CARGO_TARGET_DIR"]).resolve()
deadline = float(os.environ["DIAGNOSTIC_DEADLINE_EPOCH"])
require(
    time.time() < deadline <= time.time() + (12000 if args.jim_source_layout_only else 3600),
    "bounded absolute job deadline required",
)
require(
    not out.exists() and not out.is_relative_to(root),
    "fresh external evidence directory required",
)
require(
    not target.exists() and not target.is_relative_to(root),
    "fresh external target required",
)
out.mkdir(parents=True)
resource.setrlimit(resource.RLIMIT_CORE, (0, 0))
require(
    sha(BASE / "compiler-original58-selectors.json")
    == "d4a45ac4eb52a5743b0cb8a5a8256a7f73214f45627c05e25055f2f58bef95d2",
    "exact original58 selector bytes required",
)
require(
    sha(BASE / "compiler-original58-inputs.json")
    == "682576528999248485edd1e75f925316c5006a751736aeb60c0082c12518e9c0",
    "exact original58 input/env pin",
)
original = json.loads((BASE / "compiler-original58-selectors.json").read_text())
roster = json.loads((BASE / "compiler-original58-inputs.json").read_text())
require(roster["executor_sha256"] == EXECUTOR_SHA, "original process owner pin")
require(len(original["rows"]) == 58, "original58 count required")
require(
    len(
        {
            (
                row["package"],
                tuple(row["target_kind"]),
                row["target_name"],
                row["exact_selector"],
            )
            for row in original["rows"]
        }
    )
    == 58,
    "duplicate original58 selector",
)
original_rows = [
    {
        "group": "original58",
        "package": row["package"],
        "kind": row["target_kind"],
        "target": row["target_name"],
        "selector": row["exact_selector"],
        "env": roster["row_environments"][str(row["historical_filter_index"]).zfill(2)],
        "historical_897b_status": row["historical_897b_status"],
        "state": "UNRUN",
    }
    for row in original["rows"]
]
require(
    sha(BASE / "compiler-native35-supplemental3.json")
    == "279bd4949e4e615ffff582e0f820d1e228456d8df4c84d3a08b67165d31bdb80",
    "exact separate Native35 supplemental3 input pin",
)
supplemental = json.loads((BASE / "compiler-native35-supplemental3.json").read_text())
require(
    supplemental["expected_selected"]
    == len(supplemental["selectors"])
    == len(set(supplemental["selectors"]))
    == 3,
    "separate Native35 supplemental3 roster",
)
require(
    supplemental["package"] == "tcl-compiler"
    and supplemental["kind"] == ["lib"]
    and supplemental["target"] == "tcl_compiler"
    and supplemental["env"] == {"TCL_LSP_TRACE_AUTHENTICATED_REPORT": "1"},
    "separate supplemental target/environment differs",
)
supplemental_rows = [
    {
        "group": "supplemental-native35",
        "package": supplemental["package"],
        "kind": supplemental["kind"],
        "target": supplemental["target"],
        "selector": selector,
        "env": supplemental["env"],
        "state": "UNRUN",
    }
    for selector in supplemental["selectors"]
]
require(
    not {(row["target"], row["selector"]) for row in original_rows}.intersection(
        (row["target"], row["selector"]) for row in supplemental_rows
    ),
    "supplemental roster overlaps original58",
)
require(
    sha(BASE / "compiler-sh21-supplemental2.json")
    == "8d4eb5f02a0324659b689e0678d0ff6a83c4ff443be54c920255da6b2b7c6ccb",
    "exact separate SH21 diagnostic duplicate input pin",
)
sh21_diagnostic = json.loads((BASE / "compiler-sh21-supplemental2.json").read_text())
require(
    sh21_diagnostic["package"] == "tcl-compiler"
    and sh21_diagnostic["kind"] == ["lib"]
    and sh21_diagnostic["target"] == "tcl_compiler"
    and sh21_diagnostic["expected_selected"] == 2
    and sh21_diagnostic["env"] == {"TCL_LSP_TRACE_SH21_PRODUCER": "1"}
    and sh21_diagnostic["selectors"]
    == [
        "analyser::diagnostics::fp::sh::fp_sh_21_scalar_literal_as_list_silent",
        "analyser::diagnostics::fp::sh::fp_sh_21_incr_on_non_integer_literal_still_fires",
    ],
    "separate SH21 diagnostic duplicate roster or flags differ",
)
sh21_rows = [
    {
        "group": "supplemental-sh21",
        "package": sh21_diagnostic["package"],
        "kind": sh21_diagnostic["kind"],
        "target": sh21_diagnostic["target"],
        "selector": selector,
        "env": sh21_diagnostic["env"],
        "state": "UNRUN",
    }
    for selector in sh21_diagnostic["selectors"]
]
require(
    all(
        sum(row["selector"] == duplicate["selector"] for row in original_rows) == 1
        for duplicate in sh21_rows
    ),
    "diagnostic SH21 rows must duplicate exactly the existing original58 identities",
)
require(
    sha(BASE / "compiler-readflow-supplemental1.json")
    == "58273fd8585042a099da73aa67b9e83c91d790607225d88c05da6c324746a9a3",
    "exact separate ReadFlow1 input pin",
)
readflow = json.loads((BASE / "compiler-readflow-supplemental1.json").read_text())
require(
    readflow["package"] == "tcl-compiler"
    and readflow["kind"] == ["lib"]
    and readflow["target"] == "tcl_compiler"
    and readflow["expected_selected"] == 1
    and readflow["env"] == {}
    and readflow["selectors"]
    == [
        "command_binding::generated_procedures::original_native_body_tests::original_read_card_requires_its_actual_vector_origin_and_snapshot"
    ],
    "separate ReadFlow1 roster or environment differs",
)
readflow_rows = [
    {
        "group": "supplemental-readflow1",
        "package": readflow["package"],
        "kind": readflow["kind"],
        "target": readflow["target"],
        "selector": selector,
        "env": readflow["env"],
        "state": "UNRUN",
    }
    for selector in readflow["selectors"]
]
require(
    not {(row["target"], row["selector"]) for row in readflow_rows}.intersection(
        (row["target"], row["selector"])
        for row in original_rows + supplemental_rows + sh21_rows
    ),
    "separate ReadFlow1 selector overlaps an existing group",
)
require(
    sha(BASE / "compiler-catch-diagnostic1.json")
    == "b354a76d54d0510b4bb27e2802c66ffa3d56d2270062a908ba2ae937d3a6edeb",
    "exact separate catch diagnostic1 input pin",
)
catch_diagnostic = json.loads((BASE / "compiler-catch-diagnostic1.json").read_text())
catch_original: list[dict[str, Any]] = [
    row
    for row in original_rows
    if row["package"] == "tcl-compiler"
    and row["kind"] == ["test"]
    and row["target"] == "var_escape_residual2"
    and row["selector"] == "cu_catch_literal_body_threads_its_upvar"
]
require(
    len(catch_original) == 1, "catch duplicate requires exactly one original identity"
)
catch_env = {
    **catch_original[0]["env"],
    "TCL_LSP_TRACE_VAR_ESCAPE_BODY_OWNERS": "1",
}
require(
    catch_diagnostic["package"] == catch_original[0]["package"]
    and catch_diagnostic["kind"] == catch_original[0]["kind"]
    and catch_diagnostic["target"] == catch_original[0]["target"]
    and catch_diagnostic["expected_selected"] == 1
    and catch_diagnostic["selectors"] == [catch_original[0]["selector"]]
    and catch_diagnostic["env"] == catch_env,
    "separate catch diagnostic roster or environment differs",
)
catch_rows = [
    {
        **catch_original[0],
        "group": "diagnostic-catch1",
        "env": catch_env,
        "state": "UNRUN",
    }
]
require(
    sha(BASE / "compiler-taint-diagnostic2.json")
    == "0bf8fb32f273281cca010857e9a34527eaea395af67a949937810982a3ebc92c",
    "exact separate taint diagnostic2 input pin",
)
taint_diagnostic = json.loads((BASE / "compiler-taint-diagnostic2.json").read_text())
taint_original: list[dict[str, Any]] = [
    row
    for row in original_rows
    if row["package"] == "tcl-compiler"
    and row["kind"] == ["test"]
    and row["target"] == "taint"
    and row["selector"] == "dangerous_sinks::eval_sink"
]
require(len(taint_original) == 1, "taint duplicate requires one original identity")
taint_expected: list[dict[str, Any]] = [
    {key: taint_original[0][key] for key in ["package", "kind", "target", "selector"]}
    | {"env": {**taint_original[0]["env"], "TCL_LSP_TRACE_TAINT_CU_INGRESS": "1"}},
    {
        "package": "tcl-compiler",
        "kind": ["lib"],
        "target": "tcl_compiler",
        "selector": "taint::source_context::original_standalone_unit_tests::explicit_standalone_unit_uses_its_current_original_token_owner",
        "env": {"TCL_LSP_TRACE_TAINT_CU_INGRESS": "1"},
    },
]
require(
    taint_diagnostic["expected_selected"] == 2
    and taint_diagnostic["rows"] == taint_expected,
    "taint diagnostic2 exact identities/env differ",
)
taint_rows: list[dict[str, Any]] = [
    {**row, "group": "diagnostic-taint2", "state": "UNRUN"} for row in taint_expected
]
original65_rows = (
    original_rows + supplemental_rows + sh21_rows + readflow_rows + catch_rows
)
require(
    sha(BASE / "compiler-closed-consumer-controls.json") == "604f45e34d9ad26129fc058ddb306aa9cabaa99b16d09f537d938ff2410c40fb",
    "exact separate closed-consumer control roster",
)
closed_controls = json.loads((BASE / "compiler-closed-consumer-controls.json").read_text())
require(closed_controls["expected_selected"] == len(closed_controls["rows"]) == 16
        and closed_controls["env"] == {'TCL_LSP_NATIVE_ADMISSION_DIAGNOSTIC': 'first-decline-v1'}, "separate closed16 count/environment")
closed_rows = [dict(row, group="diagnostic-closed-consumer", env=dict(closed_controls["env"]), state="UNRUN")
               for row in closed_controls["rows"]]
require(len({(row["package"], row["target"], row["selector"]) for row in closed_rows}) == 16,
        "duplicate closed-consumer selector")
require(not {(row["package"], row["target"], row["selector"]) for row in closed_rows}.intersection(
        (row["package"], row["target"], row["selector"]) for row in original65_rows + taint_rows),
        "closed-consumer controls overlap original groups")
require(closed_controls["build_only_targets"] == [
    {"package": package, "kind": ["lib"], "name": name, "expected_selected": 0}
    for package, name in [("tcl-engine-tclvm", "tcl_engine_tclvm"),
                          ("tcl-runtime-api", "tcl_runtime_api"), ("tcl-registry", "tcl_registry")]],
        "exact closed-consumer fresh build-only targets")
require(
    sha(BASE / "compiler-inlining-readonly13.json")
    == "fcbacd7fb71957c35ceca130bc3099a41e939c4e797e7be7888da73456559af9",
    "exact separate readonly13 input roster",
)
readonly_validation = json.loads(
    (BASE / "compiler-inlining-readonly13.json").read_text()
)
require(
    readonly_validation["expected_selected"] == len(readonly_validation["rows"]) == 13,
    "separate readonly13 count",
)
readonly_rows: list[dict[str, Any]] = [
    dict(row, state="UNRUN") for row in readonly_validation["rows"]
]
require(
    [row["group"] for row in readonly_rows]
    == ["diagnostic-inlining-readonly10"] * 10 + ["diagnostic-original-body-owner3"] * 3
    and all(
        row["env"]
        == (
            {"TCL_LSP_TRACE_VAR_ESCAPE_BODY_OWNERS": "1"}
            if row["selector"] in {
                "large_body_profitability_selects_single_caller",
                "small_body_profitability_allows_bounded_growth",
                "threshold_body_profitability_allows_bounded_growth",
                "large_body_profitability_depends_on_static_call_count",
            }
            else {}
        )
        for row in readonly_rows
    ),
    "readonly13 groups/env",
)
require(
    readonly_validation["build_only_target"]
    == {
        "package": "tcl-compiler",
        "kind": ["test"],
        "name": "inlining",
        "expected_selected": 0,
    },
    "readonly fresh target",
)
require(
    len(readonly_validation["full_target_inventory"]) == 77
    and len(set(readonly_validation["full_target_inventory"])) == 77,
    "readonly full source inventory",
)
require(
    all(
        row["package"] == "tcl-compiler"
        and row["kind"] == ["test"]
        and row["target"] == "inlining"
        for row in readonly_rows[:10]
    )
    and all(
        row["package"] == "tcl-compiler"
        and row["kind"] == ["lib"]
        and row["target"] == "tcl_compiler"
        for row in readonly_rows[10:]
    ),
    "readonly13 exact target kinds",
)
require(
    len(
        {
            (row["package"], tuple(row["kind"]), row["target"], row["selector"])
            for row in readonly_rows
        }
    )
    == 13
    and not {
        (row["package"], tuple(row["kind"]), row["target"], row["selector"])
        for row in readonly_rows
    }.intersection(
        (row["package"], tuple(row["kind"]), row["target"], row["selector"])
        for row in original65_rows + taint_rows + closed_rows
    ),
    "readonly13 overlaps/duplicates",
)
require(
    sha(BASE / "compiler-source-read-ancestry3.json")
    == "ed061f196bbab53c2e9994bb74370325fec6df87099f2f83d6cc18107b4d8d08",
    "exact separate ancestry3 roster",
)
ancestry_validation = json.loads(
    (BASE / "compiler-source-read-ancestry3.json").read_text()
)
require(
    ancestry_validation["expected_selected"] == len(ancestry_validation["rows"]) == 3,
    "ancestry3 count",
)
ancestry_rows: list[dict[str, Any]] = [
    dict(row, state="UNRUN") for row in ancestry_validation["rows"]
]
require(
    all(
        row["group"] == "diagnostic-source-read-ancestry3"
        and row["package"] == "tcl-compiler"
        and row["kind"] == ["lib"]
        and row["target"] == "tcl_compiler"
        and row["env"] == {}
        for row in ancestry_rows
    ),
    "ancestry3 exact target/environment",
)
require(
    len(
        {
            (row["package"], tuple(row["kind"]), row["target"], row["selector"])
            for row in ancestry_rows
        }
    )
    == 3
    and not {
        (row["package"], tuple(row["kind"]), row["target"], row["selector"])
        for row in ancestry_rows
    }.intersection(
        (row["package"], tuple(row["kind"]), row["target"], row["selector"])
        for row in original65_rows + taint_rows + closed_rows + readonly_rows
    ),
    "ancestry3 overlaps/duplicates",
)
require(
    sha(BASE / "compiler-logical-declared-read3.json")
    == "f7b5f24d34ec0cf0bdffd79331384bcc7519adec3ac30d0a3a889296948651d3",
    "exact separate logical3 roster",
)
logical_validation = json.loads(
    (BASE / "compiler-logical-declared-read3.json").read_text()
)
require(
    logical_validation["expected_selected"] == len(logical_validation["rows"]) == 3,
    "logical3 count",
)
logical_rows: list[dict[str, Any]] = [
    dict(row, state="UNRUN") for row in logical_validation["rows"]
]
require(
    all(
        row["group"] == "diagnostic-logical-declared-read3"
        and row["package"] == "tcl-compiler"
        and row["kind"] == ["lib"]
        and row["target"] == "tcl_compiler"
        and row["env"] == {}
        for row in logical_rows
    ),
    "logical3 exact target/environment",
)
require(
    len(
        {
            (row["package"], tuple(row["kind"]), row["target"], row["selector"])
            for row in logical_rows
        }
    )
    == 3
    and not {
        (row["package"], tuple(row["kind"]), row["target"], row["selector"])
        for row in logical_rows
    }.intersection(
        (row["package"], tuple(row["kind"]), row["target"], row["selector"])
        for row in original65_rows
        + taint_rows
        + closed_rows
        + readonly_rows
        + ancestry_rows
    ),
    "logical3 overlaps/duplicates",
)
rows = (
    original65_rows
    + taint_rows
    + closed_rows
    + readonly_rows
    + ancestry_rows
    + logical_rows
)

logical_body_validation = logical_validation["separate_body_entry_controls"]
require(
    logical_body_validation["schema"] == "logical-source-body-entry-controls5-v1"
    and logical_body_validation["expected_selected"] == len(logical_body_validation["rows"]) == 5
    and logical_body_validation["source_peer_sha256"] == "738f8ace6ae56604b083c8b6a38d2d3623616679f74148aa35d41df8395cfb09",
    "logical body-entry control schema/count/source peer",
)
logical_body_rows: list[dict[str, Any]] = [dict(row, state="UNRUN") for row in logical_body_validation["rows"]]
require(
    [row["selector"] for row in logical_body_rows] == ['command_binding::declaration_preview::logical_body_entry::tests::installed_logical_declaration_gets_a_real_body_visit_without_native_entry', 'command_binding::declaration_preview::logical_body_entry::tests::replacement_keeps_old_lexical_body_without_old_body_dispatch', 'command_binding::declaration_preview::logical_body_entry::tests::body_unknown_effects_do_not_change_completed_root_world', 'command_binding::declaration_preview::logical_body_entry::tests::scoped_body_entry_refuses_foreign_origin_and_changed_configuration', 'command_binding::declaration_preview::logical_body_entry::tests::declaration_realm_must_match_the_completed_root_realm']
    and all(row["group"] == "diagnostic-logical-source-body-entry5"
        and row["package"] == "tcl-compiler" and row["kind"] == ["lib"]
        and row["target"] == "tcl_compiler" and row["env"] == {} for row in logical_body_rows)
    and not {row["selector"] for row in logical_body_rows}.intersection(row["selector"] for row in rows),
    "logical body-entry exact targets/selectors/environment/no overlap",
)
codec_validation = logical_validation["separate_compact_codec_control"]
require(codec_validation["schema"] == "compact-source-context-codec1-v1"
    and codec_validation["expected_selected"] == len(codec_validation["rows"]) == 1
    and codec_validation["source_peer_sha256"] == "93b1ada4eb7d4e21d23d92c89b69826a07172d84042ca84b6f4cb2e3d7480126",
    "compact codec control schema/count/source peer")
codec_rows: list[dict[str, Any]] = [dict(row, state="UNRUN") for row in codec_validation["rows"]]
require(codec_validation["rows"] == [{'group': 'diagnostic-compact-codec1', 'package': 'tcl-compiler', 'kind': ['lib'], 'target': 'tcl_compiler', 'selector': 'command_binding::source_execution_control::tests::compact_control_roundtrips_full_native_boundaries_and_independent_source_facts', 'env': {}}],
    "compact codec exact target/selector/environment")

named_child_validation = logical_validation["separate_original_child_controls"]
require(named_child_validation["schema"] == "compiler-original-child-all10-v1"
    and named_child_validation["expected_selected"] == len(named_child_validation["rows"]) == 10
    and named_child_validation["source_peer_reference"] == 'source-peer-known-child-live-producer-v4/receipt.json sha256:571eb418358d3ba0483bb0ec5bbbd59f2df50de4482e0e96d943a4ebdfaa278e'
    and set(named_child_validation["source_pins"]) == {'rust/tcl-compiler/src/command_binding/declaration_preview/static_read_child.rs'}
    and all(logical_validation["source_pins"].get(path) == value
        for path, value in named_child_validation["source_pins"].items()),
    "original child controls schema/count/current source owner")
named_child_rows: list[dict[str, Any]] = [dict(row, state="UNRUN") for row in named_child_validation["rows"]]
require(named_child_validation["rows"] == [{'group': 'diagnostic-original-child-all10', 'package': 'tcl-compiler', 'kind': ['lib'], 'target': 'tcl_compiler', 'selector': 'command_binding::declaration_preview::static_read_child::tests::original_nested_named_static_read_keeps_original_positive_expectations', 'env': {}}, {'group': 'diagnostic-original-child-all10', 'package': 'tcl-compiler', 'kind': ['lib'], 'target': 'tcl_compiler', 'selector': 'command_binding::declaration_preview::static_read_child::tests::original_nested_named_read_refuses_alias_rebinding_callee_and_opaque_children', 'env': {}}, {'group': 'diagnostic-original-child-all10', 'package': 'tcl-compiler', 'kind': ['lib'], 'target': 'tcl_compiler', 'selector': 'command_binding::declaration_preview::static_read_child::tests::original_nested_named_read_refuses_equal_foreign_and_stale_source_owners', 'env': {}}, {'group': 'diagnostic-original-child-all10', 'package': 'tcl-compiler', 'kind': ['lib'], 'target': 'tcl_compiler', 'selector': 'command_binding::declaration_preview::static_read_child::tests::original_nested_read_rejects_a_child_with_a_foreign_frame', 'env': {}}, {'group': 'diagnostic-original-child-all10', 'package': 'tcl-compiler', 'kind': ['lib'], 'target': 'tcl_compiler', 'selector': 'command_binding::declaration_preview::static_read_child::tests::original_child_body_keeps_its_issued_frame_distinct_from_header_layout', 'env': {}}, {'group': 'diagnostic-original-child-all10', 'package': 'tcl-compiler', 'kind': ['lib'], 'target': 'tcl_compiler', 'selector': 'command_binding::declaration_preview::static_read_child::tests::original_child_body_frame_refuses_missing_reissued_and_foreign_rows', 'env': {}}, {'group': 'diagnostic-original-child-all10', 'package': 'tcl-compiler', 'kind': ['lib'], 'target': 'tcl_compiler', 'selector': 'command_binding::declaration_preview::static_read_child::tests::original_child_body_frame_refuses_different_allocation_and_missing_formals', 'env': {}}, {'group': 'diagnostic-original-child-all10', 'package': 'tcl-compiler', 'kind': ['lib'], 'target': 'tcl_compiler', 'selector': 'command_binding::declaration_preview::static_read_child::tests::known_child_projection_uses_live_producer_and_separate_body_scope', 'env': {}}, {'group': 'diagnostic-original-child-all10', 'package': 'tcl-compiler', 'kind': ['lib'], 'target': 'tcl_compiler', 'selector': 'command_binding::declaration_preview::static_read_child::tests::known_child_projection_refuses_consistently_replaced_producer_fields', 'env': {}}, {'group': 'diagnostic-original-child-all10', 'package': 'tcl-compiler', 'kind': ['lib'], 'target': 'tcl_compiler', 'selector': 'command_binding::declaration_preview::static_read_child::tests::known_child_projection_refuses_foreign_and_retired_source_owner', 'env': {}}]
    and not {row["selector"] for row in named_child_rows}.intersection(
        row["selector"] for row in rows + logical_body_rows + codec_rows),
    "original child exact targets/selectors/environment/no overlap")

restored_child_validation = logical_validation["separate_restored_child_owner_control"]
require(restored_child_validation["schema"] == "restored-child-analysis-owner-control1-v1"
    and restored_child_validation["expected_selected"] == len(restored_child_validation["rows"]) == 1
    and restored_child_validation["source"] == 'rust/tcl-compiler/src/command_binding/compiler_inventory.rs'
    and logical_validation["source_pins"].get(restored_child_validation["source"]) == restored_child_validation["source_sha256"],
    "restored child owner control schema/count/source pin")
restored_child_rows: list[dict[str, Any]] = [dict(row, state="UNRUN") for row in restored_child_validation["rows"]]
require(restored_child_validation["rows"] == [{'group': 'diagnostic-restored-child-owner1', 'package': 'tcl-compiler', 'kind': ['lib'], 'target': 'tcl_compiler', 'selector': 'command_binding::compiler_inventory::original_inline_control_source_tests::restored_nested_compiler_carriers_issue_only_current_complete_source_owners', 'env': {'TCL_LSP_TRACE_BODY_SYNTAX': '1'}}]
    and not {row["selector"] for row in restored_child_rows}.intersection(
        row["selector"] for row in rows + logical_body_rows + codec_rows + named_child_rows),
    "restored child owner exact target/selector/environment/no overlap")

native_string_validation = logical_validation["separate_native_string_effect_controls"]
require(set(native_string_validation) == {"schema", "expected_selected", "source", "source_peer_reference", "rows"}
    and native_string_validation["schema"] == "native-string-effect-control3-v1"
    and native_string_validation["expected_selected"] == len(native_string_validation["rows"]) == 3
    and native_string_validation["source"] == 'rust/tcl-compiler/src/command_binding/native_string_access/tests.rs'
    and native_string_validation["source"] in logical_validation["source_pins"],
    "native string effects schema/count/current pinned test source")
native_string_rows: list[dict[str, Any]] = [dict(row, state="UNRUN") for row in native_string_validation["rows"]]
require(native_string_validation["rows"] == [{'group': 'diagnostic-native-string-effects3', 'package': 'tcl-compiler', 'kind': ['lib'], 'target': 'tcl_compiler', 'selector': 'command_binding::native_string_access::tests::same_actual_source_template_and_append_close_only_string_effects', 'env': {}}, {'group': 'diagnostic-native-string-effects3', 'package': 'tcl-compiler', 'kind': ['lib'], 'target': 'tcl_compiler', 'selector': 'command_binding::native_string_access::tests::unknown_component_and_observers_cannot_launder_a_string_result_receipt', 'env': {}}, {'group': 'diagnostic-native-string-effects3', 'package': 'tcl-compiler', 'kind': ['lib'], 'target': 'tcl_compiler', 'selector': 'command_binding::native_string_access::tests::captured_native_effect_owner_refuses_foreign_source_and_expires_without_fallback', 'env': {}}]
    and not {row["selector"] for row in native_string_rows}.intersection(
        row["selector"] for row in rows + logical_body_rows + codec_rows + named_child_rows + restored_child_rows),
    "native string effects exact target/selectors/environment/no overlap")

native_formal_validation = logical_validation["separate_native_formal_layout_controls"]
require(set(native_formal_validation) == {"schema", "expected_selected", "source", "source_peer_reference", "rows"}
    and native_formal_validation["schema"] == "native-formal-layout-control2-v1"
    and native_formal_validation["expected_selected"] == len(native_formal_validation["rows"]) == 2
    and native_formal_validation["source"] == 'rust/tcl-compiler/src/command_binding/compiled_preflight/ordered_locals.rs'
    and native_formal_validation["source"] in logical_validation["source_pins"],
    "native formal layout schema/count/current pinned test source")
native_formal_rows: list[dict[str, Any]] = [dict(row, state="UNRUN") for row in native_formal_validation["rows"]]
require(native_formal_validation["rows"] == [{'group': 'diagnostic-native-formal-layout2', 'package': 'tcl-compiler', 'kind': ['lib'], 'target': 'tcl_compiler', 'selector': 'command_binding::compiled_preflight::ordered_locals::original_native_request_tests::original_native_request_formals_keep_actual_scope_and_withdraw_after_owner_retirement', 'env': {}}, {'group': 'diagnostic-native-formal-layout2', 'package': 'tcl-compiler', 'kind': ['lib'], 'target': 'tcl_compiler', 'selector': 'command_binding::compiled_preflight::ordered_locals::original_native_request_tests::original_native_request_formals_refuse_foreign_scope_and_keep_existing_body_boundaries', 'env': {}}]
    and not {row["selector"] for row in native_formal_rows}.intersection(
        row["selector"] for row in rows + logical_body_rows + codec_rows + named_child_rows + restored_child_rows + native_string_rows),
    "native formal layout exact target/selectors/environment/no overlap")

analyser_layout_validation = logical_validation["separate_analyser_layout_controls"]
require(set(analyser_layout_validation) == {"schema", "expected_selected", "source", "source_peer_reference", "rows"}
    and analyser_layout_validation["schema"] == "analyser-layout-control3-v1"
    and analyser_layout_validation["expected_selected"] == len(analyser_layout_validation["rows"]) == 3
    and analyser_layout_validation["source"] == 'rust/tcl-compiler/src/analyser/handlers.rs'
    and analyser_layout_validation["source"] in logical_validation["source_pins"],
    "analyser layout schema/count/current pinned test source")
analyser_layout_rows: list[dict[str, Any]] = [dict(row, state="UNRUN") for row in analyser_layout_validation["rows"]]
require(analyser_layout_validation["rows"] == [{'group': 'diagnostic-analyser-layout3', 'package': 'tcl-compiler', 'kind': ['lib'], 'target': 'tcl_compiler', 'selector': 'analyser::handlers::original_proc_source_layout_tests::original_jim_source_layout_keeps_statics_and_actual_body_in_shared_registry', 'env': {}}, {'group': 'diagnostic-analyser-layout3', 'package': 'tcl-compiler', 'kind': ['lib'], 'target': 'tcl_compiler', 'selector': 'analyser::handlers::original_proc_source_layout_tests::original_procedure_source_layout_refuses_foreign_values_tokens_and_expansion', 'env': {}}, {'group': 'diagnostic-analyser-layout3', 'package': 'tcl-compiler', 'kind': ['lib'], 'target': 'tcl_compiler', 'selector': 'analyser::handlers::original_proc_source_layout_tests::original_procedure_source_layout_keeps_plain_and_rooted_jim_forms', 'env': {}}]
    and not {row["selector"] for row in analyser_layout_rows}.intersection(
        row["selector"] for row in rows + logical_body_rows + codec_rows + named_child_rows + restored_child_rows + native_string_rows + native_formal_rows),
    "analyser layout exact target/selectors/environment/no overlap")

own_jim_member_validation = logical_validation["separate_own_jim_member_controls"]
require(set(own_jim_member_validation) == {"schema", "expected_selected", "source", "source_peer_reference", "rows"}
    and own_jim_member_validation["schema"] == "own-jim-member-control3-v1"
    and own_jim_member_validation["expected_selected"] == len(own_jim_member_validation["rows"]) == 3
    and own_jim_member_validation["source"] == 'rust/tcl-compiler/src/command_binding/source_transition_advice/source_jim_member.rs'
    and own_jim_member_validation["source"] in logical_validation["source_pins"],
    "own Jim member schema/count/current pinned test source")
own_jim_member_rows: list[dict[str, Any]] = [dict(row, state="UNRUN") for row in own_jim_member_validation["rows"]]
require(own_jim_member_validation["rows"] == [{'group': 'diagnostic-own-jim-member3', 'package': 'tcl-compiler', 'kind': ['lib'], 'target': 'tcl_compiler', 'selector': 'command_binding::source_transition_advice::source_jim_member::tests::own_jim_member_keeps_before_and_after_factory_custody', 'env': {}}, {'group': 'diagnostic-own-jim-member3', 'package': 'tcl-compiler', 'kind': ['lib'], 'target': 'tcl_compiler', 'selector': 'command_binding::source_transition_advice::source_jim_member::tests::own_jim_member_refuses_foreign_and_corrupt_recipes', 'env': {}}, {'group': 'diagnostic-own-jim-member3', 'package': 'tcl-compiler', 'kind': ['lib'], 'target': 'tcl_compiler', 'selector': 'command_binding::source_transition_advice::source_jim_member::tests::own_jim_member_refuses_rebound_class_and_replaced_initialisers', 'env': {}}]
    and not {row["selector"] for row in own_jim_member_rows}.intersection(
        row["selector"] for row in rows + logical_body_rows + codec_rows + named_child_rows + restored_child_rows + native_string_rows + native_formal_rows + analyser_layout_rows),
    "own Jim member exact target/selectors/environment/no overlap")

advice_catalogue_validation = logical_validation["separate_advice_catalogue_controls"]
require(set(advice_catalogue_validation) == {"schema", "expected_selected", "source", "source_peer_reference", "rows"}
    and advice_catalogue_validation["schema"] == "advice-catalogue-control3-v1"
    and advice_catalogue_validation["expected_selected"] == len(advice_catalogue_validation["rows"]) == 3
    and advice_catalogue_validation["source"] == 'rust/tcl-compiler/src/command_binding/source_transition_advice/catalogue_seed.rs'
    and advice_catalogue_validation["source"] in logical_validation["source_pins"],
    "advice catalogue schema/count/current pinned test source")
advice_catalogue_rows: list[dict[str, Any]] = [dict(row, state="UNRUN") for row in advice_catalogue_validation["rows"]]
require(advice_catalogue_validation["rows"] == [{'group': 'diagnostic-advice-catalogue3', 'package': 'tcl-compiler', 'kind': ['lib'], 'target': 'tcl_compiler', 'selector': 'command_binding::source_transition_advice::catalogue_seed::tests::cached_catalogue_keeps_fresh_budgets_and_independent_mutations', 'env': {}}, {'group': 'diagnostic-advice-catalogue3', 'package': 'tcl-compiler', 'kind': ['lib'], 'target': 'tcl_compiler', 'selector': 'command_binding::source_transition_advice::catalogue_seed::tests::changed_context_store_policy_and_dialect_cannot_reuse_catalogue', 'env': {}}, {'group': 'diagnostic-advice-catalogue3', 'package': 'tcl-compiler', 'kind': ['lib'], 'target': 'tcl_compiler', 'selector': 'command_binding::source_transition_advice::catalogue_seed::tests::native_overlays_and_poison_preserve_uncached_catalogue_boundaries', 'env': {}}]
    and not {row["selector"] for row in advice_catalogue_rows}.intersection(
        row["selector"] for row in rows + logical_body_rows + codec_rows + named_child_rows + restored_child_rows + native_string_rows + native_formal_rows + analyser_layout_rows + own_jim_member_rows),
    "advice catalogue exact target/selectors/environment/no overlap")

native_eligibility_validation = readonly_validation["separate_native_eligibility_controls"]
require(native_eligibility_validation == {'schema': 'native-eligibility-control2-v1', 'expected_selected': 2, 'rows': [{'group': 'diagnostic-native-eligibility2', 'package': 'tcl-compiler', 'kind': ['test'], 'target': 'inlining', 'selector': 'native_observed_add_arguments_do_not_authorize_generic_body_relocation', 'env': {}}, {'group': 'diagnostic-native-eligibility2', 'package': 'tcl-compiler', 'kind': ['test'], 'target': 'inlining', 'selector': 'retired_native_body_custody_never_authorizes_classification', 'env': {}}]}, "exact Native eligibility2 roster")
native_eligibility_rows: list[dict[str, Any]] = [dict(row, state="UNRUN") for row in native_eligibility_validation["rows"]]

own_jim_alias_validation = logical_validation["separate_own_jim_alias_diagnostic"]
require(own_jim_alias_validation["schema"] == "own-jim-alias-firstguard1-v1" and own_jim_alias_validation["expected_selected"] == 1 and own_jim_alias_validation["source"] == 'rust/tcl-compiler/src/command_binding/source_transition_advice.rs' and own_jim_alias_validation["source"] in logical_validation["source_pins"] and own_jim_alias_validation["rows"] == [{'group': 'diagnostic-own-jim-alias-firstguard1', 'package': 'tcl-compiler', 'kind': ['lib'], 'target': 'tcl_compiler', 'selector': 'command_binding::source_transition_advice::source_jim_member::tests::own_jim_member_refuses_rebound_class_and_replaced_initialisers', 'env': {'TCL_LSP_TRACE_JIM_SOURCE_OWNER': '1'}}] and own_jim_alias_validation["repeats_existing_selector"] == 'command_binding::source_transition_advice::source_jim_member::tests::own_jim_member_refuses_rebound_class_and_replaced_initialisers', "exact authorised alias diagnostic repeat")
own_jim_alias_rows: list[dict[str, Any]] = [dict(row, state="UNRUN", alias_trace_observation="UNOBSERVED") for row in own_jim_alias_validation["rows"]]

inherited_report_validation = logical_validation["separate_inherited_point_report_controls"]
require(inherited_report_validation["schema"] == "inherited-point-report-custody3-v1"
        and inherited_report_validation["expected_selected"] == 3
        and inherited_report_validation["source_keys"] == ['rust/tcl-compiler/src/command_binding/retained_source/inherited_point_report_tests.rs', 'rust/tcl-compiler/src/command_binding/retained_source.rs']
        and set(inherited_report_validation["source_keys"]).issubset(logical_validation["source_pins"])
        and inherited_report_validation["rows"] == [{'group': 'diagnostic-inherited-point-report3', 'package': 'tcl-compiler', 'kind': ['lib'], 'target': 'tcl_compiler', 'selector': 'command_binding::retained_source::inherited_point_report_tests::inherited_allocated_report_custody_keeps_generic_membership_closed', 'env': {}}, {'group': 'diagnostic-inherited-point-report3', 'package': 'tcl-compiler', 'kind': ['lib'], 'target': 'tcl_compiler', 'selector': 'command_binding::retained_source::inherited_point_report_tests::inherited_report_custody_refuses_foreign_entry_store_and_detached_body', 'env': {}}, {'group': 'diagnostic-inherited-point-report3', 'package': 'tcl-compiler', 'kind': ['lib'], 'target': 'tcl_compiler', 'selector': 'command_binding::retained_source::inherited_point_report_tests::inherited_report_custody_refuses_changed_header_source_namespace_and_allocation', 'env': {}}],
        "exact inherited point report custody3 descriptor/source/selector/environment")
inherited_report_rows = [dict(row, state="UNRUN") for row in inherited_report_validation["rows"]]

source_point_memo_validation = logical_validation["separate_source_point_memo_controls"]
require(source_point_memo_validation["schema"] == "source-point-memo8-v1" and source_point_memo_validation["expected_selected"] == 8 and source_point_memo_validation["source"] in logical_validation["source_pins"] and source_point_memo_validation["required_absent_environment"] == "TCL_LSP_DIAGNOSTIC_SOURCE_POINT_MEMO_BYPASS" and source_point_memo_validation["rows"] == [{'group': 'diagnostic-source-point-memo8', 'package': 'tcl-compiler', 'kind': ['lib'], 'target': 'tcl_compiler', 'selector': 'command_binding::source_analysis_cache::tests::authored_source_memo_matches_complete_input_and_independent_builder_clones', 'env': {}}, {'group': 'diagnostic-source-point-memo8', 'package': 'tcl-compiler', 'kind': ['lib'], 'target': 'tcl_compiler', 'selector': 'command_binding::source_analysis_cache::tests::authored_source_memo_keeps_supplied_metadata_generation_and_missing_tag', 'env': {}}, {'group': 'diagnostic-source-point-memo8', 'package': 'tcl-compiler', 'kind': ['lib'], 'target': 'tcl_compiler', 'selector': 'command_binding::source_analysis_cache::tests::authored_source_memo_bounds_history_and_refuses_oversized_sources', 'env': {}}, {'group': 'diagnostic-source-point-memo8', 'package': 'tcl-compiler', 'kind': ['lib'], 'target': 'tcl_compiler', 'selector': 'command_binding::source_analysis_cache::tests::authored_source_memo_distinguishes_channel_config_entry_frame_and_registry', 'env': {}}, {'group': 'diagnostic-source-point-memo8', 'package': 'tcl-compiler', 'kind': ['lib'], 'target': 'tcl_compiler', 'selector': 'command_binding::source_analysis_cache::tests::source_point_cache_live_lease_keeps_snapshot_acyclic_and_shares_while_live', 'env': {}}, {'group': 'diagnostic-source-point-memo8', 'package': 'tcl-compiler', 'kind': ['lib'], 'target': 'tcl_compiler', 'selector': 'command_binding::source_analysis_cache::tests::source_point_cache_retirement_drops_parent_payload_but_retains_only_history_origin', 'env': {}}, {'group': 'diagnostic-source-point-memo8', 'package': 'tcl-compiler', 'kind': ['lib'], 'target': 'tcl_compiler', 'selector': 'command_binding::source_analysis_cache::tests::source_point_cache_rejects_foreign_and_changed_context_keys', 'env': {}}, {'group': 'diagnostic-source-point-memo8', 'package': 'tcl-compiler', 'kind': ['lib'], 'target': 'tcl_compiler', 'selector': 'command_binding::source_analysis_cache::tests::source_point_cache_full_report_cold_warm_and_mixed_stay_equal', 'env': {}}], "exact separate memo8 roster")
source_point_memo_rows: list[dict[str, Any]] = [dict(row, state="UNRUN") for row in source_point_memo_validation["rows"]]
analyser_layout_diagnostic_validation = logical_validation["separate_analyser_layout_diagnostics"]
require(analyser_layout_diagnostic_validation["schema"] == "analyser-layout-firstguard3-v1" and analyser_layout_diagnostic_validation["expected_selected"] == 3 and analyser_layout_diagnostic_validation["source"] == 'rust/tcl-compiler/src/analyser/handlers.rs' and analyser_layout_diagnostic_validation["source"] in logical_validation["source_pins"] and analyser_layout_diagnostic_validation["rows"] == [{'group': 'diagnostic-analyser-layout-firstguard3', 'package': 'tcl-compiler', 'kind': ['lib'], 'target': 'tcl_compiler', 'selector': 'analyser::handlers::original_proc_source_layout_tests::original_jim_source_layout_keeps_statics_and_actual_body_in_shared_registry', 'env': {'TCL_LSP_TRACE_JIM_SOURCE_OWNER': '1'}}, {'group': 'diagnostic-analyser-layout-firstguard3', 'package': 'tcl-compiler', 'kind': ['lib'], 'target': 'tcl_compiler', 'selector': 'analyser::handlers::original_proc_source_layout_tests::original_procedure_source_layout_refuses_foreign_values_tokens_and_expansion', 'env': {'TCL_LSP_TRACE_JIM_SOURCE_OWNER': '1'}}, {'group': 'diagnostic-analyser-layout-firstguard3', 'package': 'tcl-compiler', 'kind': ['lib'], 'target': 'tcl_compiler', 'selector': 'analyser::handlers::original_proc_source_layout_tests::original_procedure_source_layout_keeps_plain_and_rooted_jim_forms', 'env': {'TCL_LSP_TRACE_JIM_SOURCE_OWNER': '1'}}], "exact separate layout diagnostic3 roster")
analyser_layout_diagnostic_rows: list[dict[str, Any]] = [dict(row, state="UNRUN", layout_trace_observation="UNOBSERVED") for row in analyser_layout_diagnostic_validation["rows"]]

source_point_corpus_validation = json.loads((BASE / "source-point-complete-text-corpus2.json").read_text())
require(sha(BASE / "source-point-complete-text-corpus2.json") == "c0b8f8ea7c655e5cb495cd725433e84a4418c6208a4f28d3edf7efbd3e0c7818", "exact separately bounded complete text corpus roster")
source_point_corpus_rows = [dict(row, state="UNRUN") for row in source_point_corpus_validation["rows"]]

source_point_report_source_pins = {'rust/tcl-compiler/src/command_binding/source_analysis_cache.rs': 'd0bc5a4b132c7bbdf5aabb28980ef127d2c9e500b7c02906367f07cb74b3eae7', 'rust/tcl-compiler/src/analyser/authenticated_report.rs': '85341e14c948143008d17defb79b0ed5ee26d81f1b90e4c67189beaca1eccfa0', 'rust/tcl-compiler/src/analyser/authenticated_report/body_correspondence.rs': '5c71f5e71feced1fd8ec839dd7b4522d3343960e8f36f05e0813837ddcd9e27d'}
source_point_report_rows = [dict(row, state="UNRUN", capture_observation="UNOBSERVED") for row in [{'group': 'diagnostic-source-point-report-firstguard1', 'package': 'tcl-compiler', 'kind': ['lib'], 'target': 'tcl_compiler', 'selector': 'command_binding::source_analysis_cache::tests::source_point_cache_full_report_cold_warm_and_mixed_stay_equal', 'env': {'TCL_LSP_TRACE_AUTHENTICATED_REPORT': '1'}}]]

require(
    len(
        {
            (
                row["group"],
                row["package"],
                tuple(row["kind"]),
                row["target"],
                row["selector"],
            )
            for row in original65_rows
        }
    )
    == 65,
    "separate original58/native35three/SH21two/ReadFlowone/catchone group identity conflict",
)
expected_targets = [
    ("tcl-compiler", ["lib"], "tcl_compiler", 52),
    ("tcl-compiler", ["test"], "taint", 1),
    ("tcl-compiler", ["test"], "optimiser", 1),
    ("tcl-compiler", ["test"], "var_escape_residual2", 1),
    ("bpf-tcl-ir", ["lib"], "bpf_tcl_ir", 3),
]
require(
    [
        (t["package"], t["kind"], t["name"], t["expected_selected"])
        for t in roster["targets"]
    ]
    == expected_targets,
    "original58 exact five target/count roster",
)
for package, kind, name, count in expected_targets:
    require(
        sum(
            row["package"] == package and row["kind"] == kind and row["target"] == name
            for row in original_rows
        )
        == count,
        "original selected target count differs",
    )
jim_layout_validation = None
jim_layout_rows = []
layout_jim_diagnostic_rows = []
layout11_acceptance_exit = None
if args.jim_source_layout_only:
    require(sha(BASE / "compiler-jim-source-layout11.json") == "ca5d211bb57853197df3ee31a1ac0c2fe68bec047da8e78efbf31295017c3168",
            "exact Layout11 roster/direct source pins required")
    jim_layout_validation = json.loads((BASE / "compiler-jim-source-layout11.json").read_text())
    require(jim_layout_validation["expected_selected"] == 11
            and jim_layout_validation["executor_sha256"] == EXECUTOR_SHA
            and len(jim_layout_validation["rows"]) == 11
            and len({row["selector"] for row in jim_layout_validation["rows"]}) == 11,
            "Layout11 fixed roster/owner count mismatch")
    for row in jim_layout_validation["rows"]:
        require(row["expected_selected"] == 1 and row["env"] == {}
                and row["list_seconds"] == row["budget_seconds"] == 180,
                "Layout11 exact row/environment/budget mismatch")
        jim_layout_rows.append({**row, "group": "diagnostic-jim-source-layout11", "state": "UNRUN"})
    diagnostic = jim_layout_validation["separate_own_jim_diagnostics"]
    require(diagnostic["expected_required"] == diagnostic["expected_optional"] == 1
            and len(diagnostic["rows"]) == 2,
            "exact required alias1 plus optional positive1")
    for row in diagnostic["rows"]:
        require(row["list_seconds"] == row["budget_seconds"] == 180
                and row["cleanup_reservation_seconds"] == 40
                and row["expected_selected"] == 1,
                "own Jim diagnostic full fixed stage budgets")
        layout_jim_diagnostic_rows.append({**row, "state": "UNRUN",
            "admission": "NOT_ATTEMPTED", "jim_trace_observation": "UNOBSERVED",
            "first_refusal_observation": "UNOBSERVED"})
remaining = CAP
before = providers_before = dependencies_before = tool_pins = None
artifacts = []
processes = []
error = None
final_errors = []
status = 2
save(out / "outcomes.json", rows)
try:
    executor = BASE / "runtime-diagnostics-exec.py"
    require(sha(executor) == EXECUTOR_SHA, "reviewed process owner changed")
    spec = importlib.util.spec_from_file_location("compiler58_owner", executor)
    if spec is None or spec.loader is None:
        raise RuntimeError("process owner loader absent")
    owner = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(owner)
    rustup = shutil.which("rustup")
    if rustup is None:
        raise RuntimeError("rustup absent")
    tools = {"python": Path(sys.executable).resolve(), "executor": executor}
    table_filter = BASE / "compiler-original58-table-filter.py"
    require(
        sha(table_filter)
        == "7fcb9fec2c51b591a70845703c78b4326b2e24a3b7ec915279ebfc6d7f80b03b",
        "exact bounded table transport pin",
    )
    tools["table_filter"] = table_filter
    filter_spec = importlib.util.spec_from_file_location(
        "compiler58_filter", table_filter
    )
    if filter_spec is None or filter_spec.loader is None:
        raise RuntimeError("transport decoder loader absent")
    transport_filter = importlib.util.module_from_spec(filter_spec)
    filter_spec.loader.exec_module(transport_filter)
    for name in ["cargo", "rustc"]:
        tools[name] = Path(
            subprocess.check_output([rustup, "which", name], text=True).strip()
        ).resolve()
    for name in ["cc", "ar", "ld"]:
        path = shutil.which(name)
        if path is None:
            raise RuntimeError("native tool absent: " + name)
        tools[name] = Path(path).resolve()
    reference_rows = [
        line.split("\t")
        for line in (root / "rust/tcl-dialect/data/reference-toolchains.tsv")
        .read_text()
        .splitlines()
        if line.strip() and not line.startswith("#")
    ]
    require(
        len(reference_rows) == 5 and all(len(row) == 3 for row in reference_rows),
        "exact original five Tcl reference rows required",
    )
    require(
        [row[0] for row in reference_rows] == ["8.4", "8.5", "8.6", "9.0", "9.1"],
        "original Tcl release roster differs",
    )
    tcl_reference_roots = []
    for release, patchlevel, _tag in reference_rows:
        require(
            re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+", patchlevel),
            "invalid pinned Tcl patchlevel",
        )
        suffix = release.replace(".", "")
        tools["tclsh" + suffix] = Path(os.environ["TCL_LSP_TCLSH" + suffix]).resolve()
        require(
            tools["tclsh" + suffix].is_file(), "original Tcl wrapper absent: " + release
        )
        reference_root = root / "tmp" / ("tcl" + patchlevel)
        native_binary = reference_root / "unix/tclsh"
        require(
            native_binary.is_file(), "actual pinned Tcl interpreter absent: " + release
        )
        tools["tclsh" + suffix + "-native"] = native_binary.resolve()
        tcl_reference_roots.append(reference_root)
    tools["jimsh"] = Path(os.environ["TCL_LSP_JIMSH"]).resolve()
    require(tools["jimsh"].is_file(), "pinned existing Jim reference absent")
    tool_pins = files(list(tools.values()))
    versions = {
        name: subprocess.check_output([str(path), "--version"], text=True)
        for name, path in tools.items()
        if name not in ["executor", "jimsh", "table_filter"]
        and not name.startswith("tclsh")
    }
    versions["rustc_verbose"] = subprocess.check_output(
        [str(tools["rustc"]), "-vV"], text=True
    )
    env = os.environ.copy()
    for name in [
        "RUSTC_WRAPPER",
        "RUSTC_WORKSPACE_WRAPPER",
        "CARGO_ENCODED_RUSTFLAGS",
        "RUSTC_BOOTSTRAP",
        "TCL_LIBRARY",
        "TCLLIBPATH",
        "TCL_LSP_TCL_REFERENCE_MANIFEST",
        "TCL_RUNTIME_EXPECT_NO_TOMMATH",
        "CARGO_BUILD_TARGET",
        "CARGO_BUILD_RUSTC",
        "CARGO_BUILD_RUSTFLAGS",
        "CARGO_BUILD_RUSTC_WRAPPER",
        "CARGO_BUILD_RUSTC_WORKSPACE_WRAPPER",
    ]:
        require(not env.get(name), "unexpected build override: " + name)
    require(
        not any(
            name.startswith("CARGO_TARGET_") and name.endswith("_RUSTFLAGS") and value
            for name, value in env.items()
        ),
        "unexpected target Rust flags",
    )
    require(env.get("RUSTFLAGS") == "-Dwarnings", "strict Rust warnings required")
    require(
        env.get("CARGO_PROFILE_DEV_DEBUG") == "0"
        and env.get("CARGO_INCREMENTAL") == "0"
        and env.get("CARGO_BUILD_JOBS") == "2"
        and env.get("CARGO_HTTP_MULTIPLEXING") == "false",
        "original native build settings differ",
    )
    require(
        env.get("CARGO_PROFILE_TEST_DEBUG") == "0", "original test debug-info setting"
    )
    require(
        env.get("TCL_LSP_REQUIRE_TCLSH") == "9.0,8.6",
        "original required Tcl references",
    )
    require(
        not any(
            name in env
            for flags in roster["row_environments"].values()
            for name in flags
        ),
        "original diagnostic flags must remain per-selector only",
    )
    require(
        "TCL_LSP_TRACE_SH21_PRODUCER" not in env,
        "separate SH21 diagnostic flag must remain per-selector only",
    )
    require(
        "TCL_LSP_TRACE_VAR_ESCAPE_BODY_OWNERS" not in env,
        "separate catch diagnostic flag must remain per-selector only",
    )
    require(
        "TCL_LSP_TRACE_TAINT_CU_INGRESS" not in env,
        "separate taint diagnostic flag must remain per-selector only",
    )
    jim_root = Path(env["TCL_LSP_JIM_ROOT"]).resolve()
    jim_reference = dict(
        line.split("=", 1)
        for line in (root / "rust/tcl-test-support/jim-reference.txt")
        .read_text()
        .splitlines()
    )
    require(
        jim_reference["revision"] == roster["jim_revision"]
        and jim_reference["patchlevel"] == roster["jim_patchlevel"],
        "original Jim manifest pin",
    )
    jim_commit = subprocess.check_output(
        ["git", "-C", str(jim_root), "rev-parse", "HEAD"], text=True
    ).strip()
    require(jim_commit == jim_reference["revision"], "Jim source revision differs")
    require(
        not subprocess.check_output(
            ["git", "-C", str(jim_root), "diff", "HEAD", "--"], text=True
        ),
        "Jim provider tracked source changed",
    )
    require(
        tools["jimsh"].is_relative_to(jim_root),
        "Jim executable outside pinned provider",
    )
    versions["jim_patchlevel"] = subprocess.check_output(
        [str(tools["jimsh"]), "-e", "puts [info patchlevel]"], text=True
    ).strip()
    require(
        versions["jim_patchlevel"] == jim_reference["patchlevel"],
        "Jim executable version differs",
    )
    require(
        env.get("TCL_LSP_TCL_SOURCE_PARENT") == str(root / "tmp"),
        "exact original Tcl source parent required",
    )
    env["RUSTC"] = str(tools["rustc"])
    provider_roots = [
        Path(env["TCL_TOMMATH_DIR"]).parent,
        Path(env["TCL_LSP_TCL_BIN_DIR"]),
        jim_root,
        *tcl_reference_roots,
    ]
    require(
        (provider_roots[0] / "libtommath/tommath.h").is_file(),
        "full TomMath provider absent",
    )
    sysroot = Path(
        subprocess.check_output(
            [str(tools["rustc"]), "--print", "sysroot"], text=True
        ).strip()
    )
    provider_roots.append(sysroot / "lib")
    providers_before = files(provider_roots)
    save(out / "providers-before.json", providers_before)
    for pinned_path, pinned_digest in readonly_validation["source_pins"].items():
        require(sha(root / pinned_path) == pinned_digest,
                "readonly body source pin changed: " + pinned_path)
    for pinned_path, pinned_digest in ancestry_validation["source_pins"].items():
        require(sha(root / pinned_path) == pinned_digest,
                "ancestry source pin changed: " + pinned_path)
    for pinned_path, pinned_digest in logical_validation["source_pins"].items():
        require(sha(root / pinned_path) == pinned_digest,
                "logical source pin changed: " + pinned_path)
    before = source()
    save(out / "source-before.json", before)
    require(
        all(
            sha(root / name) == pin
            for name, pin in {
                **roster["source_pins"],
                **supplemental["source_pins"],
                **sh21_diagnostic["source_pins"],
                **readflow["source_pins"],
                **catch_diagnostic["source_pins"],
                **taint_diagnostic["source_pins"],
                **closed_controls["source_pins"],
            }.items()
        ),
        "original test/observer input changed",
    )
    save(
        out / "job.json",
        {
            "utc": datetime.datetime.now(datetime.UTC).isoformat(),
            "requested_sha": args.sha,
            "tools": tool_pins,
            "tool_versions": versions,
            "job": {
                name: env.get(name)
                for name in [
                    "GITHUB_RUN_ID",
                    "GITHUB_RUN_ATTEMPT",
                    "GITHUB_JOB",
                    "GITHUB_REPOSITORY",
                    "GITHUB_WORKFLOW_REF",
                    "GITHUB_SHA",
                    "GITHUB_REF",
                ]
            },
            "environment": {
                name: env.get(name)
                for name in [
                    "CARGO_TARGET_DIR",
                    "CARGO_HOME",
                    "CARGO_INCREMENTAL",
                    "CARGO_BUILD_JOBS",
                    "CARGO_PROFILE_TEST_DEBUG",
                    "RUSTFLAGS",
                    "TCL_TOMMATH_DIR",
                    "TCL_LSP_TCLSH90",
                    "TCL_LSP_REQUIRE_TCLSH",
                    "TCL_REQUIRE_SPECTCL_COMPAT",
                    "TCL_LSP_TCLSH84",
                    "TCL_LSP_TCLSH85",
                    "TCL_LSP_TCLSH86",
                    "TCL_LSP_TCLSH91",
                    "TCL_LSP_JIMSH",
                    "TCL_LSP_JIM_ROOT",
                    "TCL_LSP_TCL_SOURCE_PARENT",
                ]
            },
            "limits": {
                "build_seconds": 2400,
                "each_original_test_seconds": 180,
                "dependency_fetch_seconds": 600,
                "aggregate_log_bytes": CAP,
                "transport_filter": "Existing exact ORIGINAL_COMMAND_INSTALLED table summaries remain. Only ORIGINAL_WORD_CONFIG and REMOTE_SOURCE_WORDS repeated complete lines use bounded lossless dictionary/RLE records; each distinct full line and ordered repeat records reconstruct exact post-table stderr, verified per case. Other stderr and all stdout remain verbatim except escaped record-marker collisions, which reconstruct exactly.",
                "transport_artifact_contract": "Owned argv0/artifact is the sealed outer Python; the pinned layered filter launches the exact selected Rust ELF argv. Each row seals that fresh Cargo-ledger ELF before/after the owned operation; existing child-status/one-test terminal, Python/filter source-tool seals and all final artifact seals remain required.",
                "execution_order": ("Two fresh compiler/registry lib builds and inventories, then exact Layout11 only; historical150 and corpus2 not requested." if args.jim_source_layout_only else "Fresh compiler-lib build/list only, then pinned text117 and generated7 corpus controls with their full original180/1800/600/40 budgets; no historical test prefix or repeat." if args.source_point_corpus_only else "Separate Native35three then SH21two diagnostic duplicates then ReadFlowone first in compilerlib, plus catch diagnostic before its unchanged original integration row; original58 relative ordering, selectors and semantic flags unchanged. After ALL original65 executions, separate taint diagnostic2 reuses the already freshly built compilerlib/taint artifacts. Three additional library artifacts are build/list-only until then; separate closed-consumer16 executes only after the unchanged taint2 group, within the same absolute deadline/log budget."),
                "disk_floor_bytes": FLOOR,
                "absolute_deadline_epoch": deadline,
            },
            "scope": ("Dedicated exact source-only Layout11 geometry/refusal controls; no old150/corpus outcome transfer or Native grant." if args.jim_source_layout_only else "Dedicated exact complete-text corpus2 only: 117 pinned original fixtures and 7 generated programs, unchanged full-report/type comparisons; historical150 not requested. No other gate admission." if args.source_point_corpus_only else "Exact original native58 selectors on current WORK; standalone bounded per-test diagnosis only. No shared-process/compiler-complete/eight-package/wasm/release acceptance. Historical ambient WASI SDK path retained in provenance, not used by selected native artifacts."),
            "original_roster": original,
            "original_inputs_and_environment": roster,
            "separate_closed_consumer_controls": closed_controls,
            "separate_readonly_body13": readonly_validation,
            "separate_source_read_ancestry3": ancestry_validation,
            "separate_logical_declared_read3": logical_validation,
            "separate_supplemental3": supplemental,
            "separate_sh21_diagnostic2": sh21_diagnostic,
            "separate_readflow1": readflow,
            "separate_catch_diagnostic1": catch_diagnostic,
            "separate_taint_diagnostic2": taint_diagnostic,
        },
    )
    print("FETCH locked native dependency inputs before build", flush=True)
    if args.jim_source_layout_only:
        require(deadline - time.time() >= 10560,
                "insufficient shared time for full Layout11 fetch/build/list/test/cleanup reservation")
    fetched = execute(
        [
            str(tools["cargo"]),
            "fetch",
            "--locked",
            "--target",
            "x86_64-unknown-linux-gnu",
        ],
        "fetch",
        tools["cargo"],
        600,
        env,
        remaining,
    )
    processes.append(fetched)
    remaining -= fetched["retained_bytes"]
    require(
        lifecycle(fetched) and fetched["exit_code"] == 0,
        "dependency fetch failed or unclosed",
    )
    cargo_home = Path(env.get("CARGO_HOME", str(Path.home() / ".cargo")))
    dependency_roots = [
        path
        for path in [
            cargo_home / "registry/src",
            cargo_home / "registry/cache",
            cargo_home / "git/checkouts",
        ]
        if path.exists()
    ]
    require(bool(dependency_roots), "resolved Cargo dependency inputs absent")
    dependencies_before = files(dependency_roots)
    save(out / "dependencies-before-build.json", dependencies_before)
    execution_rows = (
        supplemental_rows + sh21_rows + readflow_rows + catch_rows + original_rows
    )
    if args.jim_source_layout_only:
        execution_rows = jim_layout_rows
    build_targets = (jim_layout_validation["build_targets"] if args.jim_source_layout_only
                     else roster["targets"] + closed_controls["build_only_targets"])
    if args.source_point_corpus_only:
        build_targets = [
            spec for spec in roster["targets"]
            if spec["package"] == "tcl-compiler"
            and spec["kind"] == ["lib"] and spec["name"] == "tcl_compiler"
        ]
        require(len(build_targets) == 1, "dedicated corpus compiler-lib target absent/ambiguous")
    for target_spec in build_targets:
        if args.jim_source_layout_only:
            require(deadline - time.time() >= 2400 + 180 + 40,
                    "insufficient shared time for full Layout target build/list/cleanup")
        package = target_spec["package"]
        target_name = target_spec["name"]
        target_kind = target_spec["kind"]
        selected_rows = [
            row
            for row in execution_rows
            if row["package"] == package
            and row["kind"] == target_kind
            and row["target"] == target_name
        ]
        selectors = [row["selector"] for row in selected_rows]
        require(remaining > 512 * 1024, "insufficient log budget before selected build")
        print("BUILD original target " + target_name, flush=True)
        argv = [
            str(tools["cargo"]),
            "test",
            "--locked",
            "-p",
            package,
            *(["--lib"] if target_kind == ["lib"] else ["--test", target_name]),
            "--no-run",
            "--message-format=json",
            "-j",
            "2",
        ]
        built = execute(
            argv, "build-" + target_name, tools["cargo"], 2400, env, remaining
        )
        processes.append(built)
        remaining -= built["retained_bytes"]
        require(
            lifecycle(built) and built["exit_code"] == 0,
            "selected target build failed or unclosed: " + target_name,
        )
        messages = [
            json.loads(line)
            for line in (out / ("build-" + target_name + ".stdout"))
            .read_text()
            .splitlines()
            if line.startswith("{")
        ]
        require(
            not any(
                row.get("reason") == "compiler-message"
                and row.get("message", {}).get("level") in ["warning", "error"]
                for row in messages
            ),
            "selected build emitted Rust warning/error",
        )
        require(
            sum(
                row.get("reason") == "build-finished" and row.get("success") is True
                for row in messages
            )
            == 1,
            "successful Cargo terminal missing",
        )
        selected = [
            row
            for row in messages
            if row.get("reason") == "compiler-artifact"
            and row.get("target", {}).get("name") == target_name
            and row.get("target", {}).get("kind") == target_kind
            and row.get("manifest_path") == str(root / "rust" / package / "Cargo.toml")
            and row.get("profile", {}).get("test") is True
            and row.get("executable")
        ]
        require(
            len(selected) == 1
            and selected[0].get("fresh") is False
            and selected[0].get("profile", {}).get("debug_assertions") is True,
            "fresh current diagnostic test artifact missing/ambiguous",
        )
        executable = Path(selected[0]["executable"]).resolve()
        require(
            executable.is_file() and executable.is_relative_to(target),
            "test artifact outside fresh owned target",
        )
        artifact = {
            "target": target_name,
            "package": package,
            "kind": target_kind,
            "path": str(executable),
            "sha256": sha(executable),
            "cargo_event": selected[0],
            "strict_Rust_0w": True,
            "admission": "selected target compilation only",
        }
        artifacts.append(artifact)
        save(out / "selected-artifacts.json", artifacts)
        require(
            files(dependency_roots) == dependencies_before
            and source() == before
            and files(provider_roots) == providers_before
            and files(list(tools.values())) == tool_pins,
            "pre-runtime source/provider/tool/dependency drift",
        )
        list_argv = [str(executable), "--list", "--format", "terse"]
        listing = execute(
            list_argv, "list-" + target_name, executable, 180, env, remaining
        )
        processes.append(listing)
        remaining -= listing["retained_bytes"]
        require(
            lifecycle(listing) and listing["exit_code"] == 0,
            "original roster listing failed or unclosed",
        )
        listed = []
        for line in (
            (out / ("list-" + target_name + ".stdout")).read_text().splitlines()
        ):
            if line.endswith(": test"):
                listed.append(line.removesuffix(": test"))
            else:
                require(
                    not line or re.fullmatch(r"\d+ tests?, \d+ benchmarks?", line),
                    "unexpected listing data: " + line[:256],
                )
        require(
            len(listed) == len(set(listed))
            and set(selectors).issubset(listed)
            and len(selectors)
            == (target_spec["expected_selected"] if args.jim_source_layout_only else
                target_spec["expected_selected"] + sum(
                    row["target"] == target_name
                    and row["package"] == package
                    and row["kind"] == target_kind
                    for row in supplemental_rows + sh21_rows + readflow_rows + catch_rows
                )),
            "original actual roster mismatch: " + target_name,
        )
        save(
            out / ("roster-" + target_name + ".json"),
            {
                "target": target_name,
                "selected": listed,
                "listing_sha256": sha(out / ("list-" + target_name + ".stdout")),
                "executable_sha256": artifact["sha256"],
            },
        )
        if args.jim_source_layout_only:
            continue
        if args.source_point_corpus_only:
            # No historical test prefix or diagnostic repeat consumes this mode's time.
            require(all(row["state"] == "UNRUN" for row in rows),
                    "dedicated corpus unexpectedly executed historical controls")
            require(deadline - time.time() >= sum(
                180 + row["budget_seconds"] + 40 for row in source_point_corpus_rows
            ), "insufficient shared time for BOTH full corpus list/test/cleanup budgets")
            run_source_point_corpus_controls()
            require(len(source_point_corpus_rows) == 2 and all(
                row["state"] in ["PASS", "FAIL"] for row in source_point_corpus_rows
            ), "dedicated complete text corpus2 incomplete")
            require(all(row.get("report_trace_observation", {}).get("state") == "OBSERVED"
                        for row in source_point_corpus_rows),
                    "dedicated corpus report trace UNOBSERVED; no absence inference")
            status = 0 if all(row["state"] == "PASS" for row in source_point_corpus_rows) else 1
            raise _DedicatedCorpusComplete()
        for index, row in enumerate(selected_rows):
            selector = row["selector"]
            require(
                remaining > 512 * 1024, "insufficient log budget before original test"
            )
            prefix = row["group"] + "-" + target_name + "-" + str(index).zfill(2)
            group_label = (
                "supplemental"
                if row["group"] in ["supplemental-readflow1", "diagnostic-catch1"]
                else "original"
            )
            print("SELECT " + group_label + " " + selector, flush=True)
            runtime_env = env.copy()
            selected_env = row["env"]
            if not isinstance(selected_env, dict):
                raise TypeError("original per-selector environment is not a mapping")
            for name, value in selected_env.items():
                if not isinstance(name, str) or not isinstance(value, str):
                    raise TypeError("original per-selector environment is not text")
                runtime_env[name] = value
            selected_list = execute(
                [str(executable), selector, "--exact", "--list", "--format", "terse"],
                "select-" + prefix,
                executable,
                180,
                runtime_env,
                remaining,
            )
            processes.append(selected_list)
            remaining -= selected_list["retained_bytes"]
            require(
                lifecycle(selected_list) and selected_list["exit_code"] == 0,
                "original exact selector list failed or unclosed",
            )
            selected_lines = (
                (out / ("select-" + prefix + ".stdout")).read_text().splitlines()
            )
            require(
                [line for line in selected_lines if line.endswith(": test")]
                == [selector + ": test"],
                "original exact selector list mismatch",
            )
            require(
                all(
                    line == selector + ": test"
                    or not line
                    or re.fullmatch(r"1 tests?, 0 benchmarks?", line)
                    for line in selected_lines
                ),
                "unexpected original exact selector list data",
            )
            require(
                remaining > 512 * 1024,
                "insufficient log budget after original exact selector list",
            )
            child_before = sha(executable)
            require(
                child_before == artifact["sha256"],
                "selected Rust test artifact changed before transport spawn",
            )
            row["selected_test_artifact"] = {
                "path": str(executable),
                "expected_sha256": artifact["sha256"],
                "before_sha256": child_before,
                "after_sha256": None,
                "unchanged": False,
            }
            row["state"] = "RUNNING"
            save(out / "outcomes.json", rows)
            print("RUN " + group_label + " " + selector, flush=True)
            receipt = execute(
                [
                    str(executable),
                    selector,
                    "--exact",
                    "--nocapture",
                    "--test-threads=1",
                ],
                prefix,
                executable,
                180,
                runtime_env,
                remaining,
            )
            processes.append(receipt)
            remaining -= receipt["retained_bytes"]
            child_after = sha(executable)
            row["selected_test_artifact"]["after_sha256"] = child_after
            row["selected_test_artifact"]["unchanged"] = (
                child_before == child_after == artifact["sha256"]
            )
            save(out / "outcomes.json", rows)
            require(
                row["selected_test_artifact"]["unchanged"],
                "selected Rust test artifact changed during transport",
            )
            transport_path = out / (prefix + ".transport.json")
            if lifecycle(receipt):
                require(
                    transport_path.is_file(), "completed test transport summary missing"
                )
                transport = json.loads(transport_path.read_text())
                encoded_stderr = (out / (prefix + ".stderr")).read_bytes()
                repeat_stage = transport["repeat_stage"]
                table_stage = transport["table_stage"]
                require(
                    transport["child_exit_code"] == receipt["exit_code"]
                    and transport["ordinary_stderr_verbatim"] is False
                    and transport[
                        "stderr_losslessly_reconstructible_after_table_filter"
                    ]
                    is True
                    and table_stage["ordinary_stderr_verbatim"] is True
                    and table_stage["stdout_verbatim"] is True
                    and transport["stdout_verbatim"] is True
                    and transport["retained_stderr_bytes"]
                    == repeat_stage["output_bytes"]
                    == len(encoded_stderr)
                    and repeat_stage["format"]
                    == "two-family-lossless-line-dictionary-rle-v1"
                    and repeat_stage["input_bytes"]
                    == table_stage["retained_stderr_bytes"]
                    and repeat_stage["output_sha256"]
                    == hashlib.sha256(encoded_stderr).hexdigest(),
                    "transport child status or layered byte census mismatch",
                )
                reconstructed = transport_filter.decode_diagnostic_repeats(
                    encoded_stderr, lambda _data: None, repeat_stage["input_bytes"]
                )
                require(
                    reconstructed["bytes"] == repeat_stage["input_bytes"]
                    and reconstructed["sha256"] == repeat_stage["input_sha256"],
                    "transport diagnostic reconstruction differs from original stream",
                )
                row["transport_reconstruction"] = reconstructed
            row.update(
                exit_code=receipt["exit_code"],
                receipt=prefix + ".process.json",
                stdout=prefix + ".stdout",
                stderr=prefix + ".stderr",
            )
            require(
                lifecycle(receipt), "original test lifecycle/capture stop: " + selector
            )
            stdout = (out / (prefix + ".stdout")).read_text(errors="replace")
            try:
                row["state"] = original_result(stdout, receipt["exit_code"])
            except RuntimeError:
                row["state"] = (
                    "SIGNAL" if receipt["exit_code"] < 0 else "PROTOCOL_FAILURE"
                )
                save(out / "outcomes.json", rows)
                raise
            save(out / "outcomes.json", rows)
            require(
                sha(executable) == artifact["sha256"], "selected test artifact changed"
            )

    if args.jim_source_layout_only:
        require(all(row["state"] == "UNRUN" for row in rows),
                "dedicated Layout11 unexpectedly executed historical controls")
        run_jim_source_layout_controls()
        require(len(jim_layout_rows) == 11 and all(
            row["state"] in ["PASS", "FAIL"] for row in jim_layout_rows),
            "dedicated Layout11 incomplete")
        layout11_acceptance_exit = 0 if all(row["state"] == "PASS" for row in jim_layout_rows) else 1
        status = layout11_acceptance_exit
        for row in layout_jim_diagnostic_rows:
            run_layout_own_jim_diagnostic(row)
            if row["state"] == "FAIL":
                status = max(status, 1)
            elif row["state"] not in ["PASS", "FAIL"] and not row["optional"]:
                status = 2
        raise _DedicatedLayoutComplete()
    require(
        all(row["state"] in ["PASS", "FAIL"] for row in original65_rows),
        "taint diagnostics cannot precede original65 closure",
    )
    require(
        files(dependency_roots) == dependencies_before
        and source() == before
        and files(provider_roots) == providers_before
        and files(list(tools.values())) == tool_pins,
        "pre-taint diagnostic source/provider/tool/dependency drift",
    )
    for index, row in enumerate(taint_rows + closed_rows):
        package, target_kind, target_name = row["package"], row["kind"], row["target"]
        selected_artifacts = [
            value
            for value in artifacts
            if value["package"] == package
            and value["kind"] == target_kind
            and value["target"] == target_name
        ]
        require(
            len(selected_artifacts) == 1,
            "taint diagnostic current built artifact absent/ambiguous",
        )
        diagnostic_artifact: dict[str, Any] = selected_artifacts[0]
        require(
            diagnostic_artifact["strict_Rust_0w"] is True
            and diagnostic_artifact["cargo_event"]["profile"]["debug_assertions"]
            is True,
            "taint diagnostic requires existing fresh strict debug-assertion test profile",
        )
        executable = Path(diagnostic_artifact["path"])
        require(
            sha(executable) == diagnostic_artifact["sha256"],
            "taint diagnostic artifact changed before listing",
        )
        selector = row["selector"]
        require(remaining > 512 * 1024, "insufficient log budget before original test")
        prefix = row["group"] + "-" + target_name + "-" + str(index).zfill(2)
        group_label = (
            "supplemental" if row["group"] == "diagnostic-taint2" else "closed-consumer"
        )
        print("SELECT " + group_label + " " + selector, flush=True)
        runtime_env = env.copy()
        selected_env = row["env"]
        if not isinstance(selected_env, dict):
            raise TypeError("original per-selector environment is not a mapping")
        for name, value in selected_env.items():
            if not isinstance(name, str) or not isinstance(value, str):
                raise TypeError("original per-selector environment is not text")
            runtime_env[name] = value
        selected_list = execute(
            [str(executable), selector, "--exact", "--list", "--format", "terse"],
            "select-" + prefix,
            executable,
            180,
            runtime_env,
            remaining,
        )
        processes.append(selected_list)
        remaining -= selected_list["retained_bytes"]
        require(
            lifecycle(selected_list) and selected_list["exit_code"] == 0,
            "original exact selector list failed or unclosed",
        )
        selected_lines = (
            (out / ("select-" + prefix + ".stdout")).read_text().splitlines()
        )
        require(
            [line for line in selected_lines if line.endswith(": test")]
            == [selector + ": test"],
            "original exact selector list mismatch",
        )
        require(
            all(
                line == selector + ": test"
                or not line
                or re.fullmatch(r"1 tests?, 0 benchmarks?", line)
                for line in selected_lines
            ),
            "unexpected original exact selector list data",
        )
        require(
            remaining > 512 * 1024,
            "insufficient log budget after original exact selector list",
        )
        child_before = sha(executable)
        require(
            child_before == diagnostic_artifact["sha256"],
            "selected Rust test artifact changed before transport spawn",
        )
        row["selected_test_artifact"] = {
            "path": str(executable),
            "expected_sha256": diagnostic_artifact["sha256"],
            "before_sha256": child_before,
            "after_sha256": None,
            "unchanged": False,
        }
        row["state"] = "RUNNING"
        save(out / "outcomes.json", rows)
        print("RUN " + group_label + " " + selector, flush=True)
        receipt = execute(
            [
                str(executable),
                selector,
                "--exact",
                "--nocapture",
                "--test-threads=1",
            ],
            prefix,
            executable,
            180,
            runtime_env,
            remaining,
        )
        processes.append(receipt)
        remaining -= receipt["retained_bytes"]
        child_after = sha(executable)
        row["selected_test_artifact"]["after_sha256"] = child_after
        row["selected_test_artifact"]["unchanged"] = (
            child_before == child_after == diagnostic_artifact["sha256"]
        )
        save(out / "outcomes.json", rows)
        require(
            row["selected_test_artifact"]["unchanged"],
            "selected Rust test artifact changed during transport",
        )
        transport_path = out / (prefix + ".transport.json")
        if lifecycle(receipt):
            require(
                transport_path.is_file(), "completed test transport summary missing"
            )
            transport = json.loads(transport_path.read_text())
            encoded_stderr = (out / (prefix + ".stderr")).read_bytes()
            repeat_stage = transport["repeat_stage"]
            table_stage = transport["table_stage"]
            require(
                transport["child_exit_code"] == receipt["exit_code"]
                and transport["ordinary_stderr_verbatim"] is False
                and transport["stderr_losslessly_reconstructible_after_table_filter"]
                is True
                and table_stage["ordinary_stderr_verbatim"] is True
                and table_stage["stdout_verbatim"] is True
                and transport["stdout_verbatim"] is True
                and transport["retained_stderr_bytes"]
                == repeat_stage["output_bytes"]
                == len(encoded_stderr)
                and repeat_stage["format"]
                == "two-family-lossless-line-dictionary-rle-v1"
                and repeat_stage["input_bytes"] == table_stage["retained_stderr_bytes"]
                and repeat_stage["output_sha256"]
                == hashlib.sha256(encoded_stderr).hexdigest(),
                "transport child status or layered byte census mismatch",
            )
            reconstructed = transport_filter.decode_diagnostic_repeats(
                encoded_stderr, lambda _data: None, repeat_stage["input_bytes"]
            )
            require(
                reconstructed["bytes"] == repeat_stage["input_bytes"]
                and reconstructed["sha256"] == repeat_stage["input_sha256"],
                "transport diagnostic reconstruction differs from original stream",
            )
            row["transport_reconstruction"] = reconstructed
        row.update(
            exit_code=receipt["exit_code"],
            receipt=prefix + ".process.json",
            stdout=prefix + ".stdout",
            stderr=prefix + ".stderr",
        )
        require(lifecycle(receipt), "original test lifecycle/capture stop: " + selector)
        stdout = (out / (prefix + ".stdout")).read_text(errors="replace")
        try:
            row["state"] = original_result(stdout, receipt["exit_code"])
        except RuntimeError:
            row["state"] = "SIGNAL" if receipt["exit_code"] < 0 else "PROTOCOL_FAILURE"
            save(out / "outcomes.json", rows)
            raise
        save(out / "outcomes.json", rows)
        require(
            sha(executable) == diagnostic_artifact["sha256"],
            "selected test artifact changed",
        )
    run_readonly_body_controls()
    run_source_read_ancestry_controls()
    run_logical_declared_read_controls()
    require(
        len(original_rows) == 58
        and len(supplemental_rows) == 3
        and len(sh21_rows) == 2
        and len(readflow_rows) == 1
        and len(catch_rows) == 1
        and len(original65_rows) == 65
        and len(taint_rows) == 2
        and len(closed_rows) == 16
        and len(readonly_rows) == 13
        and len(ancestry_rows) == 3
        and len(logical_rows) == 3
        and len(rows) == 102
        and all(row["state"] in ["PASS", "FAIL"] for row in rows),
        "original test inventory incomplete",
    )
    run_logical_body_entry_controls()
    require(len(logical_body_rows) == 5 and all(row["state"] in ["PASS", "FAIL"] for row in logical_body_rows),
        "logical body-entry controls incomplete")
    run_compact_codec_control()
    require(len(codec_rows) == 1 and all(row["state"] in ["PASS", "FAIL"] for row in codec_rows),
        "compact codec control incomplete")
    run_original_child_controls()
    require(len(named_child_rows) == 10 and all(row["state"] in ["PASS", "FAIL"] for row in named_child_rows),
        "original child controls incomplete")
    run_restored_child_owner_control()
    require(len(restored_child_rows) == 1 and all(row["state"] in ["PASS", "FAIL"] for row in restored_child_rows),
        "restored child owner control incomplete")
    status = 2
    run_native_string_effect_controls()
    require(len(native_string_rows) == 3 and all(row["state"] in ["PASS", "FAIL"] for row in native_string_rows),
        "native string effects controls incomplete")
    status = 2
    run_native_formal_layout_controls()
    require(len(native_formal_rows) == 2 and all(row["state"] in ["PASS", "FAIL"] for row in native_formal_rows), "native formal layout controls incomplete")
    status = 2
    run_analyser_layout_controls()
    require(len(analyser_layout_rows) == 3 and all(row["state"] in ["PASS", "FAIL"] for row in analyser_layout_rows), "analyser layout controls incomplete")
    status = 2
    run_own_jim_member_controls()
    require(len(own_jim_member_rows) == 3 and all(row["state"] in ["PASS", "FAIL"] for row in own_jim_member_rows), "own Jim member controls incomplete")
    status = 2
    run_advice_catalogue_controls()
    require(len(advice_catalogue_rows) == 3 and all(row["state"] in ["PASS", "FAIL"] for row in advice_catalogue_rows), "advice catalogue controls incomplete")
    status = 2
    run_native_eligibility_controls()
    require(len(native_eligibility_rows) == 2 and all(row["state"] in ["PASS", "FAIL"] for row in native_eligibility_rows), "Native eligibility controls incomplete")
    status = 2
    run_own_jim_alias_diagnostic()
    require(len(own_jim_alias_rows) == 1 and all(row["state"] in ["PASS", "FAIL"] for row in own_jim_alias_rows), "alias diagnostic repeat incomplete")
    status = 2
    run_source_point_memo_controls()
    require(len(source_point_memo_rows) == 8 and all(row["state"] in ["PASS", "FAIL"] for row in source_point_memo_rows), "memo8 incomplete")
    status = 2
    run_analyser_layout_diagnostics()
    require(len(analyser_layout_diagnostic_rows) == 3 and all(row["state"] in ["PASS", "FAIL"] for row in analyser_layout_diagnostic_rows), "layout diagnostic3 incomplete")
    run_source_point_report_diagnostic()
    require(len(source_point_report_rows) == 1 and all(row["state"] in ["PASS", "FAIL"] for row in source_point_report_rows), "report repeat incomplete")
    run_inherited_point_report_controls()
    require(len(inherited_report_rows) == 3 and all(row["state"] in ["PASS", "FAIL"] for row in inherited_report_rows), "inherited point report custody3 incomplete")
    run_source_point_corpus_controls()
    require(len(source_point_corpus_rows) == 2 and all(row["state"] in ["PASS", "FAIL"] for row in source_point_corpus_rows), "complete text corpus2 incomplete")
    status = 0 if all(row["state"] == "PASS" for row in rows) else 1
    if any(row["state"] != "PASS" for row in source_point_report_rows):
        status = 1
    if any(row["state"] != "PASS" for row in inherited_report_rows):
        status = 1
    if any(row["state"] != "PASS" for row in source_point_corpus_rows):
        status = 1
    if any(row["state"] != "PASS" for row in source_point_memo_rows + analyser_layout_diagnostic_rows):
        status = 1
    if any(row["state"] != "PASS" for row in own_jim_alias_rows):
        status = 1
    if any(row["state"] != "PASS" for row in native_eligibility_rows):
        status = 1
    if any(row["state"] != "PASS" for row in advice_catalogue_rows):
        status = 1
    if any(row["state"] != "PASS" for row in own_jim_member_rows):
        status = 1
    if any(row["state"] != "PASS" for row in analyser_layout_rows):
        status = 1
    if any(row["state"] != "PASS" for row in native_formal_rows):
        status = 1
    if any(row["state"] != "PASS" for row in native_string_rows):
        status = 1
    if any(row["state"] != "PASS" for row in restored_child_rows):
        status = 1
    if any(row["state"] != "PASS" for row in named_child_rows):
        status = 1
    if any(row["state"] != "PASS" for row in codec_rows):
        status = 1
    if any(row["state"] != "PASS" for row in logical_body_rows):
        status = 1
    if any(row["alias_trace_observation"] != "OBSERVED" for row in own_jim_alias_rows):
        status = 2
    if any(row["layout_trace_observation"] != "OBSERVED" for row in analyser_layout_diagnostic_rows):
        status = 2
    if any(row["capture_observation"] != "OBSERVED" for row in source_point_report_rows):
        status = 2

except (_DedicatedCorpusComplete, _DedicatedLayoutComplete):
    pass
except (
    RuntimeError,
    OSError,
    ValueError,
    subprocess.SubprocessError,
    ImportError,
    KeyError,
    TypeError,
    AssertionError,
    KeyboardInterrupt,
) as failure:
    error = repr(failure)
    print("COMPILER ORIGINAL58 DIAGNOSTIC STOP " + error, file=sys.stderr, flush=True)
finally:
    seals = {}
    for name, capture, original in [
        ("source", source, before),
        ("providers", lambda: files(provider_roots), providers_before),
        ("tools", lambda: files(list(tools.values())), tool_pins),
        ("dependencies", lambda: files(dependency_roots), dependencies_before),
    ]:
        if name == "dependencies" and original is None:
            seals[name] = False
            final_errors.append("dependencies: initial capture not reached")
            continue
        try:
            current = capture()
            save(out / (name + "-after.json"), current)
            seals[name] = original is not None and current == original
        except (
            RuntimeError,
            OSError,
            ValueError,
            subprocess.SubprocessError,
            NameError,
            KeyError,
            TypeError,
        ) as failure:
            seals[name] = False
            final_errors.append(name + ": " + repr(failure))
    seals["artifacts"] = bool(artifacts)
    for artifact in artifacts:
        artifact_path = artifact["path"]
        if not isinstance(artifact_path, str):
            raise TypeError("selected artifact path is not text")
        seals["artifacts"] &= (
            Path(artifact_path).is_file() and sha(artifact_path) == artifact["sha256"]
        )
    if not all(seals.values()):
        status = 2
    save(out / "logical-body-entry-outcomes.json", logical_body_rows)
    save(out / "compact-codec-outcomes.json", codec_rows)
    save(out / "original-child-outcomes.json", named_child_rows)
    save(out / "restored-child-owner-outcomes.json", restored_child_rows)
    save(out / "native-string-effects-outcomes.json", native_string_rows)
    save(out / "native-formal-layout-outcomes.json", native_formal_rows)
    save(out / "analyser-layout-outcomes.json", analyser_layout_rows)
    save(out / "own-jim-member-outcomes.json", own_jim_member_rows)
    save(out / "advice-catalogue-outcomes.json", advice_catalogue_rows)
    save(out / "native-eligibility-outcomes.json", native_eligibility_rows)
    save(out / "own-jim-alias-firstguard-outcomes.json", own_jim_alias_rows)
    save(out / "source-point-memo-outcomes.json", source_point_memo_rows)
    save(out / "source-point-report-firstguard-outcomes.json", source_point_report_rows)
    save(out / "inherited-point-report-outcomes.json", inherited_report_rows)
    save(out / "source-point-corpus-outcomes.json", source_point_corpus_rows)
    save(out / "analyser-layout-firstguard-outcomes.json", analyser_layout_diagnostic_rows)
    save(out / "outcomes.json", rows)
    save(
        out / "final.json",
        {
            "mode": "jim-source-layout11-diagnostics",
            "finished_utc": datetime.datetime.now(datetime.UTC).isoformat(),
            "requested_sha": args.sha,
            "overall_exit": status,
            "error": error,
            "final_errors": final_errors,
            "seals": seals,
            "artifacts": artifacts,
            "processes": processes,
            "remaining_log_bytes": remaining,
            "required_layout_tests": 11,
            "jim_source_layout11_outcomes": jim_layout_rows,
            "jim_source_layout11_roster": jim_layout_validation,
            "layout11_acceptance_exit": layout11_acceptance_exit,
            "separate_own_jim_diagnostic_rows": layout_jim_diagnostic_rows,
            "required_alias_diagnostic_count": 1,
            "optional_positive_diagnostic_count": 1,
            "optional_positive_not_guaranteed_by_12000_envelope": True,
            "historical150_requested": False,
            "corpus2_requested": False,
            "historical_outcome_transfer": False,
            "diagnostic_only": True,
            "full_shared_process_gate_admission": False,
            "shipping_performance_admission": False,
        } if args.jim_source_layout_only else {
            "mode": "source-point-corpus2-diagnostics",
            "finished_utc": datetime.datetime.now(datetime.UTC).isoformat(),
            "requested_sha": args.sha,
            "overall_exit": status,
            "error": error,
            "final_errors": final_errors,
            "seals": seals,
            "artifacts": artifacts,
            "processes": processes,
            "remaining_log_bytes": remaining,
            "required_complete_text_corpus_tests": 2,
            "source_point_complete_text_corpus2_outcomes": source_point_corpus_rows,
            "source_point_complete_text_corpus2_roster": source_point_corpus_validation,
            "historical150_requested": False,
            "historical150_outcome_transfer": False,
            "diagnostic_only": True,
            "full_shared_process_gate_admission": False,
            "shipping_performance_admission": False,
        } if args.source_point_corpus_only else {
            "finished_utc": datetime.datetime.now(datetime.UTC).isoformat(),
            "overall_exit": status,
            "error": error,
            "final_errors": final_errors,
            "seals": seals,
            "artifacts": artifacts,
            "processes": processes,
            "remaining_log_bytes": remaining,
            "outcomes": rows,
            "required_original_tests": 58,
            "required_separate_supplemental_tests": 3,
            "original_tests_closed": sum(
                row["state"] in ["PASS", "FAIL"] for row in original_rows
            ),
            "supplemental_tests_closed": sum(
                row["state"] in ["PASS", "FAIL"] for row in supplemental_rows
            ),
            "original58_outcomes": original_rows,
            "supplemental3_outcomes": supplemental_rows,
            "required_separate_sh21_diagnostic_tests": 2,
            "sh21_diagnostic_tests_closed": sum(
                row["state"] in ["PASS", "FAIL"] for row in sh21_rows
            ),
            "sh21_diagnostic2_outcomes": sh21_rows,
            "required_separate_readflow_tests": 1,
            "readflow_tests_closed": sum(
                row["state"] in ["PASS", "FAIL"] for row in readflow_rows
            ),
            "readflow1_outcomes": readflow_rows,
            "required_separate_catch_diagnostic_tests": 1,
            "catch_diagnostic_tests_closed": sum(
                row["state"] in ["PASS", "FAIL"] for row in catch_rows
            ),
            "catch_diagnostic1_outcomes": catch_rows,
            "required_separate_taint_diagnostic_tests": 2,
            "taint_diagnostic_tests_closed": sum(
                row["state"] in ["PASS", "FAIL"] for row in taint_rows
            ),
            "taint_diagnostic2_outcomes": taint_rows,
            "required_separate_closed_consumer_controls": 16,
            "closed_consumer_controls_closed": sum(row["state"] in ["PASS", "FAIL"] for row in closed_rows),
            "closed_consumer16_outcomes": closed_rows,
            "required_separate_readonly_body_controls": 13,
            "readonly_body_controls_closed": sum(
                row["state"] in ["PASS", "FAIL"] for row in readonly_rows
            ),
            "readonly_inlining10_outcomes": readonly_rows[:10],
            "readonly_owner3_outcomes": readonly_rows[10:],
            "separate_readonly_body_roster": readonly_validation,
            "required_separate_ancestry_controls": 3,
            "ancestry_controls_closed": sum(
                row["state"] in ["PASS", "FAIL"] for row in ancestry_rows
            ),
            "ancestry3_outcomes": ancestry_rows,
            "separate_ancestry_roster": ancestry_validation,
            "required_separate_logical_read_controls": 3,
            "logical_read_controls_closed": sum(
                row["state"] in ["PASS", "FAIL"] for row in logical_rows
            ),
            "logical3_outcomes": logical_rows,
            "separate_logical_roster": logical_validation,
            "required_separate_native_eligibility_controls": 2,
            "native_eligibility2_outcomes": native_eligibility_rows,
            "approved_cost_coverage_mapping": readonly_validation["approved_cost_coverage_mapping"],
            "required_separate_own_jim_alias_diagnostics": 1,
            "own_jim_alias_firstguard1_outcomes": own_jim_alias_rows,
            "own_jim_alias_firstguard1_roster": own_jim_alias_validation,
            "required_separate_source_point_report_diagnostics": 1,
            "source_point_report_firstguard1_outcomes": source_point_report_rows,
            "source_point_report_firstguard1_source_pins": source_point_report_source_pins,
            "required_inherited_point_report_custody_controls": 3,
            "inherited_point_report_custody3_outcomes": inherited_report_rows,
            "inherited_point_report_custody3_roster": inherited_report_validation,
            "required_separate_complete_text_corpus_tests": 2,
            "source_point_complete_text_corpus2_outcomes": source_point_corpus_rows,
            "source_point_complete_text_corpus2_roster": source_point_corpus_validation,
            "required_separate_source_point_memo_controls": 8,
            "source_point_memo8_outcomes": source_point_memo_rows,
            "source_point_memo8_roster": source_point_memo_validation,
            "required_separate_analyser_layout_diagnostics": 3,
            "analyser_layout_firstguard3_outcomes": analyser_layout_diagnostic_rows,
            "analyser_layout_firstguard3_roster": analyser_layout_diagnostic_validation,
            "required_separate_advice_catalogue_controls": 3,
            "advice_catalogue_controls_closed": sum(row["state"] in ["PASS", "FAIL"] for row in advice_catalogue_rows),
            "advice_catalogue3_outcomes": advice_catalogue_rows,
            "advice_catalogue3_roster": advice_catalogue_validation,
            "required_separate_own_jim_member_controls": 3,
            "own_jim_member_controls_closed": sum(row["state"] in ["PASS", "FAIL"] for row in own_jim_member_rows),
            "own_jim_member3_outcomes": own_jim_member_rows,
            "own_jim_member3_roster": own_jim_member_validation,
            "required_separate_analyser_layout_controls": 3,
            "analyser_layout_controls_closed": sum(row["state"] in ["PASS", "FAIL"] for row in analyser_layout_rows),
            "analyser_layout3_outcomes": analyser_layout_rows,
            "analyser_layout3_roster": analyser_layout_validation,
            "required_separate_native_formal_layout_controls": 2,
            "native_formal_layout_controls_closed": sum(row["state"] in ["PASS", "FAIL"] for row in native_formal_rows),
            "native_formal_layout2_outcomes": native_formal_rows,
            "native_formal_layout2_roster": native_formal_validation,
            "required_separate_native_string_effect_controls": 3,
            "native_string_effect_controls_closed": sum(row["state"] in ["PASS", "FAIL"] for row in native_string_rows),
            "native_string_effect3_outcomes": native_string_rows,
            "native_string_effect3_roster": native_string_validation,
            "required_separate_restored_child_owner_controls": 1,
            "restored_child_owner_control_closed": sum(row["state"] in ["PASS", "FAIL"] for row in restored_child_rows),
            "restored_child_owner1_outcomes": restored_child_rows,
            "restored_child_owner1_roster": restored_child_validation,
            "required_separate_original_child_controls": 10,
            "original_child_controls_closed": sum(row["state"] in ["PASS", "FAIL"] for row in named_child_rows),
            "original_child10_outcomes": named_child_rows,
            "original_child10_roster": named_child_validation,
            "required_separate_compact_codec_controls": 1,
            "compact_codec_control_closed": sum(row["state"] in ["PASS", "FAIL"] for row in codec_rows),
            "compact_codec1_outcomes": codec_rows,
            "compact_codec1_roster": codec_validation,
            "required_separate_logical_body_entry_controls": 5,
            "logical_body_entry_controls_closed": sum(row["state"] in ["PASS", "FAIL"] for row in logical_body_rows),
            "logical_body_entry5_outcomes": logical_body_rows,
            "logical_body_entry5_roster": logical_body_validation,
            "logical_body_entry5_qualification": "Separately labelled controls after unchanged original102, same existing fresh compiler-lib ELF, no extra build/budget reset or Native permission.",
            "separate_closed_consumer_roster": closed_controls,
            "diagnostic_only": True,
            "full_shared_process_gate_admission": False,
            "eight_package_typing_admission": False,
            "shipping_performance_admission": False,
        },
    )
sys.exit(status)
