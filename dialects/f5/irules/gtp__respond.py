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
"""GTP::respond -- Sends the GTP message back to the remote node of this connection."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/GTP__respond.html"


@register
class GtpRespondCommand(CommandDef):
    name = "GTP::respond"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="GTP::respond",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Sends the GTP message back to the remote node of this connection.",
                synopsis=("GTP::respond MESSAGE",),
                snippet=(
                    "Sends this GTP message back to the remote node of this connection.\n"
                    "If this is clientside flow, send it back to client that initiated the connection.\n"
                    "If this is serverside flow, send it to the remote node that is connected to."
                ),
                source=_SOURCE,
                examples=(
                    "when GTP_SIGNALLING_EGRESS {\n"
                    "    set t2 [GTP::new 2 10]\n"
                    "    GTP::respond $t2\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="GTP::respond MESSAGE",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.NETWORK_IO,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
