# Enriched from F5 iRules reference documentation.
"""WS::disconnect -- This command can be used to disconnect a Websocket connection."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/WS__disconnect.html"


@register
class WsDisconnectCommand(CommandDef):
    name = "WS::disconnect"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="WS::disconnect",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="This command can be used to disconnect a Websocket connection.",
                synopsis=("WS::disconnect ( CODE (RSN)? )",),
                snippet=(
                    "WS::disconnect <close-reason> <reason>\n"
                    "    The Websocket connection is disconnected by sending a close frame to both end-points when the current frame is done. The specified code and reason will be sent in the header and payload of the frame respectively."
                ),
                source=_SOURCE,
                examples=(
                    'when WS_CLIENT_FRAME_DONE {\n    WS::disconnect 1000 "some random reason"\n}'
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="WS::disconnect ( CODE (RSN)? )",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"HTTP"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.CONNECTION_CONTROL,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
