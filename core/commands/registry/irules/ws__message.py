# Enriched from F5 iRules reference documentation.
"""WS::message -- This command can be used to drop an entire Websocket message."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ..taint_hints import TaintColour, TaintHint
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/WS__message.html"


_av = make_av(_SOURCE)


@register
class WsMessageCommand(CommandDef):
    name = "WS::message"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="WS::message",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="This command can be used to drop an entire Websocket message.",
                synopsis=("WS::message ( 'drop' )",),
                snippet=("WS::message drop\n    Drop an entire Websocket message."),
                source=_SOURCE,
                examples=("when WS_CLIENT_FRAME {\n    WS::message drop\n}"),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="WS::message ( 'drop' )",
                    arg_values={0: (_av("drop", "WS::message drop", "WS::message ( 'drop' )"),)},
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"HTTP"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.NETWORK_IO,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )

    @classmethod
    def taint_hints(cls) -> TaintHint:
        return TaintHint(source={None: TaintColour.TAINTED})
