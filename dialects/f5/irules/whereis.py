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
"""whereis -- Returns geographical information on an IP address."""

from __future__ import annotations

from compiler.registry._base import CommandDef, make_av
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/whereis.html"


_av = make_av(_SOURCE)


@register
class WhereisCommand(CommandDef):
    name = "whereis"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="whereis",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns geographical information on an IP address.",
                synopsis=("whereis (ldns | IP_ADDR)",),
                snippet=(
                    "Returns the geographic location of a specific IP address.\n"
                    "For more information on using whereis in LTM, you can check Jason\n"
                    "Rahm's article\n"
                    "\n"
                    "Legal usage notes\n"
                    "\n"
                    "   The data is purchased by F5 for use on BIG-IP systems and products for\n"
                    "   traffic management. The key to understanding EULA compliance is to\n"
                    "   figure out where the geolocation decision is being made."
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="whereis (ldns | IP_ADDR)",
                    arg_values={0: (_av("ldns", "whereis ldns", "whereis (ldns | IP_ADDR)"),)},
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.BIGIP_CONFIG,
                    reads=True,
                    connection_side=ConnectionSide.GLOBAL,
                ),
            ),
        )
