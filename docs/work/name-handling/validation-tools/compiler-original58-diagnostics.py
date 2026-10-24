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

"""WORK-only original58 with separate Native35three, SH21two and ReadFlowone groups."""

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
        (
            "original58-",
            "supplemental-native35-",
            "supplemental-sh21-",
            "supplemental-readflow1-",
            "list-",
            "select-",
        )
    ):
        require(
            available >= seconds,
            "insufficient overall time for original full test/list budget",
        )
    seconds = min(seconds, available)
    if prefix.startswith(
        (
            "original58-",
            "supplemental-native35-",
            "supplemental-sh21-",
            "supplemental-readflow1-",
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
require(
    sha(BASE / "compiler-original58-selectors.json")
    == "d4a45ac4eb52a5743b0cb8a5a8256a7f73214f45627c05e25055f2f58bef95d2",
    "exact original58 selector bytes required",
)
require(
    sha(BASE / "compiler-original58-inputs.json")
    == "6711798b5e8d3cafd7dbbe21e0998a4bf0a43d6531aad28f5f0981365b81929e",
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
    == "8fc382cfe187b718bfbbb2401549bf7fdc897526428ca98ef0ec5905c30aa310",
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
    == "0add8b781c91a7a522f77363242381e6c308e3abd20dbb604f71627a1a3213cc",
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
    == "142e58518dbb68ddec64c302c2a0ec60d4f178a3ff09ff9177247299d22d099c",
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
rows = original_rows + supplemental_rows + sh21_rows + readflow_rows
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
            for row in rows
        }
    )
    == 64,
    "separate original58/native35three/SH21two/ReadFlowone group identity conflict",
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
        all(
            sha(root / name) == pin
            for name, pin in {
                **roster["source_pins"],
                **supplemental["source_pins"],
                **sh21_diagnostic["source_pins"],
                **readflow["source_pins"],
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
                "execution_order": "Separate Native35three then SH21two diagnostic duplicates then ReadFlowone first in compilerlib; original58 relative ordering, selectors and semantic flags unchanged.",
                "disk_floor_bytes": FLOOR,
                "absolute_deadline_epoch": deadline,
            },
            "scope": "Exact original native58 selectors on current WORK; standalone bounded per-test diagnosis only. No shared-process/compiler-complete/eight-package/wasm/release acceptance. Historical ambient WASI SDK path retained in provenance, not used by selected native artifacts.",
            "original_roster": original,
            "original_inputs_and_environment": roster,
            "separate_supplemental3": supplemental,
            "separate_sh21_diagnostic2": sh21_diagnostic,
            "separate_readflow1": readflow,
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
    execution_rows = supplemental_rows + sh21_rows + readflow_rows + original_rows
    for target_spec in roster["targets"]:
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
            == target_spec["expected_selected"]
            + sum(
                row["target"] == target_name
                and row["package"] == package
                and row["kind"] == target_kind
                for row in supplemental_rows + sh21_rows + readflow_rows
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
        for index, row in enumerate(selected_rows):
            selector = row["selector"]
            require(
                remaining > 512 * 1024, "insufficient log budget before original test"
            )
            prefix = row["group"] + "-" + target_name + "-" + str(index).zfill(2)
            group_label = (
                "supplemental"
                if row["group"] == "supplemental-readflow1"
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
    require(
        len(original_rows) == 58
        and len(supplemental_rows) == 3
        and len(sh21_rows) == 2
        and len(readflow_rows) == 1
        and len(rows) == 64
        and all(row["state"] in ["PASS", "FAIL"] for row in rows),
        "original test inventory incomplete",
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
            "diagnostic_only": True,
            "full_shared_process_gate_admission": False,
            "eight_package_typing_admission": False,
            "shipping_performance_admission": False,
        },
    )
sys.exit(status)
