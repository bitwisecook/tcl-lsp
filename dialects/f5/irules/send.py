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
"""send -- Sends data on an existing sideband connection."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import (
    CommandSpec,
    FormKind,
    FormSpec,
    HoverSnippet,
    OptionSpec,
    ValidationSpec,
)
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/send.html"


@register
class SendCommand(CommandDef):
    name = "send"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="send",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Sends data on an existing sideband connection.",
                synopsis=("send ((",),
                snippet=(
                    "This command sends data on an existing sideband connection (established with connect). It is one of several commands that make up the ability to create sideband connections from iRules.\n"
                    "\n"
                    "Arguments\n"
                    "\n"
                    "    <connection> is the connection identifier returned from connect\n"
                    "\n"
                    "    <data> is the data to send\n"
                    "\n"
                    "    -timeout ms specifies the amount of time to wait for the data to be sent. The default is an immediate timeout.\n"
                    "\n"
                    "    -status varname will save the result of the send command into varname. The possible status values are:\n"
                    "        1. sent - the data was sent successfully\n"
                    "        2."
                ),
                source=_SOURCE,
                examples=(
                    "when LB_SELECTED {\n"
                    "    # Save some data to send\n"
                    '    set dest "10.0.16.1:8888"\n'
                    '    set data "GET /mypage/myindex2.html HTTP/1.0\\r\\n\\r\\n"\n'
                    "\n"
                    "    # Open a new TCP connection to $dest\n"
                    "    set conn_id [connect -protocol TCP -timeout 30000 -idle 30 $dest]\n"
                    "\n"
                    "    # Send the data with a 1000ms timeout on the connection identifier received from the connect command\n"
                    "    set send_bytes [send -timeout 1000 -status send_status $conn_id $data]\n"
                    "\n"
                    "    # Log the number of bytes sent and the send status"
                ),
                return_value="Sends data on a specified sideband connection, and returns an integer representing the amount of data that was sent.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="send ?options? ?--? connection data",
                    options=(
                        OptionSpec(
                            name="-timeout",
                            detail="Time in ms to wait for data to be sent.",
                            takes_value=True,
                            value_hint="MSEC",
                        ),
                        OptionSpec(
                            name="-status",
                            detail="Save send status into variable.",
                            takes_value=True,
                            value_hint="VARIABLE",
                        ),
                        OptionSpec(name="--"),
                    ),
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.NETWORK_IO,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
