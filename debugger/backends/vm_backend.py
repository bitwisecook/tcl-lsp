"""VM debug backend — uses the project's own bytecode VM."""

from __future__ import annotations

import io
import threading
from pathlib import Path

from debugger.backends.base import DebugBackend
from debugger.controller import DebugController
from debugger.types import StackFrame, Variable


class VmBackend(DebugBackend):
    """Debug backend using the project's bytecode VM.

    Provides full stepping and variable inspection by hooking directly
    into the ``BytecodeVM`` execution loop via ``DebugController``.
    """

    def __init__(self) -> None:
        super().__init__()
        self._controller = DebugController()
        self._vm_thread: threading.Thread | None = None
        self._interp: object | None = None  # TclInterp, set during launch

    @property
    def controller(self) -> DebugController:
        return self._controller

    # -- Lifecycle ------------------------------------------------------------

    def launch(self, script_path: str, *, source: str | None = None) -> None:
        if source is None:
            source = Path(script_path).read_text(encoding="utf-8")
        self._source = source
        self._script_path = script_path

    def _run_in_thread(self) -> None:
        """Execute the script in a background thread."""
        from vm.interp import TclInterp

        interp = TclInterp(debug_hook=self._controller.debug_hook)
        self._interp = interp

        # Wrap puts to notify frontend

        class _OutputCapture(io.TextIOBase):
            def __init__(self, backend: VmBackend) -> None:
                self._backend = backend
                self._buf = io.StringIO()

            def write(self, s: str) -> int:
                self._buf.write(s)
                if self._backend._on_output and s:
                    self._backend._on_output(s)
                return len(s)

            def getvalue(self) -> str:
                return self._buf.getvalue()

        capture = _OutputCapture(self)
        interp.channels["stdout"] = capture  # type: ignore[assignment]

        try:
            interp.script_file = self._script_path
            interp.eval(self._source)
        except Exception as exc:
            if self._on_output:
                self._on_output(f"Error: {exc}\n")
        finally:
            if self._on_finished:
                self._on_finished()

    def _start_execution(self) -> None:
        """Start the VM thread if not already running."""
        if self._vm_thread is None or not self._vm_thread.is_alive():
            self._vm_thread = threading.Thread(target=self._run_in_thread, daemon=True)
            self._vm_thread.start()

    # -- Breakpoints ----------------------------------------------------------

    def set_breakpoints(self, lines: set[int]) -> list[int]:
        return self._controller.set_breakpoints(lines)

    # -- Execution control ----------------------------------------------------

    def continue_execution(self) -> None:
        from debugger.types import StepMode

        if self._vm_thread is None or not self._vm_thread.is_alive():
            self._start_execution()
        else:
            self._controller.resume(StepMode.CONTINUE)

    def step_in(self) -> None:
        from debugger.types import StepMode

        if self._vm_thread is None or not self._vm_thread.is_alive():
            self._start_execution()
        else:
            self._controller.resume(StepMode.STEP_IN)

    def step_over(self) -> None:
        from debugger.types import StepMode

        if self._vm_thread is None or not self._vm_thread.is_alive():
            self._start_execution()
        else:
            self._controller.resume(StepMode.STEP_OVER)

    def step_out(self) -> None:
        from debugger.types import StepMode

        if self._vm_thread is None or not self._vm_thread.is_alive():
            self._start_execution()
        else:
            self._controller.resume(StepMode.STEP_OUT)

    # -- State inspection -----------------------------------------------------

    def get_stack_trace(self) -> list[StackFrame]:
        return self._controller.get_stack_trace()

    def get_variables(self, frame_id: int = 0) -> list[Variable]:
        return self._controller.get_variables(frame_id)

    def evaluate(self, expression: str) -> str:
        if self._interp is None:
            return "<no interpreter>"
        try:
            from vm.interp import TclInterp

            interp: TclInterp = self._interp  # type: ignore[assignment]
            result = interp.eval(expression)
            return result.value
        except Exception as exc:
            return f"Error: {exc}"

    # -- Cleanup --------------------------------------------------------------

    def terminate(self) -> None:
        self._controller.request_terminate()
        if self._vm_thread is not None:
            self._vm_thread.join(timeout=5.0)
