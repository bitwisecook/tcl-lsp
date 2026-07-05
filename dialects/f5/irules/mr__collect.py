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
"""MR::collect -- Collect the specified amount of MR message payload data."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/MR__collect.html"


@register
class MrCollectCommand(CommandDef):
    name = "MR::collect"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="MR::collect",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Collect the specified amount of MR message payload data.",
                synopsis=("MR::collect (COLLECT)?",),
                snippet=(
                    "Collects the specified amount of MR message payload data before triggering a MR_DATA event.\n"
                    "\n"
                    "SYNTAX\n"
                    "\n"
                    "MR::collect [<collect_bytes>]\n"
                    "\n"
                    "MR::collect\n"
                    "        Collect the entire payload of the MR message. To stop collecting use MR::release command. MR_DATA event will be raised on every ingress invocation of the proxy.\n"
                    "\n"
                    "MR::collect <collect_bytes>\n"
                    "        Collect <collect_bytes> bytes of payload of the MR message.\n"
                    "        If payload is smaller than <collect_bytes> collect entire payload.\n"
                    "        The collected data can be accessed via the MR::payload command."
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="MR::collect (COLLECT)?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"MR"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.MESSAGE_STATE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
