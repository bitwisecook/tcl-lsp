"""open -- Open a file-based or command pipeline channel."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from ....compiler.types import TclType
from .._base import CommandDef
from ..dialects import DIALECTS_EXCEPT_IRULES
from ..models import (
    ArgumentValueSpec,
    CommandSpec,
    FormKind,
    FormSpec,
    HoverSnippet,
    ValidationSpec,
)
from ..signatures import Arity
from ._base import register

_SOURCE = "Tcl man page open.n"


def _mode(value: str, detail: str) -> ArgumentValueSpec:
    return ArgumentValueSpec(
        value=value,
        detail=detail,
        hover=HoverSnippet(
            summary=detail,
            synopsis=(f"open fileName {value}",),
            source=_SOURCE,
        ),
    )


_ACCESS_MODES = (
    # Simple access strings
    _mode("r", "Read only (default). File must exist."),
    _mode("r+", "Read and write. File must exist."),
    _mode("w", "Write only. Truncate if exists, create if not."),
    _mode("w+", "Read and write. Truncate if exists, create if not."),
    _mode("a", "Write only, append. Create if not exists."),
    _mode("a+", "Read and write, append. Create if not exists."),
    # POSIX flag words (used in list-form access)
    _mode("RDONLY", "Open the file for reading only."),
    _mode("WRONLY", "Open the file for writing only."),
    _mode("RDWR", "Open the file for both reading and writing."),
    _mode("APPEND", "Set file pointer to end before each write."),
    _mode("BINARY", "Configure channel with -translation binary."),
    _mode("CREAT", "Create the file if it does not already exist."),
    _mode("EXCL", "Error if file already exists (with CREAT)."),
    _mode("NOCTTY", "Prevent file from becoming controlling terminal."),
    _mode("NONBLOCK", "Non-blocking open (use fconfigure instead)."),
    _mode("TRUNC", "Truncate file to zero length if it exists."),
)


@register
class OpenCommand(CommandDef):
    name = "open"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="open",
            dialects=DIALECTS_EXCEPT_IRULES,
            hover=HoverSnippet(
                summary="Open a file-based or command pipeline channel.",
                synopsis=(
                    "open fileName",
                    "open fileName access",
                    "open fileName access permissions",
                ),
                snippet="Returns a channel identifier for use with `read`, `puts`, `close`, etc.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="open fileName ?access? ?permissions?",
                    arg_values={1: _ACCESS_MODES},
                ),
            ),
            validation=ValidationSpec(arity=Arity(1, 3)),
            return_type=TclType.STRING,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.FILE_IO,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )
