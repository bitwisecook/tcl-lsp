#!/usr/bin/env python3
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
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.
#
# SPDX-License-Identifier: AGPL-3.0-or-later

"""Capture bounded per-test terminal records from nextest libtest-json-plus."""

import argparse
import base64
import hashlib
import json
import logging
import math
import os
import re
import secrets
import selectors
import shutil
import signal
import subprocess
import sys
import time
from pathlib import Path
from typing import BinaryIO, cast

LOGGER = logging.getLogger(__name__)

MAX_LIST = 16 * 1024 * 1024
MAX_EVENT = 64 * 1024
MAX_NAME = 4096
MAX_DIAGNOSTIC = 2 * 1024 * 1024
MAX_TERMINAL = 64 * 1024 * 1024
CONSOLE_CAP = 20_000_000
MARKER = b"NEXTEST_CAPTURE_V1\t"
SAFE_NAME = re.compile(r'^[^"\\\x00-\x1f$#@]+$')


def safe_testcase_name(name):
    if not isinstance(name, str):
        return False
    parts = name.split("::")
    for index, part in enumerate(parts):
        if "#" not in part:
            continue
        if not part.startswith("r#"):
            return False
        identifier = part[2:]
        if (
            identifier in {"_", "crate", "self", "super", "Self"}
            or not identifier.isidentifier()
        ):
            return False
        parts[index] = identifier
    return SAFE_NAME.fullmatch("::".join(parts)) is not None


class CaptureError(Exception):
    pass


def unique_object(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise CaptureError("duplicate JSON key")
        result[key] = value
    return result


def write_all(fd, data):
    view = memoryview(data)
    while view:
        written = os.write(fd, view)
        if written <= 0:
            raise CaptureError("console marker write failed")
        view = view[written:]


class EventStream:
    """Remove the one unbounded failure-output string before JSON parsing."""

    def __init__(self):
        self.line = bytearray()
        self.in_string = False
        self.escaped = False
        self.unicode_left = 0
        self.recent = None
        self.current = bytearray()
        self.expect_key = None
        self.skip_output = False
        self.output_len = 0
        self.output_hash = hashlib.sha256()
        self.output_sample = bytearray()
        self.output_seen = False
        self.utf8_pending = 0
        self.utf8_low = 128
        self.utf8_high = 191

    def _reset(self):
        self.__init__()

    def feed(self, block):
        events = []
        for ch in block:
            if ch == 10 and not self.in_string:
                if self.line:
                    try:
                        event = json.loads(
                            self.line,
                            object_pairs_hook=unique_object,
                            parse_constant=lambda _value: (_ for _ in ()).throw(
                                CaptureError("non-finite JSON number")
                            ),
                        )
                    except (ValueError, UnicodeError) as exc:
                        raise CaptureError(f"invalid reporter JSON: {exc}") from exc
                    if not isinstance(event, dict):
                        raise CaptureError("reporter event is not an object")
                    events.append(
                        (
                            event,
                            self.output_len,
                            self.output_hash.hexdigest() if self.output_seen else None,
                            bytes(self.output_sample),
                        )
                    )
                self._reset()
                continue
            if self.in_string:
                if self.skip_output:
                    if self.utf8_pending:
                        if not self.utf8_low <= ch <= self.utf8_high:
                            raise CaptureError("invalid UTF-8 continuation in output")
                        self.utf8_pending -= 1
                        self.utf8_low, self.utf8_high = 128, 191
                    elif ch >= 128:
                        if 194 <= ch <= 223:
                            self.utf8_pending = 1
                        elif ch == 224:
                            self.utf8_pending, self.utf8_low = 2, 160
                        elif 225 <= ch <= 236 or 238 <= ch <= 239:
                            self.utf8_pending = 2
                        elif ch == 237:
                            self.utf8_pending, self.utf8_high = 2, 159
                        elif ch == 240:
                            self.utf8_pending, self.utf8_low = 3, 144
                        elif 241 <= ch <= 243:
                            self.utf8_pending = 3
                        elif ch == 244:
                            self.utf8_pending, self.utf8_high = 3, 143
                        else:
                            raise CaptureError("invalid UTF-8 leading byte in output")
                    if self.unicode_left:
                        if ch not in b"0123456789abcdefABCDEF":
                            raise CaptureError("invalid unicode escape in output")
                        self.unicode_left -= 1
                    elif self.escaped:
                        if ch not in b'"\\/bfnrtu':
                            raise CaptureError("invalid output string escape")
                        if ch == 117:  # u
                            self.unicode_left = 4
                    elif ch < 32:
                        raise CaptureError("raw control byte in output string")
                    if ch != 34 or self.escaped:
                        self.output_len += 1
                        self.output_hash.update(bytes((ch,)))
                        if len(self.output_sample) < 128:
                            self.output_sample.append(ch)
                else:
                    self.line.append(ch)
                    if len(self.current) < MAX_NAME:
                        self.current.append(ch)
                if self.escaped:
                    self.escaped = False
                elif ch == 92:
                    self.escaped = True
                elif ch == 34:
                    self.in_string = False
                    if self.skip_output:
                        if self.utf8_pending:
                            raise CaptureError("truncated UTF-8 in output string")
                        self.line.extend(b'"<streamed>"')
                        self.skip_output = False
                    else:
                        self.recent = bytes(self.current[:-1])
                    self.current.clear()
                continue
            if ch == 34:
                self.in_string = True
                self.escaped = False
                if self.expect_key == b"stdout":
                    self.skip_output = True
                    self.output_seen = True
                else:
                    self.line.append(ch)
                self.expect_key = None
                self.current.clear()
            else:
                self.line.append(ch)
                if ch == 58 and self.recent is not None:  # colon after a key string
                    self.expect_key = self.recent
                elif ch not in b" \t\r":
                    self.expect_key = None
                if ch not in b" \t\r":
                    self.recent = None
            if len(self.line) > MAX_EVENT:
                raise CaptureError("reporter event header exceeds bound")
        return events

    def eof(self):
        if self.line or self.in_string:
            raise CaptureError("partial reporter event at EOF")


def roster(path):
    listing = Path(path)
    if listing.stat().st_size > MAX_LIST:
        raise CaptureError("selected listing exceeds bound")
    with listing.open("rb") as stream:
        raw = stream.read(MAX_LIST + 1)
    if len(raw) > MAX_LIST:
        raise CaptureError("selected listing exceeds bound")
    try:
        data = json.loads(raw, object_pairs_hook=unique_object)
    except (UnicodeDecodeError, json.JSONDecodeError) as exc:
        raise CaptureError(f"invalid selected listing JSON: {exc}") from exc
    if not isinstance(data, dict):
        raise CaptureError("selected listing must be an object")
    suites = data.get("rust-suites")
    if not isinstance(suites, dict):
        raise CaptureError("missing rust-suites")
    by_suite = {}
    selected = {}
    ignored = {}
    for binary_id, suite in suites.items():
        if not isinstance(binary_id, str) or not isinstance(suite, dict):
            raise CaptureError("invalid suite entry")
        if not isinstance(suite.get("status"), str):
            raise CaptureError("invalid suite status")
        if suite.get("status") != "listed":
            continue
        identity = (
            suite.get("package-name"),
            suite.get("binary-name"),
            suite.get("kind"),
        )
        if not all(isinstance(x, str) and SAFE_NAME.fullmatch(x) for x in identity):
            raise CaptureError("unsafe suite identity")
        if suite.get("binary-id") != binary_id or identity in by_suite:
            raise CaptureError("ambiguous or mismatched binary id")
        by_suite[identity] = binary_id
        cases = suite.get("testcases")
        if not isinstance(cases, dict):
            raise CaptureError("missing testcases")
        for name, info in cases.items():
            if not safe_testcase_name(name):
                raise CaptureError("unsafe testcase name")
            if not isinstance(info, dict) or not isinstance(info.get("ignored"), bool):
                raise CaptureError("invalid testcase entry")
            match_info = info.get("filter-match", {})
            if not isinstance(match_info, dict):
                raise CaptureError("invalid filter-match")
            state = match_info.get("status")
            key = (binary_id, name)
            if state == "matches" and info.get("ignored") is False:
                selected[key] = identity
            elif info.get("ignored") is True and (
                state == "matches"
                or (state == "mismatch" and match_info.get("reason") == "ignored")
            ):
                ignored[key] = identity
            elif state != "mismatch":
                raise CaptureError("unknown filter-match state")
    return hashlib.sha256(raw).hexdigest(), by_suite, selected, ignored


def parent_observation(pid):
    return os.waitid(os.P_PID, pid, os.WEXITED | os.WNOHANG | os.WNOWAIT)


def start_ticks(pid):
    raw = Path(f"/proc/{pid}/stat").read_text()
    return int(raw[raw.rfind(")") + 2 :].split()[19])


def group_scan(pgid):
    live, vanished = [], []
    for entry in Path("/proc").iterdir():
        if not entry.name.isdecimal():
            continue
        try:
            raw = (entry / "stat").read_text()
        except FileNotFoundError:
            vanished.append(int(entry.name))
            continue
        fields = raw[raw.rfind(")") + 2 :].split()
        if int(fields[2]) == pgid and fields[0] not in ("Z", "X", "x"):
            live.append(int(entry.name))
    return live, not vanished


class Recorder:
    def __init__(self, listing, out, cap, raw_cap=0):
        self.list_digest, self.by_suite, self.selected, self.ignored = roster(listing)
        self.out = Path(out)
        self.out.mkdir(parents=True, exist_ok=True)
        self.console_cap = cap
        self.console_bytes = 0
        self.console_overflow = False
        self.sequence = 0
        self.nonce = secrets.token_hex(12)
        self.suites_seen = set()
        self.suites_closed = set()
        self.started = set()
        self.finished = {}
        self.invalid = []
        self.parser = EventStream()
        self.raw_cap = raw_cap
        self.raw_size = 0
        self.raw_truncated = False
        self.raw = (
            (self.out / "reporter.raw.jsonl").open("wb", buffering=0)
            if raw_cap
            else None
        )
        self.terminal = (self.out / "terminal.jsonl").open("wb", buffering=0)
        self.terminal_size = 0
        self.stderr = (self.out / "nextest-stderr.txt").open("wb", buffering=0)
        self.stderr_size = 0
        self.stderr_truncated = False
        self.stderr_chunks_reported = 0
        self.marker(
            {
                "kind": "run-start",
                "listing_sha256": self.list_digest,
                "selected": len(self.selected),
            }
        )
        for binary_id, name in self.selected:
            self.marker({"kind": "selected", "binary_id": binary_id, "test": name})

    def marker(self, payload):
        self.sequence += 1
        if self.console_overflow:
            return
        payload = {"seq": self.sequence, "nonce": self.nonce, **payload}
        encoded = base64.urlsafe_b64encode(
            json.dumps(payload, separators=(",", ":"), ensure_ascii=True).encode()
        )
        line = MARKER + encoded + b"\n"
        reserve = 1024
        if self.console_bytes + len(line) + reserve <= self.console_cap:
            try:
                write_all(1, line)
                self.console_bytes += len(line)
            except (OSError, CaptureError):
                self.console_overflow = True
                if "console marker write failed" not in self.invalid:
                    self.invalid.append("console marker write failed")
        elif not self.console_overflow:
            self.console_overflow = True
            overflow = (
                MARKER
                + base64.urlsafe_b64encode(
                    json.dumps({"kind": "console-cap", "nonce": self.nonce}).encode()
                )
                + b"\n"
            )
            if self.console_bytes + len(overflow) <= self.console_cap:
                try:
                    write_all(1, overflow)
                    self.console_bytes += len(overflow)
                except (OSError, CaptureError):
                    pass
            self.invalid.append("console cap exceeded")

    def _test(self, name):
        if not isinstance(name, str) or len(name) > MAX_NAME or "@stress-" in name:
            raise CaptureError("unsafe or stress test name")
        matches = []
        for identity, binary_id in self.by_suite.items():
            prefix = identity[0] + "::" + identity[1] + "$"
            if name.startswith(prefix):
                tail = name[len(prefix) :]
                retry = None
                match = re.fullmatch(r"(.*)#([1-9][0-9]*)", tail)
                if match:
                    tail, retry = match.group(1), int(match.group(2))
                key = (binary_id, tail)
                if key in self.selected or key in self.ignored:
                    matches.append((identity, key, retry))
        if len(matches) != 1:
            raise CaptureError("unknown or ambiguous test identity")
        return matches[0]

    def event(self, event, output_len, output_sha, output_sample):
        typ, status = event.get("type"), event.get("event")
        if typ == "suite":
            expected = (
                {"type", "event", "test_count", "nextest"}
                if status == "started"
                else {
                    "type",
                    "event",
                    "passed",
                    "failed",
                    "ignored",
                    "measured",
                    "filtered_out",
                    "exec_time",
                    "nextest",
                }
            )
            if set(event) != expected:
                raise CaptureError("unexpected suite event fields")
            for key in expected & {
                "test_count",
                "passed",
                "failed",
                "ignored",
                "measured",
                "filtered_out",
            }:
                if type(event[key]) is not int or event[key] < 0:
                    raise CaptureError("invalid suite count")
            if "exec_time" in event and (
                type(event["exec_time"]) not in (int, float)
                or not math.isfinite(event["exec_time"])
                or event["exec_time"] < 0
            ):
                raise CaptureError("invalid suite duration")
            n = event.get("nextest")
            if not isinstance(n, dict) or set(n) != {"crate", "test_binary", "kind"}:
                raise CaptureError("missing nextest suite identity")
            identity = (n.get("crate"), n.get("test_binary"), n.get("kind"))
            if identity not in self.by_suite or any(
                k in n for k in ("stress_index", "stress_total")
            ):
                raise CaptureError("foreign or stress suite")
            if status == "started":
                if identity in self.suites_seen:
                    raise CaptureError("repeated suite start")
                self.suites_seen.add(identity)
            elif status in ("ok", "failed"):
                if identity not in self.suites_seen or identity in self.suites_closed:
                    raise CaptureError("missing or repeated suite close")
                self.suites_closed.add(identity)
            else:
                raise CaptureError("unknown suite event")
            return
        if typ != "test":
            raise CaptureError("unknown event type")
        expected = (
            {"type", "event", "name"}
            if status in ("started", "ignored")
            else {"type", "event", "name", "exec_time"}
        )
        if status in ("ok", "failed"):
            if "stdout" in event:
                expected = expected | {"stdout"}
            if "reason" in event:
                expected = expected | {"reason"}
        if set(event) != expected or ("stdout" in event and "reason" in event):
            raise CaptureError("unexpected test event fields")
        if status in ("ok", "failed") and (
            type(event["exec_time"]) not in (int, float)
            or not math.isfinite(event["exec_time"])
            or event["exec_time"] < 0
        ):
            raise CaptureError("invalid test duration")
        if "stdout" in event and (
            status != "failed" or output_sha is None or event["stdout"] != "<streamed>"
        ):
            raise CaptureError("unexpected failure output payload")
        if "reason" in event and (
            status != "failed"
            or event["reason"]
            not in ("time limit exceeded", "flaky test treated as failure")
        ):
            raise CaptureError("unexpected failure reason")
        identity, key, retry = self._test(event.get("name"))
        if identity not in self.suites_seen:
            raise CaptureError("test before suite identity")
        if status == "started":
            if (
                key not in self.selected and key not in self.ignored
            ) or key in self.started:
                raise CaptureError("foreign or duplicate test start")
            self.started.add(key)
            return
        if status not in ("ok", "failed", "ignored"):
            raise CaptureError("unknown test event")
        if (
            key in self.finished
            or (status != "ignored" and key not in self.selected)
            or (status == "ignored" and key not in self.ignored)
        ):
            raise CaptureError("duplicate or unselected terminal")
        if status != "ignored" and key not in self.started:
            raise CaptureError("terminal without start")
        self.finished[key] = status
        row = {
            "binary_id": key[0],
            "test": key[1],
            "status": status,
            "retry_count": retry,
            "exec_time": event.get("exec_time"),
            "reason": event.get("reason"),
            "output_encoded_bytes": output_len,
            "output_encoded_sha256": output_sha,
            "output_sample_base64": base64.b64encode(output_sample).decode(),
        }
        raw = json.dumps(row, separators=(",", ":"), ensure_ascii=True).encode() + b"\n"
        if self.terminal_size + len(raw) <= MAX_TERMINAL:
            self.terminal.write(raw)
            self.terminal_size += len(raw)
        elif "terminal JSONL cap exceeded" not in self.invalid:
            self.invalid.append("terminal JSONL cap exceeded")
        self.marker({"kind": "terminal", **row})

    def raw_bytes(self, data):
        if self.raw is not None:
            kept = data[: max(0, self.raw_cap - self.raw_size)]
            self.raw.write(kept)
            self.raw_size += len(kept)
            if len(kept) != len(data):
                self.raw_truncated = True

    def stdout_bytes(self, data):
        for item in self.parser.feed(data):
            self.event(*item)

    def stderr_bytes(self, data):
        if self.stderr_chunks_reported < 8:
            self.marker(
                {
                    "kind": "runner-stderr",
                    "bytes": len(data),
                    "sample_base64": base64.b64encode(data[:512]).decode(),
                }
            )
            self.stderr_chunks_reported += 1
        remaining = MAX_DIAGNOSTIC - self.stderr_size
        if remaining > 0:
            self.stderr.write(data[:remaining])
            self.stderr_size += min(len(data), remaining)
        if len(data) > remaining:
            self.stderr_truncated = True

    def finish(self, exit_code, error=None, lifecycle=None):
        if error:
            self.invalid.append(str(error))
        if self.raw_truncated:
            self.invalid.append("raw reporter JSONL cap exceeded")
        if self.raw is not None and self.stderr_truncated:
            self.invalid.append("runner stderr cap exceeded")
        try:
            self.parser.eof()
        except CaptureError as exc:
            self.invalid.append(str(exc))
        incomplete = sorted([list(k) for k in self.selected if k not in self.finished])
        if exit_code == 0:
            expected_suites = {identity for identity in self.selected.values()}
            if not expected_suites <= self.suites_closed:
                self.invalid.append("missing suite closure")
        result = {
            "schema": "nextest-capture-v1",
            "listing_sha256": self.list_digest,
            "nextest_exit": exit_code,
            "terminal_count": len(self.finished),
            "selected_count": len(self.selected),
            "started_unfinished": sorted(
                [list(k) for k in self.started if k not in self.finished]
            ),
            "never_started": sorted(
                [list(k) for k in self.selected if k not in self.started]
            ),
            "ignored_unreported": sorted(
                [list(k) for k in self.ignored if k not in self.finished]
            ),
            "missing_terminal": incomplete,
            "capture_errors": self.invalid,
        }
        result["stderr_truncated"] = self.stderr_truncated
        result["raw_reporter_bytes"] = self.raw_size
        result["raw_reporter_truncated"] = self.raw_truncated
        result["lifecycle"] = lifecycle
        (self.out / "summary.json").write_text(json.dumps(result, indent=2) + "\n")
        self.marker(
            {
                "kind": "run-end",
                "nextest_exit": exit_code,
                "capture_valid": not self.invalid and not incomplete,
            }
        )
        self.terminal.close()
        self.stderr.close()
        if self.raw is not None:
            self.raw.close()
        return result


class InvalidRosterRecorder:
    """Keep the original test command running if capture preflight refuses."""

    def __init__(self, out, reason):
        self.out = Path(out)
        self.out.mkdir(parents=True, exist_ok=True)
        self.reason = reason
        self.stderr = (self.out / "nextest-stderr.txt").open("wb", buffering=0)
        self.stderr_size = 0
        payload = base64.urlsafe_b64encode(
            json.dumps({"kind": "capture-invalid", "reason": reason}).encode()
        )
        try:
            write_all(1, MARKER + payload + b"\n")
        except (OSError, CaptureError):
            pass

    def stdout_bytes(self, _data):
        pass

    def stderr_bytes(self, data):
        remaining = MAX_DIAGNOSTIC - self.stderr_size
        if remaining > 0:
            self.stderr.write(data[:remaining])
            self.stderr_size += min(len(data), remaining)

    def finish(self, exit_code, error=None, lifecycle=None):
        result = {
            "schema": "nextest-capture-v1",
            "nextest_exit": exit_code,
            "capture_errors": [self.reason] + ([str(error)] if error else []),
            "missing_terminal": ["roster unavailable"],
        }
        result["lifecycle"] = lifecycle
        (self.out / "summary.json").write_text(json.dumps(result, indent=2) + "\n")
        self.stderr.close()
        return result


def run(argv):
    parser = argparse.ArgumentParser()
    parser.add_argument("--selected-json", required=True)
    parser.add_argument("--output-dir", required=True)
    parser.add_argument("--console-cap", type=int, default=CONSOLE_CAP)
    parser.add_argument("--raw-cap", type=int, default=0)
    parser.add_argument("--deadline-seconds", type=int, default=0)
    parser.add_argument("--min-free-bytes", type=int, default=0)
    parser.add_argument("command", nargs=argparse.REMAINDER)
    args = parser.parse_args(argv)
    if not args.command or args.command[0] != "--" or len(args.command) < 2:
        parser.error("expected -- command ...")
    if min(args.raw_cap, args.deadline_seconds, args.min_free_bytes) < 0:
        parser.error("capture bounds must be nonnegative")
    try:
        rec = Recorder(
            args.selected_json, args.output_dir, args.console_cap, args.raw_cap
        )
    except (CaptureError, ValueError, OSError) as exc:
        rec = InvalidRosterRecorder(args.output_dir, str(exc))
    if args.min_free_bytes:
        floor = min(shutil.disk_usage(path).free for path in (Path.cwd(), rec.out))
        if floor < args.min_free_bytes:
            rec.finish(
                125,
                "disk below configured floor before launch",
                {"child_spawned": False, "disk_free_bytes": floor},
            )
            return 125
    env = dict(os.environ)
    env["NEXTEST_EXPERIMENTAL_LIBTEST_JSON"] = "1"
    cmd = args.command[1:]
    try:
        proc = subprocess.Popen(
            cmd,
            env=env,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            start_new_session=True,
        )
    except OSError as exc:
        rec.finish(127, f"launch failed: {exc}")
        return 127
    parent_start_ticks = start_ticks(proc.pid)
    external_signals = []
    lifecycle = {
        "child_spawned": True,
        "parent_pid": proc.pid,
        "parent_start_ticks": parent_start_ticks,
        "owned_pgid": proc.pid,
        "group_scan_attempts": 0,
        "group_empty_before_parent_reap": False,
        "cleanup_signals": [],
        "external_signals": external_signals,
    }
    stopped = False

    def forward(sig, _frame):
        nonlocal stopped
        stopped = True
        external_signals.append(signal.Signals(sig).name)
        try:
            if start_ticks(proc.pid) == parent_start_ticks:
                os.killpg(proc.pid, sig)
        except ProcessLookupError:
            pass

    old_int = signal.signal(signal.SIGINT, forward)
    old_term = signal.signal(signal.SIGTERM, forward)
    selector = selectors.DefaultSelector()
    selector.register(cast(BinaryIO, proc.stdout), selectors.EVENT_READ, "stdout")
    selector.register(cast(BinaryIO, proc.stderr), selectors.EVENT_READ, "stderr")
    error = None
    stdout_invalid = False
    parent_exited_at = None
    pipes_closed_at = None
    cleanup_started_at = None
    cleanup_stage = 0
    empty_scans = 0
    started_at = time.monotonic()
    next_disk_check = started_at
    try:
        while True:
            now = time.monotonic()
            parent = parent_observation(proc.pid)
            if cleanup_started_at is None and (
                args.deadline_seconds and now - started_at >= args.deadline_seconds
            ):
                error = "capture deadline exceeded; selected tests may be incomplete"
                cleanup_started_at = now
            if args.min_free_bytes and now >= next_disk_check:
                next_disk_check = now + 1
                floor = min(
                    shutil.disk_usage(path).free for path in (Path.cwd(), rec.out)
                )
                lifecycle["last_disk_free_bytes"] = floor
                if floor < args.min_free_bytes and cleanup_started_at is None:
                    error = (
                        "disk below configured floor; selected tests may be incomplete"
                    )
                    cleanup_started_at = now
            if stopped and cleanup_started_at is None:
                error = "external signal forwarded"
                cleanup_started_at = now
                cleanup_stage = 1
            if not selector.get_map() and pipes_closed_at is None:
                pipes_closed_at = now
            if (
                parent is None
                and pipes_closed_at is not None
                and now - pipes_closed_at >= 2
                and cleanup_started_at is None
            ):
                error = "nextest alive with closed reporter pipes"
                cleanup_started_at = now
            if parent is not None:
                if parent_exited_at is None:
                    parent_exited_at = now
                members, scan_complete = group_scan(proc.pid)
                lifecycle["group_scan_attempts"] += 1
                lifecycle["last_live_members"] = members
                lifecycle["last_scan_complete"] = scan_complete
                empty_scans = empty_scans + 1 if scan_complete and not members else 0
                if not selector.get_map() and empty_scans >= 2:
                    lifecycle["group_empty_before_parent_reap"] = True
                    break
                if cleanup_started_at is None and (
                    not selector.get_map()
                    and (members or not scan_complete and now - parent_exited_at >= 2)
                    or selector.get_map()
                    and now - parent_exited_at >= 2
                ):
                    error = "owned group live or pipe held after nextest exit"
                    cleanup_started_at = now
            if cleanup_started_at is not None:
                elapsed = now - cleanup_started_at
                desired = 3 if elapsed >= 10 else 2 if elapsed >= 5 else 1
                if desired > cleanup_stage:
                    if start_ticks(proc.pid) != parent_start_ticks:
                        raise CaptureError("owned parent identity changed")
                    sig = (signal.SIGINT, signal.SIGTERM, signal.SIGKILL)[desired - 1]
                    try:
                        os.killpg(proc.pid, sig)
                    except ProcessLookupError:
                        pass
                    lifecycle["cleanup_signals"].append(sig.name)
                    cleanup_stage = desired
                if elapsed >= 20:
                    error = "owned group cleanup deadline exceeded"
                    break
            for key, _ in selector.select(0.25):
                data = os.read(key.fd, 65536)
                if not data:
                    selector.unregister(key.fileobj)
                elif key.data == "stdout":
                    if isinstance(rec, Recorder):
                        rec.raw_bytes(data)
                    if not stdout_invalid:
                        try:
                            rec.stdout_bytes(data)
                        except CaptureError as exc:
                            if error is None:
                                error = str(exc)
                            # Continue draining even after capture invalidation.
                            stdout_invalid = True
                elif key.data == "stderr":
                    rec.stderr_bytes(data)
        exit_code = proc.wait(timeout=1)
    except subprocess.TimeoutExpired:
        error = "nextest did not exit after owned cleanup"
        if start_ticks(proc.pid) == parent_start_ticks:
            try:
                os.killpg(proc.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
        exit_code = proc.wait(timeout=2)
    except BaseException as exc:
        LOGGER.exception("Nextest terminal capture failed")
        error = f"capture runner exception: {type(exc).__name__}: {exc}"
        try:
            if start_ticks(proc.pid) == parent_start_ticks:
                os.killpg(proc.pid, signal.SIGKILL)
                lifecycle["cleanup_signals"].append("SIGKILL")
        except (OSError, ValueError):
            pass
        try:
            exit_code = proc.wait(timeout=2)
        except subprocess.TimeoutExpired:
            exit_code = 124
    finally:
        signal.signal(signal.SIGINT, old_int)
        signal.signal(signal.SIGTERM, old_term)
    if not lifecycle["group_empty_before_parent_reap"]:
        error = (error + "; " if error else "") + "owned group emptiness unproved"
    result = rec.finish(
        exit_code, error or ("signal forwarded" if stopped else None), lifecycle
    )
    # Preserve nextest failure status. A capture failure on an otherwise green run fails closed.
    if exit_code < 0:
        return (
            128 - exit_code
        )  # shell-compatible signal status, raw -signal remains in summary
    return (
        exit_code
        if exit_code
        else (2 if result["capture_errors"] or result["missing_terminal"] else 0)
    )


if __name__ == "__main__":
    sys.exit(run(sys.argv[1:]))
