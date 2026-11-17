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

"""Drive the native server through spec-pack churn and watch for a wedge (#1657).

Copies a fixture workspace, opens every small `.tcl` document in it, then
rewrites (two rounds in three) or deletes (one in three) the workspace pack
`.tcl-lsp/extensions.tclspec` every round, each change followed by
`workspace/didChangeWatchedFiles`. A probe thread asks the document-free
`tcl-lsp.getEffectiveConfig` every half second; an unanswered probe is a
wedge. The server's resident set is logged every `--rss-every` rounds, after
an optional `malloc_trim(0)` through gdb so live growth is separated from
allocator retention.

Exit status: 0 when every round ran and every probe answered, 2 on a wedge,
1 on any other failure. One JSON object per event goes to `--log`.
"""

from __future__ import annotations

import argparse
import contextlib
import json
import queue
import random
import shutil
import subprocess
import sys
import tempfile
import threading
import time
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

REPO_ROOT = Path(__file__).resolve().parents[2]
CLIENT_DIR = REPO_ROOT / ".claude" / "skills" / "lsp-client"

PACK_RELATIVE = Path(".tcl-lsp") / "extensions.tclspec"
FILE_CHANGE_CREATED = 1
FILE_CHANGE_CHANGED = 2
FILE_CHANGE_DELETED = 3


def select_documents(
    root: Path, max_bytes: int, extensions: tuple[str, ...] = ("tcl",)
) -> list[Path]:
    """Every file under *root* with one of *extensions*, smaller than *max_bytes*."""
    suffixes = {f".{extension}" for extension in extensions}
    return sorted(
        p
        for p in root.rglob("*")
        if p.suffix in suffixes and p.is_file() and p.stat().st_size < max_bytes
    )


def pack_action(round_no: int, existed: bool) -> str:
    """`delete` every third round when the pack exists, otherwise `write`."""
    if existed and round_no % 3 == 2:
        return "delete"
    return "write"


def pack_body(round_no: int, *, distinct: bool = False) -> str:
    """A speclib with one command; `distinct` adds the round so every write differs."""
    suffix = f"# round {round_no}\n" if distinct else ""
    return (
        "speclib churnpack 1.0 {\n\n"
        "command ::churnpack::noop {\n"
        "    dialects tcl8.6+\n"
        "    arity 1\n"
        "}\n\n"
        "}\n" + suffix
    )


def change_type(action: str, existed: bool) -> int:
    """The `FileChangeType` a client reports for *action* on the pack."""
    if action == "delete":
        return FILE_CHANGE_DELETED
    return FILE_CHANGE_CHANGED if existed else FILE_CHANGE_CREATED


def parse_status_kib(status_text: str, key: str) -> int | None:
    """A `kB` field of `/proc/PID/status`, such as `VmRSS`."""
    for line in status_text.splitlines():
        name, _, rest = line.partition(":")
        if name == key:
            fields = rest.split()
            if fields and fields[0].isdigit():
                return int(fields[0])
    return None


def parse_stat_cpu_ticks(stat_text: str) -> int:
    """User plus system ticks from `/proc/PID/stat` (fields 14 and 15)."""
    # The command name is parenthesised and may hold spaces; split after it.
    after = stat_text.rsplit(")", 1)[1].split()
    return int(after[11]) + int(after[12])


def parse_io(io_text: str) -> dict[str, int]:
    """The counters of `/proc/PID/io`."""
    out: dict[str, int] = {}
    for line in io_text.splitlines():
        name, _, value = line.partition(":")
        value = value.strip()
        if value.isdigit():
            out[name.strip()] = int(value)
    return out


@dataclass
class ProcSample:
    cpu_ticks: int
    rchar: int
    wchar: int

    def delta(self, later: ProcSample) -> dict[str, int]:
        return {
            "cpu": later.cpu_ticks - self.cpu_ticks,
            "rchar": later.rchar - self.rchar,
            "wchar": later.wchar - self.wchar,
        }


def classify_wedge(delta: dict[str, int]) -> str:
    """Name the shape of an unanswered probe from a `/proc` delta.

    The #1657 wedge burns no CPU and writes nothing while stdin is still read.
    A server that is computing burns CPU: that is slowness, not this wedge.
    """
    if delta["cpu"] == 0 and delta["wchar"] == 0:
        return "idle-wedge"
    if delta["cpu"] > 0:
        return "busy"
    return "writing"


def sample_proc(pid: int) -> ProcSample:
    proc = Path("/proc") / str(pid)
    io = parse_io((proc / "io").read_text())
    return ProcSample(
        parse_stat_cpu_ticks((proc / "stat").read_text()),
        io.get("rchar", 0),
        io.get("wchar", 0),
    )


def rss_kib(pid: int) -> int | None:
    return parse_status_kib((Path("/proc") / str(pid) / "status").read_text(), "VmRSS")


def thread_count(pid: int) -> int:
    return sum(1 for _ in (Path("/proc") / str(pid) / "task").iterdir())


def thread_states(pid: int) -> list[dict[str, str]]:
    """Name, state, wait channel and syscall line of every thread."""
    rows = []
    for task in sorted((Path("/proc") / str(pid) / "task").iterdir()):
        row = {"tid": task.name}
        for name in ("comm", "wchan", "syscall"):
            with contextlib.suppress(OSError):
                row[name] = (task / name).read_text().strip()
        with contextlib.suppress(OSError):
            row["state"] = parse_state((task / "status").read_text())
        rows.append(row)
    return rows


def parse_state(status_text: str) -> str:
    for line in status_text.splitlines():
        if line.startswith("State:"):
            return line.split(":", 1)[1].strip()
    return ""


def gdb_batch(pid: int, commands: list[str], timeout: float = 300.0) -> str:
    """Run gdb in batch mode against *pid*; empty when gdb is missing."""
    if shutil.which("gdb") is None:
        return ""
    argv = ["gdb", "-p", str(pid), "-batch", "-nx"]
    for command in commands:
        argv += ["-ex", command]
    try:
        done = subprocess.run(
            argv, capture_output=True, text=True, timeout=timeout, check=False
        )
    except subprocess.TimeoutExpired:
        return f"gdb timed out after {timeout:.0f}s"
    return done.stdout + done.stderr


def import_client() -> Any:
    sys.path.insert(0, str(CLIENT_DIR))
    import lsp_client

    return lsp_client


def make_client_class(lsp_client: Any, settings: dict | None = None) -> type:
    class SerialisedClient(lsp_client.LspClient):
        """Serialises writes: the probe thread and the churn loop both send."""

        def __init__(self, *args: Any, **kwargs: Any) -> None:
            super().__init__(*args, **kwargs)
            self._send_lock = threading.Lock()

        def _answer_server_request(
            self, method: str, request_id: int, params: dict | None
        ) -> None:
            if settings is None or method != "workspace/configuration":
                super()._answer_server_request(method, request_id, params)
                return
            items = len((params or {}).get("items", [])) or 1
            self._send(
                {"jsonrpc": "2.0", "id": request_id, "result": [settings] * items}
            )

        def _send(self, data: dict, timeout: float = 30.0) -> None:
            with self._send_lock:
                super()._send(data, timeout=timeout)

        def drop_notifications(self) -> None:
            with self._lock:
                self._notifications.clear()

    return SerialisedClient


@dataclass
class Probe:
    client: Any
    timeout: float
    interval: float
    answered: int = 0
    max_latency: float = 0.0
    wedged_at: float | None = None
    stop: threading.Event = field(default_factory=threading.Event)
    # One thread sends every request, so the client's ids never race: the
    # churn loop asks for an extra command through `ask`.
    _asked: queue.Queue = field(default_factory=lambda: queue.Queue(maxsize=1))
    _answers: queue.Queue = field(default_factory=lambda: queue.Queue(maxsize=1))

    def ask(self, command: str, wait: float) -> Any:
        """Run `command` from the probe thread; `None` if it does not answer."""
        self._asked.put(command)
        try:
            return self._answers.get(timeout=wait)
        except queue.Empty:
            return None

    def _execute(self, command: str) -> Any:
        return self.client.send_request(
            "workspace/executeCommand",
            {"command": command, "arguments": []},
            timeout=self.timeout,
        )

    def run(self) -> None:
        while not self.stop.is_set():
            with contextlib.suppress(queue.Empty):
                command = self._asked.get_nowait()
                try:
                    answer = self._execute(command)
                except (TimeoutError, RuntimeError) as error:
                    answer = {"error": str(error)}
                self._answers.put(answer)
            started = time.monotonic()
            try:
                self._execute("tcl-lsp.getEffectiveConfig")
            except TimeoutError:
                self.wedged_at = time.time()
                return
            except (OSError, RuntimeError):
                if self.stop.is_set():
                    return
                raise
            latency = time.monotonic() - started
            self.answered += 1
            self.max_latency = max(self.max_latency, latency)
            self.stop.wait(self.interval)


class EventLog:
    def __init__(self, path: Path | None) -> None:
        self._file = path.open("a", encoding="utf-8") if path else None

    def __call__(self, event: str, **fields: Any) -> None:
        record = {"t": round(time.time(), 3), "event": event, **fields}
        line = json.dumps(record, sort_keys=True)
        print(line, flush=True)
        if self._file:
            self._file.write(line + "\n")
            self._file.flush()


def report_wedge(pid: int, log: EventLog, sample_seconds: float, use_gdb: bool) -> None:
    before = sample_proc(pid)
    time.sleep(sample_seconds)
    delta = before.delta(sample_proc(pid))
    log("wedge", shape=classify_wedge(delta), delta=delta, threads=thread_states(pid))
    if use_gdb:
        log("stacks", text=gdb_batch(pid, ["thread apply all bt"]))


def measure_rss(
    pid: int,
    log: EventLog,
    round_no: int,
    trim: bool,
    probe: Probe | None = None,
    memory_command: str | None = None,
) -> None:
    if probe is not None and memory_command:
        log("memory", round=round_no, report=probe.ask(memory_command, 300.0))
    before = rss_kib(pid)
    after = None
    if trim:
        gdb_batch(pid, ["call (int) malloc_trim(0)"])
        after = rss_kib(pid)
    log(
        "rss",
        round=round_no,
        rss_kib=before,
        trimmed_kib=after,
        threads=thread_count(pid),
    )


def prepare_workspace(fixture: Path, workdir: Path) -> Path:
    workspace = workdir / "workspace"
    shutil.copytree(fixture, workspace)
    (workspace / PACK_RELATIVE.parent).mkdir(parents=True, exist_ok=True)
    return workspace


def churn_once(client: Any, pack: Path, round_no: int, *, distinct: bool) -> str:
    existed = pack.exists()
    action = pack_action(round_no, existed)
    if action == "delete":
        pack.unlink()
    else:
        pack.write_text(pack_body(round_no, distinct=distinct), encoding="utf-8")
    client.send_notification(
        "workspace/didChangeWatchedFiles",
        {"changes": [{"uri": pack.as_uri(), "type": change_type(action, existed)}]},
    )
    return action


def reopen_batch(documents: list[Path], count: int, rng: random.Random) -> list[Path]:
    """The documents to close and reopen this round: an editor restoring tabs."""
    return rng.sample(documents, min(count, len(documents)))


def close_all(client: Any, lsp_client: Any, paths: list[Path]) -> None:
    for path in paths:
        client.send_notification(
            "textDocument/didClose",
            {"textDocument": {"uri": lsp_client.document_uri(str(path))}},
        )


def reopen(client: Any, lsp_client: Any, paths: list[Path]) -> None:
    for path in paths:
        client.send_notification(
            "textDocument/didClose",
            {"textDocument": {"uri": lsp_client.document_uri(str(path))}},
        )
    for path in paths:
        lsp_client.open_document(client, str(path))


def run(args: argparse.Namespace) -> int:
    lsp_client = import_client()
    log = EventLog(args.log)
    rng = random.Random(args.seed)
    workdir = Path(tempfile.mkdtemp(prefix="pack-churn-", dir=args.workdir))
    workspace = prepare_workspace(args.fixture, workdir)
    documents = select_documents(
        workspace, args.max_bytes, tuple(args.extensions.split(","))
    )[: args.max_documents]
    server = lsp_client.find_native_server(args.server_bin)
    client = make_client_class(lsp_client, args.settings)(str(workspace), [server])
    client.start()
    pid = client.process.pid
    log(
        "start",
        server=server,
        pid=pid,
        documents=len(documents),
        workspace=str(workspace),
    )
    status = 0
    probe = Probe(client, args.probe_timeout, args.probe_interval)
    try:
        lsp_client.initialize(client)
        for path in documents:
            lsp_client.open_document(client, str(path))
        log("opened", documents=len(documents))
        prober = threading.Thread(target=probe.run, daemon=True)
        prober.start()
        time.sleep(args.warm_up)
        pack = workspace / PACK_RELATIVE
        for round_no in range(args.rounds):
            if probe.wedged_at is not None:
                break
            if not prober.is_alive():
                log("probe-died", round=round_no)
                status = 1
                break
            try:
                action = churn_once(
                    client, pack, round_no, distinct=args.distinct_bodies
                )
                if args.reopen:
                    reopen(
                        client, lsp_client, reopen_batch(documents, args.reopen, rng)
                    )
            except TimeoutError as blocked:
                # The server stopped reading stdin: as much a wedge as an
                # unanswered probe.
                log("send-blocked", round=round_no, error=str(blocked))
                probe.wedged_at = time.time()
                break
            client.drop_notifications()
            if round_no % args.rss_every == 0:
                measure_rss(
                    pid, log, round_no, args.malloc_trim, probe, args.memory_command
                )
                log(
                    "round",
                    round=round_no,
                    action=action,
                    answered=probe.answered,
                    max_latency=round(probe.max_latency, 3),
                )
            time.sleep(rng.uniform(args.min_interval, args.max_interval))
        else:
            measure_rss(
                pid, log, args.rounds, args.malloc_trim, probe, args.memory_command
            )
        if args.close_at_end and probe.wedged_at is None:
            close_all(client, lsp_client, documents)
            time.sleep(args.settle)
            measure_rss(pid, log, -1, True, probe, args.memory_command)
        if probe.wedged_at is None and prober.is_alive():
            # A wedge late in the last interval still counts.
            prober.join(timeout=args.probe_interval * 2)
        if probe.wedged_at is not None:
            status = 2
            report_wedge(pid, log, args.sample_seconds, args.gdb)
        log(
            "end",
            status=status,
            answered=probe.answered,
            max_latency=round(probe.max_latency, 3),
        )
    finally:
        probe.stop.set()
        if status == 2 and args.keep_wedged:
            log("kept", pid=pid)
        else:
            client.shutdown()
        if not args.keep_workspace:
            shutil.rmtree(workdir, ignore_errors=True)
    return status


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument(
        "--server-bin",
        help="tcl-lsp-server binary (default: TCL_LSP_SERVER_BIN or target/)",
    )
    parser.add_argument(
        "--fixture", type=Path, default=REPO_ROOT / "editors" / "vscode" / "testFixture"
    )
    parser.add_argument(
        "--workdir", type=Path, default=None, help="parent of the scratch workspace"
    )
    parser.add_argument("--rounds", type=int, default=250)
    parser.add_argument("--max-bytes", type=int, default=60_000)
    parser.add_argument(
        "--extensions",
        default="tcl",
        help="comma-separated source extensions to open, e.g. tcl,irul,tm,exp",
    )
    parser.add_argument(
        "--max-documents", type=int, default=None, help="open at most this many"
    )
    parser.add_argument(
        "--reopen",
        type=int,
        default=0,
        help="close and reopen this many documents each round",
    )
    parser.add_argument(
        "--memory-command",
        help="a workspace/executeCommand whose reply is logged before each RSS reading",
    )
    parser.add_argument(
        "--close-at-end",
        action="store_true",
        help="close every document, wait --settle seconds, then trim and measure (round -1)",
    )
    parser.add_argument("--settle", type=float, default=30.0)
    parser.add_argument(
        "--settings",
        type=json.loads,
        default=None,
        help="tclLsp settings object answered to workspace/configuration",
    )
    parser.add_argument("--min-interval", type=float, default=1.0)
    parser.add_argument("--max-interval", type=float, default=2.0)
    parser.add_argument("--warm-up", type=float, default=5.0)
    parser.add_argument("--probe-interval", type=float, default=0.5)
    parser.add_argument("--probe-timeout", type=float, default=120.0)
    parser.add_argument("--rss-every", type=int, default=10)
    parser.add_argument(
        "--distinct-bodies",
        action="store_true",
        help="make every pack write a new body",
    )
    parser.add_argument(
        "--malloc-trim",
        action="store_true",
        help="malloc_trim(0) through gdb before each RSS reading",
    )
    parser.add_argument(
        "--gdb", action="store_true", help="capture every thread's stack on a wedge"
    )
    parser.add_argument("--sample-seconds", type=float, default=5.0)
    parser.add_argument("--seed", type=int, default=1657)
    parser.add_argument(
        "--log", type=Path, default=None, help="append JSON events here"
    )
    parser.add_argument(
        "--keep-wedged", action="store_true", help="leave a wedged server running"
    )
    parser.add_argument("--keep-workspace", action="store_true")
    args = parser.parse_args(argv)
    if args.rss_every < 1:
        parser.error("--rss-every must be at least 1")
    return args


def main() -> int:
    return run(parse_args())


if __name__ == "__main__":
    sys.exit(main())
