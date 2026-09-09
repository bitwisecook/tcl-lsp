#!/usr/bin/env python3
"""Reproduction harness for bitwisecook/tcl-lsp#2021 (orphaned tcl-lsp-server).

Drives `tcl-lsp-server` over stdio JSON-RPC (Content-Length framing), brings a
large Tcl workspace up to (or partway through) its background workspace scan,
then tears the transport down in one of four ways and watches whether the
server process exits, and what it does with CPU/RSS/threads if it does not.

Scenarios (--scenario):
  clean            shutdown request -> response -> exit notification ->
                   close stdin AND close our stdout read end (server sees EOF
                   on stdin and EPIPE on any further write).
  eof              no shutdown/exit at all: close stdin and our stdout read end.
                   Simulates the VS Code extension host dying outright.
  shutdown-eof     shutdown request -> response -> close stdin and our stdout
                   read end, but never send `exit`. This is what
                   vscode-languageclient does when `stop()` times out.
  exit-stdout-open like `clean`, but keep draining stdout for the whole
                   observation window so the server never gets EPIPE.
                   Distinguishes EPIPE-driven from EOF-driven behaviour.

Teardown timing (--at):
  midscan   ~1.5s after the `initialized` notification, while the workspace
            scan is still running.
  settled   after the `[timing] workspace_folders_scan` log line has been seen,
            plus 3s.

Stdlib only. Python 3.9+.
"""

from __future__ import annotations

import argparse
import json
import os
import select
import signal
import subprocess
import sys
import threading
import time
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]

WORKSPACE_SCAN_SIGNAL = "[timing] workspace_folders_scan"
CLK_TCK = os.sysconf("SC_CLK_TCK")
PAGE_SIZE = os.sysconf("SC_PAGE_SIZE")

SCENARIOS = ("clean", "eof", "shutdown-eof", "exit-stdout-open")
TIMINGS = ("midscan", "settled")


# --------------------------------------------------------------------------
# /proc sampling
# --------------------------------------------------------------------------


def proc_state(pid: int):
    """Return (state_char, utime_ticks, stime_ticks) or None if gone."""
    try:
        with open(f"/proc/{pid}/stat", "rb") as f:
            raw = f.read().decode("utf-8", "replace")
    except OSError:
        return None
    # comm can contain spaces and parens; everything after the last ')' is safe.
    rest = raw[raw.rindex(")") + 1 :].split()
    # rest[0] is state; utime is field 14 overall -> rest[11]; stime -> rest[12]
    try:
        return (rest[0], int(rest[11]), int(rest[12]))
    except (IndexError, ValueError):
        return None


def proc_rss_kib(pid: int):
    try:
        with open(f"/proc/{pid}/status", "rb") as f:
            for line in f:
                if line.startswith(b"VmRSS:"):
                    return int(line.split()[1])
    except OSError:
        return None
    return None


def proc_threads(pid: int):
    try:
        with open(f"/proc/{pid}/status", "rb") as f:
            for line in f:
                if line.startswith(b"Threads:"):
                    return int(line.split()[1])
    except OSError:
        return None
    return None


# --------------------------------------------------------------------------
# LSP transport
# --------------------------------------------------------------------------


class Server:
    def __init__(self, binary: str, cwd: str, stderr_path: Path):
        self.stderr_file = open(stderr_path, "wb")
        self.proc = subprocess.Popen(
            [binary],
            cwd=cwd,
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=self.stderr_file,
            start_new_session=True,  # own process group + session: never
            # implicitly killed with the harness
        )
        self.pid = self.proc.pid
        self._lock = threading.Lock()
        self._responses = {}
        self._log_messages = []
        self._server_requests = []
        # Cleared at teardown: after the client is 'gone' nothing answers
        # server-to-client requests, in every scenario (including
        # exit-stdout-open, which keeps draining but stops replying, so the
        # only variable that scenario changes is EPIPE).
        self._answer_requests = True
        self._next_id = 1
        self._stop_reading = threading.Event()
        self._reader_done = threading.Event()
        self._stdout_closed = False
        self._stdin_closed = False
        self._eof_seen = False
        self._bytes_read = 0
        os.set_blocking(self.proc.stdin.fileno(), False)
        self._reader = threading.Thread(target=self._reader_loop, daemon=True)
        self._reader.start()

    # -- reading ----------------------------------------------------------

    def _reader_loop(self):
        fd = self.proc.stdout.fileno()
        buf = bytearray()
        try:
            while not self._stop_reading.is_set():
                try:
                    r, _, _ = select.select([fd], [], [], 0.1)
                except (OSError, ValueError):
                    break
                if not r:
                    continue
                try:
                    chunk = os.read(fd, 65536)
                except (OSError, ValueError):
                    break
                if not chunk:
                    with self._lock:
                        self._eof_seen = True
                    break
                buf += chunk
                with self._lock:
                    self._bytes_read += len(chunk)
                self._drain_frames(buf)
        finally:
            try:
                self.proc.stdout.close()
            except OSError:
                pass
            self._stdout_closed = True
            self._reader_done.set()

    def _drain_frames(self, buf: bytearray):
        while True:
            idx = buf.find(b"\r\n\r\n")
            if idx < 0:
                return
            headers = buf[:idx].decode("utf-8", "replace")
            length = None
            for line in headers.split("\r\n"):
                if line.lower().startswith("content-length:"):
                    length = int(line.split(":", 1)[1].strip())
            if length is None:
                del buf[: idx + 4]
                continue
            if len(buf) < idx + 4 + length:
                return
            body = bytes(buf[idx + 4 : idx + 4 + length])
            del buf[: idx + 4 + length]
            try:
                msg = json.loads(body.decode("utf-8"))
            except ValueError:
                continue
            self._dispatch(msg)

    def _dispatch(self, msg: dict):
        if "id" in msg and "method" not in msg:
            with self._lock:
                self._responses[msg["id"]] = msg
            return
        if "id" in msg and "method" in msg:
            # Server-to-client request. A real VS Code client answers these,
            # so we do too *while the transport is up* — otherwise
            # `initialized` never gets past `pull_and_apply_config` /
            # `register_file_watchers` and the workspace scan never starts.
            # After teardown the reader thread is gone and nothing answers,
            # which is precisely the condition under test.
            with self._lock:
                self._server_requests.append(msg["method"])
                answering = self._answer_requests
            if answering:
                self._answer_server_request(msg)
            return
        method = msg.get("method")
        if method == "window/logMessage":
            with self._lock:
                self._log_messages.append(msg.get("params", {}).get("message", ""))

    def _answer_server_request(self, msg: dict):
        method = msg.get("method")
        if method == "workspace/configuration":
            items = msg.get("params", {}).get("items") or [{}]
            result = [{} for _ in items]
        else:
            # registerCapability / unregisterCapability / *Refresh /
            # workDoneProgress/create: null is the conforming answer.
            result = None
        try:
            self._send(
                {"jsonrpc": "2.0", "id": msg["id"], "result": result}, timeout=5.0
            )
        except (OSError, TimeoutError, ValueError):
            pass

    # -- writing ----------------------------------------------------------

    def _send(self, obj: dict, timeout: float = 10.0):
        body = json.dumps(obj).encode("utf-8")
        payload = b"Content-Length: %d\r\n\r\n" % len(body) + body
        fd = self.proc.stdin.fileno()
        view = memoryview(payload)
        deadline = time.monotonic() + timeout
        while view:
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                raise TimeoutError("write to server stdin blocked")
            _, w, _ = select.select([], [fd], [], remaining)
            if not w:
                continue
            try:
                n = os.write(fd, view)
            except BlockingIOError:
                continue
            view = view[n:]

    def notify(self, method: str, params=None):
        self._send({"jsonrpc": "2.0", "method": method, "params": params or {}})

    def request(self, method: str, params=None):
        with self._lock:
            rid = self._next_id
            self._next_id += 1
        self._send(
            {"jsonrpc": "2.0", "id": rid, "method": method, "params": params or {}}
        )
        return rid

    def wait_response(self, rid: int, timeout: float):
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            with self._lock:
                if rid in self._responses:
                    return self._responses[rid]
            time.sleep(0.02)
        return None

    def wait_scan(self, timeout: float):
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            with self._lock:
                for m in self._log_messages:
                    if WORKSPACE_SCAN_SIGNAL in m:
                        return m
            time.sleep(0.05)
        return None

    # -- teardown primitives ----------------------------------------------

    def close_stdin(self):
        if self._stdin_closed:
            return
        try:
            self.proc.stdin.close()
        except OSError:
            pass
        self._stdin_closed = True

    def close_stdout_read_end(self):
        """Stop reading and close our read end so the server gets EPIPE."""
        self._stop_reading.set()
        self._reader_done.wait(timeout=5.0)


# --------------------------------------------------------------------------
# Session driving
# --------------------------------------------------------------------------


def build_initialize_params(workspace: Path) -> dict:
    uri = workspace.as_uri()
    return {
        "processId": os.getpid(),
        "clientInfo": {"name": "orphan_repro", "version": "1"},
        "rootUri": uri,
        "workspaceFolders": [{"uri": uri, "name": workspace.name}],
        "capabilities": {
            "workspace": {
                "workspaceFolders": True,
                "configuration": True,
                "didChangeWatchedFiles": {"dynamicRegistration": True},
                "foldingRange": {"refreshSupport": True},
                "codeLens": {"refreshSupport": True},
                "semanticTokens": {"refreshSupport": True},
            },
            "textDocument": {
                "synchronization": {"dynamicRegistration": False, "didSave": True},
                "semanticTokens": {
                    "dynamicRegistration": False,
                    "requests": {"full": True, "range": True},
                    "tokenTypes": [],
                    "tokenModifiers": [],
                    "formats": ["relative"],
                },
                "publishDiagnostics": {"relatedInformation": True},
                "foldingRange": {"dynamicRegistration": False, "lineFoldingOnly": True},
            },
        },
    }


def pick_document(workspace: Path) -> Path:
    """Largest .tcl file under modules/ if present, else largest .tcl overall,
    capped so we open a moderately large file rather than a generated blob."""
    roots = [workspace / "modules", workspace]
    for root in roots:
        if not root.is_dir():
            continue
        cands = []
        for p in root.rglob("*.tcl"):
            try:
                sz = p.stat().st_size
            except OSError:
                continue
            if sz <= 400_000:
                cands.append((sz, p))
        if cands:
            cands.sort()
            return cands[-1][1]
    raise SystemExit(f"no .tcl file found under {workspace}")


def run_session(args) -> dict:
    workspace = Path(args.workspace).resolve()
    outdir = Path(args.outdir)
    outdir.mkdir(parents=True, exist_ok=True)
    tag = f"{args.scenario}-{args.at}"
    if args.tag:
        tag = f"{args.scenario}-{args.at}-{args.tag}"
    stderr_path = outdir / f"{tag}.stderr.log"

    print(f"=== run {tag} ===")
    print(f"workspace: {workspace}")

    srv = Server(args.binary, str(workspace), stderr_path)
    print(f"server pid: {srv.pid}  (own session, start_new_session=True)")
    print(f"stderr -> {stderr_path}")

    result = {
        "scenario": args.scenario,
        "at": args.at,
        "workspace": str(workspace),
        "pid": srv.pid,
        "stderr": str(stderr_path),
    }

    t_start = time.monotonic()
    rid = srv.request("initialize", build_initialize_params(workspace))
    resp = srv.wait_response(rid, timeout=60.0)
    if resp is None:
        raise SystemExit("no initialize response")
    print(f"initialize ok ({time.monotonic() - t_start:.2f}s)")

    t_initialized = time.monotonic()
    srv.notify("initialized", {})

    # Open a real, moderately large document so the diagnostics worker and
    # semantic-token convergence have work.
    doc = (
        Path(args.document).resolve()
        if getattr(args, "document", "")
        else pick_document(workspace)
    )
    text = doc.read_text(encoding="utf-8", errors="replace")
    uri = doc.as_uri()
    print(f"didOpen: {doc}  ({len(text.splitlines())} lines, {len(text)} bytes)")
    srv.notify(
        "textDocument/didOpen",
        {"textDocument": {"uri": uri, "languageId": "tcl", "version": 1, "text": text}},
    )
    result["document"] = str(doc)

    # A couple of edits (full-document sync, matching the server's default
    # TextDocumentSyncKind) so the debounced diagnostics worker keeps churning.
    for version, suffix in (
        (2, "\nset __repro_a 1\n"),
        (3, "\nset __repro_b [expr {1+"),
    ):
        time.sleep(0.15)
        srv.notify(
            "textDocument/didChange",
            {
                "textDocument": {"uri": uri, "version": version},
                "contentChanges": [{"text": text + suffix}],
            },
        )
    sem_rid = srv.request(
        "textDocument/semanticTokens/full", {"textDocument": {"uri": uri}}
    )
    result["semantic_tokens_request_id"] = sem_rid

    # --- wait until the teardown point -----------------------------------
    if args.at == "midscan":
        target = t_initialized + 1.5
        while time.monotonic() < target:
            time.sleep(0.05)
        scan_line = srv.wait_scan(timeout=0.0)
        result["scan_seen_at_teardown"] = scan_line is not None
        print(
            f"teardown at midscan (+{time.monotonic() - t_initialized:.2f}s after initialized); "
            f"scan line seen already: {scan_line is not None}"
        )
    else:
        scan_line = srv.wait_scan(timeout=args.scan_timeout)
        if scan_line is None:
            print(
                f"WARNING: no {WORKSPACE_SCAN_SIGNAL!r} within {args.scan_timeout}s; "
                "proceeding anyway"
            )
            result["scan_seen_at_teardown"] = False
        else:
            print(f"scan line: {scan_line.strip()}")
            result["scan_seen_at_teardown"] = True
            result["scan_line"] = scan_line.strip()
        time.sleep(3.0)
        print(
            f"teardown at settled (+{time.monotonic() - t_initialized:.2f}s after initialized)"
        )

    sem_resp = srv.wait_response(sem_rid, timeout=0.0)
    result["semantic_tokens_answered"] = sem_resp is not None

    # --- teardown ---------------------------------------------------------
    t_teardown = time.monotonic()
    sc = args.scenario
    result["server_requests_before_teardown"] = list(srv._server_requests)
    if sc in ("clean", "shutdown-eof", "exit-stdout-open"):
        sid = srv.request("shutdown", {})
        sresp = srv.wait_response(sid, timeout=2.0)
        result["shutdown_answered"] = sresp is not None
        print(f"shutdown response: {'yes' if sresp is not None else 'NO (2s timeout)'}")
    if sc in ("clean", "exit-stdout-open"):
        try:
            srv.notify("exit")
            result["exit_sent"] = True
        except (OSError, TimeoutError) as exc:
            result["exit_sent"] = False
            print(f"exit notification failed: {exc}")
        print("sent exit notification")
    with srv._lock:
        srv._answer_requests = False
    if sc == "exit-stdout-open":
        srv.close_stdin()
        print(
            "closed stdin; KEEPING stdout read end open (draining, not "
            "replying) -> no EPIPE"
        )
    else:
        srv.close_stdin()
        srv.close_stdout_read_end()
        print("closed stdin AND our stdout read end -> server writes get EPIPE")
    result["teardown_kind"] = sc

    # --- observe ----------------------------------------------------------
    print()
    print(
        f"{'t(s)':>5} {'alive':>5} {'state':>5} {'cpu%':>7} {'rss(MiB)':>9} {'thr':>4}"
    )
    samples = []
    prev = proc_state(srv.pid)
    prev_t = time.monotonic()
    exited_at = None
    for i in range(1, args.observe + 1):
        time.sleep(1.0)
        srv.proc.poll()  # reap so an exited child is not left a zombie
        now = time.monotonic()
        st = proc_state(srv.pid)
        if st is None or st[0] == "Z":
            exited_at = now - t_teardown
            print(
                f"{now - t_teardown:5.1f} {'no':>5} {(st[0] if st else '-'):>5} "
                f"{'-':>7} {'-':>9} {'-':>4}"
            )
            break
        dt = now - prev_t
        if prev is not None:
            cpu = ((st[1] + st[2]) - (prev[1] + prev[2])) / CLK_TCK / dt * 100.0
        else:
            cpu = 0.0
        rss = proc_rss_kib(srv.pid)
        thr = proc_threads(srv.pid)
        samples.append(
            {"t": now - t_teardown, "cpu": cpu, "rss_kib": rss, "threads": thr}
        )
        print(
            f"{now - t_teardown:5.1f} {'yes':>5} {st[0]:>5} {cpu:7.1f} "
            f"{(rss or 0) / 1024:9.1f} {thr if thr is not None else '-':>4}"
        )
        prev, prev_t = st, now

    result["samples"] = samples
    result["exited_at"] = exited_at
    with srv._lock:
        result["server_requests_all"] = list(srv._server_requests)
        result["stdout_bytes_read"] = srv._bytes_read
        result["stdout_eof_seen"] = srv._eof_seen

    print()
    if exited_at is not None:
        verdict = f"EXITED after {exited_at:.1f} s"
        result["verdict"] = "EXITED"
    else:
        cpus = [s["cpu"] for s in samples]
        avg = sum(cpus) / len(cpus) if cpus else 0.0
        rss = samples[-1]["rss_kib"] if samples else 0
        thr = samples[-1]["threads"] if samples else 0
        verdict = (
            f"ORPHAN alive after {args.observe} s "
            f"(cpu avg {avg:.1f}%, rss {(rss or 0) / 1024:.1f} MiB, threads {thr})"
        )
        result["verdict"] = "ORPHAN"
        result["cpu_avg"] = avg
        result["rss_mib"] = (rss or 0) / 1024
        result["threads"] = thr
    print(f"VERDICT: {verdict}")
    result["verdict_line"] = verdict

    # --- gdb sample + cleanup --------------------------------------------
    if exited_at is None:
        gdb_path = outdir / f"{tag}.gdb.txt"
        print(f"capturing gdb stacks -> {gdb_path}")
        try:
            out = subprocess.run(
                [
                    "gdb",
                    "-p",
                    str(srv.pid),
                    "-batch",
                    "-ex",
                    "set pagination off",
                    "-ex",
                    "thread apply all bt 25",
                ],
                capture_output=True,
                timeout=180,
            )
            gdb_path.write_bytes(
                out.stdout + b"\n===== gdb stderr =====\n" + out.stderr
            )
            result["gdb"] = str(gdb_path)
        except (OSError, subprocess.TimeoutExpired) as exc:
            print(f"gdb failed: {exc}")
            result["gdb_error"] = str(exc)

        if args.keep:
            print(f"--keep: leaving server alive, PID {srv.pid}")
        else:
            try:
                os.kill(srv.pid, signal.SIGKILL)
                print(f"SIGKILLed surviving server PID {srv.pid}")
                result["killed"] = True
            except OSError as exc:
                print(f"kill failed: {exc}")
            try:
                srv.proc.wait(timeout=5)
            except subprocess.TimeoutExpired:
                pass

    srv._stop_reading.set()
    try:
        srv.stderr_file.close()
    except OSError:
        pass
    return result


def main():
    ap = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter
    )
    ap.add_argument(
        "--binary", default=str(REPO_ROOT / "target/release/tcl-lsp-server")
    )
    ap.add_argument("--workspace", default=str(REPO_ROOT / "tmp/tcllib-2.0"))
    ap.add_argument("--scenario", choices=SCENARIOS, required=True)
    ap.add_argument("--at", choices=TIMINGS, default="settled")
    ap.add_argument("--observe", type=int, default=45)
    ap.add_argument("--scan-timeout", type=float, default=300.0)
    ap.add_argument(
        "--keep",
        action="store_true",
        help="leave a surviving server alive for inspection",
    )
    ap.add_argument("--outdir", default=str(Path(__file__).resolve().parent))
    ap.add_argument("--tag", default="", help="extra suffix for output filenames")
    ap.add_argument(
        "--document",
        default="",
        help="file to open instead of the auto-picked largest .tcl",
    )
    ap.add_argument(
        "--json", default="", help="append the run result as JSON to this file"
    )
    args = ap.parse_args()

    res = run_session(args)
    if args.json:
        with open(args.json, "a") as f:
            f.write(json.dumps(res) + "\n")
    return 0


if __name__ == "__main__":
    sys.exit(main())
