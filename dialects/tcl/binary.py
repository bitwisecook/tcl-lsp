"""binary -- Manipulate binary data."""

from __future__ import annotations

from compiler.registry.models import (
    ArgumentValueSpec,
    CommandSpec,
    FormKind,
    FormSpec,
    HoverSnippet,
    SubCommand,
    ValidationSpec,
)
from compiler.registry.signatures import ArgRole, Arity
from compiler.registry.type_hints import ArgTypeHint
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from compiler.types import TclType

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
                    arg_types={0: ArgTypeHint(expected=TclType.BYTEARRAY, shimmers=True)},
                ),
                "encode": SubCommand(
                    name="encode",
                    arity=Arity(2),
                    detail="Encode a binary string.",
                    return_type=TclType.STRING,
                    arg_types={0: ArgTypeHint(expected=TclType.BYTEARRAY, shimmers=True)},
                ),
                "format": SubCommand(
                    name="format",
                    arity=Arity(1),
                    detail="Format a binary string.",
                    return_type=TclType.BYTEARRAY,
                    arg_types={0: ArgTypeHint(expected=TclType.STRING, shimmers=True)},
                ),
                "scan": SubCommand(
                    name="scan",
                    arity=Arity(2),
                    detail="Scan a binary string.",
                    return_type=TclType.INT,
                    arg_roles={
                        2: frozenset({ArgRole.VAR_WRITE}),
                        3: frozenset({ArgRole.VAR_WRITE}),
                        4: frozenset({ArgRole.VAR_WRITE}),
                        5: frozenset({ArgRole.VAR_WRITE}),
                        6: frozenset({ArgRole.VAR_WRITE}),
                        7: frozenset({ArgRole.VAR_WRITE}),
                        8: frozenset({ArgRole.VAR_WRITE}),
                        9: frozenset({ArgRole.VAR_WRITE}),
                        10: frozenset({ArgRole.VAR_WRITE}),
                        11: frozenset({ArgRole.VAR_WRITE}),
                        12: frozenset({ArgRole.VAR_WRITE}),
                        13: frozenset({ArgRole.VAR_WRITE}),
                        14: frozenset({ArgRole.VAR_WRITE}),
                        15: frozenset({ArgRole.VAR_WRITE}),
                        16: frozenset({ArgRole.VAR_WRITE}),
                        17: frozenset({ArgRole.VAR_WRITE}),
                        18: frozenset({ArgRole.VAR_WRITE}),
                        19: frozenset({ArgRole.VAR_WRITE}),
                    },
                    arg_types={
                        0: ArgTypeHint(expected=TclType.BYTEARRAY, shimmers=True),
                        1: ArgTypeHint(expected=TclType.STRING, shimmers=True),
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
