"""WORK-only exact CI browser/WASI make commands under the existing process owner."""

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
COMMAND_SECONDS = 3600
EXECUTOR_SHA = "88125bd5267ac897dda4b5f665872c5ae0ab9984b0f42a41edeadf718873d811"
ROSTER_SHA = "b5d30326543a814b86863937d8acd0706ecf221c4c04a2c1cde1455175ac0cef"
BASE = Path(__file__).resolve().parent
COMMANDS = {
    "browser": ["make", "lsp-server-wasm-test"],
    "wasi": ["make", "lsp-server-wasi-test"],
}


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
    return {
        str(p): {
            "sha256": sha(p),
            "symlink": os.readlink(p) if p.is_symlink() else None,
        }
        for p in paths
    }


def source():
    require(git("rev-parse", "HEAD") == args.sha, "source commit changed")
    dirty_status = git("status", "--porcelain=v1", "--untracked-files=all")
    if dirty_status:
        status_lines = dirty_status.splitlines()
        save(
            out / "source-dirty-paths.json",
            {
                "format": "git status --porcelain=v1; git helper strips outer whitespace",
                "total_status_lines": len(status_lines),
                "retained_status_lines": [line[:512] for line in status_lines[:128]],
                "truncated": len(status_lines) > 128
                or any(len(line) > 512 for line in status_lines[:128]),
            },
        )
    require(not dirty_status, "dirty source")
    paths = [
        root / os.fsdecode(p)
        for p in subprocess.check_output(["git", "ls-files", "-z"]).split(b"\0")
        if p
    ]
    return {
        "head": args.sha,
        "tree": git("rev-parse", "HEAD^{tree}"),
        "files": files(paths),
    }


def lifecycle(receipt):
    return (
        all(
            receipt.get(k) is True
            for k in [
                "complete",
                "group_empty_before_parent_reap",
                "timer_armed_before_spawn",
                "artifact_unchanged",
            ]
        )
        and receipt.get("stop_reason") is None
    )


def execute_command(owner, make, environment):
    make = make.resolve()
    require(
        time.time() + COMMAND_SECONDS + 30
        <= float(os.environ["DIAGNOSTIC_DEADLINE_EPOCH"]),
        "insufficient full original-command capture budget",
    )
    return owner.run(
        [str(make), COMMANDS[args.case][1]],
        cwd=root,
        env=environment,
        stdout_path=out / "original-make.stdout",
        stderr_path=out / "original-make.stderr",
        receipt_path=out / "original-make.process.json",
        artifact_path=make,
        deadline_seconds=COMMAND_SECONDS,
        cap_bytes=CAP,
        disk_floor_bytes=FLOOR,
        disk_paths=(root, out, target, Path(os.environ["RUNNER_TEMP"])),
    )


parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--sha", required=True)
parser.add_argument("--case", required=True, choices=COMMANDS)
parser.add_argument("--out", required=True, type=Path)
args = parser.parse_args()
require(re.fullmatch("[0-9a-f]{40}", args.sha), "full source SHA required")
require(
    os.environ.get("GITHUB_SHA") == args.sha
    and os.environ.get("GITHUB_REF") == "refs/heads/work",
    "WORK workflow/source mismatch",
)
require(sys.platform == "linux", "Linux CI recipe only")
root = Path.cwd().resolve()
out = args.out.resolve()
require(
    not out.exists() and not out.is_relative_to(root), "fresh external output required"
)
out.mkdir(parents=True)
resource.setrlimit(resource.RLIMIT_CORE, (0, 0))
status = 2
error = None
receipt = None
roster = None
originals = {}
tools = {}
seals = {}
outputs = {}
environment = os.environ.copy()
try:
    owner_path = BASE / "runtime-diagnostics-exec.py"
    roster_path = BASE / "lsp-host-navigation-inputs.json"
    require(
        sha(owner_path) == EXECUTOR_SHA and sha(roster_path) == ROSTER_SHA,
        "reviewed owner/host roster changed",
    )
    roster = json.loads(roster_path.read_text())
    require(
        roster["executor_sha256"] == EXECUTOR_SHA
        and {k: v["command"] for k, v in roster["cases"].items()} == COMMANDS,
        "host command map mismatch",
    )
    require(
        all(sha(root / p) == h for p, h in roster["source_pins"].items()),
        "reviewed original CI/host/readiness source changed",
    )
    crate = root / "rust" / roster["cases"][args.case]["crate"]
    target = crate / "target"
    dist = crate / "dist"
    require(
        not target.exists() and not dist.exists(), "fresh host product targets required"
    )
    overrides = [
        "RUSTC_WRAPPER",
        "RUSTC_WORKSPACE_WRAPPER",
        "CARGO_ENCODED_RUSTFLAGS",
        "RUSTC_BOOTSTRAP",
        "CARGO_BUILD_TARGET",
        "CARGO_BUILD_RUSTC",
        "CARGO_BUILD_RUSTFLAGS",
        "CARGO_BUILD_RUSTC_WRAPPER",
        "CARGO_BUILD_RUSTC_WORKSPACE_WRAPPER",
        "TCL_RUNTIME_EXPECT_NO_TOMMATH",
        "NODE_OPTIONS",
        "MAKEFLAGS",
        "MFLAGS",
        "CARGO_TARGET_DIR",
    ]
    require(
        not any(environment.get(k) for k in overrides),
        "unexpected build/harness override",
    )
    require(
        not any(
            k.startswith("CARGO_TARGET_")
            and k.endswith(("_RUSTFLAGS", "_RUNNER", "_LINKER"))
            and v
            for k, v in environment.items()
        ),
        "unexpected target flags/runner/linker override",
    )
    require(
        environment.get("RUSTFLAGS") == "-Dwarnings"
        and environment.get("CARGO_INCREMENTAL")
        == environment.get("CARGO_PROFILE_DEV_DEBUG")
        == "0",
        "original CI hygiene/strict flags required",
    )
    require(
        environment.get("CARGO_PROFILE_RELEASE_LTO")
        == ("thin" if args.case == "wasi" else None),
        "original functional CI LTO mode changed",
    )
    require(
        not any(k.startswith("TCL_LSP_TRACE_") and v for k, v in environment.items()),
        "original host commands require no diagnostic trace overrides",
    )
    # WORK-only compile-time observer; no host deadline, assertion or command changes.
    environment["TCL_LSP_TRACE_COARSE_SOURCE"] = "1"
    require(
        not any(
            k.startswith("CARGO_PROFILE_RELEASE_") and k != "CARGO_PROFILE_RELEASE_LTO"
            for k in environment
        ),
        "unexpected release profile override",
    )
    names = ["make", "bash", "git", "node", "rustup", "cargo", "rustc"] + (
        ["wasm-bindgen"] if args.case == "browser" else ["wasmtime", "wasm-opt"]
    )
    tools = {"python": Path(sys.executable).resolve(), "executor": owner_path}
    for name in names:
        path = shutil.which(name)
        require(path is not None, "required original CI tool absent: " + name)
        tools[name] = Path(path)
    tool_resolution = {name: str(tools[name]) for name in names}
    for name in ["cargo", "rustc"]:
        tools[name + "_shim"] = tools[name]
        tools[name] = Path(
            subprocess.check_output(
                [str(tools["rustup"]), "which", name], text=True
            ).strip()
        )
    require(
        not environment.get("RUSTC")
        or Path(environment["RUSTC"]).resolve() == tools["rustc"].resolve(),
        "unexpected rustc override",
    )
    versions = {
        name: subprocess.check_output([str(path), "--version"], text=True)
        for name, path in tools.items()
        if name not in ["python", "executor", "cargo_shim", "rustc_shim"]
    }
    require(
        re.fullmatch(r"v26\.\d+\.\d+\s*", versions["node"]),
        "original CI Node26 required",
    )
    if args.case == "browser":
        require(
            versions["wasm-bindgen"].strip() == "wasm-bindgen 0.2.129",
            "original wasm-bindgen version required",
        )
    else:
        require(
            re.match(r"wasmtime 49\.0\.3(?:\s|$)", versions["wasmtime"]),
            "original wasmtime version required",
        )
    originals = {"source": source(), "tools": files(list(tools.values()))}
    for name, value in originals.items():
        save(out / (name + "-before.json"), value)
    save(
        out / "job.json",
        {
            "requested_sha": args.sha,
            "case": args.case,
            "job": {
                k: environment.get(k)
                for k in [
                    "GITHUB_RUN_ID",
                    "GITHUB_RUN_ATTEMPT",
                    "GITHUB_JOB",
                    "GITHUB_WORKFLOW_REF",
                ]
            },
            "environment": {
                k: environment.get(k)
                for k in [
                    "RUSTFLAGS",
                    "CARGO_INCREMENTAL",
                    "CARGO_PROFILE_DEV_DEBUG",
                    "CARGO_PROFILE_RELEASE_LTO",
                    "TCL_LSP_TRACE_COARSE_SOURCE",
                    "PATH",
                ]
            },
            "tools": {k: str(v) for k, v in tools.items()},
            "path_resolution": tool_resolution,
            "versions": versions,
            "roster": roster,
            "limits": {
                "original_make_seconds": COMMAND_SECONDS,
                "aggregate_log_bytes": CAP,
                "disk_floor_bytes": FLOOR,
            },
        },
    )
    spec = importlib.util.spec_from_file_location("host_navigation_owner", owner_path)
    require(spec is not None and spec.loader is not None, "owner loader absent")
    owner = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(owner)
    receipt = execute_command(owner, tools["make"], environment)
    require(lifecycle(receipt), "original make capture incomplete")
    require(
        all(shutil.which(name) == path for name, path in tool_resolution.items()),
        "selected PATH tool changed",
    )
    require(
        all(
            str(tools[name])
            == subprocess.check_output(
                [str(tools["rustup"]), "which", name], text=True
            ).strip()
            for name in ["cargo", "rustc"]
        ),
        "selected Rust toolchain changed",
    )
    code = receipt["exit_code"]
    require(
        type(code) is int and 0 <= code <= 255, "original make did not exit normally"
    )
    terminal = re.findall(
        r"^(\d+)/(\d+) checks passed$",
        (out / "original-make.stdout").read_text(),
        re.MULTILINE,
    )
    if code == 0:
        require(
            len(terminal) == 1 and int(terminal[0][0]) == int(terminal[0][1]) > 0,
            "original host summary absent/incomplete",
        )
        for p in sorted(dist.rglob("*")):
            if p.is_file():
                outputs[str(p)] = {"bytes": p.stat().st_size, "sha256": sha(p)}
        module = dist / roster["cases"][args.case]["module"]
        require(module.is_file(), "selected original host module missing")
        with module.open("rb") as stream:
            require(
                stream.read(8) == b"\0asm\x01\0\0\0",
                "selected host output is not a version1 wasm module",
            )
    status = 0 if code == 0 else 1
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
    print("HOST NAVIGATION STOP " + error, file=sys.stderr, flush=True)
finally:
    for name, capture in {
        "source": source,
        "tools": lambda: files(list(tools.values())),
    }.items():
        try:
            current = capture()
            save(out / (name + "-after.json"), current)
            seals[name] = name in originals and originals[name] == current
        except (
            RuntimeError,
            OSError,
            ValueError,
            subprocess.SubprocessError,
        ) as failure:
            seals[name] = False
            save(out / (name + "-seal-error.json"), {"error": repr(failure)})
    if not all(seals.values()):
        status = 2
    save(
        out / "final.json",
        {
            "requested_sha": args.sha,
            "case": args.case,
            "overall_exit": status,
            "original_make_exit": None if receipt is None else receipt.get("exit_code"),
            "process": receipt,
            "error": error,
            "seals": seals,
            "host_outputs_after_only": outputs,
            "finished_utc": datetime.datetime.now(datetime.UTC).isoformat(),
            "limits": [
                "Exact original make command/CI tool versions, assertions and deadlines, with compile-time-gated WORK diagnostic logging only. No CLEAN required-CI, native, shipping or performance admission.",
                "Cold product target enforced; original make builds bootstrap/download dependencies as CI normally does. Full pre-build consumed-provider/dependency sealing is NOT established by this runner.",
                "Selected module hashes are after-command outputs; module payload is not retained or independently hashed at runtime. Process owner artifact is make, not the wasm module.",
                "Failed original make may stop before host assertions; read raw logs before attributing any product cause.",
            ],
        },
    )
raise SystemExit(status)
