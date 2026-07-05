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
"""STREAM::encoding -- Specifies non-default content encoding."""

from __future__ import annotations

from compiler.registry._base import CommandDef, make_av
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/STREAM__encoding.html"


_av = make_av(_SOURCE)


@register
class StreamEncodingCommand(CommandDef):
    name = "STREAM::encoding"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="STREAM::encoding",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Specifies non-default content encoding.",
                synopsis=("STREAM::encoding (ascii | utf-8 | unicode)",),
                snippet="Specifies non-default content encoding. The default value is ascii.",
                source=_SOURCE,
                examples=(
                    "when STREAM_MATCHED {\n"
                    "    set stream_match [STREAM::match]\n"
                    '    log local0. "$stream_match"\n'
                    "    STREAM::encoding utf-8\n"
                    "    # The ?/? represents unicode characters.\n"
                    '    if { $stream_match contains "hello?/?" } {\n'
                    '        STREAM::replace "hello hey"\n'
                    '        log local0. "stream match is [STREAM::match]"\n'
                    "    }\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="STREAM::encoding (ascii | utf-8 | unicode)",
                    arg_values={
                        0: (
                            _av(
                                "ascii",
                                "STREAM::encoding ascii",
                                "STREAM::encoding (ascii | utf-8 | unicode)",
                            ),
                            _av(
                                "utf-8",
                                "STREAM::encoding utf-8",
                                "STREAM::encoding (ascii | utf-8 | unicode)",
                            ),
                            _av(
                                "unicode",
                                "STREAM::encoding unicode",
                                "STREAM::encoding (ascii | utf-8 | unicode)",
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
                    target=SideEffectTarget.STREAM_PROFILE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
