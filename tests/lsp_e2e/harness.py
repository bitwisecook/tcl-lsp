"""Reusable JSON-RPC client for driving a packaged LSP server (.pyz).

This is the harness the ``tests/lsp_e2e`` suite is built on: start one
server, then run many requests against it.  It speaks the same
Content-Length-framed JSON-RPC an editor speaks, over the real
``tcl-lsp-server.pyz`` subprocess, so the assertions exercise the shipped
artifact end-to-end (the version banner regression these tests guard
against only manifested in *packaged* builds).

A background reader thread demultiplexes the stream so a long-lived server
can interleave responses, server-initiated requests, and notifications
(``window/logMessage``, ``textDocument/publishDiagnostics``, …) without the
test having to poll in lockstep:

- responses (have ``id``, no ``method``)         -> wake the waiting request
- server->client requests (``id`` + ``method``) -> auto-answered so the
  server never blocks waiting on us
- notifications (``method``, no ``id``)          -> queued for inspection

To write a test, take the ``lsp_server`` fixture and call ``request`` /
``notify`` / ``open_document`` / ``await_notification``.
"""

from __future__ import annotations

import json
import os
import subprocess
import threading
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[2]
BUILD_INFO = ROOT / "shared" / "_build_info.py"

# Used only when no build-info file has been generated yet (bare checkout /
# before ``make build-info``); a value that is unmistakably not the "dev"
# fallback so the version assertion stays meaningful.
SENTINEL_VERSION = "0.0.0-lsp-e2e-probe"


def _render_build_info(version: str) -> str:
    """Mirror the Makefile's BUILD_INFO recipe (scripts/test-slow-stamp.sh)."""
    return (
        '"""Generated at build time — do not edit."""\n\n'
        f'VERSION: str = "{version}"\n'
        f'GIT_DESCRIBE: str = "{version}"\n'
        f'GIT_HASH: str = "0000000"\n'
        f'FULL_VERSION: str = "{version}"\n'
        f'BUILD_TIMESTAMP: str = "1970-01-01T00:00:00Z"\n'
    )


def ensure_build_info() -> str:
    """Guarantee ``shared/_build_info.py`` exists; return its FULL_VERSION.

    The packaged pyz bundles whatever this file contains at build time, and
    the running server reports it back as ``serverInfo.version``.  Leaving a
    freshly written file in place is safe: it is gitignored and any ``make``
    target regenerates it (deleting it mid-run would race parallel workers).
    """
    if not BUILD_INFO.exists():
        BUILD_INFO.write_text(_render_build_info(SENTINEL_VERSION), encoding="utf-8")
    ns: dict[str, object] = {}
    exec(compile(BUILD_INFO.read_text(encoding="utf-8"), str(BUILD_INFO), "exec"), ns)
    full = ns.get("FULL_VERSION")
    assert isinstance(full, str) and full, "shared/_build_info.py has no FULL_VERSION"
    return full


class LspError(AssertionError):
    """Raised when the server returns a JSON-RPC error to a request."""


class LspServerClient:
    """Manage a language-server subprocess and talk LSP JSON-RPC to it."""

    def __init__(self, argv: list[str], cwd: Path) -> None:
        self._argv = argv
        self._cwd = cwd
        self._proc: subprocess.Popen[bytes] | None = None
        self._next_id = 0
        self._lock = threading.Lock()
        self._responses: dict[int, dict] = {}
        self._events: dict[int, threading.Event] = {}
        self._notifications: list[dict] = []
        self._notify_cv = threading.Condition()
        self._stderr: list[bytes] = []
        self._reader: threading.Thread | None = None
        self._stderr_thread: threading.Thread | None = None
        #: ``serverInfo`` from the initialize result, populated by initialize().
        self.server_info: dict | None = None
        self.initialize_result: dict | None = None

    # -- lifecycle ---------------------------------------------------------

    def start(self) -> None:
        self._proc = subprocess.Popen(
            self._argv,
            cwd=str(self._cwd),
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
        self._reader = threading.Thread(target=self._read_loop, daemon=True)
        self._reader.start()
        self._stderr_thread = threading.Thread(target=self._drain_stderr, daemon=True)
        self._stderr_thread.start()

    def initialize(
        self,
        *,
        root_uri: str | None = None,
        capabilities: dict | None = None,
        timeout: float = 60.0,
    ) -> dict:
        """Run the initialize handshake; return the InitializeResult."""
        result = self.request(
            "initialize",
            {
                "processId": os.getpid(),
                "rootUri": root_uri,
                "workspaceFolders": ([{"uri": root_uri, "name": "e2e"}] if root_uri else None),
                "capabilities": capabilities or {},
                "clientInfo": {"name": "tcl-lsp-e2e", "version": "1.0"},
            },
            timeout=timeout,
        )
        self.initialize_result = result
        self.server_info = result.get("serverInfo")
        self.notify("initialized", {})
        return result

    def shutdown(self) -> None:
        if self._proc is None:
            return
        try:
            self.request("shutdown", None, timeout=10.0)
            self.notify("exit")
        except Exception:  # pragma: no cover - best effort
            pass
        try:
            self._proc.wait(timeout=10.0)
        except subprocess.TimeoutExpired:  # pragma: no cover
            self._proc.kill()
            self._proc.wait(timeout=5.0)

    # -- requests / notifications -----------------------------------------

    def request(self, method: str, params: Any = None, *, timeout: float = 60.0) -> Any:
        """Send a request and return its result (raises LspError on error)."""
        with self._lock:
            msg_id = self._next_id
            self._next_id += 1
            event = threading.Event()
            self._events[msg_id] = event
        self._send({"jsonrpc": "2.0", "id": msg_id, "method": method, "params": params})
        if not event.wait(timeout):
            raise AssertionError(
                f"timed out after {timeout}s waiting for response to {method!r}; "
                f"stderr:\n{self.stderr_text()}"
            )
        with self._lock:
            response = self._responses.pop(msg_id)
            self._events.pop(msg_id, None)
        if "error" in response:
            raise LspError(f"{method} -> {response['error']}")
        return response.get("result")

    def notify(self, method: str, params: Any = None) -> None:
        self._send({"jsonrpc": "2.0", "method": method, "params": params})

    def open_document(
        self, uri: str, text: str, *, language_id: str = "tcl", version: int = 1
    ) -> None:
        self.notify(
            "textDocument/didOpen",
            {
                "textDocument": {
                    "uri": uri,
                    "languageId": language_id,
                    "version": version,
                    "text": text,
                }
            },
        )

    def await_notification(self, method: str, *, timeout: float = 30.0) -> dict:
        """Block until a notification with ``method`` arrives; return it."""
        import time as _time

        deadline = _time.monotonic() + timeout
        with self._notify_cv:
            while True:
                for note in self._notifications:
                    if note.get("method") == method:
                        return note
                remaining = deadline - _time.monotonic()
                if remaining <= 0:
                    raise AssertionError(
                        f"no {method!r} notification within {timeout}s; "
                        f"seen: {[n.get('method') for n in self._notifications]}"
                    )
                self._notify_cv.wait(remaining)

    def notifications(self) -> list[dict]:
        with self._notify_cv:
            return list(self._notifications)

    def stderr_text(self) -> str:
        return b"".join(self._stderr).decode("utf-8", "replace")

    # -- internals ---------------------------------------------------------

    def _send(self, payload: dict) -> None:
        assert self._proc is not None and self._proc.stdin is not None
        body = json.dumps(payload).encode("utf-8")
        self._proc.stdin.write(b"Content-Length: %d\r\n\r\n%b" % (len(body), body))
        self._proc.stdin.flush()

    def _read_loop(self) -> None:
        assert self._proc is not None and self._proc.stdout is not None
        stdout = self._proc.stdout
        while True:
            length = 0
            while True:
                line = stdout.readline()
                if not line:
                    return  # EOF: server exited
                text = line.decode("utf-8").rstrip("\r\n")
                if not text:
                    break  # blank line terminates headers
                if text.lower().startswith("content-length:"):
                    length = int(text.split(":", 1)[1].strip())
            if length <= 0:
                continue
            body = self._read_exactly(stdout, length)
            if body is None:
                return
            try:
                msg = json.loads(body.decode("utf-8"))
            except json.JSONDecodeError:  # pragma: no cover
                continue
            self._route(msg)

    @staticmethod
    def _read_exactly(stream: Any, n: int) -> bytes | None:
        chunks: list[bytes] = []
        remaining = n
        while remaining > 0:
            chunk = stream.read(remaining)
            if not chunk:
                return None
            chunks.append(chunk)
            remaining -= len(chunk)
        return b"".join(chunks)

    def _route(self, msg: dict) -> None:
        has_id = "id" in msg and msg["id"] is not None
        is_request = "method" in msg
        if has_id and not is_request:
            # Response to one of our requests.
            with self._lock:
                msg_id = msg["id"]
                self._responses[msg_id] = msg
                event = self._events.get(msg_id)
            if event is not None:
                event.set()
        elif has_id and is_request:
            # Server-initiated request — answer so the server never blocks.
            self._auto_reply(msg)
        else:
            # Notification.
            with self._notify_cv:
                self._notifications.append(msg)
                self._notify_cv.notify_all()

    def _auto_reply(self, msg: dict) -> None:
        method = msg.get("method", "")
        if method == "workspace/configuration":
            items = (msg.get("params") or {}).get("items") or []
            result: Any = [None] * len(items)
        else:
            result = None
        self._send({"jsonrpc": "2.0", "id": msg["id"], "result": result})

    def _drain_stderr(self) -> None:
        assert self._proc is not None and self._proc.stderr is not None
        for line in iter(self._proc.stderr.readline, b""):
            self._stderr.append(line)
