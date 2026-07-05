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
"""GENERICMESSAGE::peer -- Returns or sets the peer's route name."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/GENERICMESSAGE__peer.html"


@register
class GenericmessagePeerCommand(CommandDef):
    name = "GENERICMESSAGE::peer"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="GENERICMESSAGE::peer",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns or sets the peer's route name.",
                synopsis=("GENERICMESSAGE::peer name (NAME)?",),
                snippet=(
                    "The GENERICMESSAGE::peer command returns or sets the peer's route name\n"
                    "in the message routing framework. The peer name will be automatically\n"
                    "set as the source address of each message."
                ),
                source=_SOURCE,
                examples=(
                    "when CLIENT_ACCEPTED {\n"
                    '    GENERICMESSAGE::peer name "[IP::remote_addr]:[TCP::remote_port]"\n'
                    "}"
                ),
                return_value="Returns the peer's route name.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="GENERICMESSAGE::peer name (NAME)?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.MESSAGE_STATE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
