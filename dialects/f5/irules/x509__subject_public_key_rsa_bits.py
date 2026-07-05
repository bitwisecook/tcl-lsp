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
"""X509::subject_public_key_RSA_bits -- Returns the size of the subjectXs public RSA key of an X509 certificate."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/X509__subject_public_key_RSA_bits.html"


@register
class X509SubjectPublicKeyRsaBitsCommand(CommandDef):
    name = "X509::subject_public_key_RSA_bits"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="X509::subject_public_key_RSA_bits",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the size of the subjectXs public RSA key of an X509 certificate.",
                synopsis=("X509::subject_public_key_RSA_bits CERTIFICATE",),
                snippet=(
                    "Returns the size, in bits, of the subject’s public RSA key of the\n"
                    "specified X509 certificate. This command is only applicable when the\n"
                    "public key type is RSA. Otherwise, the command generates an error."
                ),
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    "  if { [info exist error_code] } {\n"
                    "    if { $error_code > 0 } {\n"
                    '      HTTP::redirect "https://some_other_site/"\n'
                    "    }\n"
                    "  }\n"
                    "}"
                ),
                return_value="Returns the size of the subject’s public RSA key of an X509 certificate.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="X509::subject_public_key_RSA_bits CERTIFICATE",
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
