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
"""XLAT::listen_lifetime -- Set/Get the listener lifetime."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/XLAT__listen_lifetime.html"


@register
class XlatListenLifetimeCommand(CommandDef):
    name = "XLAT::listen_lifetime"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="XLAT::listen_lifetime",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Set/Get the listener lifetime.",
                synopsis=("XLAT::listen_lifetime (HANDLE)+ (XLAT_LIFETIME)?",),
                snippet=(
                    "Set/Get the listener lifetime.\n"
                    "Valid range is between 0 and 31536000 (365 days)."
                ),
                source=_SOURCE,
                examples=(
                    "when SERVER_CONNECTED {\n"
                    "    set listener [XLAT::listen 30 {\n"
                    "        proto [IP::protocol]\n"
                    "        bind -allow [serverside {LINK::vlan_id}] -ip [serverside {IP::local_addr}]\n"
                    "        server [IP::client_addr] [expr [TCP::local_port] + 1]\n"
                    "        allow [LB::server addr] 0\n"
                    "    }]\n"
                    '    log local0. "[XLAT::listen_lifetime $listener]"\n'
                    "}"
                ),
                return_value="Return the listener lifetime value.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="XLAT::listen_lifetime (HANDLE)+ (XLAT_LIFETIME)?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.LSN_STATE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
