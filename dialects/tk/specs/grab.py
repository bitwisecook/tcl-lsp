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

"""grab -- Confine pointer and keyboard events to a window sub-tree."""

from __future__ import annotations

from compiler.registry._base import CommandDef, make_av
from compiler.registry.models import (
    CommandSpec,
    FormKind,
    FormSpec,
    HoverSnippet,
    OptionSpec,
    SubCommand,
    ValidationSpec,
)
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import register

_SOURCE = "Tk man page grab.n"
_av = make_av(_SOURCE)


@register
class GrabCommand(CommandDef):
    name = "grab"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="grab",
            required_package="Tk",
            hover=HoverSnippet(
                summary="Confine pointer and keyboard events to a window sub-tree.",
                synopsis=(
                    "grab ?-global? window",
                    "grab current ?window?",
                    "grab release window",
                    "grab set ?-global? window",
                    "grab status window",
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="grab option ?arg ...?",
                    options=(
                        OptionSpec(
                            name="-global",
                            takes_value=False,
                            detail="Make the grab global (applies to all displays).",
                        ),
                    ),
                    arg_values={
                        0: (
                            _av(
                                "current",
                                "Return the path name of the current grab window, if any.",
                                "grab current ?window?",
                            ),
                            _av(
                                "release",
                                "Release the grab on the window.",
                                "grab release window",
                            ),
                            _av(
                                "set",
                                "Set a grab on the window, optionally global.",
                                "grab set ?-global? window",
                            ),
                            _av(
                                "status",
                                "Return the grab status of the window (none, local, or global).",
                                "grab status window",
                            ),
                        ),
                    },
                ),
            ),
            subcommands={
                "current": SubCommand(
                    name="current",
                    arity=Arity(0, 1),
                    detail="Return the path name of the current grab window, if any.",
                    synopsis="grab current ?window?",
                ),
                "release": SubCommand(
                    name="release",
                    arity=Arity(1, 1),
                    detail="Release the grab on the window.",
                    synopsis="grab release window",
                ),
                "set": SubCommand(
                    name="set",
                    arity=Arity(1, 2),
                    detail="Set a grab on the window, optionally global.",
                    synopsis="grab set ?-global? window",
                ),
                "status": SubCommand(
                    name="status",
                    arity=Arity(1, 1),
                    detail="Return the grab status of the window (none, local, or global).",
                    synopsis="grab status window",
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
