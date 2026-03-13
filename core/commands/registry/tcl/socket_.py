"""socket -- Open a TCP network connection."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from ....compiler.types import TclType
from .._base import CommandDef
from ..dialects import DIALECTS_EXCEPT_IRULES
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, OptionSpec, ValidationSpec
from ..signatures import Arity
from ..taint_hints import TaintColour, TaintHint
from ._base import register

_SOURCE = "Tcl socket(3tcl)"


@register
class SocketCommand(CommandDef):
    name = "socket"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="socket",
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
            return_type=TclType.STRING,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.NETWORK_IO,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )

    @classmethod
    def taint_hints(cls) -> TaintHint:
        return TaintHint(source={None: TaintColour.TAINTED})
