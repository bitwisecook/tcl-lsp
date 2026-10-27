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

"""WORK-only original Jim45, separate static5/control1, then raw-method diagnosis1."""

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
STATIC_CONTROL_TARGET = "jim_static_read_alternatives"
STATIC_CONTROL_SELECTOR = (
    "original_static_alternatives_withdraw_only_their_synthetic_local_absence"
)
STATIC_CONTROL_SOURCE_PIN = (
    "70929c92a61c0c07c784a8288193a3a12880be0ef6974ff7375b4cdd58c6d287"
)
STATIC_CONTROL_CLEANUP_RESERVE_SECONDS = 30
RAW_METHOD_DIAGNOSTIC_SELECTOR = "a_raw_two_word_proc_is_a_method_without_the_class_variables"
RAW_METHOD_DIAGNOSTIC_QUOTA_MARKER = "JIM_SOURCE_OWNER stage=option_trace_quota state=UNOBSERVED limit=1024"


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


def execute(
    argv, prefix, artifact, seconds, environment, budget, *, on_budget_refusal=None
):
    if budget <= 0 and on_budget_refusal is not None:
        on_budget_refusal()
    require(budget > 0, "aggregate log budget exhausted")
    available = deadline - time.time()
    if prefix.startswith(
        (
            "build-jim_static_read_alternatives",
            "list-jim_static_read_alternatives",
            "jim_static_read_alternatives-control-",
            "jim_document-raw-method-diagnostic-",
        )
    ):
        available -= STATIC_CONTROL_CLEANUP_RESERVE_SECONDS
    if available <= 0 and on_budget_refusal is not None:
        on_budget_refusal()
    require(available > 0, "absolute diagnostic deadline reached")
    if prefix.startswith(
        (
            "jim_document-",
            "jim_own_surface-",
            "list-",
            "jim_static_read_alternatives-control-",
        )
    ):
        if available < seconds and on_budget_refusal is not None:
            on_budget_refusal()
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


def run_selected(
    executable,
    artifact,
    target_name,
    selectors,
    selected_rows,
    outcomes_name,
    prefix_target,
):
    global remaining
    for index, selector in enumerate(selectors):
        require(remaining > 512 * 1024, "insufficient log budget before original test")
        row = next(
            row
            for row in selected_rows
            if row["target"] == target_name and row["selector"] == selector
        )
        row["state"] = "RUNNING"
        save(out / outcomes_name, selected_rows)
        prefix = prefix_target + "-" + str(index).zfill(2)
        print("RUN original " + selector, flush=True)
        runtime_env = env.copy()
        selected_env = row["env"]
        if not isinstance(selected_env, dict):
            raise TypeError("original per-selector environment is not a mapping")
        for name, value in selected_env.items():
            if not isinstance(name, str) or not isinstance(value, str):
                raise TypeError("original per-selector environment is not text")
            runtime_env[name] = value

        def record_budget_refusal(row=row):
            row["state"] = "UNRUN"
            save(out / outcomes_name, selected_rows)

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
            on_budget_refusal=(
                record_budget_refusal
                if outcomes_name
                in [
                    "supplemental-outcomes.json",
                    "diagnostic-static-outcomes.json",
                    "static-alternatives-outcomes.json",
                    "raw-method-diagnostic-outcomes.json",
                ]
                else None
            ),
        )
        processes.append(receipt)
        remaining -= receipt["retained_bytes"]
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
            save(out / outcomes_name, selected_rows)
            raise
        save(out / outcomes_name, selected_rows)
        require(sha(executable) == artifact["sha256"], "selected test artifact changed")


def run_raw_method_diagnostic():
    require(
        (len(rows), len(supplemental_rows), len(diagnostic_rows), len(static_control_rows), len(raw_method_diagnostic_rows)) == (7, 38, 5, 1, 1),
        "raw-method diagnosis requires exact old51 plus separate1 inventories",
    )
    require(
        all(
            row["state"] in ["PASS", "FAIL"]
            for row in rows + supplemental_rows + diagnostic_rows + static_control_rows
        ),
        "raw-method diagnosis requires the closed unchanged old51",
    )
    document_artifacts = [item for item in artifacts if item["target"] == "jim_document"]
    require(len(document_artifacts) == 1, "one existing fresh Jim document ELF required")
    artifact = document_artifacts[0]
    listing = json.loads((out / "roster-jim_document.json").read_text())
    require(
        RAW_METHOD_DIAGNOSTIC_SELECTOR in listing["selected"]
        and listing["executable_sha256"] == artifact["sha256"],
        "raw-method selector requires the same fresh listed Jim document ELF",
    )
    print("RUN separate raw-method source-owner diagnosis after unchanged old51", flush=True)
    run_selected(
        Path(artifact["path"]),
        artifact,
        "jim_document",
        [RAW_METHOD_DIAGNOSTIC_SELECTOR],
        raw_method_diagnostic_rows,
        "raw-method-diagnostic-outcomes.json",
        "jim_document-raw-method-diagnostic",
    )
    row = raw_method_diagnostic_rows[0]
    stderr = (out / row["stderr"]).read_text(errors="replace")
    quota = RAW_METHOD_DIAGNOSTIC_QUOTA_MARKER in stderr
    observed = any(
        line.startswith("JIM_SOURCE_OWNER stage=jim.raw_member.")
        for line in stderr.splitlines()
    )
    row.update(
        observation_state="OBSERVED" if observed and not quota else "UNOBSERVED",
        option_trace_quota_reached=quota,
    )
    save(out / "raw-method-diagnostic-outcomes.json", raw_method_diagnostic_rows)
    require(
        row["observation_state"] == "OBSERVED",
        "raw-method source-owner first-refusal UNOBSERVED",
    )


parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--sha", required=True)
parser.add_argument("--out", type=Path, required=True)
parser.add_argument("--full-rosters", action="store_true")
parser.add_argument("--diagnostic-statics", action="store_true")
parser.add_argument("--static-alternatives-control", action="store_true")
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
require(
    sha(BASE / "jim-source-roster.json")
    == "7dd6c153251e4ccf4f044aa0ac4f47445a86df88b5b0fe6eafcb16159a123586",
    "exact original seven roster pin",
)
roster = json.loads((BASE / "jim-source-roster.json").read_text())
require(roster["executor_sha256"] == EXECUTOR_SHA, "original process owner pin")
require(len(roster["selected"]) == 7, "original seven Jim selectors required")
require(
    len({(row["target"], row["selector"]) for row in roster["selected"]}) == 7,
    "duplicate original selector",
)
require(
    sum(row["target"] == "jim_document" for row in roster["selected"]) == 5,
    "original class/source five required",
)
require(
    sum(row["target"] == "jim_own_surface" for row in roster["selected"]) == 2,
    "original core-row identity two required",
)
rows = [{**row, "state": "UNRUN"} for row in roster["selected"]]

supplemental_rows = [
    {"target": target_name, "selector": selector, "env": {}, "state": "UNRUN"}
    for target_name, selectors in roster["full_rosters"].items()
    for selector in selectors
    if args.full_rosters
    and (target_name, selector)
    not in {(row["target"], row["selector"]) for row in roster["selected"]}
]
require(
    len(supplemental_rows) == (38 if args.full_rosters else 0),
    "exact supplemental full-roster inventory required",
)

require(
    not args.diagnostic_statics or args.full_rosters, "static diagnosis follows full45"
)
static_selectors = [
    "a_static_is_a_local_of_the_body_and_no_argument",
    "every_static_spelling_declares_its_name",
    "the_edges_of_a_static_list",
    "the_body_of_a_proc_with_statics_is_walked",
    "a_static_adds_nothing_to_the_arity",
]
require(
    len(set(static_selectors)) == 5
    and set(static_selectors).issubset(roster["full_rosters"]["jim_document"]),
    "exact original static5 required",
)
diagnostic_rows = [
    {
        "target": "jim_document",
        "selector": selector,
        "env": {"TCL_LSP_TRACE_RBS_SOURCE_OWNER": "1"},
        "state": "UNRUN",
    }
    for selector in static_selectors
    if args.diagnostic_statics
]
require(
    not args.static_alternatives_control
    or (args.full_rosters and args.diagnostic_statics),
    "new static alternatives control follows original45 and diagnostic5",
)
static_control_rows = [
    {
        "target": STATIC_CONTROL_TARGET,
        "selector": STATIC_CONTROL_SELECTOR,
        "env": {},
        "state": "UNRUN",
    }
    for _ in range(1 if args.static_alternatives_control else 0)
]
require(
    RAW_METHOD_DIAGNOSTIC_SELECTOR in roster["full_rosters"]["jim_document"],
    "raw-method diagnostic repeats the existing original failed selector",
)
raw_method_diagnostic_rows = [
    {
        "target": "jim_document",
        "selector": RAW_METHOD_DIAGNOSTIC_SELECTOR,
        "env": {"TCL_LSP_TRACE_JIM_SOURCE_OWNER": "1"},
        "state": "UNRUN",
        "observation_state": "UNOBSERVED",
    }
    for _ in range(1 if args.static_alternatives_control else 0)
]
remaining = CAP
before = providers_before = dependencies_before = tool_pins = None
artifacts = []
processes = []
error = None
final_errors = []
status = 2
save(out / "outcomes.json", rows)
save(out / "supplemental-outcomes.json", supplemental_rows)
save(out / "diagnostic-static-outcomes.json", diagnostic_rows)
save(out / "static-alternatives-outcomes.json", static_control_rows)
save(out / "raw-method-diagnostic-outcomes.json", raw_method_diagnostic_rows)
try:
    executor = BASE / "runtime-diagnostics-exec.py"
    require(sha(executor) == EXECUTOR_SHA, "reviewed process owner changed")
    spec = importlib.util.spec_from_file_location("jim_owner", executor)
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
        if name not in ["executor", "jimsh"] and not name.startswith("tclsh")
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
        env.get("CARGO_PROFILE_TEST_DEBUG") == "0", "original test debug-info setting"
    )
    require(
        env.get("TCL_LSP_REQUIRE_TCLSH") == "9.0,8.6",
        "original required Tcl references",
    )
    require(
        not env.get("TCL_LSP_TRACE_JIM_SOURCE_OWNER"), "per-selector Jim trace only"
    )
    require(
        not env.get("TCL_LSP_TRACE_RBS_SOURCE_OWNER"), "per-diagnostic RBS trace only"
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
    before = source()
    save(out / "source-before.json", before)
    require(
        all(sha(root / name) == pin for name, pin in roster["source_pins"].items()),
        "original test/observer input changed",
    )
    if args.static_alternatives_control:
        require(
            sha(root / "rust/tcl-spectcl/tests/jim_static_read_alternatives.rs")
            == STATIC_CONTROL_SOURCE_PIN,
            "exact new static alternatives source changed",
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
                "disk_floor_bytes": FLOOR,
                "absolute_deadline_epoch": deadline,
            },
            "scope": "Exact unchanged original Jim7 (five class/source, two core-row identity) on current WORK, optionally followed by the remaining38 original full-roster tests in a separate ledger. Separate pinned-provider diagnosis only; no C1/C2, shared-process runtime, typing or release acceptance.",
            "original_roster": roster,
            "supplemental_full_rosters_requested": args.full_rosters,
            "supplemental_roster": supplemental_rows,
            "diagnostic_static_requested": args.diagnostic_statics,
            "diagnostic_static_roster": diagnostic_rows,
            "static_alternatives_control_requested": args.static_alternatives_control,
            "static_alternatives_control_roster": static_control_rows,
            "raw_method_diagnostic_requested": bool(raw_method_diagnostic_rows),
            "raw_method_diagnostic_roster": raw_method_diagnostic_rows,
            "static_alternatives_control_source_pin": STATIC_CONTROL_SOURCE_PIN,
            "static_alternatives_cleanup_reserve_seconds": (
                STATIC_CONTROL_CLEANUP_RESERVE_SECONDS
            ),
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
        (
            "tcl-spectcl",
            target_name,
            [
                row["selector"]
                for row in roster["selected"]
                if row["target"] == target_name
            ],
        )
        for target_name in ["jim_document", "jim_own_surface"]
    ]
    for package, target_name, selectors in targets:
        require(remaining > 512 * 1024, "insufficient log budget before selected build")
        print("BUILD original target " + target_name, flush=True)
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
            len(listed) == len(set(listed)) == len(roster["full_rosters"][target_name])
            and set(listed) == set(roster["full_rosters"][target_name])
            and set(selectors).issubset(listed),
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
        run_selected(
            executable,
            artifact,
            target_name,
            selectors,
            rows,
            "outcomes.json",
            target_name,
        )
    for artifact in artifacts:
        target_name = artifact["target"]
        selectors = [
            row["selector"] for row in supplemental_rows if row["target"] == target_name
        ]
        run_selected(
            Path(artifact["path"]),
            artifact,
            target_name,
            selectors,
            supplemental_rows,
            "supplemental-outcomes.json",
            target_name + "-supplemental",
        )
    if diagnostic_rows:
        artifact = next(item for item in artifacts if item["target"] == "jim_document")
        run_selected(
            Path(artifact["path"]),
            artifact,
            "jim_document",
            static_selectors,
            diagnostic_rows,
            "diagnostic-static-outcomes.json",
            "jim_document-static-diagnostic",
        )
    if args.static_alternatives_control:
        control_roster = {STATIC_CONTROL_TARGET: [STATIC_CONTROL_SELECTOR]}
        for package, target_name, selectors in [
            ("tcl-spectcl", STATIC_CONTROL_TARGET, [STATIC_CONTROL_SELECTOR])
        ]:
            require(
                remaining > 512 * 1024, "insufficient log budget before selected build"
            )
            print("BUILD new static alternatives target " + target_name, flush=True)
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
                and row.get("manifest_path")
                == str(root / "rust" / package / "Cargo.toml")
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
                len(listed) == len(set(listed)) == len(control_roster[target_name])
                and set(listed) == set(control_roster[target_name])
                and set(selectors).issubset(listed),
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
            run_selected(
                executable,
                artifact,
                target_name,
                selectors,
                static_control_rows,
                "static-alternatives-outcomes.json",
                target_name + "-control",
            )
    if raw_method_diagnostic_rows:
        run_raw_method_diagnostic()
    require(
        all(row["state"] in ["PASS", "FAIL"] for row in static_control_rows),
        "new static alternatives control inventory incomplete",
    )
    require(
        all(row["state"] in ["PASS", "FAIL"] for row in diagnostic_rows),
        "static diagnostic inventory incomplete",
    )
    require(
        all(row["state"] in ["PASS", "FAIL"] for row in supplemental_rows),
        "supplemental full-roster inventory incomplete",
    )
    require(
        len(rows) == 7 and all(row["state"] in ["PASS", "FAIL"] for row in rows),
        "original test inventory incomplete",
    )
    status = (
        0
        if all(
            row["state"] == "PASS"
            for row in rows + supplemental_rows + diagnostic_rows + static_control_rows + raw_method_diagnostic_rows
        )
        else 1
    )
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
    print("JIM SOURCE DIAGNOSTIC STOP " + error, file=sys.stderr, flush=True)
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
    save(out / "supplemental-outcomes.json", supplemental_rows)
    save(out / "diagnostic-static-outcomes.json", diagnostic_rows)
    save(out / "static-alternatives-outcomes.json", static_control_rows)
    save(out / "raw-method-diagnostic-outcomes.json", raw_method_diagnostic_rows)
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
            "required_original_tests": 7,
            "static_alternatives_control_requested": args.static_alternatives_control,
            "required_static_alternatives_control_tests": len(static_control_rows),
            "static_alternatives_control_outcomes": static_control_rows,
            "static_alternatives_control_tests_closed": sum(
                row["state"] in ["PASS", "FAIL"] for row in static_control_rows
            ),
            "diagnostic_static_requested": args.diagnostic_statics,
            "required_diagnostic_static_tests": len(diagnostic_rows),
            "diagnostic_static_outcomes": diagnostic_rows,
            "diagnostic_static_tests_closed": sum(
                row["state"] in ["PASS", "FAIL"] for row in diagnostic_rows
            ),
            "supplemental_full_rosters_requested": args.full_rosters,
            "required_supplemental_tests": len(supplemental_rows),
            "supplemental_outcomes": supplemental_rows,
            "supplemental_tests_closed": sum(
                row["state"] in ["PASS", "FAIL"] for row in supplemental_rows
            ),
            "original_tests_closed": sum(
                row["state"] in ["PASS", "FAIL"] for row in rows
            ),
            "raw_method_diagnostic_requested": bool(raw_method_diagnostic_rows),
            "required_raw_method_diagnostic_tests": len(raw_method_diagnostic_rows),
            "raw_method_diagnostic_outcomes": raw_method_diagnostic_rows,
            "raw_method_diagnostic_tests_closed": sum(
                row["state"] in ["PASS", "FAIL"] for row in raw_method_diagnostic_rows
            ),
            "raw_method_diagnostic_observations_closed": sum(
                row.get("observation_state") == "OBSERVED" for row in raw_method_diagnostic_rows
            ),
            "diagnostic_only": True,
            "full_shared_process_gate_admission": False,
            "eight_package_typing_admission": False,
            "shipping_performance_admission": False,
        },
    )
sys.exit(status)
