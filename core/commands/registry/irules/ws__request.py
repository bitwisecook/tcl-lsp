# Enriched from F5 iRules reference documentation.
"""WS::request -- This command returns the values of the various Websocket header fields seen in a client request."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ..taint_hints import TaintColour, TaintHint
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/WS__request.html"


_av = make_av(_SOURCE)


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
                    synopsis="WS::request ('protocol' | 'extension' | 'version' | 'key' )",
                    arg_values={
                        0: (
                            _av(
                                "protocol",
                                "WS::request protocol",
                                "WS::request ('protocol' | 'extension' | 'version' | 'key' )",
                            ),
                            _av(
                                "extension",
                                "WS::request extension",
                                "WS::request ('protocol' | 'extension' | 'version' | 'key' )",
                            ),
                            _av(
                                "version",
                                "WS::request version",
                                "WS::request ('protocol' | 'extension' | 'version' | 'key' )",
                            ),
                            _av(
                                "key",
                                "WS::request key",
                                "WS::request ('protocol' | 'extension' | 'version' | 'key' )",
                            ),
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
        )

    @classmethod
    def taint_hints(cls) -> TaintHint:
        return TaintHint(source={None: TaintColour.TAINTED})
