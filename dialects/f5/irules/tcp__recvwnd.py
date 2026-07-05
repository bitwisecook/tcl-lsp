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
"""TCP::recvwnd -- This command can be used to set/get the receive window size of a TCP connection."""

from __future__ import annotations

from compiler.registry._base import CommandDef, make_av
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/TCP__recvwnd.html"


_av = make_av(_SOURCE)


@register
class TcpRecvwndCommand(CommandDef):
    name = "TCP::recvwnd"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="TCP::recvwnd",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="This command can be used to set/get the receive window size of a TCP connection.",
                synopsis=("TCP::recvwnd ('auto' | WINDOW_SIZE)?",),
                snippet=(
                    "TCP::recvwnd returns the receive window size of a TCP connection.\n"
                    "TCP::recvwnd WINDOW_SIZE sets the receive window to WINDOW_SIZE bytes."
                ),
                source=_SOURCE,
                examples=(
                    "t the receive window size of the TCP flow.\n"
                    "    when CLIENT_ACCEPTED {\n"
                    '        log local0. "TCP set receive window: [TCP::recvwnd 100000]"\n'
                    '        log local0. "TCP get receive window: [TCP::recvwnd]"\n'
                    "    }"
                ),
                return_value="TCP::recvwnd returns the number of bytes that can be stored at the receive window.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="TCP::recvwnd ('auto' | WINDOW_SIZE)?",
                    arg_values={
                        0: (
                            _av(
                                "auto", "TCP::recvwnd auto", "TCP::recvwnd ('auto' | WINDOW_SIZE)?"
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
