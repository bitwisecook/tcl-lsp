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
"""SCTP::rto_min -- Returns the minimum value of SCTP retransmission timeout."""

from __future__ import annotations

from compiler.registry._base import CommandDef, make_av
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/SCTP__rto_min.html"


_av = make_av(_SOURCE)


@register
class SctpRtoMinCommand(CommandDef):
    name = "SCTP::rto_min"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="SCTP::rto_min",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the minimum value of SCTP retransmission timeout.",
                synopsis=("SCTP::rto_min (clientside | serverside)?",),
                snippet="Returns the minimum value of SCTP retranmission timeout. Can specify the value on clientside or serverside.",
                source=_SOURCE,
                examples=(
                    "when SERVER_CONNECTED {\n"
                    '        log local0.info "SCTP retransmission timeout minimum value is [SCTP::rto_min]"\n'
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="SCTP::rto_min (clientside | serverside)?",
                    arg_values={
                        0: (
                            _av(
                                "clientside",
                                "SCTP::rto_min clientside",
                                "SCTP::rto_min (clientside | serverside)?",
                            ),
                            _av(
                                "serverside",
                                "SCTP::rto_min serverside",
                                "SCTP::rto_min (clientside | serverside)?",
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
                    target=SideEffectTarget.TCP_STATE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
