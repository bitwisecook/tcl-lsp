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
"""ANTIFRAUD::disable_auto_transactions -- Disables automatic transactions for the current transaction."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ANTIFRAUD__disable_auto_transactions.html"


@register
class AntifraudDisableAutoTransactionsCommand(CommandDef):
    name = "ANTIFRAUD::disable_auto_transactions"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ANTIFRAUD::disable_auto_transactions",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Disables automatic transactions for the current transaction.",
                synopsis=("ANTIFRAUD::disable_auto_transactions",),
                snippet="Disables automatic transactions for the current transaction.",
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    '                if { [HTTP::header exists "Antifraud-Disable-AutoTransactions" ] } {\n'
                    "                    ANTIFRAUD::disable_auto_transactions\n"
                    '                    log local0. "Automatic Transactions disabled"\n'
                    "                }\n"
                    "            }"
                ),
                return_value="Disables automatic transactions for the current transaction.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ANTIFRAUD::disable_auto_transactions",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"FASTHTTP"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.ASM_STATE,
                    writes=True,
                    connection_side=ConnectionSide.CLIENT,
                ),
            ),
        )
