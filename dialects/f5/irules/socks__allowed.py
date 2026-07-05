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
"""SOCKS::allowed -- This command allows you to change whether the SOCKS request is allowed or not."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/SOCKS__allowed.html"


@register
class SocksAllowedCommand(CommandDef):
    name = "SOCKS::allowed"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="SOCKS::allowed",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="This command allows you to change whether the SOCKS request is allowed or not.",
                synopsis=("SOCKS::allowed ('0' | '1')?",),
                snippet=(
                    "This command allows you to reject a SOCKS request during the SOCKS_REQUEST event.\n"
                    "\n"
                    "Details (Syntax):\n"
                    "SOCKS::allowed '0' | '1'\n"
                    "    Sets the state of SOCKS based on the Boolean value."
                ),
                source=_SOURCE,
                examples=(
                    "# Reject all SOCKS requests:\nwhen SOCKS_REQUEST {\n    SOCKS::allowed 0\n}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="SOCKS::allowed ('0' | '1')?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"SOCKS"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.NETWORK_IO,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
