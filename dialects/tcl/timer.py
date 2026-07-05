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

"""timer -- Schedule scripts on the wall-clock or monotonic clock (Tcl 9.1)."""

from __future__ import annotations

from compiler.registry._base import CommandDef, make_av
from compiler.registry.models import (
    CommandSpec,
    FormKind,
    FormSpec,
    HoverSnippet,
    SubCommand,
    ValidationSpec,
)
from compiler.registry.signatures import ArgRole, Arity, BodyKind
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from compiler.types import TclType

from ._base import register

_SOURCE = "Tcl man page timer.n"

_av = make_av(_SOURCE)

# The ``unit`` argument of every timer subcommand accepts these spellings.
_UNITS = (
    _av("us", "Microseconds.", "timer in delay us script"),
    _av("ms", "Milliseconds.", "timer in delay ms script"),
    _av("s", "Seconds.", "timer in delay s script"),
    _av("microseconds", "Microseconds.", "timer in delay microseconds script"),
    _av("milliseconds", "Milliseconds.", "timer in delay milliseconds script"),
    _av("seconds", "Seconds.", "timer in delay seconds script"),
)

_BODY = frozenset({ArgRole.BODY})


@register
class TimerCommand(CommandDef):
    name = "timer"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="timer",
            not_proc_factory=True,
            dialects=frozenset({"tcl9.1"}),
            never_inline_body=True,
            hover=HoverSnippet(
                summary=(
                    "Execute a script at a wall-clock point or after a monotonic delay (Tcl 9.1)."
                ),
                synopsis=("timer subcommand ?arg ...?",),
                snippet=(
                    "The monotonic clock is the right time base for timeouts, GUI "
                    "refreshes, and periodic processes; the wall clock (`timer at`) "
                    "tracks calendar time. When a script is supplied a timer id is "
                    "returned, which `timer cancel` can use to remove the event."
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="timer subcommand ?arg ...?",
                    arg_values={
                        0: (
                            _av(
                                "in",
                                "Execute the script after a monotonic delay; returns a timer id.",
                                "timer in delay unit script",
                            ),
                            _av(
                                "at",
                                "Execute the script at a wall-clock time point; "
                                "returns a timer id.",
                                "timer at timepoint unit script",
                            ),
                            _av(
                                "info",
                                "Return information about a registered timer event "
                                "(or all of them).",
                                "timer info ?id?",
                            ),
                            _av(
                                "cancel",
                                "Cancel a registered timer event by id.",
                                "timer cancel id",
                            ),
                            _av(
                                "idle",
                                "Register a script to run at idle time; returns a timer id.",
                                "timer idle script",
                            ),
                            _av(
                                "sleep",
                                "Block for a monotonic duration (`for`) or until a "
                                "wall-clock point (`until`).",
                                "timer sleep for time ?unit?",
                            ),
                        )
                    },
                ),
            ),
            subcommands={
                "in": SubCommand(
                    name="in",
                    arity=Arity(3, 3),
                    detail=(
                        "Execute the script after a monotonic delay of delay time "
                        "units; returns a timer id. The blocking form is "
                        "`timer sleep`."
                    ),
                    synopsis="timer in delay unit script",
                    arg_values={1: _UNITS},
                    arg_roles={2: _BODY},
                    body_kind=BodyKind.STRUCTURAL,
                    return_type=TclType.STRING,
                    mutator=True,
                ),
                "at": SubCommand(
                    name="at",
                    arity=Arity(3, 3),
                    detail=(
                        "Execute the script at a wall-clock timepoint expressed in "
                        "time units; returns a timer id. The blocking form is "
                        "`timer sleep`."
                    ),
                    synopsis="timer at timepoint unit script",
                    arg_values={1: _UNITS},
                    arg_roles={2: _BODY},
                    body_kind=BodyKind.STRUCTURAL,
                    return_type=TclType.STRING,
                    mutator=True,
                ),
                "info": SubCommand(
                    name="info",
                    arity=Arity(0, 1),
                    detail=(
                        "Return a list describing the given timer event, or the ids "
                        "of all registered events when no id is given."
                    ),
                    synopsis="timer info ?id?",
                    pure=True,
                    return_type=TclType.LIST,
                ),
                "cancel": SubCommand(
                    name="cancel",
                    arity=Arity(1, 1),
                    detail="Cancel the timer event with the given id; a no-op when unknown.",
                    synopsis="timer cancel id",
                    return_type=TclType.STRING,
                    mutator=True,
                ),
                "idle": SubCommand(
                    name="idle",
                    arity=Arity(1, 1),
                    detail="Register a script to be evaluated at idle time; returns a timer id.",
                    synopsis="timer idle script",
                    arg_roles={0: _BODY},
                    body_kind=BodyKind.STRUCTURAL,
                    return_type=TclType.STRING,
                    mutator=True,
                ),
                "sleep": SubCommand(
                    name="sleep",
                    arity=Arity(2, 3),
                    detail=(
                        "Block for at least a monotonic duration (`timer sleep for "
                        "time ?unit?`) or until a wall-clock point (`timer sleep "
                        "until time ?unit?`)."
                    ),
                    synopsis="timer sleep for|until time ?unit?",
                    arg_values={
                        0: (
                            _av(
                                "for",
                                "Sleep for a monotonic duration.",
                                "timer sleep for time ?unit?",
                            ),
                            _av(
                                "until",
                                "Sleep until a wall-clock point.",
                                "timer sleep until time ?unit?",
                            ),
                        ),
                        2: _UNITS,
                    },
                    return_type=TclType.STRING,
                ),
            },
            validation=ValidationSpec(
                arity=Arity(1),
            ),
            return_type=TclType.STRING,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.INTERP_STATE,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )
