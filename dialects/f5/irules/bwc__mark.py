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
"""BWC::mark -- This command allows you to set or unset marking for traffic flows in bwc when configured rate limit is exceeded."""

from __future__ import annotations

from compiler.registry._base import CommandDef, make_av
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/BWC__mark.html"


_av = make_av(_SOURCE)


@register
class BwcMarkCommand(CommandDef):
    name = "BWC::mark"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="BWC::mark",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="This command allows you to set or unset marking for traffic flows in bwc when configured rate limit is exceeded.",
                synopsis=(
                    "BWC::mark SESSION_ID ('qos'|'tos') (BWC_VALUE | 'passthrough')",
                    "BWC::mark SESSION_ID APP_NAME ('qos'|'tos') (BWC_VALUE | 'passthrough')",
                ),
                snippet="This command allows you to set or unset marking for traffic flows in bwc when configured rate limit is exceeded. Marking can be on DSCP (ToS - L3) and/or QoS (L2). The ToS/QoS value needs to be in valid range and can be passthrough.",
                source=_SOURCE,
                examples=(
                    "when CLIENT_ACCEPTED {\n"
                    "    set mycookie [IP::remote_addr]:[TCP::remote_port]\n"
                    "    BWC::policy attach gold_user $mycookie\n"
                    "    BWC::color set gold_user p2p\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="BWC::mark SESSION_ID ('qos'|'tos') (BWC_VALUE | 'passthrough')",
                    arg_values={
                        0: (
                            _av(
                                "qos",
                                "BWC::mark qos",
                                "BWC::mark SESSION_ID ('qos'|'tos') (BWC_VALUE | 'passthrough')",
                            ),
                            _av(
                                "tos",
                                "BWC::mark tos",
                                "BWC::mark SESSION_ID ('qos'|'tos') (BWC_VALUE | 'passthrough')",
                            ),
                            _av(
                                "passthrough",
                                "BWC::mark passthrough",
                                "BWC::mark SESSION_ID ('qos'|'tos') (BWC_VALUE | 'passthrough')",
                            ),
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
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
