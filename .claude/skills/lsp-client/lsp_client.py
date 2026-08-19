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

"""Standalone LSP client for the Tcl language server.

Starts the server, sends LSP requests, and prints human-readable results.
Uses only the Python standard library — no external dependencies.

Cross-file assertions (definition/references/diagnostics/code-actions/
context/all/completion/code-lens touching more than one file's worth of
state — workspace variables, package tiers, sibling-file completions,
workspace-wide lens counts) wait out the server's background workspace scan
first via `LspClient.wait_for_workspace_scan()` (bounded by --scan-timeout,
default 30s — measured/tuned per issue #1111) rather than racing it — see
issue #1094 and that method's
docstring for the exact server-side signal it waits on. Single-file
subcommands (semantic-tokens, hover, format, ...) are unaffected and
don't wait.

Pass `--also-open FILE` (repeatable) to open one or more companion files
before the main <file> argument — the first-class "open two files and
assert" helper (issue #1111) for a cross-file check (a definition/reference
in <file> resolving into FILE, a sibling-file completion, a workspace-wide
lens count). Companion files are opened *after* the workspace-scan wait
above and *before* <file>, so whichever subcommand you run sees them.

Usage:
    python3 lsp_client.py semantic-tokens <file.tcl>
    python3 lsp_client.py diagnostics <file.tcl>
    python3 lsp_client.py format <file.tcl>
    python3 lsp_client.py hover <file.tcl> <line> <col>
    python3 lsp_client.py completion <file.tcl> <line> <col>
    python3 lsp_client.py definition <file.tcl> <line> <col>
    python3 lsp_client.py references <file.tcl> <line> <col>
    python3 lsp_client.py code-actions <file.tcl> <line> <col> <end_line> <end_col>
    python3 lsp_client.py optimize <file.tcl>
    python3 lsp_client.py symbols <file.tcl>
    python3 lsp_client.py diagram <file.tcl>
    python3 lsp_client.py event-info <EVENT_NAME>
    python3 lsp_client.py command-info <COMMAND_NAME>
    python3 lsp_client.py context <file.tcl>
    python3 lsp_client.py all <file.tcl>
"""

from __future__ import annotations

import argparse
import json
import os
import re
import select
import subprocess
import sys
import threading
import time
from pathlib import Path
from typing import Any

#: Default budget for one write to the server's stdin (see `_send`). Not the
#: same knob as a request's response timeout (`send_request(timeout=...)`
#: bounds the round-trip; this bounds only getting the bytes out the door) —
#: kept as a separate, generous constant so a caller that passes its own
#: request timeout down to `_send` still gets *some* bound even if that
#: timeout is small.
DEFAULT_WRITE_TIMEOUT_S = 30.0

# Constants

SEMANTIC_TOKEN_TYPES = [
    "keyword",  # 0
    "function",  # 1
    "variable",  # 2
    "string",  # 3
    "comment",  # 4
    "number",  # 5
    "operator",  # 6
    "parameter",  # 7
    "namespace",  # 8
    "regexp",  # 9
    "event",  # 10
    "decorator",  # 11
    "escape",  # 12
    "object",  # 13
    "fqdn",  # 14
    "ipAddress",  # 15
    "port",  # 16
    "routeDomain",  # 17
    "partition",  # 18
    "username",  # 19
    "encrypted",  # 20
    "pool",  # 21
    "monitor",  # 22
    "profile",  # 23
    "vlan",  # 24
    "interface",  # 25
    "regexpGroup",  # 26
    "regexpCharClass",  # 27
    "regexpQuantifier",  # 28
    "regexpAnchor",  # 29
    "regexpEscape",  # 30
    "regexpBackref",  # 31
    "regexpAlternation",  # 32
    "binarySpec",  # 33
    "binaryCount",  # 34
    "binaryFlag",  # 35
    "formatPercent",  # 36
    "formatSpec",  # 37
    "formatFlag",  # 38
    "formatWidth",  # 39
    "clockPercent",  # 40
    "clockSpec",  # 41
    "clockModifier",  # 42
]

SEMANTIC_TOKEN_MODIFIERS = [
    "declaration",  # bit 0
    "definition",  # bit 1
    "readonly",  # bit 2
    "defaultLibrary",  # bit 3
]

LSP_SEVERITY = {1: "ERROR", 2: "WARNING", 3: "INFO", 4: "HINT"}

# The one documented, intentional readiness signal for the background
# workspace scan (`Backend::scan_workspace_folders` in
# rust/tcl-lsp-server/src/lib.rs). It fires exactly once, after
# `package_resolver` / `workspace_index` have been (re)built from disk —
# unconditionally, even for a zero-root / single-file session — via a
# `window/logMessage` notification (MessageType::LOG). The server's own doc
# comment on that call site says explicitly: "a client (or a test) that
# needs to know the autoload / cross-file workspace state is current rather
# than racing this scan should wait on this line instead of an unrelated
# per-document signal (issue #1003)". See issue #1094.
#
# NOT the same marker as `[timing] workspace_state.update`, which is a
# *per-document* diagnostics-publish timing line (fires once per open
# document, unrelated to workspace-scan completion) — waiting on that one
# instead is the exact confusion issue #1094 warns against.
WORKSPACE_SCAN_SIGNAL = "[timing] workspace_folders_scan"

COMPLETION_KIND = {
    1: "Text",
    2: "Method",
    3: "Function",
    4: "Constructor",
    5: "Field",
    6: "Variable",
    7: "Class",
    8: "Interface",
    9: "Module",
    10: "Property",
    11: "Unit",
    12: "Value",
    13: "Enum",
    14: "Keyword",
    15: "Snippet",
    16: "Color",
    17: "File",
    18: "Reference",
    19: "Folder",
    20: "EnumMember",
    21: "Constant",
    22: "Struct",
    23: "Event",
    24: "Operator",
    25: "TypeParameter",
}

# Subcommands whose results can depend on cross-file workspace state
# (workspace variables, package tiers — issue #1094): `definition` and
# `references` query `workspace_index` directly per-request, so waiting
# right before the request (see `cmd_definition`/`cmd_references`) is
# sufficient. `diagnostics`, `code-actions`, `context`, and `all` are
# different: diagnostics are *pushed* once, right after `didOpen`, and only
# get republished once the scan completes if the doc was already open when
# `initialized` fired (see `initialized`'s "reschedule every open document"
# comment in rust/tcl-lsp-server/src/lib.rs) — waiting *after* `didOpen`
# would just race that republish instead. So for these, `main()` waits
# *before* `open_document()`, ensuring the first (and only) diagnostics
# publish already sees the fully-populated workspace state.
# `completion` and `code-lens` also read `workspace_index` per-request
# (completion enumerates sibling-file procedures; lenses count
# workspace-wide references), so they wait too — before `didOpen` via
# `main()`, which is also sufficient for their per-request reads.
#
# Verified live (issue #1111) against a real `tcl-lsp-server` build (this
# reasoning previously rested on code inspection only — no binary was
# buildable in the sandbox that filed the issue): `--also-open` + `definition`
# resolving a companion file's symbol on the *first* request of 20/20 freshly
# spawned server processes, with no `--scan-timeout` override, confirms
# waiting before `didOpen` sidesteps the race the comment above describes
# rather than merely happening not to trigger it.
CROSS_FILE_COMMANDS = {
    "definition",
    "references",
    "diagnostics",
    "code-actions",
    "context",
    "all",
    "completion",
    "code-lens",
}

SYMBOL_KIND = {
    1: "File",
    2: "Module",
    3: "Namespace",
    4: "Package",
    5: "Class",
    6: "Method",
    7: "Property",
    8: "Field",
    9: "Constructor",
    10: "Enum",
    11: "Interface",
    12: "Function",
    13: "Variable",
    14: "Constant",
    15: "String",
    16: "Number",
    17: "Boolean",
    18: "Array",
    19: "Object",
    20: "Key",
    21: "Null",
    22: "EnumMember",
    23: "Struct",
    24: "Event",
    25: "Operator",
    26: "TypeParameter",
}


# LspClient — JSON-RPC 2.0 transport over stdio


class LspClient:
    """Manages a language server subprocess and JSON-RPC communication."""

    def __init__(self, server_dir: str, launch_cmd: list[str] | None = None) -> None:
        self.server_dir = server_dir
        #: argv used to spawn the server.  Defaults to the Python server via
        #: ``uv``; ``launch_cmd`` overrides it (e.g. the native Rust binary).
        self._launch_cmd = launch_cmd or [
            "uv",
            "run",
            "--directory",
            server_dir,
            "--no-dev",
            "python",
            "-m",
            "lsp",
        ]
        self.process: subprocess.Popen | None = None
        self._request_id = 0
        self._pending: dict[int, dict] = {}  # id -> {"event": Event, "result": ...}
        self._notifications: list[dict] = []
        self._stderr_lines: list[str] = []
        self._lock = threading.Lock()
        self._reader_thread: threading.Thread | None = None
        self._stderr_thread: threading.Thread | None = None
        self._running = False
        #: Method/id of the most recent request this client tried to send,
        #: updated *before* the write is attempted — so a caller stuck
        #: watching for a hang (see `scripts/perf/bench.py`'s `--deadline`)
        #: can report what the process was doing even when it never got a
        #: response, or never finished sending. Not lock-protected: a reader
        #: only wants this for a diagnostic after something has already gone
        #: wrong, so a torn read is a wrong message, not a wrong result.
        self.last_request_method: str | None = None
        self.last_request_id: int | None = None

    def start(self) -> None:
        """Spawn the server and start the reader thread."""
        self.process = subprocess.Popen(
            self._launch_cmd,
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            cwd=self.server_dir,
        )
        # Non-blocking so `_send`'s select()-bounded write loop actually
        # bounds the write: a *blocking* fd's write() can itself block for
        # the full requested size once select() reports only partial room
        # (observed directly — not just a documentation nuance — in this
        # project's sandboxed CI/dev containers), which would silently
        # reintroduce the unbounded wait `_send` exists to remove.
        #
        # `Popen.stdin` is typed `IO[Any] | None` because Popen doesn't know
        # statically that this call passed `stdin=PIPE`; it always does here,
        # so a `None` at this point is not a "maybe" — it means Popen itself
        # is broken, and an assert says so honestly instead of a silent
        # AttributeError three lines further down.
        stdin = self.process.stdin
        assert stdin is not None, (
            "Popen was called with stdin=PIPE but has no stdin pipe"
        )
        os.set_blocking(stdin.fileno(), False)
        self._running = True
        self._reader_thread = threading.Thread(target=self._reader_loop, daemon=True)
        self._reader_thread.start()
        self._stderr_thread = threading.Thread(target=self._stderr_loop, daemon=True)
        self._stderr_thread.start()

    def _send(self, data: dict, timeout: float = DEFAULT_WRITE_TIMEOUT_S) -> None:
        """Send a JSON-RPC message with Content-Length framing.

        Writes through a raw, `select()`-bounded loop on the stdin file
        descriptor rather than the buffered file object's blocking
        `write()`/`flush()`. That blocking write had no timeout at all: a
        server that stops draining stdin (wedged, or mid-crash, or the
        intermittent #1399 hang — a client stuck in `write()` opposite a
        server idle in `read()`) blocked here forever, and `REQUEST_TIMEOUT_S`
        in `send_request` never even got a chance to fire because the
        request was never fully sent. `select()` reports writability before
        each chunk, so a stalled peer surfaces as a `TimeoutError` a caller
        can catch (as `Bench.check`/`Bench.request` in `bench.py` already
        do for the read side) instead of a wedged process indistinguishable
        from real work.

        `os.write` is used directly on the fd (set non-blocking in `start()`)
        rather than mixed with the buffered `Popen.stdin` object, so every
        send in this client must go through `_send` — do not call
        `self.process.stdin.write()` elsewhere. The fd must stay
        non-blocking: with a blocking fd, `os.write()` can itself block for
        the full requested size even right after `select()` reports the fd
        writable, silently reintroducing the unbounded wait this method
        exists to remove.
        """
        body = json.dumps(data).encode("utf-8")
        header = f"Content-Length: {len(body)}\r\n\r\n".encode("utf-8")
        payload = header + body
        assert self.process and self.process.stdin
        fd = self.process.stdin.fileno()
        deadline = time.monotonic() + timeout
        view = memoryview(payload)
        while view:
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                raise TimeoutError(
                    f"write() to server stdin blocked for >{timeout}s "
                    f"({len(payload) - len(view)}/{len(payload)} bytes sent) "
                    "— the server appears to have stopped draining stdin "
                    "(see issue #1399)"
                )
            try:
                _, writable, _ = select.select([], [fd], [], remaining)
            except (OSError, ValueError) as exc:
                raise BrokenPipeError(
                    f"server stdin closed while sending: {exc}"
                ) from exc
            if not writable:
                continue  # spurious wake or remaining rounded to 0; recheck deadline
            try:
                n = os.write(fd, view)
            except BlockingIOError:
                continue
            except BrokenPipeError:
                raise
            view = view[n:]

    def _read_message(self) -> dict | None:
        """Read one Content-Length-framed message from the server's stdout."""
        assert self.process and self.process.stdout
        stdout = self.process.stdout

        # Read headers until empty line
        content_length = 0
        while True:
            line = stdout.readline()
            if not line:
                return None  # EOF
            line_str = line.decode("utf-8").rstrip("\r\n")
            if not line_str:
                break  # End of headers
            if line_str.lower().startswith("content-length:"):
                content_length = int(line_str.split(":", 1)[1].strip())

        if content_length == 0:
            return None

        body = stdout.read(content_length)
        if not body:
            return None
        return json.loads(body.decode("utf-8"))

    def _reader_loop(self) -> None:
        """Background thread: read messages and dispatch."""
        while self._running:
            try:
                msg = self._read_message()
            except Exception:
                break
            if msg is None:
                break

            if "id" in msg and "method" not in msg:
                # Response to a request this client sent
                rid = msg["id"]
                with self._lock:
                    if rid in self._pending:
                        entry = self._pending[rid]
                        entry["result"] = msg
                        entry["event"].set()
            elif "method" in msg and "id" not in msg:
                # Notification from server
                with self._lock:
                    self._notifications.append(msg)
            elif "method" in msg and "id" in msg:
                # Server-to-client *request* (e.g. `workspace/configuration`,
                # `client/registerCapability`) — every real editor answers
                # these, and the server can block on the reply (the native
                # server's `initialized` handler pulls `tclLsp` config via
                # `workspace/configuration` before it scans the workspace
                # folders, so an unanswered request here silently stalls
                # cross-document indexing, not just the request itself).
                self._answer_server_request(msg["method"], msg["id"], msg.get("params"))

    def _answer_server_request(
        self, method: str, request_id: int, params: dict | None
    ) -> None:
        """Reply to a server-initiated request the way a real editor would.

        Only `workspace/configuration` needs a meaningful payload (an empty
        settings object per requested item, so the server falls back to its
        built-in defaults). Everything else this server sends
        (`client/registerCapability`, `window/workDoneProgress/create`, …)
        expects an acknowledgement with a `null` result.
        """
        if method == "workspace/configuration":
            item_count = len((params or {}).get("items", [])) or 1
            result: Any = [{} for _ in range(item_count)]
        else:
            result = None
        self._send({"jsonrpc": "2.0", "id": request_id, "result": result})

    def send_request(self, method: str, params: dict, timeout: float = 30.0) -> Any:
        """Send a request and wait for the response.

        `timeout` now bounds *both* halves of the round-trip: getting the
        request onto the wire (the `_send` write, previously unbounded — see
        `_send`'s docstring) and waiting for the reply. A caller that passed
        a generous `timeout` for a slow server got that generosity on the
        write before too; it just also now has a ceiling.
        """
        self._request_id += 1
        rid = self._request_id
        # Recorded before the write is attempted, not after: a diagnostic
        # reading this while the write itself is stuck should still say
        # which request that was.
        self.last_request_method = method
        self.last_request_id = rid
        event = threading.Event()
        with self._lock:
            self._pending[rid] = {"event": event, "result": None}

        try:
            self._send(
                {"jsonrpc": "2.0", "id": rid, "method": method, "params": params},
                timeout=timeout,
            )
        except Exception:
            with self._lock:
                self._pending.pop(rid, None)
            raise

        if not event.wait(timeout):
            with self._lock:
                self._pending.pop(rid, None)
            raise TimeoutError(f"Timeout waiting for response to {method} (id={rid})")

        with self._lock:
            entry = self._pending.pop(rid)
        result = entry["result"]

        if "error" in result:
            err = result["error"]
            raise RuntimeError(f"LSP error {err.get('code')}: {err.get('message')}")

        return result.get("result")

    def send_notification(
        self,
        method: str,
        params: dict | None = None,
        timeout: float = DEFAULT_WRITE_TIMEOUT_S,
    ) -> None:
        """Send a notification (no response expected).

        `timeout` bounds only the write (there is no response to wait for) —
        see `_send`. Defaulted generously since callers rarely pass one.
        """
        self.last_request_method = method
        self.last_request_id = None
        msg: dict = {"jsonrpc": "2.0", "method": method}
        if params is not None:
            msg["params"] = params
        self._send(msg, timeout=timeout)

    def collect_notifications(self, method: str, timeout: float = 3.0) -> list[dict]:
        """Wait up to *timeout* seconds and return all notifications matching *method*."""
        deadline = time.monotonic() + timeout
        # Always wait at least a brief moment for the server to push
        time.sleep(min(0.2, timeout))
        while time.monotonic() < deadline:
            with self._lock:
                matches = [n for n in self._notifications if n.get("method") == method]
            if matches:
                # Give a tiny bit more time for additional notifications
                time.sleep(0.1)
                with self._lock:
                    matches = [
                        n for n in self._notifications if n.get("method") == method
                    ]
                return matches
            time.sleep(0.05)
        # Timeout — return whatever we have
        with self._lock:
            return [n for n in self._notifications if n.get("method") == method]

    def _stderr_loop(self) -> None:
        """Background thread: capture server stderr lines."""
        assert self.process and self.process.stderr
        for raw in self.process.stderr:
            try:
                line = raw.decode("utf-8", errors="replace").rstrip("\n\r")
                with self._lock:
                    self._stderr_lines.append(line)
            except Exception:
                break

    def get_stderr_lines(self) -> list[str]:
        """Return all captured stderr lines."""
        with self._lock:
            return list(self._stderr_lines)

    def get_timing_lines(self) -> list[str]:
        """Return ``[timing]`` lines from stderr and ``window/logMessage`` notifications."""
        lines: list[str] = []
        with self._lock:
            for ln in self._stderr_lines:
                if "[timing]" in ln:
                    lines.append(ln)
            for n in self._notifications:
                if n.get("method") == "window/logMessage":
                    msg = n.get("params", {}).get("message", "")
                    if "[timing]" in msg:
                        lines.append(msg)
        return lines

    def get_log_messages(self, *, level: int | None = None) -> list[str]:
        """Return ``window/logMessage`` notification messages.

        *level*: optional filter — 1=Error, 2=Warning, 3=Info, 4=Log.
        """
        messages: list[str] = []
        with self._lock:
            for n in self._notifications:
                if n.get("method") != "window/logMessage":
                    continue
                params = n.get("params", {})
                if level is not None and params.get("type") != level:
                    continue
                messages.append(params.get("message", ""))
        return messages

    def wait_for_workspace_scan(self, timeout: float = 30.0) -> str:
        """Block until the server's initial background workspace scan completes.

        Cross-file navigation (`textDocument/definition`,
        `textDocument/references`) and cross-file diagnostics (workspace
        variable / package-tier resolution, W120/W123) read
        `workspace_index` / `package_resolver`, which
        `Backend::scan_workspace_folders` populates in the background —
        kicked off from the `initialized` handler, running concurrently
        with whatever `didOpen` the client sends next. Asserting before
        that scan lands is issue #1094: results flip between "resolved"
        and "unresolved" depending on scan timing.

        Waits for the `[timing] workspace_folders_scan` `window/logMessage`
        line (see `WORKSPACE_SCAN_SIGNAL` above) — the scan's one
        documented readiness signal, always emitted exactly once per scan
        regardless of workspace size (even a zero-root session gets it).
        Call this *before* `open_document()` / navigation requests for any
        assertion that depends on cross-file state; calling it again after
        the signal has already arrived returns immediately (cheap to call
        defensively from multiple places).

        Raises TimeoutError with a pointer back to the server-side signal
        and this issue if the line never arrives within *timeout* seconds.
        """
        deadline = time.monotonic() + timeout
        while True:
            for msg in self.get_log_messages():
                if WORKSPACE_SCAN_SIGNAL in msg:
                    return msg
            # Defensive: also accept a plain stderr echo of the same line,
            # in case the server (or a future build) mirrors log messages
            # there in addition to `window/logMessage`.
            for line in self.get_stderr_lines():
                if WORKSPACE_SCAN_SIGNAL in line:
                    return line
            if time.monotonic() >= deadline:
                break
            time.sleep(0.05)
        raise TimeoutError(
            f"Timed out after {timeout}s waiting for the server's workspace "
            f"scan to complete — no {WORKSPACE_SCAN_SIGNAL!r} window/logMessage "
            "was seen. Cross-file navigation/diagnostics results read here "
            "would be racy (issue #1094). If this is a legitimately large "
            "workspace, pass a higher --scan-timeout; otherwise check that "
            "the server actually reached `scan_workspace_folders` (see "
            "rust/tcl-lsp-server/src/lib.rs) — e.g. it never got past "
            "`initialized` because a server-to-client request went "
            "unanswered."
        )

    def clear_timing(self) -> None:
        """Clear collected stderr lines and notifications for fresh measurement."""
        with self._lock:
            self._stderr_lines.clear()
            self._notifications = [
                n for n in self._notifications if n.get("method") != "window/logMessage"
            ]

    def shutdown(self) -> None:
        """Cleanly shut down the server."""
        self._running = False
        try:
            if self.process and self.process.poll() is None:
                try:
                    self.send_request("shutdown", {}, timeout=5.0)
                except Exception:
                    pass
                try:
                    self.send_notification("exit")
                except Exception:
                    pass
                try:
                    self.process.wait(timeout=3.0)
                except subprocess.TimeoutExpired:
                    self.process.kill()
                    self.process.wait(timeout=2.0)
        except Exception:
            if self.process:
                try:
                    self.process.kill()
                except Exception:
                    pass


# LSP lifecycle helpers


def find_native_server(override: str | None = None) -> str:
    """Locate the native Rust ``tcl-lsp-server`` binary (serverKind=rust).

    Honours ``override`` / ``TCL_LSP_SERVER_BIN`` first, then probes
    ``target/{release,debug}/`` under the project root.
    """
    explicit = override or os.environ.get("TCL_LSP_SERVER_BIN")
    if explicit:
        p = Path(explicit).resolve()
        if p.exists():
            return str(p)
        raise FileNotFoundError(f"No native server at {p}")
    project_root = Path(__file__).resolve().parent.parent.parent.parent
    for profile in ("release", "debug"):
        candidate = project_root / "target" / profile / "tcl-lsp-server"
        if candidate.exists():
            return str(candidate)
    raise FileNotFoundError(
        "No native tcl-lsp-server binary found — build it with "
        "`cargo build -p tcl-lsp-server` (or `make rust-server`), or pass --server-bin."
    )


def find_server_dir(override: str | None = None) -> str:
    """Locate the tcl-lsp server directory."""
    if override:
        p = Path(override).resolve()
        if (p / "lsp" / "__main__.py").exists():
            return str(p)
        raise FileNotFoundError(f"No server found at {p}")

    # Walk up from script location: .claude/skills/lsp-client/lsp_client.py
    script_dir = Path(__file__).resolve().parent
    # Try: script_dir -> .claude/skills/lsp-client
    #      project root -> script_dir / ../../..
    #      server -> project_root / tcl-lsp
    project_root = script_dir.parent.parent.parent
    server_dir = project_root / "tcl-lsp"
    if (server_dir / "lsp" / "__main__.py").exists():
        return str(server_dir)

    # Also try: maybe we're already inside tcl-lsp
    cwd = Path.cwd()
    if (cwd / "lsp" / "__main__.py").exists():
        return str(cwd)

    raise FileNotFoundError(
        f"Cannot find tcl-lsp server. Tried {server_dir} and cwd={cwd}. "
        "Use --server-dir to specify the path."
    )


def initialize(client: LspClient) -> dict:
    """Send initialize + initialized."""
    result = client.send_request(
        "initialize",
        {
            "processId": os.getpid(),
            "rootUri": f"file://{client.server_dir}",
            "capabilities": {
                "textDocument": {
                    "semanticTokens": {
                        "dynamicRegistration": False,
                        "requests": {"full": True},
                        "tokenTypes": SEMANTIC_TOKEN_TYPES,
                        "tokenModifiers": SEMANTIC_TOKEN_MODIFIERS,
                        "formats": ["relative"],
                    },
                    "publishDiagnostics": {
                        "relatedInformation": False,
                    },
                },
            },
        },
    )
    client.send_notification("initialized", {})
    return result


def open_document(client: LspClient, file_path: str) -> tuple[str, str]:
    """Read a file, send textDocument/didOpen, return (uri, content)."""
    abs_path = os.path.abspath(file_path)
    if not os.path.isfile(abs_path):
        raise FileNotFoundError(f"File not found: {abs_path}")
    with open(abs_path) as f:
        content = f.read()
    uri = f"file://{abs_path}"
    client.send_notification(
        "textDocument/didOpen",
        {
            "textDocument": {
                "uri": uri,
                "languageId": "tcl",
                "version": 1,
                "text": content,
            },
        },
    )
    return uri, content


# Decode / display helpers


def decode_semantic_tokens(data: list[int], source: str) -> list[dict]:
    """Delta-decode the 5-int encoded tokens into human-readable dicts."""
    lines = source.split("\n")
    tokens = []
    line, col = 0, 0

    for i in range(0, len(data), 5):
        delta_line = data[i]
        delta_char = data[i + 1]
        length = data[i + 2]
        type_idx = data[i + 3]
        mod_bits = data[i + 4]

        if delta_line > 0:
            line += delta_line
            col = delta_char
        else:
            col += delta_char

        # Extract source text
        text = ""
        if line < len(lines) and col < len(lines[line]):
            text = lines[line][col : col + length]

        # Decode modifier bitmask
        mods = []
        for bit, name in enumerate(SEMANTIC_TOKEN_MODIFIERS):
            if mod_bits & (1 << bit):
                mods.append(name)

        type_name = (
            SEMANTIC_TOKEN_TYPES[type_idx]
            if type_idx < len(SEMANTIC_TOKEN_TYPES)
            else f"unknown({type_idx})"
        )

        tokens.append(
            {
                "line": line,
                "col": col,
                "length": length,
                "type": type_name,
                "modifiers": mods,
                "text": text,
            }
        )

    return tokens


def print_semantic_tokens(tokens: list[dict]) -> None:
    """Print decoded semantic tokens in a readable table."""
    print(f"=== Semantic Tokens ({len(tokens)} tokens) ===")
    for tok in tokens:
        mods = " [" + ",".join(tok["modifiers"]) + "]" if tok["modifiers"] else ""
        print(
            f'  {tok["line"]:3d}:{tok["col"]:<3d}  {tok["type"]:<12s}  "{tok["text"]}"{mods}'
        )


def print_diagnostics(diag_params: list[dict]) -> None:
    """Print diagnostics from publishDiagnostics notification params."""
    all_diags = []
    for params in diag_params:
        for d in params.get("diagnostics", []):
            all_diags.append(d)

    print(f"=== Diagnostics ({len(all_diags)} items) ===")
    if not all_diags:
        print("  (none)")
        return

    for d in all_diags:
        sev = LSP_SEVERITY.get(d.get("severity", 0), "?")
        code = d.get("code", "")
        r = d.get("range", {})
        s = r.get("start", {})
        e = r.get("end", {})
        msg = d.get("message", "")
        print(
            f"  {sev:<8s}  {code:<5s}  "
            f"{s.get('line', 0)}:{s.get('character', 0)}"
            f"-{e.get('line', 0)}:{e.get('character', 0)}"
            f"  {msg}"
        )


def print_formatting_edits(edits: list[dict] | None, source: str) -> None:
    """Print formatting edits."""
    if not edits:
        print("=== Formatting ===")
        print("  (no edits needed)")
        return

    print(f"=== Formatting ({len(edits)} edits) ===")
    for i, edit in enumerate(edits):
        r = edit.get("range", {})
        s = r.get("start", {})
        e = r.get("end", {})
        new_text = edit.get("newText", "")
        print(
            f"  Edit {i}: replace "
            f"{s.get('line', 0)}:{s.get('character', 0)}"
            f"-{e.get('line', 0)}:{e.get('character', 0)}"
        )
        # For whole-document replacement, show a summary
        lines = source.split("\n")
        total_lines = len(lines)
        new_lines = new_text.split("\n")
        s_line = s.get("line", 0)
        e_line = e.get("line", 0)
        if s_line == 0 and e_line >= total_lines - 1:
            # Whole document replacement — show the formatted result
            print("  (whole document replacement)")
            print()
            for ln in new_lines:
                print(f"    {ln}")
            print()
        else:
            # Partial edit — show the new text
            preview = new_text[:200]
            if len(new_text) > 200:
                preview += "..."
            print(f"  New text: {preview!r}")


def print_hover(result: dict | None) -> None:
    """Print hover result."""
    print("=== Hover ===")
    if not result:
        print("  (no hover)")
        return

    contents = result.get("contents", "")
    if isinstance(contents, dict):
        # MarkupContent
        value = contents.get("value", "")
        print(f"  {value}")
    elif isinstance(contents, str):
        print(f"  {contents}")
    elif isinstance(contents, list):
        for item in contents:
            if isinstance(item, dict):
                print(f"  {item.get('value', '')}")
            else:
                print(f"  {item}")


def print_completions(items: list[dict] | dict | None) -> None:
    """Print completion items.

    `textDocument/completion` may answer with a bare array or a CompletionList
    object; both shapes reach here.
    """
    entries: list[dict]
    if items is None:
        entries = []
    elif isinstance(items, dict):
        entries = items.get("items", [])
    else:
        entries = items

    print(f"=== Completions ({len(entries)} items) ===")
    # Show first 30 items
    for item in entries[:30]:
        label = item.get("label", "?")
        kind_num = item.get("kind", 0)
        kind = COMPLETION_KIND.get(kind_num, f"({kind_num})")
        detail = item.get("detail", "")
        detail_str = f"  -- {detail}" if detail else ""
        print(f"  {label:<30s}  {kind:<12s}{detail_str}")
    if len(entries) > 30:
        print(f"  ... and {len(entries) - 30} more")


def print_locations(locations: list[dict] | None, label: str) -> None:
    """Print location results (definition, references)."""
    if not locations:
        locations = []
    print(f"=== {label} ({len(locations)} locations) ===")
    if not locations:
        print("  (none)")
        return

    for loc in locations:
        uri = loc.get("uri", "")
        # Shorten the URI for display
        if uri.startswith("file://"):
            path = uri[7:]
            # Show just the filename
            short = os.path.basename(path)
        else:
            short = uri
        r = loc.get("range", {})
        s = r.get("start", {})
        e = r.get("end", {})
        print(
            f"  {short}  "
            f"{s.get('line', 0)}:{s.get('character', 0)}"
            f"-{e.get('line', 0)}:{e.get('character', 0)}"
        )


def print_code_actions(actions: list[dict] | None) -> None:
    """Print code action results."""
    if not actions:
        actions = []
    print(f"=== Code Actions ({len(actions)} actions) ===")
    if not actions:
        print("  (none)")
        return

    for action in actions:
        title = action.get("title", "?")
        kind = action.get("kind", "")
        print(f"  [{kind}] {title}")
        edit = action.get("edit", {})
        changes = edit.get("changes", {})
        for uri, edits in changes.items():
            for e in edits:
                r = e.get("range", {})
                s = r.get("start", {})
                end = r.get("end", {})
                new_text = e.get("newText", "")
                print(
                    f"    Replace "
                    f"{s.get('line', 0)}:{s.get('character', 0)}"
                    f"-{end.get('line', 0)}:{end.get('character', 0)}"
                    f" with: {new_text!r}"
                )
        command = action.get("command")
        if command:
            cmd_name = command.get("command", "?")
            cmd_args = command.get("arguments", [])
            print(f"    Command: {cmd_name} {cmd_args}")


def print_optimizations(result: dict | None, content: str) -> None:
    """Print optimization results from workspace/executeCommand."""
    if not result:
        print("=== Optimizations ===")
        print("  (no optimizations available)")
        return

    opts = result.get("optimisations", [])
    optimized_source = result.get("source", content)

    print(f"=== Optimizations ({len(opts)} items) ===")
    if not opts:
        print("  (none)")
    else:
        for o in opts:
            code = o.get("code", "")
            msg = o.get("message", "")
            sl = o.get("startLine", 0)
            sc = o.get("startCharacter", 0)
            el = o.get("endLine", 0)
            ec = o.get("endCharacter", 0)
            replacement = o.get("replacement", "")
            print(f"  {code:<5s}  {sl}:{sc}-{el}:{ec}  {msg}  \u2192  {replacement!r}")

    if optimized_source != content:
        print()
        print("=== Optimized Source ===")
        for ln in optimized_source.split("\n"):
            print(f"    {ln}")


def _flatten_document_symbols(
    symbols: list[dict],
    into: list[dict],
    depth: int = 0,
) -> None:
    """Recursively flatten nested DocumentSymbol hierarchy."""
    for sym in symbols:
        kind_num = sym.get("kind", 0)
        kind_label = SYMBOL_KIND.get(kind_num, f"({kind_num})")
        name = sym.get("name", "?")
        detail = sym.get("detail", "")
        sel_range = sym.get("selectionRange", sym.get("range", {}))
        start = sel_range.get("start", {})
        line = start.get("line", 0) + 1  # 1-based for display
        into.append(
            {
                "kind": kind_label,
                "name": name,
                "detail": detail,
                "line": line,
                "depth": depth,
            }
        )
        children = sym.get("children") or []
        if children:
            _flatten_document_symbols(children, into, depth + 1)


def print_symbols(symbols: list[dict] | None) -> None:
    """Print document symbols in a readable hierarchy."""
    if not symbols:
        print("=== Symbol Definitions ===")
        print("  (none)")
        return

    flat: list[dict] = []
    _flatten_document_symbols(symbols, flat)
    print(f"=== Symbol Definitions ({len(flat)} symbols) ===")
    for sym in flat:
        indent = "  " * sym["depth"]
        detail = f" {sym['detail']}" if sym["detail"] else ""
        print(f"  {indent}{sym['kind']} {sym['name']}{detail} (line {sym['line']})")


def print_event_info(result: dict | None) -> None:
    """Print iRules event registry metadata."""
    print("=== Event Info ===")
    if not result:
        print("  (no data)")
        return

    event = result.get("event", "?")
    known = result.get("known", False)
    deprecated = result.get("deprecated", False)
    cmd_count = result.get("validCommandCount", 0)
    samples = result.get("sampleCommands", [])

    print(f"  Event: {event}")
    print(f"  Known: {'yes' if known else 'no'}")
    print(f"  Deprecated: {'yes' if deprecated else 'no'}")
    print(f"  Valid commands: {cmd_count}")
    if samples:
        # Show first 20 commands, then summarize
        show = samples[:20]
        print(f"  Sample commands: {', '.join(show)}")
        if len(samples) > 20:
            print(f"    ... and {len(samples) - 20} more")


def print_command_info(result: dict | None) -> None:
    """Print iRules command registry metadata."""
    print("=== Command Info ===")
    if not result:
        print("  (no data)")
        return

    if not result.get("found", False):
        print(f"  Command '{result.get('command', '?')}' not found in registry")
        return

    command = result.get("command", "?")
    summary = result.get("summary", "")
    synopsis = result.get("synopsis", [])
    switches = result.get("switches", [])

    print(f"  Command: {command}")
    if summary:
        print(f"  Summary: {summary}")
    if synopsis:
        for syn in synopsis:
            print(f"  Synopsis: {syn}")
    if switches:
        print(f"  Switches: {', '.join(switches)}")

    valid_events = result.get("validEvents", [])
    any_event = result.get("anyEvent", False)
    if any_event:
        print("  Valid in: any event")
    elif valid_events:
        print(f"  Valid in: {', '.join(valid_events[:15])}")
        if len(valid_events) > 15:
            print(f"    ... and {len(valid_events) - 15} more")


def print_diagram_data(result: dict | None) -> None:
    """Print structured diagram data from compiler IR."""
    print("=== Diagram Data ===")
    if not result:
        print("  (no data)")
        return

    if result.get("error"):
        print(f"  Error: {result['error']}")
        return

    events = result.get("events", [])
    procedures = result.get("procedures", [])

    if events:
        print(f"\n  Events ({len(events)}, in firing order):")
        for evt in events:
            name = evt.get("name", "?")
            pri = evt.get("priority")
            mult = evt.get("multiplicity", "?")
            pri_str = f" priority={pri}" if pri is not None else ""
            flow_count = len(evt.get("flow", []))
            print(f"    {name} ({mult}{pri_str}) — {flow_count} flow nodes")

    if procedures:
        print(f"\n  Procedures ({len(procedures)}):")
        for proc in procedures:
            name = proc.get("name", "?")
            params = proc.get("params", [])
            flow_count = len(proc.get("flow", []))
            print(f"    {name}({', '.join(params)}) — {flow_count} flow nodes")

    # Print the full JSON for downstream consumption
    print("\n  --- Raw JSON ---")
    print(json.dumps(result, indent=2))


def _detect_events(source: str) -> list[str]:
    """Detect iRule event names from source (when EVENT { ... })."""
    events: list[str] = []
    seen: set[str] = set()
    for match in re.finditer(
        r"^\s*when\s+([A-Z][A-Z0-9_]{2,})\b", source, re.MULTILINE
    ):
        name = match.group(1)
        if name not in seen:
            seen.add(name)
            events.append(name)
    return events


# Subcommand handlers


def cmd_semantic_tokens(client: LspClient, uri: str, content: str) -> None:
    """Request and display semantic tokens."""
    result = client.send_request(
        "textDocument/semanticTokens/full",
        {
            "textDocument": {"uri": uri},
        },
    )
    data = result.get("data", []) if result else []
    tokens = decode_semantic_tokens(data, content)
    print_semantic_tokens(tokens)


def cmd_diagnostics(client: LspClient, uri: str) -> None:
    """Collect and display pushed diagnostics."""
    notifs = client.collect_notifications("textDocument/publishDiagnostics")
    matching = [n["params"] for n in notifs if n.get("params", {}).get("uri") == uri]
    if not matching:
        # Try with all notifications (some servers may not match URI exactly)
        matching = [n["params"] for n in notifs]
    print_diagnostics(matching)


def cmd_format(client: LspClient, uri: str, content: str) -> None:
    """Request and display formatting edits."""
    result = client.send_request(
        "textDocument/formatting",
        {
            "textDocument": {"uri": uri},
            "options": {"tabSize": 4, "insertSpaces": True},
        },
    )
    print_formatting_edits(result, content)


def cmd_hover(client: LspClient, uri: str, line: int, col: int) -> None:
    """Request and display hover information."""
    result = client.send_request(
        "textDocument/hover",
        {
            "textDocument": {"uri": uri},
            "position": {"line": line, "character": col},
        },
    )
    print_hover(result)


def cmd_completion(client: LspClient, uri: str, line: int, col: int) -> None:
    """Request and display completions."""
    result = client.send_request(
        "textDocument/completion",
        {
            "textDocument": {"uri": uri},
            "position": {"line": line, "character": col},
        },
    )
    print_completions(result)


def cmd_definition(client: LspClient, uri: str, line: int, col: int) -> None:
    """Request and display definitions.

    A "navigation request" per issue #1094: waits out the background
    workspace scan first (idempotent/cheap if `main()` already did) so a
    cross-file definition isn't raced. Belt-and-suspenders for callers that
    invoke this directly rather than through `main()`.
    """
    client.wait_for_workspace_scan()
    result = client.send_request(
        "textDocument/definition",
        {
            "textDocument": {"uri": uri},
            "position": {"line": line, "character": col},
        },
    )
    if isinstance(result, dict):
        result = [result]
    print_locations(result, "Definition")


def cmd_references(client: LspClient, uri: str, line: int, col: int) -> None:
    """Request and display references.

    A "navigation request" per issue #1094: waits out the background
    workspace scan first (idempotent/cheap if `main()` already did) so
    cross-file references aren't raced. Belt-and-suspenders for callers
    that invoke this directly rather than through `main()`.
    """
    client.wait_for_workspace_scan()
    result = client.send_request(
        "textDocument/references",
        {
            "textDocument": {"uri": uri},
            "position": {"line": line, "character": col},
            "context": {"includeDeclaration": True},
        },
    )
    if isinstance(result, dict):
        result = [result]
    print_locations(result, "References")


def print_code_lenses(lenses: list[dict] | None) -> None:
    """Print code-lens results, one line per lens.

    Shows the anchor range, the resolved title when the lens carries a
    `command`, and `(unresolved)` plus the `data` payload when it does
    not — the lazy shape a proc/class/method/classmethod reference-count
    lens takes before the client calls `codeLens/resolve` (see
    `cmd_code_lens`, which resolves every lens automatically).
    """
    if not lenses:
        lenses = []
    print(f"=== Code Lenses ({len(lenses)} lenses) ===")
    if not lenses:
        print("  (none)")
        return
    for lens in lenses:
        r = lens.get("range", {})
        s = r.get("start", {})
        e = r.get("end", {})
        pos = f"{s.get('line', 0)}:{s.get('character', 0)}-{e.get('line', 0)}:{e.get('character', 0)}"
        command = lens.get("command")
        if command:
            title = command.get("title", "?")
            cmd_name = command.get("command", "")
            if cmd_name:
                arg_count = len(command.get("arguments") or [])
                print(f"  {pos}  {title!r}  [{cmd_name}, {arg_count} args]")
            else:
                print(f"  {pos}  {title!r}  [inert — empty command id]")
        else:
            data = lens.get("data", {})
            print(f"  {pos}  (unresolved)  data={data!r}")


def cmd_code_lens(client: LspClient, uri: str) -> None:
    """Request code lenses, then resolve every unresolved one.

    Proc / class / method / classmethod reference-count lenses are
    returned lazily (range + `data`, no `command`) so the server can
    recompute the count against the live document at resolve time; a
    real editor always calls `codeLens/resolve` before display, so this
    mirrors that round-trip instead of only showing the raw lazy list.
    """
    lenses = client.send_request(
        "textDocument/codeLens", {"textDocument": {"uri": uri}}
    )
    if not lenses:
        print_code_lenses(lenses)
        return
    resolved = []
    for lens in lenses:
        if lens.get("command"):
            resolved.append(lens)
        else:
            resolved.append(client.send_request("codeLens/resolve", lens))
    print_code_lenses(resolved)


def cmd_code_actions(
    client: LspClient,
    uri: str,
    line: int,
    col: int,
    end_line: int,
    end_col: int,
) -> None:
    """Request and display code actions for a range."""
    # Collect diagnostics that the server pushed after didOpen
    notifs = client.collect_notifications("textDocument/publishDiagnostics")
    diags_in_range = []
    for notif in notifs:
        params = notif.get("params", {})
        if params.get("uri") != uri:
            continue
        for d in params.get("diagnostics", []):
            r = d.get("range", {})
            s = r.get("start", {})
            e = r.get("end", {})
            # Check overlap with requested range
            if s.get("line", 0) <= end_line and e.get("line", 0) >= line:
                diags_in_range.append(d)

    result = client.send_request(
        "textDocument/codeAction",
        {
            "textDocument": {"uri": uri},
            "range": {
                "start": {"line": line, "character": col},
                "end": {"line": end_line, "character": end_col},
            },
            "context": {"diagnostics": diags_in_range},
        },
    )
    print_code_actions(result)


def cmd_optimize(client: LspClient, uri: str, content: str) -> None:
    """Request and display optimization suggestions via workspace command."""
    result = client.send_request(
        "workspace/executeCommand",
        {
            "command": "tcl-lsp.optimiseDocument",
            "arguments": [uri],
        },
    )
    print_optimizations(result, content)


def cmd_symbols(client: LspClient, uri: str) -> None:
    """Request and display document symbols."""
    result = client.send_request(
        "textDocument/documentSymbol",
        {
            "textDocument": {"uri": uri},
        },
    )
    print_symbols(result)


def cmd_diagram(client: LspClient, content: str) -> None:
    """Request diagram data via workspace command (takes source text)."""
    result = client.send_request(
        "workspace/executeCommand",
        {
            "command": "tcl-lsp.diagramData",
            "arguments": [content],
        },
    )
    print_diagram_data(result)


def cmd_event_info(client: LspClient, event_name: str) -> None:
    """Request iRules event registry metadata."""
    result = client.send_request(
        "workspace/executeCommand",
        {
            "command": "tcl-lsp.describeIruleEvent",
            "arguments": [event_name],
        },
    )
    print_event_info(result)


def cmd_command_info(client: LspClient, command_name: str) -> None:
    """Request iRules command registry metadata."""
    result = client.send_request(
        "workspace/executeCommand",
        {
            "command": "tcl-lsp.describeIruleCommand",
            "arguments": [command_name],
        },
    )
    print_command_info(result)


def cmd_context(client: LspClient, uri: str, content: str) -> None:
    """Build a context pack: diagnostics + symbols + event metadata.

    Mirrors the context enrichment from the VS Code extension's contextPack.ts.
    """
    file_path = uri.replace("file://", "")
    basename = os.path.basename(file_path)
    line_count = len(content.split("\n"))

    # Detect dialect from extension
    ext = os.path.splitext(file_path)[1].lower()
    dialect = "f5-irules" if ext in (".irul", ".irule") else "tcl8.6"

    print("=== Context Pack ===")
    print(f"  Dialect: {dialect}")
    print(f"  File: {basename}")
    print(f"  Lines: {line_count}")

    # Diagnostics
    print()
    notifs = client.collect_notifications("textDocument/publishDiagnostics")
    matching = [n["params"] for n in notifs if n.get("params", {}).get("uri") == uri]
    if not matching:
        matching = [n["params"] for n in notifs]

    all_diags = []
    for params in matching:
        for d in params.get("diagnostics", []):
            all_diags.append(d)

    # Filter to actionable (error + warning)
    actionable = [d for d in all_diags if d.get("severity", 0) <= 2]
    actionable.sort(key=lambda d: d.get("range", {}).get("start", {}).get("line", 0))

    if actionable:
        print(f"=== Diagnostics ({len(actionable)}) ===")
        for d in actionable[:12]:
            sev = LSP_SEVERITY.get(d.get("severity", 0), "?")
            code = d.get("code", "")
            r = d.get("range", {})
            s = r.get("start", {})
            msg = d.get("message", "")
            print(f"  {sev} {code} line {s.get('line', 0) + 1}: {msg}")
        if len(actionable) > 12:
            print(f"  ... and {len(actionable) - 12} more")
    else:
        print("=== Diagnostics ===")
        print("  (no errors or warnings)")

    # Document symbols
    print()
    try:
        symbols_result = client.send_request(
            "textDocument/documentSymbol",
            {"textDocument": {"uri": uri}},
        )
        flat: list[dict] = []
        if symbols_result:
            _flatten_document_symbols(symbols_result, flat)
        if flat:
            print(f"=== Symbol Definitions ({len(flat)}) ===")
            for sym in flat[:15]:
                indent = "  " * sym["depth"]
                detail = f" {sym['detail']}" if sym["detail"] else ""
                print(
                    f"  {indent}{sym['kind']} {sym['name']}{detail} (line {sym['line']})"
                )
            if len(flat) > 15:
                print(f"  ... and {len(flat) - 15} more")
        else:
            print("=== Symbol Definitions ===")
            print("  (none)")
    except Exception:
        print("=== Symbol Definitions ===")
        print("  (unavailable)")

    # Event metadata (for iRules files)
    events = _detect_events(content)
    if events:
        print()
        print(f"=== Event Metadata ({len(events)} events, in source order) ===")
        for event_name in events[:8]:
            try:
                info = client.send_request(
                    "workspace/executeCommand",
                    {
                        "command": "tcl-lsp.describeIruleEvent",
                        "arguments": [event_name],
                    },
                )
                if info:
                    known = "yes" if info.get("known") else "no"
                    deprecated = "yes" if info.get("deprecated") else "no"
                    cmd_count = info.get("validCommandCount", 0)
                    samples = info.get("sampleCommands", [])[:8]
                    print(
                        f"  {event_name}: known={known}, deprecated={deprecated}, "
                        f"validCommands={cmd_count}"
                    )
                    if samples:
                        print(f"    sample: {', '.join(samples)}")
                else:
                    print(f"  {event_name}: metadata unavailable")
            except Exception:
                print(f"  {event_name}: metadata unavailable")
        if len(events) > 8:
            print(f"  ... and {len(events) - 8} more events")


def cmd_all(client: LspClient, uri: str, content: str) -> None:
    """Run semantic-tokens + diagnostics + symbols + format + optimize in one session."""
    cmd_semantic_tokens(client, uri, content)
    print()
    cmd_diagnostics(client, uri)
    print()
    cmd_symbols(client, uri)
    print()
    cmd_format(client, uri, content)
    print()
    cmd_optimize(client, uri, content)


_TIMING_RE = re.compile(r"\[timing\]\s+(\S+)\s+([\d.]+)ms")


def cmd_bench(
    client: LspClient, uri: str, content: str, *, iterations: int = 1
) -> None:
    """Benchmark time-to-semantic-tokens replicating VS Code's request pattern.

    VS Code sends requests sequentially after didOpen:
      1. didOpen (fire-and-forget notification)
      2. workspace/didChangeConfiguration (dialect)
      3. documentSymbol
      4. codeAction (after diagnostics arrive)
      5. documentLink
      6. inlayHint
      7. foldingRange
      8. documentSymbol (re-request)
      9. semanticTokens/full
    Each request waits for the previous response, matching the editor's
    sequential processing model.
    """
    n_lines = content.count("\n") + 1
    print("=== Benchmark (VS Code pattern) ===")
    print(f"  File: {uri.split('/')[-1]}")
    print(f"  Lines: {n_lines}, Size: {len(content)} bytes")
    print(f"  Iterations: {iterations}")
    print()

    td = {"textDocument": {"uri": uri}}

    for i in range(iterations):
        client.clear_timing()
        # Close and re-open to force a full rebuild.
        if i > 0:
            client.send_notification(
                "textDocument/didClose",
                {"textDocument": {"uri": uri}},
            )
            time.sleep(0.1)
            client.send_notification(
                "textDocument/didOpen",
                {
                    "textDocument": {
                        "uri": uri,
                        "languageId": "tcl",
                        "version": i + 1,
                        "text": content,
                    }
                },
            )

        t_open = time.perf_counter()

        # VS Code sends didChangeConfiguration shortly after didOpen —
        # detect iRules content or extension to match real editor behavior.
        ext = uri.rsplit(".", 1)[-1].lower() if "." in uri else ""
        is_irules = ext in ("irul", "irule") or "when " in content[:2000]
        if is_irules:
            client.send_notification(
                "workspace/didChangeConfiguration",
                {"settings": {"tclLsp": {"dialect": "f5-irules"}}},
            )

        # Sequential request chain — each waits for its response.
        step_times: list[tuple[str, float]] = []

        def _step(name: str, method: str, params: dict) -> Any:
            t = time.perf_counter()
            result = client.send_request(method, params, timeout=120.0)
            elapsed = (time.perf_counter() - t) * 1000
            step_times.append((name, elapsed))
            return result

        code_action_params = {
            "textDocument": {"uri": uri},
            "range": {
                "start": {"line": 0, "character": 0},
                "end": {"line": 0, "character": 0},
            },
            "context": {"diagnostics": []},
        }
        inlay_hint_params = {
            "textDocument": {"uri": uri},
            "range": {
                "start": {"line": 0, "character": 0},
                "end": {"line": min(n_lines, 50), "character": 0},
            },
        }
        _step("documentSymbol", "textDocument/documentSymbol", td)
        _step("codeAction", "textDocument/codeAction", code_action_params)
        _step("documentLink", "textDocument/documentLink", td)
        _step("inlayHint", "textDocument/inlayHint", inlay_hint_params)
        _step("foldingRange", "textDocument/foldingRange", td)
        _step("documentSymbol", "textDocument/documentSymbol", td)

        result = _step("semanticTokens/full", "textDocument/semanticTokens/full", td)

        t_tokens = time.perf_counter()
        total_ms = (t_tokens - t_open) * 1000
        data = result.get("data", []) if result else []
        n_tokens = len(data) // 5

        # Wait for timing logs to arrive.
        time.sleep(0.5)

        timing_lines = client.get_timing_lines()
        server_timings: dict[str, float] = {}
        for line in timing_lines:
            m = _TIMING_RE.search(line)
            if m:
                server_timings[m.group(1)] = float(m.group(2))

        print(f"  Iteration {i + 1}:")
        print(f"    Total (didOpen → tokens): {total_ms:.0f}ms")
        print(f"    Tokens: {n_tokens}")
        print("    Request chain:")
        for name, ms in step_times:
            print(f"      {name}: {ms:.0f}ms")
        if server_timings:
            print("    Server timings:")
            for label, ms in sorted(server_timings.items()):
                print(f"      {label}: {ms:.0f}ms")
        print()

    # Also show a simulated edit benchmark if we have >1 iteration.
    if iterations > 1:
        lines = content.split("\n")
        if len(lines) > 10:
            lines[len(lines) // 2] = lines[len(lines) // 2] + " ;# bench-edit"
            edited = "\n".join(lines)
            client.clear_timing()
            client.send_notification(
                "textDocument/didChange",
                {
                    "textDocument": {"uri": uri, "version": iterations + 10},
                    "contentChanges": [{"text": edited}],
                },
            )
            time.sleep(0.01)
            t0 = time.perf_counter()
            client.send_request(
                "textDocument/semanticTokens/full",
                {"textDocument": {"uri": uri}},
            )
            t1 = time.perf_counter()
            time.sleep(0.3)
            timing_lines = client.get_timing_lines()
            server_timings = {}
            for line in timing_lines:
                m = _TIMING_RE.search(line)
                if m:
                    server_timings[m.group(1)] = float(m.group(2))
            print("  After mid-file edit:")
            print(f"    Wall clock: {(t1 - t0) * 1000:.1f}ms")
            if server_timings:
                print("    Server timings:")
                for label, ms in sorted(server_timings.items()):
                    print(f"      {label}: {ms:.0f}ms")
            print()


def cmd_logs(client: LspClient, uri: str, *, timing_only: bool = False) -> None:
    """Collect and display server logs and timing information."""
    # Wait for logs to accumulate.
    time.sleep(0.5)

    if timing_only:
        lines = client.get_timing_lines()
        print(f"=== Timing Logs ({len(lines)} entries) ===")
        for line in lines:
            print(f"  {line.strip()}")
    else:
        stderr = client.get_stderr_lines()
        log_msgs = client.get_log_messages()
        print(f"=== Server Stderr ({len(stderr)} lines) ===")
        for line in stderr:
            print(f"  {line}")
        print()
        print(f"=== Log Messages ({len(log_msgs)} entries) ===")
        for msg in log_msgs:
            print(f"  {msg}")

    # Always show timing summary at end.
    timing_lines = client.get_timing_lines()
    if timing_lines:
        print()
        print("=== Timing Summary ===")
        timings: dict[str, float] = {}
        for line in timing_lines:
            m = _TIMING_RE.search(line)
            if m:
                timings[m.group(1)] = float(m.group(2))
        for label, ms in sorted(timings.items()):
            print(f"  {label}: {ms:.0f}ms")


# CLI


def main() -> None:
    parser = argparse.ArgumentParser(
        description="LSP client for the Tcl language server",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""\
examples:
  %(prog)s semantic-tokens samples/for_screenshots/03-completions.tcl
  %(prog)s diagnostics editors/vscode/testFixture/diagnostics.tcl
  %(prog)s hover editors/vscode/testFixture/procs.tcl 1 6
  %(prog)s all samples/for_screenshots/03-completions.tcl
""",
    )
    parser.add_argument(
        "--server-dir", help="Path to tcl-lsp directory (auto-detected by default)"
    )
    parser.add_argument(
        "--server",
        choices=["python", "rust"],
        default=os.environ.get("TCL_LSP_SERVER_KIND", "rust"),
        help="Which LSP backend to drive: the native Rust binary (default) or the Python server.",
    )
    parser.add_argument(
        "--server-bin",
        help="Path to the native tcl-lsp-server binary (for --server rust; else auto-detected).",
    )
    parser.add_argument(
        "--scan-timeout",
        type=float,
        default=30.0,
        help=(
            "Seconds to wait for the background workspace scan to complete "
            "before cross-file subcommands (definition, references, "
            "diagnostics, code-actions, context, all) proceed. See "
            "issue #1094. Default: 30.0 — tuned against a measured "
            "worst-case scan (issue #1111): a workspace at "
            "WORKSPACE_SCAN_FILE_CAP (2000 files) took ~12.3s unloaded / "
            "~15.2s under 4-way CPU contention on a 4-core box in a *debug* "
            "build (a `--release` build did the same scan in ~1.9s), so the "
            "old 15.0s default left under 20% headroom over the unloaded "
            "debug-build worst case and none at all once loaded. 30.0 keeps "
            "roughly 2x headroom over the worst measured case and matches "
            "the Rust e2e harness's own `DEFAULT_TIMEOUT`."
        ),
    )
    parser.add_argument(
        "--also-open",
        action="append",
        default=[],
        metavar="FILE",
        help=(
            "Open an additional file (textDocument/didOpen) before <file> — "
            "the first-class multi-file helper for asserting cross-file "
            "behavior (e.g. a definition/reference in <file> that resolves "
            "into FILE, or a sibling-file completion). Repeatable. Opened "
            "after the workspace-scan wait (for a cross-file subcommand) and "
            "before <file>, in the order given, so the request the "
            "subcommand issues against <file> already sees every one of "
            "them. See issue #1111."
        ),
    )

    sub = parser.add_subparsers(dest="command", required=True)

    # semantic-tokens
    p = sub.add_parser("semantic-tokens", help="Decode and display semantic tokens")
    p.add_argument("file", help="Tcl file to analyze")

    # diagnostics
    p = sub.add_parser("diagnostics", help="Show diagnostics")
    p.add_argument("file", help="Tcl file to analyze")

    # format
    p = sub.add_parser("format", help="Show formatting edits")
    p.add_argument("file", help="Tcl file to format")

    # hover
    p = sub.add_parser("hover", help="Show hover info at position")
    p.add_argument("file", help="Tcl file")
    p.add_argument("line", type=int, help="Line (0-based)")
    p.add_argument("col", type=int, help="Column (0-based)")

    # completion
    p = sub.add_parser("completion", help="Show completions at position")
    p.add_argument("file", help="Tcl file")
    p.add_argument("line", type=int, help="Line (0-based)")
    p.add_argument("col", type=int, help="Column (0-based)")

    # definition
    p = sub.add_parser("definition", help="Show definition locations")
    p.add_argument("file", help="Tcl file")
    p.add_argument("line", type=int, help="Line (0-based)")
    p.add_argument("col", type=int, help="Column (0-based)")

    # references
    p = sub.add_parser("references", help="Show reference locations")
    p.add_argument("file", help="Tcl file")
    p.add_argument("line", type=int, help="Line (0-based)")
    p.add_argument("col", type=int, help="Column (0-based)")

    # code-lens
    p = sub.add_parser(
        "code-lens", help="Show code lenses (reference-count lenses), resolved"
    )
    p.add_argument("file", help="Tcl file to analyze")

    # code-actions
    p = sub.add_parser("code-actions", help="Show code actions in a range")
    p.add_argument("file", help="Tcl file")
    p.add_argument("line", type=int, help="Start line (0-based)")
    p.add_argument("col", type=int, help="Start column (0-based)")
    p.add_argument("end_line", type=int, help="End line (0-based)")
    p.add_argument("end_col", type=int, help="End column (0-based)")

    # optimize
    p = sub.add_parser(
        "optimize", help="Show optimization suggestions and rewritten source"
    )
    p.add_argument("file", help="Tcl file to optimize")

    # symbols
    p = sub.add_parser("symbols", help="Show document symbol hierarchy")
    p.add_argument("file", help="Tcl file to analyze")

    # diagram
    p = sub.add_parser(
        "diagram", help="Extract control flow diagram data from compiler IR"
    )
    p.add_argument("file", help="Tcl/iRule file to analyze")

    # event-info (no file needed)
    p = sub.add_parser("event-info", help="Show iRules event registry metadata")
    p.add_argument("event", help="iRules event name (e.g. HTTP_REQUEST)")

    # command-info (no file needed)
    p = sub.add_parser("command-info", help="Show iRules command registry metadata")
    p.add_argument("name", help="iRules command name (e.g. HTTP::uri)")

    # context
    p = sub.add_parser(
        "context", help="Build context pack: diagnostics + symbols + event metadata"
    )
    p.add_argument("file", help="Tcl file to analyze")

    # all
    p = sub.add_parser(
        "all", help="Run semantic-tokens + diagnostics + symbols + format + optimize"
    )
    p.add_argument("file", help="Tcl file to analyze")

    # bench
    p = sub.add_parser(
        "bench", help="Benchmark time-to-semantic-tokens with server timing breakdown"
    )
    p.add_argument("file", help="Tcl file to benchmark")
    p.add_argument(
        "--iterations",
        type=int,
        default=1,
        help="Number of benchmark iterations (default: 1)",
    )

    # logs
    p = sub.add_parser(
        "logs", help="Collect and display server logs and timing information"
    )
    p.add_argument("file", help="Tcl file to open (triggers server processing)")
    p.add_argument(
        "--timing-only", action="store_true", help="Show only [timing] entries"
    )

    args = parser.parse_args()

    # Find server (Python dir or native Rust binary)
    server_kind = (args.server or "python").strip().lower()
    launch_cmd: list[str] | None = None
    try:
        if server_kind == "rust":
            native = find_native_server(args.server_bin)
            # rootUri only needs a directory; the binary doesn't need a bundle.
            server_dir = args.server_dir or str(Path.cwd())
            launch_cmd = [native]
        else:
            server_dir = find_server_dir(args.server_dir)
    except FileNotFoundError as e:
        print(f"Error: {e}", file=sys.stderr)
        sys.exit(1)

    # Create client and run
    client = LspClient(server_dir, launch_cmd=launch_cmd)
    try:
        client.start()
        init_result = initialize(client)
        # The server is the source of truth for the semantic-tokens legend
        # (LSP encodes token types/modifiers as indices into the legend the
        # server declares in its `initialize` response). Overwrite the
        # hardcoded fallback lists above with the live legend so decoding
        # never drifts out of sync with the server as it evolves.
        legend = (
            init_result.get("capabilities", {})
            .get("semanticTokensProvider", {})
            .get("legend", {})
        )
        if legend.get("tokenTypes"):
            SEMANTIC_TOKEN_TYPES[:] = legend["tokenTypes"]
        if legend.get("tokenModifiers"):
            SEMANTIC_TOKEN_MODIFIERS[:] = legend["tokenModifiers"]

        # Commands that don't require a file
        if args.command == "event-info":
            # Set dialect to f5-irules for event registry access
            client.send_notification(
                "workspace/didChangeConfiguration",
                {"settings": {"tclLsp": {"dialect": "f5-irules"}}},
            )
            time.sleep(0.2)
            cmd_event_info(client, args.event)
        elif args.command == "command-info":
            client.send_notification(
                "workspace/didChangeConfiguration",
                {"settings": {"tclLsp": {"dialect": "f5-irules"}}},
            )
            time.sleep(0.2)
            cmd_command_info(client, args.name)
        else:
            # Commands that require a file
            ext = os.path.splitext(args.file)[1].lower()
            if ext in (".irul", ".irule"):
                dialect = "f5-irules"
            elif ext in (".iapp", ".iappimpl", ".impl"):
                dialect = "f5-iapps"
            else:
                dialect = None
            if dialect:
                client.send_notification(
                    "workspace/didChangeConfiguration",
                    {"settings": {"tclLsp": {"dialect": dialect}}},
                )

            # Cross-file-sensitive commands: wait out the background
            # workspace scan *before* opening the document, so the doc's
            # one diagnostics publish (and any definition/references
            # request issued below) already sees the fully-populated
            # workspace_index / package_resolver instead of racing the
            # scan (issue #1094).
            if args.command in CROSS_FILE_COMMANDS:
                client.wait_for_workspace_scan(timeout=args.scan_timeout)

            # `--also-open FILE` (repeatable): the multi-file helper (issue
            # #1111) — open every companion file *before* the main one, in
            # the order given, after the scan wait above so a cross-file
            # subcommand's request against `args.file` already sees them.
            # Each open gets the same post-didOpen settle main() gives the
            # primary file below, so a companion file's own diagnostics
            # publish (which can itself touch workspace state a sibling-file
            # completion/definition reads) has landed before we proceed.
            for companion in args.also_open:
                open_document(client, companion)
                time.sleep(0.3)

            uri, content = open_document(client, args.file)

            # Give server a moment to process didOpen and push diagnostics
            time.sleep(0.3)

            match args.command:
                case "semantic-tokens":
                    cmd_semantic_tokens(client, uri, content)
                case "diagnostics":
                    cmd_diagnostics(client, uri)
                case "format":
                    cmd_format(client, uri, content)
                case "hover":
                    cmd_hover(client, uri, args.line, args.col)
                case "completion":
                    cmd_completion(client, uri, args.line, args.col)
                case "definition":
                    cmd_definition(client, uri, args.line, args.col)
                case "references":
                    cmd_references(client, uri, args.line, args.col)
                case "code-lens":
                    cmd_code_lens(client, uri)
                case "code-actions":
                    cmd_code_actions(
                        client, uri, args.line, args.col, args.end_line, args.end_col
                    )
                case "optimize":
                    cmd_optimize(client, uri, content)
                case "symbols":
                    cmd_symbols(client, uri)
                case "diagram":
                    cmd_diagram(client, content)
                case "context":
                    cmd_context(client, uri, content)
                case "all":
                    cmd_all(client, uri, content)
                case "bench":
                    cmd_bench(client, uri, content, iterations=args.iterations)
                case "logs":
                    cmd_logs(client, uri, timing_only=args.timing_only)

    except FileNotFoundError as e:
        print(f"Error: {e}", file=sys.stderr)
        sys.exit(1)
    except TimeoutError as e:
        print(f"Timeout: {e}", file=sys.stderr)
        sys.exit(1)
    except RuntimeError as e:
        print(f"LSP error: {e}", file=sys.stderr)
        sys.exit(1)
    except KeyboardInterrupt:
        pass
    finally:
        client.shutdown()


if __name__ == "__main__":
    main()
