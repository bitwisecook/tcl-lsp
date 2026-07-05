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
"""SSL::collect -- Collect plaintext data after SSL offloading."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/SSL__collect.html"


@register
class SslCollectCommand(CommandDef):
    name = "SSL::collect"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="SSL::collect",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Collect plaintext data after SSL offloading.",
                synopsis=("SSL::collect (LENGTH)?",),
                snippet="Starts the collection of plaintext data either indefinitely or for the specified amount of data. On successful collection, the corresponding data event is triggered. For clientside collection, the CLIENTSSL_DATA event is triggered. For serverside collection, the SERVERSSL_DATA event is triggered.",
                source=_SOURCE,
                examples=("when SERVERSSL_HANDSHAKE {\n    SSL::collect\n}"),
                return_value="SSL::collect [<length>] Starts the collection of plaintext data either indefinitely or for the specified amount of data. When is specified, the data event will not be triggered until that length has been collected.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="SSL::collect (LENGTH)?",
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
