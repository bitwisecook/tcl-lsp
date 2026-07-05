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
"""PSC::policy -- Get/set/remove policies."""

from __future__ import annotations

from compiler.registry._base import CommandDef, make_av
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/PSC__policy.html"


_av = make_av(_SOURCE)


@register
class PscPolicyCommand(CommandDef):
    name = "PSC::policy"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="PSC::policy",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Get/set/remove policies.",
                synopsis=(
                    "PSC::policy (POLICY_NAME)*",
                    "PSC::policy 'add' POLICY_NAME",
                    "PSC::policy 'remove' (POLICY_NAME)?",
                ),
                snippet="The PSC::policy commands get/set/remove the PSC policies.",
                source=_SOURCE,
                return_value="Return the list of PSC policies when no argument is given.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="PSC::policy (POLICY_NAME)*",
                    arg_values={
                        0: (
                            _av("add", "PSC::policy add", "PSC::policy 'add' POLICY_NAME"),
                            _av(
                                "remove",
                                "PSC::policy remove",
                                "PSC::policy 'remove' (POLICY_NAME)?",
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
                    target=SideEffectTarget.NETWORK_IO,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
