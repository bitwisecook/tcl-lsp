"""chan -- Read, write and manipulate channels."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from ....compiler.types import TclType
from .._base import CommandDef
from ..models import (
    ArgumentValueSpec,
    CommandSpec,
    FormKind,
    FormSpec,
    HoverSnippet,
    OptionSpec,
    SubCommand,
    ValidationSpec,
)
from ..signatures import ArgRole, Arity
from ..taint_hints import TaintColour, TaintHint
from ._base import register

_SOURCE = "Tcl man page chan.n"


def _av(value: str, detail: str, synopsis: str = "") -> ArgumentValueSpec:
    hover = HoverSnippet(
        summary=detail,
        synopsis=(synopsis,) if synopsis else (),
        source=_SOURCE,
    )
    return ArgumentValueSpec(value=value, detail=detail, hover=hover)


_SUBCOMMANDS = (
    _av(
        "blocked",
        "Test whether last input operation exhausted all available data.",
        "chan blocked channelId",
    ),
    _av("close", "Close a channel.", "chan close channelId ?direction?"),
    _av(
        "configure",
        "Query or set channel options.",
        "chan configure channelId ?optionName? ?value ...?",
    ),
    _av(
        "copy",
        "Copy data from one channel to another.",
        "chan copy inputChan outputChan ?-size size? ?-command callback?",
    ),
    _av("create", "Create a script-level channel.", "chan create mode cmdPrefix"),
    _av("eof", "Test for end of file on a channel.", "chan eof channelId"),
    _av("event", "Set up event handler for channel.", "chan event channelId event ?script?"),
    _av("flush", "Flush buffered output for a channel.", "chan flush channelId"),
    _av("gets", "Read a line from a channel.", "chan gets channelId ?varName?"),
    _av("isbinary", "Test whether channel is binary.", "chan isbinary channelId"),
    _av("names", "Return list of open channels.", "chan names ?pattern?"),
    _av("pending", "Return number of bytes pending.", "chan pending mode channelId"),
    _av("pipe", "Create a pair of connected channels.", "chan pipe"),
    _av("pop", "Remove topmost stacked transformation.", "chan pop channelId"),
    _av("postevent", "Post an event to a reflected channel.", "chan postevent channelId eventSpec"),
    _av("push", "Push a transformation on top of a channel.", "chan push channelId cmdPrefix"),
    _av("puts", "Write a string to a channel.", "chan puts ?-nonewline? channelId string"),
    _av("read", "Read data from a channel.", "chan read ?-nonewline? channelId ?numChars?"),
    _av("seek", "Set access position for a channel.", "chan seek channelId offset ?origin?"),
    _av("tell", "Return current access position.", "chan tell channelId"),
    _av("truncate", "Truncate a channel to given length.", "chan truncate channelId ?length?"),
)

_TRANSLATION_VALUES = (
    _av("auto", "Auto-detect line endings on input; platform-specific on output."),
    _av("binary", "No translation. Equivalent to lf with iso8859-1 encoding."),
    _av("cr", "Carriage return line ending."),
    _av("crlf", "Carriage return + linefeed line ending."),
    _av("lf", "Linefeed only, no end-of-line translation."),
)

_BUFFERING_VALUES = (
    _av("full", "Buffer until full or flush called."),
    _av("line", "Flush after each newline."),
    _av("none", "Flush after every output operation."),
)

_CONFIGURE_OPTIONS = (
    OptionSpec(
        name="-blocking", takes_value=True, value_hint="boolean", detail="Set blocking mode."
    ),
    OptionSpec(
        name="-buffering",
        takes_value=True,
        value_hint="mode",
        detail="Set buffering mode (full, line, none).",
    ),
    OptionSpec(
        name="-buffersize", takes_value=True, value_hint="size", detail="Set buffer size in bytes."
    ),
    OptionSpec(
        name="-encoding", takes_value=True, value_hint="encoding", detail="Set character encoding."
    ),
    OptionSpec(
        name="-eofchar",
        takes_value=True,
        value_hint="chars",
        detail="Set end-of-file character(s).",
    ),
    OptionSpec(
        name="-profile",
        takes_value=True,
        value_hint="profile",
        detail="Set encoding profile (strict, tcl8, replace).",
    ),
    OptionSpec(
        name="-translation",
        takes_value=True,
        value_hint="mode",
        detail="Set line-ending translation mode.",
    ),
)


@register
class ChanCommand(CommandDef):
    name = "chan"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="chan",
            hover=HoverSnippet(
                summary="Read, write and manipulate channels.",
                synopsis=("chan subcommand ?arg ...?",),
                snippet="Unified interface for channel operations (`configure`, `gets`, `puts`, `read`, etc.).",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="chan subcommand ?arg ...?",
                    options=_CONFIGURE_OPTIONS,
                    arg_values={0: _SUBCOMMANDS},
                ),
            ),
            subcommands={
                "blocked": SubCommand(
                    name="blocked",
                    arity=Arity(1, 1),
                    detail="Test whether last input operation exhausted all available data.",
                    synopsis="chan blocked channelId",
                    return_type=TclType.BOOLEAN,
                ),
                "close": SubCommand(
                    name="close",
                    arity=Arity(1, 2),
                    detail="Close a channel.",
                    synopsis="chan close channelId ?direction?",
                    return_type=TclType.STRING,
                ),
                "configure": SubCommand(
                    name="configure",
                    arity=Arity(1),
                    detail="Query or set channel options.",
                    synopsis="chan configure channelId ?optionName? ?value ...?",
                    return_type=TclType.STRING,
                ),
                "copy": SubCommand(
                    name="copy",
                    arity=Arity(2),
                    detail="Copy data from one channel to another.",
                    synopsis="chan copy inputChan outputChan ?-size size? ?-command callback?",
                    return_type=TclType.INT,
                ),
                "create": SubCommand(
                    name="create",
                    arity=Arity(2, 2),
                    detail="Create a script-level channel.",
                    synopsis="chan create mode cmdPrefix",
                    return_type=TclType.STRING,
                ),
                "eof": SubCommand(
                    name="eof",
                    arity=Arity(1, 1),
                    detail="Test for end of file on a channel.",
                    synopsis="chan eof channelId",
                    return_type=TclType.BOOLEAN,
                ),
                "event": SubCommand(
                    name="event",
                    arity=Arity(2, 3),
                    detail="Set up event handler for channel.",
                    synopsis="chan event channelId event ?script?",
                    return_type=TclType.STRING,
                    arg_roles={2: ArgRole.BODY},
                ),
                "flush": SubCommand(
                    name="flush",
                    arity=Arity(1, 1),
                    detail="Flush buffered output for a channel.",
                    synopsis="chan flush channelId",
                    return_type=TclType.STRING,
                ),
                "gets": SubCommand(
                    name="gets",
                    arity=Arity(1, 2),
                    detail="Read a line from a channel.",
                    synopsis="chan gets channelId ?varName?",
                    return_type=TclType.STRING,
                    arg_roles={1: ArgRole.VAR_NAME},
                ),
                "isbinary": SubCommand(
                    name="isbinary",
                    arity=Arity(1, 1),
                    detail="Test whether channel is binary.",
                    synopsis="chan isbinary channelId",
                ),
                "names": SubCommand(
                    name="names",
                    arity=Arity(0, 1),
                    detail="Return list of open channels.",
                    synopsis="chan names ?pattern?",
                    return_type=TclType.LIST,
                ),
                "pending": SubCommand(
                    name="pending",
                    arity=Arity(2, 2),
                    detail="Return number of bytes pending.",
                    synopsis="chan pending mode channelId",
                    return_type=TclType.INT,
                ),
                "pipe": SubCommand(
                    name="pipe",
                    arity=Arity(0, 0),
                    detail="Create a pair of connected channels.",
                    synopsis="chan pipe",
                    return_type=TclType.LIST,
                ),
                "pop": SubCommand(
                    name="pop",
                    arity=Arity(1, 1),
                    detail="Remove topmost stacked transformation.",
                    synopsis="chan pop channelId",
                    return_type=TclType.STRING,
                ),
                "postevent": SubCommand(
                    name="postevent",
                    arity=Arity(2, 2),
                    detail="Post an event to a reflected channel.",
                    synopsis="chan postevent channelId eventSpec",
                    return_type=TclType.STRING,
                ),
                "push": SubCommand(
                    name="push",
                    arity=Arity(2, 2),
                    detail="Push a transformation on top of a channel.",
                    synopsis="chan push channelId cmdPrefix",
                    return_type=TclType.STRING,
                ),
                "puts": SubCommand(
                    name="puts",
                    arity=Arity(1, 3),
                    detail="Write a string to a channel.",
                    synopsis="chan puts ?-nonewline? channelId string",
                    return_type=TclType.STRING,
                ),
                "read": SubCommand(
                    name="read",
                    arity=Arity(1, 2),
                    detail="Read data from a channel.",
                    synopsis="chan read ?-nonewline? channelId ?numChars?",
                    return_type=TclType.STRING,
                ),
                "seek": SubCommand(
                    name="seek",
                    arity=Arity(2, 3),
                    detail="Set access position for a channel.",
                    synopsis="chan seek channelId offset ?origin?",
                    return_type=TclType.STRING,
                ),
                "tell": SubCommand(
                    name="tell",
                    arity=Arity(1, 1),
                    detail="Return current access position.",
                    synopsis="chan tell channelId",
                    return_type=TclType.INT,
                ),
                "truncate": SubCommand(
                    name="truncate",
                    arity=Arity(1, 2),
                    detail="Truncate a channel to given length.",
                    synopsis="chan truncate channelId ?length?",
                    return_type=TclType.STRING,
                ),
            },
            validation=ValidationSpec(
                arity=Arity(1),
            ),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.FILE_IO,
                    reads=True,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )

    @classmethod
    def taint_hints(cls) -> TaintHint:
        return TaintHint(
            source={None: TaintColour.TAINTED},
            source_subcommands=frozenset({"read", "gets"}),
        )
