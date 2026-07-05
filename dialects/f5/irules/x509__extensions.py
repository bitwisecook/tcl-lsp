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
"""X509::extensions -- Returns the X509 extensions set on an X509 certificate."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/X509__extensions.html"


@register
class X509ExtensionsCommand(CommandDef):
    name = "X509::extensions"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="X509::extensions",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the X509 extensions set on an X509 certificate.",
                synopsis=("X509::extensions CERTIFICATE",),
                snippet="Returns the X509 extensions set on the specified X509 certificate.",
                source=_SOURCE,
                examples=(
                    "when CLIENTSSL_CLIENTCERT {\n"
                    "    set myCert [SSL::cert 0]\n"
                    "    set result [X509::extensions $myCert]\n"
                    '    log local0. "X509::extensions $result"\n'
                    "\n"
                    '    if { $result matches_glob "*X509v3 extensions:*X509v3 Basic*" } {\n'
                    '        log local0. "match"\n'
                    "    } else {\n"
                    '        log local0. "no match"\n'
                    "    }\n"
                    "}"
                ),
                return_value="Returns the X509 extensions set on an X509 certificate.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="X509::extensions CERTIFICATE",
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
