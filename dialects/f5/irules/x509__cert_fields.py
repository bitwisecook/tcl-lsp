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
"""X509::cert_fields -- Returns a list of X509 certificate fields to be added to HTTP headers for ModSSL behavior."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/X509__cert_fields.html"


@register
class X509CertFieldsCommand(CommandDef):
    name = "X509::cert_fields"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="X509::cert_fields",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns a list of X509 certificate fields to be added to HTTP headers for ModSSL behavior.",
                synopsis=("X509::cert_fields CERTIFICATE ERROR_CODE ((hash",),
                snippet=(
                    "When given a valid certificate, returns a TCL list of field names and\n"
                    "values which can be added to the HTTP headers in order to emulate\n"
                    "ModSSL behavior. The output can be passed to 'HTTP::header insert\n"
                    "$list' as a list for insertion in the HTTP request or response."
                ),
                source=_SOURCE,
                examples=(
                    "when CLIENTSSL_CLIENTCERT {\n"
                    "    if { [SSL::cert count] > 0 } {\n"
                    "        session add ssl [SSL::sessionid] [X509::cert_fields [SSL::cert 0] [SSL::verify_result] whole] $timeout\n"
                    "    }\n"
                    "}"
                ),
                return_value="Returns a list of X509 certificate fields to be added to HTTP headers.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="X509::cert_fields CERTIFICATE ERROR_CODE ((hash",
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
