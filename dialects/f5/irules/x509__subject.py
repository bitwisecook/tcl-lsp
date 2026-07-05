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
"""X509::subject -- Returns the subject of an X509 certificate."""

from __future__ import annotations

from compiler.registry._base import CommandDef, make_av
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/X509__subject.html"


_av = make_av(_SOURCE)


@register
class X509SubjectCommand(CommandDef):
    name = "X509::subject"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="X509::subject",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the subject of an X509 certificate.",
                synopsis=("X509::subject CERTIFICATE (commonName)?",),
                snippet=(
                    "Returns the subject of the specified X509 certificate.\n"
                    "If commonName RDN is specified, returns the Subject CN in UTF8 format."
                ),
                source=_SOURCE,
                examples=(
                    "when CLIENTSSL_HANDSHAKE {\n"
                    "\n"
                    "  # Check if the client supplied one or more client certs\n"
                    "  if {[SSL::cert count] > 0}{\n"
                    "\n"
                    "    # Check the first client cert subject\n"
                    '    if { [X509::subject [SSL::cert 0]] equals "someSubject" } {\n'
                    '      log local0. "X509 Certificate Subject [X509::subject [SSL::cert 0]]"\n'
                    "      pool my_pool\n"
                    "    }\n"
                    "    # Check the first client cert subject commonName\n"
                    '    if { [X509::subject [SSL::cert 0] commonName] equals "someCommonName" } {'
                ),
                return_value="Returns the subject of an X509 certificate. If commonName RDN is specified, returns the Subject CN in UTF8 format.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="X509::subject CERTIFICATE (commonName)?",
                    arg_values={
                        0: (
                            _av(
                                "commonName",
                                "X509::subject commonName",
                                "X509::subject CERTIFICATE (commonName)?",
                            ),
                        )
                    },
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
