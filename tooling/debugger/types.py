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

"""Data types shared across the debugger package."""

from __future__ import annotations

from collections.abc import Callable
from dataclasses import dataclass, field
from enum import Enum, auto
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from compiler.codegen.bytecode import Instruction
    from tooling.vm.scope import CallFrame


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
