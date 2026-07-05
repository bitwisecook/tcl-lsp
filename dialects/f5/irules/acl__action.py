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
"""ACL::action -- Sets or retrieves the current ACL action."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ACL__action.html"


@register
class AclActionCommand(CommandDef):
    name = "ACL::action"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ACL::action",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Sets or retrieves the current ACL action.",
                synopsis=("ACL::action (default |",),
                snippet=(
                    "The ACL::action command allows you to determine the ACL action in the\n"
                    "FLOW_INIT event. This command requires the Advanced Firewall\n"
                    "Manager module."
                ),
                source=_SOURCE,
                examples=(
                    "when FLOW_INIT {\n"
                    "  if { [IP::addr [IP::client_addr] equals 172.29.97.151] } {\n"
                    "    ACL::action allow\n"
                    "    virtual /Common/my_http_vs\n"
                    '    log "FLOW_INIT: ACL allow to /Common/my_http_vs"\n'
                    "  }\n"
                    "}"
                ),
                return_value="When no argument is provided, the command will return an integer value corresponding to an action that will be taken: + 0 is a drop + 1 is reset (or reject) + 2 is allow (or accept) + 3 is allow-final (or accept-decisively)",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ACL::action (default |",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.ASM_STATE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
