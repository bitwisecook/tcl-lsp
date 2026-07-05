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
"""BWC::debug -- This command is used for troubleshooting a bwc policy instance."""

from __future__ import annotations

from compiler.registry._base import CommandDef, make_av
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/BWC__debug.html"


_av = make_av(_SOURCE)


@register
class BwcDebugCommand(CommandDef):
    name = "BWC::debug"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="BWC::debug",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="This command is used for troubleshooting a bwc policy instance.",
                synopsis=(
                    "BWC::debug ('start')",
                    "BWC::debug ('stop')",
                ),
                snippet="This command enables debug logs on per policy instance. However the bwc sys db variables for bwc trace need to be enabled and appropriate levels needs to be set as required.",
                source=_SOURCE,
                examples=(
                    "when CLIENT_ACCEPTED {\n"
                    "    set mycookie [IP::remote_addr]:[TCP::remote_port]\n"
                    "    BWC::policy attach test_pol $mycookie\n"
                    '    log  local0. "BWC::policy attach  $mycookie"\n'
                    "    BWC::debug start session\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="BWC::debug ('start')",
                    arg_values={
                        0: (
                            _av("start", "BWC::debug start", "BWC::debug ('start')"),
                            _av("stop", "BWC::debug stop", "BWC::debug ('stop')"),
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
                    target=SideEffectTarget.CONNECTION_CONTROL,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
