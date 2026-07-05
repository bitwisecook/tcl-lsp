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
"""local_addr -- Deprecated: Use IP::local_addr instead."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register
from .ip__local_addr import IpLocalAddrCommand

_SOURCE = "https://clouddocs.f5.com/api/irules/local_addr.html"


@register
class LocalAddrCommand(CommandDef):
    name = "local_addr"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="local_addr",
            deprecated_replacement=IpLocalAddrCommand,
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Deprecated: Use IP::local_addr instead.",
                synopsis=("local_addr",),
                snippet="Deprecated: Use IP::local_addr instead",
                source=_SOURCE,
                examples=(
                    'when CLIENT_ACCEPTED {\n    log local0. "client ip addr: [local_addr]"\n}'
                ),
                return_value="Returns the IP address being used in the connection. In the clientside context, this is the destination IP address from the client request (not necessarily the virtual IP address).",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="local_addr",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.TCP_STATE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
