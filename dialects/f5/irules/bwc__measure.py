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
"""BWC::measure -- This command allows you to measure rate for a particular traffic flow or flows belonging to the bwc instance."""

from __future__ import annotations

from compiler.registry._base import CommandDef, make_av
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/BWC__measure.html"


_av = make_av(_SOURCE)


@register
class BwcMeasureCommand(CommandDef):
    name = "BWC::measure"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="BWC::measure",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="This command allows you to measure rate for a particular traffic flow or flows belonging to the bwc instance.",
                synopsis=("BWC::measure ( ('start' | 'stop') |",),
                snippet="After a flow has been assigned a policy, user can start or stop measurement on a per policy basis or on a per flow basis. Once the measurement is started the measured bandwidth can be read by the user using 'BWC::measure get ..' iRules. Optionally users can direct the bandwidth measurement results to a 'log publisher' configured on the BIGIP system. Based on the log_publisher setting the measurement results will be logged to the log server indicated in the 'log_publisher'. It is usually an external high speed log server.",
                source=_SOURCE,
                examples=("when SERVER_CONNECTED {\n        TCP::collect     set count 0\n    }"),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="BWC::measure ( ('start' | 'stop') |",
                    arg_values={
                        0: (
                            _av(
                                "start", "BWC::measure start", "BWC::measure ( ('start' | 'stop') |"
                            ),
                            _av("stop", "BWC::measure stop", "BWC::measure ( ('start' | 'stop') |"),
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
