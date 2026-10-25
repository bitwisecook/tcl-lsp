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

"""WORK-only original SpecTcl10 and separate native catch diagnostic."""

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
        ("pack_source_e2e-", "cmd_control_e2e-", "native_normal_store-", "spec_corpus-", "list-")
    ):
        require(
            available >= seconds,
            "insufficient overall time for original full test/list budget",
        )
    seconds = min(seconds, available)
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


parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--sha", required=True)
parser.add_argument("--out", type=Path, required=True)
args = parser.parse_args()
require(re.fullmatch("[0-9a-f]{40}", args.sha), "full exact SHA required")
require(os.environ.get("GITHUB_SHA") == args.sha, "checkout/workflow SHA mismatch")
require(os.environ.get("GITHUB_REF") == "refs/heads/work", "WORK only")
root = Path.cwd().resolve()
out = args.out.resolve()
target = Path(os.environ["CARGO_TARGET_DIR"]).resolve()
deadline = float(os.environ["DIAGNOSTIC_DEADLINE_EPOCH"])
require(
    time.time() < deadline <= time.time() + 3600,
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
roster = json.loads((BASE / "spectcl-fold-roster.json").read_text())
require(
    len(roster["spectcl"]) == len(set(roster["spectcl"])) == 10,
    "original SpecTcl10 roster",
)
require(
    roster["catch"] == ["a_catch_body_write_is_visible_after_the_catch"],
    "original catch1 roster",
)
require(
    roster["native_parent_store"]
    == ["native_parent_store_uses_the_actual_pack_implementation_normal_outcome"],
    "separate Native parent store1 roster",
)
prioritised = [
    "a_pack_file_folds_a_call_site_in_the_optimiser",
    "a_reference_body_answers_through_the_host_as_the_procedure_does",
    "a_derived_body_runs_under_the_release_the_call_is_analysed_under",
]
order = prioritised + [name for name in roster["spectcl"] if name not in prioritised]
rows = [
    {"target": "pack_source_e2e", "selector": name, "state": "UNRUN"} for name in order
] + [{"target": "cmd_control_e2e", "selector": roster["catch"][0], "state": "UNRUN"}]
original_rows = rows
supplemental_rows = [
    {
        "group": "supplemental_native_parent_store",
        "target": "native_normal_store",
        "selector": roster["native_parent_store"][0],
        "state": "UNRUN",
    }
]
hostile_rows = [{"group": "separate_hostile_containment", "target": "spec_corpus", "selector": roster["hostile_containment"][0], "state": "UNRUN"}]
require(roster["hostile_containment"] == ["a_hostile_pack_degrades_to_abstention_and_never_hangs"], "unchanged original hostile containment1")
rows = original_rows + supplemental_rows + hostile_rows
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
    spec = importlib.util.spec_from_file_location("spectcl_owner", executor)
    if spec is None or spec.loader is None:
        raise RuntimeError("process owner loader absent")
    owner = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(owner)
    rustup = shutil.which("rustup")
    if rustup is None:
        raise RuntimeError("rustup absent")
    tools = {"python": Path(sys.executable).resolve(), "executor": executor}
    for name in ["cargo", "rustc"]:
        tools[name] = Path(
            subprocess.check_output([rustup, "which", name], text=True).strip()
        ).resolve()
    for name in ["cc", "ar", "ld"]:
        path = shutil.which(name)
        if path is None:
            raise RuntimeError("native tool absent: " + name)
        tools[name] = Path(path).resolve()
    tools["tclsh90"] = Path(os.environ["TCL_LSP_TCLSH90"]).resolve()
    require(tools["tclsh90"].is_file(), "existing CI Tcl9.0 reference absent")
    tool_pins = files(list(tools.values()))
    versions = {
        name: subprocess.check_output([str(path), "--version"], text=True)
        for name, path in tools.items()
        if name not in ["executor", "tclsh90"]
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
        env.get("CARGO_PROFILE_TEST_DEBUG") == "0", "original test debug-info setting"
    )
    require(
        env.get("TCL_LSP_NATIVE_ADMISSION_DIAGNOSTIC") == "first-decline-v1",
        "original native diagnostic flag",
    )
    require(
        env.get("TCL_LSP_TRACE_N17_FOLD") == "first-gate-v1",
        "original N17 diagnostic flag",
    )
    require(
        roster.get("diagnostic_flags") == {"TCL_LSP_TRACE_DEFERRED_WRITES_ANY": "1"},
        "exact existing deferred-write producer diagnostic flag",
    )
    env["TCL_LSP_TRACE_DEFERRED_WRITES_ANY"] = "1"
    env["RUSTC"] = str(tools["rustc"])
    provider_roots = [
        Path(env["TCL_TOMMATH_DIR"]).parent,
        Path(env["TCL_LSP_TCL_BIN_DIR"]),
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
    before = source()
    save(out / "source-before.json", before)
    require(
        all(sha(root / name) == pin for name, pin in roster["source_pins"].items()),
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
                    "TCL_LSP_NATIVE_ADMISSION_DIAGNOSTIC",
                    "TCL_LSP_TRACE_N17_FOLD",
                    "TCL_LSP_TRACE_DEFERRED_WRITES_ANY",
                ]
            },
            "limits": {
                "build_seconds": 2400,
                "each_original_test_seconds": 180,
                "dependency_fetch_seconds": 600,
                "aggregate_log_bytes": CAP,
                "disk_floor_bytes": FLOOR,
                "absolute_deadline_epoch": deadline,
            },
            "scope": "Original pack_source_e2e ALL10 and separate original catch1 remain original11; new native_normal_store1 is a distinct supplemental group, standalone bounded diagnosis only. Full shared-process suite, eight-package typing, all-release provider and release gates remain separate.",
            "original_roster": roster,
        },
    )
    print("FETCH locked native dependency inputs before build", flush=True)
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
    targets = [
        ("tcl-spectcl", "pack_source_e2e", order),
        ("tcl-vm", "cmd_control_e2e", roster["catch"]),
        ("tcl-spectcl", "native_normal_store", roster["native_parent_store"]),
        ("tcl-spectcl", "spec_corpus", roster["hostile_containment"]),
    ]
    for package, target_name, selectors in targets:
        require(remaining > 512 * 1024, "insufficient log budget before selected build")
        target_group = (
            "separate_hostile" if target_name == "spec_corpus" else ("supplemental" if target_name == "native_normal_store" else "original")
        )
        print("BUILD " + target_group + " target " + target_name, flush=True)
        argv = [
            str(tools["cargo"]),
            "test",
            "--locked",
            "-p",
            package,
            "--test",
            target_name,
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
            and row.get("target", {}).get("kind") == ["test"]
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
        if target_name in ["cmd_control_e2e", "spec_corpus"]:
            list_argv += ["--exact", selectors[0]]
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
            len(listed) == len(set(listed)) == len(selectors)
            and set(listed) == set(selectors),
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
        for index, selector in enumerate(selectors):
            require(
                remaining > 512 * 1024, "insufficient log budget before original test"
            )
            row = next(
                row
                for row in rows
                if row["target"] == target_name and row["selector"] == selector
            )
            row["state"] = "RUNNING"
            save(out / "outcomes.json", rows)
            prefix = target_name + "-" + str(index).zfill(2)
            print("RUN " + target_group + " " + selector, flush=True)
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
                env,
                remaining,
            )
            processes.append(receipt)
            remaining -= receipt["retained_bytes"]
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
    require(
        len(original_rows) == 11
        and len(supplemental_rows) == 1
        and len(hostile_rows) == 1
        and all(row["state"] in ["PASS", "FAIL"] for row in rows),
        "original or separate supplemental test inventory incomplete",
    )
    status = 0 if all(row["state"] == "PASS" for row in rows) else 1
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
    print("SPECTCL DIAGNOSTIC STOP " + error, file=sys.stderr, flush=True)
finally:
    seals = {}
    for name, capture, original in [
        ("source", source, before),
        ("providers", lambda: files(provider_roots), providers_before),
        ("tools", lambda: files(list(tools.values())), tool_pins),
        ("dependencies", lambda: files(dependency_roots), dependencies_before),
    ]:
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
    save(out / "outcomes.json", rows)
    save(
        out / "final.json",
        {
            "finished_utc": datetime.datetime.now(datetime.UTC).isoformat(),
            "overall_exit": status,
            "error": error,
            "final_errors": final_errors,
            "seals": seals,
            "artifacts": artifacts,
            "processes": processes,
            "remaining_log_bytes": remaining,
            "outcomes": rows,
            "required_original_tests": 11,
            "original_tests_closed": sum(
                row["state"] in ["PASS", "FAIL"] for row in original_rows
            ),
            "original_outcomes": original_rows,
            "required_supplemental_tests": 1,
            "supplemental_tests_closed": sum(
                row["state"] in ["PASS", "FAIL"] for row in supplemental_rows
            ),
            "supplemental_outcomes": supplemental_rows,
            "required_separate_hostile_tests": 1,
            "separate_hostile_outcomes": hostile_rows,
            "diagnostic_only": True,
            "full_shared_process_gate_admission": False,
            "eight_package_typing_admission": False,
            "shipping_performance_admission": False,
        },
    )
sys.exit(status)
