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
"""LB::snat -- Returns information on the SNAT configuration for the current connection."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/LB__snat.html"


@register
class LbSnatCommand(CommandDef):
    name = "LB::snat"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="LB::snat",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns information on the SNAT configuration for the current connection.",
                synopsis=("LB::snat",),
                snippet=(
                    "This command returns information on the SNAT configuration for the current connection.\n"
                    "\n"
                    "Possible output values are those which can be set by the snat and snatpool commands."
                ),
                source=_SOURCE,
                examples=(
                    "when CLIENT_ACCEPTED {\n"
                    "    # Check if SNAT is enabled on the VIP\n"
                    '    if {[LB::snat] eq "none"}{\n'
                    '        log local0. "Snat disabled on [virtual name]"\n'
                    "    } else {\n"
                    '        log local0. "Snat enabled on [virtual name].  Currently set to [LB::snat]"\n'
                    "    }\n"
                    "}"
                ),
                return_value="LB::snat",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="LB::snat",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.SNAT_SELECTION,
                    reads=True,
                    writes=True,
                    connection_side=ConnectionSide.SERVER,
                ),
            ),
        )
