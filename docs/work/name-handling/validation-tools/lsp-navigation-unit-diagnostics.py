"""WORK-only separate public navigation readiness unit; no host or startup admission."""

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
ROSTER_SHA = "4a47f12be78bc36b9da7a997761564a5bb39b4c68e77d25f39f16e133b8bfa3f"
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
    require(len(terminal) == 1, "selected test terminal missing/ambiguous")
    label, passed, failed, ignored, measured, _ = terminal[0]
    require(
        "running 1 test" in stdout and int(ignored) == int(measured) == 0,
        "selected test counts mismatch",
    )
    require(
        (label, int(passed), int(failed), exit_code)
        in [("ok", 1, 0, 0), ("FAILED", 0, 1, 101)],
        "exit/terminal mismatch",
    )
    return "PASS" if int(passed) == 1 else "FAIL"


def execute(argv, prefix, artifact, seconds):
    global remaining
    require(remaining > 0, "aggregate log budget exhausted")
    require(
        time.time() + seconds <= float(os.environ["DIAGNOSTIC_DEADLINE_EPOCH"]),
        "insufficient full process budget",
    )
    receipt = owner.run(
        argv,
        cwd=root,
        env=environment,
        stdout_path=out / (prefix + ".stdout"),
        stderr_path=out / (prefix + ".stderr"),
        receipt_path=out / (prefix + ".process.json"),
        artifact_path=artifact,
        deadline_seconds=seconds,
        cap_bytes=remaining,
        disk_floor_bytes=FLOOR,
        disk_paths=(root, out, target, Path(os.environ["RUNNER_TEMP"])),
    )
    processes.append(receipt)
    remaining -= receipt["retained_bytes"]
    require(lifecycle(receipt) and remaining >= 0, "process capture incomplete")
    return receipt


parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--sha", required=True)
parser.add_argument("--out", type=Path, required=True)
args = parser.parse_args()
require(re.fullmatch("[0-9a-f]{40}", args.sha), "full SHA required")
require(os.environ.get("GITHUB_SHA") == args.sha, "workflow/source SHA mismatch")
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
processes = []
cases = []
seals = {}
originals = {}
providers = []
dependency_roots = []
tools = {}
artifact = None
error = None
status = 2
environment = os.environ.copy()
try:
    executor = BASE / "runtime-diagnostics-exec.py"
    roster_path = BASE / "lsp-navigation-unit-inputs.json"
    require(sha(executor) == EXECUTOR_SHA, "process owner pin changed")
    require(sha(roster_path) == ROSTER_SHA, "navigation unit roster pin changed")
    roster = json.loads(roster_path.read_text())
    require(
        roster["executor_sha256"] == EXECUTOR_SHA
        and len(roster["selectors"]) == 1
        and len(set(roster["selectors"])) == 1,
        "navigation unit roster mismatch",
    )
    require(
        all(sha(root / n) == pin for n, pin in roster["source_pins"].items()),
        "reviewed structural source changed",
    )
    spec = importlib.util.spec_from_file_location("navigation_unit_owner", executor)
    require(spec is not None and spec.loader is not None, "owner loader absent")
    owner = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(owner)
    rustup = shutil.which("rustup")
    require(rustup is not None, "rustup absent")
    tools = {"python": Path(sys.executable).resolve(), "executor": executor}
    for name in ["cargo", "rustc"]:
        tools[name] = Path(
            subprocess.check_output([rustup, "which", name], text=True).strip()
        ).resolve()
    for name in ["cc", "ar", "ld"]:
        found = shutil.which(name)
        require(found is not None, "native tool absent: " + name)
        tools[name] = Path(found).resolve()
    overrides = [
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
    ]
    require(not any(environment.get(n) for n in overrides), "unexpected build override")
    require(
        not any(
            n.startswith("CARGO_TARGET_") and n.endswith("_RUSTFLAGS") and v
            for n, v in environment.items()
        ),
        "target Rust flag override",
    )
    require(environment.get("RUSTFLAGS") == "-Dwarnings", "strict warnings required")
    environment["RUSTC"] = str(tools["rustc"])
    sysroot = Path(
        subprocess.check_output(
            [str(tools["rustc"]), "--print", "sysroot"], text=True
        ).strip()
    )
    providers = [Path(environment["TCL_TOMMATH_DIR"]).parent, sysroot / "lib"]
    require((providers[0] / "libtommath/tommath.h").is_file(), "full TomMath absent")
    originals = {
        "source": source(),
        "providers": files(providers),
        "tools": files(list(tools.values())),
    }
    for name, value in originals.items():
        save(out / (name + "-before.json"), value)
    save(
        out / "job.json",
        {
            "requested_sha": args.sha,
            "job": {
                n: environment.get(n)
                for n in [
                    "GITHUB_RUN_ID",
                    "GITHUB_RUN_ATTEMPT",
                    "GITHUB_JOB",
                    "GITHUB_WORKFLOW_REF",
                ]
            },
            "limits": {
                "fetch_seconds": 600,
                "build_seconds": 2400,
                "each_list_or_test_seconds": 180,
                "aggregate_log_bytes": CAP,
                "disk_floor_bytes": FLOOR,
            },
            "roster": roster,
        },
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
    )
    require(fetched["exit_code"] == 0, "dependency fetch failed")
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
    require(bool(dependency_roots), "fetched dependency inputs absent")
    originals["dependencies"] = files(dependency_roots)
    save(out / "dependencies-before.json", originals["dependencies"])
    built = execute(
        [
            str(tools["cargo"]),
            "test",
            "--locked",
            "--profile",
            "ci",
            "-p",
            "tcl-lsp-server",
            "--lib",
            "--no-run",
            "--message-format=json-render-diagnostics",
            "-j",
            "2",
        ],
        "build",
        tools["cargo"],
        2400,
    )
    require(built["exit_code"] == 0, "fresh server libtest build failed")
    rows = [
        json.loads(line)
        for line in (out / "build.stdout").read_text().splitlines()
        if line.startswith("{")
    ]
    require(
        not any(
            r.get("reason") == "compiler-message"
            and r.get("message", {}).get("level") in ["warning", "error"]
            for r in rows
        ),
        "Rust diagnostic observed",
    )
    require(
        sum(
            r.get("reason") == "build-finished" and r.get("success") is True
            for r in rows
        )
        == 1,
        "Cargo success terminal absent",
    )
    selected = [
        r
        for r in rows
        if r.get("reason") == "compiler-artifact"
        and r.get("target", {}).get("name") == roster["target"]
        and r.get("target", {}).get("kind") == ["lib"]
        and r.get("manifest_path") == str(root / "rust/tcl-lsp-server/Cargo.toml")
        and r.get("profile", {}).get("test") is True
        and r.get("executable")
    ]
    require(
        len(selected) == 1
        and selected[0].get("fresh") is False
        and str(selected[0].get("profile", {}).get("opt_level")) == "3",
        "fresh CI server libtest event absent/ambiguous",
    )
    executable = Path(selected[0]["executable"]).resolve()
    require(
        executable.is_relative_to(target) and executable.is_file(),
        "executable outside fresh target",
    )
    with executable.open("rb") as stream:
        require(stream.read(4) == b"\x7fELF", "native ELF required")
    artifact = {
        "path": str(executable),
        "sha256": sha(executable),
        "cargo_event": selected[0],
    }
    save(out / "server-artifact.json", artifact)
    require(
        source() == originals["source"]
        and files(providers) == originals["providers"]
        and files(dependency_roots) == originals["dependencies"]
        and files(list(tools.values())) == originals["tools"],
        "pre-test input seals changed",
    )
    for index, selector in enumerate(roster["selectors"]):
        require(sha(executable) == artifact["sha256"], "selected executable changed")
        prefix = "navigation-unit-" + str(index + 1)
        listed = execute(
            [str(executable), selector, "--exact", "--list", "--format", "terse"],
            prefix + "-list",
            executable,
            180,
        )
        require(
            listed["exit_code"] == 0
            and (out / (prefix + "-list.stdout")).read_text().splitlines()
            == [selector + ": test"],
            "exact selected test missing/ambiguous",
        )
        receipt = execute(
            [str(executable), selector, "--exact", "--nocapture", "--test-threads=1"],
            prefix,
            executable,
            180,
        )
        result = original_result(
            (out / (prefix + ".stdout")).read_text(), receipt["exit_code"]
        )
        cases.append(
            {"selector": selector, "result": result, "exit_code": receipt["exit_code"]}
        )
        save(out / "cases.json", cases)
    status = 0 if len(cases) == 1 and all(c["result"] == "PASS" for c in cases) else 1
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
    print("PUBLIC NAVIGATION UNIT STOP " + error, file=sys.stderr, flush=True)
finally:
    captures = {
        "source": source,
        "providers": lambda: files(providers),
        "tools": lambda: files(list(tools.values())),
        "dependencies": lambda: files(dependency_roots),
    }
    for name, capture in captures.items():
        try:
            current = capture()
            save(out / (name + "-after.json"), current)
            seals[name] = name in originals and current == originals[name]
        except (
            RuntimeError,
            OSError,
            ValueError,
            subprocess.SubprocessError,
            KeyError,
            TypeError,
        ):
            seals[name] = False
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
            "seals": seals,
            "cases": cases,
            "separate_supplemental_total": len(cases),
            "processes": processes,
            "remaining_log_bytes": remaining,
            "startup_acceptance": False,
            "performance_admission": False,
        },
    )
sys.exit(status)
