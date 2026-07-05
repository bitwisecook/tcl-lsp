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
"""ACCESS::restrict_irule_events -- Enable or disable HTTP and higher layer iRule events for the internal APM access control URIs."""

from __future__ import annotations

from compiler.registry._base import CommandDef, make_av
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ACCESS__restrict_irule_events.html"


_av = make_av(_SOURCE)


@register
class AccessRestrictIruleEventsCommand(CommandDef):
    name = "ACCESS::restrict_irule_events"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ACCESS::restrict_irule_events",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Enable or disable HTTP and higher layer iRule events for the internal APM access control URIs.",
                synopsis=("ACCESS::restrict_irule_events (enable | disable)",),
                snippet=(
                    "During access policy execution, ACCESS creates requests to various URIs\n"
                    "related to various access policy processing. These includes /my.policy\n"
                    "and other pages (logon, message box etc.) shown to the end user. By\n"
                    "default from 11.0.0 onward, HTTP and higher layer iRule events are not\n"
                    "raised for the internal access control URIs. All events except\n"
                    "ACCESS_SESSION_STARTED, ACCESS_SESSION_CLOSED,\n"
                    "ACCESS_POLICY_AGENT_EVENT, ACCESS_POLICY_COMPLETED are blocked (not\n"
                    "raised) for internal access control URI.\n"
                    "This command allows admin to overwrite the default behavior."
                ),
                source=_SOURCE,
                examples=("when CLIENT_ACCEPTED {\n    ACCESS::restrict_irule_events disable\n}"),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ACCESS::restrict_irule_events (enable | disable)",
                    arg_values={
                        0: (
                            _av(
                                "enable",
                                "ACCESS::restrict_irule_events enable",
                                "ACCESS::restrict_irule_events (enable | disable)",
                            ),
                            _av(
                                "disable",
                                "ACCESS::restrict_irule_events disable",
                                "ACCESS::restrict_irule_events (enable | disable)",
                            ),
                        )
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(also_in=frozenset({"CLIENT_ACCEPTED"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.APM_STATE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
