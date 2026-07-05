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

import contextlib
import json
import os
import shutil
import subprocess
import sys
import tempfile
import threading
from collections.abc import Callable
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[2]
BUILD_INFO = ROOT / "shared" / "_build_info.py"

# -- server selection (Python pyz vs native Rust binary) -------------------
#
# The same JSON-RPC battery can drive either backend.  ``TCL_LSP_SERVER_KIND``
# selects which:
#   * ``python`` (default) — the packaged ``tcl-lsp-server-<v>.pyz`` run under
#     the current interpreter (the shipped Python LSP server).
#   * ``rust``             — the native ``tcl-lsp-server`` binary, run directly
#     over stdio.
#
# In ``rust`` mode the binary is located via ``TCL_LSP_SERVER_BIN`` (explicit
# path) or discovered at ``target/{release,debug}/tcl-lsp-server`` under the
# repo root (build it with ``make rust-server`` / ``cargo build -p
# tcl-lsp-server``).  Keeping the selector here — rather than in conftest —
# lets every fixture and any out-of-band script share one resolution rule.


def server_kind() -> str:
    """Return the selected LSP backend: ``"python"`` (default) or ``"rust"``."""
    kind = os.environ.get("TCL_LSP_SERVER_KIND", "python").strip().lower()
    return "rust" if kind in {"rust", "native"} else "python"


def native_server_bin() -> Path | None:
    """Resolve the native Rust ``tcl-lsp-server`` binary for ``rust`` mode.

    Honours ``TCL_LSP_SERVER_BIN`` first, then a release/debug build under the
    repo's ``target/``.  Returns ``None`` when nothing is built yet.
    """
    explicit = os.environ.get("TCL_LSP_SERVER_BIN")
    if explicit:
        return Path(explicit).resolve()
    for profile in ("release", "debug"):
        candidate = ROOT / "target" / profile / "tcl-lsp-server"
        if candidate.exists():
            return candidate
    return None


def server_launch_argv(server: Path) -> list[str]:
    """Build the subprocess argv for *server*.

    A native binary runs directly; a ``.pyz`` runs under the current
    interpreter.
    """
    return [str(server)] if server_kind() == "rust" else [sys.executable, str(server)]


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

    def __init__(
        self,
        argv: list[str],
        cwd: Path,
        *,
        tcllsp_config: dict | None = None,
    ) -> None:
        self._argv = argv
        self._cwd = cwd
        #: Reply returned for the ``tclLsp`` section of ``workspace/configuration``.
        #: Defaults to an editor that has opted into linked editing only (see
        #: ``_auto_reply``); pass a richer dict to enable default-off features
        #: such as inlay hints for a dedicated server fixture.
        self._tcllsp_config: dict = (
            tcllsp_config
            if tcllsp_config is not None
            else {"features": {"linkedEditingRange": True}}
        )
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
        #: Throwaway XDG root isolating the server's config/cache (see start()).
        self._xdg_root: Path | None = None
        #: ``serverInfo`` from the initialize result, populated by initialize().
        self.server_info: dict | None = None
        self.initialize_result: dict | None = None

    # -- lifecycle ---------------------------------------------------------

    def start(self) -> None:
        # Isolate the server from the developer machine's user config/cache so a
        # local ``~/.../tcl-lsp/config.ini`` (e.g. ``[features] hover = false``)
        # can't poison the default e2e suites.  ``XDG_CONFIG_HOME`` /
        # ``XDG_CACHE_HOME`` take precedence on every platform (incl. macOS's
        # ``~/Library/Application Support``), so pointing them at fresh empty
        # dirs makes the server fall back to built-in defaults.  Config-specific
        # suites opt into their own settings via ``workspace/configuration``.
        self._xdg_root = Path(tempfile.mkdtemp(prefix="tcl-lsp-e2e-xdg-"))
        env = {
            **os.environ,
            "XDG_CONFIG_HOME": str(self._xdg_root / "config"),
            "XDG_CACHE_HOME": str(self._xdg_root / "cache"),
        }
        self._proc = subprocess.Popen(
            self._argv,
            cwd=str(self._cwd),
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            env=env,
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
        finally:
            if self._xdg_root is not None:
                shutil.rmtree(self._xdg_root, ignore_errors=True)
                self._xdg_root = None

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

    def change_document(
        self,
        uri: str,
        version: int,
        changes: list[dict],
    ) -> None:
        """Send a ``didChange`` with arbitrary content changes.

        ``changes`` is the raw LSP ``contentChanges`` array — each entry is
        either a full replace (``{"text": ...}``) or an incremental edit
        (``{"range": {...}, "text": ...}``).  The server advertises
        incremental sync (``change == 2``), so range edits exercise the
        real per-edit buffer-tracking path an editor drives.
        """
        self.notify(
            "textDocument/didChange",
            {
                "textDocument": {"uri": uri, "version": version},
                "contentChanges": changes,
            },
        )

    def replace_document(self, uri: str, version: int, text: str) -> None:
        """Full-document replace ``didChange`` (single ``{"text": ...}``)."""
        self.change_document(uri, version, [{"text": text}])

    def close_document(self, uri: str) -> None:
        self.notify("textDocument/didClose", {"textDocument": {"uri": uri}})

    def open_ready(
        self,
        uri: str,
        text: str,
        *,
        language_id: str = "tcl",
        version: int = 1,
        timeout: float = 30.0,
    ) -> list[dict]:
        """Open ``text`` and block until its analysis is ready, then return diags.

        Analysis-backed handlers (hover, definition, references, …) read the
        per-document *analysis snapshot*, which is built on the main thread.
        The first ``publishDiagnostics`` can arrive *before* that snapshot
        exists — the fast diagnostics pass runs in a subprocess — so waiting on
        diagnostics alone leaves a window where hover returns ``None``.  Block
        on both: the diagnostics publish (for the version handshake and the
        returned diagnostics) and the ``workspace_state.update`` log line that
        the server emits for this URI once the snapshot is populated.
        """
        self.open_document(uri, text, language_id=language_id, version=version)
        diags = self.await_diagnostics(uri, version=version, timeout=timeout)
        # The snapshot-built marker carries ``uri=<uri>``; for a fresh unique
        # URI there is exactly one, so a substring match can't catch a stale
        # earlier build.
        self.await_log("workspace_state.update", uri, timeout=timeout)
        return diags

    def notification_cursor(self) -> int:
        """A marker into the notification log for ``await_log(..., since=)``."""
        with self._notify_cv:
            return len(self._notifications)

    def await_log(self, *needles: str, timeout: float = 30.0, since: int = 0) -> str:
        """Block until a ``window/logMessage`` whose text contains all ``needles``.

        The packaged server routes its ``[timing] …`` instrumentation through
        ``window/logMessage`` (it emits nothing on stderr), so this is how a
        test observes server-side progress markers such as the analysis-snapshot
        build.  ``since`` skips log entries recorded before a
        :meth:`notification_cursor` marker, so a freshly produced marker can be
        distinguished from identical earlier ones (e.g. the per-edit
        ``workspace_state.update`` line for a long-lived document).  Returns the
        matching message text.
        """
        import time as _time

        deadline = _time.monotonic() + timeout
        with self._notify_cv:
            while True:
                for note in self._notifications[since:]:
                    if note.get("method") != "window/logMessage":
                        continue
                    msg = str((note.get("params") or {}).get("message", ""))
                    if all(n in msg for n in needles):
                        return msg
                remaining = deadline - _time.monotonic()
                if remaining <= 0:
                    raise AssertionError(
                        f"no window/logMessage containing all of {needles!r} within {timeout}s"
                    )
                self._notify_cv.wait(remaining)

    def settle_analysis(self, uri: str, version: int, text: str, *, timeout: float = 30.0) -> int:
        """Force ``uri``'s analysis snapshot to ``text`` and block until it is built.

        Sends a no-op full-replace at ``version`` (the content is unchanged, so
        the buffer is identical to the post-edit text) and waits for *that*
        version's diagnostics plus the per-URI ``workspace_state.update`` log
        recorded after the replace — guaranteeing the analysis-backed features
        read the final content rather than a snapshot still trailing behind a
        burst of incremental edits.  Returns ``version``.
        """
        cursor = self.notification_cursor()
        self.replace_document(uri, version, text)
        self.await_diagnostics(uri, version=version, timeout=timeout)
        self.await_log("workspace_state.update", uri, since=cursor, timeout=timeout)
        return version

    # -- feature requests --------------------------------------------------

    @staticmethod
    def _pos(line: int, char: int) -> dict:
        return {"line": line, "character": char}

    def _doc_pos(self, uri: str, line: int, char: int) -> dict:
        return {"textDocument": {"uri": uri}, "position": self._pos(line, char)}

    def hover(self, uri: str, line: int, char: int, *, timeout: float = 30.0) -> Any:
        return self.request("textDocument/hover", self._doc_pos(uri, line, char), timeout=timeout)

    def completion(self, uri: str, line: int, char: int, *, timeout: float = 30.0) -> Any:
        return self.request(
            "textDocument/completion", self._doc_pos(uri, line, char), timeout=timeout
        )

    def signature_help(self, uri: str, line: int, char: int, *, timeout: float = 30.0) -> Any:
        return self.request(
            "textDocument/signatureHelp", self._doc_pos(uri, line, char), timeout=timeout
        )

    def definition(self, uri: str, line: int, char: int, *, timeout: float = 30.0) -> Any:
        return self.request(
            "textDocument/definition", self._doc_pos(uri, line, char), timeout=timeout
        )

    def type_definition(self, uri: str, line: int, char: int, *, timeout: float = 30.0) -> Any:
        return self.request(
            "textDocument/typeDefinition", self._doc_pos(uri, line, char), timeout=timeout
        )

    def declaration(self, uri: str, line: int, char: int, *, timeout: float = 30.0) -> Any:
        return self.request(
            "textDocument/declaration", self._doc_pos(uri, line, char), timeout=timeout
        )

    def references(
        self,
        uri: str,
        line: int,
        char: int,
        *,
        include_declaration: bool = True,
        timeout: float = 30.0,
    ) -> Any:
        params = self._doc_pos(uri, line, char)
        params["context"] = {"includeDeclaration": include_declaration}
        return self.request("textDocument/references", params, timeout=timeout)

    def document_highlight(self, uri: str, line: int, char: int, *, timeout: float = 30.0) -> Any:
        return self.request(
            "textDocument/documentHighlight", self._doc_pos(uri, line, char), timeout=timeout
        )

    def document_symbols(self, uri: str, *, timeout: float = 30.0) -> Any:
        return self.request(
            "textDocument/documentSymbol", {"textDocument": {"uri": uri}}, timeout=timeout
        )

    def workspace_symbols(self, query: str, *, timeout: float = 30.0) -> Any:
        return self.request("workspace/symbol", {"query": query}, timeout=timeout)

    def semantic_tokens(self, uri: str, *, timeout: float = 30.0) -> Any:
        return self.request(
            "textDocument/semanticTokens/full", {"textDocument": {"uri": uri}}, timeout=timeout
        )

    def implementation(self, uri: str, line: int, char: int, *, timeout: float = 30.0) -> Any:
        return self.request(
            "textDocument/implementation", self._doc_pos(uri, line, char), timeout=timeout
        )

    def document_links(self, uri: str, *, timeout: float = 30.0) -> Any:
        return self.request(
            "textDocument/documentLink", {"textDocument": {"uri": uri}}, timeout=timeout
        )

    def code_lens(self, uri: str, *, timeout: float = 30.0) -> Any:
        return self.request(
            "textDocument/codeLens", {"textDocument": {"uri": uri}}, timeout=timeout
        )

    def code_lens_resolve(self, lens: dict, *, timeout: float = 30.0) -> Any:
        return self.request("codeLens/resolve", lens, timeout=timeout)

    def inlay_hints(
        self, uri: str, start: tuple[int, int], end: tuple[int, int], *, timeout: float = 30.0
    ) -> Any:
        params = {
            "textDocument": {"uri": uri},
            "range": {"start": self._pos(*start), "end": self._pos(*end)},
        }
        return self.request("textDocument/inlayHint", params, timeout=timeout)

    def linked_editing_range(self, uri: str, line: int, char: int, *, timeout: float = 30.0) -> Any:
        return self.request(
            "textDocument/linkedEditingRange", self._doc_pos(uri, line, char), timeout=timeout
        )

    def prepare_call_hierarchy(
        self, uri: str, line: int, char: int, *, timeout: float = 30.0
    ) -> Any:
        return self.request(
            "textDocument/prepareCallHierarchy", self._doc_pos(uri, line, char), timeout=timeout
        )

    def incoming_calls(self, item: dict, *, timeout: float = 30.0) -> Any:
        return self.request("callHierarchy/incomingCalls", {"item": item}, timeout=timeout)

    def outgoing_calls(self, item: dict, *, timeout: float = 30.0) -> Any:
        return self.request("callHierarchy/outgoingCalls", {"item": item}, timeout=timeout)

    def range_formatting(
        self, uri: str, start: tuple[int, int], end: tuple[int, int], *, timeout: float = 30.0
    ) -> Any:
        params = {
            "textDocument": {"uri": uri},
            "range": {"start": self._pos(*start), "end": self._pos(*end)},
            "options": {"tabSize": 4, "insertSpaces": True},
        }
        return self.request("textDocument/rangeFormatting", params, timeout=timeout)

    def prepare_rename(self, uri: str, line: int, char: int, *, timeout: float = 30.0) -> Any:
        return self.request(
            "textDocument/prepareRename", self._doc_pos(uri, line, char), timeout=timeout
        )

    def rename(
        self, uri: str, line: int, char: int, new_name: str, *, timeout: float = 30.0
    ) -> Any:
        params = self._doc_pos(uri, line, char)
        params["newName"] = new_name
        return self.request("textDocument/rename", params, timeout=timeout)

    def code_actions(
        self,
        uri: str,
        start: tuple[int, int],
        end: tuple[int, int],
        *,
        diagnostics: list[dict] | None = None,
        only: list[str] | None = None,
        timeout: float = 30.0,
    ) -> Any:
        context: dict[str, Any] = {"diagnostics": diagnostics or []}
        if only is not None:
            context["only"] = only
        params = {
            "textDocument": {"uri": uri},
            "range": {"start": self._pos(*start), "end": self._pos(*end)},
            "context": context,
        }
        return self.request("textDocument/codeAction", params, timeout=timeout)

    def folding_range(self, uri: str, *, timeout: float = 30.0) -> Any:
        return self.request(
            "textDocument/foldingRange", {"textDocument": {"uri": uri}}, timeout=timeout
        )

    def selection_range(
        self, uri: str, positions: list[tuple[int, int]], *, timeout: float = 30.0
    ) -> Any:
        params = {
            "textDocument": {"uri": uri},
            "positions": [self._pos(line, char) for line, char in positions],
        }
        return self.request("textDocument/selectionRange", params, timeout=timeout)

    def formatting(
        self,
        uri: str,
        *,
        tab_size: int = 4,
        insert_spaces: bool = True,
        timeout: float = 30.0,
    ) -> Any:
        params = {
            "textDocument": {"uri": uri},
            "options": {"tabSize": tab_size, "insertSpaces": insert_spaces},
        }
        return self.request("textDocument/formatting", params, timeout=timeout)

    def execute_command(self, command: str, arguments: list[Any], *, timeout: float = 30.0) -> Any:
        return self.request(
            "workspace/executeCommand",
            {"command": command, "arguments": arguments},
            timeout=timeout,
        )

    # -- configuration injection ------------------------------------------

    def effective_config(self, uri: str = "", *, timeout: float = 30.0) -> dict:
        """Return the server's *resolved* config for ``uri`` (``getEffectiveConfig``).

        This is the view the analyser/formatter actually applies — folder
        override → workspace fallback → process default — so a test can assert
        the server honoured a config change rather than sleeping on the
        ``workspace/configuration`` round-trip.
        """
        result = self.execute_command("tcl-lsp.getEffectiveConfig", [uri], timeout=timeout)
        return result if isinstance(result, dict) else {}

    def apply_configuration(
        self,
        config: dict,
        *,
        settle: Callable[[dict], bool] | None = None,
        settle_uri: str = "",
        timeout: float = 30.0,
    ) -> dict:
        """Make the server adopt *config* as the ``tclLsp`` section, then settle.

        Updates the reply this client returns for the ``tclLsp`` section of
        ``workspace/configuration`` and notifies ``didChangeConfiguration`` so
        the server re-pulls and re-applies.  The pull/apply is async, so this
        blocks until ``getEffectiveConfig`` for ``settle_uri`` satisfies
        *settle* (a predicate over the resolved config) — turning a config
        change into a deterministic barrier with no wall-clock sleeps.

        Returns the resolved config the server settled on.  Tests must
        restore the prior config (or use a dedicated server fixture) to avoid
        leaking state across the shared session — see ``config_session``.
        """
        self._tcllsp_config = config
        # An empty/None ``settings`` payload makes the server fall through to a
        # full ``workspace/configuration`` re-pull (see settings.register).
        self.notify("workspace/didChangeConfiguration", {"settings": {}})
        if settle is None:
            return self.effective_config(settle_uri, timeout=timeout)
        import time as _time

        deadline = _time.monotonic() + timeout
        last: dict = {}
        while _time.monotonic() < deadline:
            last = self.effective_config(settle_uri, timeout=timeout)
            if settle(last):
                return last
            _time.sleep(0.05)
        raise AssertionError(
            f"config did not settle within {timeout}s; last effective config: {last!r}"
        )

    @contextlib.contextmanager
    def config_session(
        self,
        config: dict,
        *,
        settle: Callable[[dict], bool] | None = None,
        settle_uri: str = "",
        timeout: float = 30.0,
    ):
        """Apply *config* for the duration of the ``with`` block, then restore.

        Snapshots the prior ``tclLsp`` reply *and* the resolved feature/scalar
        state, applies *config* (settling as :meth:`apply_configuration` does),
        and on exit restores the prior reply and re-pulls — blocking until the
        resolved config matches the pre-block snapshot again.  The settled
        restore is what makes this safe on the *shared* server: a feature
        toggled off here is provably back on before the next test queries it.
        The restore runs even if the body raises.
        """
        before = self.effective_config(settle_uri, timeout=timeout)
        before_features = before.get("features")
        before_optimiser = before.get("optimiser_enabled")

        # Defined outside the ``finally`` below (rather than inside it) so its
        # ``return``s belong unambiguously to this predicate and never read as
        # control flow escaping the finally (CodeQL py/exit-from-finally).
        def _restored(c: dict) -> bool:
            if isinstance(before_features, dict) and c.get("features") != before_features:
                return False
            if "optimiser" in config and c.get("optimiser_enabled") != before_optimiser:
                return False
            return True

        self.apply_configuration(config, settle=settle, settle_uri=settle_uri, timeout=timeout)
        try:
            yield self
        finally:
            # A pulled config only *sets* the keys it carries — omitted keys keep
            # their last-applied value (the server treats absent as "unchanged").
            # So restoring the prior *reply* would leave a disabled toggle stuck
            # off.  Re-assert the full prior resolved feature map (and optimiser
            # switch) explicitly, then block until the resolved state matches the
            # pre-block snapshot — the settled restore is what keeps the shared
            # server clean for the next test.
            undo: dict[str, Any] = {}
            if isinstance(before_features, dict):
                undo["features"] = dict(before_features)
            if "optimiser" in config and isinstance(before_optimiser, bool):
                undo["optimiser"] = {"enabled": before_optimiser}

            restored = self.apply_configuration(
                undo, settle=_restored, settle_uri=settle_uri, timeout=timeout
            )
            # Surface a botched restore loudly rather than leaking into the
            # next test on the shared server.
            assert _restored(restored), (
                "config_session failed to restore prior config: "
                f"features={restored.get('features')!r} (want {before_features!r}), "
                f"optimiser_enabled={restored.get('optimiser_enabled')!r} "
                f"(want {before_optimiser!r})"
            )

    # -- push diagnostics --------------------------------------------------

    def await_diagnostics(
        self,
        uri: str,
        *,
        version: int | None = None,
        timeout: float = 30.0,
    ) -> list[dict]:
        """Block until a ``publishDiagnostics`` for ``uri`` arrives; return it.

        The server pushes diagnostics (it advertises no pull provider), debounced
        per document and tagged with the document version.  When ``version`` is
        given, wait for the publish carrying *exactly* that version so a stale
        earlier publish can't satisfy the wait — the supersession contract the
        async pipeline guards.  Returns the ``diagnostics`` array of the matching
        publish.
        """
        import time as _time

        deadline = _time.monotonic() + timeout
        with self._notify_cv:
            while True:
                match = None
                for note in self._notifications:
                    if note.get("method") != "textDocument/publishDiagnostics":
                        continue
                    params = note.get("params") or {}
                    if params.get("uri") != uri:
                        continue
                    if version is not None and params.get("version") != version:
                        continue
                    match = params  # keep the latest matching publish
                if match is not None:
                    return list(match.get("diagnostics") or [])
                remaining = deadline - _time.monotonic()
                if remaining <= 0:
                    seen = [
                        (n["params"].get("uri"), n["params"].get("version"))
                        for n in self._notifications
                        if n.get("method") == "textDocument/publishDiagnostics"
                    ]
                    raise AssertionError(
                        f"no publishDiagnostics for {uri!r} "
                        f"{'version ' + str(version) if version is not None else ''} "
                        f"within {timeout}s; saw {seen}"
                    )
                self._notify_cv.wait(remaining)

    def clear_diagnostics_log(self) -> None:
        """Drop buffered notifications so a later ``await_diagnostics`` only sees fresh ones."""
        with self._notify_cv:
            self._notifications.clear()

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
            # Reply per requested section.  For the ``tclLsp`` section, return
            # this client's configured payload — by default an editor that has
            # opted into the features the schema leaves off (linked editing
            # inherits ``editor.linkedEditing``, off in VS Code) so those
            # providers are exercised in e2e; a dedicated fixture can pass a
            # richer dict (e.g. ``features.inlayTypeHints``).  Every other section
            # falls back to ``null`` (server defaults).
            result: Any = [
                self._tcllsp_config if (item or {}).get("section") == "tclLsp" else None
                for item in items
            ]
        else:
            result = None
        self._send({"jsonrpc": "2.0", "id": msg["id"], "result": result})

    def _drain_stderr(self) -> None:
        assert self._proc is not None and self._proc.stderr is not None
        for line in iter(self._proc.stderr.readline, b""):
            self._stderr.append(line)
