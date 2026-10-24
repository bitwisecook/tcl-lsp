# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook)
# SPDX-License-Identifier: AGPL-3.0-or-later

"""Linux-only bounded ownership for the private hosted tool validation."""

import ctypes
import hashlib
import json
import os
import selectors
import shutil
import signal
import subprocess
import time
from contextlib import nullcontext
from pathlib import Path
from typing import BinaryIO, cast

FLOOR = 2 * 1024**3
MAX_LOG = 65536
REPORT_LIMIT = 1024 * 1024
STREAM_LIMIT = 20 * 1024**2
CLEANUP_SECONDS = 20
TERM_SECONDS = 5
POLL_SECONDS = 0.1
_root = None
_deadline = None
_stop = None
_number = 0


def initialize(root):
    global _root, _deadline
    if _root is not None:
        raise RuntimeError("process owner already initialized")
    # Adopt orphaned members so their zombies can be reaped before closure.
    libc = ctypes.CDLL(None, use_errno=True)
    if libc.prctl(36, 1, 0, 0, 0) != 0:  # PR_SET_CHILD_SUBREAPER
        raise OSError(ctypes.get_errno(), "cannot establish child subreaper")
    _root = Path(root)
    _deadline = time.monotonic() + 1800
    for sig in (signal.SIGINT, signal.SIGTERM, signal.SIGHUP):
        signal.signal(sig, _request_stop)


def _request_stop(signum, _frame):
    global _stop
    _stop = f"signal {signum}"


def group_members(pgid):
    """A complete procfs scan, including zombies; vanished entries are benign."""
    members = []
    for path in Path("/proc").iterdir():
        if not path.name.isdecimal():
            continue
        try:
            fields = (path / "stat").read_bytes().rsplit(b") ", 1)[1].split()
        except (FileNotFoundError, ProcessLookupError):
            continue
        if int(fields[2]) == pgid:
            if int(fields[3]) != pgid:
                raise RuntimeError("process group has an unexpected session")
            members.append((int(path.name), fields[0].decode(), int(fields[19])))
    return members


def _signal_group(pgid, signum):
    # Signal only while this owned session still has actual group members.
    if group_members(pgid):
        try:
            os.killpg(pgid, signum)
        except ProcessLookupError:
            pass


def _reap_adopted(pgid):
    # Keep the direct child unreaped: its PID anchors this group until closure.
    for pid, _state, _birth in group_members(pgid):
        if pid == pgid:
            continue
        try:
            os.waitpid(pid, os.WNOHANG)
        except ChildProcessError:
            pass  # This member has not yet been orphaned/adopted.


def _leader_exited(proc):
    if proc.returncode is not None:
        return True
    return (
        os.waitid(
            os.P_PID,
            proc.pid,
            os.WEXITED | os.WNOHANG | os.WNOWAIT,
        )
        is not None
    )


def _observe_and_reap(proc, selector):
    if proc.returncode is None and _leader_exited(proc):
        _reap_adopted(proc.pid)
        members = group_members(proc.pid)
        if not selector.get_map() and not any(pid != proc.pid for pid, _, _ in members):
            # WNOWAIT already observed exit; zero timeout cannot block here.
            proc.wait(timeout=0)
    return group_members(proc.pid)


def _close_group(proc, selector, consume):
    """Bounded TERM/KILL, direct-child reap, pipe drain, and two empty scans."""
    started = time.monotonic()
    if proc.returncode is None:
        _signal_group(proc.pid, signal.SIGTERM)
    empty = 0
    killed = False
    while time.monotonic() < started + CLEANUP_SECONDS:
        now = time.monotonic()
        if not killed and now >= started + TERM_SECONDS:
            if proc.returncode is None:
                _signal_group(proc.pid, signal.SIGKILL)
            killed = True
        consume(selector.select(POLL_SECONDS))
        members = _observe_and_reap(proc, selector)
        empty = empty + 1 if not members else 0
        if empty >= 2 and not selector.get_map() and proc.returncode is not None:
            return True
    return False


def supervise(argv, env=None, report=None, expect_nonzero=False, seconds=120):
    global _number
    if _root is None or _deadline is None:
        raise RuntimeError("process owner not initialized")
    if _stop is not None:
        raise RuntimeError(_stop)
    deadline = min(time.monotonic() + seconds, _deadline - CLEANUP_SECONDS)
    if deadline <= time.monotonic():
        raise RuntimeError("validation deadline exhausted before spawn")
    if shutil.disk_usage(_root).free < FLOOR:
        raise RuntimeError("2 GiB disk floor before spawn")
    _number += 1
    record = _root / f"process-{_number:02}.json"
    with Path(report).open("wb") if report else nullcontext(None) as output:
        selector = selectors.DefaultSelector()
        proc = None
        head = bytearray()
        tail = bytearray()
        log_total = report_total = 0
        raw_total = 0
        raw_hash = hashlib.sha256()
        report_failed = False
        reason = None
        closed = False
        cleanup_attempted = False
        code = None

        def consume(events):
            nonlocal log_total, report_total, raw_total, reason, report_failed
            for key, _mask in events:
                try:
                    chunk = os.read(key.fileobj.fileno(), 8192)
                except BlockingIOError:
                    continue
                except OSError as exc:
                    reason = reason or f"pipe read failed: {exc}"
                    selector.unregister(key.fileobj)
                    cast(BinaryIO, key.fileobj).close()
                    continue
                if not chunk:
                    selector.unregister(key.fileobj)
                    cast(BinaryIO, key.fileobj).close()
                    continue
                raw_total += len(chunk)
                raw_hash.update(chunk)
                if raw_total > STREAM_LIMIT:
                    reason = reason or "20 MiB raw command stream limit"
                if output is not None and key.data == "stdout":
                    available = max(0, REPORT_LIMIT - report_total)
                    if not report_failed:
                        try:
                            output.write(chunk[:available])
                        except OSError as exc:
                            report_failed = True
                            reason = reason or f"report write failed: {exc}"
                    report_total += len(chunk)
                    if report_total > REPORT_LIMIT:
                        reason = reason or "report exceeded 1 MiB safety bound"
                else:
                    log_total += len(chunk)
                    available = MAX_LOG // 2 - len(head)
                    head.extend(chunk[:available])
                    tail.extend(chunk[available:])
                    if len(tail) > MAX_LOG // 2:
                        del tail[: -MAX_LOG // 2]

        try:
            proc = subprocess.Popen(
                argv,
                cwd=Path(__file__).resolve().parents[3],
                env=env,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                start_new_session=True,
            )
            for stream, label in ((proc.stdout, "stdout"), (proc.stderr, "stderr")):
                if stream is None:
                    raise RuntimeError("spawned child pipe is unavailable")
                os.set_blocking(stream.fileno(), False)
                selector.register(stream, selectors.EVENT_READ, label)
            empty = 0
            next_disk_check = 0.0
            while reason is None:
                now = time.monotonic()
                if _stop is not None:
                    reason = _stop
                elif now >= deadline:
                    reason = "command deadline"
                elif now >= next_disk_check:
                    next_disk_check = now + 1
                    if shutil.disk_usage(_root).free < FLOOR:
                        reason = "2 GiB disk floor during command"
                consume(selector.select(POLL_SECONDS))
                if _leader_exited(proc):
                    members = _observe_and_reap(proc, selector)
                    code = proc.returncode
                    if any(pid != proc.pid for pid, _, _ in members):
                        reason = reason or "members survived direct-child exit"
                    empty = empty + 1 if not members else 0
                    if empty >= 2 and not selector.get_map():
                        closed = True
                        break
            if not closed:
                cleanup_attempted = True
                closed = _close_group(proc, selector, consume)
                code = proc.returncode
            if not closed:
                raise RuntimeError("owned process group/pipe closure unproved")
            if reason is not None:
                raise RuntimeError(reason)
            if expect_nonzero:
                if code == 0:
                    raise RuntimeError("command unexpectedly accepted invalid input")
            elif code != 0:
                raise RuntimeError(f"command failed ({code})")
            text = bytes(head)
            if log_total > MAX_LOG:
                text += f"\n...[{log_total - MAX_LOG} bytes omitted]...\n".encode()
            text += bytes(tail)
            return text.decode(errors="replace")
        except BaseException as exc:
            reason = reason or str(exc)
            if proc is not None and not closed and not cleanup_attempted:
                cleanup_attempted = True
                closed = _close_group(proc, selector, consume)
                code = proc.returncode
            diagnostic = (bytes(head) + bytes(tail)).decode(errors="replace")
            raise RuntimeError(
                f"{reason}; group_closed={closed}\n{diagnostic}"
            ) from exc
        finally:
            # Retain every source/build/install artifact for runner teardown.
            # This function never deletes a newly built executable or target tree.
            record.write_text(
                json.dumps(
                    {
                        "argv": argv,
                        "exit_code": code,
                        "reason": reason,
                        "group_closed": closed,
                        "spawned": proc is not None,
                        "raw_bytes": raw_total,
                        "raw_sha256": raw_hash.hexdigest(),
                        "report_bytes": report_total,
                        "log_bytes": log_total,
                    },
                    sort_keys=True,
                )
                + "\n"
            )
            for key in list(selector.get_map().values()):
                cast(BinaryIO, key.fileobj).close()
            selector.close()
