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

# Enriched from F5 iRules reference documentation.
"""HSL::open -- Opens a handle for High Speed Logging communication."""

from __future__ import annotations

from compiler.registry._base import CommandDef, make_av
from compiler.registry.models import (
    CommandSpec,
    FormKind,
    FormSpec,
    HoverSnippet,
    OptionSpec,
    ValidationSpec,
)
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from compiler.types import TclType

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/HSL__open.html"


_av = make_av(_SOURCE)


@register
class HslOpenCommand(CommandDef):
    name = "HSL::open"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="HSL::open",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Opens a handle for High Speed Logging communication.",
                synopsis=(
                    "HSL::open ('-publisher' | '-pub') PUBLISHER",
                    "HSL::open '-proto' ('UDP' | 'TCP') '-pool' POOL_OBJ",
                ),
                snippet=(
                    "Open a handle for High Speed Logging communication. After creating the\n"
                    "connection, send data on the connection using HSL::send."
                ),
                source=_SOURCE,
                examples=(
                    "#2\n"
                    "when CLIENT_ACCEPTED {\n"
                    "    set hsl [HSL::open -publisher /Common/lpAll]\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="HSL::open ('-publisher' | '-pub') PUBLISHER",
                    options=(
                        OptionSpec(
                            name="-publisher", detail="Option -publisher.", takes_value=True
                        ),
                        OptionSpec(name="-pub", detail="Option -pub.", takes_value=True),
                        OptionSpec(name="-proto", detail="Option -proto.", takes_value=True),
                        OptionSpec(name="-pool", detail="Option -pool.", takes_value=True),
                    ),
                    arg_values={
                        0: (
                            _av(
                                "UDP",
                                "HSL::open UDP",
                                "HSL::open '-proto' ('UDP' | 'TCP') '-pool' POOL_OBJ",
                            ),
                            _av(
                                "TCP",
                                "HSL::open TCP",
                                "HSL::open '-proto' ('UDP' | 'TCP') '-pool' POOL_OBJ",
                            ),
                        )
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            return_type=TclType.CHANNEL,
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.LOG_IO, writes=True, connection_side=ConnectionSide.BOTH
                ),
            ),
        )
