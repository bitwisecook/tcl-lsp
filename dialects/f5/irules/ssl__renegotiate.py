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
"""SSL::renegotiate -- Controls renegotiation of an SSL connection."""

from __future__ import annotations

from compiler.registry._base import CommandDef, make_av
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/SSL__renegotiate.html"


_av = make_av(_SOURCE)


@register
class SslRenegotiateCommand(CommandDef):
    name = "SSL::renegotiate"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="SSL::renegotiate",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Controls renegotiation of an SSL connection.",
                synopsis=("SSL::renegotiate (enable | disable)?",),
                snippet=(
                    "Controls renegotiation of an SSL connection, often used to enforce new encryption settings or certificate requirements.\n"
                    "\n"
                    "This command has different results depending on whether the BIG-IP system evaluates the command under a client-side or a server-side context. The command only succeeds if SSL is enabled on the connection; otherwise, the command returns an error."
                ),
                source=_SOURCE,
                examples=("when CLIENTSSL_HANDSHAKE {\n    SSL::renegotiate disable\n}"),
                return_value="SSL::renegotiate Renegotiates a client-side or server-side SSL connection, depending on the context. When the system evaluates the command under a client-side context, the system immediately renegotiates a request for the associated client-side connection, if client-side renegotiation is enabled.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="SSL::renegotiate (enable | disable)?",
                    arg_values={
                        0: (
                            _av(
                                "enable",
                                "SSL::renegotiate enable",
                                "SSL::renegotiate (enable | disable)?",
                            ),
                            _av(
                                "disable",
                                "SSL::renegotiate disable",
                                "SSL::renegotiate (enable | disable)?",
                            ),
                        )
                    },
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
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
