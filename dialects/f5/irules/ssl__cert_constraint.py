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
"""SSL::cert_constraint -- Inserts cert constraint information to the certificate."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/SSL__cert_constraint.html"


@register
class SslCertConstraintCommand(CommandDef):
    name = "SSL::cert_constraint"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="SSL::cert_constraint",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Inserts cert constraint information to the certificate.",
                synopsis=("SSL::cert_constraint (ARG ARG)",),
                snippet="Inserts a certificate extension to the certificate.",
                source=_SOURCE,
                examples=(
                    "when CLIENTSSL_HANDSHAKE {\n"
                    '    log local0.info "CLIENTSSL_HANDSHAKE"\n'
                    '    SSL::cert_constraint 1.2.3.4.5 "This is the oid-value of 1.2.3.4.5"\n'
                    "}"
                ),
                return_value="SSL::cert_constraint <oid oid-value> Inserts the <oid oid-value> as an extension with OID=oid and value=oid-value to the certificate.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="SSL::cert_constraint (ARG ARG)",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(
                client_side=True, transport="tcp", profiles=frozenset({"CLIENTSSL"})
            ),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.SSL_STATE,
                    reads=True,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
