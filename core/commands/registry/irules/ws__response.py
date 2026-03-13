# Enriched from F5 iRules reference documentation.
"""WS::response -- This command returns the values of the various Websocket header fields seen in a server response."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/WS__response.html"


_av = make_av(_SOURCE)


@register
class WsResponseCommand(CommandDef):
    name = "WS::response"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="WS::response",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="This command returns the values of the various Websocket header fields seen in a server response.",
                synopsis=(
                    "WS::response ('protocol' | 'extension' | 'version' | 'key' | 'valid' )",
                ),
                snippet=(
                    "WS::response protocol\n"
                    "    Returns the value of Sec-WebSocket-Protocol header field in server response.\n"
                    "\n"
                    "WS::response extension\n"
                    "    Returns the value of Sec-WebSocket-Extensions header field in server response.\n"
                    "\n"
                    "WS::response version\n"
                    "    Returns the value of Sec-WebSocket-Version header field in server response.\n"
                    "\n"
                    "WS::response key\n"
                    "    Returns the value of Sec-WebSocket-Accept header field in server response.\n"
                    "\n"
                    "WS::response valid\n"
                    "    Returns whether the client request and server response resulted in a successful Websocket upgrade."
                ),
                source=_SOURCE,
                examples=(
                    "when WS_RESPONSE {\n"
                    '    if { [WS::response key] equals "s3pPLMBiTxaQ9kYGzzhZRbK+xOo="} {\n'
                    "        WS::enabled false\n"
                    "    }\n"
                    "}"
                ),
                return_value="This command can be used to lookup the values of various Websocket header fields seen in a server response.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="WS::response ('protocol' | 'extension' | 'version' | 'key' | 'valid' )",
                    arg_values={
                        0: (
                            _av(
                                "protocol",
                                "WS::response protocol",
                                "WS::response ('protocol' | 'extension' | 'version' | 'key' | 'valid' )",
                            ),
                            _av(
                                "extension",
                                "WS::response extension",
                                "WS::response ('protocol' | 'extension' | 'version' | 'key' | 'valid' )",
                            ),
                            _av(
                                "version",
                                "WS::response version",
                                "WS::response ('protocol' | 'extension' | 'version' | 'key' | 'valid' )",
                            ),
                            _av(
                                "key",
                                "WS::response key",
                                "WS::response ('protocol' | 'extension' | 'version' | 'key' | 'valid' )",
                            ),
                            _av(
                                "valid",
                                "WS::response valid",
                                "WS::response ('protocol' | 'extension' | 'version' | 'key' | 'valid' )",
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
