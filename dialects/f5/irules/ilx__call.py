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
"""ILX::call -- Calls an ILX method."""

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

_SOURCE = "https://clouddocs.f5.com/api/irules/ILX__call.html"


@register
class IlxCallCommand(CommandDef):
    name = "ILX::call"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ILX::call",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Calls an ILX method.",
                synopsis=("ILX::call HANDLE ?-timeout ms? ?--? METHOD ?args ...?",),
                snippet="Make a call to a method defined within the plugin extension referenced by the handle.  Provide the method with the arguments listed in ARGS, do not continue processing the iRule until a response is received.",
                source=_SOURCE,
                examples=(
                    "when CLIENT_ACCEPTED {\n"
                    "    # Get a handle to the running extension instance to call into.\n"
                    "    set RPC_HANDLE [ILX::init my_plugin my_extension]\n"
                    "    # Make the call and store the response in $rpc_response\n"
                    "    set rpc_response [ILX::call $RPC_HANDLE my_js_function arg1 arg2]\n"
                    "}"
                ),
                return_value="The return value is the argument passed to response.reply() call on the extension side (eg. an array, a string, etc).",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ILX::call HANDLE ?-timeout ms? ?--? METHOD ?args ...?",
                    options=(
                        OptionSpec(
                            name="-timeout",
                            detail="Timeout in milliseconds.",
                            takes_value=True,
                            value_hint="MSEC",
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
