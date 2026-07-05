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
"""DNSMSG::section -- Returns a section of a dns_message."""

from __future__ import annotations

from compiler.registry._base import CommandDef, make_av
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.registry.taint_hints import TaintColour, TaintHint
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/DNSMSG-section.html"


_av = make_av(_SOURCE)


@register
class DnsmsgSectionCommand(CommandDef):
    name = "DNSMSG::section"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="DNSMSG::section",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns a section of a dns_message.",
                synopsis=(
                    "DNSMSG::section DNS_MESSAGE ('question' | 'answer' | 'authority' | 'additional' )",
                ),
                snippet="This iRule gets the specified section of a dns_message.",
                source=_SOURCE,
                examples=(
                    "when CLIENT_ACCEPTED {\n"
                    '        set result [RESOLVER::name_lookup "/Common/r1" www.abc.com a]\n'
                    "        set answer [DNSMSG::section $result answer]\n"
                    "}"
                ),
                return_value="Returns a TCL list of resource records from the specified section.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="DNSMSG::section DNS_MESSAGE ('question' | 'answer' | 'authority' | 'additional' )",
                    arg_values={
                        0: (
                            _av(
                                "question",
                                "DNSMSG::section question",
                                "DNSMSG::section DNS_MESSAGE ('question' | 'answer' | 'authority' | 'additional' )",
                            ),
                            _av(
                                "answer",
                                "DNSMSG::section answer",
                                "DNSMSG::section DNS_MESSAGE ('question' | 'answer' | 'authority' | 'additional' )",
                            ),
                            _av(
                                "authority",
                                "DNSMSG::section authority",
                                "DNSMSG::section DNS_MESSAGE ('question' | 'answer' | 'authority' | 'additional' )",
                            ),
                            _av(
                                "additional",
                                "DNSMSG::section additional",
                                "DNSMSG::section DNS_MESSAGE ('question' | 'answer' | 'authority' | 'additional' )",
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
                    target=SideEffectTarget.DNS_STATE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )

    @classmethod
    def taint_hints(cls) -> TaintHint:
        return TaintHint(source={None: TaintColour.TAINTED})
