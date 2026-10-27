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

"""WORK-only isolated original WASM real-link25 diagnostic; not shared-process admission."""

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
ROSTER_SHA = "617f9e4dc557c0766156e1b7e83ded9f16c044aa83babb562cfbbc8fbc27d522"


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


def providers():
    directories = {}
    resolved_roots = [path.resolve() for path in provider_roots]
    for base in provider_roots:
        for path in [base, *sorted(base.rglob("*"))]:
            if not path.is_dir():
                continue
            resolved = path.resolve()
            require(
                any(resolved.is_relative_to(root) for root in resolved_roots),
                "unsealed provider directory target: " + str(path),
            )
            directories[str(path)] = {
                "resolved": str(resolved),
                "symlink": os.readlink(path) if path.is_symlink() else None,
                "children": sorted(child.name for child in path.iterdir()),
            }
    return {"files": files(provider_roots), "directories": directories}


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
    if prefix.startswith(("wasm_real_link-", "list-")):
        require(
            available >= seconds,
            "insufficient overall time for original full test/list budget",
        )
    if prefix == "list-wasm_tiers" or prefix.startswith("wasm_tiers-"):
        require(available >= seconds + 25,
                "insufficient original absolute time for broad8 full budget and cleanup")
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


def run_broad_wasm8(messages, environment, budget):
    global remaining
    require(len(original_rows) == 25
            and all(row["state"] in ["PASS", "FAIL"] for row in original_rows),
            "original25 must close before separate broad8")
    require(budget > 512 * 1024, "insufficient shared log cap before broad8")
    selected = [message for message in messages
                if message.get("reason") == "compiler-artifact"
                and message.get("target", {}).get("name") == "wasm_tiers"
                and message.get("target", {}).get("kind") == ["test"]
                and message.get("manifest_path") == str(root / "rust/tcl-compiler/Cargo.toml")
                and message.get("profile", {}).get("test") is True
                and message.get("executable")]
    require(len(selected) == 1 and selected[0].get("fresh") is False
            and selected[0].get("profile", {}).get("debug_assertions") is True,
            "fresh unique actual wasm_tiers artifact required")
    executable = Path(selected[0]["executable"]).resolve()
    require(executable.is_file() and executable.is_relative_to(target)
            and all(str(executable) != artifact["path"] for artifact in artifacts),
            "broad8 must own a distinct ELF inside the same fresh target")
    with executable.open("rb") as stream:
        require(stream.read(4) == b"\x7fELF", "broad8 selected artifact is not ELF")
    artifact = {"target": "wasm_tiers", "path": str(executable),
                "sha256": sha(executable), "cargo_event": selected[0],
                "strict_Rust_0w": True, "admission": "separate broad8 compilation only"}
    artifacts.append(artifact)
    save(out / "selected-artifacts.json", artifacts)
    require(files(dependency_roots) == dependencies_before and source() == before
            and providers() == providers_before and files(list(tools.values())) == tool_pins,
            "broad8 pre-runtime source/provider/tool/dependency drift")
    require(deadline - time.time() >= 205,
            "broad8 listing full180 plus25 cleanup reserve unavailable")
    listed = execute([str(executable), "--list", "--format", "terse"],
                     "list-wasm_tiers", executable, 180, environment, budget)
    processes.append(listed)
    budget -= listed["retained_bytes"]
    remaining = budget
    require(lifecycle(listed) and listed["exit_code"] == 0,
            "broad8 listing lifecycle failed")
    names = []
    for line in (out / "list-wasm_tiers.stdout").read_text().splitlines():
        if line.endswith(": test"):
            names.append(line.removesuffix(": test"))
        else:
            require(not line or re.fullmatch(r"\d+ tests?, \d+ benchmarks?", line),
                    "unexpected broad8 listing data")
    require(len(names) == len(set(names)) == 8 and set(names) == set(broad_order),
            "actual broad8 listing differs from approved exact8")
    save(out / "roster-wasm_tiers.json", {"target": "wasm_tiers", "selected": names,
         "listing_sha256": sha(out / "list-wasm_tiers.stdout"),
         "executable_sha256": artifact["sha256"]})
    require(deadline - time.time() >= 1225,
            "full broad8 1200-second child plus25 cleanup reserve unavailable")
    require(budget > 512 * 1024 and sha(executable) == artifact["sha256"],
            "broad8 remaining shared cap or ELF guard failed")
    require(runtime_identity is not None and runtime.is_file()
            and sha(runtime) == runtime_identity["sha256"],
            "original25 reserved runtime changed before broad8")
    broad_summary.update(state="RUNNING", artifact=artifact,
                         reserved_runtime_before=runtime_identity)
    for row in broad_rows:
        row["state"] = "RUNNING"
    save(out / "broad-wasm8-outcomes.json", broad_rows)
    receipt = execute([str(executable), "--nocapture", "--test-threads=1"],
                      "wasm_tiers-all8", executable, 1200, environment, budget)
    processes.append(receipt)
    budget -= receipt["retained_bytes"]
    remaining = budget
    for row in broad_rows:
        row.update(state="INCOMPLETE", receipt="wasm_tiers-all8.process.json",
                   stdout="wasm_tiers-all8.stdout", stderr="wasm_tiers-all8.stderr",
                   exit_code=receipt["exit_code"])
    require(lifecycle(receipt), "broad8 owned lifecycle/capture stop")
    aggregate = broad8_terminal((out / "wasm_tiers-all8.stdout").read_text(errors="replace"),
                                receipt["exit_code"])
    require(sha(executable) == artifact["sha256"] and runtime.is_file()
            and sha(runtime) == runtime_identity["sha256"],
            "broad8 ELF or original reserved runtime changed")
    for row in broad_rows:
        row["state"] = ("PASS" if aggregate["passed"] == 8 else "FAIL"
                        if aggregate["failed"] == 8 else "UNRESOLVED_PASS_OR_FAIL")
    broad_summary.update(state="PASS" if aggregate["failed"] == 0 else "FAIL",
                         aggregate=aggregate, process=receipt,
                         reserved_runtime_after=runtime_identity,
                         individual_results=("ALL8_SAME" if aggregate["passed"] in [0, 8]
                                             else "MIXED_IDENTITIES_UNOBSERVED"))
    save(out / "broad-wasm8-outcomes.json", broad_rows)
    save(out / "broad-wasm8-summary.json", broad_summary)
    return budget


def broad8_terminal(text, exit_code):
    require(len(re.findall(r"^running 8 tests$", text, re.MULTILINE)) == 1,
            "broad8 must run the full exact8 target")
    found = re.findall(
        r"^test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored; "
        r"(\d+) measured; (\d+) filtered out;", text, re.MULTILINE)
    require(len(found) == 1, "broad8 unique structured aggregate terminal required")
    label, passed, failed, ignored, measured, filtered = found[0]
    passed, failed = int(passed), int(failed)
    require(passed + failed == 8 and ignored == measured == filtered == "0",
            "broad8 ignored/filtered/incomplete target is not acceptance")
    require((label == "ok" and passed == 8 and exit_code == 0)
            or (label == "FAILED" and failed > 0 and exit_code == 101),
            "broad8 terminal/code disagree")
    return {"passed": passed, "failed": failed, "ignored": 0,
            "measured": 0, "filtered": 0, "attempted": 8}


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
roster_path = BASE / "wasm-real-link-roster.json"
require(sha(roster_path) == ROSTER_SHA, "approved exact original25 roster changed")
roster = json.loads(roster_path.read_text())
order = roster["wasm_real_link"]
require(len(order) == len(set(order)) == 25, "original WASM25 roster")
rows = [
    {"target": "wasm_real_link", "selector": name, "state": "UNRUN"} for name in order
]
original_rows = rows
broad_order = roster["wasm_tiers"]
require(len(broad_order) == len(set(broad_order)) == 8, "approved broad WASM8 roster")
broad_rows = [{"target": "wasm_tiers", "selector": name, "state": "UNRUN"}
              for name in broad_order]
broad_summary = {"state": "UNRUN", "required": 8, "child_budget_seconds": 1200,
                 "cleanup_reserve_seconds": 25, "same_compile_invocation_required": True}
reserved_target = root / "target/tcl_reserved_runtime"
runtime = reserved_target / "wasm32-wasip1/debug/tcl_runtime.wasm"
runtime_identity = None
runtime_observations = []
require(not reserved_target.exists(), "fresh reserved WASM runtime target required")
remaining = CAP
before = providers_before = dependencies_before = tool_pins = None
artifacts = []
processes = []
error = None
final_errors = []
status = 2
save(out / "outcomes.json", rows)
save(out / "broad-wasm8-outcomes.json", broad_rows)
save(out / "broad-wasm8-summary.json", broad_summary)
try:
    executor = BASE / "runtime-diagnostics-exec.py"
    require(sha(executor) == EXECUTOR_SHA, "reviewed process owner changed")
    spec = importlib.util.spec_from_file_location("wasm25_owner", executor)
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
    for name in ["cc", "ar", "ld", "wasmtime", "cargo", "rustc", "rustup"]:
        path = shutil.which(name)
        if path is None:
            raise RuntimeError("native tool absent: " + name)
        tools[name + "_dispatch" if name in ["cargo", "rustc"] else name] = Path(
            path
        ).resolve()
    sdk = Path("/opt/wasi-sdk")
    for name in ["clang", "llvm-ar", "wasm-ld"]:
        tools["wasi_" + name] = sdk / "bin" / name
        require(tools["wasi_" + name].is_file(), "pinned WASI tool absent: " + name)
    tool_pins = files(list(tools.values()))
    versions = {
        name: subprocess.check_output([str(path), "--version"], text=True)
        for name, path in tools.items()
        if name not in ["executor", "cargo_dispatch", "rustc_dispatch"]
    }
    versions["rustc_verbose"] = subprocess.check_output(
        [str(tools["rustc"]), "-vV"], text=True
    )
    env = os.environ.copy()
    require(
        env.get("TCL_LSP_NATIVE_ADMISSION_DIAGNOSTIC")
        in [None, "", "first-decline-v1"],
        "foreign native diagnostic flag",
    )
    env["TCL_LSP_NATIVE_ADMISSION_DIAGNOSTIC"] = "first-decline-v1"
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
    require(env.get("TCL_REQUIRE_WASM_LINK") == "1", "original real-link required flag")
    require(
        not env.get("WASI_SDK_PATH") or env["WASI_SDK_PATH"] == str(sdk),
        "foreign WASI SDK override",
    )
    for name in ["TCL_WASM_CC", "TCL_WASM_AR", "TCL_WASM_SYSROOT", "CC", "AR"]:
        require(not env.get(name), "unexpected native/WASM tool override: " + name)
    require(
        versions["wasmtime"].startswith("wasmtime 49.0.3"),
        "original CI Wasmtime49.0.3 required",
    )
    require((sdk / "share/wasi-sysroot").is_dir(), "WASI sysroot absent")
    wasm_lib = Path(
        subprocess.check_output(
            [
                str(tools["rustc"]),
                "--print",
                "target-libdir",
                "--target",
                "wasm32-wasip1",
            ],
            text=True,
        ).strip()
    )
    require(wasm_lib.is_dir(), "original CI wasm32-wasip1 std absent")
    env["RUSTC"] = str(tools["rustc"])
    provider_roots = [Path(env["TCL_TOMMATH_DIR"]).parent, sdk]
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
    providers_before = providers()
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
                    "TCL_REQUIRE_WASM_LINK",
                    "TCL_LSP_NATIVE_ADMISSION_DIAGNOSTIC",
                    "WASI_SDK_PATH",
                    "TCL_WASM_CC",
                    "TCL_WASM_AR",
                    "TCL_WASM_SYSROOT",
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
            "scope": "Original wasm_real_link25, exact isolated processes with existing WORK witnesses. Ordinary shared-process shipping suite remains required. Nested reserved runtime build/required features remain test-owned and unchanged; no selected runtime Rust0W claim or runtime ELF/WASM payload upload.",
            "original_roster": roster,
        },
    )
    for label, manifest, triple in [
        ("fetch-native", root / "Cargo.toml", "x86_64-unknown-linux-gnu"),
        ("fetch-runtime", root / "runtime/rust/Cargo.toml", "wasm32-wasip1"),
    ]:
        print("FETCH locked original " + label, flush=True)
        fetched = execute(
            [
                str(tools["cargo"]),
                "fetch",
                "--locked",
                "--manifest-path",
                str(manifest),
                "--target",
                triple,
            ],
            label,
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
    targets = [("tcl-compiler", "wasm_real_link", order)]
    for package, target_name, selectors in targets:
        require(remaining > 512 * 1024, "insufficient log budget before selected build")
        target_group = "original"
        print("BUILD " + target_group + " target " + target_name, flush=True)
        argv = [
            str(tools["cargo"]),
            "test",
            "--locked",
            "-p",
            package,
            "--test",
            target_name,
            "--test",
            "wasm_tiers",
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
        with executable.open("rb") as stream:
            require(stream.read(4) == b"\x7fELF", "selected test is not an ELF")
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
            and providers() == providers_before
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
            if runtime_identity is not None:
                require(
                    runtime.is_file() and sha(runtime) == runtime_identity["sha256"],
                    "reserved runtime changed before original test",
                )
            else:
                require(not runtime.exists(), "unissued reserved runtime appeared")
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
            runtime_after = (
                {
                    "path": str(runtime),
                    "sha256": sha(runtime),
                    "bytes": runtime.stat().st_size,
                }
                if runtime.is_file()
                else None
            )
            runtime_observations.append(
                {
                    "selector": selector,
                    "before": runtime_identity,
                    "after": runtime_after,
                }
            )
            save(out / "reserved-runtime-observations.json", runtime_observations)
            if runtime_identity is None and runtime_after is not None:
                require(
                    runtime.read_bytes()[:4] == b"\0asm",
                    "reserved artifact is not WASM",
                )
                runtime_identity = runtime_after
            elif runtime_identity is not None:
                require(
                    runtime_after == runtime_identity,
                    "reserved runtime changed during original test",
                )
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
        len(original_rows) == 25
        and all(row["state"] in ["PASS", "FAIL"] for row in rows),
        "original25 test inventory incomplete",
    )
    require(runtime_identity is not None, "required real runtime was never produced")
    remaining = run_broad_wasm8(messages, env, remaining)
    status = (0 if all(row["state"] == "PASS" for row in rows)
              and broad_summary["state"] == "PASS" else 1)
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
    if broad_summary["state"] == "RUNNING":
        broad_summary["state"] = "INCOMPLETE"
        for row in broad_rows:
            if row["state"] == "RUNNING":
                row["state"] = "INCOMPLETE"
    broad_summary["error"] = error
    for row in broad_rows:
        if row["state"] == "UNRUN":
            row["unrun_reasons"] = [error]
    print("WASM25 DIAGNOSTIC STOP " + error, file=sys.stderr, flush=True)
finally:
    seals = {}
    for name, capture, original in [
        ("source", source, before),
        ("providers", providers, providers_before),
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
    seals["reserved-runtime"] = (
        runtime_identity is not None
        and runtime.is_file()
        and sha(runtime) == runtime_identity["sha256"]
    )
    if not all(seals.values()):
        status = 2
    save(out / "outcomes.json", rows)
    save(out / "broad-wasm8-outcomes.json", broad_rows)
    save(out / "broad-wasm8-summary.json", broad_summary)
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
            "required_original_tests": 25,
            "required_separate_broad_tests": 8,
            "separate_broad_outcomes": broad_rows,
            "separate_broad_summary": broad_summary,
            "original_tests_closed": sum(
                row["state"] in ["PASS", "FAIL"] for row in original_rows
            ),
            "original_outcomes": original_rows,
            "reserved_runtime": runtime_identity,
            "reserved_runtime_observations": runtime_observations,
            "retained_binary_payloads": False,
            "diagnostic_only": True,
            "full_shared_process_gate_admission": False,
            "eight_package_typing_admission": False,
            "shipping_performance_admission": False,
        },
    )
sys.exit(status)
