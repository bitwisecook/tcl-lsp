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
"""SCTP::sack_timeout -- Returns the SCTP's delayed selective acknowledgement timeout."""

from __future__ import annotations

from compiler.registry._base import CommandDef, make_av
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/SCTP__sack_timeout.html"


_av = make_av(_SOURCE)


@register
class SctpSackTimeoutCommand(CommandDef):
    name = "SCTP::sack_timeout"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="SCTP::sack_timeout",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the SCTP's delayed selective acknowledgement timeout.",
                synopsis=("SCTP::sack_timeout (clientside | serverside)?",),
                snippet="Returns the SCTP's delayed selective acknowledgement timeout. Can specify the value on clientside or serverside.",
                source=_SOURCE,
                examples=(
                    "when SERVER_CONNECTED {\n"
                    '        log local0.info "SCTP selective acknowledgement timeout value is [SCTP::sack_timeout]"\n'
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="SCTP::sack_timeout (clientside | serverside)?",
                    arg_values={
                        0: (
                            _av(
                                "clientside",
                                "SCTP::sack_timeout clientside",
                                "SCTP::sack_timeout (clientside | serverside)?",
                            ),
                            _av(
                                "serverside",
                                "SCTP::sack_timeout serverside",
                                "SCTP::sack_timeout (clientside | serverside)?",
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
