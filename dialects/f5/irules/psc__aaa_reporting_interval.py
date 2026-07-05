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
"""PSC::aaa_reporting_interval -- Get or set AAA reporting interval."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/PSC__aaa_reporting_interval.html"


@register
class PscAaaReportingIntervalCommand(CommandDef):
    name = "PSC::aaa_reporting_interval"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="PSC::aaa_reporting_interval",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Get or set AAA reporting interval.",
                synopsis=("PSC::aaa_reporting_interval (INTERVAL)?",),
                snippet="The PSC::aaa_reporting_interval command gets the AAA reporting interval or sets the AAA reporting interval when the optional value is given.",
                source=_SOURCE,
                return_value="Return AAA reporting interval when no argument is given.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="PSC::aaa_reporting_interval (INTERVAL)?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.NETWORK_IO,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
