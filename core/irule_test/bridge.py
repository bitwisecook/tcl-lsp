"""Python bridge to the Tcl iRule test framework.

Supports two execution modes (selected automatically):

1. **In-process** via ``tkinter.Tcl()`` — preferred when Python is linked
   with Tcl (common on most installs).  No subprocess, no PATH dependency.
2. **Subprocess** via ``tclsh`` + ``runner.tcl`` JSON protocol — fallback
   when ``tkinter`` is unavailable.

For simpler use cases, the Tcl orchestrator can be used directly
without Python -- see ``orchestrator.tcl``.
"""

from __future__ import annotations

import asyncio
import json
import logging
import shutil
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

log = logging.getLogger(__name__)

# Path to the Tcl framework files
_TCL_DIR = Path(__file__).parent / "tcl"

# Framework source files in load order (before TMM init locks down the env)
_FRAMEWORK_FILES = [
    "compat84.tcl",
    "state_layers.tcl",
    "tmm_shim.tcl",
    "expr_ops.tcl",
    "command_mocks.tcl",
    "_mock_stubs.tcl",
    "itest_core.tcl",
    "orchestrator.tcl",
]


# Data classes


@dataclass
class EventResult:
    """Result of firing a single event."""

    event: str
    fired: bool
    handlers: list[dict[str, Any]] = field(default_factory=list)
    reason: str = ""


@dataclass
class RequestResult:
    """Result of a complete HTTP request/response cycle.

    Provides both data attributes and assertion methods for concise testing::

        result = await session.run_http_request(host="api.example.com")
        result.assert_pool("api_pool")
        result.assert_decision("lb", "pool_select", "api_pool")
        result.assert_no_decision("connection", "reject")
        result.assert_log_matches("*api*")
    """

    events_fired: list[EventResult] = field(default_factory=list)
    pool_selected: str = ""
    node_selected: str = ""
    decisions: list[tuple[str, str, Any]] = field(default_factory=list)
    logs: list[tuple[str, str, str]] = field(default_factory=list)
    connection_state: str = ""
    http_response_committed: bool = False

    # Assertion methods

    def assert_pool(self, expected: str, msg: str = "") -> None:
        """Assert the pool selected matches *expected*."""
        if self.pool_selected != expected:
            detail = msg or f'expected pool "{expected}" but got "{self.pool_selected}"'
            raise AssertionError(detail)

    def assert_node(self, expected: str, msg: str = "") -> None:
        """Assert the node selected matches *expected*."""
        if self.node_selected != expected:
            detail = msg or f'expected node "{expected}" but got "{self.node_selected}"'
            raise AssertionError(detail)

    def assert_decision(
        self, category: str, action: str, value: str | None = None, msg: str = ""
    ) -> None:
        """Assert a decision was logged, optionally with a specific value."""
        for d in self.decisions:
            d_cat = d[0] if isinstance(d, (list, tuple)) else ""
            d_act = d[1] if isinstance(d, (list, tuple)) and len(d) > 1 else ""
            d_args = d[2] if isinstance(d, (list, tuple)) and len(d) > 2 else ""
            if d_cat == category and d_act == action:
                if value is None:
                    return
                # d_args can be a list or string
                if isinstance(d_args, (list, tuple)):
                    if d_args and d_args[0] == value:
                        return
                elif str(d_args) == value:
                    return
        detail = msg or f"expected decision: {category} {action}"
        if value is not None:
            detail += f" {value}"
        detail += f" (got: {self.decisions})"
        raise AssertionError(detail)

    def assert_no_decision(self, category: str, action: str, msg: str = "") -> None:
        """Assert no decision was logged for *category*/*action*."""
        for d in self.decisions:
            d_cat = d[0] if isinstance(d, (list, tuple)) else ""
            d_act = d[1] if isinstance(d, (list, tuple)) and len(d) > 1 else ""
            if d_cat == category and d_act == action:
                detail = msg or f"expected no decision {category} {action} but found one"
                raise AssertionError(detail)

    def assert_log_matches(self, pattern: str, msg: str = "") -> None:
        """Assert at least one log message matches *pattern* (glob)."""
        import fnmatch

        for entry in self.logs:
            log_msg = (
                entry[2] if isinstance(entry, (list, tuple)) and len(entry) > 2 else str(entry)
            )
            if fnmatch.fnmatch(log_msg, pattern):
                return
        detail = msg or f'no log message matching "{pattern}"'
        raise AssertionError(detail)

    def assert_no_log_matches(self, pattern: str, msg: str = "") -> None:
        """Assert no log message matches *pattern* (glob)."""
        import fnmatch

        for entry in self.logs:
            log_msg = (
                entry[2] if isinstance(entry, (list, tuple)) and len(entry) > 2 else str(entry)
            )
            if fnmatch.fnmatch(log_msg, pattern):
                detail = msg or f'found unexpected log message matching "{pattern}"'
                raise AssertionError(detail)

    def assert_event_fired(self, event_name: str, msg: str = "") -> None:
        """Assert a specific event was fired."""
        for er in self.events_fired:
            if er.event == event_name and er.fired:
                return
        detail = msg or f"event {event_name} was not fired"
        raise AssertionError(detail)

    def assert_event_not_fired(self, event_name: str, msg: str = "") -> None:
        """Assert a specific event was NOT fired (no handler or disabled)."""
        for er in self.events_fired:
            if er.event == event_name and er.fired:
                detail = msg or f"event {event_name} was fired but expected not"
                raise AssertionError(detail)

    def assert_connection_state(self, expected: str, msg: str = "") -> None:
        """Assert the connection state matches *expected*."""
        if self.connection_state != expected:
            detail = (
                msg or f'expected connection state "{expected}" but got "{self.connection_state}"'
            )
            raise AssertionError(detail)


# Backend detection


def _has_tkinter_tcl() -> bool:
    """Check if tkinter.Tcl() is available."""
    try:
        import tkinter

        interp = tkinter.Tcl()
        # Verify the interp actually works
        interp.eval("expr {1 + 1}")
        del interp
        return True
    except Exception:
        return False


def _find_tclsh() -> str | None:
    """Find a suitable tclsh binary, or return None."""
    for name in ("tclsh8.5", "tclsh8.6", "tclsh8.4", "tclsh"):
        path = shutil.which(name)
        if path:
            return path
    return None


# In-process Tcl backend


class _InProcessBackend:
    """Drives the Tcl framework via ``tkinter.Tcl()`` in-process."""

    def __init__(self) -> None:
        self._interp: Any = None  # tkinter.Tcl instance

    def start(self, tmos_version: str) -> None:
        import tkinter

        self._interp = tkinter.Tcl()

        # Source framework files (must happen before ::tmm::init locks down)
        for filename in _FRAMEWORK_FILES:
            path = _TCL_DIR / filename
            if not path.exists() and filename.startswith("_"):
                continue  # Optional generated files
            self._interp.eval(f"source {{{path}}}")

        # Initialise TMM environment (::orch::init calls tmm::init,
        # expr_ops::install, register_all, and install_unknown)
        self._interp.call("::orch::init", "-tmos_version", tmos_version)

    def stop(self) -> None:
        if self._interp is not None:
            try:
                self._interp.call("interp", "delete", "")
            except Exception:
                pass
            self._interp = None

    def send(self, msg: dict[str, Any]) -> dict[str, Any]:
        """Execute a command and return a result dict."""
        cmd = msg.get("cmd", "")
        try:
            result = self._dispatch(cmd, msg)
            return {"status": "ok", "result": result}
        except Exception as e:
            import traceback

            raise RuntimeError(f"Tcl error: {e}\n{traceback.format_exc()}") from e

    def _dispatch(self, cmd: str, msg: dict[str, Any]) -> Any:
        interp = self._interp
        assert interp is not None

        if cmd == "init":
            return "initialised"

        if cmd == "load_irule":
            interp.call("::itest::clear_irule")
            interp.call("::itest::load_irule", msg.get("source", ""))
            events_str = interp.eval("::itest::registered_events")
            if events_str:
                return list(interp.splitlist(events_str))
            return []

        if cmd == "set_state":
            self._apply_state(msg.get("layer", ""), msg.get("values", []))
            return "ok"

        if cmd == "fire_event":
            event = msg.get("event", "")
            result_str = str(interp.call("::itest::fire_event", event))
            return self._parse_flat_dict(result_str)

        if cmd == "get_state":
            return self._read_state(msg.get("layer", ""))

        if cmd == "get_decisions":
            category = msg.get("category", "")
            if category:
                result_str = str(interp.call("::itest::get_decisions", category))
            else:
                result_str = str(interp.call("::itest::get_decisions"))
            return self._parse_tcl_list(result_str)

        if cmd == "get_logs":
            return self._parse_tcl_list(str(interp.eval("::state::log_capture::get")))

        if cmd == "reset":
            scope = msg.get("scope", "connection")
            if scope == "connection":
                interp.eval("::state::reset_connection_state")
            elif scope == "request":
                interp.eval("::state::reset_request_state")
            elif scope == "all":
                interp.eval("::state::reset_all")
                interp.eval("::itest::reset_decisions")
            return "ok"

        if cmd == "add_pool":
            name = msg.get("name", "")
            members = msg.get("members", [])
            # Convert Python list to Tcl list via tuple
            interp.call("::state::lb::add_pool", name, tuple(members))
            return "ok"

        if cmd == "add_datagroup":
            name = msg.get("name", "")
            dg_type = msg.get("type", "string")
            records = msg.get("records", [])
            interp.call("::state::datagroup::add", name, dg_type, tuple(records))
            return "ok"

        if cmd == "eval":
            script = msg.get("script", "")
            return str(interp.eval(script))

        if cmd == "quit":
            return "ok"

        msg_text = f"unknown command: {cmd}"
        raise RuntimeError(msg_text)

    def _apply_state(self, layer: str, values: list[Any]) -> None:
        interp = self._interp
        assert interp is not None
        ns_map = {
            "connection": "::state::connection",
            "tls_client": "::state::tls::client",
            "tls_server": "::state::tls::server",
            "http_request": "::state::http::request",
            "http_response": "::state::http::response",
            "lb": "::state::lb",
        }
        ns = ns_map.get(layer)
        if ns is None:
            return
        # values is a flat list [key, val, key, val, ...]
        it = iter(values)
        for k in it:
            v = next(it)
            interp.eval(f"set {ns}::{k} {{{v}}}")

    def _read_state(self, layer: str) -> list[str]:
        interp = self._interp
        assert interp is not None
        var_map: dict[str, list[tuple[str, str]]] = {
            "connection": [
                ("client_addr", "::state::connection::client_addr"),
                ("client_port", "::state::connection::client_port"),
                ("local_addr", "::state::connection::local_addr"),
                ("local_port", "::state::connection::local_port"),
                ("server_addr", "::state::connection::server_addr"),
                ("server_port", "::state::connection::server_port"),
                ("state", "::state::connection::state"),
            ],
            "http_request": [
                ("method", "::state::http::request::method"),
                ("uri", "::state::http::request::uri"),
                ("path", "::state::http::request::path"),
                ("query", "::state::http::request::query"),
                ("host", "::state::http::request::host"),
                ("version", "::state::http::request::version"),
                ("headers", "::state::http::request::headers"),
            ],
            "http_response": [
                ("status", "::state::http::response::status"),
                ("reason", "::state::http::response::reason"),
                ("version", "::state::http::response::version"),
                ("headers", "::state::http::response::headers"),
            ],
            "lb": [
                ("pool", "::state::lb::pool"),
                ("pool_member", "::state::lb::pool_member"),
                ("node_addr", "::state::lb::node_addr"),
                ("node_port", "::state::lb::node_port"),
                ("snat_type", "::state::lb::snat_type"),
                ("selected", "::state::lb::selected"),
            ],
            "tls_client": [
                ("sni", "::state::tls::client::sni"),
                ("cipher_name", "::state::tls::client::cipher_name"),
                ("cipher_version", "::state::tls::client::cipher_version"),
                ("handshake_done", "::state::tls::client::handshake_done"),
            ],
        }
        if layer == "all":
            result: list[str] = []
            for sub_layer in ("connection", "http_request", "http_response", "lb", "tls_client"):
                result.extend([sub_layer, str(self._read_state(sub_layer))])
            return result

        pairs = var_map.get(layer)
        if pairs is None:
            msg = f"unknown state layer: {layer}"
            raise RuntimeError(msg)
        result = []
        for key, var in pairs:
            val = str(interp.eval(f"set {var}"))
            result.extend([key, val])
        return result

    def _parse_tcl_list(self, tcl_str: str) -> list[str]:
        """Parse a Tcl list string into a Python list of strings."""
        interp = self._interp
        assert interp is not None
        if not tcl_str or tcl_str.isspace():
            return []
        try:
            parts = interp.splitlist(tcl_str)
            return list(parts)
        except Exception:
            return [tcl_str]

    def _parse_flat_dict(self, tcl_str: str) -> list[str]:
        """Parse a Tcl dict-like string into a flat key-value list."""
        return self._parse_tcl_list(tcl_str)


# Subprocess Tcl backend


class _SubprocessBackend:
    """Drives the Tcl framework via ``tclsh`` subprocess + JSON protocol."""

    def __init__(self, tclsh: str) -> None:
        self._tclsh = tclsh
        self._process: asyncio.subprocess.Process | None = None

    async def start(self, tmos_version: str) -> None:
        runner = _TCL_DIR / "runner.tcl"
        self._process = await asyncio.create_subprocess_exec(
            self._tclsh,
            str(runner),
            stdin=asyncio.subprocess.PIPE,
            stdout=asyncio.subprocess.PIPE,
            stderr=asyncio.subprocess.PIPE,
        )
        ready_line = await self._process.stdout.readline()  # type: ignore[union-attr]
        ready = json.loads(ready_line)
        if ready.get("status") != "ready":
            msg = f"Unexpected ready signal: {ready}"
            raise RuntimeError(msg)
        await self.send({"cmd": "init", "tmos_version": tmos_version})

    async def stop(self) -> None:
        if self._process and self._process.returncode is None:
            try:
                await self.send({"cmd": "quit"})
            except (BrokenPipeError, ConnectionResetError):
                pass
            try:
                self._process.terminate()
                await asyncio.wait_for(self._process.wait(), timeout=5.0)
            except asyncio.TimeoutError:
                self._process.kill()

    async def send(self, msg: dict[str, Any]) -> dict[str, Any]:
        assert self._process is not None
        assert self._process.stdin is not None
        assert self._process.stdout is not None

        line = json.dumps(msg, separators=(",", ":")) + "\n"
        self._process.stdin.write(line.encode())
        await self._process.stdin.drain()

        resp_line = await self._process.stdout.readline()
        if not resp_line:
            stderr_data = await self._process.stderr.read()  # type: ignore[union-attr]
            msg_text = f"Tcl process closed unexpectedly: {stderr_data.decode()}"
            raise RuntimeError(msg_text)

        resp = json.loads(resp_line)
        if resp.get("status") == "error":
            error_msg = resp.get("message", "unknown error")
            error_info = resp.get("errorInfo", "")
            raise RuntimeError(f"Tcl error: {error_msg}\n{error_info}")

        return resp


# Main session class


class IruleTestSession:
    """Manages a Tcl interpreter running the iRule test framework.

    Backend selection (``backend`` parameter):

    - ``"auto"`` (default): try ``tkinter.Tcl()`` first, fall back to
      ``tclsh`` subprocess, raise if neither is available.
    - ``"inprocess"``: force ``tkinter.Tcl()`` (raises if unavailable).
    - ``"subprocess"``: force ``tclsh`` subprocess (raises if not on PATH).

    Usage::

        async with IruleTestSession(profiles=["TCP", "HTTP"]) as session:
            session.load_irule(source)
            result = await session.run_http_request(host="example.com")
    """

    def __init__(
        self,
        *,
        profiles: list[str] | None = None,
        tmos_version: str = "16.1",
        tclsh: str | None = None,
        backend: str = "auto",
        client_addr: str = "10.0.0.1",
        client_port: int = 54321,
        local_addr: str = "192.168.1.100",
        local_port: int = 443,
    ) -> None:
        self._profiles = profiles or ["TCP", "HTTP"]
        self._tmos_version = tmos_version
        self._client_addr = client_addr
        self._client_port = client_port
        self._local_addr = local_addr
        self._local_port = local_port
        self._inprocess: _InProcessBackend | None = None
        self._subprocess: _SubprocessBackend | None = None
        self._backend_type = self._resolve_backend(backend, tclsh)

    @staticmethod
    def _resolve_backend(backend: str, tclsh: str | None) -> str:
        if backend == "inprocess":
            if not _has_tkinter_tcl():
                msg = "tkinter.Tcl() is not available"
                raise RuntimeError(msg)
            return "inprocess"

        if backend == "subprocess":
            path = tclsh or _find_tclsh()
            if path is None:
                msg = "No tclsh found on PATH -- install Tcl 8.4+"
                raise FileNotFoundError(msg)
            return "subprocess"

        if backend == "auto":
            if _has_tkinter_tcl():
                return "inprocess"
            if tclsh or _find_tclsh():
                return "subprocess"
            msg = (
                "No Tcl interpreter available. Install Python with Tcl/Tk "
                "support or put tclsh on PATH."
            )
            raise RuntimeError(msg)

        msg = f"Unknown backend: {backend!r} (expected 'auto', 'inprocess', or 'subprocess')"
        raise ValueError(msg)

    @property
    def backend_type(self) -> str:
        """Return the active backend type: ``'inprocess'`` or ``'subprocess'``."""
        return self._backend_type

    async def __aenter__(self) -> IruleTestSession:
        await self.start()
        return self

    async def __aexit__(self, *exc: object) -> None:
        await self.stop()

    async def start(self) -> None:
        """Launch the Tcl interpreter and initialise the framework."""
        if self._backend_type == "inprocess":
            self._inprocess = _InProcessBackend()
            self._inprocess.start(self._tmos_version)
            log.debug("iRule test session started (in-process tkinter.Tcl)")
        else:
            tclsh = _find_tclsh()
            assert tclsh is not None
            self._subprocess = _SubprocessBackend(tclsh)
            await self._subprocess.start(self._tmos_version)
            log.debug("iRule test session started (subprocess %s)", tclsh)

        # Configure the Tcl orchestrator with profiles and connection settings
        profiles_tcl = " ".join(self._profiles)
        await self._send(
            {
                "cmd": "eval",
                "script": (
                    f"::orch::configure"
                    f" -profiles {{{profiles_tcl}}}"
                    f" -client_addr {self._client_addr}"
                    f" -client_port {self._client_port}"
                    f" -local_addr {self._local_addr}"
                    f" -local_port {self._local_port}"
                ),
            }
        )

    async def stop(self) -> None:
        """Shut down the Tcl interpreter."""
        if self._inprocess:
            self._inprocess.stop()
            self._inprocess = None
        if self._subprocess:
            await self._subprocess.stop()
            self._subprocess = None

    async def _send(self, msg: dict[str, Any]) -> dict[str, Any]:
        """Send a command to the active backend."""
        if self._inprocess:
            return self._inprocess.send(msg)
        if self._subprocess:
            return await self._subprocess.send(msg)
        msg_text = "Session not started"
        raise RuntimeError(msg_text)

    # High-level API

    def load_irule(self, source: str) -> None:
        """Load an iRule source synchronously (queues for async send)."""
        loop = asyncio.get_event_loop()
        if loop.is_running():
            raise RuntimeError("Use load_irule_async in async context")
        loop.run_until_complete(self.load_irule_async(source))

    async def load_irule_async(self, source: str) -> list[str]:
        """Load an iRule and return the list of registered events."""
        resp = await self._send({"cmd": "load_irule", "source": source})
        return resp.get("result", [])

    async def add_pool(self, name: str, members: list[str]) -> None:
        """Register a pool with its members."""
        await self._send({"cmd": "add_pool", "name": name, "members": members})

    async def add_datagroup(
        self,
        name: str,
        records: dict[str, str],
        dg_type: str = "string",
    ) -> None:
        """Register a data group."""
        flat_records: list[str] = []
        for k, v in records.items():
            flat_records.extend([k, v])
        await self._send(
            {
                "cmd": "add_datagroup",
                "name": name,
                "type": dg_type,
                "records": flat_records,
            }
        )

    async def set_state(self, layer: str, values: dict[str, Any]) -> None:
        """Set state on a specific layer."""
        flat: list[str] = []
        for k, v in values.items():
            flat.extend([k, str(v)])
        await self._send({"cmd": "set_state", "layer": layer, "values": flat})

    async def fire_event(self, event: str) -> EventResult:
        """Fire a single event and return the result."""
        resp = await self._send({"cmd": "fire_event", "event": event})
        result_data = resp.get("result", {})
        if isinstance(result_data, list):
            result_dict = dict(zip(result_data[::2], result_data[1::2]))
        else:
            result_dict = result_data
        return EventResult(
            event=event,
            fired=bool(result_dict.get("fired", 0)),
            reason=str(result_dict.get("reason", "")),
        )

    async def get_decisions(self, category: str = "") -> list[tuple[str, str, Any]]:
        """Get the decision log, optionally filtered by category."""
        resp = await self._send({"cmd": "get_decisions", "category": category})
        return resp.get("result", [])

    async def get_logs(self) -> list[tuple[str, str, str]]:
        """Get captured log messages."""
        resp = await self._send({"cmd": "get_logs"})
        return resp.get("result", [])

    async def get_state(self, layer: str) -> dict[str, Any]:
        """Read state from a specific layer."""
        resp = await self._send({"cmd": "get_state", "layer": layer})
        result = resp.get("result", [])
        if isinstance(result, list) and len(result) % 2 == 0:
            return dict(zip(result[::2], result[1::2]))
        return {}

    async def run_http_request(
        self,
        *,
        method: str = "GET",
        uri: str = "/",
        host: str = "",
        headers: dict[str, str] | None = None,
        sni: str = "",
        response_status: int = 200,
    ) -> RequestResult:
        """Run a complete HTTP request/response cycle.

        Delegates to the Tcl orchestrator (``::orch::run_http_request``),
        which handles connection lifecycle, event chain selection,
        per-request state isolation, and keep-alive tracking.

        On the first call, fires the full event chain (RULE_INIT through
        CLIENT_CLOSED or until short-circuit).  On subsequent calls
        without an intervening :meth:`close_connection`, fires only
        per-request events (keep-alive).
        """
        return await self._run_orch_request(
            "run_http_request",
            **{
                "method": method,
                "uri": uri,
                "host": host,
                "headers": headers,
                "sni": sni,
                "response_status": response_status,
            },
        )

    async def run_next_request(
        self,
        *,
        method: str = "GET",
        uri: str = "/",
        host: str = "",
        headers: dict[str, str] | None = None,
        sni: str = "",
        response_status: int = 200,
    ) -> RequestResult:
        """Run an additional HTTP request on an existing keep-alive connection.

        Resets per-request state (HTTP, LB) while preserving connection
        state (TCP, TLS, connection-scoped variables).  Raises if no
        active connection exists.
        """
        return await self._run_orch_request(
            "run_next_request",
            **{
                "method": method,
                "uri": uri,
                "host": host,
                "headers": headers,
                "sni": sni,
                "response_status": response_status,
            },
        )

    async def close_connection(self) -> None:
        """Close the current connection, firing CLIENT_CLOSED."""
        await self._send({"cmd": "eval", "script": "::orch::close_connection"})

    @staticmethod
    def _tcl_quote(value: str) -> str:
        """Brace-quote a value for safe Tcl eval.

        If the value contains unbalanced braces, fall back to
        backslash-escaping special characters.
        """
        # Fast path: if braces are balanced, brace-quoting is safe
        depth = 0
        for ch in value:
            if ch == "{":
                depth += 1
            elif ch == "}":
                depth -= 1
                if depth < 0:
                    break
        if depth == 0:
            return "{" + value + "}"
        # Fallback: backslash-escape Tcl special chars
        escaped = value.replace("\\", "\\\\")
        for ch in ('"', "$", "[", "]", "{", "}", ";"):
            escaped = escaped.replace(ch, f"\\{ch}")
        return escaped

    async def _run_orch_request(self, tcl_proc: str, **kwargs: Any) -> RequestResult:
        """Build ::orch Tcl call, execute, and gather results."""
        # Build the Tcl command args — all user values are brace-quoted
        tcl_args: list[str] = []
        method = kwargs.get("method", "GET")
        uri = kwargs.get("uri", "/")
        host = kwargs.get("host", "")
        sni = kwargs.get("sni", "")
        headers = kwargs.get("headers")
        response_status = kwargs.get("response_status", 200)

        if method != "GET":
            tcl_args.extend(["-method", self._tcl_quote(method)])
        if uri != "/":
            tcl_args.extend(["-uri", self._tcl_quote(uri)])
        if host:
            tcl_args.extend(["-host", self._tcl_quote(host)])
        if sni:
            tcl_args.extend(["-sni", self._tcl_quote(sni)])
        if response_status != 200:
            tcl_args.extend(["-response_status", str(int(response_status))])
        if headers:
            # Build Tcl list of header key-value pairs
            hdr_parts = []
            for hname, hval in headers.items():
                hdr_parts.extend([self._tcl_quote(hname), self._tcl_quote(hval)])
            hdr_list = " ".join(hdr_parts)
            tcl_args.extend(["-headers", "{" + hdr_list + "}"])

        args_str = " ".join(tcl_args)
        script = f"::orch::{tcl_proc} {args_str}"
        await self._send({"cmd": "eval", "script": script})

        # Gather results from Tcl state
        lb_state = await self.get_state("lb")
        conn_state = await self.get_state("connection")
        decisions = await self.get_decisions()
        logs = await self.get_logs()
        committed = await self._send(
            {"cmd": "eval", "script": "set ::state::http::response_committed"}
        )

        return RequestResult(
            pool_selected=lb_state.get("pool", ""),
            node_selected=lb_state.get("node_addr", ""),
            decisions=decisions,
            logs=logs,
            connection_state=conn_state.get("state", ""),
            http_response_committed=committed.get("result", "0") == "1",
        )

    async def reset(self, scope: str = "all") -> None:
        """Reset framework state."""
        await self._send({"cmd": "reset", "scope": scope})

    # fakeCMP tools

    async def fakecmp_which_tmm(
        self,
        src_addr: str | None = None,
        src_port: int | None = None,
        dst_addr: str | None = None,
        dst_port: int | None = None,
    ) -> int:
        """Look up which TMM a connection tuple maps to via fakeCMP hash.

        With no arguments, uses the current config (client_addr/port,
        local_addr/port).  With all four arguments, hashes that tuple.
        """
        if src_addr is not None:
            script = f"::orch::fakecmp_which_tmm {src_addr} {src_port} {dst_addr} {dst_port}"
        else:
            script = "::orch::fakecmp_which_tmm"
        resp = await self._send({"cmd": "eval", "script": script})
        return int(resp.get("result", "0"))

    async def fakecmp_suggest_sources(
        self,
        count: int = 1,
        dst_addr: str | None = None,
        dst_port: int | None = None,
    ) -> dict[int, list[tuple[str, int]]]:
        """Suggest client_addr/port combos that land on each TMM.

        Returns ``{tmm_id: [(addr, port), ...], ...}``.
        """
        args = f"-count {count}"
        if dst_addr is not None:
            args += f" -dst_addr {dst_addr}"
        if dst_port is not None:
            args += f" -dst_port {dst_port}"
        script = f"::orch::fakecmp_suggest_sources {args}"
        resp = await self._send({"cmd": "eval", "script": script})
        raw = resp.get("result", "")
        return self._parse_suggest_sources(raw)

    async def fakecmp_plan(self, count: int = 1) -> str:
        """Pretty-print fakeCMP distribution plan."""
        script = f"::orch::fakecmp_plan -count {count}"
        resp = await self._send({"cmd": "eval", "script": script})
        return resp.get("result", "")

    @staticmethod
    def _parse_suggest_sources(raw: str) -> dict[int, list[tuple[str, int]]]:
        """Parse Tcl dict from fakecmp_suggest_sources into Python dict."""
        result: dict[int, list[tuple[str, int]]] = {}
        # raw is a Tcl dict: "0 {10.0.0.1 10001} 1 {10.0.0.2 10001} ..."
        # Simple parser: split on top-level whitespace respecting braces
        import re

        # Match key {value} or key value pairs
        pairs = re.findall(r"(\d+)\s+\{([^}]*)\}", raw)
        if not pairs:
            # Fallback: try simple space-delimited pairs
            tokens = raw.split()
            i = 0
            while i < len(tokens) - 1:
                try:
                    tmm_id = int(tokens[i])
                    result[tmm_id] = []
                    i += 1
                except ValueError:
                    i += 1
            return result

        for tmm_str, sources_str in pairs:
            tmm_id = int(tmm_str)
            sources_str = sources_str.strip()
            if not sources_str:
                result[tmm_id] = []
                continue
            parts = sources_str.split()
            tuples: list[tuple[str, int]] = []
            for j in range(0, len(parts) - 1, 2):
                tuples.append((parts[j], int(parts[j + 1])))
            result[tmm_id] = tuples
        return result


class IruleTestSessionSync:
    """Synchronous wrapper around :class:`IruleTestSession`.

    For use in pytest and other synchronous test runners::

        with IruleTestSessionSync(profiles=["TCP", "HTTP"]) as session:
            session.load_irule(source)
            result = session.run_http_request(host="example.com")
            assert result.pool_selected == "my_pool"
    """

    def __init__(self, **kwargs: Any) -> None:
        self._kwargs = kwargs
        self._session: IruleTestSession | None = None
        self._loop: asyncio.AbstractEventLoop | None = None

    def __enter__(self) -> IruleTestSessionSync:
        self._loop = asyncio.new_event_loop()
        self._session = IruleTestSession(**self._kwargs)
        self._loop.run_until_complete(self._session.start())
        return self

    def __exit__(self, *exc: object) -> None:
        if self._session and self._loop:
            self._loop.run_until_complete(self._session.stop())
        if self._loop:
            self._loop.close()

    @property
    def backend_type(self) -> str:
        """Return the active backend type."""
        assert self._session is not None
        return self._session.backend_type

    def load_irule(self, source: str) -> list[str]:
        assert self._session and self._loop
        return self._loop.run_until_complete(self._session.load_irule_async(source))

    def add_pool(self, name: str, members: list[str]) -> None:
        assert self._session and self._loop
        self._loop.run_until_complete(self._session.add_pool(name, members))

    def add_datagroup(
        self,
        name: str,
        records: dict[str, str],
        dg_type: str = "string",
    ) -> None:
        assert self._session and self._loop
        self._loop.run_until_complete(self._session.add_datagroup(name, records, dg_type))

    def fire_event(self, event: str) -> EventResult:
        assert self._session and self._loop
        return self._loop.run_until_complete(self._session.fire_event(event))

    def run_http_request(self, **kwargs: Any) -> RequestResult:
        assert self._session and self._loop
        return self._loop.run_until_complete(self._session.run_http_request(**kwargs))

    def run_next_request(self, **kwargs: Any) -> RequestResult:
        assert self._session and self._loop
        return self._loop.run_until_complete(self._session.run_next_request(**kwargs))

    def close_connection(self) -> None:
        assert self._session and self._loop
        self._loop.run_until_complete(self._session.close_connection())

    def get_decisions(self, category: str = "") -> list[tuple[str, str, Any]]:
        assert self._session and self._loop
        return self._loop.run_until_complete(self._session.get_decisions(category))

    def get_logs(self) -> list[tuple[str, str, str]]:
        assert self._session and self._loop
        return self._loop.run_until_complete(self._session.get_logs())

    def get_state(self, layer: str) -> dict[str, Any]:
        assert self._session and self._loop
        return self._loop.run_until_complete(self._session.get_state(layer))

    def reset(self, scope: str = "all") -> None:
        assert self._session and self._loop
        self._loop.run_until_complete(self._session.reset(scope))

    def fakecmp_which_tmm(
        self,
        src_addr: str | None = None,
        src_port: int | None = None,
        dst_addr: str | None = None,
        dst_port: int | None = None,
    ) -> int:
        assert self._session and self._loop
        return self._loop.run_until_complete(
            self._session.fakecmp_which_tmm(src_addr, src_port, dst_addr, dst_port)
        )

    def fakecmp_suggest_sources(
        self,
        count: int = 1,
        dst_addr: str | None = None,
        dst_port: int | None = None,
    ) -> dict[int, list[tuple[str, int]]]:
        assert self._session and self._loop
        return self._loop.run_until_complete(
            self._session.fakecmp_suggest_sources(count, dst_addr, dst_port)
        )

    def fakecmp_plan(self, count: int = 1) -> str:
        assert self._session and self._loop
        return self._loop.run_until_complete(self._session.fakecmp_plan(count))
