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
"""listen -- Sets up a related ephemeral listener to allow an incoming related connection to be established."""

from __future__ import annotations

from compiler.registry._base import CommandDef, make_av
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/listen.html"


_av = make_av(_SOURCE)


@register
class ListenCommand(CommandDef):
    name = "listen"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="listen",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Sets up a related ephemeral listener to allow an incoming related connection to be established.",
                synopsis=("listen (<'proto' UNSIGNED_SHORT> |",),
                snippet=(
                    "Sets up a related ephemeral listener to allow an incoming related\n"
                    "connection to be established. The source address and/or port of the\n"
                    "related connection is unknown but the destination address and port are\n"
                    "known."
                ),
                source=_SOURCE,
                examples=('when RULE_INIT {\n      set my_port ""\n   }'),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="listen (<'proto' UNSIGNED_SHORT> |",
                    arg_values={
                        0: (_av("proto", "listen proto", "listen (<'proto' UNSIGNED_SHORT> |"),)
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(client_side=True, also_in=frozenset({"PERSIST_DOWN"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.NETWORK_IO,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
