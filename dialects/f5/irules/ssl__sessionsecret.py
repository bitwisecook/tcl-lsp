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
"""SSL::sessionsecret -- Return data about SSL handshake master secret."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/SSL__sessionsecret.html"


@register
class SslSessionsecretCommand(CommandDef):
    name = "SSL::sessionsecret"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="SSL::sessionsecret",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Return data about SSL handshake master secret.",
                synopsis=("SSL::sessionsecret",),
                snippet="Return data about SSL handshake master secret.",
                source=_SOURCE,
                examples=(
                    "when CLIENTSSL_HANDSHAKE {\n"
                    '    log local0. "ClientSSL: Secret for [SSL::sessionid] is -> [SSL::sessionsecret]"\n'
                    "}"
                ),
                return_value="SSL::sessionsecret Returns the current SSL handshake master secret.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="SSL::sessionsecret",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(
                transport="tcp", profiles=frozenset({"CLIENTSSL", "SERVERSSL"})
            ),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.SSL_STATE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
