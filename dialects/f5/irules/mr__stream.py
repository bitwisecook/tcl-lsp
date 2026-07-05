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
"""MR::stream -- Start egressing bytes previously collected and stored."""

from __future__ import annotations

from compiler.registry._base import CommandDef, make_av
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/MR__stream.html"


_av = make_av(_SOURCE)


@register
class MrStreamCommand(CommandDef):
    name = "MR::stream"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="MR::stream",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Start egressing bytes previously collected and stored.",
                synopsis=("MR::stream ( 'end' )? (BYTES)",),
                snippet=(
                    "Start egressing bytes previously collected and stored say in sessionDB. If payload has been split in multiple segments, use end to indicate the final segment.\n"
                    "\n"
                    "SYNTAX\n"
                    "\n"
                    "MR::stream <payload>\n"
                    "    Stream payload segment.\n"
                    "\n"
                    "MR::stream end <payload>\n"
                    "    Stream payload segement. End indicates final segment."
                ),
                source=_SOURCE,
                examples=('when MR_EGRESS {\n    MR::stream end "abcd"\n}'),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="MR::stream ( 'end' )? (BYTES)",
                    arg_values={
                        0: (_av("end", "MR::stream end", "MR::stream ( 'end' )? (BYTES)"),)
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"MR"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.MESSAGE_STATE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
