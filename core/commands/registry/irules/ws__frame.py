# Enriched from F5 iRules reference documentation.
"""WS::frame -- This command allows you to perform various operations on a Websocket frame, determine whether this frame indicates the end of the message, insert a new frame, drop the current frame, or manipulate the frame by prepending, appending or replacing the contents of the frame."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, SubCommand, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/WS__frame.html"


_av = make_av(_SOURCE)

_READ_EFFECT = (
    SideEffect(target=SideEffectTarget.NETWORK_IO, reads=True, connection_side=ConnectionSide.BOTH),
)
_WRITE_EFFECT = (
    SideEffect(
        target=SideEffectTarget.NETWORK_IO,
        reads=True,
        writes=True,
        connection_side=ConnectionSide.BOTH,
    ),
)


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
                    "WS::frame <subcommand> ?args?",
                    "WS::frame insert <type> <payload> ?mask?",
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
                    synopsis="WS::frame <subcommand> ?args?",
                    arg_values={
                        0: (
                            _av("eom", "Check if frame is end of message.", "WS::frame eom"),
                            _av("drop", "Drop the current frame.", "WS::frame drop"),
                            _av(
                                "orig_masked", "Check if frame was masked.", "WS::frame orig_masked"
                            ),
                            _av("type", "Get frame type.", "WS::frame type"),
                            _av("mask", "Get frame mask.", "WS::frame mask"),
                            _av(
                                "insert",
                                "Insert a new frame.",
                                "WS::frame insert <type> <payload> ?mask?",
                            ),
                            _av("prepend", "Prepend data to frame.", "WS::frame prepend <data>"),
                            _av("append", "Append data to frame.", "WS::frame append <data>"),
                            _av("replace", "Replace frame contents.", "WS::frame replace <data>"),
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
                "eom": SubCommand(
                    name="eom",
                    arity=Arity(0, 0),
                    detail="Check if frame is end of message.",
                    synopsis="WS::frame eom",
                    pure=True,
                    side_effect_hints=_READ_EFFECT,
                ),
                "drop": SubCommand(
                    name="drop",
                    arity=Arity(0, 0),
                    detail="Drop the current frame.",
                    synopsis="WS::frame drop",
                    mutator=True,
                    side_effect_hints=_WRITE_EFFECT,
                ),
                "orig_masked": SubCommand(
                    name="orig_masked",
                    arity=Arity(0, 0),
                    detail="Check if frame was masked.",
                    synopsis="WS::frame orig_masked",
                    pure=True,
                    side_effect_hints=_READ_EFFECT,
                ),
                "type": SubCommand(
                    name="type",
                    arity=Arity(0, 0),
                    detail="Get frame type.",
                    synopsis="WS::frame type",
                    pure=True,
                    side_effect_hints=_READ_EFFECT,
                ),
                "mask": SubCommand(
                    name="mask",
                    arity=Arity(0, 0),
                    detail="Get frame mask.",
                    synopsis="WS::frame mask",
                    pure=True,
                    side_effect_hints=_READ_EFFECT,
                ),
                "insert": SubCommand(
                    name="insert",
                    arity=Arity(2, 3),
                    detail="Insert a new frame.",
                    synopsis="WS::frame insert <type> <payload> ?mask?",
                    mutator=True,
                    side_effect_hints=_WRITE_EFFECT,
                ),
                "prepend": SubCommand(
                    name="prepend",
                    arity=Arity(1, 1),
                    detail="Prepend data to frame.",
                    synopsis="WS::frame prepend <data>",
                    mutator=True,
                    side_effect_hints=_WRITE_EFFECT,
                ),
                "append": SubCommand(
                    name="append",
                    arity=Arity(1, 1),
                    detail="Append data to frame.",
                    synopsis="WS::frame append <data>",
                    mutator=True,
                    side_effect_hints=_WRITE_EFFECT,
                ),
                "replace": SubCommand(
                    name="replace",
                    arity=Arity(1, 1),
                    detail="Replace frame contents.",
                    synopsis="WS::frame replace <data>",
                    mutator=True,
                    side_effect_hints=_WRITE_EFFECT,
                ),
            },
        )
