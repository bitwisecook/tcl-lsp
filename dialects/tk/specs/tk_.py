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

"""tk -- Manipulate Tk internal state."""

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
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import register

_SOURCE = "Tk man page tk.n"
_av = make_av(_SOURCE)


@register
class TkCommand(CommandDef):
    name = "tk"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="tk",
            required_package="Tk",
            hover=HoverSnippet(
                summary="Manipulate Tk internal state.",
                synopsis=(
                    "tk appname ?newName?",
                    "tk busy subcommand ?arg ...?",
                    "tk caret window ?-x x? ?-y y? ?-height height?",
                    "tk fontchooser subcommand ?arg ...?",
                    "tk inactive ?-displayof window? ?reset?",
                    "tk scaling ?-displayof window? ?number?",
                    "tk useinputmethods ?-displayof window? ?boolean?",
                    "tk windowingsystem",
                ),
                snippet=(
                    "Provides access to miscellaneous Tk internal state and the windowing system."
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="tk subcommand ?arg ...?",
                    arg_values={
                        0: (
                            _av(
                                "appname",
                                "Query or set the application name for send commands.",
                                "tk appname ?newName?",
                            ),
                            _av(
                                "busy",
                                "Make a window appear busy (greyed out with a busy cursor).",
                                "tk busy subcommand ?arg ...?",
                            ),
                            _av(
                                "caret",
                                "Query or set the caret (text cursor) position for accessibility.",
                                "tk caret window ?-x x? ?-y y? ?-height height?",
                            ),
                            _av(
                                "fontchooser",
                                "Control the platform font selection dialogue.",
                                "tk fontchooser subcommand ?arg ...?",
                            ),
                            _av(
                                "inactive",
                                "Query or reset the user inactivity timer in milliseconds.",
                                "tk inactive ?-displayof window? ?reset?",
                            ),
                            _av(
                                "scaling",
                                "Query or set the number of pixels per point on the display.",
                                "tk scaling ?-displayof window? ?number?",
                            ),
                            _av(
                                "useinputmethods",
                                "Query or set whether Tk should use XIM input methods.",
                                "tk useinputmethods ?-displayof window? ?boolean?",
                            ),
                            _av(
                                "windowingsystem",
                                "Return the windowing system in use: x11, win32, or aqua.",
                                "tk windowingsystem",
                            ),
                        ),
                    },
                ),
            ),
            subcommands={
                "appname": SubCommand(
                    name="appname",
                    arity=Arity(0, 1),
                    detail="Query or set the application name for send commands.",
                    synopsis="tk appname ?newName?",
                ),
                "busy": SubCommand(
                    name="busy",
                    arity=Arity(1),
                    detail="Make a window appear busy (greyed out with a busy cursor).",
                    synopsis="tk busy subcommand ?arg ...?",
                ),
                "caret": SubCommand(
                    name="caret",
                    arity=Arity(1),
                    detail="Query or set the caret (text cursor) position for accessibility.",
                    synopsis="tk caret window ?-x x? ?-y y? ?-height height?",
                ),
                "fontchooser": SubCommand(
                    name="fontchooser",
                    arity=Arity(1),
                    detail="Control the platform font selection dialogue.",
                    synopsis="tk fontchooser subcommand ?arg ...?",
                ),
                "inactive": SubCommand(
                    name="inactive",
                    arity=Arity(0),
                    detail="Query or reset the user inactivity timer in milliseconds.",
                    synopsis="tk inactive ?-displayof window? ?reset?",
                ),
                "scaling": SubCommand(
                    name="scaling",
                    arity=Arity(0),
                    detail="Query or set the number of pixels per point on the display.",
                    synopsis="tk scaling ?-displayof window? ?number?",
                ),
                "useinputmethods": SubCommand(
                    name="useinputmethods",
                    arity=Arity(0),
                    detail="Query or set whether Tk should use XIM input methods.",
                    synopsis="tk useinputmethods ?-displayof window? ?boolean?",
                ),
                "windowingsystem": SubCommand(
                    name="windowingsystem",
                    arity=Arity(0, 0),
                    detail="Return the windowing system in use: x11, win32, or aqua.",
                    synopsis="tk windowingsystem",
                ),
            },
            validation=ValidationSpec(
                arity=Arity(1),
            ),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.INTERP_STATE,
                    reads=True,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )
