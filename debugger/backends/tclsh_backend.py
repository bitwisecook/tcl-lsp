"""tclsh debug backend — uses an external tclsh subprocess."""

from __future__ import annotations

import json
import subprocess
import threading
from pathlib import Path
from typing import Any

from debugger.backends.base import DebugBackend
from debugger.types import StackFrame, Variable

_HELPER_SCRIPT = Path(__file__).resolve().parent.parent / "tcl" / "debug_helper.tcl"


class TclshBackend(DebugBackend):
    """Debug backend using an external ``tclsh`` subprocess.

    Communicates with the Tcl debug helper script via JSON over
    stdin/stdout, mirroring the pattern used by
    ``core/irule_test/bridge.py``'s ``_SubprocessBackend``.
    """

    def __init__(self, tclsh_path: str) -> None:
        super().__init__()
        self._tclsh = tclsh_path
        self._process: subprocess.Popen[str] | None = None
        self._script_path: str = ""
        self._source: str | None = None
        self._breakpoints: set[int] = set()
        self._thread: threading.Thread | None = None
        self._current_line: int = 0
        self._current_cmd: str = ""
        self._stopped = threading.Event()
        self._lock = threading.Lock()
        self._terminated = False

    # -- Lifecycle ------------------------------------------------------------

    def launch(self, script_path: str, *, source: str | None = None) -> None:
        self._script_path = script_path

        # If source is provided, write it to a temp file
        if source is not None:
            self._source = source
            import tempfile

            tmp = tempfile.NamedTemporaryFile(
                mode="w", suffix=".tcl", delete=False, encoding="utf-8"
            )
            tmp.write(source)
            tmp.flush()
            tmp.close()
            self._script_path = tmp.name

    def _start(self) -> None:
        """Launch the tclsh subprocess with the debug helper."""
        self._process = subprocess.Popen(
            [self._tclsh, str(_HELPER_SCRIPT), self._script_path],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            bufsize=1,
        )

        # Wait for ready signal
        line = self._process.stdout.readline()  # type: ignore[union-attr]
        if not line:
            msg = "tclsh debug helper failed to start"
            raise RuntimeError(msg)

        try:
            msg = json.loads(line.strip())
            if msg.get("type") != "ready":
                raise RuntimeError(f"Unexpected initial message: {msg}")
        except json.JSONDecodeError:
            raise RuntimeError(f"Invalid JSON from tclsh: {line!r}")

    def _send_command(self, cmd: dict[str, Any]) -> None:
        """Send a JSON command to the tclsh subprocess."""
        if self._process is None or self._process.stdin is None:
            return
        line = json.dumps(cmd, separators=(",", ":"))
        self._process.stdin.write(line + "\n")
        self._process.stdin.flush()

    def _read_response(self) -> dict[str, Any] | None:
        """Read a JSON response from the tclsh subprocess."""
        if self._process is None or self._process.stdout is None:
            return None
        line = self._process.stdout.readline()
        if not line:
            return None
        try:
            return json.loads(line.strip())
        except json.JSONDecodeError:
            # Non-JSON output — treat as program output
            if self._on_output:
                self._on_output(line)
            return None

    def _run_loop(self) -> None:
        """Read events from tclsh in a background thread."""
        try:
            # Send start command with breakpoints
            bp_list = sorted(self._breakpoints)
            self._send_command({"cmd": "start", "lines": bp_list})

            while not self._terminated:
                msg = self._read_response()
                if msg is None:
                    break

                msg_type = msg.get("type")
                if msg_type == "stopped":
                    self._current_line = int(msg.get("line", 0))
                    self._current_cmd = msg.get("cmd", "")
                    self._stopped.set()
                    # Block until frontend tells us to resume
                    # (the frontend calls continue/step which sends a command)
                elif msg_type == "finished":
                    break
                elif msg_type == "error":
                    if self._on_output:
                        self._on_output(f"Error: {msg.get('message', '')}\n")
                    break
                elif msg_type == "output":
                    if self._on_output:
                        self._on_output(msg.get("text", ""))
        finally:
            if self._on_finished:
                self._on_finished()

    # -- Breakpoints ----------------------------------------------------------

    def set_breakpoints(self, lines: set[int]) -> list[int]:
        self._breakpoints = lines
        if self._process is not None:
            self._send_command(
                {
                    "cmd": "set_breakpoints",
                    "lines": sorted(lines),
                }
            )
        return sorted(lines)

    # -- Execution control ----------------------------------------------------

    def continue_execution(self) -> None:
        if self._process is None:
            self._start()
            self._thread = threading.Thread(target=self._run_loop, daemon=True)
            self._thread.start()
        else:
            self._stopped.clear()
            self._send_command({"cmd": "continue"})

    def step_in(self) -> None:
        if self._process is None:
            self._start()
            self._thread = threading.Thread(target=self._run_loop, daemon=True)
            self._thread.start()
        else:
            self._stopped.clear()
            self._send_command({"cmd": "step_in"})

    def step_over(self) -> None:
        if self._process is None:
            self.step_in()
            return
        self._stopped.clear()
        self._send_command({"cmd": "step_over"})

    def step_out(self) -> None:
        if self._process is None:
            self.step_in()
            return
        self._stopped.clear()
        self._send_command({"cmd": "step_out"})

    # -- State inspection -----------------------------------------------------

    def get_stack_trace(self) -> list[StackFrame]:
        return [
            StackFrame(
                id=0,
                name="global",
                line=self._current_line,
                namespace="::",
            )
        ]

    def get_variables(self, frame_id: int = 0) -> list[Variable]:
        # For tclsh, variable inspection requires a round-trip
        # to the subprocess — simplified for now
        return []

    def evaluate(self, expression: str) -> str:
        return "<not yet supported for tclsh backend>"

    # -- Cleanup --------------------------------------------------------------

    def terminate(self) -> None:
        self._terminated = True
        if self._process is not None:
            try:
                self._send_command({"cmd": "terminate"})
            except Exception:
                pass
            try:
                self._process.terminate()
                self._process.wait(timeout=5)
            except Exception:
                self._process.kill()
        self._stopped.set()  # unblock any waiters

    def wait_for_stop(self, timeout: float | None = None) -> bool:
        """Block until tclsh stops at a breakpoint/step."""
        result = self._stopped.wait(timeout=timeout)
        if result:
            self._stopped.clear()
        return result
