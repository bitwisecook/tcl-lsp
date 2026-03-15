"""Data types shared across the debugger package."""

from __future__ import annotations

from collections.abc import Callable
from dataclasses import dataclass, field
from enum import Enum, auto
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from core.compiler.codegen import Instruction
    from vm.scope import CallFrame


class DebugAction(Enum):
    """Action returned by the debug hook to the VM."""

    CONTINUE = auto()
    STOP = auto()


class StepMode(Enum):
    """Current stepping mode for the debugger controller."""

    CONTINUE = auto()
    STEP_IN = auto()
    STEP_OVER = auto()
    STEP_OUT = auto()


@dataclass(slots=True)
class StackFrame:
    """A single frame in the debug call stack."""

    id: int
    name: str  # proc name or "global"
    line: int
    namespace: str


@dataclass(slots=True)
class Variable:
    """A variable visible in the debugger."""

    name: str
    value: str
    type: str  # "scalar", "array", "alias"
    alias_target: str | None = None  # for upvar visualisation
    children: list[Variable] | None = None  # array elements


@dataclass(slots=True)
class StopEvent:
    """Emitted when the debugger stops (breakpoint, step, etc.)."""

    line: int
    command_text: str
    reason: str  # "breakpoint", "step", "entry"
    frames: list[StackFrame] = field(default_factory=list)


# Callable signature for the VM debug hook.
DebugHook = Callable[["Instruction", int, list[str], "CallFrame"], DebugAction]

__all__ = [
    "DebugAction",
    "DebugHook",
    "StackFrame",
    "StepMode",
    "StopEvent",
    "Variable",
]
