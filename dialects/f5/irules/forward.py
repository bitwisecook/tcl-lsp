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
"""forward -- Sets the connection to forward IP packets."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/forward.html"


@register
class ForwardCommand(CommandDef):
    name = "forward"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="forward",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Sets the connection to forward IP packets.",
                synopsis=("forward",),
                snippet=(
                    "Sets the connection to forward IP packets. This is strict forwarding\n"
                    "and will bypass any pool configured on the virtual server.\n"
                    "The request will be forwarded out the appropriate interface according\n"
                    "to the routes in the LTM routing table. No destination address or port\n"
                    "translation is performed."
                ),
                source=_SOURCE,
                examples=(
                    "when CLIENT_ACCEPTED {\n"
                    "  if { [class match [IP::client_addr] equals my_hosts_class]} {\n"
                    "    snat 192.168.100.12\n"
                    "  } else {\n"
                    "    forward\n"
                    "  }\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="forward",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(client_side=True, also_in=frozenset({"PERSIST_DOWN"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.CONNECTION_CONTROL,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
