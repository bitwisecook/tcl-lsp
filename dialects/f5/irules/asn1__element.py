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
"""ASN1::element -- Returns ASN1.1 record elements."""

from __future__ import annotations

from compiler.registry._base import CommandDef, make_av
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ASN1__element.html"


_av = make_av(_SOURCE)


@register
class Asn1ElementCommand(CommandDef):
    name = "ASN1::element"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ASN1::element",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns ASN1.1 record elements.",
                synopsis=(
                    "ASN1::element init ('BER' | 'DER')",
                    "ASN1::element next ELEMENT (NUM_ELEMENTS)?",
                    "ASN1::element byte_offset ELEMENT (OFFSET)?",
                    "ASN1::element tag ELEMENT",
                ),
                snippet=(
                    "This command returns ASN1.1 record elements.\n"
                    "\n"
                    "ASN1::element init encodingType\n"
                    "\n"
                    "     * Returns an element (Tcl_Obj) handle used by the remaining commands.\n"
                    "       encodingType specifies the encoding type that subsequent commands\n"
                    "       should use (BER|DER).\n"
                    "\n"
                    "ASN1::element next element ?numberOfElements?\n"
                    "\n"
                    "     * Returns the next element found after element. If numberOfElements\n"
                    "       is specified, the command will move ahead that many elements,\n"
                    "       otherwise, the default is 1.\n"
                    "\n"
                    "ASN1::element byte_offset element ?offset?\n"
                    "\n"
                    "     * Returns the byte offset within the payload."
                ),
                source=_SOURCE,
                examples=("when CLIENT_ACCEPTED {\n  TCP::collect\n}"),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ASN1::element init ('BER' | 'DER')",
                    arg_values={
                        0: (
                            _av("BER", "ASN1::element BER", "ASN1::element init ('BER' | 'DER')"),
                            _av("DER", "ASN1::element DER", "ASN1::element init ('BER' | 'DER')"),
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
                    target=SideEffectTarget.UNKNOWN, reads=True, connection_side=ConnectionSide.NONE
                ),
            ),
        )
