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
"""DIAMETER::retransmit -- Triggers the request associated to the current answer message for retransmission."""

from __future__ import annotations

from compiler.registry._base import CommandDef, make_av
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/DIAMETER__retransmit.html"


_av = make_av(_SOURCE)


@register
class DiameterRetransmitCommand(CommandDef):
    name = "DIAMETER::retransmit"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="DIAMETER::retransmit",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Triggers the request associated to the current answer message for retransmission.",
                synopsis=("DIAMETER::retransmit (disabled | busy | unable | retransmit) (NOTE)?",),
                snippet=(
                    "This iRule command triggers the request in the retransmission queue\n"
                    "that is associated with the current answer message for\n"
                    "retransmission. This command will fail the current message is a\n"
                    "request or if there is not an associated request message in the\n"
                    "retransmission queue."
                ),
                source=_SOURCE,
                examples=(
                    "when DIAMETER_EGRESS {\n"
                    "    if { [DIAMETER::is_response] && ![DIAMETER::is_retransmission] } {\n"
                    '        log local0. "reason [DIAMETER::retransmission_reason]"\n'
                    "        DIAMETER::retransmit\n"
                    "    }\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="DIAMETER::retransmit (disabled | busy | unable | retransmit) (NOTE)?",
                    arg_values={
                        0: (
                            _av(
                                "disabled",
                                "DIAMETER::retransmit disabled",
                                "DIAMETER::retransmit (disabled | busy | unable | retransmit) (NOTE)?",
                            ),
                            _av(
                                "busy",
                                "DIAMETER::retransmit busy",
                                "DIAMETER::retransmit (disabled | busy | unable | retransmit) (NOTE)?",
                            ),
                            _av(
                                "unable",
                                "DIAMETER::retransmit unable",
                                "DIAMETER::retransmit (disabled | busy | unable | retransmit) (NOTE)?",
                            ),
                            _av(
                                "retransmit",
                                "DIAMETER::retransmit retransmit",
                                "DIAMETER::retransmit (disabled | busy | unable | retransmit) (NOTE)?",
                            ),
                        )
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"DIAMETER"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.NETWORK_IO,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
