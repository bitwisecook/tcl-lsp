"""tkinter debug backend — uses Python's built-in tkinter.Tcl() interpreter."""

from __future__ import annotations

import threading
from pathlib import Path
from typing import Any

from debugger.backends.base import DebugBackend
from debugger.types import StackFrame, Variable

_HELPER_SCRIPT = Path(__file__).resolve().parent.parent / "tcl" / "debug_helper.tcl"


class TkinterBackend(DebugBackend):
    """Debug backend using ``tkinter.Tcl()`` for in-process Tcl execution.

    Bridges Tcl trace callbacks directly to Python via
    ``interp.createcommand()``, avoiding subprocess overhead.
    """

    def __init__(self) -> None:
        super().__init__()
        self._interp: Any = None  # tkinter.Tcl instance
        self._script_path: str = ""
        self._source: str | None = None
        self._breakpoints: set[int] = set()
        self._thread: threading.Thread | None = None
        self._current_line: int = 0
        self._current_cmd: str = ""
        self._stopped = threading.Event()
        self._resume = threading.Event()
        self._step_mode: str = "step_in"
        self._terminated = False
        self._prev_line: int = -1
        self._variables: list[Variable] = []

    # -- Lifecycle ------------------------------------------------------------

    def launch(self, script_path: str, *, source: str | None = None) -> None:
        self._script_path = script_path
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

    def _create_interp(self) -> None:
        """Create the tkinter.Tcl() interpreter and register callbacks."""
        import tkinter

        self._interp = tkinter.Tcl()

        # Register Python callback for the debug step hook
        self._interp.createcommand("__dbg_python_step", self._step_callback)

        # Set up the execution trace using a Tcl wrapper that calls our Python callback
        self._interp.eval("""
            proc ::__dbg_enterstep {args} {
                set depth [info frame]
                set line 0
                set cmd_text ""
                for {set i $depth} {$i >= 1} {incr i -1} {
                    set finfo [info frame $i]
                    if {[dict exists $finfo line]} {
                        set line [dict get $finfo line]
                        if {[dict exists $finfo cmd]} {
                            set cmd_text [dict get $finfo cmd]
                        }
                        break
                    }
                }
                if {$line > 0} {
                    __dbg_python_step $line $cmd_text
                }
            }
            trace add execution source enterstep ::__dbg_enterstep
        """)

    def _step_callback(self, line_str: str, cmd_text: str = "") -> str:
        """Called from Tcl on each traced step.

        This runs on the Tcl execution thread.  When we need to stop,
        we signal the frontend and block until resumed.
        """
        if self._terminated:
            return ""

        try:
            line = int(line_str)
        except (ValueError, TypeError):
            return ""

        if line <= 0:
            return ""

        # Dedup same-line hits when in continue mode
        if line == self._prev_line and self._step_mode == "continue":
            return ""

        # Check whether we should stop
        should_stop = False
        if self._step_mode == "step_in":
            should_stop = True
        elif line in self._breakpoints:
            should_stop = True
        elif self._step_mode in ("step_over", "step_out"):
            should_stop = True

        if not should_stop:
            return ""

        self._prev_line = line
        self._current_line = line
        self._current_cmd = cmd_text if len(cmd_text) <= 200 else cmd_text[:197] + "..."

        # Collect variables from the Tcl interp
        self._collect_variables()

        # Signal the frontend that we have stopped
        self._stopped.set()

        # Block until the frontend tells us to resume
        self._resume.wait()
        self._resume.clear()

        return ""

    def _collect_variables(self) -> None:
        """Snapshot variables from the current Tcl scope."""
        self._variables = []
        if self._interp is None:
            return
        try:
            var_names = self._interp.eval("info vars").split()
            for name in sorted(var_names):
                # Skip debugger internals
                if name.startswith("__dbg") or name.startswith("::__dbg"):
                    continue
                try:
                    is_array = self._interp.eval(f"array exists {name}")
                    if is_array == "1":
                        self._variables.append(Variable(name=name, value="(array)", type="array"))
                    else:
                        val = self._interp.eval(f"set {name}")
                        self._variables.append(Variable(name=name, value=val, type="scalar"))
                except Exception:
                    pass
        except Exception:
            pass

    def _run_in_thread(self) -> None:
        """Run the Tcl script in a background thread."""
        try:
            self._create_interp()
            self._interp.eval(f"source {{{self._script_path}}}")
        except Exception as exc:
            error_msg = str(exc)
            # Filter out debugger internals
            if "__dbg" not in error_msg and self._on_output:
                self._on_output(f"Error: {error_msg}\n")
        finally:
            if self._on_finished:
                self._on_finished()

    # -- Breakpoints ----------------------------------------------------------

    def set_breakpoints(self, lines: set[int]) -> list[int]:
        self._breakpoints = lines
        return sorted(lines)

    # -- Execution control ----------------------------------------------------

    def continue_execution(self) -> None:
        if self._thread is None or not self._thread.is_alive():
            self._step_mode = "step_in"  # stop at first line
            self._thread = threading.Thread(target=self._run_in_thread, daemon=True)
            self._thread.start()
        else:
            self._step_mode = "continue"
            self._resume.set()

    def step_in(self) -> None:
        if self._thread is None or not self._thread.is_alive():
            self._step_mode = "step_in"
            self._thread = threading.Thread(target=self._run_in_thread, daemon=True)
            self._thread.start()
        else:
            self._step_mode = "step_in"
            self._resume.set()

    def step_over(self) -> None:
        if self._thread is None or not self._thread.is_alive():
            self.step_in()
            return
        self._step_mode = "step_over"
        self._resume.set()

    def step_out(self) -> None:
        if self._thread is None or not self._thread.is_alive():
            self.step_in()
            return
        self._step_mode = "step_out"
        self._resume.set()

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
        return list(self._variables)

    def evaluate(self, expression: str) -> str:
        if self._interp is None:
            return "<no interpreter>"
        try:
            return self._interp.eval(expression)
        except Exception as exc:
            return f"Error: {exc}"

    # -- Cleanup --------------------------------------------------------------

    def terminate(self) -> None:
        self._terminated = True
        self._resume.set()  # unblock the step callback
        self._stopped.set()  # unblock any waiters
        if self._thread is not None:
            self._thread.join(timeout=5.0)

    def wait_for_stop(self, timeout: float | None = None) -> bool:
        """Block until tkinter stops at a breakpoint/step."""
        result = self._stopped.wait(timeout=timeout)
        if result:
            self._stopped.clear()
        return result
