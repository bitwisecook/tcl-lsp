"""Debugger controller — breakpoints, stepping, and state inspection."""

from __future__ import annotations

import threading
from typing import TYPE_CHECKING, Any

from .types import DebugAction, StackFrame, StepMode, StopEvent, Variable

if TYPE_CHECKING:
    from core.compiler.codegen import Instruction
    from vm.scope import CallFrame


class DebugController:
    """Coordinates breakpoints, stepping, and state inspection.

    The controller provides a ``debug_hook`` method that the VM calls at
    each source line boundary.  When the debugger should stop (breakpoint
    hit, step completed) the hook blocks the VM thread and notifies the
    frontend via ``on_stop``.
    """

    def __init__(self) -> None:
        self._breakpoints: dict[int, bool] = {}  # line -> enabled
        self._step_mode: StepMode = StepMode.STEP_IN  # stop at first line
        self._step_depth: int = 0
        self._stopped = threading.Event()
        self._resume = threading.Event()
        self._next_action: StepMode = StepMode.CONTINUE
        self._current_frame: CallFrame | None = None
        self._current_line: int = 0
        self._current_cmd: str = ""
        self._stack: list[str] = []
        self._terminated = False

    # -- Breakpoint management ------------------------------------------------

    def add_breakpoint(self, line: int) -> int:
        """Add a breakpoint at *line*.  Returns the line number."""
        self._breakpoints[line] = True
        return line

    def remove_breakpoint(self, line: int) -> None:
        """Remove the breakpoint at *line*."""
        self._breakpoints.pop(line, None)

    def get_breakpoints(self) -> list[int]:
        """Return all active breakpoint lines."""
        return sorted(line for line, enabled in self._breakpoints.items() if enabled)

    def set_breakpoints(self, lines: set[int]) -> list[int]:
        """Replace all breakpoints with the given set of *lines*."""
        self._breakpoints = {line: True for line in lines}
        return sorted(lines)

    # -- VM debug hook --------------------------------------------------------

    def debug_hook(
        self,
        instr: Instruction,
        pc: int,
        stack: list[str],
        frame: CallFrame,
    ) -> DebugAction:
        """Called by the VM at each source line boundary."""
        line = instr.source_line
        should_stop = False

        # Check breakpoints
        if self._breakpoints.get(line, False):
            should_stop = True

        # Check step mode
        match self._step_mode:
            case StepMode.STEP_IN:
                should_stop = True
            case StepMode.STEP_OVER:
                if frame.level <= self._step_depth:
                    should_stop = True
            case StepMode.STEP_OUT:
                if frame.level < self._step_depth:
                    should_stop = True
            case StepMode.CONTINUE:
                pass  # only stop on breakpoints

        if not should_stop:
            return DebugAction.CONTINUE

        # We are stopping — capture state and notify frontend
        self._current_frame = frame
        self._current_line = line
        self._current_cmd = instr.source_cmd_text
        self._stack = list(stack)
        self._step_depth = frame.level

        # Signal that we have stopped
        self._stopped.set()

        # Wait for the frontend to tell us what to do
        self._resume.wait()
        self._resume.clear()

        if self._terminated:
            return DebugAction.STOP

        # Apply the next action
        self._step_mode = self._next_action
        self._step_depth = frame.level
        return DebugAction.CONTINUE

    # -- Frontend control (called from the CLI/DAP thread) --------------------

    def wait_for_stop(self, timeout: float | None = None) -> bool:
        """Block until the VM stops.  Returns ``True`` if stopped."""
        result = self._stopped.wait(timeout=timeout)
        if result:
            self._stopped.clear()
        return result

    def resume(self, mode: StepMode) -> None:
        """Tell the VM to resume with the given stepping *mode*."""
        self._next_action = mode
        self._resume.set()

    def request_terminate(self) -> None:
        """Ask the VM to stop execution."""
        self._terminated = True
        self._resume.set()

    # -- State inspection (safe to call while stopped) ------------------------

    @property
    def current_line(self) -> int:
        return self._current_line

    @property
    def current_command(self) -> str:
        return self._current_cmd

    def get_stop_event(self) -> StopEvent:
        """Build a ``StopEvent`` from the current stopped state."""
        reason = "step"
        if self._breakpoints.get(self._current_line, False):
            reason = "breakpoint"
        return StopEvent(
            line=self._current_line,
            command_text=self._current_cmd,
            reason=reason,
            frames=self.get_stack_trace(),
        )

    def get_stack_trace(self) -> list[StackFrame]:
        """Walk the call frame chain and build a stack trace."""
        frames: list[StackFrame] = []
        frame = self._current_frame
        frame_id = 0
        while frame is not None:
            ns = "::"
            if frame.namespace is not None:
                ns = frame.namespace.qualname
            frames.append(
                StackFrame(
                    id=frame_id,
                    name=frame.proc_name or "global",
                    line=self._current_line if frame_id == 0 else 0,
                    namespace=ns,
                )
            )
            frame = frame.parent
            frame_id += 1
        return frames

    def get_variables(self, frame_id: int = 0) -> list[Variable]:
        """Return variables visible in the frame at *frame_id*.

        Frame 0 is the current (innermost) frame.
        """
        frame = self._get_frame(frame_id)
        if frame is None:
            return []

        variables: list[Variable] = []

        # Scalars
        for name, value in sorted(frame._scalars.items()):
            variables.append(Variable(name=name, value=value, type="scalar"))

        # Arrays
        for name, elements in sorted(frame._arrays.items()):
            children = [
                Variable(name=f"{name}({k})", value=v, type="scalar")
                for k, v in sorted(elements.items())
            ]
            variables.append(
                Variable(
                    name=name,
                    value=f"array ({len(elements)} elements)",
                    type="array",
                    children=children,
                )
            )

        # Aliases (upvar)
        for local_name, (target_frame, target_name) in sorted(frame._aliases.items()):
            # Resolve the alias target value
            try:
                value = target_frame.get_var(target_name)
            except Exception:
                value = "<unresolved>"
            target_desc = target_name
            if target_frame.proc_name:
                target_desc = f"{target_frame.proc_name}::{target_name}"
            variables.append(
                Variable(
                    name=local_name,
                    value=value,
                    type="alias",
                    alias_target=target_desc,
                )
            )

        return variables

    def _get_frame(self, frame_id: int) -> Any:
        """Walk the frame chain to find frame at *frame_id* depth."""
        frame = self._current_frame
        for _ in range(frame_id):
            if frame is None:
                return None
            frame = frame.parent
        return frame
