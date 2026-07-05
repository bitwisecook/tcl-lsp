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
"""ASM::support_id -- Returns the support id of the HTTP transaction."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ASM__support_id.html"


@register
class AsmSupportIdCommand(CommandDef):
    name = "ASM::support_id"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ASM::support_id",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the support id of the HTTP transaction.",
                synopsis=("ASM::support_id",),
                snippet=(
                    "Returns the support id of the HTTP transaction, a unique\n"
                    "identifier assigned by ASM to the transaction, regardless of whether\n"
                    "violations were found in the transaction or not. The support id can be\n"
                    "used to correlate the transaction with its corresponding entry in the\n"
                    "request log and with the blocking page returned to the user in case of\n"
                    "blocking violations"
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ASM::support_id",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"ASM"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.ASM_STATE,
                    reads=True,
                    connection_side=ConnectionSide.CLIENT,
                ),
            ),
        )
