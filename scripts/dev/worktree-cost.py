#!/usr/bin/env python3
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
# SPDX-License-Identifier: AGPL-3.0-or-later
"""What one isolated edit costs a worktree.

For each edit in EDITS the script appends `const _: u32 = <n>;` to one file
and measures three commands before and after the edit: the test build
(`cargo test -p <crate> --no-run`), the lint build (`cargo clippy -p <crate>
--all-targets`) and the test run (`cargo test -p <crate>`). It counts the
units cargo compiled from its JSON messages, times each command, samples the
summed RSS of the command's process tree from /proc and sizes the target
directory. Design: docs/design/build-time/259-worktree-cost.md.

    worktree-cost.py run [--edits analyser,registry] [--no-test-run] [--no-ra]
                         [--commands test_build,clippy] [--phases cold,after]
                         [--jobs N] [--in-place] [--out FILE] [--merge]
    worktree-cost.py table   RUN.json
    worktree-cost.py compare BEFORE.json AFTER.json

`run` uses a fresh detached worktree with an empty target directory unless
`--in-place` is given, which measures the current checkout and its warm target
directory instead (the "cold" phase is then "warm"). `--merge` adds the edits
of this run to an existing `--out` document, so a long baseline can be taken
one edit, command or phase at a time; a merged run keeps the measurements of
the earlier ones.
"""

from __future__ import annotations

import argparse
import json
import os
import platform
import select
import shutil
import signal
import subprocess
import sys
import tempfile
import threading
import time
from dataclasses import dataclass
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
COMMANDS = ("test_build", "clippy", "test_run")
SAMPLE_INTERVAL_S = 0.2
WORKTREE_FREE_BYTES = 40 << 30
IN_PLACE_FREE_BYTES = 5 << 30
RA_TIMEOUT_S = 20 * 60


@dataclass(frozen=True)
class Edit:
    key: str
    file: str
    crate: str
    # The first commit the row holds from; a step that moves the file adds a
    # new row and closes this one.
    since: str


EDITS: tuple[Edit, ...] = (
    Edit("lexer", "rust/tcl-syntax/src/glob.rs", "tcl-syntax", "f9694053"),
    Edit(
        "registry",
        "rust/tcl-registry/src/commands/irules/active_members.rs",
        "tcl-registry",
        "f9694053",
    ),
    Edit(
        "analyser",
        "rust/tcl-compiler/src/analyser/diagnostics/widget_command.rs",
        "tcl-compiler",
        "f9694053",
    ),
    Edit("lsp", "rust/tcl-lsp-core/src/folding.rs", "tcl-lsp-core", "f9694053"),
)


class CostError(RuntimeError):
    pass


# ---------------------------------------------------------------- environment


def expected_env(root: Path) -> dict[str, str | None]:
    """The variables agent-build-env.sh --print sets (None: unset) for `root`."""
    out = subprocess.run(
        ["bash", str(root / "scripts/dev/agent-build-env.sh"), "--print"],
        cwd=root,
        check=True,
        capture_output=True,
        text=True,
    ).stdout
    return parse_env_print(out)


def parse_env_print(text: str) -> dict[str, str | None]:
    env: dict[str, str | None] = {}
    for line in text.splitlines():
        line = line.strip()
        if line.startswith("export ") and "=" in line:
            name, value = line[len("export ") :].split("=", 1)
            env[name] = value.strip("'\"")
        elif line.startswith("unset "):
            env[line[len("unset ") :].strip()] = None
    return env


def env_problems(
    expected: dict[str, str | None], actual: dict[str, str] | os._Environ[str]
) -> list[str]:
    problems = []
    for name, value in expected.items():
        have = actual.get(name)
        if have != value:
            if name == "CARGO_TARGET_DIR":
                problems.append(
                    f"CARGO_TARGET_DIR is {have!r}, not this worktree's own {value!r}"
                )
            elif value is None:
                problems.append(f"{name} is set to {have!r}; agent-build-env unsets it")
            else:
                problems.append(f"{name} is {have!r}; agent-build-env sets {value!r}")
    return problems


# What cargo reads. Another session's documentation edits in the same checkout
# do not change a build, so only these paths must be clean.
BUILD_INPUTS = ("rust", "Cargo.toml", "Cargo.lock", ".cargo", "rust-toolchain.toml")


def dirty(root: Path) -> list[str]:
    out = subprocess.run(
        ["git", "status", "--porcelain", "--", *BUILD_INPUTS],
        cwd=root,
        check=True,
        capture_output=True,
        text=True,
    ).stdout
    return [line for line in out.splitlines() if line.strip()]


def build_inputs_id(root: Path) -> str:
    """The tree ids of HEAD's build inputs: equal ids build the same thing."""
    ids = []
    for path in BUILD_INPUTS:
        done = subprocess.run(
            ["git", "rev-parse", "--short=12", f"HEAD:{path}"],
            cwd=root, capture_output=True, text=True,
        )
        ids.append(done.stdout.strip() if done.returncode == 0 else "-")
    return " ".join(ids)


def busy_builders(proc_root: Path = Path("/proc")) -> list[str]:
    mine = os.getpid()
    found = []
    for entry in proc_root.iterdir():
        if not entry.name.isdigit() or int(entry.name) == mine:
            continue
        try:
            comm = (entry / "comm").read_text().strip()
        except OSError:
            continue
        if comm in ("cargo", "rustc", "clippy-driver"):
            found.append(f"{comm} (pid {entry.name})")
    return found


def preflight(
    root: Path, edits: list[Edit], environ, need_free: int, check_busy: bool = True
) -> None:
    """Raise CostError naming every reason the run must not start."""
    problems = []
    for edit in edits:
        if not (root / edit.file).is_file():
            problems.append(f"edit {edit.key}: {edit.file} does not exist")
    if changes := dirty(root):
        problems.append(f"the build inputs are not clean ({len(changes)} changed paths)")
    problems += env_problems(expected_env(root), environ)
    if check_busy and (others := busy_builders()):
        problems.append("another build is running: " + ", ".join(others))
    free = shutil.disk_usage(root).free
    if free < need_free:
        problems.append(
            f"{free >> 30} GB free, the run needs {need_free >> 30} GB"
        )
    if problems:
        raise CostError("; ".join(problems))


# ---------------------------------------------------------------- edits


def apply_edit(root: Path, edit: Edit, n: int) -> None:
    path = root / edit.file
    text = path.read_text()
    if not text.endswith("\n"):
        text += "\n"
    path.write_text(f"{text}const _: u32 = {n};\n")


def restore_edit(root: Path, edit: Edit) -> None:
    subprocess.run(["git", "checkout", "--", edit.file], cwd=root, check=True)


# ---------------------------------------------------------------- cargo JSON


def build_script_ran_since(out_dir: str, started: float) -> bool | None:
    """Whether the build script owning `out_dir` ran at or after `started`.

    Cargo reports `build-script-executed` for a fresh script too, replaying
    its recorded output, so the message alone does not say it ran. A run
    rewrites the unit's run record: `run/invoked.timestamp` and `run/stdout`
    beside `out/` in cargo's build-dir layout, `output` in the older one.
    None when no record is found.
    """
    unit = Path(out_dir).parent
    times = []
    for record in ("run/invoked.timestamp", "run/stdout", "output"):
        try:
            times.append((unit / record).stat().st_mtime)
        except OSError:
            continue
    if not times:
        return None
    return max(times) >= started


def parse_messages(lines, started: float | None = None) -> dict:
    """Count compiled units in cargo's JSON message stream."""
    workspace = registry = 0
    by_profile: dict[str, int] = {}
    features: dict[str, set[tuple[str, ...]]] = {}
    build_scripts: dict[str, int] = {}
    replayed: dict[str, int] = {}
    unknown: dict[str, int] = {}
    compiled: list[str] = []
    for line in lines:
        line = line.strip()
        if not line.startswith("{"):
            continue
        try:
            msg = json.loads(line)
        except json.JSONDecodeError:
            continue
        reason = msg.get("reason")
        package_id = msg.get("package_id", "")
        name = package_name(package_id)
        if reason == "build-script-executed":
            ran = (
                None
                if started is None or not msg.get("out_dir")
                else build_script_ran_since(msg["out_dir"], started)
            )
            bucket = build_scripts if ran else replayed if ran is False else unknown
            bucket[name] = bucket.get(name, 0) + 1
            continue
        if reason != "compiler-artifact":
            continue
        if package_id.startswith("path+file://"):
            features.setdefault(name, set()).add(tuple(msg.get("features", [])))
        if msg.get("fresh", True):
            continue
        if package_id.startswith("path+file://"):
            workspace += 1
            compiled.append(name)
        else:
            registry += 1
        profile = msg.get("profile", {})
        filenames = msg.get("filenames", [])
        check = bool(filenames) and all(f.endswith(".rmeta") for f in filenames)
        key = f"opt{profile.get('opt_level', '?')}" + (
            "-test" if profile.get("test") else ""
        ) + ("-check" if check else "")
        by_profile[key] = by_profile.get(key, 0) + 1
    return {
        "units_compiled": {
            "workspace": workspace,
            "third_party": registry,
            "total": workspace + registry,
        },
        "workspace_compiled": sorted(set(compiled)),
        "units_by_profile": dict(sorted(by_profile.items())),
        "features": {k: sorted(map(list, v)) for k, v in sorted(features.items())},
        # Reruns only; a fresh script's replayed output is not a run.
        "build_scripts_run": dict(sorted(build_scripts.items())),
        "build_scripts_replayed": dict(sorted(replayed.items())),
        "build_scripts_unknown": dict(sorted(unknown.items())),
    }


def package_name(package_id: str) -> str:
    """`path+file:///x/rust/foo#0.1.0` or `registry+…#foo@1.2` → `foo`."""
    tail = package_id.rsplit("#", 1)[-1]
    if "@" in tail:
        return tail.split("@", 1)[0]
    head = package_id.split("#", 1)[0]
    return head.rstrip("/").rsplit("/", 1)[-1]


def feature_variants(per_command: list[dict]) -> dict[str, int]:
    merged: dict[str, set[tuple[str, ...]]] = {}
    for result in per_command:
        for name, lists in result.get("features", {}).items():
            merged.setdefault(name, set()).update(tuple(x) for x in lists)
    return {name: len(v) for name, v in sorted(merged.items())}


# ---------------------------------------------------------------- /proc sampling


def children_of(pid: int, proc_root: Path) -> list[int]:
    task_dir = proc_root / str(pid) / "task"
    kids: list[int] = []
    try:
        tasks = list(task_dir.iterdir())
    except OSError:
        tasks = []
    have_children_file = False
    for task in tasks:
        try:
            text = (task / "children").read_text()
        except OSError:
            continue
        have_children_file = True
        kids += [int(x) for x in text.split()]
    if have_children_file:
        return kids
    for entry in proc_root.iterdir():
        if not entry.name.isdigit():
            continue
        try:
            stat = (entry / "stat").read_text()
        except OSError:
            continue
        # The command name may hold spaces and parentheses; ppid follows the last ')'.
        fields = stat.rsplit(")", 1)[-1].split()
        if len(fields) > 1 and fields[1] == str(pid):
            kids.append(int(entry.name))
    return kids


def rss_kb(pid: int, proc_root: Path) -> int | None:
    try:
        for line in (proc_root / str(pid) / "status").read_text().splitlines():
            if line.startswith("VmRSS:"):
                return int(line.split()[1])
    except (OSError, ValueError, IndexError):
        return None
    return 0


def argv0(pid: int, proc_root: Path) -> str:
    try:
        return (proc_root / str(pid) / "cmdline").read_bytes().split(b"\0", 1)[0].decode()
    except OSError:
        return ""


def sample_tree(root_pid: int, proc_root: Path = Path("/proc"), match=None) -> int:
    """Summed VmRSS in kB of `root_pid` and its descendants (filtered by `match`)."""
    total = 0
    stack, seen = [root_pid], set()
    while stack:
        pid = stack.pop()
        if pid in seen:
            continue
        seen.add(pid)
        kb = rss_kb(pid, proc_root)
        if kb is None:
            continue  # exited between listing and reading
        if match is None or match(argv0(pid, proc_root)):
            total += kb
        stack += children_of(pid, proc_root)
    return total


class Sampler(threading.Thread):
    def __init__(self, pid: int, match=None, proc_root: Path = Path("/proc")):
        super().__init__(daemon=True)
        self.pid, self.match, self.proc_root = pid, match, proc_root
        self.peak_kb = 0
        self.peak_match_kb = 0
        self._halt = threading.Event()

    def run(self) -> None:
        while not self._halt.is_set():
            self.peak_kb = max(self.peak_kb, sample_tree(self.pid, self.proc_root))
            if self.match is not None:
                self.peak_match_kb = max(
                    self.peak_match_kb, sample_tree(self.pid, self.proc_root, self.match)
                )
            self._halt.wait(SAMPLE_INTERVAL_S)

    def stop(self) -> None:
        self._halt.set()
        self.join()


# ---------------------------------------------------------------- target sizing


def allocated_bytes(path: Path) -> dict[str, int]:
    """Allocated size under `path`, hard links once, split by top-level kind."""
    seen: set[tuple[int, int]] = set()
    split = {"total": 0, "build": 0, "incremental": 0, "deps": 0, "tests": 0, "other": 0}

    def kind(rel: Path) -> str:
        parts = rel.parts
        if "incremental" in parts:
            return "incremental"
        if "build" in parts:
            return "build"
        if "deps" in parts:
            return "deps"
        return "other"

    if not path.exists():
        return split
    for dirpath, _dirs, files in os.walk(path):
        for name in files:
            full = Path(dirpath) / name
            try:
                st = full.lstat()
            except OSError:
                continue
            ident = (st.st_dev, st.st_ino)
            if ident in seen:
                continue
            seen.add(ident)
            size = st.st_blocks * 512
            split["total"] += size
            k = kind(full.relative_to(path))
            if k == "deps" and os.access(full, os.X_OK) and "." not in name:
                k = "tests"
            split[k] += size
    return split


def variants_on_disk(target: Path, packages: list[str]) -> dict[str, int]:
    build = target / "debug" / "build"
    out = {}
    for pkg in packages:
        d = build / pkg
        if d.is_dir():
            out[pkg] = sum(1 for e in d.iterdir() if e.is_dir())
    return out


# ---------------------------------------------------------------- commands


def run_measured(argv: list[str], cwd: Path, env: dict, target: Path, harness=None) -> dict:
    """Run one cargo command, returning its measurements."""
    before = allocated_bytes(target)
    load = os.getloadavg()[0]
    started_wall = time.time()
    start = time.monotonic()
    proc = subprocess.Popen(
        argv, cwd=cwd, env=env, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True
    )
    sampler = Sampler(proc.pid, harness)
    sampler.start()
    assert proc.stdout is not None
    try:
        lines = list(proc.stdout)
        _, status, usage = os.wait4(proc.pid, 0)
    except BaseException:
        proc.kill()
        proc.wait()
        sampler.stop()
        raise
    proc.returncode = os.waitstatus_to_exitcode(status)
    wall = time.monotonic() - start
    sampler.stop()
    after = allocated_bytes(target)
    result = parse_messages(lines, started_wall)
    result.update(
        {
            "argv": argv,
            "exit": proc.returncode,
            "wall_s": round(wall, 1),
            "load_1m": round(load, 2),
            "peak_rss_tree_mb": round(sampler.peak_kb / 1024),
            # Linux reports the largest of the child and its waited descendants.
            "peak_rss_single_mb": round(usage.ru_maxrss / 1024),
            "target_bytes_before": before,
            "target_bytes_after": after,
            "target_growth_bytes": after["total"] - before["total"],
        }
    )
    if harness is not None:
        result["test_peak_rss_mb"] = round(sampler.peak_match_kb / 1024)
    return result


def command_argv(kind: str, crate: str, jobs: int | None) -> list[str]:
    jobs_arg = [f"-j{jobs}"] if jobs else []
    fmt = ["--message-format=json-render-diagnostics"]
    if kind == "test_build":
        return ["cargo", "test", "-p", crate, "--no-run", *jobs_arg, *fmt]
    if kind == "clippy":
        return ["cargo", "clippy", "-p", crate, "--all-targets", *jobs_arg, *fmt]
    return ["cargo", "test", "-p", crate, *jobs_arg, *fmt]


def harness_matcher(target: Path, crate: str):
    prefix = str(target / "debug" / "deps" / crate.replace("-", "_")) + "-"
    return lambda exe: exe.startswith(prefix)


# ---------------------------------------------------------------- rust-analyzer


def measure_rust_analyzer(root: Path, binary: str | None = None) -> dict:
    binary = binary or shutil.which("rust-analyzer")
    if binary is None:
        return {"ra_rss_mb": None, "reason": "rust-analyzer is not on PATH"}
    probe = subprocess.run([binary, "--version"], capture_output=True, text=True)
    if probe.returncode != 0:
        reason = (probe.stderr or probe.stdout).strip().splitlines()
        return {
            "ra_rss_mb": None,
            "reason": "rust-analyzer does not run: " + (reason[0] if reason else "no output"),
        }
    proc = subprocess.Popen(
        [binary], cwd=root, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
    )
    assert proc.stdin is not None and proc.stdout is not None

    def send(msg: dict) -> None:
        body = json.dumps(msg).encode()
        proc.stdin.write(b"Content-Length: %d\r\n\r\n" % len(body) + body)
        proc.stdin.flush()

    def receive(deadline: float) -> dict | None:
        header = b""
        while not header.endswith(b"\r\n\r\n"):
            if not select.select([proc.stdout], [], [], max(0, deadline - time.monotonic()))[0]:
                return None
            chunk = proc.stdout.read(1)
            if not chunk:
                return None
            header += chunk
        length = int(header.split(b"Content-Length:")[1].split(b"\r\n")[0])
        return json.loads(proc.stdout.read(length))

    send({
        "jsonrpc": "2.0", "id": 1, "method": "initialize",
        "params": {
            "processId": os.getpid(), "rootUri": root.as_uri(),
            "capabilities": {"window": {"workDoneProgress": True}},
            "initializationOptions": {"checkOnSave": False, "cargo": {"targetDir": True}},
        },
    })
    start = time.monotonic()
    deadline = start + RA_TIMEOUT_S
    open_tokens: set = set()
    seen_progress = timed_out = False
    peak = 0
    while True:
        msg = receive(min(deadline, time.monotonic() + 5))
        peak = max(peak, sample_tree(proc.pid))
        if time.monotonic() >= deadline:
            timed_out = True
            break
        if msg is None:
            if seen_progress and not open_tokens:
                break
            continue
        if msg.get("id") == 1 and "result" in msg:
            send({"jsonrpc": "2.0", "method": "initialized", "params": {}})
        elif "method" in msg and "id" in msg:
            send({"jsonrpc": "2.0", "id": msg["id"], "result": None})
        elif msg.get("method") == "$/progress":
            token, kind = msg["params"]["token"], msg["params"]["value"].get("kind")
            seen_progress = True
            if kind == "begin":
                open_tokens.add(token)
            elif kind == "end":
                open_tokens.discard(token)
    rss = sample_tree(proc.pid)
    try:
        send({"jsonrpc": "2.0", "id": 2, "method": "shutdown"})
        send({"jsonrpc": "2.0", "method": "exit"})
        proc.wait(timeout=30)
    except (OSError, subprocess.TimeoutExpired):
        proc.kill()
    ra_target = root / "target" / "rust-analyzer"
    return {
        "ra_rss_mb": round(rss / 1024),
        "ra_peak_rss_mb": round(peak / 1024),
        "index_s": round(time.monotonic() - start, 1),
        "timed_out": timed_out,
        "ra_target_bytes": allocated_bytes(ra_target)["total"],
    }


# ---------------------------------------------------------------- run


def machine_info(jobs: int | None) -> dict:
    def version(tool: str) -> str:
        try:
            return subprocess.run(
                [tool, "--version"], capture_output=True, text=True
            ).stdout.strip()
        except OSError:
            return "missing"

    mem_kb = 0
    try:
        for line in Path("/proc/meminfo").read_text().splitlines():
            if line.startswith("MemTotal:"):
                mem_kb = int(line.split()[1])
    except OSError:
        pass
    return {
        "nproc": os.cpu_count(),
        "mem_total_gb": round(mem_kb / (1 << 20), 1),
        "jobs": jobs,
        "rustc": version("rustc"),
        "cargo": version("cargo"),
        "python": platform.python_version(),
        "time_v": shutil.which("time") is not None and Path("/usr/bin/time").exists(),
    }


def workspace_packages(root: Path, env: dict) -> list[str]:
    out = subprocess.run(
        ["cargo", "metadata", "--no-deps", "--format-version", "1"],
        cwd=root, env=env, check=True, capture_output=True, text=True,
    ).stdout
    return sorted(p["name"] for p in json.loads(out)["packages"])


def measure_edit(
    root: Path, edit: Edit, n: int, args, env: dict, target: Path, record: dict | None = None
) -> dict:
    wanted = getattr(args, "commands", None) or ",".join(COMMANDS)
    kinds = [
        k for k in COMMANDS
        if k in wanted.split(",") and (k != "test_run" or not args.no_test_run)
    ]
    phases = (getattr(args, "phases", None) or "cold,after").split(",")
    record = record if record is not None else {}
    record.update({"crate": edit.crate, "file": edit.file})
    for phase in ("cold", "after"):
        if phase not in phases:
            continue
        if phase == "after":
            apply_edit(root, edit, n)
        try:
            record.setdefault(phase, {})
            for kind in kinds:
                harness = harness_matcher(target, edit.crate) if kind == "test_run" else None
                print(f"worktree-cost: {edit.key} {phase} {kind}", file=sys.stderr, flush=True)
                record[phase][kind] = run_measured(
                    command_argv(kind, edit.crate, args.jobs), root, env, target, harness
                )
        finally:
            if phase == "after":
                restore_edit(root, edit)
    return record


def run(args) -> int:
    keys = args.edits.split(",") if args.edits else [e.key for e in EDITS]
    table = {e.key: e for e in EDITS}
    unknown = [k for k in keys if k not in table]
    if unknown:
        raise CostError(f"unknown edits: {', '.join(unknown)}")
    edits = [table[k] for k in keys]
    commit = subprocess.run(
        ["git", "rev-parse", "--short=12", "HEAD"], cwd=REPO, check=True,
        capture_output=True, text=True,
    ).stdout.strip()
    inputs = build_inputs_id(REPO)
    preflight(
        REPO, edits, os.environ,
        IN_PLACE_FREE_BYTES if args.in_place else WORKTREE_FREE_BYTES,
    )

    scratch = None
    root = REPO
    if not args.in_place:
        scratch = Path(tempfile.mkdtemp(prefix="worktree-cost-"))
        root = scratch / "wt"
        subprocess.run(["git", "worktree", "add", "--detach", str(root), "HEAD"],
                       cwd=REPO, check=True)
    try:
        env = dict(os.environ)
        for name, value in expected_env(root).items():
            if value is None:
                env.pop(name, None)
            else:
                env[name] = value
        target = Path(env["CARGO_TARGET_DIR"])
        out = Path(args.out) if args.out else REPO / "target" / "worktree-cost" / f"{commit}.json"
        doc: dict = {}
        if args.merge and out.exists():
            doc = json.loads(out.read_text())
            if doc.get("inputs") != inputs:
                raise CostError(f"{out} measured other build inputs than HEAD ({commit})")
        doc.update({
            "commit": commit,
            "inputs": inputs,
            "mode": "in-place" if args.in_place else "fresh-worktree",
            "cold_means": "warm target of the checkout" if args.in_place else "empty target",
            "machine": machine_info(args.jobs),
            "env": {k: env.get(k) for k in expected_env(root)},
        })
        doc.setdefault("edits", {})
        doc.setdefault("skipped", {})
        def save() -> None:
            out.parent.mkdir(parents=True, exist_ok=True)
            out.write_text(json.dumps(doc, indent=1, sort_keys=True) + "\n")

        try:
            for n, edit in enumerate(edits, start=1):
                measure_edit(
                    root, edit, n, args, env, target, doc["edits"].setdefault(edit.key, {})
                )
                if args.no_test_run:
                    doc["skipped"][f"{edit.key}.test_run"] = "--no-test-run"
            packages = workspace_packages(root, env)
            doc["variants"] = variants_on_disk(target, packages)
            doc["feature_variants"] = feature_variants(
                [
                    c
                    for e in doc["edits"].values()
                    for p in ("cold", "after")
                    for c in e.get(p, {}).values()
                ]
            )
            if not args.no_ra or "ra" not in doc:
                doc["ra"] = (
                    {"ra_rss_mb": None, "reason": "--no-ra"} if args.no_ra
                    else measure_rust_analyzer(root)
                )
        finally:
            # A partial run keeps what it measured; --merge continues it.
            save()
        print(out)
    finally:
        if scratch is not None:
            subprocess.run(["git", "worktree", "remove", "--force", str(root)], cwd=REPO)
            shutil.rmtree(scratch, ignore_errors=True)
    return 0


# ---------------------------------------------------------------- reports


def _rows(doc: dict):
    for key, record in doc.get("edits", {}).items():
        for phase in ("cold", "after"):
            for kind, r in record.get(phase, {}).items():
                yield (key, record["crate"], phase, kind), r


def _cells(r: dict) -> list:
    u = r["units_compiled"]
    return [
        u["workspace"], u["total"], r["wall_s"], r["peak_rss_tree_mb"],
        r["peak_rss_single_mb"], round(r["target_growth_bytes"] / (1 << 20)),
        r.get("test_peak_rss_mb", "–"),
    ]


HEADER = (
    "| Edit | Crate | Phase | Command | Workspace units | Total units | Wall (s) "
    "| Peak RSS tree (MB) | Peak RSS single (MB) | Target growth (MB) | Test run RSS (MB) |\n"
    "|---|---|---|---|---:|---:|---:|---:|---:|---:|---:|"
)


def table(doc: dict) -> str:
    lines = [HEADER]
    for (key, crate, phase, kind), r in _rows(doc):
        cells = [key, f"`{crate}`", phase, kind, *_cells(r)]
        lines.append("| " + " | ".join(str(c) for c in cells) + " |")
    return "\n".join(lines)


def _signed(a, b) -> str:
    if not isinstance(a, (int, float)) or not isinstance(b, (int, float)):
        return "–"
    d = round(b - a, 1)
    if isinstance(d, float) and d.is_integer():
        d = int(d)
    return f"{d:+}"


def compare(before: dict, after: dict) -> str:
    old = dict(_rows(before))
    lines = [HEADER]
    for row, r in _rows(after):
        if row not in old:
            continue
        deltas = [_signed(a, b) for a, b in zip(_cells(old[row]), _cells(r))]
        key, crate, phase, kind = row
        lines.append("| " + " | ".join([key, f"`{crate}`", phase, kind, *deltas]) + " |")
    return "\n".join(lines)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    sub = parser.add_subparsers(dest="cmd", required=True)
    p_run = sub.add_parser("run")
    p_run.add_argument("--edits")
    p_run.add_argument("--no-test-run", action="store_true")
    p_run.add_argument("--commands", help="subset of test_build,clippy,test_run")
    p_run.add_argument("--phases", help="subset of cold,after")
    p_run.add_argument("--no-ra", action="store_true")
    p_run.add_argument("--jobs", type=int)
    p_run.add_argument("--in-place", action="store_true")
    p_run.add_argument("--merge", action="store_true")
    p_run.add_argument("--out")
    p_table = sub.add_parser("table")
    p_table.add_argument("run_json")
    p_cmp = sub.add_parser("compare")
    p_cmp.add_argument("before")
    p_cmp.add_argument("after")
    args = parser.parse_args(argv)
    # A `timeout` or Ctrl-C still restores the edited file and stops cargo.
    signal.signal(signal.SIGTERM, lambda *_: sys.exit(143))
    try:
        if args.cmd == "run":
            return run(args)
        if args.cmd == "table":
            print(table(json.loads(Path(args.run_json).read_text())))
            return 0
        print(compare(json.loads(Path(args.before).read_text()),
                      json.loads(Path(args.after).read_text())))
        return 0
    except CostError as err:
        print(f"worktree-cost: {err}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
