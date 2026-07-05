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

# Scaffolded from vwait.n -- refine and commit
"""vwait -- Process events until a variable is written."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.dialects import DIALECTS_EXCEPT_IRULES
from compiler.registry.models import (
    CommandSpec,
    FormKind,
    FormSpec,
    HoverSnippet,
    OptionSpec,
    ValidationSpec,
)
from compiler.registry.signatures import ArgRole, Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from compiler.types import TclType

from ._base import register

_SOURCE = "Tcl man page vwait.n"


@register
class VwaitCommand(CommandDef):
    name = "vwait"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="vwait",
            byte_compiled=True,
            dialects=DIALECTS_EXCEPT_IRULES,
            hover=HoverSnippet(
                summary="Process events until a variable is written",
                synopsis=(
                    "vwait varName",
                    "vwait ?options? ?varName ...?",
                ),
                snippet="This command enters the Tcl event loop to process events, blocking the application if no events are ready.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="vwait varName",
                    options=(
                        OptionSpec(
                            name="--", detail="Marks the end of options.", takes_value=False
                        ),
                        # All option-driven vwait forms are Tcl 9.0+ (TIP
                        # 555).  In 8.x ``vwait`` takes only a single
                        # variable name (``vwait varName``); a leading
                        # ``-foo`` is treated as the variable name and
                        # fails the "wrong # args" arity check when
                        # extra positional args follow.
                        OptionSpec(
                            name="-all",
                            detail="All wait conditions must be met to complete (Tcl 9.0+).",
                            takes_value=False,
                            dialects=frozenset({"tcl9.0"}),
                        ),
                        OptionSpec(
                            name="-extended",
                            detail="Return an extended list-form result (Tcl 9.0+).",
                            takes_value=False,
                            dialects=frozenset({"tcl9.0"}),
                        ),
                        OptionSpec(
                            name="-nofileevents",
                            detail="File events not handled during the wait (Tcl 9.0+).",
                            takes_value=False,
                            dialects=frozenset({"tcl9.0"}),
                        ),
                        OptionSpec(
                            name="-noidleevents",
                            detail="Idle handlers not invoked during the wait (Tcl 9.0+).",
                            takes_value=False,
                            dialects=frozenset({"tcl9.0"}),
                        ),
                        OptionSpec(
                            name="-notimerevents",
                            detail="Timer handlers not serviced during the wait (Tcl 9.0+).",
                            takes_value=False,
                            dialects=frozenset({"tcl9.0"}),
                        ),
                        OptionSpec(
                            name="-nowindowevents",
                            detail="Window-system events not handled during the wait (Tcl 9.0+).",
                            takes_value=False,
                            dialects=frozenset({"tcl9.0"}),
                        ),
                        OptionSpec(
                            name="-readable",
                            detail="Channel must be open for reading (Tcl 9.0+).",
                            takes_value=True,
                            dialects=frozenset({"tcl9.0"}),
                        ),
                        OptionSpec(
                            name="-timeout",
                            detail="Bound the wait to N milliseconds (Tcl 9.0+).",
                            takes_value=True,
                            dialects=frozenset({"tcl9.0"}),
                        ),
                        OptionSpec(
                            name="-variable",
                            detail="VarName must name a global variable (Tcl 9.0+).",
                            takes_value=True,
                            dialects=frozenset({"tcl9.0"}),
                        ),
                        OptionSpec(
                            name="-writable",
                            detail="Channel must be open for writing (Tcl 9.0+).",
                            takes_value=True,
                            dialects=frozenset({"tcl9.0"}),
                        ),
                    ),
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(1, 1),
            ),
            creates_dynamic_barrier=True,
            arg_roles={0: frozenset({ArgRole.VAR_READ})},
            return_type=TclType.STRING,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.INTERP_STATE,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )
