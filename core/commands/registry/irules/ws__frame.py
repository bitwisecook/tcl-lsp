# Enriched from F5 iRules reference documentation.
"""WS::frame -- This command allows you to perform various operations on a Websocket frame, determine whether this frame indicates the end of the message, insert a new frame, drop the current frame, or manipulate the frame by prepending, appending or replacing the contents of the frame."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/WS__frame.html"


_av = make_av(_SOURCE)


@register
class WsFrameCommand(CommandDef):
    name = "WS::frame"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="WS::frame",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="This command allows you to perform various operations on a Websocket frame, determine whether this frame indicates the end of the message, insert a new frame, drop the current frame, or manipulate the frame by prepending, appending or replacing the contents of the frame.",
                synopsis=(
                    "WS::frame ( 'eom' | 'drop' | 'orig_masked' | 'type' | 'mask' | ('insert' FRAME_TYPE PAYLOAD (MASK)? ) |",
                ),
                snippet=(
                    "WS::frame eom\n"
                    "    The command can be used to determine whether current frame is last one in the Websocket message.\n"
                    "\n"
                    "WS::frame orig_masked\n"
                    "    The command can be used to determine whether current frame received from the client or server was masked.\n"
                    "\n"
                    "WS::frame type\n"
                    "    The command can be used to determine the type of current frame received from the client or server.\n"
                    "\n"
                    "WS::frame mask\n"
                    "    The command can be used to determine the mask of the current frame.\n"
                    "\n"
                    "WS::frame drop\n"
                    "    The command can be used to drop the current frame."
                ),
                source=_SOURCE,
                examples=(
                    "when WS_SERVER_FRAME {\n"
                    '    log local0. "Websocket frame eom: [WS::frame eom]"\n'
                    '    log local0. "Websocket frame received mask: [WS::frame orig_masked]"\n'
                    '    log local0. "Websocket frame type: [WS::frame type]"\n'
                    '    log local0. "Websocket frame mask: [WS::frame mask]"\n'
                    "    WS::frame drop\n"
                    '    WS::frame insert 1 "abcdefghi"\n'
                    '    WS::frame prepend "Using WS I sent "\n'
                    '    WS::frame append "message was sent"\n'
                    '    WS::frame replace "replaced"\n'
                    "}"
                ),
                return_value="The eom, orig_masked, type and mask commands return the values of corresponding fields in the Websockets frame header. Drop, insert, prepend, append and replace can be used to manipulate the frame contents.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="WS::frame ( 'eom' | 'drop' | 'orig_masked' | 'type' | 'mask' | ('insert' FRAME_TYPE PAYLOAD (MASK)? ) |",
                    arg_values={
                        0: (
                            _av(
                                "eom",
                                "WS::frame eom",
                                "WS::frame ( 'eom' | 'drop' | 'orig_masked' | 'type' | 'mask' | ('insert' FRAME_TYPE PAYLOAD (MASK)? ) |",
                            ),
                            _av(
                                "drop",
                                "WS::frame drop",
                                "WS::frame ( 'eom' | 'drop' | 'orig_masked' | 'type' | 'mask' | ('insert' FRAME_TYPE PAYLOAD (MASK)? ) |",
                            ),
                            _av(
                                "orig_masked",
                                "WS::frame orig_masked",
                                "WS::frame ( 'eom' | 'drop' | 'orig_masked' | 'type' | 'mask' | ('insert' FRAME_TYPE PAYLOAD (MASK)? ) |",
                            ),
                            _av(
                                "type",
                                "WS::frame type",
                                "WS::frame ( 'eom' | 'drop' | 'orig_masked' | 'type' | 'mask' | ('insert' FRAME_TYPE PAYLOAD (MASK)? ) |",
                            ),
                            _av(
                                "mask",
                                "WS::frame mask",
                                "WS::frame ( 'eom' | 'drop' | 'orig_masked' | 'type' | 'mask' | ('insert' FRAME_TYPE PAYLOAD (MASK)? ) |",
                            ),
                            _av(
                                "insert",
                                "WS::frame insert",
                                "WS::frame ( 'eom' | 'drop' | 'orig_masked' | 'type' | 'mask' | ('insert' FRAME_TYPE PAYLOAD (MASK)? ) |",
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
