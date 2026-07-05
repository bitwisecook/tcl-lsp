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
"""DIAMETER::persist -- Returns the persistence key being used for the current message."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/DIAMETER__persist.html"


@register
class DiameterPersistCommand(CommandDef):
    name = "DIAMETER::persist"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="DIAMETER::persist",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the persistence key being used for the current message.",
                synopsis=(
                    "DIAMETER::persist",
                    "DIAMETER::persist reset",
                    "DIAMETER::persist use",
                ),
                snippet=(
                    "This iRule command returns the persistence key being used for the\n"
                    "current message. If new persist key is provided, the existing\n"
                    "persistence key will be replaced. The value of the new key MUST be the\n"
                    "value of a valid AVP in the message. An AVP attribute name should not\n"
                    "be given as the new key value.\n"
                    "\n"
                    "If bidirection is specified as false, disable(d), no, 0, or is\n"
                    "unspecified, then persistence is not bidirectional. If bidirection is\n"
                    "specified as true, enable(d), yes, or 1 this persistence entry is\n"
                    "bidirectional."
                ),
                source=_SOURCE,
                examples=(
                    "when DIAMETER_INGRESS {\n"
                    '    log local0. "Received a DIAMETER message, persistence key is [DIAMETER::persist]"\n'
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="DIAMETER::persist",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"DIAMETER", "MR"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.PERSISTENCE_TABLE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
