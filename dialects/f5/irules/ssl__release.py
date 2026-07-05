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
"""SSL::release -- Releases the collected plaintext data."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/SSL__release.html"


@register
class SslReleaseCommand(CommandDef):
    name = "SSL::release"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="SSL::release",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Releases the collected plaintext data.",
                synopsis=("SSL::release (LENGTH)?",),
                snippet="Releases the collected plaintext data to the next layer/filter up.",
                source=_SOURCE,
                examples=(
                    "when SERVERSSL_DATA {\n"
                    "    # Do something with the decrypted data\n"
                    "    set payload [SSL::payload]\n"
                    "\n"
                    "    # Release the payload\n"
                    "    SSL::release\n"
                    "}"
                ),
                return_value="SSL::release [<length>] Releases the collected plaintext data to the next layer/filter up.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="SSL::release (LENGTH)?",
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
