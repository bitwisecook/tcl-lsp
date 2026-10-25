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

"""WORK-only original native startup diagnostics; no release gate admission."""

import argparse
import base64
import datetime
import hashlib
import importlib.util
import json
import os
import re
import resource
import shutil
import stat
import subprocess
import sys
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


SYSTEM_SEARCH_BASES = (
    "/usr/lib",
    "/usr/lib64",
    "/usr/local/lib",
    "/usr/local/lib64",
    "/usr/share/tcltk",
    "/usr/lib/tcltk",
    "/usr/lib/x86_64-linux-gnu",
    "/opt/local/lib",
    "/opt/homebrew/lib",
)
SYSTEM_CONFIGURED_PATHS = (
    "/opt/proj-a/tcl-lib",
    "/opt/proj-b/tcl-lib",
    "/usr/local/share/tcl",
)
SYSTEM_FILE_LIMIT = 50000
SYSTEM_BYTE_LIMIT = 256 * 1024**2
SYSTEM_INDEX_LIMIT = 4 * 1024**2


def system_node(path) -> dict[str, object]:
    try:
        info = path.lstat()
    except FileNotFoundError:
        return {"kind": "absent"}
    kind = stat.S_IFMT(info.st_mode)
    return {
        "kind": kind,
        "symlink": os.readlink(path) if stat.S_ISLNK(info.st_mode) else None,
        "resolved": str(path.resolve()),
    }


def system_providers(bases=None, supplied_environment=None):
    # Evidence for filesystem discovery; the Rust owner remains authoritative.
    require(sys.platform == "linux", "Linux system provider evidence only")
    env = os.environ if supplied_environment is None else supplied_environment
    overrides = {name: env.get(name) for name in ["TCL_LIBRARY", "TCLLIBPATH"]}
    config_home = Path(env.get("XDG_CONFIG_HOME") or str(Path(env["HOME"]) / ".config"))
    config = config_home / "tcl-lsp/config.ini"
    config_state = system_node(config)
    require(
        all(value is None for value in overrides.values()),
        "unhandled system discovery environment override: " + repr(overrides),
    )
    require(
        config_state["kind"] == "absent",
        "unhandled user library configuration: " + str(config),
    )
    selected_bases = [
        Path(value) for value in (SYSTEM_SEARCH_BASES if bases is None else bases)
    ]
    geometry = []
    libraries = []
    for base in selected_bases:
        children = list(base.iterdir()) if base.is_dir() else []
        require(
            len(children) <= SYSTEM_FILE_LIMIT,
            "system discovery membership limit exceeded",
        )
        candidates = [base] + [child for child in children if child.is_dir()]
        geometry.append(
            {
                "base": str(base),
                "identity": system_node(base),
                "children": [
                    {"path": str(child), "identity": system_node(child)}
                    for child in children
                ],
                "candidates": [
                    {
                        "path": str(candidate),
                        "init": system_node(candidate / "init.tcl"),
                        "init_sha256": sha(candidate / "init.tcl")
                        if (candidate / "init.tcl").is_file()
                        else None,
                    }
                    for candidate in candidates
                ],
            }
        )
        for candidate in candidates:
            if (candidate / "init.tcl").is_file() and candidate not in libraries:
                libraries.append(candidate)
    roots = []
    for library in libraries:
        for path in [library.parent, library]:
            if path not in roots:
                roots.append(path)
    optional_paths = (
        [Path(path) for path in SYSTEM_CONFIGURED_PATHS] if bases is None else []
    )
    for path in optional_paths:
        if path.is_dir() and path not in roots:
            roots.append(path)
    ancestry = {
        str(parent): system_node(parent)
        for path in selected_bases + roots
        for parent in path.parents
    }
    nodes = {}
    total_bytes = 0

    def visit(path, ancestors):
        nonlocal total_bytes
        require(len(ancestors) <= 128, "system provider depth limit exceeded")
        if str(path) in nodes:
            return
        require(
            len(nodes) < SYSTEM_FILE_LIMIT, "system provider inventory limit exceeded"
        )
        entry = system_node(path)
        nodes[str(path)] = entry
        if path.is_dir():
            physical = path.resolve(strict=True)
            require(
                physical not in ancestors,
                "system provider directory symlink cycle: " + str(path),
            )
            children = sorted(path.iterdir())
            entry["children"] = [child.name for child in children]
            for child in children:
                visit(child, ancestors + (physical,))
        elif path.is_file():
            size = path.stat().st_size
            require(
                total_bytes + size <= SYSTEM_BYTE_LIMIT,
                "system provider byte limit exceeded",
            )
            total_bytes += size
            entry.update(bytes=size, sha256=sha(path))
        else:
            require(
                entry["kind"] == "absent" or path.is_symlink(),
                "unsupported system provider node: " + str(path),
            )

    for path in roots:
        visit(path, ())
    return {
        "scope": "Linux filesystem discovery evidence only; no independent discovery semantic authority",
        "discovery_owner": "rust/tcl-lsp-core/src/tcl_install.rs",
        "discovery_owner_sha256": sha(root / "rust/tcl-lsp-core/src/tcl_install.rs"),
        "environment": overrides,
        "user_config": {"path": str(config), "identity": config_state},
        "ordered_bases": geometry,
        "path_ancestors": ancestry,
        "original_context_library_paths": {
            str(path): system_node(path) for path in optional_paths
        },
        "lexical_library_paths": [str(path) for path in libraries],
        "auto_path_roots": [str(path) for path in roots],
        "nodes": nodes,
        "hashed_file_bytes": total_bytes,
    }


def retain_system_indexes(snapshot):
    indexes = []
    total = 0
    for name, entry in snapshot["nodes"].items():
        if Path(name).name not in ["tclIndex", "pkgIndex.tcl"] or "sha256" not in entry:
            continue
        data = Path(name).read_bytes()
        total += len(data)
        require(
            total <= SYSTEM_INDEX_LIMIT, "system index evidence byte limit exceeded"
        )
        require(
            hashlib.sha256(data).hexdigest() == entry["sha256"],
            "system index changed during retention",
        )
        indexes.append(
            {
                "path": name,
                "sha256": entry["sha256"],
                "bytes": len(data),
                "base64": base64.b64encode(data).decode("ascii"),
            }
        )
    return {
        "scope": "All sealed system tclIndex/pkgIndex.tcl bytes; logs identify actually consumed paths",
        "indexes": indexes,
    }


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


def execute(argv, prefix, artifact, seconds, environment, budget):
    require(budget > 0, "aggregate log budget exhausted")
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
require(re.fullmatch("[0-9a-f]{40}", args.sha), "full lowercase SHA required")
require(
    os.environ.get("GITHUB_SHA") == args.sha, "dispatch checkout/workflow SHA mismatch"
)
require(os.environ.get("GITHUB_REF") == "refs/heads/work", "WORK only")
root = Path.cwd().resolve()
out = args.out.resolve()
target = Path(os.environ["CARGO_TARGET_DIR"]).resolve()
require(
    not out.exists() and not out.is_relative_to(root), "fresh external output required"
)
require(
    not target.exists() and not target.is_relative_to(root),
    "fresh external target required",
)
out.mkdir(parents=True)
resource.setrlimit(resource.RLIMIT_CORE, (0, 0))
remaining = CAP
before = providers_before = dependencies_before = artifact = tool_pins = None
dependency_roots: list[Path] = []
system_providers_before = None
processes = []
cases = []
error = None
final_errors = []
status = 2
try:
    executor = BASE.parent / "runtime-diagnostics-exec.py"
    require(sha(executor) == EXECUTOR_SHA, "reviewed process owner changed")
    spec = importlib.util.spec_from_file_location("startup_owner", executor)
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
    tool_pins = files(list(tools.values()))
    versions = {
        name: subprocess.check_output([str(path), "--version"], text=True)
        for name, path in tools.items()
        if name != "executor"
    }
    versions["rustc_verbose"] = subprocess.check_output(
        [str(tools["rustc"]), "-vV"], text=True
    )
    environment = os.environ.copy()
    # No wrappers, encoded flags, alternate targets or no-TomMath fallback may
    # change the selected native server build behind the evidence receipt.
    for name in [
        "RUSTC_WRAPPER",
        "RUSTC_WORKSPACE_WRAPPER",
        "CARGO_ENCODED_RUSTFLAGS",
        "RUSTC_BOOTSTRAP",
        "TCL_RUNTIME_EXPECT_NO_TOMMATH",
        "CARGO_BUILD_TARGET",
        "CARGO_BUILD_RUSTC",
        "CARGO_BUILD_RUSTFLAGS",
        "CARGO_BUILD_RUSTC_WRAPPER",
        "CARGO_BUILD_RUSTC_WORKSPACE_WRAPPER",
    ]:
        require(not environment.get(name), "unexpected build override: " + name)
    require(
        not any(
            name.startswith("CARGO_TARGET_") and name.endswith("_RUSTFLAGS") and value
            for name, value in environment.items()
        ),
        "unexpected target Rust flags",
    )
    require(
        environment.get("RUSTFLAGS") == "-Dwarnings",
        "strict normal Rust warnings required",
    )
    environment["RUSTC"] = str(tools["rustc"])
    provider_roots = [Path(environment["TCL_TOMMATH_DIR"]).parent]
    sysroot = Path(
        subprocess.check_output(
            [str(tools["rustc"]), "--print", "sysroot"], text=True
        ).strip()
    )
    provider_roots.append(sysroot / "lib")
    require(
        (provider_roots[0] / "libtommath/tommath.h").is_file(),
        "full TomMath provider absent",
    )
    providers_before = files(provider_roots)
    save(out / "providers-before.json", providers_before)
    before = source()
    save(out / "source-before.json", before)
    expected_inputs = json.loads((BASE / "original-inputs.json").read_text())
    require(
        all(sha(root / name) == pin for name, pin in expected_inputs.items()),
        "original client/input pin changed",
    )
    save(
        out / "job.json",
        {
            "utc": datetime.datetime.now(datetime.UTC).isoformat(),
            "requested_sha": args.sha,
            "python_version": sys.version,
            "tools": tool_pins,
            "tool_versions": versions,
            "job": {
                name: environment.get(name)
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
                name: environment.get(name)
                for name in [
                    "CARGO_TARGET_DIR",
                    "CARGO_HOME",
                    "RUSTUP_TOOLCHAIN",
                    "CARGO_BUILD_JOBS",
                    "CARGO_INCREMENTAL",
                    "RUSTFLAGS",
                    "TCL_TOMMATH_DIR",
                ]
            },
            "limits": {
                "dependency_fetch_seconds": 600,
                "build_seconds": 2400,
                "each_worker_seconds": 180,
                "aggregate_log_bytes": CAP,
                "disk_floor_bytes": FLOOR,
            },
            "scope": "ci-profile native startup diagnostic only; original typical and Context assertions; no shipping performance, editor-host or eight-package typecheck admission",
        },
    )
    argv = [
        str(tools["cargo"]),
        "build",
        "--locked",
        "-p",
        "tcl-lsp-server",
        "--bin",
        "tcl-lsp-server",
        "--profile",
        "ci",
        "--message-format=json-render-diagnostics",
        "-j",
        "2",
    ]
    print(
        "FETCH locked native Cargo inputs before build;600s housekeeping bound",
        flush=True,
    )
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
        environment,
        remaining,
    )
    processes.append(fetched)
    remaining -= fetched["retained_bytes"]
    require(
        lifecycle(fetched) and fetched["exit_code"] == 0,
        "Cargo dependency fetch failed or unclosed",
    )
    cargo_home = Path(environment.get("CARGO_HOME", str(Path.home() / ".cargo")))
    dependency_roots = [
        p
        for p in [
            cargo_home / "registry/src",
            cargo_home / "registry/cache",
            cargo_home / "git/checkouts",
        ]
        if p.exists()
    ]
    require(bool(dependency_roots), "resolved Cargo dependency inputs absent")
    dependencies_before = files(dependency_roots)
    save(out / "dependencies-before-build.json", dependencies_before)
    print(
        "BUILD current selected ci-profile server; strict Rust warnings; fresh target",
        flush=True,
    )
    built = execute(argv, "build", tools["cargo"], 2400, environment, remaining)
    processes.append(built)
    remaining -= built["retained_bytes"]
    require(
        lifecycle(built) and built["exit_code"] == 0,
        "selected server build failed or unclosed",
    )
    rows = [
        json.loads(line)
        for line in (out / "build.stdout").read_text().splitlines()
        if line.startswith("{")
    ]
    require(
        not any(
            row.get("reason") == "compiler-message"
            and row.get("message", {}).get("level") in ["warning", "error"]
            for row in rows
        ),
        "Rust diagnostic observed",
    )
    require(
        sum(
            row.get("reason") == "build-finished" and row.get("success") is True
            for row in rows
        )
        == 1,
        "successful Cargo terminal missing",
    )
    selected = [
        row
        for row in rows
        if row.get("reason") == "compiler-artifact"
        and row.get("target", {}).get("name") == "tcl-lsp-server"
        and row.get("target", {}).get("kind") == ["bin"]
        and row.get("manifest_path") == str(root / "rust/tcl-lsp-server/Cargo.toml")
        and row.get("profile", {}).get("test") is False
        and row.get("executable")
    ]
    require(
        len(selected) == 1 and selected[0].get("fresh") is False,
        "fresh exact server event missing or ambiguous",
    )
    server = Path(selected[0]["executable"]).resolve()
    require(
        server.is_file() and server.is_relative_to(target),
        "server outside owned fresh target",
    )
    artifact = {
        "path": str(server),
        "sha256": sha(server),
        "cargo_event": selected[0],
        "profile_scope": "ci, functional diagnosis only; not shipping release",
    }
    # Retain the already selected build for separately bounded local diagnosis.
    # It is an artifact payload, not retained process output or another build.
    payload = out / "server-payload"
    shutil.copyfile(server, payload)
    require(
        payload.read_bytes()[:4] == b"\x7fELF" and sha(payload) == artifact["sha256"],
        "retained selected ELF mismatch",
    )
    artifact["retained_payload"] = payload.name
    artifact["elf_magic"] = "7f454c46"
    save(out / "server-artifact.json", artifact)
    dependencies_after_build = files(dependency_roots)
    save(out / "dependencies-after-build.json", dependencies_after_build)
    require(
        dependencies_after_build == dependencies_before,
        "Cargo dependency build-input seal mismatch",
    )
    require(
        source() == before
        and files(provider_roots) == providers_before
        and files(list(tools.values())) == tool_pins,
        "pre-runtime source/provider/tool seal mismatch",
    )
    system_providers_before = system_providers()
    save(out / "system-providers-before.json", system_providers_before)
    save(
        out / "system-indexes-before.json",
        retain_system_indexes(system_providers_before),
    )
    work = Path(os.environ["RUNNER_TEMP"]) / "lsp-startup-workers"
    require(not work.exists(), "fresh worker materialisation required")
    shutil.copytree(BASE / "typical", work / "typical")
    shutil.copytree(BASE / "context", work / "context")
    save(work / "typical/input-pins.json", {"root": str(root), "pins": expected_inputs})
    save(
        work / "typical/manifest.json",
        {
            "files": {
                p.name: sha(p) for p in (work / "typical").iterdir() if p.is_file()
            }
        },
    )
    context_inputs = {str(root / name): pin for name, pin in expected_inputs.items()}
    context_inputs.update(
        {
            str(path): sha(path)
            for path in (work / "context").iterdir()
            if path.is_file()
        }
    )
    save(work / "context/repro-inputs.json", context_inputs)
    materialised_pins = files([work])
    save(out / "materialised-worker-inputs.json", materialised_pins)
    for case in ["typical", "context"]:
        require(
            system_providers() == system_providers_before,
            "pre-worker system provider seal mismatch",
        )
        require(
            remaining > 1024 * 1024, "insufficient aggregate budget for original worker"
        )
        case_out = out / case
        wire_cap = remaining - 512 * 1024
        env = environment.copy()
        env.update(
            TCL_LSP_TRACE_COARSE_SOURCE="1",
            TCL_LSP_TRACE_COMPLETED_COMMAND_WORLD="1",
            TCL_LSP_TRACE_WORKSPACE_SCAN="1",
            TCL_LSP_DIAGNOSTIC_ROOT=str(root),
            TCL_LSP_WIP_WIRE_CAP=str(wire_cap),
            TCL_LSP_CONTEXT_WIRE_CAP=str(wire_cap),
        )
        if case == "typical":
            argv = [
                str(tools["python"]),
                "-I",
                "-S",
                "-B",
                str(work / case / "worker.py"),
                "--server",
                str(server),
                "--server-sha256",
                artifact["sha256"],
                "--protocol-sha256",
                sha(work / case / "manifest.json"),
                "--case",
                "typical",
                "--out",
                str(case_out),
            ]
        else:
            argv = [
                str(tools["python"]),
                "-I",
                "-S",
                "-B",
                str(work / case / "reproduce.py"),
                "--server",
                str(server),
                "--server-sha256",
                artifact["sha256"],
                "--out",
                str(case_out),
                "--inputs-sha256",
                sha(work / case / "repro-inputs.json"),
            ]
        print("RUN original " + case + " worker;180s owned bound", flush=True)
        receipt = execute(argv, case, tools["python"], 180, env, 512 * 1024)
        processes.append(receipt)
        wire_bytes = sum(
            (case_out / name).stat().st_size
            for name in ["server.stdout.frames", "server.stderr"]
            if (case_out / name).is_file()
        )
        remaining -= receipt["retained_bytes"] + wire_bytes
        result_path = case_out / (
            "measurement.json" if case == "typical" else "result.json"
        )
        result = json.loads(result_path.read_text()) if result_path.is_file() else None
        good = result is not None and result.get("capture_error") is None
        if case == "typical" and result is not None:
            good = (
                good
                and result.get("status") == "MEASURED_NOT_ADMITTED"
                and result.get("fixture_bytes_stable") is True
                and result.get("server_sha256_before")
                == result.get("server_sha256_after")
                == artifact["sha256"]
                and result.get("worker_pgid")
                == result.get("server_pgid")
                == receipt["process_group"]
            )
        elif result is not None:
            good = (
                good
                and result.get("status") == "COMPLETE_ORIGINAL_ASSERTIONS"
                and result.get("server_unchanged") is True
                and result.get("fixture_unchanged") is True
                and result.get("server_before")
                == result.get("server_after")
                == artifact["sha256"]
            )
        cases.append(
            {
                "case": case,
                "exit_code": receipt["exit_code"],
                "original_assertions_complete": good and receipt["exit_code"] == 0,
                "wire_bytes": wire_bytes,
            }
        )
        save(out / "cases.json", cases)
        require(
            lifecycle(receipt) and remaining >= 0 and wire_bytes <= wire_cap,
            "worker lifecycle or log bound failed",
        )
        require(
            result is not None and not result.get("capture_error"),
            "worker durable final receipt absent/incomplete",
        )
        require(files([work]) == materialised_pins, "worker inputs changed")
        require(sha(server) == artifact["sha256"], "server changed after worker")
    status = 0 if all(case["original_assertions_complete"] for case in cases) else 1
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
    print("STARTUP DIAGNOSTIC STOP " + error, file=sys.stderr, flush=True)
finally:
    seals = {}
    for name, capture, original in [
        ("source", source, before),
        ("providers", lambda: files(provider_roots), providers_before),
        ("system-providers", system_providers, system_providers_before),
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
    seals["artifact"] = (
        artifact is not None
        and Path(artifact["path"]).is_file()
        and sha(artifact["path"]) == artifact["sha256"]
    )
    if not all(seals.values()):
        status = 2
    save(
        out / "final.json",
        {
            "finished_utc": datetime.datetime.now(datetime.UTC).isoformat(),
            "overall_exit": status,
            "error": error,
            "final_errors": final_errors,
            "seals": seals,
            "selected_server_compilation_0w": artifact is not None,
            "cases": cases,
            "processes": processes,
            "remaining_log_bytes": remaining,
            "diagnostic_only": True,
            "shipping_performance_admission": False,
            "eight_package_typing_admission": False,
            "editor_host_admission": False,
        },
    )
sys.exit(status)
