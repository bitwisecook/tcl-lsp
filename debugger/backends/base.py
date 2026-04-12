"""Abstract base class for debugger backends."""

from __future__ import annotations

from abc import ABC, abstractmethod
from collections.abc import Callable

from debugger.types import StackFrame, StopEvent, Variable


class DebugBackend(ABC):
    """Common interface for all debug backends (VM, tclsh, tkinter)."""

    def __init__(self) -> None:
        self._on_stop: Callable[[StopEvent], None] | None = None
        self._on_output: Callable[[str], None] | None = None
        self._on_finished: Callable[[], None] | None = None

    # -- Event registration ---------------------------------------------------

    def on_stop(self, callback: Callable[[StopEvent], None]) -> None:
        """Register a callback invoked when execution stops."""
        self._on_stop = callback

    def on_output(self, callback: Callable[[str], None]) -> None:
        """Register a callback invoked when the script produces output."""
        self._on_output = callback

    def on_finished(self, callback: Callable[[], None]) -> None:
        """Register a callback invoked when the script finishes."""
        self._on_finished = callback

    # -- Lifecycle ------------------------------------------------------------

    @abstractmethod
    def launch(self, script_path: str, *, source: str | None = None) -> None:
        """Load and prepare a script for debugging.

        If *source* is provided it is used instead of reading *script_path*.
        """

    @abstractmethod
    def set_breakpoints(self, lines: set[int]) -> list[int]:
        """Set breakpoints; return lines that were actually accepted."""

    # -- Execution control ----------------------------------------------------

    @abstractmethod
    def continue_execution(self) -> None:
        """Resume execution until the next breakpoint or end."""

    @abstractmethod
    def step_in(self) -> None:
        """Execute one statement, stepping into calls."""

    @abstractmethod
    def step_over(self) -> None:
        """Execute one statement, stepping over calls."""

    @abstractmethod
    def step_out(self) -> None:
        """Run until the current procedure returns."""

    # -- State inspection -----------------------------------------------------

    @abstractmethod
    def get_stack_trace(self) -> list[StackFrame]:
        """Return the current call stack."""

    @abstractmethod
    def get_variables(self, frame_id: int) -> list[Variable]:
        """Return variables visible in the given scope."""

    @abstractmethod
    def evaluate(self, expression: str) -> str:
        """Evaluate a Tcl expression in the current context."""

    # -- Cleanup --------------------------------------------------------------

    @abstractmethod
    def terminate(self) -> None:
        """Stop execution and clean up resources."""


__all__ = ["DebugBackend"]
