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
"""client_port -- Returns the TCP port number/service of the specified client."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register
from .tcp__client_port import TcpClientPortCommand

_SOURCE = "https://clouddocs.f5.com/api/irules/client_port.html"


@register
class ClientPortCommand(CommandDef):
    name = "client_port"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="client_port",
            deprecated_replacement=TcpClientPortCommand,
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the TCP port number/service of the specified client.",
                synopsis=("client_port",),
                snippet="Returns the TCP port number/service of the specified client. This is a BIG-IP version 4.X variable, provided for backward compatibility. You can use the equivalent 9.X command, TCP::client_port instead.",
                source=_SOURCE,
                return_value="client_port Returns the TCP port number/service of the specified client.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="client_port",
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
                    connection_side=ConnectionSide.CLIENT,
                ),
            ),
        )
