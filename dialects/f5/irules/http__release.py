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
"""HTTP::release -- Releases the data collected via HTTP::collect."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/HTTP__release.html"


@register
class HttpReleaseCommand(CommandDef):
    name = "HTTP::release"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="HTTP::release",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Releases the data collected via HTTP::collect.",
                synopsis=("HTTP::release",),
                snippet=(
                    "Releases the data collected via HTTP::collect. Unless a subsequent\n"
                    "HTTP::collect command was issued, there is no need to use the\n"
                    "HTTP::release command inside of the HTTP_REQUEST_DATA and\n"
                    "HTTP_RESPONSE_DATA events, since (in these cases) the data is\n"
                    "implicitly released.\n"
                    "It is important to note that these semantics are different than those\n"
                    "of the TCP::collect and TCP::release commands."
                ),
                source=_SOURCE,
                examples=(
                    "when CLIENT_ACCEPTED {\n"
                    "    set tmm_auth_ldap_sid [AUTH::start pam default_ldap]\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="HTTP::release",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            diagram_action=True,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.HTTP_BODY,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
