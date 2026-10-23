#!/usr/bin/env python3
"""Run one already-built exact selector with retained process authority.

The caller owns Cargo, source/provider guards, selector inventory, and the
overall suite receipt. This module never searches for or reuses an artifact.
"""

from __future__ import annotations

import datetime
import hashlib
import json
import os
from pathlib import Path
import selectors
import shutil
import signal
import subprocess
import time
from typing import Mapping, Sequence


def _utc() -> str:
    return datetime.datetime.now(datetime.timezone.utc).isoformat()


def _sha256(path: Path) -> str:
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def _start_ticks(pid: int) -> int:
    raw = Path(f"/proc/{pid}/stat").read_text()
    # comm is parenthesized and may itself contain spaces or parentheses.
    fields_after_comm = raw[raw.rfind(")") + 2 :].split()
    return int(fields_after_comm[19])  # Linux field 22: process starttime.


def _free_bytes(paths: Sequence[Path]) -> int:
    free = []
    for requested in paths:
        path = requested
        while not path.exists():
            parent = path.parent
            if parent == path:
                raise RuntimeError(f"no existing disk ancestor: {requested}")
            path = parent
        free.append(shutil.disk_usage(path).free)
    return min(free)


def _write_receipt(path: Path, receipt: dict) -> None:
    temporary = path.with_name(path.name + ".writing")
    temporary.write_text(json.dumps(receipt, indent=2, sort_keys=True) + "\n")
    os.replace(temporary, path)


def _parent_observation(pid: int) -> dict | None:
    info = os.waitid(os.P_PID, pid, os.WEXITED | os.WNOHANG | os.WNOWAIT)
    if info is None:
        return None
    return {"si_pid": info.si_pid, "si_code": info.si_code, "si_status": info.si_status}


def _group_scan(group: int) -> dict:
    """Find live members; a disappearing PID makes this scan inconclusive."""
    members = []
    vanished = []
    for entry in Path("/proc").iterdir():
        if not entry.name.isdecimal():
            continue
        try:
            raw = (entry / "stat").read_text()
        except FileNotFoundError:
            # It could have been a group member that forked a replacement.
            vanished.append(int(entry.name))
            continue
        fields = raw[raw.rfind(")") + 2 :].split()
        if int(fields[2]) == group and fields[0] not in {"Z", "X", "x"}:
            members.append({"pid": int(entry.name), "start_ticks": int(fields[19]), "state": fields[0]})
    return {"members": members, "vanished_pids": vanished, "complete": not vanished}


def run(
    argv: Sequence[str],
    *,
    cwd: str | Path,
    env: Mapping[str, str],
    stdout_path: str | Path,
    stderr_path: str | Path,
    receipt_path: str | Path,
    artifact_path: str | Path,
    deadline_seconds: float = 180,
    cap_bytes: int = 20 * 1024**2,
    disk_floor_bytes: int = 2 * 1024**3,
    disk_paths: Sequence[str | Path] = ("/workspace", "/tmp"),
) -> dict:
    """Execute argv directly; return a durable, fail-closed receipt.

    A nonzero test result is retained as data. The suite decides whether its
    exact assertion passed. A guard stop or incomplete cleanup is always bad.
    """
    if not argv or not all(isinstance(item, str) and item for item in argv):
        raise ValueError("nonempty explicit argv required")
    if deadline_seconds <= 0 or cap_bytes <= 0 or disk_floor_bytes <= 0:
        raise ValueError("positive deadline/cap/disk floor required")
    stdout_path = Path(stdout_path)
    stderr_path = Path(stderr_path)
    receipt_path = Path(receipt_path)
    artifact_path = Path(artifact_path)
    if len({stdout_path, stderr_path, receipt_path}) != 3:
        raise ValueError("stdout, stderr, and receipt paths must differ")
    for path in (stdout_path, stderr_path, receipt_path):
        path.parent.mkdir(parents=True, exist_ok=True)
        if path.exists():
            raise FileExistsError(path)
    if not artifact_path.is_file() or Path(argv[0]).resolve() != artifact_path.resolve():
        raise ValueError("argv[0] must be the explicit existing artifact")

    observed_paths = tuple(Path(p) for p in disk_paths) + (
        artifact_path,
        stdout_path.parent,
        stderr_path.parent,
        receipt_path.parent,
    )
    first_free = _free_bytes(observed_paths)
    if first_free < disk_floor_bytes:
        raise RuntimeError("disk below floor before selector spawn")
    artifact_before = _sha256(artifact_path)
    receipt: dict = {
        "argv": list(argv),
        "cwd": str(Path(cwd)),
        "artifact": str(artifact_path),
        "artifact_sha256_before": artifact_before,
        "artifact_sha256_after": None,
        "stdout": str(stdout_path),
        "stderr": str(stderr_path),
        "deadline_seconds": deadline_seconds,
        "cap_bytes": cap_bytes,
        "disk_floor_bytes": disk_floor_bytes,
        "disk_paths": [str(p) for p in observed_paths],
        "minimum_free_bytes": first_free,
        "timer_armed_before_spawn": False,
        "pid": None,
        "pid_start_ticks": None,
        "process_group": None,
        "parent_waitid_wnowait": None,
        "owned_live_group_members": None,
        "last_group_scan": None,
        "group_scan_attempts": 0,
        "consecutive_complete_empty_group_scans": 0,
        "group_empty_before_parent_reap": False,
        "exit_code": None,
        "stop_reason": None,
        "cleanup_signals": [],
        "stdout_bytes": 0,
        "stderr_bytes": 0,
        "retained_bytes": 0,
        "complete": False,
    }
    poller = selectors.DefaultSelector()
    proc: subprocess.Popen | None = None
    streams: dict[str, object] = {}
    stop_at_ns: int | None = None
    signal_stage = 0
    parent_exit_ns: int | None = None
    started_ns = time.monotonic_ns()
    receipt["timer_armed_before_spawn"] = True
    receipt["timer_armed_monotonic_ns"] = started_ns
    receipt["timer_armed_utc"] = _utc()
    _write_receipt(receipt_path, receipt)
    try:
        with stdout_path.open("xb") as stdout, stderr_path.open("xb") as stderr:
            streams = {"stdout": stdout, "stderr": stderr}
            proc = subprocess.Popen(
                list(argv),
                cwd=cwd,
                env=dict(env),
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                start_new_session=True,
            )
            receipt["pid"] = proc.pid
            receipt["pid_start_ticks"] = _start_ticks(proc.pid)
            receipt["process_group"] = os.getpgid(proc.pid)
            if receipt["process_group"] != proc.pid:
                raise RuntimeError("spawned process is not its own group leader")
            receipt["spawn_return_monotonic_ns"] = time.monotonic_ns()
            receipt["spawn_return_utc"] = _utc()
            _write_receipt(receipt_path, receipt)
            assert proc.stdout is not None and proc.stderr is not None
            poller.register(proc.stdout, selectors.EVENT_READ, "stdout")
            poller.register(proc.stderr, selectors.EVENT_READ, "stderr")

            while True:
                now_ns = time.monotonic_ns()
                free = _free_bytes(observed_paths)
                receipt["minimum_free_bytes"] = min(receipt["minimum_free_bytes"], free)
                parent = _parent_observation(proc.pid)
                if parent is not None:
                    receipt["parent_waitid_wnowait"] = parent
                    if parent_exit_ns is None:
                        parent_exit_ns = now_ns
                        receipt["parent_exit_observed_monotonic_ns"] = now_ns
                    scan = _group_scan(proc.pid)
                    receipt["last_group_scan"] = scan
                    receipt["group_scan_attempts"] += 1
                    receipt["owned_live_group_members"] = scan["members"]
                    if scan["complete"] and not scan["members"]:
                        receipt["consecutive_complete_empty_group_scans"] += 1
                    else:
                        receipt["consecutive_complete_empty_group_scans"] = 0
                if receipt["stop_reason"] is None:
                    if free < disk_floor_bytes:
                        receipt["stop_reason"] = "disk_below_floor"
                    elif now_ns - started_ns >= int(deadline_seconds * 1e9):
                        receipt["stop_reason"] = "selector_timeout"
                    elif parent is not None and not poller.get_map() and receipt["owned_live_group_members"]:
                        receipt["stop_reason"] = "owned_group_live_after_parent_exit"
                    elif parent_exit_ns is not None and poller.get_map() and now_ns - parent_exit_ns >= 2_000_000_000:
                        receipt["stop_reason"] = "pipe_holder_after_parent_exit"
                if receipt["stop_reason"] is not None and stop_at_ns is None:
                    stop_at_ns = now_ns
                if stop_at_ns is not None:
                    elapsed = now_ns - stop_at_ns
                    desired = 3 if elapsed >= 10_000_000_000 else 2 if elapsed >= 5_000_000_000 else 1
                    if desired > signal_stage:
                        # WNOWAIT retains the exact parent PID. Also verify
                        # its start ticks before every group signal.
                        if _start_ticks(proc.pid) != receipt["pid_start_ticks"]:
                            raise RuntimeError("owned parent identity changed before group signal")
                        sig = (signal.SIGINT, signal.SIGTERM, signal.SIGKILL)[desired - 1]
                        try:
                            os.killpg(proc.pid, sig)
                            receipt["cleanup_signals"].append({"signal": sig.name, "monotonic_ns": now_ns})
                        except ProcessLookupError:
                            receipt["cleanup_signals"].append({"signal": sig.name, "group_absent": True, "monotonic_ns": now_ns})
                        signal_stage = desired
                if parent is not None and not poller.get_map() and receipt["consecutive_complete_empty_group_scans"] >= 2:
                    receipt["group_empty_before_parent_reap"] = True
                    break
                if stop_at_ns is not None and now_ns - stop_at_ns >= 20_000_000_000:
                    receipt["stop_reason"] += ";cleanup_deadline_exceeded"
                    break
                for key, _ in poller.select(0.25):
                    chunk = os.read(key.fileobj.fileno(), 65536)
                    if not chunk:
                        poller.unregister(key.fileobj)
                        key.fileobj.close()
                        continue
                    receipt[key.data + "_bytes"] += len(chunk)
                    remaining = max(0, cap_bytes - receipt["retained_bytes"])
                    kept = chunk[:remaining]
                    streams[key.data].write(kept)
                    streams[key.data].flush()
                    receipt["retained_bytes"] += len(kept)
                    if len(kept) != len(chunk) and receipt["stop_reason"] is None:
                        receipt["stop_reason"] = "aggregate_log_cap_exceeded"
            # Do not release the parent's PID until the pipe loop/cleanup ends.
            if _parent_observation(proc.pid) is not None:
                receipt["exit_code"] = proc.wait(timeout=1)
            else:
                receipt["stop_reason"] = (receipt["stop_reason"] or "parent_not_exited") + ";unreaped_parent"
    except BaseException as error:
        receipt["stop_reason"] = (receipt["stop_reason"] or "runner_exception") + ":" + repr(error)
        if proc is not None and receipt["pid_start_ticks"] is not None:
            try:
                if _start_ticks(proc.pid) == receipt["pid_start_ticks"]:
                    os.killpg(proc.pid, signal.SIGKILL)
                    receipt["cleanup_signals"].append({"signal": "SIGKILL", "exception_cleanup": True})
                    receipt["exit_code"] = proc.wait(timeout=1)
            except (ProcessLookupError, subprocess.TimeoutExpired):
                pass
    finally:
        poller.close()
        if proc is not None:
            for pipe in (proc.stdout, proc.stderr):
                if pipe is not None and not pipe.closed:
                    pipe.close()
        receipt["artifact_sha256_after"] = _sha256(artifact_path) if artifact_path.is_file() else None
        receipt["artifact_unchanged"] = receipt["artifact_sha256_after"] == artifact_before
        receipt["ended_utc"] = _utc()
        receipt["elapsed_seconds"] = (time.monotonic_ns() - started_ns) / 1e9
        receipt["complete"] = (
            receipt["stop_reason"] is None
            and receipt["parent_waitid_wnowait"] is not None
            and receipt["group_empty_before_parent_reap"]
            and receipt["exit_code"] is not None
            and receipt["artifact_unchanged"]
        )
        _write_receipt(receipt_path, receipt)
    return receipt
