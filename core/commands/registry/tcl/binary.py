"""binary -- Manipulate binary data."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from ....compiler.types import TclType
from ..models import (
    ArgumentValueSpec,
    CommandSpec,
    FormKind,
    FormSpec,
    HoverSnippet,
    SubCommand,
    ValidationSpec,
)
from ..signatures import ArgRole, Arity
from ..type_hints import ArgTypeHint
from ._base import CommandDef, register

_SOURCE = "Tcl man page binary.n"

_SUBCOMMANDS = (
    ArgumentValueSpec(value="format", detail="Format a binary string."),
    ArgumentValueSpec(value="scan", detail="Scan a binary string."),
    ArgumentValueSpec(value="encode", detail="Encode a binary string."),
    ArgumentValueSpec(value="decode", detail="Decode a binary string."),
)


@register
class BinaryCommand(CommandDef):
    name = "binary"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="binary",
            hover=HoverSnippet(
                summary="Manipulate binary data",
                synopsis=(
                    "binary format formatString ?arg arg ...?",
                    "binary scan string formatString ?varName varName ...?",
                    "binary encode format ?-option value ...? data",
                    "binary decode format ?-option value ...? data",
                ),
                snippet="This command provides facilities for manipulating binary data. The principal operations are inserting values into a binary string and extracting values from a binary string.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="binary format formatString ?arg arg ...?",
                    arg_values={0: _SUBCOMMANDS},
                ),
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="binary scan string formatString ?varName varName ...?",
                    arg_values={0: _SUBCOMMANDS},
                ),
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="binary encode format ?-option value ...? data",
                    arg_values={0: _SUBCOMMANDS},
                ),
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="binary decode format ?-option value ...? data",
                    arg_values={0: _SUBCOMMANDS},
                ),
            ),
            subcommands={
                "decode": SubCommand(
                    name="decode",
                    arity=Arity(2),
                    detail="Decode a binary string.",
                    return_type=TclType.BYTEARRAY,
                    arg_types={0: ArgTypeHint(expected=TclType.STRING)},
                ),
                "encode": SubCommand(
                    name="encode",
                    arity=Arity(2),
                    detail="Encode a binary string.",
                    return_type=TclType.STRING,
                    arg_types={0: ArgTypeHint(expected=TclType.STRING)},
                ),
                "format": SubCommand(
                    name="format",
                    arity=Arity(1),
                    detail="Format a binary string.",
                    return_type=TclType.BYTEARRAY,
                    arg_types={0: ArgTypeHint(expected=TclType.STRING)},
                ),
                "scan": SubCommand(
                    name="scan",
                    arity=Arity(2),
                    detail="Scan a binary string.",
                    return_type=TclType.INT,
                    arg_roles={
                        2: ArgRole.VAR_NAME,
                        3: ArgRole.VAR_NAME,
                        4: ArgRole.VAR_NAME,
                        5: ArgRole.VAR_NAME,
                        6: ArgRole.VAR_NAME,
                        7: ArgRole.VAR_NAME,
                        8: ArgRole.VAR_NAME,
                        9: ArgRole.VAR_NAME,
                        10: ArgRole.VAR_NAME,
                        11: ArgRole.VAR_NAME,
                        12: ArgRole.VAR_NAME,
                        13: ArgRole.VAR_NAME,
                        14: ArgRole.VAR_NAME,
                        15: ArgRole.VAR_NAME,
                        16: ArgRole.VAR_NAME,
                        17: ArgRole.VAR_NAME,
                        18: ArgRole.VAR_NAME,
                        19: ArgRole.VAR_NAME,
                    },
                    arg_types={
                        0: ArgTypeHint(expected=TclType.BYTEARRAY),
                        1: ArgTypeHint(expected=TclType.STRING),
                    },
                ),
            },
            validation=ValidationSpec(
                arity=Arity(1),
            ),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.UNKNOWN, reads=True, connection_side=ConnectionSide.NONE
                ),
            ),
        )
