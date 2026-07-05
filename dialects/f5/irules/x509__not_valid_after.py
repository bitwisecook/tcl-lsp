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
"""X509::not_valid_after -- Returns the not-valid-after date of an X509 certificate."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/X509__not_valid_after.html"


@register
class X509NotValidAfterCommand(CommandDef):
    name = "X509::not_valid_after"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="X509::not_valid_after",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the not-valid-after date of an X509 certificate.",
                synopsis=("X509::not_valid_after CERTIFICATE",),
                snippet="Returns the not-valid-after date of the specified X509 certificate.",
                source=_SOURCE,
                examples=(
                    "when SERVERSSL_HANDSHAKE {\n"
                    "  set server_cert [SSL::cert 0]\n"
                    '  log local0. "Server Certificate Valid Date -\n'
                    "   [X509::not_valid_before $server_cert] -\n"
                    '   [X509::not_valid_after $server_cert]"\n'
                    "}"
                ),
                return_value="Returns the not-valid-after date of an X509 certificate.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="X509::not_valid_after CERTIFICATE",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.SSL_STATE,
                    reads=True,
                    connection_side=ConnectionSide.GLOBAL,
                ),
            ),
        )
