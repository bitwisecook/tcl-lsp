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
"""ICAP::header -- Sets or returns ICAP attributes in the ICAP header."""

from __future__ import annotations

from compiler.registry._base import CommandDef, make_av
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.registry.taint_hints import TaintColour, TaintHint
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ICAP__header.html"


_av = make_av(_SOURCE)


@register
class IcapHeaderCommand(CommandDef):
    name = "ICAP::header"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ICAP::header",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Sets or returns ICAP attributes in the ICAP header.",
                synopsis=(
                    "ICAP::header 'names'",
                    "ICAP::header 'at' HEADER_INDEX",
                    "ICAP::header 'count' (HEADER_NAME)?",
                    "ICAP::header 'exists' HEADER_NAME",
                ),
                snippet="The ICAP::header command sets or returns attributes in the ICAP header.",
                source=_SOURCE,
                examples=(
                    "when ICAP_RESPONSE {\n"
                    "                ICAP::header remove X-ICAP-my-custom-header\n"
                    "            }"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ICAP::header 'names'",
                    arg_values={
                        0: (
                            _av("names", "ICAP::header names", "ICAP::header 'names'"),
                            _av("at", "ICAP::header at", "ICAP::header 'at' HEADER_INDEX"),
                            _av(
                                "count", "ICAP::header count", "ICAP::header 'count' (HEADER_NAME)?"
                            ),
                            _av(
                                "exists", "ICAP::header exists", "ICAP::header 'exists' HEADER_NAME"
                            ),
                        )
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"ICAP"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.ICAP_STATE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )

    @classmethod
    def taint_hints(cls) -> TaintHint:
        return TaintHint(source={None: TaintColour.TAINTED})
