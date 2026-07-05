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
"""POLICY::names -- Returns details about the policy names for the virtual server the iRule is enabled on."""

from __future__ import annotations

from compiler.registry._base import CommandDef, make_av
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/policy__names.html"


_av = make_av(_SOURCE)


@register
class PolicyNamesCommand(CommandDef):
    name = "POLICY::names"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="POLICY::names",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns details about the policy names for the virtual server the iRule is enabled on.",
                synopsis=("POLICY::names (active | matched | unmatched)",),
                snippet=(
                    "iRule command which returns details about the policy names for the\n"
                    "virtual server the iRule is enabled on."
                ),
                source=_SOURCE,
                examples=(
                    "# Log the policy names for this virtual server\n"
                    "when HTTP_REQUEST {\n"
                    '        log local0. "Enabled on this VS: \\[POLICY::names active\\]: [POLICY::names active]"\n'
                    '        log local0. "Matched: \\[POLICY::names matched\\]: [POLICY::names matched]"\n'
                    '        log local0. "Not matched: \\[POLICY::names unmatched\\]: [POLICY::names unmatched]"\n'
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="POLICY::names (active | matched | unmatched)",
                    arg_values={
                        0: (
                            _av(
                                "active",
                                "POLICY::names active",
                                "POLICY::names (active | matched | unmatched)",
                            ),
                            _av(
                                "matched",
                                "POLICY::names matched",
                                "POLICY::names (active | matched | unmatched)",
                            ),
                            _av(
                                "unmatched",
                                "POLICY::names unmatched",
                                "POLICY::names (active | matched | unmatched)",
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
                    target=SideEffectTarget.BIGIP_CONFIG,
                    reads=True,
                    connection_side=ConnectionSide.GLOBAL,
                ),
            ),
        )
