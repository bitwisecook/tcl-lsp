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

"""socket -- Open a TCP network connection."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.dialects import DIALECTS_EXCEPT_IRULES
from compiler.registry.models import (
    CommandSpec,
    FormKind,
    FormSpec,
    HoverSnippet,
    OptionSpec,
    ValidationSpec,
    WasmRuntimeImport,
)
from compiler.registry.signatures import Arity
from compiler.registry.taint_hints import TaintColour, TaintHint
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from compiler.types import TclType

from ._base import register

_SOURCE = "Tcl socket(3tcl)"


@register
class SocketCommand(CommandDef):
    name = "socket"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="socket",
            byte_compiled=True,
            dialects=DIALECTS_EXCEPT_IRULES,
            hover=HoverSnippet(
                summary="Open a TCP client or server socket channel.",
                synopsis=(
                    "socket ?options? host port",
                    "socket -server command ?options? port",
                ),
                snippet=(
                    "`-server` creates a listening socket and invokes the callback "
                    "with `channel clientaddr clientport` for each accepted connection."
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="socket ?options? host port",
                    options=(
                        OptionSpec(
                            name="-myaddr",
                            takes_value=True,
                            value_hint="addr",
                            detail="Client-side local interface.",
                            hover=HoverSnippet(
                                summary="Choose the client-side local address/interface.",
                                synopsis=("socket -myaddr addr host port",),
                                snippet="Useful on multi-homed hosts when binding to a specific interface.",
                                source=_SOURCE,
                            ),
                        ),
                        OptionSpec(
                            name="-myport",
                            takes_value=True,
                            value_hint="port",
                            detail="Client-side local port.",
                            hover=HoverSnippet(
                                summary="Choose the client-side local port.",
                                synopsis=("socket -myport port host port",),
                                snippet="If omitted, the OS picks an ephemeral local port.",
                                source=_SOURCE,
                            ),
                        ),
                        OptionSpec(
                            name="-async",
                            detail="Connect asynchronously.",
                            hover=HoverSnippet(
                                summary="Create the socket immediately and complete connect asynchronously.",
                                synopsis=("socket -async host port",),
                                snippet="Use `fconfigure $chan -error` to inspect connection errors later.",
                                source=_SOURCE,
                            ),
                        ),
                    ),
                ),
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="socket -server command ?options? port",
                    options=(
                        OptionSpec(
                            name="-server",
                            takes_value=True,
                            value_hint="command",
                            detail="Server accept callback.",
                            hover=HoverSnippet(
                                summary="Enable server mode and provide an accept callback command.",
                                synopsis=("socket -server command ?options? port",),
                                snippet=(
                                    "The callback receives three appended args: "
                                    "channel, client address, and client port."
                                ),
                                source=_SOURCE,
                            ),
                        ),
                        # -reuseaddr / -reuseport: Tcl 9.0+ (rejected as
                        # "bad option" in 8.x where SO_REUSEADDR is
                        # always on for server sockets and not exposed).
                        OptionSpec(
                            name="-reuseaddr",
                            takes_value=True,
                            value_hint="boolean",
                            detail="Allow server address reuse, default 1 (Tcl 9.0+).",
                            dialects=frozenset({"tcl9.0"}),
                        ),
                        OptionSpec(
                            name="-reuseport",
                            takes_value=True,
                            value_hint="boolean",
                            detail="Allow server port reuse, default 0 (Tcl 9.0+).",
                            dialects=frozenset({"tcl9.0"}),
                        ),
                        OptionSpec(
                            name="-myaddr",
                            takes_value=True,
                            value_hint="addr",
                            detail="Server-side bind address.",
                            hover=HoverSnippet(
                                summary="Bind server socket to a specific local interface.",
                                synopsis=("socket -server cb -myaddr addr port",),
                                snippet="If omitted, Tcl binds to INADDR_ANY and accepts from all interfaces.",
                                source=_SOURCE,
                            ),
                        ),
                    ),
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(2),
            ),
            taint_network_sink_args=(0, 1),
            return_type=TclType.CHANNEL,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.NETWORK_IO,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
            wasm_runtime_import=WasmRuntimeImport(
                import_key="tcl_socket",
                argc=2,
                params=("i32", "i32"),
                results=("i32",),
            ),
        )

    @classmethod
    def taint_hints(cls) -> TaintHint:
        return TaintHint(source={None: TaintColour.TAINTED})
