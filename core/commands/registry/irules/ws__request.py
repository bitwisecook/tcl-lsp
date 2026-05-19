# Enriched from F5 iRules reference documentation.
"""WS::request -- This command returns the values of the various Websocket header fields seen in a client request."""

# Introduced: BIG-IP v14+ (WebSocket) (approximate, from F5 documentation)

from __future__ import annotations

from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, SubCommand, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ..taint_hints import TaintColour, TaintHint
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/WS__request.html"


_av = make_av(_SOURCE)

_READ_EFFECT = (
    SideEffect(target=SideEffectTarget.NETWORK_IO, reads=True, connection_side=ConnectionSide.BOTH),
)

_FIELDS = {
    "protocol": "Get Sec-WebSocket-Protocol header value.",
    "extension": "Get Sec-WebSocket-Extensions header value.",
    "version": "Get Sec-WebSocket-Version header value.",
    "key": "Get Sec-WebSocket-Key header value.",
}


@register
class WsRequestCommand(CommandDef):
    name = "WS::request"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="WS::request",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="This command returns the values of the various Websocket header fields seen in a client request.",
                synopsis=("WS::request ('protocol' | 'extension' | 'version' | 'key' )",),
                snippet=(
                    "WS::request protocol\n"
                    "    Returns the value of Sec-WebSocket-Protocol header field in client request.\n"
                    "\n"
                    "WS::request extension\n"
                    "    Returns the value of Sec-WebSocket-Extensions header field in client request.\n"
                    "\n"
                    "WS::request version\n"
                    "    Returns the value of Sec-WebSocket-Version header field in client request.\n"
                    "\n"
                    "WS::request key\n"
                    "    Returns the value of Sec-WebSocket-Key header field in client request."
                ),
                source=_SOURCE,
                examples=(
                    "when WS_REQUEST {\n"
                    '    if { [WS::request protocol] equals "chat" } {\n'
                    "        WS::enabled false\n"
                    "    }\n"
                    "}"
                ),
                return_value="This command can be used to lookup the values of various Websocket header fields seen in a client request.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="WS::request <field>",
                    arg_values={
                        0: tuple(
                            _av(field, detail, f"WS::request {field}")
                            for field, detail in _FIELDS.items()
                        )
                    },
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
            subcommands={
                field: SubCommand(
                    name=field,
                    arity=Arity(0, 0),
                    detail=detail,
                    synopsis=f"WS::request {field}",
                    pure=True,
                    side_effect_hints=_READ_EFFECT,
                )
                for field, detail in _FIELDS.items()
            },
        )

    @classmethod
    def taint_hints(cls) -> TaintHint:
        return TaintHint(source={None: TaintColour.TAINTED})
